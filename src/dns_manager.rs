use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;

#[derive(Serialize,Deserialize)]
pub struct DnsRecord {
    pub r#type: String,
    pub name: String,
    pub data: String,
    pub ttl: u32,
}

impl DnsRecord {
    pub fn new(r#type: &str, name: &str, data: &str, ttl: u32) -> Self {
        DnsRecord {
            r#type: r#type.to_string(),
            name: name.to_string(),
            data: data.to_string(),
            ttl,
        }
    }
}

#[derive(Deserialize)]
struct DnsRecordResponse {
    domain_record: DomainRecord,
}

#[derive(Deserialize)]
struct DomainRecord {
    id: u64,
}

pub async fn manage_dns_records(domain: &str, mut dns_record: DnsRecord) -> Result<u64> {
    dns_record.name = dns_record.name.replace("_","-");
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    let response = client.post(&format!("https://api.digitalocean.com/v2/domains/{}/records", domain))
        .header("Authorization", format!("Bearer {}", do_api_key))
        .json(&dns_record)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("Request error: {}", e))?
        .error_for_status()
        .with_context(|| "Failed to process DNS record response")?;

    let record_response: DnsRecordResponse = response.json().await.context("Failed to parse response")?;
    Ok(record_response.domain_record.id)
} 