
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::OnceLock;
use base64::Engine;
use mongodb::Client;
use mongodb::options::{ClientOptions, ServerApi, ServerApiVersion};
use rdkafka::ClientConfig;
use rdkafka::consumer::BaseConsumer;
use rdkafka::producer::FutureProducer;
use redis::aio::ConnectionManager;
use rust_decimal::Decimal;
use raydium_amm::solana_program::native_token::LAMPORTS_PER_SOL;



pub const BLOX_HEADER: &str = env!("BLOX_HEADER");
pub const NEXT_BLOCK: &str = env!("NEXT_BLOCK");
pub const BASE_64_SSH_PRIVATE_KEY: &str = env!("SSH_PRIVATE_KEY");




pub const PUMP_MIGRATION_PRICE:f64 = 0.0000004108264862252296;






pub const DO_API: &str = env!("DO_API");


pub struct Cfg {
    pub solana_ws_url: &'static str,
}

pub static KEYPAIR: OnceLock<Keypair> = OnceLock::new();
pub const INTERNAL_KP: &str = env!("INTERNAL_KP");

pub const JITO_TIPS:[&str;8] = [
    "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
    "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
    "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
    "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
    "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
    "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
    "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
];

pub const PUMP_SWAP_CODE:[u8;8] = [189, 219, 127, 211, 78, 230, 97, 238];
pub const PUMP_BUY_CODE:[u8;8] = [102, 6, 61, 18, 1, 218, 235, 234];
pub const PUMP_SELL_CODE:[u8;8] = [51, 230, 133, 164, 1, 127, 131, 173];

pub const SOLANA_MINT_STR: &str = "So11111111111111111111111111111111111111112";
pub const SOLANA_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

pub const PUMP_MIGRATION:Pubkey = Pubkey::from_str_const("39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg");
pub const PUMP_PROGRAM:Pubkey = Pubkey::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMP_GLOBAL:Pubkey = Pubkey::from_str_const("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMP_EVENT_AUTHORITY:Pubkey = Pubkey::from_str_const("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMP_FEE:Pubkey = Pubkey::from_str_const("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");


pub const RPC1: &str = env!("RPC1");
pub const RPC1_NAME: &str = env!("RPC1_NAME");
pub const RPC1_LOCATION: &str = env!("RPC1_LOCATION");

pub const RPC2: &str = env!("RPC2");
pub const RPC2_NAME: &str = env!("RPC2_NAME");
pub const RPC2_LOCATION: &str = env!("RPC2_LOCATION");

pub const RPC3: &str = env!("RPC3");
pub const RPC3_NAME: &str = env!("RPC3_NAME");
pub const RPC3_LOCATION: &str = env!("RPC3_LOCATION");

pub const JITO1: &str = env!("JITO1");
pub const JITO1_LOCATION: &str = env!("JITO1_LOCATION");

pub const JITO2: &str = env!("JITO2");
pub const JITO2_LOCATION: &str = env!("JITO2_LOCATION");

pub const GRPC1: &str = env!("GRPC1");
pub const GRPC1_NAME: &str = env!("GRPC1_NAME");
pub const GRPC_LOCATION: &str = env!("GRPC1_LOCATION");

pub const KAFKA_URL: &str = env!("KAFKA_URL");
pub const PG_URL: &str = env!("PG_URL");
pub const REDIS_URL: &str = env!("REDIS_URL");

pub const RAYDIUM_V4_PROGRAM:Pubkey = Pubkey::from_str_const("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const RAYDIUM_V4_AUTHORITY:Pubkey = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
pub const SOLANA_SYSTEM_PROGRAM:Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");
pub const SOLANA_RENT_PROGRAM:Pubkey = Pubkey::from_str_const("SysvarRent111111111111111111111111111111111");
pub const SOLANA_SERUM_PROGRAM:Pubkey = Pubkey::from_str_const("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
pub const SOLANA_ATA_PROGRAM:Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const MONGO_URL: &str = env!("MONGO_URL");

const MONGO_CON: tokio::sync::OnceCell<mongodb::Database> = tokio::sync::OnceCell::const_new();

pub async fn mongo() -> mongodb::Database {
    MONGO_CON
        .get_or_init(|| async {
            // Replace the placeholder with your Atlas connection string
            let mut client_options = ClientOptions::parse(MONGO_URL).await.unwrap();
            // Set the server_api field of the client_options object to Stable API version 1
            let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
            client_options.server_api = Some(server_api);
            // Create a new client and connect to the server
            let client = Client::with_options(client_options).unwrap();
            client.database("wt")
        })
        .await
        .clone()
}
const RED: tokio::sync::OnceCell<ConnectionManager> = tokio::sync::OnceCell::const_new();
const KAFKA: tokio::sync::OnceCell<FutureProducer> = tokio::sync::OnceCell::const_new();

pub async fn redis_pool() -> ConnectionManager {
    RED
        .get_or_init(|| async {
            let client = redis::Client::open(REDIS_URL).unwrap();
            client.get_connection_manager().await.unwrap()
        })
        .await
        .clone()
}

pub async fn kakfa_producer() -> FutureProducer {
    KAFKA.get_or_init(|| async {
        let user_access_key = base64::engine::general_purpose::STANDARD.decode(
            env!("KAFKA_USER_ACCESS_KEY")
        ).unwrap();
        let user_access_cert = base64::engine::general_purpose::STANDARD.decode(
            env!("KAFKA_USER_ACCESS_CERTIFICATE")
        ).unwrap();
        let ca = base64::engine::general_purpose::STANDARD.decode(
            env!("KAFKA_CA")
        ).unwrap();
        tokio::fs::write("user-access-key.key",user_access_key).await.unwrap();
        tokio::fs::write("user-access-certificate.crt",user_access_cert).await.unwrap();
        tokio::fs::write("ca-cert.pem",ca).await.unwrap();

        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", KAFKA_URL)
            .set("security.protocol", "SSL",)
            .set("ssl.ca.location", "ca-cert.pem")// Adjust if using a different server
            .set("enable.ssl.certificate.verification", "false")
            .set("ssl.key.location", "user-access-key.key") // Path to user access key
            .set("ssl.certificate.location", "user-access-certificate.crt")
            .create()
            .expect("Producer creation failed");
        producer
    })
        .await
        .clone()
}


pub fn get_keypair() -> &'static Keypair {
    KEYPAIR.get_or_init(|| {
        Keypair::from_base58_string(
            INTERNAL_KP
        )
    })
}

pub fn geyser() -> yellowstone_grpc_client::GeyserGrpcBuilder {
    yellowstone_grpc_client::GeyserGrpcClient::build_from_static(GRPC1)
}





