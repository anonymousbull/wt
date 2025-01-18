use std::time::Duration;
use tokio::time::{sleep, timeout};
use anyhow::{Result, Context};
use reqwest::Client;
use serde_json::json;
use crate::constant::DO_API;
use std::env;
use crate::digitalocean_droplet_utils::get_droplet_id_by_name;

pub enum DropletSize {
    Small,
}

impl DropletSize {
    pub fn to_str(&self) -> &str {
        match self {
            DropletSize::Small => "s-1vcpu-1gb",
        }
    }
}

impl Default for DropletSize {
    fn default() -> Self {
        DropletSize::Small
    }
}

pub async fn create_digitalocean_droplet(droplet_name: &str, size: DropletSize, ssh_public_keys: Vec<String>) -> Result<String> {
    let droplet_name = droplet_name.replace("_","-");
    let droplet_name = droplet_name.as_str();
    let client = Client::new();

    // Check if the droplet already exists
    let droplets_response = client.get("https://api.digitalocean.com/v2/droplets")
        .header("Authorization", format!("Bearer {}", DO_API))
        .send()
        .await
        .context("Failed to fetch droplets list")?
        .error_for_status()
        .context("Failed to process droplets list response")?;

    let droplets = droplets_response.json::<serde_json::Value>().await?;
    if let Some(droplets_array) = droplets.get("droplets").and_then(|d| d.as_array()) {
        if let Some(existing_droplet) = droplets_array.iter().find(|d| d.get("name").and_then(|n| n.as_str()) == Some(droplet_name)) {
            println!("Droplet '{}' already exists.", droplet_name);
            return get_droplet_ip(existing_droplet);
        }
    }

    // Create the droplet if it doesn't exist
    let user_data_script = ssh_public_keys.iter().fold(String::from("#!/bin/bash\nmkdir -p /root/.ssh\n"), |mut acc, key| {
        acc.push_str(&format!("echo '{}' >> /root/.ssh/authorized_keys\n", key));
        acc
    });

    let droplet_details = json!({
        "name": droplet_name,
        "region": "lon1", // London region
        "size": size.to_str(),
        "image": "ubuntu-24-10-x64", // Default to Ubuntu 20.04 LTS
        "ssh_keys": vec!["44955098"], // No SSH key IDs are needed since we're using user_data
        "backups": false,
        "ipv6": true,
        "user_data": null,
        "private_networking": null,
        "volumes": null,
        "tags": [
            "web"
        ]
    });

    let response = client.post("https://api.digitalocean.com/v2/droplets")
        .header("Authorization", format!("Bearer {}", DO_API))
        .json(&droplet_details)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create droplet: {}", e))?
        .error_for_status()
        .with_context(|| "Failed to process droplet creation response")?;

    let droplet_id = response.json::<serde_json::Value>().await?
        .get("droplet")
        .and_then(|d| d.get("id"))
        .and_then(|id| id.as_u64())
        .context("Failed to extract droplet ID")?;

    let result = timeout(Duration::from_secs(60), async {
        loop {
            let status_response = client.get(&format!("https://api.digitalocean.com/v2/droplets/{}", droplet_id))
                .header("Authorization", format!("Bearer {}", DO_API))
                .send()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to check droplet status: {}", e))?;

            let status_json = status_response.json::<serde_json::Value>().await?;
            let status = status_json
                .get("droplet")
                .and_then(|d| d.get("status"))
                .and_then(|status| status.as_str())
                .context("Failed to extract droplet status")?;

            if status == "active" {
                println!("Droplet '{}' is now active.", droplet_name);
                return get_droplet_ip(status_json.get("droplet").context("Failed to get droplet details")?);
            } else {
                println!("Droplet status: {}. Waiting for it to become active...", status);
                sleep(Duration::from_secs(10)).await;
            }
        }
    }).await;

    result.map_err(|_| anyhow::anyhow!("Timed out waiting for droplet to become active"))?
}

fn get_droplet_ip(droplet: &serde_json::Value) -> Result<String> {
    droplet.get("networks")
        .and_then(|n| n.get("v4"))
        .and_then(|v4| v4.as_array())
        .and_then(|arr| arr.iter().find(|net| net.get("type").and_then(|t| t.as_str()) == Some("public")))
        .and_then(|net| net.get("ip_address"))
        .and_then(|ip| ip.as_str())
        .map(|ip| ip.to_string())
        .context("Failed to extract public IP address")
}

pub async fn delete_droplet_by_name(droplet_name: &str) -> Result<()> {
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    if let Some(droplet_id) = get_droplet_id_by_name(droplet_name).await? {
        client.delete(&format!("https://api.digitalocean.com/v2/droplets/{}", droplet_id))
            .header("Authorization", format!("Bearer {}", do_api_key))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete droplet: {}", e))?
            .error_for_status()
            .with_context(|| format!("Failed to delete droplet with ID {}", droplet_id))?;

        println!("Droplet '{}' deleted successfully.", droplet_name);
    } else {
        println!("Droplet '{}' not found.", droplet_name);
    }

    Ok(())
} 