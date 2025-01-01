use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, pg_conn, rpcs, solana_rpc_client, PUMP_MIGRATION, PUMP_PROGRAM, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_ATA_PROGRAM, SOLANA_GRPC_URL, SOLANA_MINT, SOLANA_RENT_PROGRAM, SOLANA_SERUM_PROGRAM, SOLANA_SYSTEM_PROGRAM, SURREAL_DB_URL};
use crate::position::PositionConfig;
use crate::ray_log::{RayLog, RayLogInfo};
use crate::send_tx::{send_tx, SendTxConfig};
use crate::trade::{Trade, TradePrice, TradeState};
use chrono::Utc;
use diesel::expression::array_comparison::In;
use futures::StreamExt;
use log::{error, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeUpdateTransactionInfo,
};
use yellowstone_grpc_proto::prelude::{
    Message, SubscribeUpdate, SubscribeUpdateTransaction, Transaction, TransactionStatusMeta,
};
use yellowstone_grpc_proto::tonic::Status;

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
    pub ws: tokio::sync::mpsc::Sender<InternalCommand>,
}

#[derive(Debug)]
pub struct InterestedTx {
    pub signature: Signature,
    pub accounts: Vec<Pubkey>,
    pub logs: String,
    pub message: Message,
}

fn decode_tx(
    update: Result<SubscribeUpdate, Status>,
    trades: &HashMap<String, Trade>,
    open_txs: &[String],
    close_txs: &[String],
) -> Option<InternalCommand> {
    if let Ok(SubscribeUpdate {
        update_oneof:
            Some(UpdateOneof::Transaction(SubscribeUpdateTransaction {
                transaction:
                    Some(SubscribeUpdateTransactionInfo {
                        signature,
                        transaction:
                            Some(Transaction {
                                message: Some(m), ..
                            }),
                        meta: Some(meta),
                        ..
                    }),
                ..
            })),
        ..
    }) = update
    {
        let accounts = m
            .account_keys
            .iter()
            .map(|a| Pubkey::try_from(a.as_slice()).unwrap())
            .collect::<Vec<_>>();

        let mut interested_tx = InterestedTx {
            signature: Signature::try_from(signature).unwrap(),
            accounts,
            logs: Default::default(),
            message: m,
        };
        let watch_trades = interested_tx
            .accounts
            .iter()
            .find_map(|x| {
                trades.get(&x.to_string())
            });
        let mut logs_iter = meta.log_messages.iter();

        let ray_log_maybe = logs_iter.find(|x| x.contains("ray_log"));
        if let Some(ray_log) = ray_log_maybe {
            interested_tx.logs = ray_log.clone();

            if let Some(trade) = watch_trades {
                Some(InternalCommand::TradeState {
                    trade:trade.clone(),
                    interested_tx,
                })
            } else if meta.log_messages.iter().any(|z| z.contains("init_pc_amount")) {
                Some(InternalCommand::PumpMigration(interested_tx))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
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

fn poll_trade_and_cmd(
    signature: Signature,
    chan: Chan,
    status: InternalCommand,
    log_action: String,
) {
    tokio::spawn(async move {
        let mut err = false;
        let mut err_msg = None;
        let rpc = solana_rpc_client();
        let res = rpc.poll_for_signature(&signature).await;
        if res.is_err() {
            err = true;
            err_msg = Some(format!("{:?}", res));
        } else {
            let res = rpc.get_signature_status(&signature).await;
            if let Ok(Some(Ok(_))) = res {
                info!("{log_action} poll success");
            } else {
                err = true;
                err_msg = Some(format!("{:?}", res));
            }
        }
        if err {
            error!("could not {log_action} {:?}", err_msg);
            chan.trade.try_send(status).unwrap();
        }
    });
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
    let db = surrealdb::Surreal::new::<surrealdb::engine::remote::ws::Ws>(SURREAL_DB_URL).await.unwrap();
    db.use_ns("wt").use_db("wt").await.unwrap();

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
    let mut open_txs = vec![];
    let mut close_txs = vec![];

    let mut external_trade = None;
    info!("active trades {}", trades.len());
    let cfg = PositionConfig {
        min_sol: dec!(0.008),
        max_sol: dec!(0.01),
        ata_fee: dec!(0.00203928),
        jito: dec!(0.001),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(20_00000), // 0.0007 SOL
        base_fee: dec!(0.000005),
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
                account_include: vec![RAYDIUM_V4_PROGRAM.to_string(), PUMP_PROGRAM.to_string()],
                account_exclude: vec![],
                account_required: vec![],
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    let (_subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();
    let mut heartbeat = Instant::now();

    while let Some(message) = stream.next().await {
        let mut output = json!({});
        let mut err = false;

        if heartbeat.elapsed().as_secs() > 30 {
            info!("heartbeat {:?}", heartbeat.elapsed());
            heartbeat = Instant::now();
        }

        if let Ok(cmd) = rec.try_recv() {
            match cmd {
                InternalCommand::ExternalTrade(trade) => {
                    external_trade = Some(trade);
                }
                InternalCommand::AddTxOpen(Trade{tx_in_id,state,pool_id,..}) => {
                    let mut trade = trades.get_mut(&pool_id).unwrap();
                    // trade.tx_in_ids.push(tx_in_id);
                    if trade.state != TradeState::PositionFilled {
                        trade.state = TradeState::PositionPendingFill;
                    }
                }
                InternalCommand::AddTxClose(Trade{tx_out_id,state,pool_id,..}) => {
                    let mut trade = trades.get_mut(&pool_id).unwrap();
                    // trade.tx_out_ids.push(tx_out_id);
                    if trade.state != TradeState::PositionClosed {
                        trade.state = TradeState::PositionPendingClose;
                    }
                }
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

        let maybe_cmd = decode_tx(message, &trades);
        match maybe_cmd {
            Some(InternalCommand::TradeState { mut trade, interested_tx: tx, }) => {
                output["signature"] = json!(tx.signature.to_string());

                let signature = tx.signature;
                let ray_log = RayLog::from_log(tx.logs.clone());
                if let RayLogInfo::Withdraw(q) = &ray_log.log {
                    output["log"] = json!(ray_log);
                    warn!("{output}");
                    continue;
                }

                let amount_out = ray_log.amount_out;
                price_id += 1;
                let trade_price = trade.update_from_ray_log(&ray_log, price_id, false);
                price_id += 1;
                let next_trade_price = trade.update_from_ray_log(&ray_log, price_id, true);

                match trade.state {
                    TradeState::PositionPendingFill => {
                        if &tx.signature.to_string() == trade.tx_in_id.as_ref().unwrap() {
                            trade.entry_time = Some(Utc::now());
                            trade.entry_price = Some(trade_price.price);
                            trade.state = TradeState::PositionFilled;
                            trade.amount = Decimal::from(amount_out);
                            trades.insert(trade.pool_id.clone(), trade.clone());
                            output["trade"] = json!(trade);
                            output["trade_price"] = json!(trade_price);
                            output["next_trade_price"] = json!(next_trade_price);
                            output["log"] = json!(ray_log);
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
                            chan.bg
                                .try_send(InternalCommand::InsertPrice(trade_price))
                                .unwrap();
                        }
                    }
                    TradeState::PositionFilled => {
                        let entry_price = trade.entry_price.unwrap();
                        trade.pct =
                            (next_trade_price.price - entry_price) / entry_price * dec!(100);
                        trades.insert(trade.pool_id.clone(), trade.clone());

                        output["pct"] = json!(trade.pct);
                        output["mint"] = json!(trade.mint());
                        output["trade_price"] = json!(trade_price);
                        output["next_trade_price"] = json!(next_trade_price);

                        if trade.pct > dec!(5) || trade.pct < dec!(-1) {
                            let trade_req = trade.create_position(next_trade_price.price, cfg, false);
                            trade = trade_req.trade.clone();
                            warn!("spamming close");

                            let c = chan.clone();
                            let chan = c.clone();
                            let mut futs = send_tx(trade_req, Some(SendTxConfig{ jito_tip: cfg.jito, })).await;
                            while let Some(Ok(s)) = futs.next().await {
                                match s {
                                    Ok(trade_res) => {
                                        trade = trade_res.trade;
                                        let sig = Signature::from_str(trade_res.sig.as_str()).unwrap();
                                        close_txs.push(Some(trade_res.sig));
                                        if trade.state != TradeState::PositionClosed {
                                            trade.state = TradeState::PositionPendingClose;
                                        }
                                        trades.insert(trade.pool_id.clone(), trade.clone());
                                        // chan.trade.try_send(InternalCommand::AddTxClose(trade.clone())).unwrap();
                                        poll_trade_and_cmd(
                                            sig,
                                            chan.clone(),
                                            InternalCommand::RevertPendingTrade(trade.clone()),
                                            "close position".to_string(),
                                        );
                                    }
                                    Err(e) => {
                                        output["error"] = json!({
                                                "message":"could not close trade",
                                                "data": e.to_string()
                                            });
                                        err = true;
                                    }
                                }
                            }
                        }
                    }
                    TradeState::PositionPendingClose => {
                        if tx.accounts.iter().any(|x| x == &pubkey) {
                            trade.state = TradeState::PositionClosed;
                            trade.exit_time = Some(Utc::now());
                            trade.exit_price = Some(trade_price.price);
                            trades.insert(trade.pool_id.clone(), trade.clone());
                            output["trade"] = json!(trade);
                        }
                    }
                    TradeState::PositionClosed => {
                        trades.remove(&trade.id);
                    }
                    _ => {}
                }


                let r = chan
                    .bg
                    .try_send(InternalCommand::UpdateTrade(trade.clone()));
                if r.is_err() {
                    error!("update trade error {:?}", r);
                }
                chan.bg
                    .try_send(InternalCommand::InsertPrice(next_trade_price))
                    .unwrap();

                if err {
                    error!("{output}");
                } else {
                    info!("{output}");
                }
            }
            Some(InternalCommand::PumpMigration(tx)) => {
                let accounts = tx.message.instructions.iter().find_map(|x| {
                    if x.accounts.len() == 21 {
                        let keys_maybe = x
                            .accounts
                            .iter()
                            .filter_map(|&index| tx.accounts.get(index as usize))
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
                        for &x in &keys_maybe {
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
                            } else if x == &spl_token::id() || x == &spl_token_2022::id() {
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
                let ray_log = RayLog::from_log(tx.logs.clone());
                let mut trade = Trade::from_solana_account_grpc(
                    accounts.iter().cloned().cloned().collect::<Vec<_>>().as_slice(),
                    tx.signature.to_string(),
                );
                if &trade.user_wallet != &PUMP_MIGRATION.to_string() {
                    info!("not a pump coin {:?} {:?}",tx.signature, trade);
                    continue;
                }
                trade.root_kp = kp.to_bytes().to_vec();
                info!("init_log {:?}", ray_log.log);
                info!("trade:init {:?}", trade);

                let trade_price = trade.update_from_ray_log(&ray_log, price_id, false);

                info!("trade_price {trade_price:?}");
                let price = trade_price.price;

                let trade_req = trade.create_position(price, cfg, true);
                let mut futs = send_tx(trade_req, Some(SendTxConfig{ jito_tip: cfg.jito, })).await;
                while let Some(Ok(s)) = futs.next().await {
                    match s {
                        Ok(mut trade_res) => {
                            trade = trade_res.trade;
                            let sig = trade_res.sig;
                            let signature = solana_sdk::signature::Signature::from_str(&sig.as_str()).unwrap();
                            // trade_res.trade.tx_in_id = Some(sig);
                            open_txs.push(Some(sig));
                            trades.insert(trade.pool_id.clone(), trade.clone());
                            // chan.trade
                            //     .try_send(InternalCommand::AddTxOpen(trade_res.trade.clone()))
                            //     .unwrap();
                            chan.bg
                                .try_send(InternalCommand::InsertTrade(trade.clone()))
                                .unwrap();
                            poll_trade_and_cmd(
                                signature,
                                chan.clone(),
                                InternalCommand::StopWatchTrade(trade),
                                "open position".to_string(),
                            );
                        }
                        Err(e) => {
                            output["error"] = json!({
                                                "message":"could not close trade",
                                                "data": format!("{:?}", e),
                                            });
                            err = true;
                        }
                    }

                }
                if err {
                    error!("{output}");
                } else {
                    info!("{output}");
                }
            }
            _ => {}
        }
    }
}
