use serde::{Deserialize, Serialize};
use crate::instruction::TradeInstruction;
use crate::rpc::{RpcState, TradeRpcLog};
use crate::transaction::TradeTransaction;

#[derive(Clone, Debug,Serialize,Deserialize)]
pub struct TradeInternal {
    pub instructions: Vec<TradeInstruction>,
    pub transactions: Vec<TradeTransaction>,
    pub rpc_status: RpcState,
    pub rpc_logs:Vec<TradeRpcLog>,
}