use log::info;
use solana_sdk::pubkey::Pubkey;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[cfg(feature = "diesel")]
pub mod diesel_export {
    pub use diesel::RunQueryDsl;
    pub use diesel_async::pooled_connection::deadpool::Pool;
    pub use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    pub use diesel_async::pooled_connection::deadpool::Object;
    pub use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
    pub use diesel::dsl::count_star;
    pub use diesel::prelude::*;
}

pub async fn get_ignore_mints() -> Vec<Pubkey> {
    let trades_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open("trades.txt")
        .await
        .unwrap();
    let mut reader = BufReader::new(trades_file); // Create a buffered reader
    let mut lines    = reader.lines(); // Initialize a vector to hold the lines
    let mut ignore_mints = vec![];
    while let Ok(Some(line)) = lines.next_line().await {
        ignore_mints.push(Pubkey::from_str_const(line.as_str()));
    }
    ignore_mints
}

pub async fn update_ignore_mints(mint:String) {
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("trades.txt")
        .await
        .unwrap();
    info!("updaing mints {mint}");
    file.write(format!("\n{mint}").as_bytes()).await.unwrap();
}

//
// pub trait SdbImpl:Sized where Self: serde::de::DeserializeOwned+serde::Serialize +'static + Clone
// {
//
//     fn table_name() -> &'static str;
//     fn field_id(&self) -> i64;
//
//     async fn upsert_bulk(pg: &Sdb, data: Vec<Self>) -> anyhow::Result<()>{
//         let start_time = ::std::time::Instant::now();
//         for x in &data {
//             pg.upsert::<Option<Self>>((Self::table_name(),x.field_id()))
//                 .content(x.clone())
//                 .await?;
//         }
//         let elapsed_time = start_time.elapsed();
//         info!("Time taken for bulk upsert: {} {} {:?}",data.len(), Self::table_name(), elapsed_time);
//         Ok(())
//     }
//     async fn upsert(&self,pg: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) -> anyhow::Result<()> {
//         // let start_time = ::std::time::Instant::now();
//         pg
//             .upsert::<Option<Self>>((Self::table_name(), self.field_id()))
//             .content(self.clone())
//             .await.unwrap();
//         // let elapsed_time = start_time.elapsed();
//         // log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
//         Ok(())
//     }
//     async fn delete_one(&self, c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>){
//         c.delete::<Option<Self>>((Self::table_name(), self.field_id())).await.unwrap();
//     }
//     async fn id(c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) -> i64 {
//         let sql = format!("SELECT count() FROM {}",Self::table_name());
//         let a = c
//             .query(sql)
//         .await.unwrap().take::<Option<serde_json::Value>>(0).unwrap();
//         a.unwrap_or(serde_json::json!({"count": 0}))["count"].as_i64().unwrap()
//     }
//     async fn insert_bulk(pg: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>, data: Vec<Self>) -> anyhow::Result<()> {
//         // let start_time = ::std::time::Instant::now();
//         pg.insert::<Vec<Self>>(Self::table_name()).content(vec![data]).await.unwrap();
//         // let elapsed_time = start_time.elapsed();
//         // log::info!("Time taken for bulk insert: {} {} {:?}",data.len(), stringify!($table_name), elapsed_time);
//         Ok(())
//     }
//     async fn insert(&self,pg: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) -> anyhow::Result<()> {
//         // let start_time = ::std::time::Instant::now();
//         pg
//             .insert::<Option<Self>>((Self::table_name(), self.field_id()))
//             .content(self.clone())
//             .await.unwrap();
//         // let elapsed_time = start_time.elapsed();
//         // log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
//         Ok(())
//     }
//     async fn get_all(c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) ->Vec<Self>{
//         c.select("trades").await.unwrap()
//     }
// }

#[macro_export]
macro_rules! implement_diesel_insert {
    ($struct_name:ident, $table_name:ident, $id_struct_name:ident) => {
        impl $struct_name {
            pub async fn insert_bulk(mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>, data: Vec<$id_struct_name>) -> anyhow::Result<()> {
                // let start_time = ::std::time::Instant::now();
                diesel::insert_into(crate::schema::$table_name::table)
                    .values(&data)
                    .execute(&mut pg).await?;
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {} {:?}",data.len(), stringify!($table_name), elapsed_time);
                Ok(())
            }
            pub async fn insert(&self,mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>) -> anyhow::Result<$id_struct_name> {
                // let start_time = ::std::time::Instant::now();
                let record = diesel::insert_into(crate::schema::$table_name::table)
                    .values(self)
                    .get_result(&mut pg).await?;
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
                Ok(record)
            }
        }
    };
}


