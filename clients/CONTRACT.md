# Client / Server contract

Single source of truth for what every SDK must implement against `mdfs-server`. Change this file deliberately when the API changes — drift between server and clients is the failure mode this document exists to prevent.

## Auth

| Header                          | Effect                            |
|---------------------------------|-----------------------------------|
| `Authorization: Bearer <token>` | Authenticates by issued token     |
| `Authorization: User <name>`    | Authenticates by username (dev)   |
| (none)                          | Anonymous root session            |

## Endpoints

### Health & auth

| Method | Path           | Body                | Response                                                  |
|--------|----------------|---------------------|-----------------------------------------------------------|
| GET    | `/health`      | —                   | `{status, version, commits, inodes, objects}`             |
| POST   | `/auth/login`  | `{username}`        | `{username, uid, gid, groups}`                            |

### Filesystem

| Method | Path                | Query / headers                                       | Response                                            |
|--------|---------------------|-------------------------------------------------------|-----------------------------------------------------|
| GET    | `/fs`               | —                                                     | `{entries, path}`                                   |
| GET    | `/fs/{path}`        | `?stat=true` for metadata                             | file body, `{entries,path}`, or stat JSON           |
| PUT    | `/fs/{path}`        | `X-MarkdownFS-Type: directory` to mkdir               | `{written, size}` or `{created, type}`              |
| DELETE | `/fs/{path}`        | `?recursive=true` for rm -rf                          | `{deleted}`                                         |
| POST   | `/fs/{path}`        | `?op=copy\|move&dst=...`                              | `{copied/moved, to}`                                |

### Search

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

## Errors

Non-2xx responses return JSON `{error: string}` whenever possible. Clients map this to a typed `MarkdownFSError` exposing `status` and the parsed body.

## Versioning

The contract is tied to the server's `Cargo.toml` version. Any breaking change here requires:

1. Bumping the server version
2. Bumping each SDK's major version
3. Documenting the change in `CHANGELOG.md`
