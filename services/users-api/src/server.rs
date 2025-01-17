use std::net::SocketAddr;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use constant::{mongo, USER_API_KEY};
use mongodb::Collection;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use axum_server::tls_rustls::RustlsConfig;
use crate::UserWithId;

#[derive(Clone)]
struct ServerState {
    pub db: Collection<UserWithId>,
    pub id: Arc<AtomicI64>
}

type HttpErrorResponse = (StatusCode,String);

pub async fn start(certs:RustlsConfig, port:u16)  {
    let mon = mongo().await;
    let col = mon.collection::<UserWithId>(env!("USER_COL"));
    let id = UserWithId::count(&col).await;
    let app = Router::new()
        .route("/users", post(create_user).get(get_users))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .with_state(ServerState {
            db: col,
            id: Arc::new(AtomicI64::new(id))
        });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    axum_server::bind_rustls(addr, certs)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn load_ssl(env_var: Vec<u8>, a:Vec<u8>) -> RustlsConfig {
    let config = RustlsConfig::from_pem(
        env_var,
        a
    ).await.unwrap();
    config
}


fn is_admin(headers: &HeaderMap) -> Result<(),HttpErrorResponse> {
    let mut auth = false;
    if let Some(auth_header) = headers.get("x-api-key") {
        if let Ok(api_key) = auth_header.to_str() {
            if api_key == USER_API_KEY {
                auth = true;
            }
        }
    }
    if auth == false {
        Err((StatusCode::UNAUTHORIZED,"Unauthorized".to_string()))
    } else {
        Ok(())
    }
}


async fn create_user(
    headers: HeaderMap,
    State(state): State<ServerState>,
    Json(mut payload): Json<UserWithId>,
) -> Result<Json<UserWithId>, HttpErrorResponse> {
    is_admin(&headers)?;


    let c = state.db;
    payload.id = state.id.load(SeqCst)+1;
    let user = payload.insert(&c)
        .await
        .map(|x| Json(x))
        .map_err(internal_error)?;
    state.id.fetch_add(1, SeqCst);
    Ok(user)
}

async fn get_users(
    headers: HeaderMap,
    State(state): State<ServerState>
) -> Result<Json<Vec<UserWithId>>,HttpErrorResponse> {
    is_admin(&headers)?;
    let c = state.db;
    Ok(Json(UserWithId::get_all(&c).await.unwrap()))
}

async fn get_user(
    headers: HeaderMap,
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    is_admin(&headers)?;
    let c = state.db;
    UserWithId::get_by_id(id,&c)
        .await
        .map_err(internal_error)?
        .map(|x| Json(x))
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
}

// Handler to delete a user by ID
async fn delete_user(
    headers: HeaderMap,
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    is_admin(&headers)?;
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
