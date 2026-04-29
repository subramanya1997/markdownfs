# MCP Guide — AI Agent Integration

mdfs includes an MCP (Model Context Protocol) server that lets AI agents — like Cursor, Claude Desktop, or any MCP-compatible client — interact with the filesystem using structured tool calls.

## What is MCP?

MCP is a protocol that lets AI assistants call tools exposed by external servers. Instead of generating shell commands, the AI calls structured functions like `read_file(path: "docs/readme.md")` and gets structured responses back.

## Setup

### 1. Build the MCP Binary

```bash
cargo build --release --bin mdfs-mcp
```

The binary is at `target/release/mdfs-mcp`.

### 2. Configure Your MCP Client

#### Cursor

Add to your project's `.cursor/mcp.json` or global MCP config:

```json
{
  "mcpServers": {
    "mdfs": {
      "command": "/absolute/path/to/target/release/mdfs-mcp",
      "env": {
        "MARKDOWNFS_DATA_DIR": "/path/to/your/data"
      }
    }
  }
}
```

#### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "mdfs": {
      "command": "/absolute/path/to/target/release/mdfs-mcp",
      "env": {
        "MARKDOWNFS_DATA_DIR": "/path/to/your/data"
      }
    }
  }
}
```

### 3. Verify

After restarting your MCP client, the mdfs tools should appear in the tool list. The server communicates over stdio (stdin/stdout).

## Available Tools

### `read_file`

Read the content of a markdown file.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | Path to the file |

**Example call:**
```json
{"path": "docs/readme.md"}
```

**Response:** File content as text.

---

### `write_file`

Write content to a file. Creates the file (and parent directories) if it doesn't exist.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | Path to the file (must end in `.md`) |
| `content` | string | Yes | Content to write |

**Example call:**
```json
{"path": "notes/meeting.md", "content": "# Meeting Notes\n\nDiscussed project roadmap."}
```

---

### `list_directory`

List entries in a directory. Directories are suffixed with `/`.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | No | Directory path (defaults to root) |

**Example call:**
```json
{"path": "docs"}
```

**Response:**
```
api.md
readme.md
specs/
```

---

### `search_files`

Search file contents using a regex pattern.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `pattern` | string | Yes | Regex pattern to search for |
| `path` | string | No | Directory or file to search in |
| `recursive` | boolean | No | Search subdirectories (default: `true`) |

**Example call:**
```json
{"pattern": "TODO", "recursive": true}
```

**Response:** Matching lines with file paths and line numbers.

---

### `find_files`

Find files by glob pattern.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | No | Starting directory |
| `name` | string | No | Glob pattern (e.g., `*.md`, `readme*`) |

**Example call:**
```json
{"path": ".", "name": "*.md"}
```

---

### `create_directory`

Create a directory, including any missing parent directories.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | Directory path to create |

**Example call:**
```json
{"path": "project/docs/specs"}
```

---

### `delete_file`

Delete a file or directory.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `path` | string | Yes | Path to delete |
| `recursive` | boolean | No | Delete directory contents recursively (default: `false`) |

**Example call:**
```json
{"path": "old-notes.md"}
```

---

### `move_file`

Move or rename a file or directory.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `source` | string | Yes | Current path |
| `destination` | string | Yes | New path |

**Example call:**
```json
{"source": "draft.md", "destination": "docs/final.md"}
```

---

### `commit`

Snapshot the current filesystem state.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `message` | string | Yes | Commit message |

**Example call:**
```json
{"message": "add API documentation"}
```

**Response:** Commit hash and confirmation.

---

### `get_history`

View the commit log.

**No parameters.**

**Response:** List of commits with hashes, timestamps, authors, and messages.

---

### `revert`

Revert the filesystem to a previous commit.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `hash` | string | Yes | Commit hash (or prefix) to revert to |

**Example call:**
```json
{"hash": "a1b2c3d4"}
```

---

## Resources

The MCP server also exposes read-only resources:

| URI | Description |
|---|---|
| `mdfs://tree` | Full directory tree (text/plain) |
| `mdfs://files/<path>` | Read a specific file's content |

The legacy `markdownfs://tree` and `markdownfs://files/<path>` resource URIs are still accepted as aliases.

## Important Notes

- **All MCP operations run as root** (uid=0, gid=0). There is no per-user authentication within the MCP protocol — the agent has full access.
- **Files must have `.md` extension.** Attempting to create `notes.txt` will return an error.
- **Write creates parent directories.** Calling `write_file` with path `a/b/c/file.md` automatically creates `a/`, `a/b/`, and `a/b/c/`.
- **Data persists across restarts.** The MCP server auto-saves to `.vfs/state.bin`.

## Example AI Workflow

Here's how an AI agent might use mdfs in a typical session:

```
1. list_directory(path: "/")
   → See what exists

2. create_directory(path: "project/docs")
   → Set up structure

3. write_file(path: "project/docs/design.md", content: "# Design\n\n...")
   → Create documentation

4. search_files(pattern: "TODO", recursive: true)
   → Find all TODOs across the project

5. read_file(path: "project/docs/design.md")
   → Review what was written

6. commit(message: "initial documentation draft")
   → Save a snapshot

7. write_file(path: "project/docs/design.md", content: "# Design v2\n\n...")
   → Make changes

8. get_history()
   → See all commits

9. revert(hash: "a1b2c3d4")
   → Go back to the previous version if needed
```

## Sharing Data Between Access Methods

The CLI, HTTP server, and MCP server all use the same `MARKDOWNFS_DATA_DIR` for persistence. Files created through one method are visible to all others:

```bash
# Write via CLI
alice@markdownfs:/ $ write notes.md # Created from CLI

# Read via HTTP
curl http://localhost:3000/fs/notes.md

# Read via MCP
# Agent calls: read_file(path: "notes.md")
```

**Note:** Only one process can safely write to the same `state.bin` at a time. If you need concurrent access from multiple clients, use the HTTP server as the single point of access.
