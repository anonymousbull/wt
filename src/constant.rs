use std::sync::OnceLock;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::pooled_connection::deadpool::Pool;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;

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

pub static KEYPAIR: OnceLock<Keypair> = OnceLock::new();
pub const SOLANA_WS_URL: &str = env!("SOLANA_WSS_URL");
pub const BLOX_HEADER: &str = env!("BLOX_HEADER");
pub const SOLANA_MINT_STR: &str = "So11111111111111111111111111111111111111112";
pub const SOLANA_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
pub const PUMP_MIGRATION:Pubkey = Pubkey::from_str_const("39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg");
pub const SOLANA_PK: &str = env!("SOLANA_PK");
pub const SOLANA_RPC_URL: &str = env!("SOLANA_RPC_URL");
pub const SOLANA_GRPC_URL: &str = env!("SOLANA_GRPC_URL");
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

pub fn solana_rpc_client() -> RpcClient {
    RpcClient::new_with_commitment(SOLANA_RPC_URL.to_string(), CommitmentConfig::processed())
}