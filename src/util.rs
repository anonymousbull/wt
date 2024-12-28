

pub mod diesel_export {
    pub use diesel_async::pooled_connection::deadpool::Pool;
    pub use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    pub use diesel_async::pooled_connection::deadpool::Object;
    pub use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
    pub use diesel::dsl::count_star;
    pub use diesel::prelude::*;
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
            pub async fn id(mut c: &mut Object<AsyncPgConnection>) -> i64 {
                crate::schema::$table_name::dsl::$table_name.select(
                    count_star()
                ).first(&mut c).await.unwrap()
            }
            pub async fn insert_bulk(mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>, data: Vec<Self>) -> anyhow::Result<()> {
                let start_time = ::std::time::Instant::now();
                diesel::insert_into(crate::schema::$table_name::table)
                    .values(&data)
                    .execute(&mut pg).await?;
                let elapsed_time = start_time.elapsed();
                log::info!("Time taken for bulk insert: {} {} {:?}",data.len(), stringify!($table_name), elapsed_time);
                Ok(())
            }
            pub async fn insert(&self,mut pg: &mut diesel_async::pooled_connection::deadpool::Object<diesel_async::AsyncPgConnection>) -> anyhow::Result<()> {
                let start_time = ::std::time::Instant::now();
                diesel::insert_into(crate::schema::$table_name::table)
                    .values(self)
                    .execute(&mut pg).await?;
                let elapsed_time = start_time.elapsed();
                log::info!("Time taken for bulk insert: {} {:?}", stringify!($table_name), elapsed_time);
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