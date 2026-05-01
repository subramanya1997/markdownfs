pub mod middleware;
pub mod perms;
pub mod routes_admin;
pub mod routes_auth;
pub mod routes_fs;
pub mod routes_vcs;
pub mod ui;

use axum::Router;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::db::MarkdownDb;
use crate::mcp::McpServer;

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<MarkdownDb>,
}

pub type AppState = Arc<ServerState>;

pub fn build_router(db: MarkdownDb) -> Router {
    let mcp_db = db.clone();
    let state: AppState = Arc::new(ServerState {
        db: Arc::new(db),
    });

    let mcp_service = StreamableHttpService::new(
        move || Ok(McpServer::new(mcp_db.clone())),
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    Router::new()
        .merge(ui::routes())
        .merge(routes_auth::routes())
        .merge(routes_admin::routes())
        .merge(routes_fs::routes())
        .merge(routes_vcs::routes())
        .with_state(state)
        .nest_service("/mcp", mcp_service)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
