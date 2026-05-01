# markdownfs (Python)

Python client for [MarkdownFS](https://github.com/subramanya1997/markdownfs) — a versioned virtual filesystem for markdown, designed for AI agents.

## Install

With [uv](https://docs.astral.sh/uv/):

```bash
uv add markdownfs
```

Or pip:

```bash
pip install markdownfs
```

Requires Python 3.9+.

## Quickstart (sync)

```python
from markdownfs import MarkdownFS

mdfs = MarkdownFS(
    base_url="https://you-markdownfs.hf.space",
    token="...",   # or username="alice"
)

mdfs.fs.write("notes/idea.md", "# my idea\n")
text = mdfs.fs.read("notes/idea.md")
listing = mdfs.fs.list("notes")

hits = mdfs.search.grep("TODO", path="notes")

commit = mdfs.vcs.commit("snapshot")
mdfs.vcs.log()
mdfs.vcs.revert(commit["hash"])
```

## Async

```python
from markdownfs import AsyncMarkdownFS

async with AsyncMarkdownFS(base_url="...", token="...") as mdfs:
    await mdfs.fs.write("a.md", "# hi")
    text = await mdfs.fs.read("a.md")
```

## API

### Constructor

```python
MarkdownFS(
    base_url: str,
    *,
    token: str | None = None,
    username: str | None = None,
    on_behalf_of: str | None = None,   # delegate to user / ":group" / "Bearer <tok>"
    timeout: float = 30.0,
    client: httpx.Client | None = None,
)
```

`AsyncMarkdownFS` mirrors the sync API and accepts an optional `httpx.AsyncClient`.

### Agents acting on behalf of a user

```python
agent = MarkdownFS(
    base_url=...,
    token=os.environ["AGENT_TOKEN"],
    on_behalf_of="alice",  # intersection semantics
)
agent.fs.read("home/alice/notes.md")
```

### Top level

- `health()` → server status
- `login(username)` → session info

### `fs`

- `read(path)` → str
- `read_bytes(path)` → bytes
- `list(path="")` → `{ entries, path }`
- `stat(path)` → metadata dict
- `write(path, content)` — `content` is str or bytes; parents auto-created
- `mkdir(path)` — recursive
- `remove(path, recursive=False)`
- `copy(src, dst)` / `move(src, dst)`
- `tree(path="")` → tree text

### `search`

- `grep(pattern, path=None, recursive=None)` → `{ results, count }`
- `find(path=None, name=None)` → `{ results, count }`

### `vcs`

- `commit(message)` → `{ hash, message, author }`
- `log()` → `{ commits }`
- `revert(hash)`
- `status()` → status text

### `auth`

- `whoami()` — session info (incl. `on_behalf_of`)
- `bootstrap(username)` — first-run admin creation

### `admin` (root or wheel only)

- `admin.users.list()` / `create(name, is_agent=...)` / `delete(name)` / `issue_token(name)` / `add_to_group(name, group)` / `remove_from_group(name, group)`
- `admin.groups.list()` / `create(name)` / `delete(name)`
- `admin.chmod(path, mode)` — e.g. `mode="0644"`
- `admin.chown(path, owner, group=None)`

### Errors

Non-2xx responses raise `MarkdownFSError` with `.status` and `.body`.

## License

MIT
