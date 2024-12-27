use std::cmp::{max, min};
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use chrono::Utc;
use futures::StreamExt;
use log::{error, info, warn};
use rust_decimal_macros::dec;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use solana_sdk::signer::Signer;
use tokio::sync::mpsc::Receiver;
use tokio::time::Instant;
use yellowstone_grpc_proto::geyser::{SubscribeRequest, SubscribeRequestFilterTransactions};
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use raydium_amm::log::{InitLog, SwapBaseInLog, SwapBaseOutLog};
use raydium_amm::math::SwapDirection;
use crate::cmd::InternalCommand;
use crate::constant::{get_keypair, pg_conn, solana_rpc_client, RAYDIUM_V4_PROGRAM, SOLANA_GRPC_URL, SOLANA_RPC_URL};
use crate::position::PositionConfig;
use crate::price::{get_price_tvl, Price};
use crate::send_tx::send_tx;
use crate::trade::{Trade, TradeState};

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


pub async fn bg_chan(
    mut rec: Receiver<InternalCommand>,
){
    let pg = pg_conn().await;
    let mut c = pg.get().await.unwrap();
    let mut insert_prices = Vec::<Price>::new();
    let mut update_trades = HashMap::<String, Trade>::new();
    let mut trade_time = Instant::now();
    while let Some(message) = rec.recv().await {
        match message {
            InternalCommand::InsertTrade(trade) => {
                trade.insert(&mut c).await.unwrap();
            }
            InternalCommand::InsertPrice(price) => {
                insert_prices.push(price);
                if insert_prices.len() > 20 {
                    Price::insert_bulk(&mut c, insert_prices).await.unwrap();
                    insert_prices = vec![];
                }
            }
            InternalCommand::UpdateTrade(trade) => {
                update_trades.insert(trade.id.clone(), trade);
                if trade_time.elapsed() >= Duration::from_secs(1) {
                    let trades = update_trades.values().cloned().collect::<Vec<_>>();
                    Trade::upsert_bulk(&mut c, trades).await.unwrap();
                    trade_time = Instant::now();
                }
            }
            _ => {}
        }
    }
}

