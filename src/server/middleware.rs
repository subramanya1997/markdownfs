use axum::http::HeaderMap;

use crate::auth::session::Session;
use crate::db::MarkdownDb;
use crate::error::VfsError;

pub async fn session_from_headers(
    db: &MarkdownDb,
    headers: &HeaderMap,
) -> Result<Session, VfsError> {
    if let Some(auth_header) = headers.get("authorization") {
        let header_str = auth_header.to_str().map_err(|_| VfsError::AuthError {
            message: "invalid authorization header".to_string(),
        })?;

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            return db.authenticate_token(token).await;
        }

        if let Some(username) = header_str.strip_prefix("User ") {
            return db.login(username).await;
        }
    }

    Ok(Session::root())
}
