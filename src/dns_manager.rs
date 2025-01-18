use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use tracing_subscriber::fmt::format;

#[derive(Serialize, Deserialize,Debug)]
pub struct DnsRecord {
    pub r#type: String,
    pub name: String,
    pub data: String,
    pub ttl: u32,
}

#[derive(Deserialize)]
struct DnsRecordsResponse {
    domain_records: Vec<DomainRecord>,
}

#[derive(Deserialize,Debug)]
struct DomainRecord {
    id: u64,
    r#type: String,
    name: String,
    data: String,
    ttl: u32,
    priority: Option<u32>,
    port: Option<u32>,
    weight: Option<u32>,
    flags: Option<u32>,
    tag: Option<String>,
}

#[derive(Deserialize)]
struct CreateDnsRecordResponse {
    domain_record: DomainRecord,
}

pub async fn manage_dns_records(host: &str, mut dns_record: DnsRecord) -> Result<u64> {
    dns_record.name = dns_record.name.replace("_", "-");
    let domain = format!("{}.{host}",dns_record.name);
    let client = Client::new();
    let do_api_key = env::var("DO_API").expect("DO_API environment variable must be set");

    // Fetch existing DNS records
    let response = client.get(&format!("https://api.digitalocean.com/v2/domains/{host}/records?name={domain}&type={}&per_page=100&page=1",dns_record.r#type))
        .header("Authorization", format!("Bearer {}", do_api_key))
        .send()
        .await
        .context("Failed to fetch DNS records")?;

    let records_response: DnsRecordsResponse = response.json().await.context("Failed to parse DNS records")?;
    let records = records_response.domain_records;

    // Check if the record already exists
    if let Some(existing_record) = records.iter().find(|record| {
        record.r#type == dns_record.r#type && (record.name.starts_with(dns_record.name.as_str()))
    }) {
        // Update the existing record if necessary
        if existing_record.data != dns_record.data  {
            let update_response = client.put(&format!(
                "https://api.digitalocean.com/v2/domains/{}/records/{}",
                host, existing_record.id
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
    println!("we are exisg111");

    // Create a new record if it doesn't exist
    let create_response = client.post(&format!(
        "https://api.digitalocean.com/v2/domains/{}/records",
        host
    ))
    .header("Authorization", format!("Bearer {}", do_api_key))
    .json(&dns_record)
    .send()
    .await
    .context("Failed to create DNS record")?;

    let record_response: CreateDnsRecordResponse = create_response.json().await.context("Failed to parse response")?;
    Ok(record_response.domain_record.id)
} 