use crate::chan::Chan;
use crate::prompt_type::BuyPrompt;
use crate::trade_type::Trade;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use mongodb::Collection;
use redis::aio::ConnectionManager;
use solana_sdk::signature::Keypair;
use users_api::db::UserDbClient;
//
// #[derive(Clone)]
// struct ServerState {
//     pub chan:Chan,
//     pub trade_db: Collection<Trade>,
//     pub user_db: UserDbClient,
//     pub red:ConnectionManager
// }
//
// type HttpErrorResponse = (StatusCode,String);
//
//
// fn get_api_key(headers: &HeaderMap) -> Result<String,HttpErrorResponse> {
//     if let Some(auth_header) = headers.get("x-api-key") {
//         if let Ok(api_key) = auth_header.to_str() {
//             Ok(api_key.to_string())
//         } else {
//             Err((StatusCode::UNAUTHORIZED,"Unauthorized".to_string()))
//         }
//     } else {
//         Err((StatusCode::UNAUTHORIZED,"Unauthorized".to_string()))
//     }
// }
//
//
// pub async fn start(chan: Chan) {
//     let cors_layer = tower_http::cors::CorsLayer::permissive();
//     let mon = mongo().await;
//     let mut red = redis_pool().await;
//
//     let user_db = UserDbClient::new(MONGO_URL,MONGO_DB_NAME,USER_COL).await;
//
//     let user_coll = mon.collection::<Trade>("traders");
//     let app = Router::new()
//         .route("/buy", post(buy))
//         .with_state(ServerState {
//             chan,
//             trade_db: user_coll,
//             user_db,
//             red
//         })
//         .layer(cors_layer);
//     // run our app with hyper, listening globally on port 3000
//     let listener = tokio::net::TcpListener::bind("0.0.0.0:4000").await.unwrap();
//     axum::serve(listener, app).await.unwrap();
// }
//
// async fn buy(
//     State(mut state): State<ServerState>,
//     Json(payload): Json<BuyPrompt>,
// ) -> Result<(),HttpErrorResponse> {
//     let kp = state.user_db.get_by_private_key(payload.kp.as_str()).await.ok_or(
//         (StatusCode::NOT_FOUND,"Not Found".to_string())
//     ).map(|x|Keypair::from_base58_string(x.private_key.as_str()))?;
//     let trade_id = Trade::db_id_inc_red(&mut state.red).await;
//
//
//
//     let mut trade = Trade::login(payload.kp.as_str(), state.chan.clone()).await
//         .map_err(|e|(StatusCode::NOT_FOUND,"Not Found".to_string()))?;
//     trade = trade.buy_now(state.chan.clone()).await
//         .map_err(|e|(StatusCode::NOT_FOUND,"Not Found".to_string()))?;
//     Ok(())
// }
