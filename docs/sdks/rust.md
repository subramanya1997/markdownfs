# Rust SDK

The core of MarkdownFS is itself a Rust crate — you can embed the database directly in your application without going through the HTTP server.

## Install

```toml
[dependencies]
markdownfs = "0.2"
tokio = { version = "1", features = ["full"] }
```

## Quickstart

```rust
use markdownfs::config::Config;
use markdownfs::db::MarkdownDb;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default().with_data_dir("./data");
    let db = MarkdownDb::open(config)?;

    db.write_file("notes/idea.md", b"# my idea".to_vec()).await?;
    let content = db.cat("notes/idea.md").await?;
    println!("{}", String::from_utf8_lossy(&content));

    db.commit("first note", "alice").await?;

    db.save().await?;
    Ok(())
}
```

## When to use the crate vs. the server

- **Crate** — embedding into a single Rust process, lowest latency, no HTTP overhead.
- **Server** — multi-process / multi-language setups, agents, anything that needs the SDKs.

See [crates.io](https://crates.io/crates/markdownfs) for the full API reference.
