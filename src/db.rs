pub mod diesel_export {
    pub use diesel_async::pooled_connection::deadpool::Pool;
    pub use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    pub use diesel_async::pooled_connection::deadpool::Object;
    pub use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
    pub use diesel::dsl::count_star;
    pub use diesel::prelude::*;
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
macro_rules! implement_diesel {
    ($struct_name:ident, $table_name:ident) => {
        impl $struct_name {
            pub async fn delete_one(&self, mut c: &mut Object<AsyncPgConnection>) {
                diesel::delete(
                    crate::schema::$table_name::dsl::$table_name.filter(crate::schema::$table_name::dsl::id.eq(self.id.clone()))
                ).execute(&mut c).await.unwrap();
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