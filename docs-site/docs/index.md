# MarkdownFS

A high-performance, versioned virtual filesystem for markdown — built for AI agents and the developers who orchestrate them.

## Why

Agents need durable, inspectable, searchable workspaces. Plain disk doesn't version. Git is too heavy for ephemeral writes. Object stores aren't filesystems. **MarkdownFS** gives agents a fast in-memory filesystem with built-in commits, permissions, and a REST API — self-hostable on a free Hugging Face Space.

## Three ways to use it

<div class="grid cards" markdown>

- :material-web: **REST API**
  Hit it from any language. Boots in seconds.
  [→ HTTP API](api/http.md)

- :material-language-typescript: **TypeScript SDK**
  `bun add markdownfs` and start writing files.
  [→ TypeScript](sdks/typescript.md)

- :material-language-python: **Python SDK**
  `uv add markdownfs`, sync or async.
  [→ Python](sdks/python.md)

- :material-robot: **MCP server**
  Drop-in agent memory for Claude, Cursor, and more.
  [→ MCP](api/mcp.md)

</div>

## 60-second quickstart

```bash
docker run -p 7860:7860 -e MARKDOWNFS_LISTEN=0.0.0.0:7860 \
  ghcr.io/subramanya1997/markdownfs:latest
```

Then from anywhere:

=== "TypeScript"

    ```ts
    import { MarkdownFS } from "markdownfs";
    const mdfs = new MarkdownFS({ baseUrl: "http://localhost:7860" });
    await mdfs.fs.write("notes/idea.md", "# my idea");
    await mdfs.vcs.commit("first note");
    ```

=== "Python"

    ```python
    from markdownfs import MarkdownFS
    mdfs = MarkdownFS(base_url="http://localhost:7860")
    mdfs.fs.write("notes/idea.md", "# my idea")
    mdfs.vcs.commit("first note")
    ```

=== "curl"

    ```bash
    curl -X PUT http://localhost:7860/fs/notes/idea.md --data-binary "# my idea"
    curl -X POST http://localhost:7860/vcs/commit \
      -H 'content-type: application/json' \
      -d '{"message":"first note"}'
    ```

## What you get

- **In-memory speed** — ~100x faster than native filesystems for typical agent workloads
- **Git-style commits** — `commit`, `log`, `revert`, content-addressable dedup
- **Real users & permissions** — Unix-style `chmod` / `chown`, groups, tokens
- **Three transports** — CLI, REST API, MCP server, all sharing one core
- **Self-hostable** — single Rust binary or one-click [Hugging Face Space](getting-started/huggingface.md)
