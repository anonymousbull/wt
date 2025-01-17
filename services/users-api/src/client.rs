use reqwest::Client;
use anyhow::{Result, Context};
use crate::UserWithId;

pub struct ReqwestClient {
    client: Client,
    base_url: String,
    api_key: String,
}

impl ReqwestClient {
    pub fn new(base_url: &str, api_key:&str) -> Self {
        let client = Client::new();
        ReqwestClient {
            client,
            api_key:api_key.to_string(),
            base_url: base_url.to_string(),
        }
    }

    pub async fn create_user(&self, user: UserWithId) -> Result<UserWithId> {
        let response = self.client
            .post(&format!("{}/users", self.base_url))
            .header("x-api-key",self.api_key.as_str())
            .json(&user)
            .send()
            .await
            .context("Failed to send request to create user")?;

        if response.status().is_success() {
            let created_user: UserWithId = response.json().await.context("Failed to parse response")?;
            Ok(created_user)
        } else {
            Err(anyhow::anyhow!("Failed to create user: {}", response.status()))
        }
    }

    pub async fn get_users(&self) -> Result<Vec<UserWithId>> {
        let response = self.client
            .get(&format!("{}/users", self.base_url))
            .header("x-api-key",self.api_key.as_str())
            .send()
            .await
            .context("Failed to send request to get users")?;

        if response.status().is_success() {
            let users: Vec<UserWithId> = response.json().await.context("Failed to parse response")?;
            Ok(users)
        } else {
            Err(anyhow::anyhow!("Failed to get users: {}", response.status()))
        }
    }

    pub async fn get_user(&self, id: i64) -> Result<UserWithId> {
        let response = self.client
            .get(&format!("{}/users/{}", self.base_url, id))
            .header("x-api-key",self.api_key.as_str())
            .send()
            .await
            .context("Failed to send request to get user")?;

        if response.status().is_success() {
            let user: UserWithId = response.json().await.context("Failed to parse response")?;
            Ok(user)
        } else {
            Err(anyhow::anyhow!("Failed to get user: {}", response.status()))
        }
    }

    pub async fn delete_user(&self, id: i64) -> Result<UserWithId> {
        let response = self.client
            .delete(&format!("{}/users/{}", self.base_url, id))
            .header("x-api-key",self.api_key.as_str())
            .send()
            .await
            .context("Failed to send request to delete user")?;

        if response.status().is_success() {
            let deleted_user: UserWithId = response.json().await.context("Failed to parse response")?;
            Ok(deleted_user)
        } else {
            Err(anyhow::anyhow!("Failed to delete user: {}", response.status()))
        }
    }
}
