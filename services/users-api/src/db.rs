use macros::implement_mongo_crud;
use crate::UserWithId;

implement_mongo_crud!(UserDbClient,UserWithId);

impl UserDbClient {
    pub async fn get_by_private_key(&self, private_key: &str) -> Option<UserWithId> {
        let user = self.col
            .find_one(mongodb::bson::doc! { "private_key": "65fKKEPAf8CUUmbtTAaLudPXHW8ZigaDRFE3cdbQSgy7WdSNZv3eWfXChvLSM8NfSfxteLAFesbLa7FksNG6zwDu" })
            .await.unwrap();
        println!("{:?}",user);
        user
    }
}