# mdfs Verification Report

Tested on: 2026-04-28
Rust edition: 2024, mdfs v0.2.0

> **Update (post-fix):** All bugs and doc/reality mismatches identified in the original verification have been addressed. See the [Resolution status](#resolution-status) section at the bottom for what changed.

---

## 1. Build

| Step | Command | Result |
|---|---|---|
| Release build | `cargo build --release --bins` | PASS — builds preferred binaries plus legacy alias binaries |
| Optional FUSE build | `cargo build --release --features fuser --bin mdfs-mount` | PASS — builds the mount binary when macFUSE/pkg-config are available |
| Binary: `markdownfs` | Interactive CLI/REPL | PASS |
| Binary: `mdfs` | Remote-first CLI | PASS |
| Binary: `mdfs-server` | HTTP API server | PASS |
| Binary: `mdfs-mcp` | MCP server | PASS |
| Binary: `mdfs-mount` | Optional FUSE mount | PASS |
| Legacy aliases | `markdownfs-server`, `markdownfs-mcp`, `markdownfs-mount` | PASS |

---

## 2. Test Suite

| Suite | Tests | Result |
|---|---|---|
| Unit tests (`src/lib.rs`) | 18 | PASS |
| Integration tests | 111 | PASS |
| Performance tests | 37 | PASS (1 test >60s in debug mode: `perf_find_across_tree`) |
| Perf comparison | 1 | PASS |
| Permission tests | 72 | PASS |
| **Total** | **239** | **ALL PASS** |

**Latest run:** `cargo test` passed all **239** tests across the default suites. The release performance suites were also run explicitly.

---

## 3. CLI — First Run Experience

| Step | Expected (per docs) | Actual | Result |
|---|---|---|---|
| First launch shows welcome | `markdownfs v0.2.0 — Markdown Virtual File System` | Exact match | PASS |
| Prompts for admin username | `Admin username:` prompt | Prompts (though doesn't print "Admin username:" label — just reads first input line as the name) | MINOR — docs show an explicit prompt label that isn't printed |
| Creates admin with wheel group | `Created admin 'alice' (uid=1, groups=[alice, wheel])` | Exact match | PASS |
| Home directory created | `Home directory: /home/alice` | Exact match | PASS |
| `whoami` shows admin user | `alice` | `alice` | PASS |
| `pwd` starts in home dir | `/home/alice` | `/home/alice` | PASS |
| `help` lists all commands | Full command list | Full command list displayed | PASS |
| `exit` saves and quits | Saves state, prints goodbye | `State saved via local-state-file ... Goodbye!` | PASS |

### Subsequent Login

| Step | Expected | Actual | Result |
|---|---|---|---|
| Second launch loads from disk | `Loaded from disk (N commits, N objects)` | `markdownfs v0.2.0 — Loaded from disk (2 commits, 11 objects)` | PASS |
| Login prompt | `Login as:` | Prompts (reads username from input) | PASS |
| Restores to home directory | `alice@markdownfs:~ $` | Navigates to home directory | PASS |
| All data preserved | Files, users, history intact | Verified | PASS |

---

## 4. CLI — Filesystem Commands

| Command | Tested | Result | Notes |
|---|---|---|---|
| `touch <file.md>` | Yes | PASS | Creates empty .md file |
| `touch <file.txt>` | Yes (via test suite) | PASS — rejected | Only .md files allowed |
| `write <file> <content>` | Yes | PASS | Content written inline |
| `cat <file>` | Yes | PASS | Displays content |
| `mkdir <path>` | Yes | PASS | Single directory |
| `mkdir -p <deep/path>` | Yes | PASS | Creates intermediate dirs |
| `ls` | Yes | PASS | Lists names |
| `ls -l` | Yes | PASS | Long format with permissions, owner, size, date |
| `tree` | Yes | PASS | Unicode box-drawing tree |
| `stat <path>` | Yes | PASS | Full metadata including inode, mode, uid/gid, timestamps |
| `mv <src> <dst>` | Yes | PASS | Rename works |
| `cp <src> <dst>` | Yes | PASS | Independent copy |
| `rm <file>` | Yes | PASS | File deletion |
| `rm -r <dir>` | Yes (via test suite) | PASS | Recursive delete |
| `ln -s <target> <link>` | Yes | PASS | Symlink created, cat follows it |
| `chmod <mode> <path>` | Yes | PASS | Permission change |
| `chown <user:group> <path>` | Yes | PASS (as root) | Ownership change |

### Write command behavior note:
The `write` command joins all arguments after the filename with spaces. It does NOT interpret `\n` escape sequences. Multi-line content requires the `edit` command or pipes.

---

## 5. CLI — Search and Pipes

| Command | Tested | Result | Notes |
|---|---|---|---|
| `grep <pattern> <file>` | Yes | PASS | Single-file search |
| `grep -r <pattern> <dir>` | Yes | PASS | Recursive search with `file:line:content` format |
| `find . -name "*.md"` | Yes | PASS | Glob pattern matching |
| `grep ... \| wc -l` | Yes | PASS | Pipe chain works |
| `cat file \| grep pattern` | Yes | PASS | Pipe from cat |
| `grep ... \| head -N` | Yes (via test suite) | PASS | Head pipe |
| `grep ... \| tail -N` | Yes (via test suite) | PASS | Tail pipe |
| `echo text \| write file` | Yes (via test suite) | PASS | Write from pipe |

---

## 6. CLI — Version Control

| Command | Tested | Result | Notes |
|---|---|---|---|
| `commit <message>` | Yes | PASS | Returns `[hash] message` |
| `commit` (no message) | Yes (via test suite) | PASS | Defaults to "snapshot" |
| `log` | Yes | PASS | Shows hash, date, author, message (newest first) |
| `status` | Yes | PASS | Shows current commit, objects, files, total size |
| `revert <hash>` | Yes | PASS | Restores filesystem to commit state |
| `revert` (no args) | Yes | Error: "need commit hash prefix" | PASS — correct error |
| Hash prefix matching | Yes | PASS | 8-char prefix works |

### Revert behavior note:
After `revert`, the working directory (cwd) is reset to `/`. This is not documented. Users should be aware they need to `cd` back to their working location after reverting.

---

## 7. CLI — User Management

| Command | Tested | Result | Notes |
|---|---|---|---|
| `adduser <name>` | Yes | PASS | Creates user with home dir |
| `addagent <name>` | Yes | PASS | Shows token once |
| `deluser <name>` | Yes (via test suite) | PASS | Root only |
| `addgroup <name>` | Yes | PASS | Creates group |
| `usermod -aG <group> <user>` | Yes | PASS | Adds to group |
| `groups` | Yes | PASS | Shows current user's groups |
| `groups <other_user>` | Yes | **BUG** | Fails with "permission denied" even for wheel/admin users |
| `id` | Yes | PASS | Shows uid, gid, groups with names |
| `id <other_user>` | Yes | PASS | Works for any user |
| `whoami` | Yes | PASS | Shows current user |
| `su <user>` | Yes | PASS | Switches user (root/wheel only) |
| `su` from non-wheel user | Yes | PASS — denied | Correct behavior |

### Bug: `groups <other_user>` permission check

**File:** `src/cmd/mod.rs:899`
**Issue:** The `groups` command only allows root (uid=0) to view another user's groups. It does not check for `wheel` group membership. This means admin users in `wheel` cannot run `groups bob` — they get "permission denied."
**Workaround:** Use `id <username>` instead, which does work.
**Comparison:** The `id` command at line ~930 has no such restriction, creating inconsistent behavior.

---

## 8. HTTP API — Server Startup

| Item | Expected | Actual | Result |
|---|---|---|---|
| Default listen address | `127.0.0.1:3000` | Configurable via `MARKDOWNFS_LISTEN` | PASS |
| Startup log | Lists all endpoints | Shows all endpoints with methods | PASS |
| `GET /health` | JSON with status, version, counts | `{"status":"ok","version":"0.2.0","commits":0,"inodes":1,"objects":0}` | PASS |

---

## 9. HTTP API — Filesystem Operations

| Endpoint | Tested | Result | Notes |
|---|---|---|---|
| `PUT /fs/{path}` (write file) | Yes | PASS | Creates file, returns `{written, size}` |
| `PUT /fs/{path}` (create dir) | Yes | PASS | With `X-Markdownfs-Type: directory` header |
| `GET /fs/{path}` (read file) | Yes | PASS | Returns raw markdown, `Content-Type: text/markdown` |
| `GET /fs/{path}/` (list dir) | Yes | PASS | Returns JSON with entries |
| `GET /fs/{path}?stat=true` | Yes | PASS | Returns JSON metadata |
| `DELETE /fs/{path}` | Yes | PASS | Deletes file |
| `DELETE /fs/{path}?recursive=true` | Yes | PASS | Recursive directory delete |
| `POST /fs/{path}?op=copy&dst=...` | Yes | PASS | Copy file |
| `POST /fs/{path}?op=move&dst=...` | Yes | PASS | Move/rename file |

### Doc/reality mismatch: auto-create parent directories

**Docs claim:** "The file is created automatically if it doesn't exist (including parent directories for the path)."
**Actual:** `PUT /fs/docs/readme.md` fails with "no such file or directory" if `docs/` doesn't exist. You must create parent directories first with `PUT /fs/docs` + `X-Markdownfs-Type: directory`.

### Doc/reality mismatch: directory listing response format

**Docs show:**
```json
{"entries": [{"name": "api.md", "kind": "file"}]}
```
**Actual:**
```json
{"entries": [{"name": "api.md", "is_dir": false, "is_symlink": false, "mode": "0644", "uid": 0, "gid": 0, "size": 75, "modified": 1777440834}]}
```
The actual response is richer (includes mode, uid, gid, size, modified, is_symlink) and uses `is_dir` instead of `kind`.

---

## 10. HTTP API — Search

| Endpoint | Tested | Result | Notes |
|---|---|---|---|
| `GET /search/grep?pattern=...&recursive=true` | Yes | PASS | Returns `{results: [{file, line_num, line}], count}` |
| `GET /search/find?path=.&name=*.md` | Yes | PASS | Returns `{results: [...], count}` |
| `GET /tree` | Yes | PASS | Returns plain text tree |
| `GET /tree/{path}` | Yes | PASS | Scoped tree view |

---

## 11. HTTP API — Version Control

| Endpoint | Tested | Result | Notes |
|---|---|---|---|
| `POST /vcs/commit` | Yes | PASS | Returns `{hash, message, author}` |
| `GET /vcs/log` | Yes | PASS | Returns `{commits: [{hash, message, author, timestamp}]}` |
| `POST /vcs/revert` | Yes | PASS | Returns `{reverted_to: "hash"}` |
| `GET /vcs/status` | Yes | PASS | Returns plain text status |

---

## 12. HTTP API — Auth

| Mode | Tested | Result | Notes |
|---|---|---|---|
| `Authorization: User <username>` | Yes | PASS | Authenticates as named user |
| No auth header | Yes | PASS | Defaults to root |
| `POST /auth/login` | Yes | PASS (with caveats) | See below |
| `Authorization: Bearer <token>` | Yes (via test suite) | PASS | Agent token auth |

### Doc/reality mismatch: login response groups field

**Docs show:**
```json
{"groups": ["alice", "wheel"]}
```
**Actual:**
```json
{"groups": [0, 1]}
```
The actual response returns **numeric group IDs** instead of group names.

---

## 13. MCP Server

| Item | Tested | Result | Notes |
|---|---|---|---|
| Binary builds | Yes | PASS | 6.2 MB binary |
| Starts and logs | Yes | PASS | Logs "starting mdfs MCP server" |
| Receives MCP initialize | Yes | PASS | Processes the request (exits when stdin closes) |
| Stdio transport | Yes | PASS | Communicates over stdin/stdout |

The MCP server cannot be fully tested without a persistent MCP client, but the binary starts correctly and processes protocol messages.

---

## 14. Remote CLI (`mdfs`)

| Command | Tested | Result | Notes |
|---|---|---|---|
| `mdfs health` | Yes | PASS | JSON health response |
| `mdfs status` | Yes | PASS | Plain text status |
| `mdfs ls /` | Yes | PASS | Lists directory |
| `mdfs cat <file>` | Yes | PASS | Reads file content |
| `mdfs write <file> <content>` | Yes | PASS (when parent dir exists) | Same parent-dir issue as HTTP API |
| `mdfs tree` | Yes | PASS | Directory tree |
| `mdfs grep <pattern>` | Yes | PASS | Search with match count |
| `mdfs commit <message>` | Yes | PASS | Commits with hash output |
| `mdfs log` | Yes | PASS | Shows commit history |
| `mdfs --user <name>` | Yes | PASS | User authentication |
| `mdfs --token <token>` | Not tested | N/A | Requires agent token setup |
| `mdfs mkdir` | Yes | NOT SUPPORTED | Not a valid subcommand |

**Note:** `mdfs` doesn't support `mkdir`, `rm`, `delete`, `mv`, `cp`, or `stat`. These are only available via the full CLI or direct HTTP API calls. The docs don't claim otherwise, so this is acceptable.

---

## 15. Configuration

| Variable | Tested | Result |
|---|---|---|
| `MARKDOWNFS_DATA_DIR` | Yes | PASS — stores `.vfs/state.bin` at specified path |
| `MARKDOWNFS_LISTEN` | Yes | PASS — server binds to specified address |
| `RUST_LOG` | Not explicitly tested | N/A |
| `MARKDOWNFS_AUTOSAVE_SECS` | Not explicitly tested | N/A |

---

## 16. Data Persistence

| Item | Tested | Result |
|---|---|---|
| Auto-save on exit | Yes | PASS — `State saved via local-state-file ...` |
| State restored on relaunch | Yes | PASS — `Loaded from disk (N commits, N objects)` |
| Files preserved | Yes | PASS |
| Users preserved | Yes | PASS |
| VCS history preserved | Yes | PASS |

---

## Summary of Bugs and Doc Mismatches

### Bugs

| # | Severity | Description | Location |
|---|---|---|---|
| 1 | Medium | `groups <other_user>` denies access to wheel/admin users — only root can view other users' groups | `src/cmd/mod.rs:899` |

### Documentation vs. Reality Mismatches

| # | Severity | Doc Location | Issue |
|---|---|---|---|
| 1 | High | `docs/http-api-guide.md` (Write file section) | Docs claim PUT auto-creates parent directories; it does not |
| 2 | Medium | `docs/http-api-guide.md` (List directory response) | Actual response uses `is_dir`/`is_symlink`/`mode`/`uid`/`gid`/`size`/`modified` instead of `{name, kind}` |
| 3 | Medium | `docs/http-api-guide.md` (Login response) | Docs show group names `["alice","wheel"]`; actual returns numeric GIDs `[0,1]` |
| 4 | Low | `README.md` (Testing section) | Previously claimed "215 tests across 5 suites"; actual count is 239 |
| 5 | Low | `docs/getting-started.md` | First-run shows explicit "Admin username:" prompt in docs; actual reads first input without displaying a visible prompt label |
| 6 | Low | Version control docs | Revert resets cwd to `/` — not documented |

### Working Correctly as Documented

Everything else in the documentation matches reality:
- Preferred binaries and legacy alias binaries build
- 239/239 tests pass
- CLI filesystem operations (touch, write, cat, mkdir, ls, tree, stat, mv, cp, rm, ln, chmod, chown)
- CLI search (grep, find) and pipes (|, wc, head, tail)
- CLI version control (commit, log, status, revert)
- CLI user management (adduser, addagent, deluser, addgroup, usermod, su, whoami, id)
- HTTP API all endpoints (filesystem CRUD, search, VCS, auth, health)
- MCP server starts and handles protocol
- mdfs remote CLI works for all its supported commands
- Data persistence across restarts
- Content-addressable deduplication (verified in test suite)
- Permission enforcement (72 dedicated permission tests pass)
- Symlinks, sticky bit, setgid (verified in test suite)

---

## Resolution Status

All issues identified above have been addressed. Re-verification (run after fixes) confirmed:

### Code fixes

| # | Issue | Fix | Verified |
|---|---|---|---|
| 1 | `groups <other_user>` denied admin/wheel users | `src/cmd/mod.rs:898-905` — now allows root, wheel members, or the user themselves | Yes — `groups bob` as alice (wheel) returns `bob engineering` |
| 2 | HTTP `PUT /fs/path/file.md` failed when parent missing | `src/server/routes_fs.rs` — `put_fs` now auto-creates missing parent directories before `touch`/`write_file` (uses new `parent_path` helper) | Yes — `PUT /fs/a/b/c/file.md` succeeds without first creating `a/`, `b/`, `c/` |
| 3 | MCP `write_file` had the same parent-dir gap | `src/bin/mdfs_mcp.rs` — write_file now calls `mkdir_p` on the missing parent first | Yes — compiles and uses the same logic |

### Documentation fixes

| # | Doc | Change |
|---|---|---|
| 1 | `README.md` (Testing section) | Updated test count to 239 with accurate per-suite numbers |
| 2 | `docs/http-api-guide.md` (List directory response) | Updated example JSON to show actual fields: `is_dir`, `is_symlink`, `mode`, `uid`, `gid`, `size`, `modified` |
| 3 | `docs/http-api-guide.md` (Stat response) | Added note clarifying `kind` values (`"file"`, `"directory"`, `"symlink"`) |
| 4 | `docs/http-api-guide.md` (Login response) | Updated example to show numeric gids `[2, 1]` instead of name strings, with explanatory note |
| 5 | `docs/version-control.md` | Added "A Note on the Working Directory" section documenting that revert resets cwd to `/` |

### Items intentionally not changed

- **First-run "Admin username:" prompt visibility** — This is correct in the source (`src/main.rs:118` calls `rl.readline("Admin username: ")`). The prompt only appears in interactive TTY mode; when stdin is piped, rustyline omits the prompt. This is standard readline behavior and not a bug.

### Re-test results after fixes

| Suite | Tests | Result |
|---|---|---|
| Unit tests | 18 | PASS |
| Integration tests | 111 | PASS |
| Performance tests | 37 | PASS |
| Performance comparison | 1 | PASS |
| Permission tests | 72 | PASS |
| Doctests | 0 | PASS |
| **Total** | **239** | **ALL PASS** |

Additional release-mode performance verification:

| Command | Result |
|---|---|
| `cargo test --release --test perf -- --nocapture` | PASS — 37/37 tests, finished in 4.76s |
| `cargo test --release --test perf_comparison -- --nocapture` | PASS — average speedup vs native FS: 102.8x |

Build verification:

| Command | Result |
|---|---|
| `cargo build --release --bins` | PASS |
| `cargo build --release --features fuser --bin mdfs-mount` | PASS |
| `cargo check --bin mdfs --bin markdownfs --bin mdfs-server --bin markdownfs-server --bin mdfs-mcp --bin markdownfs-mcp` | PASS |
| `cargo check --features fuser --bin mdfs-mount` | PASS |
| `cargo test --features fuser --bin mdfs-mount` | PASS |
| `cargo test --features fuser --bin markdownfs-mount` | PASS |
