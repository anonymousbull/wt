use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;

#[derive(Serialize, Deserialize)]
pub struct DnsRecord {
    pub r#type: String,
    pub name: String,
    pub data: String,
    pub ttl: u32,
}

#[derive(Deserialize)]
struct DnsRecordResponse {
    domain_record: DomainRecord,
}

#[derive(Deserialize)]
struct DomainRecord {
    id: u64,
    r#type: String,
    name: String,
    data: String,
    ttl: u32,
}

pub async fn manage_dns_records(domain: &str, mut dns_record: DnsRecord) -> Result<u64> {
    dns_record.name = dns_record.name.replace("_", "-");
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    // Fetch existing DNS records
    let response = client.get(&format!("https://api.digitalocean.com/v2/domains/{}/records", domain))
        .header("Authorization", format!("Bearer {}", do_api_key))
        .send()
        .await
        .context("Failed to fetch DNS records")?;

    let records: Vec<DomainRecord> = response.json().await.context("Failed to parse DNS records")?;

    // Check if the record already exists
    if let Some(existing_record) = records.iter().find(|record| {
        record.r#type == dns_record.r#type && record.name == dns_record.name
    }) {
        // Update the existing record if necessary
        if existing_record.data != dns_record.data || existing_record.ttl != dns_record.ttl {
            let update_response = client.put(&format!(
                "https://api.digitalocean.com/v2/domains/{}/records/{}",
                domain, existing_record.id
            ))
            .header("Authorization", format!("Bearer {}", do_api_key))
            .json(&dns_record)
            .send()
            .await
            .context("Failed to update DNS record")?;

            update_response.error_for_status().context("Failed to update DNS record")?;
            return Ok(existing_record.id);
        } else {
            // Record is already up-to-date
            return Ok(existing_record.id);
        }
    }

    // Create a new record if it doesn't exist
    let create_response = client.post(&format!(
        "https://api.digitalocean.com/v2/domains/{}/records",
        domain
    ))
    .header("Authorization", format!("Bearer {}", do_api_key))
    .json(&dns_record)
    .send()
    .await
    .context("Failed to create DNS record")?;

    let record_response: DnsRecordResponse = create_response.json().await.context("Failed to parse response")?;
    Ok(record_response.domain_record.id)
} 