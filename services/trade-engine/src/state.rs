use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[repr(i32)]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash, Default, JsonSchema)]
pub enum TradeState {
    #[default]
    /// trade is in buy state with intent to buy later
    Buy = 1,
    /// trade has been placed and pending buy confirmation
    PendingBuy,
    /// trade has been placed and buy was successful
    BuySuccess,
    /// trade is in sell state with intent to sell previous buy trade later
    PendingSell,
    /// trade has been placed and sell was successful
    SellSuccess,
    /// trade has been placed and buy was failed
    BuyFailed,
    /// trade has been placed and sell was failed
    SellFailed
}
