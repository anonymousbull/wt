use crate::db::diesel_export::*;
use crate::implement_mongo_crud;
use futures::TryStreamExt;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWithId {
    pub id: i64,
    pub private_key: String,
}


// implement_mongo_crud!(UserWithId);
implement_mongo_crud!(UserWithId);

// impl UserWithId {
//     pub async fn delete_one(&self, collection: &Collection<UserWithId>) -> anyhow::Result<()> {
//         collection
//             .delete_one(doc! { "id": self.id })
//             .await?;
//         Ok(())
//     }
//
//     pub async fn delete_by_id(
//         id: i64,
//         collection: &Collection<UserWithId>,
//     ) -> anyhow::Result<Option<UserWithId>> {
//         let deleted = collection
//             .find_one_and_delete(doc! { "id": id })
//             .await?;
//         Ok(deleted)
//     }
//
//     pub async fn count(collection: &Collection<UserWithId>) -> i64 {
//         let count = collection.count_documents(doc! {}).await.unwrap();
//         count as i64
//     }
//
//     pub async fn insert_bulk(
//         collection: &Collection<UserWithId>,
//         data: Vec<UserWithId>,
//     ) -> anyhow::Result<()> {
//         collection.insert_many(data).await?;
//         Ok(())
//     }
//
//     pub async fn insert(&self, collection: &Collection<UserWithId>) -> anyhow::Result<()> {
//         collection.insert_one(self).await?;
//         Ok(())
//     }
//
//     pub async fn get_by_id(
//         id: i64,
//         collection: &Collection<UserWithId>,
//     ) -> anyhow::Result<Option<UserWithId>> {
//         let user = collection
//             .find_one(doc! { "id": id })
//             .await?;
//         Ok(user)
//     }
//
//     pub async fn get_all(collection: &Collection<UserWithId>) -> anyhow::Result<Vec<UserWithId>> {
//         let mut cursor = collection.find(doc! {}).await?;
//         let users = cursor.try_collect().await?;
//         Ok(users)
//     }
// }


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

