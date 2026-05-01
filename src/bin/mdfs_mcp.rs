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

    let server = McpServer::new(db);
    let transport = rmcp::transport::io::stdio();
    let service = server
        .serve(transport)
        .await
        .expect("failed to start MCP server");
    service.waiting().await.expect("MCP server error");
}
