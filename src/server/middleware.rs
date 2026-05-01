use axum::http::HeaderMap;

use crate::auth::session::{DelegateContext, Session};
use crate::error::VfsError;
use crate::server::AppState;

pub const DELEGATE_HEADER: &str = "x-markdownfs-on-behalf-of";

pub async fn session_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Session, VfsError> {
    let mut session = principal_from_headers(state, headers).await?;
    apply_delegation(state, headers, &mut session).await?;
    Ok(session)
}

async fn principal_from_headers(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Session, VfsError> {
    if let Some(auth_header) = headers.get("authorization") {
        let header_str = auth_header.to_str().map_err(|_| VfsError::AuthError {
            message: "invalid authorization header".to_string(),
        })?;

        if let Some(token) = header_str.strip_prefix("Bearer ") {
            return state.db.authenticate_token(token).await;
        }

        if let Some(username) = header_str.strip_prefix("User ") {
            return state.db.login(username).await;
        }
    }

    Ok(Session::root())
}

/// Resolve the X-MarkdownFS-On-Behalf-Of header (if present) into a
/// `DelegateContext` and attach it to the session.
///
/// Header value forms:
///   `Bearer <user-token>`  → authenticate by token
///   `User <username>`      → look up by name (gated)
///   `<username>`           → bare username (gated)
///   `:<groupname>`         → group-only delegation (gated)
///
/// Username/group delegation is only allowed when the principal is root,
/// a wheel member, or marked as an agent.
async fn apply_delegation(
    state: &AppState,
    headers: &HeaderMap,
    session: &mut Session,
) -> Result<(), VfsError> {
    let raw = match headers.get(DELEGATE_HEADER) {
        Some(v) => v.to_str().map_err(|_| VfsError::AuthError {
            message: format!("invalid {DELEGATE_HEADER} header"),
        })?,
        None => return Ok(()),
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    if let Some(token) = trimmed.strip_prefix("Bearer ") {
        let delegate_session = state.db.authenticate_token(token).await?;
        session.delegate = Some(DelegateContext {
            uid: delegate_session.uid,
            gid: delegate_session.gid,
            groups: delegate_session.groups,
            username: delegate_session.username,
        });
        return Ok(());
    }

    let target = trimmed.strip_prefix("User ").unwrap_or(trimmed);
    require_delegate_authority(state, session).await?;

    if let Some(group_name) = target.strip_prefix(':') {
        let gid = state
            .db
            .lookup_gid(group_name)
            .await
            .ok_or_else(|| VfsError::AuthError {
                message: format!("no such group: {group_name}"),
            })?;
        session.delegate = Some(DelegateContext {
            uid: u32::MAX,
            gid,
            groups: vec![gid],
            username: format!(":{group_name}"),
        });
    } else {
        let user_session = state.db.login(target).await?;
        session.delegate = Some(DelegateContext {
            uid: user_session.uid,
            gid: user_session.gid,
            groups: user_session.groups,
            username: user_session.username,
        });
    }
    Ok(())
}

async fn require_delegate_authority(
    state: &AppState,
    session: &Session,
) -> Result<(), VfsError> {
    if session.is_effectively_root() {
        return Ok(());
    }
    let (is_wheel, is_agent) = state.db.principal_flags(session.uid).await;
    if is_wheel || is_agent {
        Ok(())
    } else {
        Err(VfsError::PermissionDenied {
            path: "delegate: only agents or admins can delegate by username".to_string(),
        })
    }
}
