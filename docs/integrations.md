# Integrations

How to connect MarkdownFS to Claude Desktop, Claude Code, Cursor, the Anthropic API, and your own scripts.

Throughout this page we'll use a placeholder Space URL. Replace it with yours:

```bash
export MDFS_URL=https://subramanya97-markdownfs.hf.space
```

If you don't have a Space yet, see the [Hugging Face deployment guide](getting-started/huggingface.md). Otherwise [docs.markdownfs.com](https://docs.markdownfs.com/) has the full reference.

---

## Get a token

Every authenticated client needs an API token. Issue one from the **👥 users** panel in the web UI (admin/wheel only):

1. Sign in as admin → click **👥 users**
2. Type a username (tick **agent** for AI clients), click **add**
3. Click the **copy** button to grab the token

Or from any terminal that's already authenticated as admin:

```bash
ADMIN_TOKEN=...   # the token from /auth/bootstrap or admin panel

# Create the user
curl -X POST $MDFS_URL/admin/users \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"claude-agent","is_agent":true}'

# Issue / regenerate a token (returns the raw token once — save it)
curl -X POST $MDFS_URL/admin/users/claude-agent/tokens \
  -H "Authorization: Bearer $ADMIN_TOKEN"
```

```bash
export AGENT_TOKEN=hf_xxx_or_whatever_you_got_back
```

---

## Claude Desktop (MCP over HTTPS)

Open `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows) and add:

```json
{
  "mcpServers": {
    "markdownfs": {
      "type": "http",
      "url": "https://subramanya97-markdownfs.hf.space/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_AGENT_TOKEN_HERE"
      }
    }
  }
}
```

To make the agent act on behalf of a specific user (intersection of perms — see [Concepts: users](concepts/users.md)):

```json
{
  "mcpServers": {
    "markdownfs": {
      "type": "http",
      "url": "https://subramanya97-markdownfs.hf.space/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_AGENT_TOKEN_HERE",
        "X-MarkdownFS-On-Behalf-Of": "alice"
      }
    }
  }
}
```

Restart Claude Desktop. You'll see 11 tools: `read_file`, `write_file`, `list_directory`, `search_files`, `find_files`, `create_directory`, `delete_file`, `move_file`, `commit`, `get_history`, `revert`. All paths in tool calls are resolved under the agent's (or delegate's) home — `notes/idea.md` becomes `/home/claude-agent/notes/idea.md` (or `/home/alice/notes/idea.md` when delegating).

---

## Cursor (MCP over HTTPS)

Cursor uses the same streamable-HTTP MCP transport. Edit `~/.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "markdownfs": {
      "url": "https://subramanya97-markdownfs.hf.space/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_AGENT_TOKEN_HERE"
      }
    }
  }
}
```

Restart Cursor and the tools appear in the agent panel.

---

## Claude Code (curl from Bash)

Claude Code calls REST directly through Bash. Drop these lines into your project's `CLAUDE.md` so future sessions know where to find the workspace:

```markdown
## Markdown workspace

Persistent agent workspace lives at https://subramanya97-markdownfs.hf.space.
Authenticate with `Authorization: Bearer $MDFS_TOKEN`. Relative paths resolve
under your home; use a leading `/` for absolute. Auto-creates parent dirs.

Common operations:
- read   `curl $MDFS_URL/fs/notes/idea.md -H "Authorization: Bearer $MDFS_TOKEN"`
- write  `curl -X PUT $MDFS_URL/fs/notes/idea.md -H "Authorization: Bearer $MDFS_TOKEN" --data-binary "$BODY"`
- list   `curl $MDFS_URL/fs/notes -H "Authorization: Bearer $MDFS_TOKEN"`
- grep   `curl "$MDFS_URL/search/grep?pattern=TODO&recursive=true" -H "Authorization: Bearer $MDFS_TOKEN"`
- commit `curl -X POST $MDFS_URL/vcs/commit -H "Authorization: Bearer $MDFS_TOKEN" -H 'content-type: application/json' -d '{"message":"snapshot"}'`
```

Set `MDFS_TOKEN` in your shell once and Claude Code will pick it up.

---

## Generic MCP client

For any MCP client that supports the streamable-HTTP transport, use:

- **Endpoint:** `https://<your-space>.hf.space/mcp`
- **Method:** `POST` for tool calls, `GET` for SSE streaming
- **Required headers:**
  - `Content-Type: application/json`
  - `Accept: application/json, text/event-stream`
  - `Authorization: Bearer <token>` (or `User <name>`)
  - `X-MarkdownFS-On-Behalf-Of: <username>` (optional; for delegation)

