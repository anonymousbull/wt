use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::AsyncPgConnection;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use std::sync::OnceLock;

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

pub static DB: std::sync::LazyLock<surrealdb::Surreal<surrealdb::engine::remote::ws::Client>> =
    std::sync::LazyLock::new(surrealdb::Surreal::init);

pub type Sdb = surrealdb::Surreal<surrealdb::engine::remote::ws::Client>;

pub static KEYPAIR: OnceLock<Keypair> = OnceLock::new();
pub const SOLANA_WS_URL: &str = env!("SOLANA_WSS_URL");
pub const SURREAL_DB_URL: &str = env!("SURREAL_DB_URL");
pub const ENABLE_WEBSOCKET: &str = env!("ENABLE_WEBSOCKET");
pub const BLOX_HEADER: &str = env!("BLOX_HEADER");
pub const NEXT_BLOCK: &str = env!("NEXT_BLOCK");
pub const SOLANA_MINT_STR: &str = "So11111111111111111111111111111111111111112";
pub const SOLANA_MINT: Pubkey = Pubkey::from_str_const("So11111111111111111111111111111111111111112");
pub const PUMP_MIGRATION:Pubkey = Pubkey::from_str_const("39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg");
pub const PUMP_PROGRAM:Pubkey = Pubkey::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const SOLANA_PK: &str = env!("SOLANA_PK");
pub const SOLANA_RPC_URL1: &str = env!("SOLANA_RPC_URL");
pub const SOLANA_RPC_URL2: &str = env!("SOLANA_RPC_URL2");
pub const SOLANA_RPC_URL3: &str = env!("SOLANA_RPC_URL3");
pub const JITO_URL1: &str = env!("JITO_URL1");
pub const JITO_URL2: &str = env!("JITO_URL2");
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
    RpcClient::new_with_commitment(SOLANA_RPC_URL1.to_string(), CommitmentConfig::processed())
}
pub fn geyser() -> yellowstone_grpc_client::GeyserGrpcBuilder {
    yellowstone_grpc_client::GeyserGrpcClient::build_from_static(SOLANA_GRPC_URL)
}

pub fn rpc1() -> RpcClient {
    RpcClient::new_with_commitment(SOLANA_RPC_URL1.to_string(), CommitmentConfig::processed())
}
pub fn rpc2() -> RpcClient {
    RpcClient::new_with_commitment(SOLANA_RPC_URL2.to_string(), CommitmentConfig::processed())
}
pub fn rpc3() -> RpcClient {
    RpcClient::new_with_commitment(SOLANA_RPC_URL3.to_string(), CommitmentConfig::processed())
}
pub fn jito1() -> jito_sdk_rust::JitoJsonRpcSDK {
    jito_sdk_rust::JitoJsonRpcSDK::new(JITO_URL1, None)
}
pub fn jito2() -> jito_sdk_rust::JitoJsonRpcSDK {
    jito_sdk_rust::JitoJsonRpcSDK::new(JITO_URL1, None)
}

pub fn rpcs() -> SolanaRpcs {
    SolanaRpcs {
        rpc1:RpcClient::new_with_commitment(SOLANA_RPC_URL1.to_string(), CommitmentConfig::processed()),
        rpc2:RpcClient::new_with_commitment(SOLANA_RPC_URL2.to_string(), CommitmentConfig::processed()),
        rpc3:RpcClient::new_with_commitment(SOLANA_RPC_URL3.to_string(), CommitmentConfig::processed()),
        jito1: jito_sdk_rust::JitoJsonRpcSDK::new(env!("JITO_URL1"), None),
        jito2: jito_sdk_rust::JitoJsonRpcSDK::new(env!("JITO_URL2"), None),
    }
}

pub struct SolanaRpcs {
    rpc1: RpcClient,
    rpc2: RpcClient,
    rpc3: RpcClient,
    jito1: jito_sdk_rust::JitoJsonRpcSDK,
    jito2: jito_sdk_rust::JitoJsonRpcSDK,
}

