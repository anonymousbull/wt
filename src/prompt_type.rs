use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use serde_json::Value;
use schemars::{schema_for, JsonSchema};
use solana_sdk::pubkey::Pubkey;
use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::trade_type::Trade;

// pub fn trade_prompt(
//     chan:Chan,
//     prompt:TradePrompt
// ){
//     let (s, r) = oneshot::channel::<InternalCommand>();
//     chan.trade.send(InternalCommand::TradeRequest(
//
//     )).unwrap()
// }

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct PromptRequest {
    pub trade:Trade,
    pub message:String
}

/// Configuration to buy at current prices
#[derive(JsonSchema, Deserialize, Serialize, Clone,Debug)]
pub struct BuyPrompt {
    /// User ID
    #[schemars(skip)]
    pub kp: String,
    /// SOL amount
    pub sol_ui: f64,
    /// mint address
    pub mint: String,
    /// Take profit percent
    pub tp: f64,
    /// Stop loss percent
    pub sl: f64,
}

/// Configuration to close existing trade at current prices
#[derive(JsonSchema, Deserialize, Serialize, Clone,Debug)]
pub struct SellPrompt {
    /// User ID
    #[schemars(skip)]
    pub user_id: Option<String>,
    /// mint address to close
    pub mint: String,
}

impl SellPrompt {
    pub fn schema() -> Value {
        schema_for!(SellPrompt).into()
    }
}