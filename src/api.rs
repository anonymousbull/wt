use crate::bg_chan::bg_chan;
use crate::chan::Chan;
use crate::constant::*;
use crate::trade::*;
use crate::trade_chan::trade_chan;
use crate::trade_cmd::InternalCommand;
use crate::user::*;
use anyhow::anyhow;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use axum_server::tls_rustls::RustlsConfig;
use mongodb::Collection;
use redis::aio::ConnectionManager;
use serde_json::{json, Value};
use solana_sdk::signature::Keypair;
use std::net::SocketAddr;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use crate::feedback::Feedback;

#[derive(Clone)]
struct ServerState {
    pub chan:Chan,
    pub trade_db: Collection<Trade>,
    pub user_db: Collection<UserWithId>,
    pub feedback_db: Collection<Feedback>,
    pub red:ConnectionManager,
    pub id: Arc<AtomicI64>
}

pub async fn start(port:u16)  {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                    .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let ssl_cert  = include_bytes!("../ssl/fullchain.pem").to_vec();
    let ssl_key  = include_bytes!("../ssl/privkey.pem").to_vec();

    let (bg_send, bg_r) = tokio::sync::mpsc::channel::<InternalCommand>(100);
    let (trade_send, trade_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);
    // let (dsl_s, dsl_r) = tokio::sync::mpsc::channel::<InternalCommand>(500);

    let chan = Chan{
        bg: bg_send,
        trade: trade_send,
    };

    let mon = mongo().await;
    let red = redis_pool().await;
    let col = mon.collection::<UserWithId>("users");
    let id = UserWithId::count(&col).await;

    tokio::spawn(async move {
        bg_chan(bg_r).await
    });

    tokio::spawn({
        let chan = chan.clone();
        async move {
            trade_chan(chan,trade_r).await;
        }
    });


    let app = Router::new()
        .route("/users", post(login).get(get_users))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .route("/buy", post(buy))
        .route("/feedbacks", post(create_feedback).get(get_feedbacks))
        .with_state(ServerState {
            chan,
            id: Arc::new(AtomicI64::new(id)),
            trade_db: mon.collection::<Trade>("trades"),
            user_db: mon.collection::<UserWithId>("users"),
            feedback_db: mon.collection::<Feedback>("feedbacks"),
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

    let kp = UserWithId::get_by_private_key(&state.user_db, payload.kp.as_str()).await.ok_or(
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


pub async fn is_admin(
    auth: Authorization<Basic>,
    state:&Collection<UserWithId>,
) -> Result<(),HttpErrorResponse> {
    let user = UserWithId::get_by_user_name(&state, auth.username())
        .await;
    match user {
        None => Err(internal_error("unauthorised")),
        Some(user) => {
            if user.password == auth.password() {
                Ok(())
            } else {
                Err(internal_error("unauthorised"))
            }
        }
    }
}




type HttpErrorResponse = (StatusCode,String);



async fn login(
    State(state): State<ServerState>,
    Json(mut payload): Json<UserWithId>,
) -> Result<Json<UserWithId>, HttpErrorResponse> {
    let user = UserWithId::get_by_user_name(&state.user_db,&payload.username).await;
    match user {
        None => {
            let kp = Keypair::new();
            payload.private_key = kp.to_base58_string();
            payload.id = state.id.load(SeqCst)+1;
            let user = payload.insert(&state.user_db)
                .await
                .map(|x| Json(x))
                .map_err(internal_error)?;
            state.id.fetch_add(1, SeqCst);
            Ok(user)
        }
        Some(user) => {
            if user.password == payload.password {
                Ok(Json(user))
            } else {
                Err(internal_error(anyhow!("invalid credentials")))
            }
        }
    }
}

async fn get_users(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>
) -> Result<Json<Vec<UserWithId>>,HttpErrorResponse> {
    is_admin(auth,&state.user_db).await?;
    let c = state.user_db;
    Ok(Json(UserWithId::get_all(&c).await.unwrap()))
}

async fn get_user(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    is_admin(auth,&state.user_db).await?;
    let c = state.user_db;
    UserWithId::get_by_id(id,&c)
        .await
        .map_err(internal_error)?
        .map(|x| Json(x))
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
}

// Handler to delete a user by ID
async fn delete_user(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    is_admin(auth,&state.user_db).await?;
    let c = state.user_db;
    UserWithId::delete_by_id(id,&c)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
        .map(|x| Json(x))
}

async fn create_feedback(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>,
    Json(mut payload): Json<Feedback>,
) -> Result<Json<Feedback>, HttpErrorResponse> {
    let user = UserWithId::get_by_user_name(&state.user_db, auth.username())
        .await
        .ok_or(internal_error("unknown user"))?;
    if user.password == auth.password() {
        payload.username = user.username;
        let feedback = payload.insert(&state.feedback_db)
            .await
            .map(|x| Json(x))
            .map_err(internal_error)?;
        Ok(feedback)
    } else {
        Err(internal_error("invalid credentials"))
    }
}

async fn get_feedbacks(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>
) -> Result<Json<Vec<Feedback>>,HttpErrorResponse> {
    is_admin(auth,&state.user_db).await?;
    let c = state.feedback_db;
    Ok(Json(Feedback::get_all(&c).await.unwrap()))
}


/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: ToString,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}


