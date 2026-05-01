---
title: MarkdownFS
emoji: 📝
colorFrom: indigo
colorTo: purple
sdk: docker
app_port: 7860
pinned: true
license: mit
short_description: Versioned markdown FS for AI agents — REST + MCP + UI.
tags:
  - agent
  - mcp
  - filesystem
  - rust
  - markdown
  - vcs
---

# MarkdownFS

**A versioned virtual filesystem for markdown — built for AI agents, runnable on a free Hugging Face Space.**

Every commit, write, search, permission check, and MCP call you see in the app is the same Rust binary you can run locally. One database, three transports, zero lock-in.

[![Source](https://img.shields.io/badge/source-github-blue?logo=github)](https://github.com/subramanya1997/markdownfs)
[![Docs](https://img.shields.io/badge/docs-markdownfs.com-indigo)](https://docs.markdownfs.com/)
[![License](https://img.shields.io/badge/license-MIT-green)](https://github.com/subramanya1997/markdownfs/blob/master/LICENSE)

## What it does

- **Markdown-only filesystem** with Unix-style permissions, users, groups, and tokens.
- **Git-style version control** — commit working state, browse log, revert to any snapshot.
- **Search** across content (grep) and names (find), permission-aware.
- **Three transports** sharing one database:
  - **Web UI** at `/` — tree, viewer/editor, commits, search, admin panel.
  - **REST API** under `/fs`, `/vcs`, `/search`, `/admin`, `/auth` — for any HTTP client.
  - **MCP server** at `/mcp` — drop-in agent memory for Claude Desktop, Cursor, or any MCP client.

## Try it now

Click the **App** tab above to open the live UI. On first visit you'll create the admin account and get an API token — save it.

## Use it from your code

```ts
// TypeScript
import { MarkdownFS } from "markdownfs";
const mdfs = new MarkdownFS({
  baseUrl: "https://subramanya97-markdownfs.hf.space",
  token: process.env.MDFS_TOKEN,
});
await mdfs.fs.write("notes/idea.md", "# my idea");
const text = await mdfs.fs.read("notes/idea.md");
await mdfs.vcs.commit("first note");
```

```python
# Python
from markdownfs import MarkdownFS
mdfs = MarkdownFS(base_url="https://subramanya97-markdownfs.hf.space",
                  token=os.environ["MDFS_TOKEN"])
mdfs.fs.write("notes/idea.md", "# my idea")
print(mdfs.fs.read("notes/idea.md"))
```

```bash
# curl
SPACE=https://subramanya97-markdownfs.hf.space
curl -X PUT  "$SPACE/fs/notes/idea.md" -H "Authorization: Bearer $TOK" --data-binary "# my idea"
curl         "$SPACE/fs/notes/idea.md" -H "Authorization: Bearer $TOK"
curl -X POST "$SPACE/vcs/commit"        -H "Authorization: Bearer $TOK" -H 'content-type: application/json' -d '{"message":"first note"}'
```

## Use it from Claude Desktop / Cursor (MCP)

Add this to your client's MCP config:

```json
{
  "mcpServers": {
    "markdownfs": {
      "type": "http",
      "url": "https://subramanya97-markdownfs.hf.space/mcp"
    }
  }
}
```

The agent now has 11 tools: `read_file`, `write_file`, `list_directory`, `search_files`, `find_files`, `create_directory`, `delete_file`, `move_file`, `commit`, `get_history`, `revert`. Every call respects Unix permissions on the Space.

### Agent acting on behalf of a user

Send `X-MarkdownFS-On-Behalf-Of: <username>` alongside your token. The session's permissions become the **intersection** of agent + user (least privilege). Useful when an autonomous agent should only do what one specific user could do.

## Multi-user from the UI

The admin panel (👥 button, top-right when signed in as a wheel member) lets you:

- Create users (with optional `agent` flag)
- Issue / regenerate API tokens
- Add users to groups
- chmod / chown any file or directory
- Delete users

User homes default to `0700` so tenants are isolated by default.

## Persistence

This Space mounts a Hugging Face Persistent Storage volume at `/data`. To enable it, set `MARKDOWNFS_DATA_DIR=/data` under **Settings → Variables and secrets** and restart. Without persistent storage, data is wiped on every container restart.

## Configuration

| Variable                       | Default            | Notes                            |
|--------------------------------|--------------------|----------------------------------|
| `MARKDOWNFS_DATA_DIR`          | `/home/mdfs/data`  | Set to `/data` for persistence   |
| `MARKDOWNFS_LISTEN`            | `0.0.0.0:7860`     | HF routes traffic here           |
| `MARKDOWNFS_AUTOSAVE_SECS`     | `5`                | Auto-save interval               |
| `MARKDOWNFS_MAX_FILE_SIZE`     | `10485760` (10 MB) |                                  |
| `RUST_LOG`                     | `markdownfs=info`  |                                  |

## Run it elsewhere

```bash
# Docker
docker run -p 7860:7860 ghcr.io/subramanya1997/markdownfs:latest

# Build from source (Rust 1.85+)
git clone https://github.com/subramanya1997/markdownfs && cd markdownfs
cargo build --release && ./target/release/mdfs-server
```

Full docs at [docs.markdownfs.com](https://docs.markdownfs.com/).
