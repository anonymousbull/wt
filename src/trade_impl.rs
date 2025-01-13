use std::time::Duration;
use anyhow::anyhow;
use futures::TryStreamExt;
use mongodb::bson::{doc, Binary};
use mongodb::bson::spec::BinarySubtype;
use postgrest::Postgrest;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use tokio::sync::oneshot;
use yellowstone_grpc_proto::tonic::codegen::tokio_stream::StreamExt;
use crate::chan::Chan;
use crate::cmd::InternalCommand;
use crate::constant::{SOLANA_MINT, SUPABASE_URL};
use crate::trade_type::{Trade, TradePool};


impl Trade {
    pub async fn db_upsert_mongo(
        &self,
        c: &mongodb::Collection<Self>,
    ) {
        c.insert_one(self).await.unwrap();
    }
    pub async fn db_get_users_supabase(
        client: &Postgrest,
    ) -> Vec<Self> {
        #[derive(Serialize, Deserialize, Debug, Clone)]
        pub struct UserKey {
            pub private_key: String,
        }
        let resp = client.from("keys").select("*").execute().await.unwrap();
        let users = resp
            .json::<Vec<UserKey>>()
            .await
            .unwrap()
            .iter()
            .filter_map(|x| bs58::decode(x.private_key.clone()).into_vec().ok())
            .filter_map(|x| Keypair::from_bytes(&x).ok())
            .map(|x| Trade::dangerous_user(x.to_bytes().to_vec()))
            .collect::<Vec<_>>();
        users
    }
    pub async fn db_get_users_mongo(
        c: &mongodb::Collection<Self>,
    ) -> Vec<Self> {
        c.find(doc! {}).await.unwrap().try_collect().await.unwrap_or(vec![])

        // let binary_data = Binary {
        //     subtype: BinarySubtype::Generic, // Adjust subtype as needed
        //     bytes: self.root_kp.clone(),
        // };
        // let filter = doc! {
        //     "kp": binary_data
        // };
        // c.find_one(filter).await.unwrap_or(None)
    }

    pub async fn buy_now(
        self,
        chan:Chan
    ) -> anyhow::Result<Trade> {
        let (s, r) = oneshot::channel::<InternalCommand>();
        chan.trade.try_send(InternalCommand::TradeRequest(self.clone(),s))?;
        match tokio::time::timeout(Duration::from_secs(5), r).await {
            Ok(Ok(InternalCommand::TradeResponse(trade))) => {
                Ok(trade)
            }
            Ok(e) => {
                Err(e.map_err(anyhow::Error::from).unwrap_err())
            }
            e => {
                Err(e.map_err(anyhow::Error::from).unwrap_err())
            }
        }
    }

    pub async fn login(
        kp_str:&str,
        chan:Chan
    ) -> anyhow::Result<Trade> {
        let kp = std::panic::catch_unwind(|| Keypair::from_base58_string(kp_str));
        match kp {
            Ok(kp) => {
                let (s, r) = oneshot::channel::<InternalCommand>();
                chan.trade.try_send(InternalCommand::LoginRequest(kp,s))?;
                match tokio::time::timeout(Duration::from_secs(5), r).await {
                    Ok(Ok(InternalCommand::LoginResponse(Some(trade)))) => {
                        Ok(trade)
                    }

                    e => {
                        Err(e.map_err(anyhow::Error::from).unwrap_err())
                    }
                }
            }
            Err(e) => Err(anyhow!("not a valid keypair"))
        }
    }

    /// This is very dangerous and will fix later
    pub fn dangerous_user(kp:Vec<u8>) ->Self{
        Trade{
            id: 0,
            amount: Default::default(),
            buy_time: None,
            buy_price: None,
            sell_time: None,
            sell_price: None,
            pct: Default::default(),
            state: Default::default(),
            sol_before: Default::default(),
            sol_after: None,
            root_kp: kp,
            user_id: "".to_string(),
            amm: TradePool::RayAmm4Mint(SOLANA_MINT),
            user_wallet: "".to_string(),
            price: Default::default(),
            k: Default::default(),
            tvl: Default::default(),
            cfg: Default::default(),
            error: None,
            plog: Default::default(),
            buy_out: None,
            internal: None,
        }
    }

}