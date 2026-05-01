pub mod middleware;
pub mod paths;
pub mod perms;
pub mod routes_admin;
pub mod routes_auth;
pub mod routes_fs;
pub mod routes_vcs;
pub mod ui;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::routing::any;
use axum::Router;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::tower::{
    StreamableHttpServerConfig, StreamableHttpService,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::auth::session::Session;
use crate::db::MarkdownDb;
use crate::mcp::McpServer;
use crate::server::middleware::session_from_headers;

#[derive(Clone)]
pub struct ServerState {
    pub db: Arc<MarkdownDb>,
}

pub type AppState = Arc<ServerState>;

tokio::task_local! {
    pub static MCP_SESSION: Session;
}

type McpService = StreamableHttpService<McpServer, LocalSessionManager>;

#[derive(Clone)]
struct McpRouterState {
    app: AppState,
    mcp: Arc<McpService>,
}

pub fn build_router(db: MarkdownDb) -> Router {
    let mcp_db = db.clone();
    let state: AppState = Arc::new(ServerState {
        db: Arc::new(db),
    });

    // Each new MCP session reads MCP_SESSION (set by mcp_handler) to seed
    // the McpServer with the right Session. Default to root if absent.
    let mcp_service: McpService = StreamableHttpService::new(
        move || {
            let session = MCP_SESSION
                .try_with(|s| s.clone())
                .unwrap_or_else(|_| Session::root());
            Ok(McpServer::with_session(mcp_db.clone(), session))
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    let mcp_router_state = McpRouterState {
        app: state.clone(),
        mcp: Arc::new(mcp_service),
    };

    let mcp_router: Router = Router::new()
        .route("/", any(mcp_handler))
        .route("/{*rest}", any(mcp_handler))
        .with_state(mcp_router_state);

    Router::new()
        .merge(ui::routes())
        .merge(routes_auth::routes())
        .merge(routes_admin::routes())
        .merge(routes_fs::routes())
        .merge(routes_vcs::routes())
        .with_state(state)
        .nest("/mcp", mcp_router)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

async fn mcp_handler(
    State(router_state): State<McpRouterState>,
    headers: HeaderMap,
    request: Request,
) -> axum::response::Response {
    let session = match session_from_headers(&router_state.app, &headers).await {
        Ok(s) => s,
        Err(e) => {
            return (
                axum::http::StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response()
        }
    };
    let mcp = router_state.mcp.clone();
    MCP_SESSION
        .scope(session, async move {
            mcp.handle(request).await.map(Body::new).into_response()
        })
        .await
}
