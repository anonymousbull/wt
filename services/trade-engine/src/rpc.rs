use serde::{Deserialize, Serialize};
use solana_sdk::transaction::Transaction;
use raydium_amm::solana_program::instruction::Instruction;
use std::str::FromStr;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use schemars::JsonSchema;
use crate::constant::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcInfo {
    pub name:String,
    pub location: String
}

#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcResponse {
    pub signature:String,
    pub metric:RpcResponseMetric,
    pub rpc_info:RpcInfo,
    pub rpc_response_data: RpcResponseData
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq, Eq)]
pub enum TradeRpcLog {
    BuyJito(TradeRpcLogJito,TradeRpcLogStatus),
    SellJito(TradeRpcLogJito,TradeRpcLogStatus),
    BuyGeneral(TradeRpcLogGeneral,TradeRpcLogStatus),
    SellGeneral(TradeRpcLogGeneral,TradeRpcLogStatus),
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum TradeRpcLogStatus {
    Pending,
    Success,
    Fail,
}

#[derive(Debug, Serialize, Deserialize,Clone,Copy)]
pub struct TipStatistics {
    pub time: DateTime<Utc>,
    pub landed_tips_25th_percentile: Decimal,
    pub landed_tips_50th_percentile: Decimal,
    pub landed_tips_75th_percentile: Decimal,
    pub landed_tips_95th_percentile: Decimal,
    pub landed_tips_99th_percentile: Decimal,
    pub ema_landed_tips_50th_percentile: Decimal,
}

impl TradeRpcLog {
    pub fn change_state(&mut self, signature: Signature, status:TradeRpcLogStatus){
        match self {
            TradeRpcLog::BuyJito(x, _) => {
                if x.general.signature == signature.to_string() {
                    *self = TradeRpcLog::BuyJito(x.clone(), status);
                }
            }
            TradeRpcLog::SellJito(x, _) => {
                if x.general.signature == signature.to_string() {
                    *self = TradeRpcLog::SellJito(x.clone(), status);
                }
            }
            TradeRpcLog::BuyGeneral(x, _) => {
                if x.signature == signature.to_string() {
                    *self = TradeRpcLog::BuyGeneral(x.clone(), status);
                }
            }
            TradeRpcLog::SellGeneral(x, _) => {
                if x.signature == signature.to_string() {
                    *self = TradeRpcLog::SellGeneral(x.clone(), status);
                }
            }
        }
    }
    pub fn signature(&self)->Signature{
        let s = match self {
            TradeRpcLog::BuyJito(v,_) => v.general.signature.to_string(),
            TradeRpcLog::SellJito(v,_) => v.general.signature.to_string(),
            TradeRpcLog::BuyGeneral(v,_) => v.signature.to_string(),
            TradeRpcLog::SellGeneral(v,_) => v.signature.to_string()
        };
        Signature::from_str(s.as_str()).unwrap()
    }
    pub fn success_signature(&self)->Option<Signature>{
        let s = match self {
            TradeRpcLog::BuyJito(v,TradeRpcLogStatus::Success) => Some(v.general.signature.to_string()),
            TradeRpcLog::SellJito(v,TradeRpcLogStatus::Success) => Some(v.general.signature.to_string()),
            TradeRpcLog::BuyGeneral(v,TradeRpcLogStatus::Success) => Some(v.signature.to_string()),
            TradeRpcLog::SellGeneral(v,TradeRpcLogStatus::Success) => Some(v.signature.to_string()),
            _ => None
        }.map(|x|Signature::from_str(x.as_str()).unwrap());
        s
    }

}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct TradeRpcLogJito {
    pub general:TradeRpcLogGeneral,
    pub tip_sol_ui: String
}

#[derive(Clone,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub struct TradeRpcLogGeneral {
    pub signature:String,
    pub response_time:String,
    pub name:String,
    pub tx: Transaction,
    pub ix: Vec<Instruction>,
}

#[derive(Clone,Debug,Serialize,Deserialize,Default,PartialEq,Eq,JsonSchema)]
pub enum RpcState {
    #[default]
    Free=1,
    Busy
}

#[derive(Clone,Debug,Serialize,Deserialize,Copy)]
pub enum RpcResponseData {
    Jito {
        tip_amount_sol:Decimal,
        priority_amount_micro_sol:Decimal,
    },
    General {
        pfee_sol_ui:Decimal,
    }
}



#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcResponseMetric {
    pub response_time:u128,
    pub response_time_string:String,
}

pub enum RpcType {
    Jito,
    General
}



pub enum Rpc {
    Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK,
        tip: Option<TipStatistics>,
        info: RpcInfo
    },
    General {
        rpc: RpcClient,
        info: RpcInfo
    }
}


#[derive(Clone,Debug,Serialize,Deserialize,Default,Copy)]
pub struct RpcsConfig {
    pub jito_tip: Option<TipStatistics>,
}

pub fn rpcs(config:RpcsConfig) -> Vec<Rpc> {
    vec![
        // rpc1(),
        // rpc2(),
        // rpc3(),
        jito1(config.jito_tip),
        // jito2(config.jito_tip)
    ]
}

pub fn solana_rpc_client() -> RpcClient {
    RpcClient::new_with_commitment(RPC1.to_string(), CommitmentConfig::processed())
}



pub fn rpc1_unchecked() -> RpcClient {
    RpcClient::new_with_commitment(RPC1.to_string(), CommitmentConfig::processed())
}

pub fn rpc1() -> Rpc {
    Rpc::General {
        rpc: RpcClient::new_with_commitment(RPC1.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC1_NAME.to_string(),
            location:RPC1_LOCATION.to_string(),
        }
    }
}

pub fn rpc2() -> Rpc {
    Rpc::General {
        rpc: RpcClient::new_with_commitment(RPC2.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC2_NAME.to_string(),
            location:RPC2_LOCATION.to_string(),
        }
    }
}

pub fn rpc3() -> Rpc {
    Rpc::General {
        rpc: RpcClient::new_with_commitment(RPC3.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC3_NAME.to_string(),
            location:RPC3_LOCATION.to_string(),
        }
    }
}

const JITO_NAME:&'static str = "JITO";

pub fn jito1(tip: Option<TipStatistics>) -> Rpc {
    Rpc::Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK::new(JITO1, None),
        tip,
        info: RpcInfo{
            name: JITO_NAME.to_string(),
            location:JITO1_LOCATION.to_string(),
        }
    }
}

pub fn jito2(tip: Option<TipStatistics>) -> Rpc {
    Rpc::Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK::new(JITO2, None),
        tip,
        info: RpcInfo{
            name: JITO_NAME.to_string(),
            location:JITO2_LOCATION.to_string(),
        }
    }
}







