    use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use mongodb::Collection;
use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use crate::app_user_type::UserWithId;
use crate::constant::mongo;

#[derive(Clone)]
struct ServerState {
    pub db: Collection<UserWithId>,
    pub id: Arc<AtomicI64>
}

pub async fn start() {
    let router = router().await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

type HttpErrorResponse = (StatusCode,String);

pub async fn router() -> Router {
    let cors_layer = tower_http::cors::CorsLayer::permissive();
    let mon = mongo().await;
    let col = mon.collection::<UserWithId>("traders");
    let id = UserWithId::count(&col).await;
    Router::new()
        .route("/users", post(create_user).get(get_users))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .with_state(ServerState {
            db: col,
            id: Arc::new(AtomicI64::new(id))
        }).layer(cors_layer)
}


// Handler to create a new user
async fn create_user(
    State(state): State<ServerState>,
    Json(mut payload): Json<UserWithId>,
) -> Result<Json<UserWithId>, HttpErrorResponse> {
    println!("whattt {:?}",2);

    let c = state.db;
    payload.id = state.id.load(SeqCst)+1;
    let user = payload.insert(&c)
        .await
        .map(|x| Json(x))
        .map_err(internal_error)?;
    state.id.fetch_add(1, SeqCst);
    Ok(user)
}

// Handler to retrieve all users
async fn get_users(State(state): State<ServerState>) -> Result<Json<Vec<UserWithId>>,HttpErrorResponse> {
    let c = state.db;
    Ok(Json(UserWithId::get_all(&c).await.unwrap()))
}

// Handler to retrieve a specific user by ID
async fn get_user(
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    let c = state.db;
    UserWithId::get_by_id(id,&c)
        .await
        .map_err(internal_error)?
        .map(|x| Json(x))
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
}

// Handler to delete a user by ID
async fn delete_user(
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
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