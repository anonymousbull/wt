use log::{error, info};
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use yellowstone_grpc_proto::geyser::subscribe_update::UpdateOneof;
use yellowstone_grpc_proto::geyser::{SubscribeUpdate, SubscribeUpdateTransaction, SubscribeUpdateTransactionInfo};
use yellowstone_grpc_proto::prelude::{Message, Transaction, TransactionStatusMeta};
use yellowstone_grpc_proto::tonic::Status;
use crate::cache::Cache;
use crate::constant::*;
use crate::event::InternalCommand;
use crate::state::TradeState;
use crate::trade::Trade;

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

