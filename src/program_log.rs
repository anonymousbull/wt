use serde::{Deserialize, Serialize};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{DateTime, Utc};
use log::info;
use rust_decimal::Decimal;
use solana_sdk::pubkey::Pubkey;
use raydium_amm::log::{InitLog, LogType, SwapBaseInLog, SwapBaseOutLog, WithdrawLog};
use raydium_amm::math::SwapDirection;
use crate::constant::PUMP_BUY_CODE;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ProgramLog {
    pub next_pc: u64,
    pub next_coin: u64,
    pub amount_out: u64,
    pub pc: u64,
    pub coin: u64,
    pub log: ProgramLogInfo
}

#[derive(Debug, Clone, Serialize, Deserialize,Default)]
pub enum ProgramLogInfo {
    RaySwapBaseIn(SwapBaseInLog),
    RaySwapBaseOut(SwapBaseOutLog),
    RayWithdraw(WithdrawLog),
    RayInitLog(InitLog),
    PumpTradeLog(PumpTradeLog),
    #[default]
    Empty
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PumpTradeLog {
    pub mint:Pubkey,
    pub sol_amount:u64,
    pub token_amount:u64,
    pub is_buy:bool,
    pub user:Pubkey,
    pub timestamp:i64,
    pub virtual_sol_reserves:u64,
    pub virtual_token_reserves:u64,
}



impl ProgramLog {
    pub fn from_ray(log: String) -> Self {
        let log = log.replace("Program log: ray_log: ", "");
        let bytes = BASE64_STANDARD.decode(log).unwrap();
        let swap_info = match LogType::from_u8(bytes[0]) {
            LogType::Withdraw => {
                let withdraw = bincode::deserialize::<WithdrawLog>(&bytes).unwrap();
                // https://solscan.io/tx/56rnYcib58eJc1EsZJ8RiWGyQyWoVv5VihZUvyy2vrbG84PCT6iEcE5soR9pMQH6uGVp1XfosaVFHDtnWkbyMAGx

                let next_coin = withdraw.pool_coin - withdraw.out_coin;
                let next_pc = withdraw.pool_pc - withdraw.out_pc;
                ProgramLog {
                    next_pc,
                    next_coin,
                    amount_out: 0,
                    pc: withdraw.pool_pc,
                    coin: withdraw.pool_coin,
                    log: ProgramLogInfo::RayWithdraw(withdraw),
                }
            }
            LogType::SwapBaseIn => {
                let swap = bincode::deserialize::<SwapBaseInLog>(&bytes).unwrap();
                if swap.direction == SwapDirection::PC2Coin as u64 {
                    let next_pc = swap.pool_pc + swap.amount_in;
                    let next_coin = swap.pool_coin - swap.out_amount;
                    ProgramLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.out_amount,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: ProgramLogInfo::RaySwapBaseIn(swap),
                    }
                } else {
                    let coin = swap.pool_coin + swap.amount_in;
                    let pc = swap.pool_pc - swap.out_amount;
                    ProgramLog {
                        next_pc: pc,
                        next_coin: coin,
                        amount_out: swap.out_amount,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: ProgramLogInfo::RaySwapBaseIn(swap),

                    }
                }
            }
            LogType::SwapBaseOut => {
                let swap = bincode::deserialize::<SwapBaseOutLog>(&bytes).unwrap();
                if swap.direction == 1 {
                    let next_pc = swap.pool_pc + swap.deduct_in;
                    let next_coin = swap.pool_coin - swap.amount_out;
                    ProgramLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.amount_out,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: ProgramLogInfo::RaySwapBaseOut(swap),
                    }
                } else {
                    let next_coin = swap.pool_coin + swap.deduct_in;
                    let next_pc = swap.pool_pc - swap.amount_out;
                    ProgramLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.amount_out,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: ProgramLogInfo::RaySwapBaseOut(swap),
                    }
                }
            }
            LogType::Init => {
                let init = bincode::deserialize::<InitLog>(&bytes).unwrap();
                let pc = init.pc_amount;
                let coin = init.coin_amount;
                ProgramLog {
                    next_pc: 0,
                    next_coin: 0,
                    amount_out: 0,
                    pc,
                    coin,
                    log: ProgramLogInfo::RayInitLog(init),
                }
            }
            _ => unreachable!(),
        };
        swap_info
    }
    pub fn from_pump(log: String) -> Option<Self> {
        let log = log.replace("Program data: ", "");
        info!("log {log}");
        let bytes = BASE64_STANDARD.decode(log).unwrap();
        let code = &bytes[..8];
        if code == PUMP_BUY_CODE.as_slice() {
            let log = bincode::deserialize::<PumpTradeLog>(&bytes[8..]).unwrap();
            Some(ProgramLog {
                next_pc:log.virtual_sol_reserves,
                next_coin:log.virtual_token_reserves,
                amount_out: if log.is_buy { log.token_amount } else { log.sol_amount },
                pc: 0,
                coin: 0,
                log: ProgramLogInfo::PumpTradeLog(log),
            })
        } else {
            None
        }
    }
}