#[macro_export]
macro_rules! implement_diesel {
    ($struct_name:ident, $table_name:ident) => {
        impl $struct_name {
            pub async fn delete_one(&self, mut c: &mut Object<AsyncPgConnection>) {
                diesel::delete(
                    crate::schema::$table_name::dsl::$table_name.filter(crate::schema::$table_name::dsl::id.eq(self.id.clone()))
                ).execute(&mut c).await.unwrap();
            }
            pub async fn delete_by_id(id:i64, mut c: &mut Object<AsyncPgConnection>)->Self {
                diesel::delete(
                    crate::schema::$table_name::dsl::$table_name.filter(crate::schema::$table_name::dsl::id.eq(id.clone()))
                )
                .returning(crate::schema::$table_name::dsl::$table_name::all_columns())
                .get_result(&mut c)
                .await
                .unwrap()
            }
            pub async fn id(mut c: &mut Object<AsyncPgConnection>) -> i64 {
                crate::schema::$table_name::dsl::$table_name.select(
                    count_star()
                ).first(&mut c).await.unwrap()
            }
            pub async fn insert_bulk(mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>, data: Vec<Self>) -> anyhow::Result<()> {
                // let start_time = ::std::time::Instant::now();
                diesel::insert_into(crate::schema::$table_name::table)
                    .values(&data)
                    .execute(&mut pg).await?;
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {} {:?}",data.len(), stringify!($table_name), elapsed_time);
                Ok(())
            }
            pub async fn insert(&self,mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>) -> anyhow::Result<()> {
                // let start_time = ::std::time::Instant::now();
                diesel::insert_into(crate::schema::$table_name::table)
                    .values(self)
                    .execute(&mut pg).await?;
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
                Ok(())
            }

            pub async fn get_by_id(id:i64, mut c: &mut Object<AsyncPgConnection>) ->Option<Self>{
                crate::schema::$table_name::dsl::$table_name.filter(
                    crate::schema::$table_name::dsl::id.eq(id)
                ).first(&mut c).await.ok()
            }

            pub async fn get_all(mut c: &mut Object<AsyncPgConnection>) ->Vec<Self>{
                crate::schema::$table_name::dsl::$table_name.select(
                    $struct_name::as_select()
                ).load(&mut c).await.unwrap()
            }

        }
    };
}

#[macro_export]
macro_rules! implement_surreal {
    ($struct_name:ident, $table_name:ident) => {
        impl $struct_name {
            pub async fn upsert_bulk(mut pg: &Sdb, data: Vec<Self>) -> anyhow::Result<()> {
                let start_time = ::std::time::Instant::now();
                let tbl = stringify!($table_name);
                for x in &data {
                    pg.upsert::<Option<$struct_name>>((tbl,format!("{}",x.id)))
                        .content(x.clone())
                        .await?;
                }
                let elapsed_time = start_time.elapsed();
                info!("Time taken for bulk upsert: {} {} {:?}",data.len(), tbl, elapsed_time);
                Ok(())
            }
            pub async fn delete_one(&self, mut c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) {
                let tbl = stringify!($table_name);
                c.delete::<Option<Self>>((tbl, self.id.clone())).await.unwrap();
            }
            pub async fn id(mut c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) -> i64 {
                let sql = format!("SELECT count() FROM {}",stringify!($table_name));
                let a = c
                    .query(sql)
                    .await.unwrap().take::<Option<i64>>(0).unwrap();
                a.unwrap_or(0)
            }
            pub async fn insert_bulk(mut pg: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>, data: Vec<Self>) -> anyhow::Result<()> {
                // let start_time = ::std::time::Instant::now();
                pg.insert::<Vec<Self>>(stringify!($table_name)).content(vec![data]).await.unwrap();
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {} {:?}",data.len(), stringify!($table_name), elapsed_time);
                Ok(())
            }
            pub async fn insert(&self,mut pg: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) -> anyhow::Result<()> {
                // let start_time = ::std::time::Instant::now();
                pg
                    .insert::<Option<Self>>((stringify!($table_name), format!("{}",self.id)))
                    .content(self.clone())
                    .await.unwrap();
                // let elapsed_time = start_time.elapsed();
                // log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
                Ok(())
            }
            pub async fn get_all(mut c: &surrealdb::Surreal<surrealdb::engine::remote::ws::Client>) ->Vec<Self>{
                c.select(stringify!($table_name)).await.unwrap()
            }

        }
    };
}

