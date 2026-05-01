# markdownfs (TypeScript)

TypeScript / JavaScript client for [MarkdownFS](https://github.com/subramanya1997/markdownfs) — a versioned virtual filesystem for markdown, designed for AI agents.

## Install

```bash
npm i markdownfs
```

Requires Node 18+ (uses native `fetch`). Works in modern browsers.

## Quickstart

```ts
import { MarkdownFS } from "markdownfs";

const mdfs = new MarkdownFS({
  baseUrl: "https://you-markdownfs.hf.space",
  token: process.env.MDFS_TOKEN, // or: username: "alice"
});

await mdfs.fs.write("notes/idea.md", "# my idea\n");
const text = await mdfs.fs.read("notes/idea.md");
const list = await mdfs.fs.list("notes");

const hits = await mdfs.search.grep("TODO", { path: "notes" });

const { hash } = await mdfs.vcs.commit("snapshot");
const { commits } = await mdfs.vcs.log();
await mdfs.vcs.revert(hash);
```

## API

### Constructor

```ts
new MarkdownFS({
  baseUrl: string;        // required
  token?: string;         // Bearer token
  username?: string;      // alternative auth (User <name>)
  onBehalfOf?: string;    // delegate: agent acting on behalf of a user/group
                          // (intersection semantics; see CONTRACT.md)
  fetch?: typeof fetch;   // override (e.g. for testing)
  headers?: Record<string, string>;
})
```

### Top level

- `health()` → server status
- `login(username)` → session info

### `fs`

- `read(path)` → string
- `readBytes(path)` → Uint8Array
- `list(path?)` → `{ entries, path }`
- `stat(path)` → metadata
- `write(path, content)` — content is string or Uint8Array; parents auto-created
- `mkdir(path)` — recursive
- `remove(path, { recursive? })`
- `copy(src, dst)` / `move(src, dst)`
- `tree(path?)` → tree text

### `search`

- `grep(pattern, { path?, recursive? })` → `{ results, count }`
- `find({ path?, name? })` → `{ results, count }`

### `vcs`

- `commit(message)` → `{ hash, message, author }`
- `log()` → `{ commits }`
- `revert(hash)`
- `status()` → status text

### `auth`

- `whoami()` → session info (incl. `on_behalf_of`)
- `bootstrap(username)` → `{ username, token }` (first-run only)

### `admin` (root or wheel only)

- `admin.users.list()` / `create(name, { isAgent? })` / `delete(name)` / `issueToken(name)` / `addToGroup(name, group)` / `removeFromGroup(name, group)`
- `admin.groups.list()` / `create(name)` / `delete(name)`
- `admin.chmod(path, mode)` — e.g. `mode: "0644"`
- `admin.chown(path, owner, { group? })`

### Agents acting on behalf of a user

```ts
const agent = new MarkdownFS({
  baseUrl,
  token: process.env.AGENT_TOKEN,
  onBehalfOf: "alice",   // or ":alice-group", or "Bearer <user-token>"
});
// Permissions are the *intersection* of agent and alice — least privilege.
await agent.fs.read("home/alice/notes.md");
```

### Errors

All non-2xx responses throw `MarkdownFSError` with `status` and parsed `body`.

## License

MIT
