use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};


#[derive(Clone,Debug,Copy,Serialize,Deserialize)]
pub struct PositionConfig {
    pub min_sol:Decimal,
    pub max_sol:Decimal,
    pub fee:Decimal,
    pub jito:Decimal,
    pub close_trade_fee: Decimal,
    pub priority_fee: Decimal,
}

impl PositionConfig {
    pub fn total_normal(&self) ->Decimal {
        self.max_sol + (self.fee * dec!(2)) + self.jito + self.close_trade_fee
    }
    pub fn total_minus_fees_normal(&self) ->Decimal {
        self.max_sol
    }
}

