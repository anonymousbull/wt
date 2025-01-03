use rust_decimal::Decimal;
use crate::constant::*;
use crate::jito_chan::TipStatistics;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;


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

#[derive(Clone,Debug,Serialize,Deserialize,Copy)]
pub enum RpcResponseData {
    Jito {
        tip_amount_sol:Decimal,
        priority_amount_micro_sol:Decimal,
    },
    General {
        priority_amount_normal:Decimal,
    }
}



#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct RpcResponseMetric {
    pub response_time:u128,
    pub response_time_string:String,
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

pub fn geyser() -> yellowstone_grpc_client::GeyserGrpcBuilder {
    yellowstone_grpc_client::GeyserGrpcClient::build_from_static(GRPC1)
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

