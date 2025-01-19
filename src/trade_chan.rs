use crate::chan::Chan;
use crate::constant::*;
use crate::plog::*;
use crate::swap_config::*;
use crate::trade::*;
use crate::trade_cache::*;
use crate::trade_cmd::*;
use crate::trade_rpc::*;
use anyhow::anyhow;
use chrono::Utc;
use futures::StreamExt;
use log::*;
use mongodb::Collection;
use raydium_amm::solana_program::pubkey::Pubkey;
use rdkafka::producer::FutureProducer;
use redis::aio::ConnectionManager;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use solana_sdk::signature::{Keypair, Signature};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::{CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeUpdate, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::prelude::{Message, Transaction, TransactionStatusMeta};
use yellowstone_grpc_proto::tonic::Status;

pub struct Engine {
    pub red: ConnectionManager,
    pub kafka: FutureProducer,
    pub mon: Collection<Trade>,
    pub db: Cache,
    pub chan: Chan,
}

pub fn swap(chan: Chan, mut trade: Trade, cache: &mut Cache) {
    trade.update_rpc_status(RpcState::Busy);
    if trade.state == TradeState::Buy {
        trade.state = TradeState::PendingBuy;
    } else {
        trade.state = TradeState::PendingSell;
    }
    cache.upsert(&trade);
    trade = trade.rebuild_instructions();

    let rpcs_config = RpcsConfig { jito_tip: None };
    tokio::spawn({
        async move {
            let mut r = trade.send_many(rpcs(rpcs_config)).await;
            while let Some(Ok(res)) = r.next().await {
                chan.trade
                    .try_send(InternalCommand::RpcTradeResponse(res))
                    .unwrap();
            }
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
    let mut heartbeat = Instant::now();
    let m = mongo().await;
    let red = redis_pool().await;
    let k = kakfa_producer().await;
    let c = m.collection::<Trade>("trades");
    let max_positions = 1;
    let mut price_id = 0;
    let trades = Trade::db_get_active_trades(&c)
        .await
        .iter()
        .map(|x| (x.id.clone(), x.clone()))
        .collect::<HashMap<_, _>>();

    let mut mints = HashMap::new();
    for x in trades.values() {
        mints
            .entry(x.mint())
            .or_insert_with(HashMap::new)
            .insert(x.id, x.clone());
    }
    let mut db = Cache {
        trades,
        mints,
        oneshots: Default::default(),
    };

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
        slippage: dec!(0.05),
        ray_buy_gas_limit: 60_000,
        ray_sell_gas_limit: 60_000,
        pump_buy_gas_limit: 80_000,
        pump_sell_gas_limit: 80_000,
        tp: dec!(0.1),
        sl: dec!(-0.01),
        fee_pct: dec!(0.2),
    };
    let geyser = geyser();

    // let mut ignore_mints = get_ignore_mints().await;
    let mut c = geyser.connect().await.unwrap();
    info!("connected to geyser");
    tokio::time::sleep(Duration::from_secs(5)).await;
    let r = SubscribeRequest {
        commitment: Some(i32::from(CommitmentLevel::Processed)),
        transactions: vec![(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                account_required: vec![PUMP_FEE.to_string()],
                failed: Some(false),
                vote: Some(false),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    let mut rpcs_config = RpcsConfig { jito_tip: None };
    let mut rpc_len = rpcs(rpcs_config).len();
    // let mut rpc_state = RpcState::Free;
    let (_subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();

    while let Some(message) = stream.next().await {
        if heartbeat.elapsed().as_secs() > 60 {
            info!("heartbeat {:?}", heartbeat.elapsed());
            heartbeat = Instant::now();
        }

        if let Ok(cmd) = rec.try_recv() {
            match cmd {
                InternalCommand::SwapRequest(trade) => {
                    swap(chan.clone(), trade, &mut db);
                }
                InternalCommand::SwapLimitRequest(trade, _) => {
                    db.upsert(&trade);
                }
                InternalCommand::RpcTradeResponse(res) => match res {
                    Ok(mut updated_trade) => {
                        let mut old_trade = db.get_by_trade(&updated_trade).cloned().unwrap();
                        old_trade.extend_rpc_logs(&updated_trade.rpc_logs);
                        let signature = updated_trade.signature_unchecked();

                        db.upsert(&updated_trade);

                        poll_trade_and_emit(
                            signature,
                            chan.clone(),
                            updated_trade.clone(),
                            "position".to_string(),
                            |chan, mut trade, error| {
                                trade.error = Some(error.unwrap().to_string());
                                chan.trade
                                    .try_send(InternalCommand::RpcTradeResponse(Err(trade)))
                                    .unwrap();
                            },
                        );
                    }
                    Err(mut update) => {
                        let mut trade = db.get_by_trade(&update).unwrap().clone();
                        trade.extend_rpc_logs(&update.rpc_logs);

                        db.upsert(&update);

                        // pro version will have this
                        if trade.rpc_logs.len() == rpc_len {
                            db.remove(&update);
                            chan.bg.try_send(InternalCommand::TradeConfirmation(trade)).unwrap();
                            error!("{:?}", update.console_log());
                        }
                    }
                },
                _ => {}
            }
        }

        let commands = decode_tx(message, &db);
        for cmd in commands {
            match cmd {
                InternalCommand::TradeUpdate {
                    mut trade,
                    interested_tx: InterestedTx { signature, .. },
                } => {
                    if let ProgramLogInfo::RayWithdraw(q) = &trade.plog.log {
                        warn!("{q:?}");
                        continue;
                    }

                    let amount_out = trade.plog.amount_out;
                    price_id += 1;
                    let trade_price = trade.to_trade_price(price_id);

                    let rpc_signatures = trade
                        .rpc_logs
                        .iter()
                        .map(|x| x.signature())
                        .collect::<Vec<_>>();

                    match trade.state {
                        TradeState::PendingBuy => {
                            if rpc_signatures.contains(&signature) {
                                trade.buy_out = Some(trade.plog.amount_out);
                                // trade.signatures.push(TradeSignature::BuySuccess(signature.to_string()));
                                trade.buy_time = Some(Utc::now());
                                trade.buy_price = Some(trade_price.price);
                                trade.state = TradeState::BuySuccess;
                                trade.amount = Decimal::from(amount_out);

                                let logs = trade
                                    .clone()
                                    .rpc_logs
                                    .iter_mut()
                                    .map(|x| {
                                        x.change_state(signature, TradeRpcLogStatus::Success);
                                        x.clone()
                                    })
                                    .collect::<Vec<_>>();
                                trade.rpc_logs = logs;

                                db.upsert(&trade);

                                chan.bg
                                    .try_send(InternalCommand::TradeConfirmation(trade.clone()))
                                    .unwrap();
                                // let _ = chan.bg.try_send(InternalCommand::LogTrade(trade.clone()));
                                // chan.ws.try_send(InternalCommand::UpdateTrade(trade.clone())).unwrap();

                                info!("trade buy success");
                                info!("{}", trade.console_log());

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
                            let buy_price = trade.buy_price.unwrap();
                            trade.pct = (trade.price - buy_price) / buy_price;
                            db.upsert(&trade);

                            if trade.pct > trade.cfg.tp || trade.pct < trade.cfg.sl {
                                info!(
                                    "buy_price = {buy_price}, price={},   pct={}",
                                    trade.price, trade.pct
                                );
                                info!("{}", trade.console_log());
                                info!("{:?}", &db.get_by_trade(&trade).unwrap());

                                warn!("spamming close");
                                swap(chan.clone(), trade, &mut db);
                            }
                        }
                        TradeState::PendingSell => {
                            if rpc_signatures.contains(&signature) {
                                trade.state = TradeState::SellSuccess;
                                trade.sell_time = Some(Utc::now());
                                trade.sell_price = Some(trade_price.price);
                                db.upsert(&trade);
                                // chan.bg.try_send(InternalCommand::LogTrade(trade.clone()));
                            }
                        }
                        TradeState::SellSuccess => {
                            info!("trade sell success");
                            let key = trade.mint();
                            let mints_empty = false;
                            db.remove(&trade);
                            chan.bg
                                .try_send(InternalCommand::TradeConfirmation(trade.clone()))
                                .unwrap();
                        }
                        _ => {}
                    }
                }
                // InternalCommand::RaydiumInit(event) => {
                //
                //     if rpc_state == RpcState::Busy {
                //         continue;
                //     }
                //
                //     // if db.len() >= max_positions {
                //     //     info!("can't trade new, previous already in progress");
                //     //     continue;
                //     // }
                //
                //     let accounts_maybe = event.try_ray_init2();
                //     if accounts_maybe.is_none() {
                //         continue;
                //     }
                //
                //     let valid_accounts = accounts_maybe
                //         .unwrap()
                //         .into_iter()
                //         .cloned()
                //         .collect::<Vec<_>>();
                //
                //     let InterestedTx {
                //         signature,
                //         logs,
                //         message,
                //         ..
                //     } = event;
                //
                //     let mut trade =
                //         trade.with_raydium_init(valid_accounts.as_slice(), cfg, logs).unwrap();
                //
                //     trade_id += 1;
                //     trade.id = trade_id;
                //     trade.root_kp = kp.to_bytes().to_vec();
                //
                //     let trade_price = trade.to_trade_price(price_id);
                //     let price = trade_price.price;
                //
                //     // DEAL WITH THIS LATER
                //     if &trade.user_wallet != &PUMP_MIGRATION.to_string() {
                //         // info!("not a pump coin {:?} {:?}",signature, trade);
                //         continue;
                //     }
                //
                //     let mut trade = trade.build_instructions();
                //     tokio::spawn({
                //         let chan = chan.clone();
                //         async move {
                //             let mut r = trade.send_many(rpcs(rpcs_config)).await;
                //             while let Some(Ok(res)) = r.next().await {
                //                 chan.trade
                //                     .try_send(InternalCommand::RpcTradeResponse(res))
                //                     .unwrap();
                //             }
                //         }
                //     });
                // }
                InternalCommand::PumpSwapMaybe {
                    trade,
                    interested_tx: event,
                } => {
                    let accounts_maybe = event.try_pump_swap();
                    if accounts_maybe.is_none() {
                        // info!("skipping {:?} {}",event.accounts, event.signature.to_string());
                        continue;
                    }

                    let valid_accounts = accounts_maybe.unwrap().into_iter().collect::<Vec<_>>();
                    let InterestedTx {
                        signature,
                        logs,
                        message,
                        ..
                    } = event;

                    let mut trade = trade.with_pump_swap(valid_accounts.as_slice(), logs);
                    if let Err(e) = trade {
                        error!("{e}");
                        continue;
                    }
                    let mut trade = trade.unwrap();
                    chan.bg
                        .try_send(InternalCommand::PriceUpdate(trade.clone()))
                        .unwrap();

                    if trade.is_admin() {
                        continue;
                    }
                    // info!("program log {:?} {}", program_log, signature.to_string());

                    info!("price {}", trade.price);
                    // info!("pump price = {}",trade.price);

                    // // To introduce sending dynamic rust code
                    // let mut buy = false;
                    // if let ProgramLogInfo::PumpTradeLog(d) = trade.plog.log {
                    //     if d.sol_amount >= LAMPORTS_PER_SOL*1 &&
                    //         d.token_amount.to_f64().unwrap() >= (37951768488745.0 * 0.95)  &&
                    //         d.token_amount.to_f64().unwrap() >= (37951768488745.0 * 1.05)
                    //     {
                    //         buy = true;
                    //     }
                    // }

                    if trade.rpc_status == RpcState::Free && trade.state == TradeState::Buy {
                        info!("time to buy bro {} {}", trade.price, signature.to_string());
                        swap(chan.clone(), trade, &mut db);
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn poll_trade_and_emit<F>(
    signature: Signature,
    chan: Chan,
    trade: Trade,
    log_action: String,
    handle_err: F,
) where
    F: Fn(Chan, Trade, Option<anyhow::Error>) + Send + 'static,
{
    tokio::spawn(async move {
        let rpc = solana_rpc_client();
        let res = rpc.poll_for_signature(&signature).await;
        let is_poll_err = if res.is_err() {
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
            handle_err(chan, trade, e);
        } else {
            // trade.db_upsert_mongo()
            info!("{log_action} poll success");
        }
    });
}

#[derive(Debug, Clone)]
pub struct InterestedTx {
    pub signature: Signature,
    pub accounts: Vec<Pubkey>,
    pub logs: String,
    pub message: Message,
    meta: TransactionStatusMeta,
}

impl InterestedTx {
    pub fn try_ray_init2(&self) -> Option<Vec<&Pubkey>> {
        self.message.instructions.iter().find_map(|x| {
            if x.accounts.len() == 21 {
                let keys_maybe = x
                    .accounts
                    .iter()
                    .filter_map(|&index| self.accounts.get(index as usize))
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
        })
    }
    pub fn try_pump_swap(&self) -> Option<Vec<Pubkey>> {
        let mut maybe_keys = vec![];

        for inners in &self.meta.inner_instructions {
            for inner in &inners.instructions {
                let mut a = vec![];
                for index in &inner.accounts {
                    if let Some(v) = self.accounts.get(*index as usize) {
                        a.push(v);
                    }
                }
                if a.len() >= 12 {
                    maybe_keys.push(a);
                }
            }
        }

        for comp in &self.message.instructions {
            let mut a = vec![];
            for index in &comp.accounts {
                if let Some(v) = self.accounts.get(*index as usize) {
                    a.push(v);
                }
            }
            if a.len() >= 12 {
                maybe_keys.push(a);
            }
        }

        let mut resp = None;

        for accounts in maybe_keys {
            let mut correct = vec![];
            let mut has_rent_program = false;
            let mut has_system_program = false;
            let mut has_pump_fee = false;
            let mut has_pump_program = false;
            let mut has_pump_event_auth_program = false;
            let mut has_token_program = false;
            for x in accounts {
                correct.push(*x);
                if x == &SOLANA_SYSTEM_PROGRAM {
                    has_system_program = true
                } else if x == &SOLANA_RENT_PROGRAM {
                    has_rent_program = true
                } else if x == &PUMP_FEE {
                    has_pump_fee = true
                } else if x == &PUMP_PROGRAM {
                    has_pump_program = true
                } else if x == &PUMP_EVENT_AUTHORITY {
                    has_pump_event_auth_program = true
                } else if x == &spl_token::id() || x == &spl_token_2022::id() {
                    has_token_program = true
                }
            }
            let all_good = has_system_program
                && has_pump_event_auth_program
                && has_pump_fee
                && has_rent_program
                && has_pump_program
                && has_pump_event_auth_program
                && has_token_program
                && correct[11] == PUMP_PROGRAM
                && correct[10] == PUMP_EVENT_AUTHORITY
                && correct.len() == 12;

            if all_good {
                // if resp.is_some() {
                //     info!("again again again {:?} {:?}", correct, resp.unwrap());
                // }
                resp = Some(correct.clone());
            }
        }
        resp
    }
}

pub fn decode_tx(update: Result<SubscribeUpdate, Status>, db: &Cache) -> Vec<InternalCommand> {
    let mut txs = vec![];
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
            meta: meta.clone(),
        };
        let root_trade = Trade::get_internal_trade();
        let mut watch_trades = interested_tx
            .accounts
            .iter()
            .filter_map(|x| db.get_trades_by_mint(x))
            .flatten()
            .collect::<Vec<_>>();

        watch_trades.push(&root_trade);

        // DO NOT SUPPORT RAY AND PUMP IN SAME TX
        let mut ray_log_maybe = None;
        let mut pump_log_maybe = None;
        for x in &meta.log_messages {
            if x.contains("ray_log") {
                ray_log_maybe = Some(x);
                break;
            } else if x.contains("vdt/007mYe") {
                pump_log_maybe = Some(x);
                break;
            }
        }

        let mut log = String::new();
        if let Some(l) = ray_log_maybe {
            log = l.clone();
        } else if let Some(l) = pump_log_maybe {
            log = l.clone();
        }

        if log.is_empty() {
            return vec![];
        }

        for x in watch_trades {
            let mut trade = x.clone();

            let trade = trade.update_price_from_log(log.clone(), true);
            match trade {
                Ok(trade) => match trade.state {
                    TradeState::Buy => {
                        if let Some(pump_log) = pump_log_maybe {
                            interested_tx.logs = pump_log.clone();
                            txs.push(InternalCommand::PumpSwapMaybe {
                                trade,
                                interested_tx: interested_tx.clone(),
                            });
                        } else if let Some(ray_log) = ray_log_maybe {
                            interested_tx.logs = ray_log.clone();
                            if meta
                                .log_messages
                                .iter()
                                .any(|z| z.contains("init_pc_amount"))
                            {
                                txs.push(InternalCommand::PumpSwapMaybe {
                                    trade,
                                    interested_tx: interested_tx.clone(),
                                });
                            }
                        }
                    }
                    TradeState::PendingBuy
                    | TradeState::BuySuccess
                    | TradeState::PendingSell
                    | TradeState::SellSuccess => {
                        txs.push(InternalCommand::TradeUpdate {
                            trade,
                            interested_tx: interested_tx.clone(),
                        });
                    }
                    _ => unreachable!(),
                },
                Err(e) => {
                    error!("{e}");
                }
            }
        }
    }
    txs
}
