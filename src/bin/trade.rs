use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_sdk::account::ReadableAccount;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::program_pack::Pack;
use wolf_trader::constant::solana_rpc_client;
use wolf_trader::position::PositionConfig;
use wolf_trader::send_tx::send_tx;
use wolf_trader::trade::Trade;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let pg = wolf_trader::constant::pg_conn().await;
    let mut c = pg.get().await.unwrap();
    let args = std::env::args().collect::<Vec<_>>();

    let cfg = PositionConfig {
        min_sol: dec!(0.008),
        max_sol: dec!(0.01),
        ata_fee: dec!(0.00203928),
        jito: dec!(0.04),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(20_00000), // 0.0007 SOL
    };
    let rpc = solana_rpc_client();

    let bs = &args[1];
    let coin = &args[2];

    let trade = Trade::get_by_pool(&mut c, coin.clone()).await;

    let accounts = rpc.get_multiple_accounts(
        vec![
            trade.pc_vault(),
            trade.coin_vault()
        ].as_slice()
    ).await.unwrap();

    let pc = spl_token::state::Account::unpack(accounts[0].as_ref().unwrap().data()).unwrap();
    let coin = spl_token::state::Account::unpack(accounts[1].as_ref().unwrap().data()).unwrap();

    let sol = std::cmp::min(pc.amount, coin.amount);
    let coin = std::cmp::max(pc.amount, coin.amount);


    let sol_normal = Decimal::from(sol) / Decimal::from(LAMPORTS_PER_SOL);
    let coin_normal = Decimal::from(coin) / dec!(1_000_000);
    let price = sol_normal / coin_normal;


    let req = if bs.as_str() == "buy" {
        let req = trade.create_position(
            price,
            cfg,
            true
        );
        tokio::fs::remove_file("trade.json").await;

        tokio::fs::write("trade.json",serde_json::to_string(&req.trade).unwrap()).await.unwrap();
        req
    } else {
        let st = tokio::fs::read("trade.json").await.unwrap();
        let trade = serde_json::from_slice::<Trade>(st.as_slice()).unwrap();
        let req = trade.create_position(
            price,
            cfg,
           false
        );
        req
    };



    let resp = send_tx(req).await;

    println!("{:?}",resp);
}