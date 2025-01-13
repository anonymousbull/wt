use crate::db::diesel_export::*;
use crate::{implement_diesel, implement_diesel_insert};
use crate::trade_type::Trade;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, AsChangeset, Identifiable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithId {
    pub id: i64,
    pub private_key: String,
}

#[derive(Selectable, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub private_key: String,
}

implement_diesel!(UserWithId, users);
implement_diesel_insert!(User, users, UserWithId);


impl UserWithId {
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

