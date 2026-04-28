use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub root_path: String,
}

#[derive(Deserialize)]
pub struct IssueTokenRequest {
    pub name: String,
    pub agent_token: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/workspaces", get(list_workspaces).post(create_workspace))
        .route("/workspaces/{id}", get(get_workspace))
        .route("/workspaces/{id}/tokens", post(issue_workspace_token))
}

async fn list_workspaces(State(state): State<AppState>) -> impl IntoResponse {
    match state.workspaces.list_workspaces().await {
        Ok(workspaces) => Json(serde_json::json!({ "workspaces": workspaces })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn create_workspace(
    State(state): State<AppState>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> impl IntoResponse {
    match state
        .workspaces
        .create_workspace(&req.name, &req.root_path)
        .await
    {
        Ok(workspace) => (StatusCode::CREATED, Json(workspace)).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn get_workspace(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.workspaces.get_workspace(id).await {
        Ok(Some(workspace)) => Json(workspace).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("unknown workspace: {id}")})),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

async fn issue_workspace_token(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<IssueTokenRequest>,
) -> impl IntoResponse {
    match state
        .workspaces
        .issue_workspace_token(id, &req.name, &req.agent_token)
        .await
    {
        Ok(issued) => Json(serde_json::json!({
            "workspace_id": id,
            "token_id": issued.token.id,
            "name": issued.token.name,
            "workspace_token": issued.raw_secret,
            "agent_token": issued.token.agent_token,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}
