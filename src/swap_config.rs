use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone,Debug,Copy,Serialize,Deserialize,JsonSchema)]
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
    /// How much of profit is for fees
    pub fee_pct: Decimal,
}

impl Default for PositionConfig {
    fn default() -> Self {
        Self {
            max_sol: dec!(0),
            ata_fee: dec!(0.00203928),
            max_jito: dec!(0.001),
            close_trade_fee: dec!(0.00203928),
            priority_fee: dec!(7_00_000), // 0.0007 SOL
            base_fee: dec!(0.000005),
            jito_priority_percent: dec!(0.7),
            jito_tip_percent: dec!(0.3),
            cu_limit: Default::default(),
            // When you're fast this can be low
            slippage: dec!(0.05),
            ray_buy_gas_limit: 60_000,
            ray_sell_gas_limit: 60_000,
            pump_buy_gas_limit: 80_000,
            pump_sell_gas_limit: 70_000,
            tp: dec!(0),
            sl: dec!(0),
            fee_pct: dec!(0.2),
        }
    }
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
