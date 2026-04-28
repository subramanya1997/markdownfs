use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use uuid::Uuid;

use super::middleware::session_from_headers;
use super::AppState;

#[derive(Deserialize)]
pub struct CommitRequest {
    pub message: String,
}

#[derive(Deserialize)]
pub struct RevertRequest {
    pub hash: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/vcs/commit", post(vcs_commit))
        .route("/vcs/log", get(vcs_log))
        .route("/vcs/revert", post(vcs_revert))
        .route("/vcs/status", get(vcs_status))
}

async fn vcs_commit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CommitRequest>,
) -> impl IntoResponse {
    let session = match session_from_headers(&state, &headers).await {
        Ok(s) => s,
        Err(e) => return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };

    match state.db.commit(&req.message, &session.username).await {
        Ok(hash) => {
            if let Some(workspace_id) = headers
                .get("x-markdownfs-workspace")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| Uuid::parse_str(value).ok())
            {
                let _ = state
                    .workspaces
                    .update_head_commit(workspace_id, Some(hash.clone()))
                    .await;
            }
            Json(serde_json::json!({
            "hash": hash,
            "message": req.message,
            "author": session.username,
        }))
        .into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn vcs_log(State(state): State<AppState>) -> impl IntoResponse {
    let commits = state.db.vcs_log().await;
    let items: Vec<serde_json::Value> = commits
        .iter()
        .map(|c| {
            serde_json::json!({
                "hash": c.id.short_hex(),
                "message": c.message,
                "author": c.author,
                "timestamp": c.timestamp,
            })
        })
        .collect();
    Json(serde_json::json!({"commits": items}))
}

async fn vcs_revert(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RevertRequest>,
) -> impl IntoResponse {
    match state.db.revert(&req.hash).await {
        Ok(()) => {
            if let Some(workspace_id) = headers
                .get("x-markdownfs-workspace")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| Uuid::parse_str(value).ok())
            {
                let _ = state
                    .workspaces
                    .update_head_commit(workspace_id, Some(req.hash.clone()))
                    .await;
            }
            Json(serde_json::json!({"reverted_to": req.hash})).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn vcs_status(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.vcs_status().await {
        Ok(status) => (StatusCode::OK, status).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}
