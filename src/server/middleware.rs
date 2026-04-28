use axum::http::HeaderMap;
use uuid::Uuid;

use crate::auth::session::Session;
use crate::error::VfsError;
use crate::server::AppState;

pub async fn session_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Session, VfsError> {
    if let Some(auth_header) = headers.get("authorization") {
        let header_str = auth_header.to_str().map_err(|_| VfsError::AuthError {
            message: "invalid authorization header".to_string(),
        })?;

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            if let Some(workspace_id) = headers
                .get("x-markdownfs-workspace")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| Uuid::parse_str(value).ok())
            {
                if let Some(valid) = state
                    .workspaces
                    .validate_workspace_token(workspace_id, token)
                    .await?
                {
                    return state.db.authenticate_token(&valid.token.agent_token).await;
                }
            }

            return state.db.authenticate_token(token).await;
        }

        if let Some(username) = header_str.strip_prefix("User ") {
            return state.db.login(username).await;
        }
    }

    Ok(Session::root())
}
