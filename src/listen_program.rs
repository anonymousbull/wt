use crate::constant::{RAYDIUM_V4_PROGRAM, SOLANA_RPC_URL, SOLANA_WS_URL};
use futures::StreamExt;
use log::info;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig, RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::UiTransactionEncoding;
use std::str::FromStr;
use base64::Engine;
use solana_account_decoder::{UiAccountData, UiAccountEncoding};
use solana_client::rpc_filter::{MemcmpEncodedBytes, RpcFilterType};
use raydium_amm::solana_program::pubkey::Pubkey;
use raydium_amm::state::{AmmInfo, Loadable};

pub async fn listen_to_program(_: &Chan, amm:Pubkey) {
    // let pg = pg_conn().await;
    // let mut c = pg.get().await.unwrap();
    // let mut swap_ix_id = SwapIx::id(&mut c).await;

    let pubsub_client = PubsubClient::new(SOLANA_WS_URL).await.unwrap();
    let a = solana_client::rpc_filter::Memcmp::new(
        336,
        MemcmpEncodedBytes::Base58(amm.to_string())
    );
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::DataSize(752),RpcFilterType::Memcmp(a)]),
        account_config: RpcAccountInfoConfig{
            encoding: Some(UiAccountEncoding::JsonParsed),
            data_slice: None,
            commitment: Some(CommitmentConfig::processed()),
            min_context_slot: None,
        },
        with_context: None,
        sort_results: None,
    };

    let rpc =
        RpcClient::new_with_commitment(SOLANA_RPC_URL.to_string(), CommitmentConfig::processed());

    let (mut stream, _) = pubsub_client.program_subscribe(&RAYDIUM_V4_PROGRAM, Some(config)).await.unwrap();

    // let mut cache = vec![];
    info!("listening to solana blocks");

    // let a = rpc.get_transaction_with_config(
    //     &solana_sdk::signature::Signature::from_str("5LRrQVXAnK25DGUi7QLRCUHe8uSqrVRMTwYdzzfPUagXhQNkpRX5h1hQrNX8f8zXQUF9J9GnB4Z2V5Mobfs4SXuo")
    //         .unwrap(),
    //     RpcTransactionConfig{
    //         encoding: Some(UiTransactionEncoding::JsonParsed),
    //         commitment: None,
    //         max_supported_transaction_version: Some(0),
    //     },
    // ).await;
    //
    // info!("{:?}",a);

    while let Some(response) = stream.next().await {
       let a = response.value.account.data.decode().unwrap();



        let b= AmmInfo::load_from_bytes(a.as_slice()).unwrap();

        info!("{:?}",b.state_data);
        info!("{:?}",b.coin_vault.to_bytes());
        info!("{:?}",b.coin_vault.to_bytes());
        info!("{:?}",&a[336..368]);

        // let b = String::from_utf8(a).unwrap();
        info!("binary {:?} {:?}",response.value.pubkey,2);
        // match response.value.account.data {
        //     UiAccountData::LegacyBinary(a) => {
        //         info!("LegacyBinary {:?}",a);
        //     }
        //     UiAccountData::Json(x) => {
        //         info!("{:?}",x);
        //     }
        //     UiAccountData::Binary(a, x) => {
        //
        //
        //
        //     }
        // }

        // if !found.is_empty() {
        //     info!("found {:?}",block);
        // }
        // for ref tx in block.transactions.unwrap() {
        //     match &tx.transaction {
        //         EncodedTransaction::Json(json) => {
        //             match &json.message {
        //                 UiMessage::Parsed(parsed) => {
        //                     for instruction in &parsed.instructions {
        //                         match instruction {
        //                             UiInstruction::Compiled(c) => {}
        //                             UiInstruction::Parsed(p_instruction) => {
        //                                 match p_instruction {
        //                                     UiParsedInstruction::PartiallyDecoded(partial) => {
        //                                         // SWAP
        //                                         if partial.accounts.len() == 18
        //                                             && partial.program_id
        //                                             == constants::raydium_amm()
        //                                         {
        //                                             swap_ix_id += 1;
        //                                             cache.push(SwapIx::from_solana_account_18(
        //                                                 partial, swap_ix_id,
        //                                             ));
        //                                         }
        //                                         // SWAP
        //                                         else if partial.accounts.len() == 17
        //                                             && partial.program_id
        //                                             == constants::raydium_amm()
        //                                             && partial.data.len() > 7
        //                                         {
        //                                             swap_ix_id += 1;
        //                                             cache.push(crate::swap_ix::SwapIx(
        //                                                 partial, swap_ix_id,
        //                                             ));
        //                                         }
        //                                         // NEW POOL
        //                                         else if partial.accounts.len() == 21
        //                                             && partial.program_id
        //                                             == constants::raydium_amm()
        //                                         {
        //                                             let cmd = InternalCommand::NewRaydiumPool {
        //                                                 data: PoolIx::from_solana_account(
        //                                                     partial.accounts.as_slice(),
        //                                                 ),
        //                                             };
        //                                             chan.app
        //                                                 .send(cmd)
        //                                                 .expect("server channel never closes");
        //                                         };
        //                                     }
        //                                     _ => {}
        //                                 }
        //                             }
        //                         }
        //                     }
        //                 }
        //                 _ => {}
        //             }
        //         }
        //         _ => {}
        //     }
        // }
    }
}
