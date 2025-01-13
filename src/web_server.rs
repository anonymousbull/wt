use crate::chan::Chan;
use crate::trade_type::{Trade, TradeRequest2};
use axum::extract::State;
use axum::routing::post;
use axum::{extract, Json, Router};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use mongodb::Collection;
use solana_sdk::signature::Keypair;
use tower_http::cors::Any;
use crate::constant::mongo;
use crate::prompt_type::BuyPrompt;

#[derive(Clone)]
struct ServerState {
    pub chan:Chan,
    pub db: Collection<Trade>,
}



pub async fn start(chan: Chan) {
    let cors_layer = tower_http::cors::CorsLayer::permissive();
    let mon = mongo().await;
    let col = mon.collection::<Trade>("traders");
    let app = Router::new()
        .route("/buy", post(buy))
        .with_state(ServerState {
            chan,
            db: col
        })
        .layer(cors_layer);
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn buy(
    State(state): State<ServerState>,
    Json(payload): Json<BuyPrompt>,
) -> axum::response::Result<&'static str> {
    let mut trade = Trade::login(payload.kp.as_str(), state.chan.clone()).await
        .map_err(|e|e.to_string())?;
    trade = trade.buy_now(state.chan.clone()).await
        .map_err(|e|e.to_string())?;
    Ok("it worked!")
}
