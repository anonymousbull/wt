use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, pg_conn, solana_rpc_client, PUMP_MIGRATION, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_ATA_PROGRAM, SOLANA_GRPC_URL, SOLANA_MINT, SOLANA_RENT_PROGRAM, SOLANA_SERUM_PROGRAM, SOLANA_SYSTEM_PROGRAM};
use crate::position::PositionConfig;
use crate::send_tx::send_tx;
use crate::trade::{Trade, TradePrice, TradeState};
use chrono::Utc;
use futures::StreamExt;
use log::{error, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterTransactions};
use crate::ray_log::{RayLog, RayLogInfo};

#[derive(Clone)]
pub struct Chan {
    // pub memecoin: Sender<InternalCommand>,
    // pub memecoin_tick: Sender<InternalCommand>,
    // pub pa: Sender<InternalCommand>,
    // pub app: tokio::sync::mpsc::UnboundedSender<InternalCommand>,
    pub bg: tokio::sync::mpsc::Sender<InternalCommand>,
    // pub bg: tokio::sync::mpsc::UnboundedSender<InternalCommand>,
    // pub user: Sender<InternalCommand>,
    // pub http: tokio::sync::mpsc::UnboundedSender<InternalCommand>,
    pub trade: tokio::sync::mpsc::Sender<InternalCommand>,
}

pub async fn bg_chan(mut rec: Receiver<InternalCommand>) {
    let pg = pg_conn().await;
    let mut c = pg.get().await.unwrap();
    let mut insert_prices = Vec::<TradePrice>::new();
    let mut update_trades = HashMap::<String, Trade>::new();
    while let Some(message) = rec.recv().await {
        match message {
            InternalCommand::InsertTrade(trade) => {
                trade.insert(&mut c).await.unwrap();
            }
            InternalCommand::InsertPrice(price) => {
                insert_prices.push(price);
                insert_prices.clear();
                // if insert_prices.len() > 20 {
                //     TradePrice::insert_bulk(&mut c, insert_prices).await.unwrap();
                //     insert_prices = vec![];
                // }
            }
            InternalCommand::UpdateTrade(trade) => {
                update_trades.insert(trade.pool_id.clone(), trade);
                let trades = update_trades.values().cloned().collect::<Vec<_>>();
                info!("straight up updating db {:?}", trades[0].state);
                Trade::upsert_bulk(&mut c, trades).await.unwrap();
                update_trades.clear();
            }
            InternalCommand::StopWatchTrade(mut trade) => {
                trade.exit_time = Some(Utc::now());
                Trade::upsert_bulk(&mut c, vec![trade]).await.unwrap();
            }
            _ => {}
        }
    }
}

