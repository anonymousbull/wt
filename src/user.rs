use mongodb::Collection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::implement_mongo_crud_struct;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserWithId {
    #[schemars(skip)]
    pub id: i64,
    #[schemars(rename = "wallet_key")]
    pub private_key: String,
    #[serde(skip)]
    pub username: String,
    #[serde(skip)]
    pub password: String,
    #[serde(skip)]
    pub admin:bool
}

implement_mongo_crud_struct!(UserWithId);



impl UserWithId {
    pub async fn get_by_private_key(col:&Collection<Self>, private_key: &str) -> Option<UserWithId> {
        let user = col
            .find_one(mongodb::bson::doc! { "private_key": private_key })
            .await.unwrap();
        user
    }
    pub async fn get_by_user_name(col:&Collection<Self>, username: &str) -> Option<UserWithId> {
        let user = col
            .find_one(mongodb::bson::doc! { "username": username })
            .await.unwrap();
        user
    }
}