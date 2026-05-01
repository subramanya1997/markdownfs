use crate::auth::perms::Access;
use crate::auth::session::Session;
use crate::db::MarkdownDb;
use crate::error::VfsError;

/// Resolve the parent directory path for a given path.
/// Returns "" for top-level entries (root parent).
pub fn parent_of(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(idx) => trimmed[..idx].to_string(),
        None => String::new(),
    }
}

/// Look up the inode for `path` and verify the session has the requested access.
/// Returns Ok(true) if the path exists and access is allowed,
/// Ok(false) if the path does not exist (caller decides what to do),
/// Err(PermissionDenied) if it exists but access is denied.
pub async fn require_perm(
    db: &MarkdownDb,
    session: &Session,
    path: &str,
    access: Access,
) -> Result<bool, VfsError> {
    if session.is_effectively_root() {
        return Ok(db.stat(path).await.is_ok());
    }
    match db.stat(path).await {
        Ok(info) => {
            if session.has_permission_bits(info.mode, info.uid, info.gid, access) {
                Ok(true)
            } else {
                Err(VfsError::PermissionDenied {
                    path: path.to_string(),
                })
            }
        }
        Err(_) => Ok(false),
    }
}

/// Verify the session has write access to the parent directory of `path`.
/// Used for create/delete/rename operations.
pub async fn require_parent_write(
    db: &MarkdownDb,
    session: &Session,
    path: &str,
) -> Result<(), VfsError> {
    if session.is_effectively_root() {
        return Ok(());
    }
    let parent = parent_of(path);
    let parent_path = if parent.is_empty() { "/" } else { parent.as_str() };
    let info = db.stat(parent_path).await.map_err(|_| VfsError::NotFound {
        path: parent_path.to_string(),
    })?;
    if session.has_permission_bits(info.mode, info.uid, info.gid, Access::Write) {
        Ok(())
    } else {
        Err(VfsError::PermissionDenied {
            path: parent_path.to_string(),
        })
    }
}
