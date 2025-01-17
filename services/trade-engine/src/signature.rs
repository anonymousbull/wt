use serde::{Deserialize, Serialize};

#[derive(Clone,Serialize,Deserialize,Debug, PartialEq, Eq)]
pub enum TradeSignature {
    PendingBuy(String),
    PendingSell(String),
    BuySuccess(String),
    SellSuccess(String),
}