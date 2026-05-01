use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use super::middleware::session_from_headers;
use super::AppState;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
}

#[derive(Deserialize)]
pub struct BootstrapRequest {
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
pub struct WhoAmI {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub groups: Vec<u32>,
    pub is_root: bool,
    pub authenticated: bool,
    pub on_behalf_of: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/auth/login", post(login))
        .route("/auth/whoami", get(whoami))
        .route("/auth/bootstrap", post(bootstrap))
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

async fn whoami(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let session = match session_from_headers(&state, &headers).await {
        Ok(s) => s,
        Err(e) => {
            let status = match e {
                crate::error::VfsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
                _ => StatusCode::UNAUTHORIZED,
            };
            return (
                status,
                Json(ErrorResponse {
                    error: e.to_string(),
                }),
            )
                .into_response();
        }
    };

    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let authenticated = auth_header.starts_with("Bearer ") || auth_header.starts_with("User ");

    Json(WhoAmI {
        username: session.username.clone(),
        uid: session.uid,
        gid: session.gid,
        groups: session.groups.clone(),
        is_root: session.is_effectively_root(),
        authenticated,
        on_behalf_of: session.delegate.as_ref().map(|d| d.username.clone()),
    })
    .into_response()
}

/// Create the very first admin account. Only succeeds if no users exist yet.
/// Returns a fresh API token. Used by the first-run UI flow.
async fn bootstrap(
    State(state): State<AppState>,
    Json(req): Json<BootstrapRequest>,
) -> impl IntoResponse {
    if state.db.has_users().await {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "users already exist; bootstrap is only for empty workspaces".to_string(),
            }),
        )
            .into_response();
    }

    if let Err(e) = state.db.create_admin(&req.username).await {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )
            .into_response();
    }

    match state.db.admin_issue_token(&crate::auth::session::Session::root(), &req.username).await {
        Ok(token) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "username": req.username,
                "token": token,
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
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
    let needs_bootstrap = !state.db.has_users().await;

    Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "commits": commits,
        "inodes": inodes,
        "objects": objects,
        "needs_bootstrap": needs_bootstrap,
    }))
}
