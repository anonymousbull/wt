use crate::db::diesel_export::*;
use crate::implement_diesel;
use crate::trade22::Trade;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub sk: String,
    pub trade: Trade,
}

impl User {
    // pub async fn get_by_sk(sk: &str, mut c: &mut Object<AsyncPgConnection>) -> Option<Self> {
    //     crate::schema::users::dsl::users
    //         .filter(crate::schema::users::dsl::sk.eq(sk))
    //         .first(&mut c)
    //         .await
    //         .ok()
    // }
    pub async fn get_by_sk(sk: &str, mut c: &mongodb::Collection<Self>) -> Option<Self> {
        let filter = doc! {
            "sk": sk
        };
        c.find_one(filter).await.unwrap()
    }
    pub async fn update(&self, mut c: &mongodb::Collection<Self>) {
        c.replace_one(doc! { "id": self.id }, self).await.unwrap();
    }
}

