use std::cmp::PartialEq;
use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, PUMP_MIGRATION, PUMP_PROGRAM, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_ATA_PROGRAM, SOLANA_MINT, SOLANA_RENT_PROGRAM, SOLANA_SERUM_PROGRAM, SOLANA_SYSTEM_PROGRAM};
use crate::position::PositionConfig;
use crate::ray_log::{RayLog, RayLogInfo};
use crate::rpc::{geyser, rpcs, solana_rpc_client, RpcResponseData, RpcsConfig};
use crate::trade::{Trade, TradePrice, TradeResponse, TradeResponseError, TradeState};
use chrono::{DateTime, Utc};
use futures::StreamExt;
use log::{error, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::collections::HashMap;
use std::str::FromStr;
use anyhow::anyhow;
use futures::stream::FuturesUnordered;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncReadExt};
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{
    SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeUpdateTransactionInfo,
};
use yellowstone_grpc_proto::prelude::{
    Message, SubscribeUpdate, SubscribeUpdateTransaction, Transaction,
};
use yellowstone_grpc_proto::tonic::Status;
use crate::chan::Chan;
use crate::solana::*;

#[derive(Debug)]
pub struct InterestedTx {
    pub signature: Signature,
    pub accounts: Vec<Pubkey>,
    pub logs: String,
    pub message: Message,
}

