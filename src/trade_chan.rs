use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, PUMP_EVENT_AUTHORITY, PUMP_FEE, PUMP_MIGRATION, PUMP_MIGRATION_PRICE, PUMP_PROGRAM, RAYDIUM_V4_AUTHORITY, RAYDIUM_V4_PROGRAM, SOLANA_ATA_PROGRAM, SOLANA_MINT, SOLANA_RENT_PROGRAM, SOLANA_SERUM_PROGRAM, SOLANA_SYSTEM_PROGRAM};
use crate::position::PositionConfig;
use crate::program_log::{ProgramLog, ProgramLogInfo};
use crate::rpc::{geyser, rpcs, solana_rpc_client, RpcResponseData, RpcState, RpcsConfig, TradeRpcLogStatus};
use crate::solana::*;
use crate::trade::{Trade, TradePrice, TradeResponse, TradeResponseError, TradeSignature, TradeState};
use anyhow::anyhow;
use chrono::{DateTime, Utc};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::{error, info, warn};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;
use tokio::fs::OpenOptions;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{CommitmentLevel, SubscribeRequest, SubscribeRequestFilterTransactions, SubscribeUpdateTransactionInfo};
use yellowstone_grpc_proto::prelude::{
    Message, SubscribeUpdate, SubscribeUpdateTransaction, Transaction, TransactionStatusMeta,
};
use yellowstone_grpc_proto::tonic::Status;
use crate::db::{get_ignore_mints, update_ignore_mints};

#[derive(Debug)]
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
            meta: meta.clone(),
        };
        let watch_trades = interested_tx
            .accounts
            .iter()
            .find_map(|x| mints.get(&x).and_then(|x| cache.get(&x.id)));

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
        if let Some(trade) = watch_trades {
            info!("trade mint = {}",trade.mint());
            let mut trade = trade.clone();
            let mut log = String::new();

            if let Some(l) = ray_log_maybe {
                log = l.clone();
            } else if let Some(l) = pump_log_maybe {
                log = l.clone();
            }

            let trade = trade.update_price_from_log(log, true);
            match trade {
                Ok(trade) => {
                    Some(InternalCommand::TradeUpdate {
                        trade,
                        interested_tx,
                    })
                }
                Err(e) => {
                    error!("{e}");
                    None
                }
            }
        } else if let Some(pump_log) = pump_log_maybe {
            interested_tx.logs = pump_log.clone();
            Some(InternalCommand::PumpSwapMaybe(interested_tx))
        } else if let Some(ray_log) = ray_log_maybe {
            interested_tx.logs = ray_log.clone();
            if meta
                .log_messages
                .iter()
                .any(|z| z.contains("init_pc_amount"))
            {
                Some(InternalCommand::RaydiumInit(interested_tx))
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

fn poll_trade_and_cmd<F>(
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
            info!("{log_action} poll success");
        }
    });
}




