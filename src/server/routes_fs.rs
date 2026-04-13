use axum::body::{Body, Bytes};
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;

use super::middleware::session_from_headers;
use super::AppState;

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

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/fs", get(get_fs_root))
        .route("/fs/{*path}", get(get_fs).put(put_fs).delete(delete_fs).post(post_fs))
        .route("/search/grep", get(search_grep))
        .route("/search/find", get(search_find))
        .route("/tree", get(get_tree_root))
        .route("/tree/{*path}", get(get_tree))
}

async fn get_fs_root(
    State(db): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    match db.ls(None).await {
        Ok(entries) => Json(ls_to_json(&entries, "/")).into_response(),
        Err(e) => err_json(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_fs(
    State(db): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    if query.stat.unwrap_or(false) {
        return match db.stat(&path).await {
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
            Err(e) => err_json(StatusCode::NOT_FOUND, e.to_string()).into_response(),
        };
    }

    match db.cat(&path).await {
        Ok(content) => {
            (StatusCode::OK, [("content-type", "text/markdown")], Body::from(content))
                .into_response()
        }
        Err(crate::error::VfsError::IsDirectory { .. }) => {
            match db.ls(Some(&path)).await {
                Ok(entries) => Json(ls_to_json(&entries, &path)).into_response(),
                Err(e) => {
                    err_json(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
                }
            }
        }
        Err(e) => err_json(StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

async fn put_fs(
    State(db): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    let is_dir = headers
        .get("x-markdownfs-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "directory")
        .unwrap_or(false);

    if is_dir {
        match db.mkdir_p(&path, session.uid, session.gid).await {
            Ok(()) => {
                Json(serde_json::json!({"created": path, "type": "directory"})).into_response()
            }
            Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    } else {
        if db.stat(&path).await.is_err() {
            if let Err(e) = db.touch(&path, session.uid, session.gid).await {
                return err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response();
            }
        }

        let size = body.len();
        match db.write_file(&path, body.to_vec()).await {
            Ok(()) => Json(serde_json::json!({"written": path, "size": size})).into_response(),
            Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    }
}

async fn delete_fs(
    State(db): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    let recursive = query.recursive.unwrap_or(false);
    let result = if recursive {
        db.rm_rf(&path).await
    } else {
        db.rm(&path).await
    };

    match result {
        Ok(()) => Json(serde_json::json!({"deleted": path})).into_response(),
        Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn post_fs(
    State(db): State<AppState>,
    Path(path): Path<String>,
    Query(query): Query<FsQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    match query.op.as_deref() {
        Some("copy") => {
            let dst = match &query.dst {
                Some(d) => d.as_str(),
                None => return err_json(StatusCode::BAD_REQUEST, "missing dst parameter").into_response(),
            };
            match db.cp(&path, dst, session.uid, session.gid).await {
                Ok(()) => Json(serde_json::json!({"copied": path, "to": dst})).into_response(),
                Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
        Some("move") => {
            let dst = match &query.dst {
                Some(d) => d.as_str(),
                None => return err_json(StatusCode::BAD_REQUEST, "missing dst parameter").into_response(),
            };
            match db.mv(&path, dst).await {
                Ok(()) => Json(serde_json::json!({"moved": path, "to": dst})).into_response(),
                Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            }
        }
        Some(op) => err_json(StatusCode::BAD_REQUEST, format!("unknown op: {op}")).into_response(),
        None => err_json(StatusCode::BAD_REQUEST, "missing op parameter").into_response(),
    }
}

async fn search_grep(
    State(db): State<AppState>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    let pattern = match &query.pattern {
        Some(p) => p.clone(),
        None => return err_json(StatusCode::BAD_REQUEST, "missing pattern parameter").into_response(),
    };

    let path = query.path.as_deref();
    let recursive = query.recursive.unwrap_or(true);

    match db.grep(&pattern, path, recursive, Some(&session)).await {
        Ok(results) => {
            let items: Vec<serde_json::Value> = results
                .iter()
                .map(|r| serde_json::json!({"file": r.file, "line_num": r.line_num, "line": r.line}))
                .collect();
            Json(serde_json::json!({"results": items, "count": items.len()})).into_response()
        }
        Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn search_find(
    State(db): State<AppState>,
    Query(query): Query<SearchQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };

    let path = query.path.as_deref();
    let name = query.name.as_deref();

    match db.find(path, name, Some(&session)).await {
        Ok(results) => {
            Json(serde_json::json!({"results": results, "count": results.len()})).into_response()
        }
        Err(e) => err_json(StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn get_tree_root(
    State(db): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };
    match db.tree(None, Some(&session)).await {
        Ok(tree) => (StatusCode::OK, tree).into_response(),
        Err(e) => err_json(StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

async fn get_tree(
    State(db): State<AppState>,
    Path(path): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let session = match session_from_headers(&db, &headers).await {
        Ok(s) => s,
        Err(e) => return err_json(StatusCode::UNAUTHORIZED, e.to_string()).into_response(),
    };
    match db.tree(Some(&path), Some(&session)).await {
        Ok(tree) => (StatusCode::OK, tree).into_response(),
        Err(e) => err_json(StatusCode::NOT_FOUND, e.to_string()).into_response(),
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
