# Getting Started

This guide walks you through installing, building, and running markdownfs for the first time.

## Prerequisites

- **Rust toolchain** (1.85+) — install from [rustup.rs](https://rustup.rs)
- macOS, Linux, or WSL on Windows

Verify your installation:

```bash
rustc --version
cargo --version
```

## Building from Source

Clone the repository and build all three binaries in release mode:

```bash
git clone <repo-url> markdownfs
cd markdownfs
cargo build --release
```

This produces three binaries in `target/release/`:

| Binary | Purpose |
|---|---|
| `markdownfs` | Interactive CLI/REPL |
| `markdownfs-server` | HTTP/REST API server |
| `markdownfs-mcp` | MCP server for AI agents |
| `mdfs` | Remote-first CLI over the HTTP/gateway surface |

## First Run — CLI

Start the interactive shell:

```bash
cargo run --release --bin markdownfs
```

On first launch, there are no users besides `root`. markdownfs prompts you to create an admin account, automatically creates your home directory, and drops you right in:

```
markdownfs v0.2.0 — Markdown Virtual File System

Welcome! Let's set up your account.
Admin username: alice

Created admin 'alice' (uid=1, groups=[alice, wheel])
Home directory: /home/alice

Type 'help' for available commands, 'exit' to quit.

alice@markdownfs:~ $
```

You're now in your home directory (`~` is `/home/alice`), ready to start working immediately — no setup required.

### Try It Out

```
alice@markdownfs:~ $ whoami
alice

alice@markdownfs:~ $ pwd
/home/alice

alice@markdownfs:~ $ touch hello.md
alice@markdownfs:~ $ write hello.md # Welcome to markdownfs
alice@markdownfs:~ $ cat hello.md
# Welcome to markdownfs

alice@markdownfs:~ $ mkdir docs
alice@markdownfs:~ $ touch docs/readme.md
alice@markdownfs:~ $ write docs/readme.md # My Project

alice@markdownfs:~ $ ls -l
drwxr-xr-x alice    alice           1 Apr 13 10:30 docs/
-rw-r--r-- alice    alice          23 Apr 13 10:30 hello.md

alice@markdownfs:~ $ tree
.
├── docs/
│   └── readme.md
└── hello.md

alice@markdownfs:~ $ commit initial setup
[0c5fd42b] initial setup
```

Type `exit` or `quit` to leave. Your data is automatically saved to `.vfs/state.bin` and restored on next launch.

### A Note on the Root Directory

The root directory `/` is owned by `root:root` with mode `0755`, just like a real Unix system. Regular users (including admins) work inside their home directory. If you need to create top-level directories, switch to root:

```
alice@markdownfs:~ $ su root
root@markdownfs:~ $ mkdir /shared
root@markdownfs:~ $ chmod 777 /shared
root@markdownfs:~ $ su alice
```

## First Run — HTTP Server

Start the REST API:

```bash
MARKDOWNFS_LISTEN=127.0.0.1:3000 cargo run --release --bin markdownfs-server
```

The server is now accepting requests:

```bash
# Write a file
curl -X PUT http://localhost:3000/fs/notes/readme.md \
  -H "Authorization: User alice" \
  -d "# My Notes"

# Read it back
curl http://localhost:3000/fs/notes/readme.md

# Check health
curl http://localhost:3000/health
```

See the [HTTP API Guide](http-api-guide.md) for the full endpoint reference.

## First Run — MCP Server

For AI agent integration (Cursor, Claude Desktop, etc.):

```bash
cargo run --release --bin markdownfs-mcp
```

The MCP server communicates over stdio. Add it to your MCP client config — for example, in Cursor's `mcp.json`:

```json
{
  "mcpServers": {
    "markdownfs": {
      "command": "/absolute/path/to/target/release/markdownfs-mcp",
      "env": {
        "MARKDOWNFS_DATA_DIR": "/path/to/your/data"
      }
    }
  }
}
```

See the [MCP Guide](mcp-guide.md) for tool descriptions and usage.

## First Run — Remote CLI

The `mdfs` binary is a thin client over the HTTP API and future hosted gateway.

```bash
cargo run --release --bin mdfs -- --url http://127.0.0.1:3000 health
```

Examples:

```bash
# As a named user
cargo run --release --bin mdfs -- --url http://127.0.0.1:3000 --user alice ls /incidents

# As an agent token
cargo run --release --bin mdfs -- --url http://127.0.0.1:3000 --token "$MARKDOWNFS_TOKEN" grep timeout /runbooks

# Workspace-scoped hosted token
cargo run --release --bin mdfs -- \
  --url http://127.0.0.1:3000 \
  --workspace-id "<workspace-uuid>" \
  --workspace-token "<workspace-secret>" \
  status
```

## Configuration

All configuration is through environment variables. Set them before launching any binary:

| Variable | Default | Description |
|---|---|---|
| `MARKDOWNFS_DATA_DIR` | Current working directory | Where `.vfs/state.bin` is stored |
| `MARKDOWNFS_LISTEN` | `127.0.0.1:3000` | HTTP server bind address |
| `MARKDOWNFS_AUTOSAVE_SECS` | `5` | Auto-save interval (seconds) |
| `MARKDOWNFS_AUTOSAVE_WRITES` | `100` | Auto-save after N write operations |
| `MARKDOWNFS_MAX_FILE_SIZE` | `10485760` (10 MB) | Maximum file size in bytes |
| `RUST_LOG` | `markdownfs=info` | Log verbosity (`debug`, `trace`, etc.) |

Example — custom data directory with verbose logging:

```bash
MARKDOWNFS_DATA_DIR=/var/data/markdownfs \
RUST_LOG=markdownfs=debug \
cargo run --release --bin markdownfs
```

## Data Persistence

markdownfs stores its entire state (filesystem, users, version history) in a single binary file:

```
<MARKDOWNFS_DATA_DIR>/.vfs/state.bin
```

- **Auto-save** runs every 5 seconds (or after 100 writes, whichever comes first)
- **On exit**, the CLI and HTTP server perform a final save
- **Atomic writes** — the file is written to a temp file first, then renamed, so a crash never corrupts your data

To start fresh, simply delete the state file:

```bash
rm -rf .vfs/
```

## Subsequent Logins

After the first run, subsequent CLI launches show a login prompt and automatically navigate to your home directory:

```
markdownfs v0.2.0 — Loaded from disk (1 commits, 7 objects)
Login as: alice
Logged in as 'alice' (uid=1, gid=2)

alice@markdownfs:~ $
```

All files, users, and version history are restored from the previous session.

## What's Next

- [User Management](user-management.md) — create users, groups, set permissions
- [Filesystem Guide](filesystem-guide.md) — files, directories, search, pipes
- [Version Control](version-control.md) — commit, log, revert
- [HTTP API Guide](http-api-guide.md) — REST endpoints with curl examples
- [MCP Guide](mcp-guide.md) — AI agent integration
