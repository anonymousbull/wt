use crate::constant::*;
use crate::trade::{RpcInfo, RpcsConfig, TipStatistics, TradeRpc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;

pub fn rpcs(config:RpcsConfig) -> Vec<TradeRpc> {
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

pub fn rpc1() -> TradeRpc {
    TradeRpc::General {
        rpc: RpcClient::new_with_commitment(RPC1.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC1_NAME.to_string(),
            location:RPC1_LOCATION.to_string(),
        }
    }
}

pub fn rpc2() -> TradeRpc {
    TradeRpc::General {
        rpc: RpcClient::new_with_commitment(RPC2.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC2_NAME.to_string(),
            location:RPC2_LOCATION.to_string(),
        }
    }
}

pub fn rpc3() -> TradeRpc {
    TradeRpc::General {
        rpc: RpcClient::new_with_commitment(RPC3.to_string(), CommitmentConfig::processed()),
        info: RpcInfo{
            name: RPC3_NAME.to_string(),
            location:RPC3_LOCATION.to_string(),
        }
    }
}

const JITO_NAME:&'static str = "JITO";

pub fn jito1(tip: Option<TipStatistics>) -> TradeRpc {
    TradeRpc::Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK::new(JITO1, None),
        tip,
        info: RpcInfo{
            name: JITO_NAME.to_string(),
            location:JITO1_LOCATION.to_string(),
        }
    }
}

pub fn jito2(tip: Option<TipStatistics>) -> TradeRpc {
    TradeRpc::Jito {
        rpc: jito_sdk_rust::JitoJsonRpcSDK::new(JITO2, None),
        tip,
        info: RpcInfo{
            name: JITO_NAME.to_string(),
            location:JITO2_LOCATION.to_string(),
        }
    }
}







