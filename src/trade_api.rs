use std::net::SocketAddr;
use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::chan::Chan;
use crate::constant::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use axum_server::tls_rustls::RustlsConfig;
use mongodb::Collection;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use solana_sdk::signature::Keypair;
use crate::bg_chan::bg_chan;
use crate::trade::Trade;
use crate::trade_chan::trade_chan;
use crate::trade_cmd::InternalCommand;
use crate::user_api::UserWithId;

/// Configuration to buy at current prices
#[derive(JsonSchema, Deserialize, Serialize, Clone,Debug)]
pub struct BuyPrompt {
    /// User ID
    #[schemars(skip)]
    pub kp: String,
    /// SOL amount
    pub sol_ui: Decimal,
    /// mint address
    pub mint: String,
    /// Take profit percent, should be positive number
    pub tp: Decimal,
    /// Stop loss percent, should be negative number
    pub sl: Decimal,
}

#[derive(Clone)]
struct ServerState {
    pub chan:Chan,
    pub trade_db: Collection<Trade>,
    pub user_db: Collection<UserWithId>,
    pub red:ConnectionManager
}

type HttpErrorResponse = (StatusCode,String);


fn get_api_key(headers: &HeaderMap) -> Result<String,HttpErrorResponse> {
    if let Some(auth_header) = headers.get("x-api-key") {
        if let Ok(api_key) = auth_header.to_str() {
            Ok(api_key.to_string())
        } else {
            Err((StatusCode::UNAUTHORIZED,"Unauthorized".to_string()))
        }
    } else {
        Err((StatusCode::UNAUTHORIZED,"Unauthorized".to_string()))
    }
}


pub async fn start(port:u16) {
    let f = kakfa_producer().await;
    let ssl_cert  = include_bytes!("../ssl/fullchain.pem").to_vec();
    let ssl_key  = include_bytes!("../ssl/privkey.pem").to_vec();
    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (ws_s, ws_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (dsl_s, dsl_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let (web_s, web_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
        dsl: dsl_s,
    };

    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            trade_chan(chan,trade_r).await;
        }
    });
    // let cors_layer = tower_http::cors::CorsLayer::permissive();
    let mon = mongo().await;
    let mut red = redis_pool().await;
    let app = Router::new()
        .route("/buy", post(buy))
        .with_state(ServerState {
            chan,
            trade_db: mon.collection::<Trade>("trades"),
            user_db: mon.collection::<UserWithId>("users"),
            red
        });
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let ssl = RustlsConfig::from_pem(ssl_cert,ssl_key).await.unwrap();
    axum_server::bind_rustls(addr, ssl)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn buy(
    State(mut state): State<ServerState>,
    Json(payload): Json<BuyPrompt>,
) -> Result<Json<Value>,HttpErrorResponse> {

    let kp = UserWithId::get_by_private_key(state.user_db,payload.kp.as_str()).await.ok_or(
        (StatusCode::NOT_FOUND,"Not Found".to_string())
    ).map(|x|Keypair::from_base58_string(x.private_key.as_str()))?;

    let mut mint = Trade::db_get_mint_red(&mut state.red, payload.mint.as_str())
        .await
        .map_err(internal_error)?;

    mint.root_kp = kp.to_bytes().to_vec();
    mint.cfg.tp = payload.tp;
    mint.cfg.sl = payload.sl;
    mint.cfg.max_sol = payload.sol_ui;
    let id = Trade::db_id_inc_red(&mut state.red).await;
    mint.id = id;
    state.chan.trade.try_send(InternalCommand::SwapRequest(mint)).unwrap();
    Ok(Json(json!({"id":id})))
}


/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: ToString,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