fn decode_tx(
    update: Result<SubscribeUpdate, Status>,
    cache: &HashMap<i64, Trade>,
    mints: &HashMap<Pubkey, Trade>,
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
                mints.get(&x).and_then(|x|cache.get(&x.id))
            });
        let mut logs_iter = meta.log_messages.iter();

        let ray_log_maybe = logs_iter.find(|x| x.contains("ray_log"));
        if let Some(ray_log) = ray_log_maybe {

            interested_tx.logs = ray_log.clone();

            if let Some(trade) = watch_trades {
                if trade.state == TradeState::BuySuccess {
                    info!("what is going on");
                }
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

fn poll_trade_and_cmd<F,F2>(
    signature: Signature,
    chan: Chan,
    trade:Trade,
    log_action: String,
    handle_err: F,
    handle_success: F2,
) where
    F: Fn(Chan,Trade,Option<anyhow::Error>)+Send+'static,
    F2: Fn(Chan,Trade,Option<anyhow::Error>)+Send+'static,
{
    tokio::spawn(async move {
        let rpc = solana_rpc_client();
        let res = rpc.poll_for_signature(&signature).await;
        let is_poll_err =  if res.is_err() {
            Some(format!("{:?}", res))
        } else {
            let res = rpc.get_signature_status(&signature).await;
            if let Ok(Some(Ok(_))) = res {
                None
            } else {
                Some(format!("{:?}", res))
            }
        };
        if let Some(err_msg) = is_poll_err {
            let e = Some(anyhow!("could not {log_action} {:?}", err_msg));
            handle_err(chan,trade, e);
        } else {
            info!("{log_action} poll success");
            handle_success(chan,trade, None);
        }
    });
}

#[derive(Serialize,Deserialize,Debug,Default,Clone)]
pub struct TradeChanLog {
    pub signature: Option<Signature>,
    pub trade_price:Option<TradePrice>,
    pub trade: Trade,
    pub ray_log: Option<RayLog>,
    pub error: Option<String>,
    pub dt: DateTime<Utc>,
}



impl TradeChanLog {
    pub fn console_log(&self)->String{
        let mut json = json!({});
        json["id"] = json!(self.trade.id);
        json["state"] = json!(self.trade.state);
        json["signature"] = json!(self.signature.clone().map(|s| s.to_string()));
        json["pct"] = json!(self.trade.pct);
        json["price"] = json!(self.trade_price.clone().map(|p|p.price));
        json["mint"] = json!(self.trade.mint().to_string());
        let jito_tip = self.trade.buy_logs.iter().find_map(|x|
            match x.rpc_response_data {
                RpcResponseData::Jito { tip_amount_sol,.. } => Some(lamports_to_sol_dec(tip_amount_sol)),
                RpcResponseData::General { .. } => None
            }
        );
        ;
        json["jito_tip"] = json!(jito_tip);
        json["jito_signature"] = json!(self.trade.buy_ids.get(0));
        json.to_string()
    }
}

#[derive(Serialize,Deserialize,Debug,Default)]
pub enum TradeLevel {
    Error = 1,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

/// example of swap_base_in log
/// https://solscan.io/token/4xDVi6XiDU6rAdvm4VjAdMoaXACXVjBuzLS74Cw1uvA3?activity_type=ACTIVITY_SPL_INIT_MINT&activity_type=ACTIVITY_TOKEN_ADD_LIQ&activity_type=ACTIVITY_TOKEN_REMOVE_LIQ&activity_type=ACTIVITY_TOKEN_SWAP&time=1735257600000&time=1735321439000&page=6#defiactivities
/// SwapBaseInLog { log_type: 3, amount_in: 670000000000, minimum_out: 1, direction: 1, user_source: 670000000000, pool_coin: 9000000000000000000, pool_pc: 150000000, out_amount: 8997980477953550992 }
/// example of swap_base_out log
/// pool init - https://solscan.io/token/W2tX3GxsVH6Jng4UfaaUkgsHqU1c1sTeTaiAG4Npump?time=1735321560000&time=1735321679000&page=5#defiactivities
/// pool swap - https://solscan.io/tx/SggjhnEzULzofBb6njNaaFP7T31z6uddrx2ibafA6FhpQxuifEoYh2WiRWgg5geysqRkiAuhS7esgDMxLxtmTp5
/// SwapBaseOutLog { log_type: 4, max_in: 2955714285, amount_out: 2364571428, direction: 1, user_source: 10000000, pool_coin: 171880473738872568, pool_pc: 843000100000, deduct_in: 11628 }
pub async fn trade_chan(chan: Chan, mut rec: Receiver<InternalCommand>) {
    let mut heartbeat = Instant::now();
    let kp = get_keypair();
    let max_positions = 1;
    let mut price_id = 0;
    let mut trades_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("trades.json")
        .await.unwrap();
    let mut contents = String::new();
    trades_file.read_to_string(&mut contents).await.unwrap();
    let trades = serde_json::from_str::<Vec<Trade>>(&contents)
        .unwrap_or(vec![]);
    let mut trade_id = trades.len() as i64;
    let mut cache = trades
        .into_iter()
        .filter(|x| x.exit_time.is_none())
        .map(|x| (x.id.clone(), x))
        .collect::<HashMap<_, _>>();
    let mut mints = cache.values().map(|x|(x.mint(), x.clone()))
        .collect::<HashMap<_, _>>();
    let mut external_trade = None;
    info!("active trades {}", cache.len());

    let cfg = PositionConfig {
        max_sol: dec!(0.01),
        ata_fee: dec!(0.00203928),
        max_jito: dec!(0.001),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(7_00_000), // 0.0007 SOL
        base_fee: dec!(0.000005),
        jito_priority_percent: dec!(0.7),
        jito_tip_percent: dec!(0.3),
        cu_limit: Default::default(),
        slippage: dec!(0.2),
        buy_gas_limit: dec!(60_000),
        sell_gas_limit:dec!(60_000),
        tp: dec!(0.05),
        fee_pct: dec!(0.8),
    };
    let geyser = geyser();

    let mut c = geyser.connect().await.unwrap();
    info!("connected to geyser");

    let r = SubscribeRequest {
        transactions: vec![(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                account_include: vec![RAYDIUM_V4_PROGRAM.to_string(), PUMP_PROGRAM.to_string()],
                failed: Some(false),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let mut rpcs_config = RpcsConfig{
        jito_tip:None,
    };
    let mut rpc_len = 0usize;

    let (_subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();

    while let Some(message) = stream.next().await {
        let mut trade_log = TradeChanLog::default();

        if heartbeat.elapsed().as_secs() > 30 {
            info!("heartbeat {:?}", heartbeat.elapsed());
            heartbeat = Instant::now();
        }

        if let Ok(cmd) = rec.try_recv() {
            match cmd {
                InternalCommand::JitoTip(tip) => {
                    rpcs_config.jito_tip = Some(tip);
                }
                InternalCommand::ExternalTrade(trade) => {
                    external_trade = Some(trade);
                }
                InternalCommand::BuyTradeFail {
                    trade:Trade{id,..},
                    error
                } => {
                    let mut trade = cache.get(&id).unwrap().clone();
                    trade.buy_attempts += 1;
                    cache.insert(id, trade.clone());
                    if trade.buy_attempts == rpc_len {
                        trade.state = TradeState::BuyFailed;
                        mints.remove(&trade.mint());
                        cache.remove(&id);
                        error!("buy trade failed {trade_log:?}");
                    }
                    let trade_log = TradeChanLog{
                        trade: trade.clone(),
                        dt: Utc::now(),
                        error: Some(error.to_string()),
                        ..Default::default()
                    };
                    chan.bg.try_send(InternalCommand::LogTrade(trade_log)).unwrap();
                }
                InternalCommand::SellTradeFail {
                    trade: Trade{id,..},
                    error
                } => {
                    let mut trade = cache.get(&id).unwrap().clone();
                    trade.sell_attempts += 1;
                    cache.insert(id, trade.clone());
                    if trade.sell_attempts == rpc_len {
                        trade.state = TradeState::SellFailed;
                        mints.remove(&trade.mint());
                        cache.remove(&id);
                        error!("sell trade failed {trade_log:?}");
                    }
                    let trade_log = TradeChanLog{
                        trade: trade.clone(),
                        dt: Utc::now(),
                        error: Some(error.to_string()),
                        ..Default::default()
                    };
                    chan.bg.try_send(InternalCommand::LogTrade(trade_log)).unwrap();
                }
                _ => {}
            }
        }

        let maybe_cmd = decode_tx(message, &cache, &mints);
        match maybe_cmd {
            Some(InternalCommand::TradeState { mut trade, interested_tx: InterestedTx{accounts:_,logs,signature,..}, }) => {
                trade_log.signature = Some(signature);

                let ray_log = RayLog::from_log(logs.clone());
                trade_log.ray_log = Some(ray_log.clone());
                if let RayLogInfo::Withdraw(q) = &ray_log.log {
                    warn!("{q:?}");
                    continue;
                }

                let amount_out = ray_log.amount_out;
                price_id += 1;
                let trade_price = trade.update_from_ray_log(&ray_log, price_id, false);
                price_id += 1;
                let next_trade_price = trade.update_from_ray_log(&ray_log, price_id, true);
                trade_log.trade_price = Some(next_trade_price.clone());
                chan.bg
                    .try_send(InternalCommand::InsertPrice(next_trade_price.clone()))
                    .unwrap();

                match trade.state {
                    TradeState::PendingBuy => {
                        if trade.buy_ids.contains(&signature.to_string()) {
                            trade.entry_time = Some(Utc::now());
                            trade.entry_price = Some(trade_price.price);
                            trade.state = TradeState::BuySuccess;
                            trade.amount = Decimal::from(amount_out);
                            trade.buy_id = Some(signature.to_string());
                            trade.buy_log = trade.buy_logs.iter().find(|x|&x.signature == &signature.to_string()).cloned();
                            cache.insert(trade.id, trade.clone());

                            trade_log.trade = trade.clone();
                            chan.bg.try_send(InternalCommand::LogTrade(trade_log.clone())).unwrap();
                            info!("trade buy success");
                            info!("{}",trade_log.console_log());

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
                        }
                    }
                    TradeState::BuySuccess => {
                        info!("checking profit");
                        let entry_price = trade.entry_price.unwrap();
                        trade.pct = (next_trade_price.price - entry_price) / entry_price * dec!(100);
                        cache.insert(trade.id, trade.clone());

                        trade_log.trade = trade.clone();
                        chan.bg.try_send(InternalCommand::LogTrade(trade_log)).unwrap();

                        if trade.pct > dec!(5) || trade.pct < dec!(-1) {
                            let trade_req = trade.create_position(next_trade_price.price, cfg, false);
                            warn!("spamming close");

                            let mut futs = FuturesUnordered::new();

                            rpcs(rpcs_config).into_iter().for_each(|x|
                                futs.push(
                                    tokio::spawn({
                                        let trade_req = trade_req.clone();
                                        async move {trade_req.clone().build_tx_and_send(x).await}
                                    })
                                )
                            );
                            while let Some(Ok(s)) = futs.next().await {
                                match s {
                                    Ok(TradeResponse{mut trade,sig,rpc_response,..}) => {
                                        trade.sell_ids.push(sig.clone());
                                        trade.sell_logs.push(rpc_response);
                                        trade.state = TradeState::PendingSell;
                                        if trade.state != TradeState::PositionClosed {
                                            trade.state = TradeState::PendingSell;
                                        }
                                        cache.insert(trade.id.clone(), trade.clone());

                                        poll_trade_and_cmd(
                                            signature,
                                            chan.clone(),
                                            trade.clone(),
                                            "close position".to_string(),
                                            |chan,trade,error|{
                                                chan.trade.try_send(
                                                    InternalCommand::SellTradeFail {
                                                        trade,
                                                        error:error.unwrap()
                                                    },
                                                ).unwrap()
                                            },
                                            |_chan,_trade,_|{
                                                // chan.trade.try_send(
                                                //     InternalCommand::SellTradeSuccess(trade),
                                                // ).unwrap()
                                            }
                                        );
                                    }
                                    Err(TradeResponseError{trade:_trade,error}) => {
                                        trade = _trade;
                                        chan.trade.try_send(
                                            InternalCommand::SellTradeFail {
                                                trade,
                                                error
                                            },
                                        ).unwrap();
                                    }
                                }
                            }
                        }
                    }
                    TradeState::PendingSell => {
                        if trade.sell_ids.contains(&signature.to_string()) {
                            trade.state = TradeState::PositionClosed;
                            trade.sell_id = Some(signature.to_string());
                            trade.sell_log = trade.sell_logs.iter().find(|x|&x.signature == &signature.to_string()).cloned();
                            trade.exit_time = Some(Utc::now());
                            trade.exit_price = Some(trade_price.price);
                            cache.insert(trade.id, trade.clone());

                            trade_log.trade = trade.clone();
                            chan.bg.try_send(InternalCommand::LogTrade(trade_log)).unwrap();
                        }
                    }
                    TradeState::PositionClosed => {
                        mints.remove(&trade.mint());
                        cache.remove(&trade.id);
                    }
                    _ => {}
                }
            }
            Some(InternalCommand::PumpMigration(InterestedTx{accounts,signature,logs,message})) => {
                trade_log.signature = Some(signature);

                if cache.len() >= max_positions {
                    info!("can't trade new, previous already in progress");
                    continue;
                }

                let accounts_maybe = message.instructions.iter().find_map(|x| {
                    if x.accounts.len() == 21 {
                        let keys_maybe = x
                            .accounts
                            .iter()
                            .filter_map(|&index| accounts.get(index as usize))
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

                if accounts_maybe.is_none() {
                    continue;
                }

                let valid_accounts= accounts_maybe.unwrap().into_iter().cloned().collect::<Vec<_>>();
                let ray_log = RayLog::from_log(logs.clone());
                let mut trade = Trade::from_solana_account_grpc(
                    valid_accounts.as_slice(),
                );
                trade_id += 1;
                trade.id = trade_id;
                trade.root_kp = kp.to_bytes().to_vec();
                let trade_price = trade.update_from_ray_log(&ray_log, price_id, false);
                trade_log.trade_price = Some(trade_price.clone());
                let price = trade_price.price;

                // DEAL WITH THIS LATER
                if &trade.user_wallet != &PUMP_MIGRATION.to_string() {
                    // info!("not a pump coin {:?} {:?}",signature, trade);
                    continue;
                }

                let trade_req = trade.create_position(price, cfg, true);


                let mut futs = FuturesUnordered::new();

                let rpcs = rpcs(rpcs_config);
                rpc_len = rpcs.len();
                rpcs.into_iter().for_each(|x|
                    futs.push(
                        tokio::spawn({
                            let trade_req = trade_req.clone();
                            async move {trade_req.clone().build_tx_and_send(x).await}
                        })
                    )
                );
                while let Some(Ok(s)) = futs.next().await {

                    match s {
                        Ok(TradeResponse{trade:_trade,sig,rpc_response,..}) => {
                            trade = cache.get(&_trade.id).cloned().unwrap_or(_trade);
                            let signature = solana_sdk::signature::Signature::from_str(&sig.as_str()).unwrap();
                            trade.buy_ids.push(sig);
                            trade.buy_logs.push(rpc_response);
                            trade.state = TradeState::PendingBuy;
                            cache.insert(trade.id, trade.clone());
                            mints.insert(trade.mint(), trade.clone());

                            poll_trade_and_cmd(
                                signature,
                                chan.clone(),
                                trade.clone(),
                                "open position".to_string(),
                                |chan,trade,error|{
                                    chan.trade.try_send(
                                        InternalCommand::BuyTradeFail {
                                            trade,
                                            error:error.unwrap()
                                        },
                                    ).unwrap()
                                },
                                |_chan,_trade,_|{
                                    // chan.trade.try_send(
                                    //     InternalCommand::BuyTradeSuccess(trade),
                                    // ).unwrap()
                                }
                            );
                        }
                        Err(TradeResponseError{trade:_trade,error}) => {
                            trade = _trade;
                            trade_log.error = Some(error.to_string());
                            chan.trade.try_send(
                                InternalCommand::BuyTradeFail {
                                    trade:trade.clone(),
                                    error
                                },
                            ).unwrap()
                        }
                    }
                    trade_log.trade = trade;
                }

                if trade_log.error.is_none() {
                    info!("{}",trade_log.console_log());
                }
            }
            _ => {}
        }
    }
}
