use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};


#[derive(Clone,Debug,Copy,Serialize,Deserialize)]
pub struct PositionConfig {
    pub max_sol:Decimal,
    pub ata_fee:Decimal,
    pub max_jito:Decimal,
    pub close_trade_fee: Decimal,
    pub priority_fee: Decimal,
    pub base_fee: Decimal,
    /// what % fees should go to priority
    pub jito_priority_percent: Decimal,
    /// what % fees should go to jito
    pub jito_tip_percent: Decimal,
    /// compute unit limit e.g 100k
    pub cu_limit: Decimal,
    /// percentage of max_sol to fall to slippage
    pub slippage: Decimal,
    pub buy_gas_limit: Decimal,
    pub sell_gas_limit: Decimal,
    pub tp: Decimal,
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

