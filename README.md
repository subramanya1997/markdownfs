# markdownfs

A high-performance, concurrent markdown database built in Rust. Supports Unix-like commands, Git-style versioning with content-addressable storage, disk persistence, multi-user permissioning, HTTP/REST API, and MCP (Model Context Protocol) for AI agents.

Only Markdown (`.md`) files are supported by design.

## Access Methods

markdownfs can be used three ways:

| Method | Binary | Use Case |
|---|---|---|
| **CLI/REPL** | `markdownfs` | Interactive terminal use |
| **HTTP/REST API** | `markdownfs-server` | Web apps, services, any HTTP client |
| **MCP Server** | `markdownfs-mcp` | AI agents (Cursor, Claude, etc.) |

All three share the same concurrent core (`MarkdownDb`) with `tokio::RwLock` for safe multi-reader/single-writer access.

## Quick Start

### CLI

```bash
cargo build --release
cargo run --release --bin markdownfs
```

### HTTP Server

```bash
MARKDOWNFS_LISTEN=127.0.0.1:3000 cargo run --release --bin markdownfs-server
```

### MCP Server

```bash
cargo run --release --bin markdownfs-mcp
```

Add to your MCP client config (e.g., Cursor `mcp.json`):

```json
{
  "mcpServers": {
    "markdownfs": {
      "command": "/path/to/markdownfs-mcp",
      "env": {
        "MARKDOWNFS_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

## HTTP API Reference

All endpoints accept `Authorization: Bearer <token>` or `Authorization: User <username>` headers.

### Filesystem

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/fs/{path}` | Read file (markdown) or list directory (JSON) |
| `PUT` | `/fs/{path}` | Write file or create directory (`X-Markdownfs-Type: directory`) |
| `DELETE` | `/fs/{path}?recursive=true` | Delete file or directory |
| `POST` | `/fs/{path}?op=copy&dst=...` | Copy file |
| `POST` | `/fs/{path}?op=move&dst=...` | Move file |
| `GET` | `/fs/{path}?stat=true` | File metadata (JSON) |

### Search

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/search/grep?pattern=...&path=...&recursive=true` | Search file contents |
| `GET` | `/search/find?path=...&name=...` | Find files by glob |
| `GET` | `/tree/{path}` | Directory tree |

### Version Control

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/vcs/commit` | Commit (`{"message": "..."}`) |
| `GET` | `/vcs/log` | Commit history |
| `POST` | `/vcs/revert` | Revert (`{"hash": "..."}`) |
| `GET` | `/vcs/status` | Status |

### Auth & Health

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/auth/login` | Login (`{"username": "..."}`) |
| `GET` | `/health` | Health check + stats |

### Example

```bash
# Write a file
curl -X PUT http://localhost:3000/fs/docs/readme.md \
  -H "Authorization: User alice" \
  -d "# Hello World"

# Read it back
curl http://localhost:3000/fs/docs/readme.md

# Commit
curl -X POST http://localhost:3000/vcs/commit \
  -H "Content-Type: application/json" \
  -d '{"message": "initial commit"}'

