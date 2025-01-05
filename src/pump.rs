use serde::{Deserialize, Serialize};
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use raydium_amm::solana_program::program_error::ProgramError;
use crate::constant::{PUMP_SWAP_CODE, PUMP_EVENT_AUTHORITY, PUMP_FEE, PUMP_GLOBAL, PUMP_PROGRAM, SOLANA_RENT_PROGRAM, SOLANA_SYSTEM_PROGRAM, PUMP_BUY_CODE};

pub struct PumpBuy {
    pub data:PumpBuyData,
    pub mint:Pubkey,
    pub curve:Pubkey,
    pub ata_curve:Pubkey,
    pub ata_user:Pubkey,
    pub user:Pubkey,
}

#[derive(Debug,Copy, Clone,Serialize,Deserialize)]
pub struct PumpBuyData {
    /// minimum tokens you want to receive
    pub min:u64,
    /// maximum tokens you want to deposit
    pub max:u64,
}

pub fn buy(
    data:PumpBuy
) -> Result<Instruction, ProgramError> {
    let PumpBuy{ data,user, mint, curve, ata_curve, ata_user } = data;
    let data = vec![
        PUMP_BUY_CODE.to_vec().as_slice(),
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
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(SOLANA_RENT_PROGRAM, false),
        AccountMeta::new_readonly(PUMP_EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(PUMP_PROGRAM, false),
    ];
    Ok(Instruction {
        program_id: PUMP_PROGRAM,
        accounts,
        data,
    })
}