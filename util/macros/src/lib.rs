

#[macro_export]
macro_rules! implement_mongo_crud {
    ($client_name:ident,$struct_name:ident) => {

        #[derive(Debug, Clone)]
        pub struct $client_name {
            client: mongodb::Client,
            db: mongodb::Database,
            pub col: mongodb::Collection<$struct_name>,
        }

        impl $client_name {
            pub async fn new(url:&str,db:&str,col:&str) -> $client_name {
                let client = mongodb::Client::with_uri_str(url).await.unwrap();
                let db = client.database(db);
                let col = db.collection::<$struct_name>(col);
                $client_name {
                    client,
                    db,
                    col
                }
            }

            pub async fn delete_one(&self, entity:&$struct_name) -> anyhow::Result<()> {
                self.col
                    .delete_one(mongodb::bson::doc! { "id": entity.id })
                    .await?;
                Ok(())
            }

            pub async fn delete_by_id(
                &self,
                id: i64,
            ) -> anyhow::Result<Option<$struct_name>> {
                let deleted = self.col
                    .find_one_and_delete(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(deleted)
            }

            pub async fn count(&self) -> i64 {
                let count = self.col.count_documents(mongodb::bson::doc! {}).await.unwrap();
                count as i64
            }

            pub async fn insert_bulk(
                &self,
                data: Vec<$struct_name>,
            ) -> anyhow::Result<()> {
                self.col.insert_many(data).await?;
                Ok(())
            }

            pub async fn insert(&self,entity:$struct_name) -> anyhow::Result<$struct_name> {
                self.col.insert_one(entity.clone()).await?;
                Ok(entity.clone())
            }

            pub async fn get_by_id(
                &self,
                id: i64,
            ) -> anyhow::Result<Option<$struct_name>> {
                let user = self.col
                    .find_one(mongodb::bson::doc! { "id": id })
                    .await?;
                Ok(user)
            }

            pub async fn get_all(&self) -> anyhow::Result<Vec<$struct_name>> {
                use futures::TryStreamExt;

                let mut cursor = self.col.find(mongodb::bson::doc! {}).await?;
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
                let replace = mongodb::bson::to_document(self).unwrap();
                c.update_one(filter,replace).await.unwrap();
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
                let mut cursor = collection.find(mongodb::bson::doc! {}).await?;
                let users = cursor.try_collect().await?;
                Ok(users)
            }
        }
    };
}