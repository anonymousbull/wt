use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::OnceLock;
use rust_decimal::Decimal;
use raydium_amm::solana_program::native_token::LAMPORTS_PER_SOL;

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

pub static KEYPAIR: OnceLock<Keypair> = OnceLock::new();
pub const SURREAL_DB_URL: &str = env!("SURREAL_DB_URL");
pub const ENABLE_WEBSOCKET: &str = env!("ENABLE_WEBSOCKET");
pub const BLOX_HEADER: &str = env!("BLOX_HEADER");
pub const NEXT_BLOCK: &str = env!("NEXT_BLOCK");
pub const SOLANA_MINT_STR: &str = "So11111111111111111111111111111111111111112";
pub const SOLANA_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");

pub const PUMP_MIGRATION:Pubkey = Pubkey::from_str_const("39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg");
pub const PUMP_PROGRAM:Pubkey = Pubkey::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMP_GLOBAL:Pubkey = Pubkey::from_str_const("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMP_EVENT_AUTHORITY:Pubkey = Pubkey::from_str_const("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const PUMP_FEE:Pubkey = Pubkey::from_str_const("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");


pub const SOLANA_PK: &str = env!("SOLANA_PK");


    pub const PUMP_MIGRATION_PRICE:f64 = 0.0000004108264862252296;


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

pub const PG_URL: &str = env!("PG_URL");
pub const RAYDIUM_V4_PROGRAM:Pubkey = Pubkey::from_str_const("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
pub const RAYDIUM_V4_AUTHORITY:Pubkey = Pubkey::from_str_const("5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1");
pub const SOLANA_SYSTEM_PROGRAM:Pubkey = Pubkey::from_str_const("11111111111111111111111111111111");
pub const SOLANA_RENT_PROGRAM:Pubkey = Pubkey::from_str_const("SysvarRent111111111111111111111111111111111");
pub const SOLANA_SERUM_PROGRAM:Pubkey = Pubkey::from_str_const("srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX");
pub const SOLANA_ATA_PROGRAM:Pubkey = Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
const PG_CON: tokio::sync::OnceCell<Pool<AsyncPgConnection>> = tokio::sync::OnceCell::const_new();

pub async fn pg_conn() -> Pool<AsyncPgConnection> {
    PG_CON
        .get_or_init(|| async {
            let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(PG_URL);
            let pool = Pool::builder(config).build().unwrap();
            pool
        })
        .await
        .clone()
}

pub struct Cfg {
    pub solana_ws_url: &'static str,
}

pub fn get_keypair() -> Keypair {
    KEYPAIR.get_or_init(|| {
        Keypair::from_base58_string(
            SOLANA_PK,
        )
    }).insecure_clone()
}

