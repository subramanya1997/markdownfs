use markdownfs::auth::session::{DelegateContext, Session};
use markdownfs::config::Config;
use markdownfs::db::MarkdownDb;
use markdownfs::mcp::McpServer;
use rmcp::ServiceExt;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "markdownfs=info".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = Config::from_env();
    tracing::info!(data_dir = %config.data_dir.display(), "starting mdfs MCP server");

    let db = MarkdownDb::open(config).expect("failed to open database");
    let _save_handle = db.spawn_auto_save();

    let session = build_session(&db).await;
    if let Some(d) = session.delegate.as_ref() {
        tracing::info!(
            principal = %session.username,
            on_behalf_of = %d.username,
            "MCP session established with delegation"
        );
    } else {
        tracing::info!(principal = %session.username, "MCP session established");
    }

    let server = McpServer::with_session(db, session);
    let transport = rmcp::transport::io::stdio();
    let service = server
        .serve(transport)
        .await
        .expect("failed to start MCP server");
    service.waiting().await.expect("MCP server error");
}

async fn build_session(db: &MarkdownDb) -> Session {
    let mut session = if let Ok(token) = std::env::var("MARKDOWNFS_API_TOKEN") {
        match db.authenticate_token(&token).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("MARKDOWNFS_API_TOKEN invalid ({e}); falling back to root");
                Session::root()
            }
        }
    } else if let Ok(name) = std::env::var("MARKDOWNFS_AS_USER") {
        match db.login(&name).await {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("MARKDOWNFS_AS_USER='{name}' invalid ({e}); falling back to root");
                Session::root()
            }
        }
    } else {
        Session::root()
    };

    if let Ok(target) = std::env::var("MARKDOWNFS_ON_BEHALF_OF") {
        if let Some(stripped) = target.strip_prefix(':') {
            if let Some(gid) = db.lookup_gid(stripped).await {
                session.delegate = Some(DelegateContext {
                    uid: u32::MAX,
                    gid,
                    groups: vec![gid],
                    username: format!(":{stripped}"),
                });
            }
        } else if let Ok(other) = db.login(&target).await {
            session.delegate = Some(DelegateContext {
                uid: other.uid,
                gid: other.gid,
                groups: other.groups,
                username: other.username,
            });
        } else {
            tracing::warn!("MARKDOWNFS_ON_BEHALF_OF='{target}' did not resolve");
        }
    }

    session
}
