use markdownfs::config::Config;
use markdownfs::db::MarkdownDb;
use markdownfs::server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "markdownfs=info,tower_http=info".parse().unwrap()),
        )
        .init();

    let config = Config::from_env();
    let listen_addr = config.listen_addr.clone();

    tracing::info!(data_dir = %config.data_dir.display(), "starting mdfs server");

    let db = MarkdownDb::open(config).expect("failed to open database");

    let save_handle = db.spawn_auto_save();

    let app = server::build_router(db.clone());

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind {listen_addr}: {e}"));

    tracing::info!("listening on {listen_addr}");
    tracing::info!("endpoints:");
    tracing::info!("  GET    /health          — health check");
    tracing::info!("  POST   /auth/login      — login (JSON: {{\"username\": \"...\" }})");
    tracing::info!("  GET    /workspaces      — list hosted workspaces");
    tracing::info!("  POST   /workspaces      — create hosted workspace");
    tracing::info!("  POST   /workspaces/{{id}}/tokens — issue workspace-scoped token");
    tracing::info!("  GET    /fs/{{path}}      — read file or list directory");
    tracing::info!("  PUT    /fs/{{path}}      — write file or create directory");
    tracing::info!("  DELETE /fs/{{path}}      — delete file or directory");
    tracing::info!("  POST   /fs/{{path}}      — copy/move (op=copy|move&dst=...)");
    tracing::info!("  GET    /search/grep     — grep (pattern=...&path=...&recursive=true)");
    tracing::info!("  GET    /search/find     — find (path=...&name=...)");
    tracing::info!("  GET    /tree/{{path}}    — directory tree");
    tracing::info!("  POST   /vcs/commit      — commit (JSON: {{\"message\": \"...\" }})");
    tracing::info!("  GET    /vcs/log         — commit history");
    tracing::info!("  POST   /vcs/revert      — revert (JSON: {{\"hash\": \"...\" }})");
    tracing::info!("  GET    /vcs/status       — VCS status");

    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down...");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("server error");

    save_handle.abort();

    if let Err(e) = db.save().await {
        tracing::error!("failed to save on shutdown: {e}");
    } else {
        tracing::info!("state saved");
    }
}
