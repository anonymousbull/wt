use crate::constant::*;
use crate::implement_mongo_crud_struct;
use crate::user_api::{is_admin, UserWithId};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use axum_extra::headers::authorization::Basic;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use axum_server::tls_rustls::RustlsConfig;
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feedback {
    pub id: i64,
    pub username: String,
    pub value: String,
}

implement_mongo_crud_struct!(Feedback);

#[derive(Clone)]
struct ServerState {
    pub user_db: Collection<UserWithId>,
    pub feedback_db: Collection<Feedback>,
}

type HttpErrorResponse = (StatusCode,String);

pub async fn start(port:u16)  {
    let ssl_cert  = include_bytes!("../ssl/fullchain.pem").to_vec();
    let ssl_key  = include_bytes!("../ssl/privkey.pem").to_vec();
    let mon = mongo().await;
    let feedback_db = mon.collection::<Feedback>("feedbacks");
    let app = Router::new()
        .route("/feedbacks", post(create).get(get_route))
        .with_state(ServerState {
            feedback_db,
            user_db: mon.collection::<UserWithId>("users"),
        });
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let ssl = RustlsConfig::from_pem(ssl_cert,ssl_key).await.unwrap();
    axum_server::bind_rustls(addr, ssl)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn create(
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

async fn get_route(
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
