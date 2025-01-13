use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use crate::prompt_type::BuyPrompt;
use crate::trade_type::Trade;

impl Trade {
    pub fn update(&mut self, input: String) {
        if let Some((id,sl)) = extract_sl_float(&input) {
            self.cfg.sl = Decimal::from_f64(sl).unwrap();
        }
    }
}

fn extract_sl_float(input: &str) -> Option<(i32,f64)> {
    let mut parts = input.split_whitespace();

    Some((
            parts.next()?.parse().ok()?,
            parts.next()?.strip_prefix("sl")?.parse::<f64>().ok()?,
    ))
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(tag = "name", content = "arguments")]
pub enum PromptResponse {
    Config(CfgPrompt),
    Trade(BuyPrompt),
}


/// Global trade configuration
#[derive(JsonSchema, Deserialize, Serialize, Clone, Debug)]
pub struct CfgPrompt {
    /// The user's secret key
    pub user_sk: String,
    /// Take profit percentage
    pub tp: f64,
    /// Stop loss percentage
    pub sl: f64,
}

#[derive(JsonSchema, Deserialize, Serialize, Clone, Debug)]
pub struct BalancePrompt {
    pub user_sk: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageRequest {
    /// The name of the model used for the completion.
    pub model: String,
    /// The creation time of the completion, in such format: `2023-08-04T08:52:19.385406455-07:00`.
    pub created_at: String,
    /// The generated chat message.
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageResponse {
    /// The name of the model used for the completion.
    pub model: String,
    /// The creation time of the completion, in such format: `2023-08-04T08:52:19.385406455-07:00`.
    pub created_at: String,
    /// The generated chat message.
    pub message: MessageResponse,
    pub done: bool,
    // #[serde(flatten)]
    // /// The final data of the completion. This is only present if the completion is done.
    // pub final_data: Option<ChatMessageFinalResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageResponse {
    pub content: String,
    pub role: MessageRole,
    pub tool_calls: Vec<ToolCallResponse>,
    // /// The final data of the completion. This is only present if the completion is done.


}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageFinalResponseData {
    /// Time spent generating the response
    pub total_duration: u64,
    /// Number of tokens in the prompt
    pub prompt_eval_count: u16,
    /// Time spent in nanoseconds evaluating the prompt
    pub prompt_eval_duration: u64,
    /// Number of tokens the response
    pub eval_count: u16,
    /// Time in nanoseconds spent generating the response
    pub eval_duration: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub function: ToolCallFunction,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallResponse {
    pub function: PromptResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "system")]
    System,
    #[serde(rename = "tool")]
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub model:String,
    pub stream:bool,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub role: MessageRole,
}



#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallFunction {
    pub name: String,
    // I don't love this (the Value)
    // But fixing it would be a big effort
    #[serde(flatten)]
    pub flattened_value: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolCallFunctionResponse {
    pub name: String,
    // I don't love this (the Value)
    // But fixing it would be a big effort
    pub arguments: PromptResponse,
}