#[macro_export]
macro_rules! implement_mongo_crud {
    ($struct_name:ident) => {
        impl $struct_name {
            pub async fn delete_one(&self, collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<()> {
                collection
                    .delete_one(mongodb::bson::doc! { "id": self.id })
                    .await?;
                Ok(())
            }

            pub async fn delete_by_id(
                id: i64,
                collection: &mongodb::Collection<$struct_name>,
            ) -> anyhow::Result<Option<$struct_name>> {
                let deleted = collection
                    .find_one_and_delete(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(deleted)
            }

            pub async fn count(collection: &mongodb::Collection<$struct_name>) -> i64 {
                let count = collection.count_documents(mongodb::bson::doc! {}).await.unwrap();
                count as i64
            }

            pub async fn insert_bulk(
                collection: &mongodb::Collection<$struct_name>,
                data: Vec<$struct_name>,
            ) -> anyhow::Result<()> {
                collection.insert_many(data).await?;
                Ok(())
            }

            pub async fn insert(&self, collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<Self> {
                collection.insert_one(self).await?;
                Ok(self.clone())
            }

            pub async fn get_by_id(
                id: i64,
                collection: &mongodb::Collection<$struct_name>,
            ) -> anyhow::Result<Option<$struct_name>> {
                let user = collection
                    .find_one(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(user)
            }

            pub async fn get_all(collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<Vec<$struct_name>> {
                use futures::TryStreamExt;
                let mut cursor = collection.find(mongodb::bson::doc! {}).await?;
                let users = cursor.try_collect().await?;
                Ok(users)
            }
        }
    };
}

#[macro_export]
macro_rules! implement_mongo_crud_struct {
    ($struct_name:ident) => {
        impl $struct_name {
            pub async fn delete_one(&self, collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<()> {
                collection
                    .delete_one(mongodb::bson::doc! { "id": self.id })
                    .await?;
                Ok(())
            }

            pub async fn delete_by_id(
                id: i64,
                collection: &mongodb::Collection<$struct_name>,
            ) -> anyhow::Result<Option<$struct_name>> {
                let deleted = collection
                    .find_one_and_delete(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(deleted)
            }

            pub async fn db_upsert_mongo(
                &self,
                c: &mongodb::Collection<Self>,
            ) {
                let filter = mongodb::bson::doc! { "id": self.id };
                let replace = mongodb::bson::doc! {
                    "$set": mongodb::bson::to_document(self).unwrap(),
                };
                c.update_one(filter,replace).upsert(true).await.unwrap();
            }

            pub async fn count(collection: &mongodb::Collection<$struct_name>) -> i64 {
                let count = collection.count_documents(mongodb::bson::doc! {}).await.unwrap();
                count as i64
            }

            pub async fn insert_bulk(
                collection: &mongodb::Collection<$struct_name>,
                data: Vec<$struct_name>,
            ) -> anyhow::Result<()> {
                collection.insert_many(data).await?;
                Ok(())
            }

            pub async fn insert(&self, collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<Self> {
                collection.insert_one(self).await?;
                Ok(self.clone())
            }

            pub async fn get_by_id(
                id: i64,
                collection: &mongodb::Collection<$struct_name>,
            ) -> anyhow::Result<Option<$struct_name>> {
                let user = collection
                    .find_one(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(user)
            }

            pub async fn get_all(collection: &mongodb::Collection<$struct_name>) -> anyhow::Result<Vec<$struct_name>> {
                use futures::TryStreamExt;
                let mut cursor = collection.find(mongodb::bson::doc! {}).await?;
                let users = cursor.try_collect::<Vec<_>>().await?;
                Ok(users)
            }
        }
    };
}

