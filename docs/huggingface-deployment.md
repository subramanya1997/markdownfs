# Hugging Face Space deployment

This guide deploys `mdfs-server` to a Hugging Face Space so any client —
Claude, a script, an agent framework — can use MarkdownFS over HTTPS.

## 1. Build context

The Space repo expects `Dockerfile` and `README.md` at its root, plus the
Rust source. Two layouts work:

- **Subtree push (recommended).** Keep the `huggingface/` directory in this
  repo. At deploy time, copy it to the Space repo root alongside
  `Cargo.toml`, `Cargo.lock`, and `src/`.
- **Standalone Space repo.** Mirror the source into a dedicated repo whose
  root contains the `Dockerfile` and `README.md` from `huggingface/`.

## 2. Create the Space

1. Sign in at https://huggingface.co.
2. New → Space → SDK = **Docker** → name it (e.g. `markdownfs`).
3. Clone it locally:
   ```bash
   git clone https://huggingface.co/spaces/<user>/markdownfs hf-space
   ```
4. Populate the Space repo:
   ```bash
   cp huggingface/Dockerfile      hf-space/Dockerfile
   cp huggingface/.dockerignore   hf-space/.dockerignore
   cp huggingface/README.md       hf-space/README.md
   cp -R Cargo.toml Cargo.lock src tests examples hf-space/
   ```
5. Push:
   ```bash
   cd hf-space
   git add .
   git commit -m "Deploy MarkdownFS"
   git push
   ```

The first build takes ~5–10 minutes (Rust compile). Subsequent pushes are
cached.

## 3. Verify

```bash
SPACE=https://<user>-markdownfs.hf.space
curl "$SPACE/health"
```

## 4. Use it from Claude

### Claude Code (this CLI)

Claude Code can call the REST API directly via Bash:

```bash
SPACE=https://<user>-markdownfs.hf.space
curl -X PUT "$SPACE/fs/notes/idea.md" --data-binary "# my idea"
curl "$SPACE/fs/notes/idea.md"
```

You can also save the base URL into project memory so future sessions know
where the workspace lives.

### Claude Desktop (MCP)

Claude Desktop speaks **stdio MCP**, while the Space exposes REST. Bridge
the two by running `mdfs-mcp` locally pointed at the remote workspace, or
use a generic stdio→HTTP MCP shim. A minimal local config:

```json
{
  "mcpServers": {
    "markdownfs-remote": {
      "command": "curl-mcp-bridge",
      "args": ["--base-url", "https://<user>-markdownfs.hf.space"]
    }
  }
}
```

(A native HTTP transport for `mdfs-mcp` is on the roadmap; until then, the
local-binary-talking-to-remote-REST pattern works for most agent setups.)

## 5. Use it from anywhere

Any HTTP client works. JavaScript:

```js
const SPACE = "https://<user>-markdownfs.hf.space";
await fetch(`${SPACE}/fs/notes/hello.md`, {
  method: "PUT",
  headers: { "Content-Type": "text/markdown" },
  body: "# hello",
});
```

Python:

```python
import requests
SPACE = "https://<user>-markdownfs.hf.space"
requests.put(f"{SPACE}/fs/notes/hello.md", data="# hello")
print(requests.get(f"{SPACE}/fs/notes/hello.md").text)
```

## 6. Persistence

The default Docker filesystem is ephemeral. For durable state:

- **HF Persistent Storage** (paid add-on): in Space settings, enable
  Persistent Storage; mount path is `/data`. Set
  `MARKDOWNFS_DATA_DIR=/data` under Variables.
- **HF Dataset sync**: write a small startup hook that pulls a dataset
  repo into `MARKDOWNFS_DATA_DIR` on boot and pushes it on `vcs/commit`.

## 7. Auth

The default Space is public and unauthenticated. Before exposing real
data, set up tokens via the `/workspaces/{id}/tokens` endpoint and
require them on writes. See the user management guide.
