use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};


#[derive(Clone,Debug,Copy,Serialize,Deserialize,Default,JsonSchema)]
pub struct PositionConfig {
    pub max_sol:Decimal,
    #[schemars(skip)]
    pub ata_fee:Decimal,
    #[schemars(skip)]
    pub max_jito:Decimal,
    #[schemars(skip)]
    pub close_trade_fee: Decimal,
    #[schemars(skip)]
    pub priority_fee: Decimal,
    #[schemars(skip)]
    pub base_fee: Decimal,
    #[schemars(skip)]
    /// what % fees should go to priority
    pub jito_priority_percent: Decimal,
    #[schemars(skip)]
    /// what % fees should go to jito
    pub jito_tip_percent: Decimal,
    #[schemars(skip)]
    /// compute unit limit e.g 100k
    pub cu_limit: Decimal,
    #[schemars(skip)]
    /// percentage of max_sol to fall to slippage
    pub slippage: Decimal,
    #[schemars(skip)]
    pub ray_buy_gas_limit: u64,
    #[schemars(skip)]
    pub ray_sell_gas_limit: u64,
    #[schemars(skip)]
    pub pump_buy_gas_limit: u64,
    #[schemars(skip)]
    pub pump_sell_gas_limit: u64,
    #[schemars(skip)]
    pub tp: Decimal,
    #[schemars(skip)]
    pub sl: Decimal,
    #[schemars(skip)]
    pub fee_pct: Decimal,
}

impl PositionConfig {
    pub fn tp(&self) -> Decimal {
        self.tp * self.max_sol
    }
    pub fn fee(&self) -> Decimal {
        self.fee_pct * self.tp()
    }
    pub fn total_normal(&self) ->Decimal {
        self.max_sol + (self.ata_fee * dec!(2)) + self.max_jito + self.close_trade_fee
    }
    pub fn total_minus_fees_normal(&self) ->Decimal {
        self.max_sol
    }
}

