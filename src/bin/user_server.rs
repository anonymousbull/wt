use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use wolf_trader::app_user_type::{User, UserWithId};

#[derive(Clone)]
struct ServerState {
    pub db: Pool<AsyncPgConnection>,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    tokio_rustls::rustls::crypto::aws_lc_rs::default_provider().install_default().expect("Failed to install rustls crypto provider");

    // let config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
    //     PathBuf::new()
    //         .join("cert.crt"),
    //     PathBuf::new()
    //         .join("cert.key"),
    // )
    //     .await
    //     .unwrap();
    let cors_layer = tower_http::cors::CorsLayer::permissive();

    let pg = wolf_trader::constant::pg_conn().await;
    let app = Router::new()
        .route("/users", post(create_user).get(get_users))
        .route("/users/{id}", get(get_user).delete(delete_user))
        .with_state(ServerState {
            db: pg
        }).layer(cors_layer);
    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

type HttpErrorResponse = (StatusCode,String);


// Handler to create a new user
async fn create_user(
    State(state): State<ServerState>,
    Json(payload): Json<User>,
) -> Result<Json<UserWithId>, HttpErrorResponse> {
    let mut c = state.db.get().await.unwrap();
    let user = payload.insert(&mut c)
        .await
        .map(|x| Json(x))
        .map_err(internal_error);
    user

}

// Handler to retrieve all users
async fn get_users(State(state): State<ServerState>) -> Result<Json<Vec<UserWithId>>,HttpErrorResponse> {
    let mut c = state.db.get().await.unwrap();
    Ok(Json(UserWithId::get_all(&mut c).await))
}

// Handler to retrieve a specific user by ID
async fn get_user(
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    let mut c = state.db.get().await.unwrap();
    UserWithId::get_by_id(id,&mut c)
        .await
        .map(|x| Json(x))
        .ok_or((StatusCode::NOT_FOUND,"User not found".to_string()))
}

// Handler to delete a user by ID
async fn delete_user(
    State(state): State<ServerState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<Json<UserWithId>,HttpErrorResponse> {
    let mut c = state.db.get().await.unwrap();
   Ok(Json(UserWithId::delete_by_id(id,&mut c)
       .await))
}


/// Utility function for mapping any error into a `500 Internal Server Error`
/// response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: ToString,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}