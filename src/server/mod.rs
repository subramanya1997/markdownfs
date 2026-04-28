pub mod middleware;
pub mod routes_auth;
pub mod routes_fs;
pub mod routes_workspace;
pub mod routes_vcs;

use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::db::MarkdownDb;
use crate::workspace::{InMemoryWorkspaceMetadataStore, SharedWorkspaceMetadataStore};

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<MarkdownDb>,
    pub workspaces: SharedWorkspaceMetadataStore,
}

pub type AppState = Arc<ServerState>;

pub fn build_router(db: MarkdownDb) -> Router {
    let state: AppState = Arc::new(ServerState {
        db: Arc::new(db),
        workspaces: Arc::new(InMemoryWorkspaceMetadataStore::new()),
    });

    Router::new()
        .merge(routes_auth::routes())
        .merge(routes_fs::routes())
        .merge(routes_workspace::routes())
        .merge(routes_vcs::routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
