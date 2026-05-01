# Client / Server contract

Single source of truth for what every SDK must implement against `mdfs-server`. Change this file deliberately when the API changes — drift between server and clients is the failure mode this document exists to prevent.

## Auth

| Header                                    | Effect                                              |
|-------------------------------------------|-----------------------------------------------------|
| `Authorization: Bearer <token>`           | Authenticate the principal by issued token          |
| `Authorization: User <name>`              | Authenticate by username (no password; dev/local)   |
| `X-MarkdownFS-On-Behalf-Of: Bearer <tok>` | Delegate to the user identified by that token       |
| `X-MarkdownFS-On-Behalf-Of: <username>`   | Delegate to a user by name (gated)                  |
| `X-MarkdownFS-On-Behalf-Of: :<group>`     | Delegate to a group (gated)                         |
| (none)                                    | Anonymous root session                              |

**Delegation semantics:** the resulting session's permissions are the **intersection** of the principal and the delegate. Both must allow access. Name- and group-based delegation is allowed only when the principal is root, a wheel member, or marked as an agent.

## Endpoints

### Health & auth

| Method | Path             | Body                | Response                                                                  |
|--------|------------------|---------------------|---------------------------------------------------------------------------|
| GET    | `/health`        | —                   | `{status, version, commits, inodes, objects, needs_bootstrap}`            |
| GET    | `/auth/whoami`   | —                   | `{username, uid, gid, groups, is_root, authenticated, on_behalf_of}`      |
| POST   | `/auth/login`    | `{username}`        | `{username, uid, gid, groups}`                                            |
| POST   | `/auth/bootstrap`| `{username}`        | `{username, token}` — only succeeds if `needs_bootstrap == true`          |

### Filesystem

| Method | Path                | Query / headers                                       | Response                                            |
|--------|---------------------|-------------------------------------------------------|-----------------------------------------------------|
| GET    | `/fs`               | —                                                     | `{entries, path}`                                   |
| GET    | `/fs/{path}`        | `?stat=true` for metadata                             | file body, `{entries,path}`, or stat JSON           |
| PUT    | `/fs/{path}`        | `X-MarkdownFS-Type: directory` to mkdir               | `{written, size}` or `{created, type}`              |
| DELETE | `/fs/{path}`        | `?recursive=true` for rm -rf                          | `{deleted}`                                         |
| POST   | `/fs/{path}`        | `?op=copy\|move&dst=...`                              | `{copied/moved, to}`                                |

### Search & tree

| Method | Path             | Query                              | Response                                       |
|--------|------------------|------------------------------------|------------------------------------------------|
| GET    | `/search/grep`   | `pattern`, `path?`, `recursive?`   | `{results: [{file, line_num, line}], count}`   |
| GET    | `/search/find`   | `path?`, `name?`                   | `{results, count}`                             |
| GET    | `/tree[/{path}]` | —                                  | tree text                                      |

### VCS

| Method | Path           | Body              | Response                              |
|--------|----------------|-------------------|---------------------------------------|
| POST   | `/vcs/commit`  | `{message}`       | `{hash, message, author}`             |
| GET    | `/vcs/log`     | —                 | `{commits: [{hash,message,author,timestamp}]}` |
| POST   | `/vcs/revert`  | `{hash}`          | `{reverted_to}`                       |
| GET    | `/vcs/status`  | —                 | status text                           |

### Admin (gated to root or wheel)

| Method | Path                                          | Body                       | Response                                                                  |
|--------|-----------------------------------------------|----------------------------|---------------------------------------------------------------------------|
| GET    | `/admin/users`                                | —                          | `{users: [{uid, name, groups, is_agent, has_token}]}`                     |
| POST   | `/admin/users`                                | `{name, is_agent?}`        | `{uid, name, token}` (token only set if `is_agent`)                       |
| DELETE | `/admin/users/{name}`                         | —                          | `{deleted}`                                                               |
| POST   | `/admin/users/{name}/tokens`                  | —                          | `{name, token}`                                                           |
| POST   | `/admin/users/{name}/groups/{group}`          | —                          | `{user, group, added: true}`                                              |
| DELETE | `/admin/users/{name}/groups/{group}`          | —                          | `{user, group, removed: true}`                                            |
| GET    | `/admin/groups`                               | —                          | `{groups: [{gid, name, members}]}`                                        |
| POST   | `/admin/groups`                               | `{name}`                   | `{gid, name}`                                                             |
| DELETE | `/admin/groups/{name}`                        | —                          | `{deleted}`                                                               |
| POST   | `/admin/chmod/{*path}`                        | `{mode: "0644"}`           | `{path, mode}`                                                            |
| POST   | `/admin/chown/{*path}`                        | `{owner, group?}`          | `{path, owner, group}`                                                    |

### MCP (Model Context Protocol)

| Method | Path     | Notes                                                                                          |
|--------|----------|------------------------------------------------------------------------------------------------|
| POST   | `/mcp`   | rmcp streamable-HTTP transport. Same Auth + delegation headers as REST.                        |

## Errors

Non-2xx responses return JSON `{error: string}` whenever possible. Clients map this to a typed `MarkdownFSError` exposing `status` and the parsed body.

Status mapping:
- `401 Unauthorized` — invalid credential
- `403 Forbidden` — credential is valid but lacks permission (or denied delegation)
- `404 Not Found` — path / user / group does not exist
- `409 Conflict` — bootstrap attempted on an initialized workspace
- `400 Bad Request` — malformed input

## Versioning

The contract is tied to the server's `Cargo.toml` version. Any breaking change here requires:

1. Bumping the server version
2. Bumping each SDK's major version
3. Documenting the change in `CHANGELOG.md`
