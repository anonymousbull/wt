use crate::implement_mongo_crud_struct;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: i64,
    pub username: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackCreateDto {
    pub value: String,
}

implement_mongo_crud_struct!(Feedback);


