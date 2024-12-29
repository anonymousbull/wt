use serde::{Deserialize, Serialize};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use raydium_amm::log::{InitLog, LogType, SwapBaseInLog, SwapBaseOutLog, WithdrawLog};
use raydium_amm::math::SwapDirection;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RayLog {
    pub next_pc: u64,
    pub next_coin: u64,
    pub amount_out: u64,
    pub pc: u64,
    pub coin: u64,
    pub log: RayLogInfo
}

#[derive(Debug, Clone, Serialize, Deserialize,)]
pub enum RayLogInfo {
    SwapBaseIn(SwapBaseInLog),
    SwapBaseOut(SwapBaseOutLog),
    Withdraw(WithdrawLog),
    InitLog(InitLog),
}

impl RayLog {
    pub fn from_log(log: String) -> Self {
        let log = log.replace("Program log: ray_log: ", "");
        let bytes = BASE64_STANDARD.decode(log).unwrap();
        let swap_info = match LogType::from_u8(bytes[0]) {
            LogType::Withdraw => {
                let withdraw = bincode::deserialize::<WithdrawLog>(&bytes).unwrap();
                // https://solscan.io/tx/56rnYcib58eJc1EsZJ8RiWGyQyWoVv5VihZUvyy2vrbG84PCT6iEcE5soR9pMQH6uGVp1XfosaVFHDtnWkbyMAGx

                let next_coin = withdraw.pool_coin - withdraw.out_coin;
                let next_pc = withdraw.pool_pc - withdraw.out_pc;
                RayLog {
                    next_pc,
                    next_coin,
                    amount_out: 0,
                    pc: withdraw.pool_pc,
                    coin: withdraw.pool_coin,
                    log: RayLogInfo::Withdraw(withdraw),
                }
            }
            LogType::SwapBaseIn => {
                let swap = bincode::deserialize::<SwapBaseInLog>(&bytes).unwrap();
                if swap.direction == SwapDirection::PC2Coin as u64 {
                    let next_pc = swap.pool_pc + swap.amount_in;
                    let next_coin = swap.pool_coin - swap.out_amount;
                    RayLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.out_amount,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: RayLogInfo::SwapBaseIn(swap),
                    }
                } else {
                    let coin = swap.pool_coin + swap.amount_in;
                    let pc = swap.pool_pc - swap.out_amount;
                    RayLog {
                        next_pc: pc,
                        next_coin: coin,
                        amount_out: swap.out_amount,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: RayLogInfo::SwapBaseIn(swap),

                    }
                }
            }
            LogType::SwapBaseOut => {
                let swap = bincode::deserialize::<SwapBaseOutLog>(&bytes).unwrap();
                if swap.direction == 1 {
                    let next_pc = swap.pool_pc + swap.deduct_in;
                    let next_coin = swap.pool_coin - swap.amount_out;
                    RayLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.amount_out,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: RayLogInfo::SwapBaseOut(swap),
                    }
                } else {
                    let next_coin = swap.pool_coin + swap.deduct_in;
                    let next_pc = swap.pool_pc - swap.amount_out;
                    RayLog {
                        next_pc,
                        next_coin,
                        amount_out: swap.amount_out,
                        pc: swap.pool_pc,
                        coin: swap.pool_coin,
                        log: RayLogInfo::SwapBaseOut(swap),
                    }
                }
            }
            LogType::Init => {
                let init = bincode::deserialize::<InitLog>(&bytes).unwrap();
                let pc = init.pc_amount;
                let coin = init.coin_amount;
                RayLog {
                    next_pc: 0,
                    next_coin: 0,
                    amount_out: 0,
                    pc,
                    coin,
                    log: RayLogInfo::InitLog(init),
                }
            }
            _ => unreachable!(),
        };
        swap_info
    }
}