# Search
curl "http://localhost:3000/search/grep?pattern=Hello&recursive=true"
```

## MCP Tools

The MCP server exposes these tools for AI agents:

| Tool | Description |
|---|---|
| `read_file` | Read a markdown file by path |
| `write_file` | Write content to a file (creates if needed) |
| `list_directory` | List files in a directory |
| `search_files` | Grep for a pattern across files |
| `find_files` | Find files by glob pattern |
| `create_directory` | Create a directory (with parents) |
| `delete_file` | Delete a file or directory |
| `move_file` | Move or rename |
| `commit` | Commit current state |
| `get_history` | Show commit log |
| `revert` | Revert to a commit |

## CLI Commands

### File Operations

| Command | Description |
|---|---|
| `ls [-l] [path]` | List directory contents (filtered by permission) |
| `cd [path]` | Change directory |
| `pwd` | Print working directory |
| `mkdir [-p] <path>` | Create directory |
| `touch <file.md>` | Create empty markdown file |
| `cat <file>` | Display file contents |
| `write <file> [content]` | Write content to file |
| `edit <file.md>` | Multi-line editor with auto-commit |
| `rm [-r] <path>` | Remove file or directory |
| `mv <src> <dst>` | Move or rename |
| `cp <src> <dst>` | Copy file |
| `stat <path>` | Show metadata |
| `tree [path]` | Directory tree |
| `find [path] [-name pattern]` | Find files |
| `grep [-r] <pattern> [path]` | Search contents |
| `chmod <mode> <path>` | Change permissions |
| `chown <user:group> <path>` | Change ownership |
| `ln -s <target> <link>` | Symbolic link |

### User Management

| Command | Description |
|---|---|
| `adduser <name>` | Create user (admin only) |
| `addagent <name>` | Create agent with API token |
| `deluser <name>` | Delete user (root only) |
| `addgroup <name>` | Create group |
| `delgroup <name>` | Delete group |
| `usermod -aG <group> <user>` | Add user to group |
| `groups [user]` | Show group memberships |
| `whoami` | Show current user |
| `su <user>` | Switch user |

### Version Control

| Command | Description |
|---|---|
| `commit <message>` | Snapshot state |
| `log` | Show history |
| `revert <hash>` | Revert to commit |
| `status` | Summary |

## Configuration

Environment variables:

| Variable | Default | Description |
|---|---|---|
| `MARKDOWNFS_DATA_DIR` | Current directory | Data storage directory |
| `MARKDOWNFS_LISTEN` | `127.0.0.1:3000` | HTTP server listen address |
| `MARKDOWNFS_AUTOSAVE_SECS` | `5` | Auto-save interval (seconds) |
| `MARKDOWNFS_AUTOSAVE_WRITES` | `100` | Auto-save after N writes |
| `MARKDOWNFS_MAX_FILE_SIZE` | `10485760` (10MB) | Maximum file size |
| `MARKDOWNFS_MAX_INODES` | `1000000` | Maximum number of inodes |
| `MARKDOWNFS_MAX_DEPTH` | `256` | Maximum directory depth |
| `RUST_LOG` | `markdownfs=info` | Log level (tracing) |

## Architecture

```
src/
  db.rs            Concurrent MarkdownDb (Arc<RwLock<DbInner>>)
  config.rs        Configuration from env vars
  server/          HTTP/REST API (axum)
    mod.rs           Router setup
    routes_fs.rs     Filesystem endpoints
    routes_vcs.rs    VCS endpoints
    routes_auth.rs   Auth + health endpoints
    middleware.rs    Auth extraction
  bin/
    markdownfs_server.rs  HTTP server binary
    markdownfs_mcp.rs     MCP server binary
  auth/            Multi-user identity & permissions
    mod.rs           User, Group types
    registry.rs      UserRegistry CRUD
    perms.rs         Permission checks
    session.rs       Session context
  cmd/             Command dispatch & pipes
  fs/              Virtual filesystem core
    mod.rs           VirtualFs — inodes, path resolution, all ops
    inode.rs         Inode types
  store/           Content-addressable object store
  vcs/             Version control (commit, revert, log)
  persist.rs       Disk persistence (atomic bincode)
  error.rs         VfsError enum
  main.rs          CLI/REPL binary
```

## Performance

~130x average speedup over native filesystem (in-memory, zero-copy reads, content-addressable dedup).

```bash
cargo test --release --test perf -- --nocapture
cargo test --release --test perf_comparison -- --nocapture
```

## Testing

233 tests across 5 suites:

```bash
cargo test                        # all tests
cargo test --test integration     # 106 integration tests
cargo test --test permissions     # 72 permission tests
cargo test --test perf --release  # 37 perf benchmarks
```

## License

MIT
