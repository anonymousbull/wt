use std::net::SocketAddr;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use mongodb::Collection;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use anyhow::anyhow;
use axum_extra::headers::Authorization;
use axum_extra::headers::authorization::Basic;
use axum_extra::TypedHeader;
use axum_server::tls_rustls::RustlsConfig;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use solana_sdk::signature::Keypair;
use crate::constant::*;
use crate::implement_mongo_crud_struct;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserWithId {
    #[schemars(skip)]
    pub id: i64,
    #[schemars(rename = "wallet_key")]
    pub private_key: String,
    #[serde(skip)]
    pub username: String,
    #[serde(skip)]
    pub password: String,
    #[serde(skip)]
    pub admin:bool
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

impl UserWithId {
    pub async fn get_by_private_key(col:Collection<Self>, private_key: &str) -> Option<UserWithId> {
        let user = col
            .find_one(mongodb::bson::doc! { "private_key": private_key })
            .await.unwrap();
        user
    }
    pub async fn get_by_user_name(col:&Collection<Self>, username: &str) -> Option<UserWithId> {
        let user = col
            .find_one(mongodb::bson::doc! { "username": username })
            .await.unwrap();
        user
    }
}


implement_mongo_crud_struct!(UserWithId);

#[derive(Clone)]
struct ServerState {
    pub db: Collection<UserWithId>,
    pub id: Arc<AtomicI64>
}

type HttpErrorResponse = (StatusCode,String);

pub async fn start(port:u16)  {
    let ssl_cert  = include_bytes!("../ssl/fullchain.pem").to_vec();
    let ssl_key  = include_bytes!("../ssl/privkey.pem").to_vec();
    let mon = mongo().await;
    let col = mon.collection::<UserWithId>("users");
    let id = UserWithId::count(&col).await;
    let app = Router::new()
        .route("/users", post(login).get(get_users))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .with_state(ServerState {
            db: col,
            id: Arc::new(AtomicI64::new(id))
        });
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let ssl = RustlsConfig::from_pem(ssl_cert,ssl_key).await.unwrap();
    axum_server::bind_rustls(addr, ssl)
        .serve(app.into_make_service())
        .await
        .unwrap();
}


async fn login(
    State(state): State<ServerState>,
    Json(mut payload): Json<UserWithId>,
) -> Result<Json<UserWithId>, HttpErrorResponse> {
    let user = UserWithId::get_by_user_name(&state.db,&payload.username).await;
    match user {
        None => {
            let kp = Keypair::new();
            payload.private_key = kp.to_base58_string();
            payload.id = state.id.load(SeqCst)+1;
            let user = payload.insert(&state.db)
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
    is_admin(auth,&state.db).await?;
    let c = state.db;
    Ok(Json(UserWithId::get_all(&c).await.unwrap()))
}

async fn get_user(
    TypedHeader(auth): TypedHeader<Authorization<Basic>>,
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    is_admin(auth,&state.db).await?;
    let c = state.db;
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
    is_admin(auth,&state.db).await?;
    let c = state.db;
    UserWithId::delete_by_id(id,&c)
        .await
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
        .map(|x| Json(x))
}


/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: ToString,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
