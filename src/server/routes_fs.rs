use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use super::middleware::session_from_headers;
use super::perms::{require_parent_write, require_perm};
use super::AppState;
use crate::auth::perms::Access;
use crate::auth::session::Session;
use crate::error::VfsError;

#[derive(Deserialize, Default)]
pub struct FsQuery {
    pub stat: Option<bool>,
    pub op: Option<String>,
    pub dst: Option<String>,
    pub recursive: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct SearchQuery {
    pub pattern: Option<String>,
    pub path: Option<String>,
    pub name: Option<String>,
    pub recursive: Option<bool>,
}

fn err_json(status: StatusCode, msg: impl Into<String>) -> impl IntoResponse {
    (status, Json(serde_json::json!({"error": msg.into()})))
}

fn vfs_status(err: &VfsError) -> StatusCode {
    match err {
        VfsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
        VfsError::NotFound { .. } => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_REQUEST,
    }
}

fn vfs_err(e: VfsError) -> axum::response::Response {
    err_json(vfs_status(&e), e.to_string()).into_response()
}

async fn auth_or_403(state: &AppState, headers: &HeaderMap) -> Result<Session, axum::response::Response> {
    session_from_headers(state, headers)
        .await
        .map_err(|e| {
            let status = match e {
                VfsError::PermissionDenied { .. } => StatusCode::FORBIDDEN,
                _ => StatusCode::UNAUTHORIZED,
            };
            err_json(status, e.to_string()).into_response()
        })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/fs", get(get_fs_root))
        .route(
            "/fs/{*path}",
            get(get_fs).put(put_fs).delete(delete_fs).post(post_fs),
        )
        .route("/search/grep", get(search_grep))
        .route("/search/find", get(search_find))
        .route("/tree", get(get_tree_root))
        .route("/tree/{*path}", get(get_tree))
}

async fn get_fs_root(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    if let Err(e) = require_perm(&state.db, &session, "/", Access::Read).await {
        return vfs_err(e);
    }

    match state.db.ls(None).await {
        Ok(entries) => {
            let filtered: Vec<_> = entries
                .into_iter()
                .filter(|e| {
                    session.is_effectively_root()
                        || session.has_permission_bits(e.mode, e.uid, e.gid, Access::Read)
                })
                .collect();
            Json(ls_to_json(&filtered, "/")).into_response()
        }
        Err(e) => vfs_err(e),
    }
}

async fn get_fs(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    let exists = match require_perm(&state.db, &session, &path, Access::Read).await {
        Ok(found) => found,
        Err(e) => return vfs_err(e),
    };
    if !exists {
        return err_json(StatusCode::NOT_FOUND, format!("not found: {path}")).into_response();
    }

    if query.stat.unwrap_or(false) {
        return match state.db.stat(&path).await {
            Ok(info) => Json(serde_json::json!({
                "inode_id": info.inode_id,
                "kind": info.kind,
                "size": info.size,
                "mode": format!("0{:o}", info.mode),
                "uid": info.uid,
                "gid": info.gid,
                "created": info.created,
                "modified": info.modified,
            }))
            .into_response(),
            Err(e) => vfs_err(e),
        };
    }

    match state.db.cat(&path).await {
        Ok(content) => (
            StatusCode::OK,
            [("content-type", "text/markdown")],
            Body::from(content),
        )
            .into_response(),
        Err(VfsError::IsDirectory { .. }) => match state.db.ls(Some(&path)).await {
            Ok(entries) => {
                let filtered: Vec<_> = entries
                    .into_iter()
                    .filter(|e| {
                        session.is_effectively_root()
                            || session.has_permission_bits(e.mode, e.uid, e.gid, Access::Read)
                    })
                    .collect();
                Json(ls_to_json(&filtered, &path)).into_response()
            }
            Err(e) => vfs_err(e),
        },
        Err(e) => vfs_err(e),
    }
}

async fn put_fs(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    let is_dir = headers
        .get("x-markdownfs-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "directory")
        .unwrap_or(false);

    let target_exists = state.db.stat(&path).await.is_ok();

    if target_exists {
        if let Err(e) = require_perm(&state.db, &session, &path, Access::Write).await {
            return vfs_err(e);
        }
    } else if let Err(e) = require_parent_write(&state.db, &session, &path).await {
        return vfs_err(e);
    }

    let uid = session.effective_uid();
    let gid = session.effective_gid();

    if is_dir {
        match state.db.mkdir_p(&path, uid, gid).await {
            Ok(()) => {
                Json(serde_json::json!({"created": path, "type": "directory"})).into_response()
            }
            Err(e) => vfs_err(e),
        }
    } else {
        if !target_exists {
            let trimmed = path.trim_end_matches('/');
            if let Some(idx) = trimmed.rfind('/') {
                let parent = &trimmed[..idx];
                if !parent.is_empty() && state.db.stat(parent).await.is_err() {
                    if let Err(e) = state.db.mkdir_p(parent, uid, gid).await {
                        return vfs_err(e);
                    }
                }
            }
            if let Err(e) = state.db.touch(&path, uid, gid).await {
                return vfs_err(e);
            }
        }

        let size = body.len();
        match state.db.write_file(&path, body.to_vec()).await {
            Ok(()) => Json(serde_json::json!({"written": path, "size": size})).into_response(),
            Err(e) => vfs_err(e),
        }
    }
}

async fn delete_fs(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    if let Err(e) = require_parent_write(&state.db, &session, &path).await {
        return vfs_err(e);
    }
    if !session.is_effectively_root() {
        if let Err(e) = require_perm(&state.db, &session, &path, Access::Write).await {
            return vfs_err(e);
        }
    }

    let recursive = query.recursive.unwrap_or(false);
    let result = if recursive {
        state.db.rm_rf(&path).await
    } else {
        state.db.rm(&path).await
    };

    match result {
        Ok(()) => Json(serde_json::json!({"deleted": path})).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn post_fs(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    match query.op.as_deref() {
        Some("copy") => {
            let dst = match &query.dst {
                Some(d) => d.as_str(),
                None => {
                    return err_json(StatusCode::BAD_REQUEST, "missing dst parameter")
                        .into_response()
                }
            };
            if let Err(e) = require_perm(&state.db, &session, &path, Access::Read).await {
                return vfs_err(e);
            }
            if let Err(e) = require_parent_write(&state.db, &session, dst).await {
                return vfs_err(e);
            }
            let uid = session.effective_uid();
            let gid = session.effective_gid();
            match state.db.cp(&path, dst, uid, gid).await {
                Ok(()) => Json(serde_json::json!({"copied": path, "to": dst})).into_response(),
                Err(e) => vfs_err(e),
            }
        }
        Some("move") => {
            let dst = match &query.dst {
                Some(d) => d.as_str(),
                None => {
                    return err_json(StatusCode::BAD_REQUEST, "missing dst parameter")
                        .into_response()
                }
            };
            if let Err(e) = require_parent_write(&state.db, &session, &path).await {
                return vfs_err(e);
            }
            if let Err(e) = require_parent_write(&state.db, &session, dst).await {
                return vfs_err(e);
            }
            match state.db.mv(&path, dst).await {
                Ok(()) => Json(serde_json::json!({"moved": path, "to": dst})).into_response(),
                Err(e) => vfs_err(e),
            }
        }
        Some(op) => {
            err_json(StatusCode::BAD_REQUEST, format!("unknown op: {op}")).into_response()
        }
        None => err_json(StatusCode::BAD_REQUEST, "missing op parameter").into_response(),
    }
}

async fn search_grep(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    let pattern = match &query.pattern {
        Some(p) => p.clone(),
        None => {
            return err_json(StatusCode::BAD_REQUEST, "missing pattern parameter").into_response()
        }
    };

    let path = query.path.as_deref();
    let recursive = query.recursive.unwrap_or(true);

    match state.db.grep(&pattern, path, recursive, Some(&session)).await {
        Ok(results) => {
            let items: Vec<serde_json::Value> = results
                .iter()
                .map(|r| serde_json::json!({"file": r.file, "line_num": r.line_num, "line": r.line}))
                .collect();
            Json(serde_json::json!({"results": items, "count": items.len()})).into_response()
        }
        Err(e) => vfs_err(e),
    }
}

async fn search_find(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };

    let path = query.path.as_deref();
    let name = query.name.as_deref();

    match state.db.find(path, name, Some(&session)).await {
        Ok(results) => {
            Json(serde_json::json!({"results": results, "count": results.len()})).into_response()
        }
        Err(e) => vfs_err(e),
    }
}

async fn get_tree_root(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.tree(None, Some(&session)).await {
        Ok(tree) => (StatusCode::OK, tree).into_response(),
        Err(e) => vfs_err(e),
    }
}

async fn get_tree(
    State(state): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match auth_or_403(&state, &headers).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match state.db.tree(Some(&path), Some(&session)).await {
        Ok(tree) => (StatusCode::OK, tree).into_response(),
        Err(e) => vfs_err(e),
    }
}

fn ls_to_json(entries: &[crate::fs::LsEntry], path: &str) -> serde_json::Value {
    let items: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "is_dir": e.is_dir,
                "is_symlink": e.is_symlink,
                "size": e.size,
                "mode": format!("0{:o}", e.mode),
                "uid": e.uid,
                "gid": e.gid,
                "modified": e.modified,
            })
        })
        .collect();
    serde_json::json!({"entries": items, "path": path})
}
