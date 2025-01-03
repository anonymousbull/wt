use base64::Engine;
use log::info;
use reqwest::Client;
use serde_json::json;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcSendTransactionConfig;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::native_token::sol_to_lamports;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;
use raydium_amm::instruction::swap_base_in;
use raydium_amm::solana_program::pubkey::Pubkey;
use wolf_trader::constant::{get_keypair, BLOX_HEADER, RAYDIUM_V4_PROGRAM, SOLANA_MINT_STR, RPC1};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    let rpc = RpcClient::new(RPC1.to_string());

    // PoolIx{
    //     token_program: "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA".to_string(),
    //     spl_associated_token: "".to_string(),
    //     system_program: "".to_string(),
    //     rent: "".to_string(),
    //     amm: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
    //     amm_authority: "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1".to_string(),
    //     amm_open_orders: "58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2".to_string(),
    //     lp_mint: "".to_string(),
    //     coin_mint: "".to_string(),
    //     pool_coin_mint: "".to_string(),
    //     coin_vault: "".to_string(),
    //     pool_coin_vault: "".to_string(),
    //     pool_withdraw_queue: "".to_string(),
    //     amm_target_orders: "".to_string(),
    //     pool_temp_lp: "".to_string(),
    //     serum_program: "HzLxx6SoViXoqB2KYDGT1gWEckZ9ooRnv8n3xN6uvfez".to_string(),
    //     serum_market: "HzLxx6SoViXoqB2KYDGT1gWEckZ9ooRnv8n3xN6uvfez".to_string(),
    //     user_wallet: "".to_string(),
    //     user_coin_vault: "".to_string(),
    //     user_pool_coin_vault: "".to_string(),
    //     user_lp_vault: "".to_string(),
    // };

    let amm_program = RAYDIUM_V4_PROGRAM;
    let amm_pool = Pubkey::from_str_const("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2");
    let amm_auth = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let open = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let coin_vault = Pubkey::from_str_const("GosSw8RSzry4CiXTcczYPrjuSfgH7eeKzVEzoYic4KJR");
    let pc_vault = Pubkey::from_str_const("4PG3U2o8XgoXFcFhMvS4tiCjQQHF34DFywPK8JkAvzbG");
    let market_program = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let market = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let bid = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let ask = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let event = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let market_coin = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let market_pc = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let market_signer = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let user_src = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let user_dest = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let user = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
    let kp = get_keypair();

    let usdc =
        spl_associated_token_account_client::address::get_associated_token_address(
            &kp.pubkey(),
            &Pubkey::from_str_const("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
        );
    let usdc_vault = Pubkey::from_str_const("HLmqeL62xR1QoZ1HKKbXRrdN1p3phKpxRMb2VVopvBBz");
    let sol_vault = Pubkey::from_str_const("DQyrAcCrDXQ7NeoqGgDCZwBvWDcYmFCjSb9JtteuvPpz");
    let sol =
        spl_associated_token_account_client::address::get_associated_token_address(
            &kp.pubkey(),
            &Pubkey::from_str_const(SOLANA_MINT_STR),
        );

    println!("{:?}",usdc);
    println!("{:?}",sol);
    let recent_blockhash = rpc.get_latest_blockhash().await.unwrap();
    fn build_memo_instruction() -> Instruction {
        let trader_apimemo_program =
            Pubkey::from_str_const("HQ2UUt18uJqKaQFJhgV9zaTdQxUZjNrsKFgoEDquBkcx");
        let accounts = vec![AccountMeta::new(trader_apimemo_program, false)];
        let bx_memo_marker_msg = String::from("Powered by bloXroute Trader Api")
            .as_bytes()
            .to_vec();
        Instruction {
            program_id: trader_apimemo_program,
            accounts,
            data: bx_memo_marker_msg,
        }
    }
    let mut tx = Transaction::new_signed_with_payer(
        &vec![
            ComputeBudgetInstruction::set_compute_unit_price(10_00000),
            ComputeBudgetInstruction::set_compute_unit_limit(100_000),
            // swap_base_in(
            //     &RAYDIUM_V4_PROGRAM,
            //     &amm_pool,
            //     &amm_auth,
            //     &amm_pool,
            //     &sol_vault,
            //     &usdc_vault,
            //
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //     &amm_pool,
            //
            //     &usdc,
            //     &sol,
            //
            //     &kp.pubkey(),
            //
            //     1_0000,
            //     0
            // ).unwrap(),
            build_memo_instruction(),
            solana_sdk::system_instruction::transfer(
                &kp.pubkey(),
                &Pubkey::from_str_const("HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY"),
                sol_to_lamports(0.001),
            ),

        ],
        Some(&kp.pubkey()),
        &[kp],
        recent_blockhash,
    );


    let serialized_tx =
        base64::engine::general_purpose::STANDARD.encode(bincode::serialize(&tx).unwrap());
    let client = Client::new();

    let response = client.post("https://ny.solana.dex.blxrbdn.com/api/v2/submit")
        .header("Authorization", BLOX_HEADER)
        .json(&json!({
            "transaction": {"content": serialized_tx},
            "frontRunningProtection": false,
            "useStakedRPCs": true,
        }))
        .send()
        .await.unwrap();

    println!("Status: {}", response.status());
    info!("Body: {}", response.text().await.unwrap());

    // let sig = rpc
    //     .send_transaction_with_config(
    //         &tx,
    //         RpcSendTransactionConfig{
    //             skip_preflight: true,
    //             preflight_commitment: None,
    //             encoding: None,
    //             max_retries: Some(0),
    //             min_context_slot: None,
    //         }
    //     ).await;
    //
    // println!("dwe {:?}",sig);

}