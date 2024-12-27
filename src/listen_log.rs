use crate::constant::{SOLANA_RPC_URL, SOLANA_WS_URL};
use crate::util::Chan;
use futures::StreamExt;
use log::info;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionConfig, RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::{EncodableWithMeta, EncodedConfirmedTransactionWithStatusMeta, EncodedTransaction, UiTransactionEncoding};
use std::str::FromStr;
use solana_sdk::pubkey::Pubkey;
use raydium_amm::solana_program::message::VersionedMessage;
use crate::listen_program::listen_to_program;

pub async fn listen_to_memecoin_logs(_: &Chan) {
    // let pg = pg_conn().await;
    // let mut c = pg.get().await.unwrap();
    // let mut swap_ix_id = SwapIx::id(&mut c).await;

    let pubsub_client = PubsubClient::new(SOLANA_WS_URL).await.unwrap();
    let config = RpcTransactionLogsConfig {
        commitment: Some(CommitmentConfig::processed()),
    };

    let rpc =
        RpcClient::new_with_commitment(SOLANA_RPC_URL.to_string(), CommitmentConfig::confirmed());

    let filter = RpcTransactionLogsFilter::Mentions(vec![
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8".to_string(),
    ]);
    let (mut stream, _) = pubsub_client.logs_subscribe(filter, config).await.unwrap();

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
        let block = response.value.logs;
        let is_new_pool = block.iter().any(|x| x.contains("init_pc_amount"));
        if is_new_pool {
            info!("nipped {:?}", block);
            info!("siga {:?}", response.value.signature);



            let tx = rpc
                .get_transaction_with_config(
                    &solana_sdk::signature::Signature::from_str(response.value.signature.as_str())
                        .unwrap(),
                    RpcTransactionConfig{
                        encoding: Some(UiTransactionEncoding::Binary),
                        commitment: Some(CommitmentConfig::confirmed()),
                        max_supported_transaction_version: Some(0),
                    },
                )
                .await.unwrap();

            let a = tx.transaction.transaction.decode().unwrap();
            let amm = match a.message {
                VersionedMessage::Legacy(x) => x.account_keys[2],
                VersionedMessage::V0(x) => x.account_keys[2]
            };

            info!("{:?}", amm);


            // listen_to_program(
            //     &Chan{},
            //     amm
            // ).await;

            // let tx = rpc
            //     .poll_for_signature(
            //         &solana_sdk::signature::Signature::from_str(response.value.signature.as_str())
            //             .unwrap(),
            //     )
            //     .await;
            // info!("{:?}", tx);

            // response.value.
            //             match block.iter().find(|x|x.contains("ray_log")) {
            //                 None => {}
            //                 Some(log) => {
            //                     let clean = log.replace("Program log: ray_log: ","");
            //                     let init_log = bincode::deserialize::<InitLog>(&clean.as_bytes()).unwrap();
            //                     let rpc = solana_rpc_client();
            //                     let mut m = rpc.get_account(&init_log.market).await.unwrap();
            //                     let flags =  serum_dex::state::Market::account_flags(m.data.as_slice()).unwrap();
            //                     let g = &mut 0;
            // let ai = AccountInfo::new(
            //     &init_log.market,false,false,g,m.data.as_mut_slice(),
            //     &m.owner, false, 0
            // );
            //                     let bb = serum_dex::state::Market::load(
            //                         &ai,
            //                         &Pubkey::from_str_const("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"),
            //                         true
            //                     );
            //                     info!("so far so good");
            //                     let  a= bb.unwrap().pc_vault;
            //                     info!("yes {:?}",a);
            //
            //                 }
            //             }
        }
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
