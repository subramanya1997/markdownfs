use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use super::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub groups: Vec<u32>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/health", axum::routing::get(health))
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.db.login(&req.username).await {
        Ok(session) => (
            StatusCode::OK,
            Json(LoginResponse {
                username: session.username.clone(),
                uid: session.uid,
                gid: session.gid,
                groups: session.groups.clone(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let commits = state.db.commit_count().await;
    let inodes = state.db.inode_count().await;
    let objects = state.db.object_count().await;

    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "commits": commits,
        "inodes": inodes,
        "objects": objects,
    }))
}