#[derive(Serialize, Deserialize, Debug, Default)]
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
        .open("trades.txt")
        .await
        .unwrap();
    let mut contents = String::new();
    trades_file.read_to_string(&mut contents).await.unwrap();
    let trades = serde_json::from_str::<Vec<Trade>>(&contents).unwrap_or(vec![]);
    let mut trade_id = trades.len() as i64;
    // let pumps
    let mut cache = trades
        .into_iter()
        .filter(|x| x.sell_time.is_none())
        .map(|x| (x.id.clone(), x))
        .collect::<HashMap<_, _>>();
    let mut mints = cache
        .values()
        .map(|x| (x.mint(), x.clone()))
        .collect::<HashMap<_, _>>();
    info!("active trades {}", cache.len());

    let cfg = PositionConfig {
        max_sol: dec!(0.008),
        ata_fee: dec!(0.00203928),
        max_jito: dec!(0.001),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(7_00_000), // 0.0007 SOL
        base_fee: dec!(0.000005),
        jito_priority_percent: dec!(0.7),
        jito_tip_percent: dec!(0.3),
        cu_limit: Default::default(),
        slippage: dec!(0.2),
        ray_buy_gas_limit: 60_000,
        ray_sell_gas_limit: 60_000,
        pump_buy_gas_limit: 100_000,
        pump_sell_gas_limit: 100_000,
        tp: dec!(0.05),
        fee_pct: dec!(0.3),
    };
    let geyser = geyser();

    let ignore_mints = get_ignore_mints().await;
    let mut c = geyser.connect().await.unwrap();
    info!("connected to geyser");

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
    let mut rpc_state = RpcState::Free;
    let (_subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();

    while let Some(message) = stream.next().await {
        if heartbeat.elapsed().as_secs() > 60 {
            info!("heartbeat {:?}", heartbeat.elapsed());
            heartbeat = Instant::now();
        }

        if let Ok(cmd) = rec.try_recv() {
            match cmd {
                InternalCommand::RpcTradeResponse(res) => {
                    match res {
                        Ok(mut trade) => {
                            let Trade{rpc_logs,..} = cache.get(&trade.id).cloned().unwrap_or(trade.clone());
                            let signature = trade.tx_id_unchecked().unwrap();

                            trade.rpc_logs.extend_from_slice(&rpc_logs);
                            trade.rpc_logs.dedup();
                            cache.insert(trade.id, trade.clone());
                            mints.insert(trade.mint(), trade.clone());
                            poll_trade_and_cmd(
                                signature,
                                chan.clone(),
                                trade.clone(),
                                "position".to_string(),
                                |chan, mut trade, error| {
                                    trade.error = Some(error.unwrap().to_string());
                                    chan.trade
                                        .try_send(InternalCommand::RpcTradeResponse(Err(trade)))
                                        .unwrap();
                                },
                            );
                        }
                        Err(mut trade) => {
                            let Trade{mut attempts,rpc_logs,..} = cache.get(&trade.id).unwrap().clone();

                            trade.attempts = attempts + 1;
                            trade.rpc_logs.extend_from_slice(&rpc_logs);

                            cache.insert(trade.id, trade.clone());
                            mints.insert(trade.mint(), trade.clone());

                            if trade.attempts == rpc_len {
                                mints.remove(&trade.mint());
                                cache.remove(&trade.id);
                                error!("{:?}",trade.console_log());
                            }
                            chan.bg
                                .try_send(InternalCommand::LogTrade(trade))
                                .unwrap();
                        }
                    }
                }
                _ => {}
            }
        }

        let maybe_cmd = decode_tx(message, &cache, &mints);
        match maybe_cmd {
            Some(InternalCommand::TradeUpdate { mut trade, interested_tx: InterestedTx { signature, .. }, }) => {

                if let ProgramLogInfo::RayWithdraw(q) = &trade.plog.log {
                    warn!("{q:?}");
                    continue;
                }

                let amount_out = trade.plog.amount_out;
                price_id += 1;
                let trade_price = trade.to_trade_price(price_id);

                chan.bg
                    .try_send(InternalCommand::InsertPrice(trade_price.clone()))
                    .unwrap();

                let rpc_signatures = trade.rpc_logs.iter().map(|x|{
                    x.signature()
                }).collect::<Vec<_>>();

                match trade.state {
                    TradeState::PendingBuy => {
                        if rpc_signatures.contains(&signature) {

                            trade.buy_out = Some(trade.plog.amount_out);
                            // trade.signatures.push(TradeSignature::BuySuccess(signature.to_string()));
                            trade.buy_time = Some(Utc::now());
                            trade.buy_price = Some(trade_price.price);
                            trade.state = TradeState::BuySuccess;
                            trade.amount = Decimal::from(amount_out);
                            trade.rpc_logs.iter_mut().for_each(|x|{
                                x.change_state(signature, TradeRpcLogStatus::Success);
                            });
                            cache.insert(trade.id, trade.clone());

                            chan.bg
                                .try_send(InternalCommand::LogTrade(trade.clone()))
                                .unwrap();
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

                        info!("checking profit");
                        let buy_price = trade.buy_price.unwrap();
                        trade.pct = (trade.price - buy_price) / buy_price * dec!(100);
                        cache.insert(trade.id, trade.clone());

                        chan.bg
                            .try_send(InternalCommand::LogTrade(trade.clone()))
                            .unwrap();


                        if trade.pct > dec!(5) || trade.pct < dec!(-1) {
                            info!("buy_price = {buy_price}, price={}, pct={}",trade.price,trade.pct);
                            info!("{}",trade.console_log());
                            info!("{:?}",&cache.get(&trade.id).unwrap());

                            trade = trade.build_instructions();

                            warn!("spamming close");
                            tokio::spawn({
                                let chan = chan.clone();
                                async move {
                                    let mut r = trade.send_many(rpcs(rpcs_config)).await;
                                    while let Some(Ok(res)) = r.next().await {
                                        chan.trade.try_send(InternalCommand::RpcTradeResponse(res)).unwrap();
                                    }
                                }
                            });
                        }
                    }
                    TradeState::PendingSell => {
                        if rpc_signatures.contains(&signature) {
                            trade.state = TradeState::SellSuccess;
                            trade.sell_time = Some(Utc::now());
                            trade.sell_price = Some(trade_price.price);
                            cache.insert(trade.id, trade.clone());
                            chan.bg
                                .try_send(InternalCommand::LogTrade(trade.clone()))
                                .unwrap();
                        }
                    }
                    TradeState::SellSuccess => {
                        mints.remove(&trade.mint());
                        cache.remove(&trade.id);
                    }
                    _ => {}
                }
            }
            Some(InternalCommand::RaydiumInit(event)) => {
                if rpc_state == RpcState::Busy {
                    continue;
                }

                if cache.len() >= max_positions {
                    info!("can't trade new, previous already in progress");
                    continue;
                }

                let accounts_maybe = event.try_ray_init2();
                if accounts_maybe.is_none() {
                    continue;
                }

                let valid_accounts = accounts_maybe
                    .unwrap()
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();

                let InterestedTx {
                    signature,
                    logs,
                    message,
                    ..
                } = event;

                let mut trade = Trade::from_raydium_init(
                    valid_accounts.as_slice(),
                    cfg,
                    logs
                ).unwrap();

                trade_id += 1;
                trade.id = trade_id;
                trade.root_kp = kp.to_bytes().to_vec();


                let trade_price = trade.to_trade_price(price_id);
                let price = trade_price.price;

                // DEAL WITH THIS LATER
                if &trade.user_wallet != &PUMP_MIGRATION.to_string() {
                    // info!("not a pump coin {:?} {:?}",signature, trade);
                    continue;
                }



                let mut trade = trade.build_instructions();
                tokio::spawn({
                    let chan = chan.clone();
                    async move {
                        let mut r = trade.send_many(rpcs(rpcs_config)).await;
                        while let Some(Ok(res)) = r.next().await {
                            chan.trade.try_send(InternalCommand::RpcTradeResponse(res)).unwrap();
                        }
                    }
                });
            }
            Some(InternalCommand::PumpSwapMaybe(event)) => {

                if rpc_state == RpcState::Busy {
                    continue;
                }

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


                let mut trade = Trade::from_pump_swap(
                    valid_accounts.as_slice(),
                    cfg,
                    logs
                );
                if let Err(e) = trade {
                    error!("{e}");
                    continue;
                }
                let mut trade = trade.unwrap();


                if ignore_mints.contains(&trade.mint()) {
                    continue;
                }

                // info!("program log {:?} {}", program_log, signature.to_string());
                trade.root_kp = kp.to_bytes().to_vec();

                info!("price {}",trade.price);
                // info!("pump price = {}",trade.price);
                if trade.price.to_f64().unwrap() >= PUMP_MIGRATION_PRICE * 0.50 {

                    info!("time to buy bro {} {}",trade.price,signature.to_string());
                    let mut trade = trade.build_instructions();
                    rpc_state = RpcState::Busy;
                    tokio::spawn({
                        let mint = trade.mint();
                        async move {
                            update_ignore_mints(mint.to_string()).await;
                        }
                    });
                    tokio::spawn({
                        let chan = chan.clone();
                        async move {
                            let mut r = trade.send_many(rpcs(rpcs_config)).await;
                            while let Some(Ok(res)) = r.next().await {
                                chan.trade.try_send(InternalCommand::RpcTradeResponse(res)).unwrap();
                            }
                        }
                    });

                }
            }
            _ => {}
        }
    }
}
