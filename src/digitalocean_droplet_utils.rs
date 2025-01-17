use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Deserialize,Serialize)]
struct Droplet {
    id: u64,
    name: String,
}

#[derive(Deserialize,Serialize)]
struct DropletsResponse {
    droplets: Vec<Droplet>,
}

pub async fn droplet_exists_by_name(droplet_name: &str) -> Result<bool> {
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    let response = client.get("https://api.digitalocean.com/v2/droplets")
        .header("Authorization", format!("Bearer {}", do_api_key))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Request error: {}", e))?
        .error_for_status()
        .with_context(|| "Failed to fetch droplets")?;

    let droplets_response: DropletsResponse = response.json().await.context("Failed to parse response")?;

    Ok(droplets_response.droplets.iter().any(|d| d.name == droplet_name))
}

pub async fn get_droplet_id_by_name(droplet_name: &str) -> Result<Option<u64>> {
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    let response = client.get("https://api.digitalocean.com/v2/droplets")
        .header("Authorization", format!("Bearer {}", do_api_key))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Request error: {}", e))?
        .error_for_status()
        .with_context(|| "Failed to fetch droplets")?;

    let droplets_response: DropletsResponse = response.json().await.context("Failed to parse response")?;

    Ok(droplets_response.droplets.iter().find(|d| d.name == droplet_name).map(|d| d.id))
} 