pub async fn trade_chan(chan: Chan) {
    let pg = pg_conn().await;
    let mut c = pg.get().await.unwrap();
    let kp = get_keypair();
    let pubkey = kp.pubkey();
    let max_positions = 1;
    let mut price_id = Price::id(&mut c).await;
    let mut trades = Trade::get_all(&mut c)
        .await
        .into_iter()
        .filter(|x| x.exit_time.is_none())
        .map(|x| (x.id.clone(), x))
        .collect::<HashMap<_, _>>();

    let cfg = PositionConfig {
        min_sol: dec!(0.008),
        max_sol: dec!(0.01),
        fee: dec!(0.00203928),
        jito: dec!(0.001),
        close_trade_fee: dec!(0.00203928),
        priority_fee: dec!(7_00000), // 0.0007 SOL
    };
    let a = yellowstone_grpc_client::GeyserGrpcClient::build_from_static(SOLANA_GRPC_URL);
    let mut c = a.connect().await.unwrap();
    let rpc =
        RpcClient::new_with_commitment(SOLANA_RPC_URL.to_string(), CommitmentConfig::confirmed());
    let mut r = SubscribeRequest {
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

    // let mut ar = Vec::new();
    // loop {
    //     if ar.is_empty() {
    //         ar.push(async{}.boxed());
    //     }
    //     let (res, idx, remaining_futures) = select_all(ar).await;
    //     ar = remaining_futures;
    // }

    let (mut subscribe_tx, mut stream) = c.subscribe_with_request(Some(r.clone())).await.unwrap();


    while let Some(message) = stream.next().await {
        let is_trading = trades.len() < max_positions;
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
                                        .map(|a| Pubkey::try_from(a.as_slice()).unwrap())
                                        .collect::<Vec<_>>();
                                    let watch_trade = trades
                                        .keys()
                                        .find_map(|x| trades.get(x));
                                    if watch_trade.is_some() && meta.log_messages.iter().any(|x| x.contains(&"ray_log".to_string())) {
                                        let mut trade = watch_trade.unwrap().clone();

                                        let sig = solana_sdk::signature::Signature::try_from(
                                            t.signature.as_slice(),
                                        )
                                            .unwrap();
                                        info!("price swap {sig}");
                                        info!("accounts {:?}", accounts);

                                        let log = meta
                                            .log_messages
                                            .iter()
                                            .find(|x| x.contains("ray_log"))
                                            .unwrap();
                                        let log = log.replace("Program log: ray_log: ", "");

                                        let swap_in = bincode::deserialize::<SwapBaseInLog>(
                                            &BASE64_STANDARD.decode(log.clone()).unwrap(),
                                        );
                                        let swap_out = bincode::deserialize::<SwapBaseOutLog>(
                                            &BASE64_STANDARD.decode(log).unwrap(),
                                        );

                                        let (pc, coin) = if let Ok(swap) = swap_in {
                                            if swap.direction == SwapDirection::PC2Coin as u64 {
                                                info!("swap_base_in: pc2coin");
                                                info!(
                                                    "user swaps {} pc for {} coin",
                                                    swap.amount_in, swap.out_amount
                                                );
                                                let pc = swap.pool_pc - swap.amount_in;
                                                let coin = swap.pool_coin - swap.out_amount;
                                                (pc, coin)
                                            } else {
                                                info!("swap_base_in: coin2pc");
                                                info!(
                                                    "user swaps {} coin for {} pc",
                                                    swap.amount_in, swap.out_amount
                                                );
                                                let coin = swap.pool_coin - swap.amount_in;
                                                let pc = swap.pool_pc - swap.out_amount;
                                                (pc, coin)
                                            }
                                        } else if let Ok(swap) = swap_out {
                                            if swap.direction == 1 {
                                                info!("swap_base_out: pc2coin");
                                                info!(
                                                    "user swaps {} pc for {} coin",
                                                    swap.deduct_in, swap.amount_out
                                                );
                                                let pc = swap.pool_pc - swap.deduct_in;
                                                let coin = swap.pool_coin - swap.amount_out;
                                                (pc, coin)
                                            } else {
                                                info!("swap_base_out: coin2pc");
                                                info!(
                                                    "user swaps {} coin for {} pc",
                                                    swap.deduct_in, swap.amount_out
                                                );
                                                let coin = swap.pool_coin - swap.deduct_in;
                                                let pc = swap.pool_pc - swap.amount_out;
                                                (pc, coin)
                                            }
                                        } else {
                                            unreachable!()
                                        };

                                        let sol_amount = min(pc, coin);
                                        let token_amount = max(pc, coin);
                                        let trade_price_builder = get_price_tvl(
                                            sol_amount,
                                            token_amount,
                                            trade.decimals as u8,
                                        );
                                        let price =
                                            trade_price_builder.build(price_id, trade.id.clone()).price;

                                        if TradeState::PositionPendingFill == trade.state {
                                            if &sig.to_string() == trade.tx_in_id.as_ref().unwrap() {
                                                trade.entry_time = Some(Utc::now());
                                                trade.entry_price = Some(price);
                                                trade.state = TradeState::PositionFilled;
                                            }
                                        }

                                        if TradeState::PositionFilled == trade.state {
                                            let entry_price = trade.entry_price.unwrap();
                                            trade.pct = (price - entry_price) / entry_price * dec!(100);
                                            info!("trade:id-pct-price {} {} {}",trade.id,trade.pct,price);

                                            // trades.insert(trade.id.clone(), trade);

                                            if trade.pct > dec!(20) || trade.pct < dec!(-3) {
                                                let trade_req = trade.create_position(
                                                    price,
                                                    cfg,
                                                    false
                                                );

                                                warn!("spamming close");
                                                match send_tx(trade_req).await  {
                                                    Ok(trade_res) => {
                                                        let mut trade = trade_res.trade;
                                                        trade.state = TradeState::PositionPendingClose;
                                                        let sig = Signature::from_str(trade_res.sig.as_str()).unwrap();
                                                        trade.tx_out_id = Some(trade_res.sig);
                                                        tokio::spawn(async move {
                                                            let rpc = solana_rpc_client();
                                                            let res = rpc.poll_for_signature(&sig).await;
                                                            info!("close result {:?}",res);
                                                        });
                                                        trades.insert(trade.id.clone(), trade);
                                                    }
                                                    Err(e) => {
                                                        error!("could not close trade {e}");
                                                    }
                                                };
                                            }
                                        }

                                        else if TradeState::PositionPendingClose == trade.state {
                                            if accounts.contains(&pubkey) {
                                                trade.state = TradeState::PositionClosed;
                                                trade.exit_time = Some(Utc::now());
                                                trade.exit_price = Some(price);
                                                info!("closed {trade:?}");
                                                trades.insert(trade.id.clone(), trade.clone());
                                            }
                                        }

                                        let mut removes = vec![];
                                        trades.iter().for_each(|(id,trade)| {
                                            chan.bg.try_send(InternalCommand::UpdateTrade(trade.clone())).unwrap();
                                            if TradeState::PositionClosed == trade.state {
                                                removes.push(id);
                                            }
                                        });
                                        removes.iter().for_each(|&id| {
                                            trades.remove(id);
                                        });
                                    }
                                }
                            }

                            if is_trading
                                && meta
                                .log_messages
                                .iter()
                                .any(|z| z.contains("init_pc_amount"))
                            {
                                info!("found");

                                if let Some(tx) = &t.transaction {
                                    if let Some(m) = &tx.message {
                                        let accounts = m
                                            .instructions
                                            .iter()
                                            .find_map(|x| {
                                                if x.accounts.len() == 21
                                                    && m.account_keys.len() >= 21
                                                {
                                                    Some(
                                                        x.accounts
                                                            .iter()
                                                            .map(|&index| {
                                                                Pubkey::try_from(
                                                                    m.account_keys[index as usize]
                                                                        .as_slice(),
                                                                )
                                                                    .unwrap()
                                                            })
                                                            .collect::<Vec<_>>(),
                                                    )
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap();

                                        // .and_then(|x|{
                                        //     if x.contains(&PUMP_MIGRATION) {
                                        //         Some(x)
                                        //     } else {
                                        //         None
                                        //     }
                                        // });

                                        info!("{:?}", meta.log_messages);
                                        let log = meta
                                            .log_messages
                                            .iter()
                                            .find(|x| x.contains("ray_log"))
                                            .unwrap();
                                        let log = log.replace("Program log: ray_log: ", "");
                                        info!("ray_log {log}");
                                        let sig = solana_sdk::signature::Signature::try_from(
                                            t.signature.as_slice(),
                                        )
                                            .unwrap();
                                        info!("sig {:?}", sig);
                                        let init_log = bincode::deserialize::<InitLog>(
                                            &BASE64_STANDARD.decode(log).unwrap(),
                                        )
                                            .unwrap();
                                        info!("accounts {:?}", accounts);

                                        let trade =
                                            Trade::from_solana_account_grpc(accounts.as_slice());
                                        let trade_id = trade.id.clone();
                                        info!("trade:init {:?}", trade);

                                        // THIS IS RISKY
                                        let price_tvl_builder = get_price_tvl(
                                            min(init_log.pc_amount, init_log.coin_amount),
                                            max(init_log.pc_amount, init_log.coin_amount),
                                            if init_log.pc_decimals == 9 {
                                                init_log.coin_decimals
                                            } else {
                                                init_log.pc_decimals
                                            },
                                        );

                                        price_id += 1;
                                        let price_tvl =
                                            price_tvl_builder.build(price_id, trade.id.clone());
                                        let price = price_tvl.price;
                                        chan.bg.try_send(InternalCommand::InsertPrice(price_tvl)).unwrap();
                                        let trade_req = trade.create_position(
                                            price,
                                            cfg,
                                            true
                                        );
                                        let mut trade_res = send_tx(trade_req).await.unwrap();
                                        chan.bg.try_send(InternalCommand::InsertTrade(trade_res.trade.clone())).unwrap();
                                        info!("position accepted {:?}", trade_res.sig);
                                        trade_res.trade.tx_in_id = Some(trade_res.sig);
                                        trades.insert(trade_id, trade_res.trade);
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