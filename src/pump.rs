use serde::{Deserialize, Serialize};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use raydium_amm::solana_program::program_error::ProgramError;
use crate::constant::{PUMP_SWAP_CODE, PUMP_EVENT_AUTHORITY, PUMP_FEE, PUMP_GLOBAL, PUMP_PROGRAM, SOLANA_RENT_PROGRAM, SOLANA_SYSTEM_PROGRAM, PUMP_BUY_CODE, PUMP_SELL_CODE, SOLANA_ATA_PROGRAM};

pub struct PumpTrade {
    pub data: PumpTradeData,
    pub mint:Pubkey,
    pub curve:Pubkey,
    pub ata_curve:Pubkey,
    pub ata_user:Pubkey,
    pub user:Pubkey,
}


/// When it's a buy min = min_token, sell min = min_sol
/// When you sell, you want the min from a buy to cash out
#[derive(Debug,Copy, Clone,Serialize,Deserialize)]
pub struct PumpTradeData {
    /// minimum tokens you want to receive
    pub min:u64,
    /// maximum tokens you want to deposit
    pub max:u64,
}

/// this should be separated to 2 fns
pub fn trade(
    data: PumpTrade,
    buy: bool
) -> Result<Instruction, ProgramError> {
    let PumpTrade { data,user, mint, curve, ata_curve, ata_user } = data;

    let min = if {buy}{data.min} else {data.max};
    let max = if {buy}{data.max} else {data.min};
    let data = PumpTradeData{
        min,max
    };
    let data = vec![
        if buy {PUMP_BUY_CODE} else {PUMP_SELL_CODE}.to_vec().as_slice(),
        bincode::serialize(&data).unwrap().as_slice(),
    ].concat();
    let accounts = vec![
        AccountMeta::new_readonly(PUMP_GLOBAL, false),
        AccountMeta::new(PUMP_FEE, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(curve, false),
        AccountMeta::new(ata_curve, false),
        AccountMeta::new(ata_user, false),
        AccountMeta::new(user, true),
        AccountMeta::new_readonly(SOLANA_SYSTEM_PROGRAM, false),
        if buy {AccountMeta::new_readonly(spl_token::id(), false)} else {AccountMeta::new_readonly(SOLANA_ATA_PROGRAM, false)},
        if buy {AccountMeta::new_readonly(SOLANA_RENT_PROGRAM, false)} else {AccountMeta::new_readonly(spl_token::id(), false)},
        AccountMeta::new_readonly(PUMP_EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(PUMP_PROGRAM, false),
    ];
    Ok(Instruction {
        program_id: PUMP_PROGRAM,
        accounts,
        data,
    })
}