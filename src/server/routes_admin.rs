use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;

use super::middleware::session_from_headers;
use super::paths::resolve_user_path;
use super::AppState;
use crate::auth::session::Session;
use crate::error::VfsError;

fn err_json(status: StatusCode, msg: impl Into<String>) -> impl IntoResponse {
    (status, Json(serde_json::json!({"error": msg.into()})))
}

fn vfs_status(err: &VfsError) -> StatusCode {
    match err {
        VfsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
        VfsError::NotFound { .. } => StatusCode::NOT_FOUND,
        VfsError::AuthError { .. } => StatusCode::BAD_REQUEST,
        _ => StatusCode::BAD_REQUEST,
    }
}

fn vfs_err(e: VfsError) -> axum::response::Response {
    err_json(vfs_status(&e), e.to_string()).into_response()
}

async fn auth_or_401(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Session, axum::response::Response> {
    session_from_headers(state, headers).await.map_err(|e| {
        let status = match e {
            VfsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
            _ => StatusCode::UNAUTHORIZED,
        };
        err_json(status, e.to_string()).into_response()
    })
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub name: String,
    #[serde(default)]
    pub is_agent: bool,
}

#[derive(Deserialize)]
pub struct CreateGroupRequest {
    pub name: String,
}

#[derive(Deserialize)]
pub struct ChmodRequest {
    pub mode: String,
}

#[derive(Deserialize)]
pub struct ChownRequest {
    pub owner: String,
    #[serde(default)]
    pub group: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/admin/users", get(list_users).post(create_user))
        .route("/admin/users/{name}", delete(delete_user))
        .route("/admin/users/{name}/tokens", post(issue_token))
        .route(
            "/admin/users/{name}/groups/{group}",
            post(usermod_add).delete(usermod_remove),
        )
        .route("/admin/groups", get(list_groups).post(create_group))
        .route("/admin/groups/{name}", delete(delete_group))
        .route("/admin/chmod/{*path}", post(chmod))
        .route("/admin/chown/{*path}", post(chown))
}

async fn list_users(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_list_users(&s).await {
        Ok(users) => Json(serde_json::json!({ "users": users })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_add_user(&s, &req.name, req.is_agent).await {
        Ok((uid, token)) => Json(serde_json::json!({
            "uid": uid,
            "name": req.name,
            "token": token,
        }))
        .into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn delete_user(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_del_user(&s, &name).await {
        Ok(()) => Json(serde_json::json!({ "deleted": name })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn issue_token(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_issue_token(&s, &name).await {
        Ok(token) => Json(serde_json::json!({ "name": name, "token": token })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn usermod_add(
    State(state): State<AppState>,
    Path((name, group)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_usermod_add(&s, &name, &group).await {
        Ok(()) => Json(serde_json::json!({ "user": name, "group": group, "added": true })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn usermod_remove(
    State(state): State<AppState>,
    Path((name, group)): Path<(String, String)>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_usermod_remove(&s, &name, &group).await {
        Ok(()) => Json(serde_json::json!({ "user": name, "group": group, "removed": true })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn list_groups(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_list_groups(&s).await {
        Ok(groups) => Json(serde_json::json!({ "groups": groups })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn create_group(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateGroupRequest>,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_add_group(&s, &req.name).await {
        Ok(gid) => Json(serde_json::json!({ "gid": gid, "name": req.name })).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn delete_group(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.admin_del_group(&s, &name).await {
        Ok(()) => Json(serde_json::json!({ "deleted": name })).into_response(),
        Err(e) => vfs_err(e),
    }
}

fn parse_mode(s: &str) -> Option<u16> {
    let trimmed = s.trim_start_matches('0');
    if trimmed.is_empty() {
        return Some(0);
    }
    u16::from_str_radix(trimmed, 8).ok()
}

async fn chmod(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ChmodRequest>,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    let mode = match parse_mode(&req.mode) {
        Some(m) => m,
        None => return err_json(StatusCode::BAD_REQUEST, format!("invalid mode: {}", req.mode)).into_response(),
    };
    let path = resolve_user_path(&s, &path);
    match state.db.admin_chmod(&s, &path, mode).await {
        Ok(()) => Json(serde_json::json!({"path": path, "mode": format!("0{:o}", mode)})).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn chown(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ChownRequest>,
) -> impl IntoResponse {
    let s = match auth_or_401(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    let path = resolve_user_path(&s, &path);
    match state
        .db
        .admin_chown(&s, &path, &req.owner, req.group.as_deref())
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "path": path,
            "owner": req.owner,
            "group": req.group,
        }))
        .into_response(),
        Err(e) => vfs_err(e),
    }
}
