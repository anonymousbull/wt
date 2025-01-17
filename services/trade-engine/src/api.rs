use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration to buy at current prices
#[derive(JsonSchema, Deserialize, Serialize, Clone,Debug)]
pub struct BuyPrompt {
    /// User ID
    #[schemars(skip)]
    pub kp: String,
    /// SOL amount
    pub sol_ui: Decimal,
    /// mint address
    pub mint: String,
    /// Take profit percent, should be positive number
    pub tp: Decimal,
    /// Stop loss percent, should be negative number
    pub sl: Decimal,
}