Initialize, then call any tool by name. See [the contract](https://github.com/subramanya1997/markdownfs/blob/master/clients/CONTRACT.md) for the full surface.

---

## Anthropic API (Claude SDK with MCP)

Use the Claude API's [MCP connector](https://docs.claude.com/en/docs/build-with-claude/mcp) feature to give Claude direct access:

```python
import anthropic

client = anthropic.Anthropic()

response = client.messages.create(
    model="claude-opus-4-7",
    max_tokens=2048,
    mcp_servers=[
        {
            "type": "url",
            "url": "https://subramanya97-markdownfs.hf.space/mcp",
            "name": "markdownfs",
            "authorization_token": "YOUR_AGENT_TOKEN_HERE",
        }
    ],
    messages=[
        {"role": "user", "content": "Read notes/idea.md, refine it, then commit."}
    ],
    extra_headers={
        "anthropic-beta": "mcp-client-2025-04-04",
    },
)
print(response.content)
```

Claude will call MarkdownFS tools (`read_file`, `write_file`, `commit`, ...) inline as part of completing the task. Every call respects Unix permissions on the Space.

---

## Python (without MCP)

Use the [`markdownfs`](sdks/python.md) SDK directly:

```bash
uv add markdownfs   # or: pip install markdownfs
```

```python
import os
from markdownfs import MarkdownFS

mdfs = MarkdownFS(
    base_url=os.environ["MDFS_URL"],
    token=os.environ["MDFS_TOKEN"],
    # on_behalf_of="alice",   # optional; act on behalf of alice
)

# Relative paths land under your home
mdfs.fs.write("notes/idea.md", "# my idea")
print(mdfs.fs.read("notes/idea.md"))

# Search
hits = mdfs.search.grep("TODO", recursive=True)

# Commit
result = mdfs.vcs.commit("snapshot")
print("committed", result["hash"])
```

Async mirror:

```python
from markdownfs import AsyncMarkdownFS

async with AsyncMarkdownFS(base_url=..., token=...) as mdfs:
    await mdfs.fs.write("a.md", "# hi")
```

---

## TypeScript / Bun (without MCP)

```bash
bun add markdownfs   # or: npm i markdownfs
```

```ts
import { MarkdownFS } from "markdownfs";

const mdfs = new MarkdownFS({
  baseUrl: process.env.MDFS_URL!,
  token: process.env.MDFS_TOKEN,
  // onBehalfOf: "alice",
});

await mdfs.fs.write("notes/idea.md", "# my idea");
console.log(await mdfs.fs.read("notes/idea.md"));

const { hash } = await mdfs.vcs.commit("snapshot");
```

Works in Node 18+ and modern browsers (CORS is permissive on the Space).

---

## Stdio MCP (for local-only Claude Desktop / Cursor setups)

If you want to skip HTTP and run MCP locally:

```bash
git clone https://github.com/subramanya1997/markdownfs && cd markdownfs
cargo build --release --bin mdfs-mcp
```

Then in Claude Desktop config:

```json
{
  "mcpServers": {
    "markdownfs-local": {
      "command": "/absolute/path/to/markdownfs/target/release/mdfs-mcp",
      "env": {
        "MARKDOWNFS_DATA_DIR": "/path/to/local/data",
        "MARKDOWNFS_AS_USER": "alice",
        "MARKDOWNFS_ON_BEHALF_OF": "engineering"
      }
    }
  }
}
```

`MARKDOWNFS_AS_USER` selects the principal; `MARKDOWNFS_ON_BEHALF_OF` adds delegation. Local stdio data is stored on your disk, not on a Space.

---

## Path conventions you should know

These apply to **every** transport (REST, MCP, SDKs, CLI):

| You write | What the server resolves |
|---|---|
| `notes/idea.md` | `/home/<your-username>/notes/idea.md` |
| `~/diary/today.md` | `/home/<your-username>/diary/today.md` |
| `~` | `/home/<your-username>` |
| `/etc/foo` | `/etc/foo` (absolute — unchanged) |
| (empty) | `/home/<your-username>` |

When delegating with `X-MarkdownFS-On-Behalf-Of: alice`, **alice's** home is used.

For root / anonymous-root sessions, relative paths resolve at `/` (no `/home/root`).

Parent directories are auto-created on write — your first `notes/idea.md` will create `notes/` for you.

---

## Permissions cheat sheet

- Each user has a private home at `/home/<name>` (mode `0700` by default).
- Wheel members (admins) can **read** every directory and file regardless of mode.
- Wheel members can **chmod** files they don't own.
- Only literal root (`uid=0`) can chown files between users.
- Delegation enforces **intersection** semantics: an agent acting on behalf of alice can only do what *both* the agent and alice could do independently.

If you need a regular user to share files with an agent, either add the agent to the user's group + chmod the file to `0660`, or use `chown` to give the file to the agent.

---

## Quick troubleshooting

- **403 on first write** — your token is valid but the file you're writing to is owned by someone else. Try a relative path (lands under your home where you have write perms).
- **404 on relative path you just created** — your token may have switched users between calls. Run `GET /auth/whoami` to confirm identity.
- **Tokens missing** — they're only shown once when issued. Use the admin panel's **regen token** button to generate a fresh one.
- **MCP client says "no tools"** — make sure the URL ends in `/mcp` and the token has access. Try `curl https://<space>.hf.space/auth/whoami -H "Authorization: Bearer $TOKEN"` to verify auth.