/// example of swap_base_in log
/// https://solscan.io/token/4xDVi6XiDU6rAdvm4VjAdMoaXACXVjBuzLS74Cw1uvA3?activity_type=ACTIVITY_SPL_INIT_MINT&activity_type=ACTIVITY_TOKEN_ADD_LIQ&activity_type=ACTIVITY_TOKEN_REMOVE_LIQ&activity_type=ACTIVITY_TOKEN_SWAP&time=1735257600000&time=1735321439000&page=6#defiactivities
/// SwapBaseInLog { log_type: 3, amount_in: 670000000000, minimum_out: 1, direction: 1, user_source: 670000000000, pool_coin: 9000000000000000000, pool_pc: 150000000, out_amount: 8997980477953550992 }
/// example of swap_base_out log
/// pool init - https://solscan.io/token/W2tX3GxsVH6Jng4UfaaUkgsHqU1c1sTeTaiAG4Npump?time=1735321560000&time=1735321679000&page=5#defiactivities
/// pool swap - https://solscan.io/tx/SggjhnEzULzofBb6njNaaFP7T31z6uddrx2ibafA6FhpQxuifEoYh2WiRWgg5geysqRkiAuhS7esgDMxLxtmTp5
/// SwapBaseOutLog { log_type: 4, max_in: 2955714285, amount_out: 2364571428, direction: 1, user_source: 10000000, pool_coin: 171880473738872568, pool_pc: 843000100000, deduct_in: 11628 }
pub async fn trade_chan(chan: Chan, mut rec: Receiver<InternalCommand>) {
    let pg = pg_conn().await;
    let mut c = pg.get().await.unwrap();
    let kp = get_keypair();
    let pubkey = kp.pubkey();
    let max_positions = 1;
    let mut price_id = TradePrice::id(&mut c).await;
    let mut trades = Trade::get_all(&mut c)
        .await
        .into_iter()
        .filter(|x| x.exit_time.is_none())
        .map(|x| (x.pool_id.clone(), x))
        .collect::<HashMap<_, _>>();
    info!("active trades {}", trades.len());
    let cfg = PositionConfig {
        min_sol: dec!(0.08),
        max_sol: dec!(0.1),
        fee: dec!(0.00203928),
        jito: dec!(0.004),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(7_00000), // 0.0007 SOL
    };
    let geyser = yellowstone_grpc_client::GeyserGrpcClient::build_from_static(SOLANA_GRPC_URL);
    let mut c = geyser.connect().await.unwrap();
    let r = SubscribeRequest {
        transactions: vec![(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: None,
                failed: Some(false),
                signature: None,
                account_include: vec![RAYDIUM_V4_PROGRAM.to_string()],
                account_exclude: vec![],
                account_required: vec![RAYDIUM_V4_PROGRAM.to_string()],
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    let (_subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();
    let mut heartbeat = Instant::now();

    while let Some(message) = stream.next().await {
        if heartbeat.elapsed().as_secs() > 60 {
            info!("heartbeat {:?}", heartbeat.elapsed());
            heartbeat = Instant::now();
        }

        if let Ok(cmd) = rec.try_recv() {
            match cmd {
                InternalCommand::StopWatchTrade(trade) => {
                    info!("discarding trade");
                    trades.remove(&trade.pool_id);
                    chan.bg
                        .try_send(InternalCommand::StopWatchTrade(trade))
                        .unwrap();
                    info!("trades now {}", trades.len());
                }
                InternalCommand::RevertPendingTrade(mut trade) => {
                    info!("retrying trade status");
                    trade.state = TradeState::PositionFilled;
                    trades.insert(trade.pool_id.clone(), trade);
                }
                _ => {}
            }
        }

        if let Ok(m) = message {
            if let Some(update) = m.update_oneof {
                if let UpdateOneof::Transaction(v) = &update {
                    if let Some(t) = &v.transaction {
                        if let Some(meta) = &t.meta {
                            if let Some(tx) = &t.transaction {
                                if let Some(m) = &tx.message {

                                    let accounts = m
                                        .account_keys
                                        .iter()
                                        // .inspect(|x|{
                                        //     let result = std::panic::catch_unwind(|| {
                                        //         let amm_instrudction = AmmInstruction::unpack(x.as_slice());
                                        //         amm_instrudction
                                        //     });
                                        //     info!("amm_instrudction {:?}", result);
                                        //
                                        // })
                                        .map(|a| Pubkey::try_from(a.as_slice()).unwrap())
                                        .collect::<Vec<_>>();
                                    let watch_trade =
                                        accounts.iter()
                                            .find_map(|x| trades.get(&x.to_string()));
                                    if watch_trade.is_some()
                                        && meta
                                            .log_messages
                                            .iter()
                                            .any(|x| x.contains(&"ray_log".to_string()))
                                    {

                                        let mut trade = watch_trade.unwrap().clone();
                                        let sig = solana_sdk::signature::Signature::try_from(
                                            t.signature.as_slice(),
                                        )
                                        .unwrap();

                                        let log = meta
                                            .log_messages
                                            .iter()
                                            .find(|x| x.contains("ray_log"))
                                            .unwrap();


                                        let ray_log = RayLog::from_log(log.clone());
                                        if let RayLogInfo::Withdraw(_) = &ray_log.log {
                                            error!("RUG ALERT {sig:?}");
                                        }

                                        let amount_out = ray_log.amount_out;
                                        price_id += 1;
                                        let trade_price = trade.update_from_ray_log(
                                            &ray_log,
                                            price_id,
                                            false
                                        );
                                        price_id += 1;
                                        let next_trade_price = trade.update_from_ray_log(
                                            &ray_log,
                                            price_id,
                                            true
                                        );

                                        if TradeState::PositionPendingFill == trade.state {
                                            if &sig.to_string() == trade.tx_in_id.as_ref().unwrap()
                                            {
                                                trade.entry_time = Some(Utc::now());
                                                trade.entry_price = Some(trade_price.price);
                                                trade.state = TradeState::PositionFilled;
                                                trade.amount = Decimal::from(amount_out);
                                                trades.insert(trade.pool_id.clone(), trade.clone());
                                                info!("log {:?}", log);
                                                info!("sig {:?}", sig);
                                                // info!(
                                                //     "please look at this! {:?}",
                                                //     m.account_keys.len()
                                                // );
                                                // info!("please look at this! {:?}", accounts);
                                                // m.instructions.iter().enumerate().for_each(
                                                //     |(u, zz)| {
                                                //         info!("instruction {u}");
                                                //         info!(
                                                //             "instruction account len {}",
                                                //             zz.accounts.len()
                                                //         );
                                                //         info!("{:?}", zz.accounts);
                                                //     },
                                                // );
                                                chan.bg.try_send(InternalCommand::InsertPrice(
                                                    trade_price,
                                                )).unwrap();
                                                info!("position filled {:?}", trade);
                                            }
                                        } else if TradeState::PositionFilled == trade.state {
                                            let entry_price = trade.entry_price.unwrap();
                                            trade.pct =
                                                (next_trade_price.price - entry_price) / entry_price * dec!(100);
                                            let log = serde_json::json!({
                                                "pct":trade.pct,
                                                "signature": sig,
                                                "price": next_trade_price.price,
                                            });
                                            info!("{}",log);
                                            // info!("trade_price.price {}", trade_price.price);
                                            // info!("next_trade_price.price {}", next_trade_price.price);
                                            // info!("pct {}", trade.pct);
                                            // info!("trade:id {}", trade.id);
                                            // info!("sig {:?}", sig);

                                            trades.insert(trade.pool_id.clone(), trade.clone());

                                            if trade.pct > dec!(60) || trade.pct < dec!(-10) {
                                                let trade_req =
                                                    trade.create_position(next_trade_price.price, cfg, false);

                                                warn!("spamming close");
                                                match send_tx(trade_req).await {
                                                    Ok(trade_res) => {
                                                        let mut trade = trade_res.trade;
                                                        trade.state = TradeState::PositionPendingClose;
                                                        let sig = Signature::from_str(
                                                            trade_res.sig.as_str(),
                                                        )
                                                        .unwrap();
                                                        trade.tx_out_id = Some(trade_res.sig);
                                                        trades.insert(trade.pool_id.clone(), trade.clone());
                                                        let chan = chan.clone();
                                                        tokio::spawn(async move {
                                                            let rpc = solana_rpc_client();
                                                            rpc.poll_for_signature(&sig)
                                                                .await
                                                                .unwrap();
                                                            let res = rpc
                                                                .get_signature_status(&sig)
                                                                .await
                                                                .unwrap()
                                                                .unwrap();
                                                            if res.is_err() {
                                                                chan.trade
                                                                    .try_send(InternalCommand::RevertPendingTrade(
                                                                        trade,
                                                                    ))
                                                                    .unwrap();
                                                                error!(
                                                                    "create close position result {:?}",
                                                                    res
                                                                );
                                                            } else {
                                                                info!("closed trade polled successfully");
                                                            }
                                                        });
                                                    }
                                                    Err(e) => {
                                                        error!("could not close trade {e}");
                                                    }
                                                };
                                            }
                                        } else if TradeState::PositionPendingClose == trade.state {
                                            if accounts.iter().any(|x| x == &pubkey) {
                                                trade.state = TradeState::PositionClosed;
                                                trade.exit_time = Some(Utc::now());
                                                trade.exit_price = Some(trade_price.price);
                                                trades.insert(trade.pool_id.clone(), trade.clone());
                                                info!("closed {trade:?}");
                                            }

                                        }

                                        trades.clone().iter().for_each(|(id, trade)| {
                                            if TradeState::PositionClosed == trade.state {
                                                trades.remove(id);
                                            }
                                            let r = chan.bg.try_send(InternalCommand::UpdateTrade(
                                                trade.clone(),
                                            ));
                                            if r.is_err() {
                                                error!("update trade error {:?}", r);
                                            }
                                        });

                                        chan.bg.try_send(InternalCommand::InsertPrice(
                                            next_trade_price,
                                        )).unwrap();
                                    }
                                }
                            }

                            let is_trading = trades.len() < max_positions;

                            if is_trading
                                && meta
                                    .log_messages
                                    .iter()
                                    .any(|z| z.contains("init_pc_amount"))
                            {
                                let sig = solana_sdk::signature::Signature::try_from(
                                    t.signature.as_slice(),
                                ).unwrap();
                                info!("found {sig}");

                                if let Some(tx) = &t.transaction {
                                    if let Some(m) = &tx.message {
                                        let accounts = m.instructions.iter().find_map(|x| {
                                            if x.accounts.len() == 21 {
                                                let keys_maybe = x
                                                    .accounts
                                                    .iter()
                                                    .filter_map(|&index| {
                                                        m.account_keys.get(index as usize)
                                                    })
                                                    .map(|x| Pubkey::try_from(x.as_slice()))
                                                    .map(Result::unwrap)
                                                    .collect::<Vec<_>>();

                                                if keys_maybe.len() != 21 {
                                                    return None;
                                                }

                                                let mut has_system_program = false;
                                                let mut has_rent_program = false;
                                                let mut has_wsol_mint = false;
                                                let mut has_serum_program = false;
                                                let mut has_ray_auth_program = false;
                                                let mut has_token_program = false;
                                                let mut has_ata_program = false;
                                                for x in &keys_maybe {
                                                    if x == &SOLANA_SYSTEM_PROGRAM {
                                                        has_system_program = true
                                                    } else if x == &SOLANA_RENT_PROGRAM {
                                                        has_rent_program = true
                                                    } else if x == &SOLANA_MINT {
                                                        has_wsol_mint = true
                                                    } else if x == &SOLANA_SERUM_PROGRAM {
                                                        has_serum_program = true
                                                    } else if x == &RAYDIUM_V4_AUTHORITY {
                                                        has_ray_auth_program = true
                                                    } else if x == &spl_token::id()
                                                        || x == &spl_token_2022::id()
                                                    {
                                                        has_token_program = true
                                                    } else if x == &SOLANA_ATA_PROGRAM {
                                                        has_ata_program = true
                                                    }
                                                }
                                                let all_good = has_system_program
                                                    && has_ray_auth_program
                                                    && has_rent_program
                                                    && has_wsol_mint
                                                    && has_serum_program
                                                    && has_ray_auth_program
                                                    && has_token_program
                                                    && has_ata_program;

                                                if all_good {
                                                    Some(keys_maybe)
                                                } else {
                                                    None
                                                }
                                            } else {
                                                None
                                            }
                                        });
                                        if accounts.is_none() {
                                            continue;
                                        }
                                        let accounts = accounts.unwrap();

                                        // .and_then(|x|{
                                        //     if x.contains(&PUMP_MIGRATION) {
                                        //         Some(x)
                                        //     } else {
                                        //         None
                                        //     }
                                        // });

                                        let log = meta
                                            .log_messages
                                            .iter()
                                            .find(|x| x.contains("ray_log"))
                                            .unwrap();

                                        let ray_log = RayLog::from_log(log.clone());
                                        info!("init_log {:?}", ray_log.log);


                                        let mut trade =
                                            Trade::from_solana_account_grpc(accounts.as_slice(), sig.to_string());
                                        if &trade.user_wallet != &PUMP_MIGRATION.to_string() {
                                            info!("not a pump coin");
                                            continue
                                        }


                                        trade.root_kp = kp.to_bytes().to_vec();
                                        info!("trade:init {:?}", trade);

                                        let trade_price = trade.update_from_ray_log(
                                            &ray_log,
                                            price_id,
                                            false
                                        );

                                        info!("trade_price {trade_price:?}");
                                        let price = trade_price.price;




                                        let trade_req = trade.create_position(price, cfg, true);
                                        let mut trade_res = send_tx(trade_req).await.unwrap();
                                        let trade = trade_res.trade.clone();
                                        let sig = trade_res.sig;
                                        let signature = solana_sdk::signature::Signature::from_str(
                                            &sig.as_str(),
                                        )
                                        .unwrap();
                                        trade_res.trade.tx_in_id = Some(sig);
                                        trades.insert(trade.pool_id.clone(), trade_res.trade.clone());
                                        chan.bg
                                            .try_send(InternalCommand::InsertTrade(trade_res.trade))
                                            .unwrap();

                                        let chan = chan.clone();
                                        tokio::spawn(async move {
                                            info!("starting poll");
                                            let rpc = solana_rpc_client();
                                            rpc.poll_for_signature(&signature).await.unwrap();
                                            info!("polled");
                                            let res = rpc
                                                .get_signature_status(&signature)
                                                .await
                                                .unwrap()
                                                .unwrap();
                                            info!("poll result below");
                                            if res.is_err() {
                                                chan.trade
                                                    .try_send(InternalCommand::StopWatchTrade(
                                                        trade,
                                                    ))
                                                    .unwrap();
                                            }
                                            info!("create open position result {:?}", res);
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
