use crate::chan::Chan;
use crate::constant::*;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use mongodb::Collection;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use solana_sdk::signature::Keypair;
use crate::api::BuyPrompt;
use crate::trade::Trade;
use users_api::db::UserDbClient;
use crate::event::InternalCommand;

#[derive(Clone)]
struct ServerState {
    pub chan:Chan,
    pub trade_db: Collection<Trade>,
    pub user_db: UserDbClient,
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


pub async fn start(chan: Chan) {
    // let cors_layer = tower_http::cors::CorsLayer::permissive();
    let mon = mongo().await;
    let mut red = redis_pool().await;
    let user_db = UserDbClient::new(MONGO_URL,MONGO_DB_NAME,USER_COL).await;
    let app = Router::new()
        .route("/buy", post(buy))
        .with_state(ServerState {
            chan,
            trade_db: mon.collection::<Trade>("trades"),
            user_db,
            red
        });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn buy(
    State(mut state): State<ServerState>,
    Json(payload): Json<BuyPrompt>,
) -> Result<Json<Value>,HttpErrorResponse> {

    let kp = state.user_db.get_by_private_key(payload.kp.as_str()).await.ok_or(
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