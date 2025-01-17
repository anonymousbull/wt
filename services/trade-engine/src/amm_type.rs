use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use raydium_amm::solana_program::pubkey::Pubkey;

#[derive(Clone,Copy,Serialize,Deserialize,Debug,JsonSchema)]
pub struct PumpBondingCurve{
    #[schemars(with = "String")]
    pub amm:Pubkey,
    #[schemars(with = "String")]
    pub vault:Pubkey,
    #[schemars(with = "String")]
    pub mint:Pubkey,
    /// decimal places of asset
    pub decimals: Option<i16>,
    /// token_program_id
    #[schemars(with = "String")]
    pub token_program_id: Pubkey,
}

#[derive(Clone,Copy,Serialize,Deserialize,Debug,JsonSchema)]
pub struct RayAmm4{
    #[schemars(with = "String")]
    pub amm:Pubkey,
    #[schemars(with = "String")]
    pub coin_vault: Pubkey,
    #[schemars(with = "String")]
    pub pc_vault: Pubkey,
    #[schemars(with = "String")]
    pub coin_mint: Pubkey,
    #[schemars(with = "String")]
    pub pc_mint: Pubkey,
    /// decimal places of asset
    pub decimals: Option<i16>,
    /// token_program_id
    #[schemars(with = "String")]
    pub token_program_id: Pubkey,
}

/// Mint information, used to determine what mint to buy or sell
#[derive(Debug, Serialize, Deserialize, Clone, Copy, JsonSchema)]
pub enum TradePool {
    /// Raydium AMM v4 mint address
    Generic(#[schemars(with = "String")] Pubkey),
    /// Pump bonding curve mint address
    PumpBondingCurveMint(#[schemars(with = "String")] Pubkey),
    #[schemars(skip)]
    /// Raydium AMM v4 object
    RayAmm4(RayAmm4),
    #[schemars(skip)]
    /// Pump bonding curve object
    PumpBondingCurve(PumpBondingCurve),
}

impl TradePool {
    pub fn to_string(&self)->String{
        match self {
            TradePool::RayAmm4(v) => v.amm.to_string(),
            TradePool::PumpBondingCurve(v) => v.amm.to_string(),
            _ => unreachable!()
        }
    }
    pub fn amm(&self) ->Pubkey{
        match self {
            TradePool::RayAmm4(v) => v.amm,
            TradePool::PumpBondingCurve(v) => v.amm,
            _ => unreachable!()
        }
    }
}
