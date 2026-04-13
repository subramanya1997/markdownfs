# markdownfs

A high-performance, in-memory virtual file system built in Rust. markdownfs supports Unix-like commands, Git-style versioning with content-addressable storage, disk persistence, and multi-user permissioning — designed as a new-age database for AI agents and multi-tenant systems.

Only Markdown (`.md`) files are supported by design.

## Features

- **Unix-like CLI** — `ls`, `cd`, `mkdir`, `touch`, `cat`, `rm`, `mv`, `cp`, `grep`, `find`, `tree`, `chmod`, `chown`, `ln -s`, and more
- **Pipe support** — chain commands: `grep "TODO" notes/ | head -5 | wc -l`
- **Inline editor** — `edit file.md` opens a multi-line editor with auto-commit
- **Git-style versioning** — `commit`, `log`, `revert`, `status` with full snapshot history and author tracking
- **Content-addressable storage** — SHA-256 deduplication (10K identical files = 1 stored blob)
- **Disk persistence** — atomic save/load with bincode serialization, survives restarts
- **Multi-user permissions** — uid/gid ownership, rwx permission enforcement on every operation, setgid inheritance, sticky bit
- **Visibility filtering** — users only see files they have permission to read (`ls`, `tree`, `find`, `grep -r` all filter)
- **Agent authentication** — token-based auth for AI agents and third-party systems
- **User management** — `adduser`, `addagent`, `deluser`, `addgroup`, `delgroup`, `usermod`, `groups`, `whoami`, `su`, `id`
- **Blazing fast** — in-memory HashMap-based inodes, ~130x average speedup over native filesystem operations

## Quick Start

```bash
cargo build --release
cargo run --release
```

On first run, you'll be prompted to create an admin account:

```
markdownfs v0.1.0 — Markdown Virtual File System
No users found. Let's create an admin account.
Admin username: alice
Logged in as 'alice' (uid=1, gid=2)

Type 'help' for available commands, 'exit' to quit.

alice@markdownfs:/ $
```

On subsequent runs, you'll be prompted to log in:

```
markdownfs v0.1.0 — Loaded from disk (5 commits, 42 objects)
Login as: alice
Logged in as 'alice' (uid=1, gid=2)
```

State is automatically saved to `.vfs/state.bin` on exit and restored on next launch.

## Commands

### File Operations

| Command | Description |
|---|---|
| `ls [-l] [path]` | List directory contents (filtered by permission) |
| `cd [path]` | Change directory |
| `pwd` | Print working directory |
| `mkdir [-p] <path>` | Create directory (`-p` for nested) |
| `touch <file.md>` | Create empty markdown file |
| `cat <file>` | Display file contents |
| `write <file> [content]` | Write content to file |
| `edit <file.md>` | Multi-line editor with auto-commit |
| `rm [-r] <path>` | Remove file or directory |
| `rmdir <path>` | Remove empty directory |
| `mv <src> <dst>` | Move or rename |
| `cp <src> <dst>` | Copy file (owned by caller) |
| `stat <path>` | Show file metadata (inode, mode, owner, group, timestamps) |
| `tree [path]` | Directory tree view (filtered by permission) |
| `find [path] [-name pattern]` | Find files by glob pattern (filtered by permission) |
| `grep [-r] <pattern> [path]` | Search file contents (filtered by permission) |
| `head [-n N]` | First N lines (pipe) |
| `tail [-n N]` | Last N lines (pipe) |
| `wc [-l\|-w\|-c]` | Count lines/words/bytes (pipe) |
| `chmod <mode> <path>` | Change permissions (owner or root) |
| `chown <user:group> <path>` | Change ownership (root or owner for group) |
| `ln -s <target> <link>` | Create symbolic link |
| `echo <text>` | Print text |

### User Management

| Command | Description |
|---|---|
| `adduser <name>` | Create user (admin only) |
| `addagent <name>` | Create agent with API token (admin only) |
| `deluser <name>` | Delete user (root only) |
| `addgroup <name>` | Create group (admin only) |
| `delgroup <name>` | Delete group (root only) |
| `usermod -aG <group> <user>` | Add user to group (admin only) |
| `usermod -rG <group> <user>` | Remove user from group (admin only) |
| `groups [user]` | Show group memberships |
| `whoami` | Show current user |
| `id [user]` | Show user/group identity |
| `su <user>` | Switch user (root or wheel members) |

### Version Control

| Command | Description |
|---|---|
| `commit <message>` | Snapshot current filesystem state (tracks author) |
| `log` | Show commit history with author |
| `revert <hash>` | Revert to a commit (preserves file ownership) |
| `status` | Show current state summary |

### Pipes

Commands can be chained with `|`:

```
alice@markdownfs:/ $ echo "# My Notes" | write notes.md
alice@markdownfs:/ $ cat notes.md | grep "Notes" | wc -l
1
alice@markdownfs:/ $ find . -name *.md | head -5
```

### The `edit` Command

`edit` opens an interactive multi-line editor and auto-commits on save:

```
alice@markdownfs:/ $ edit readme.md
Enter new content (type EOF on a blank line to finish, CANCEL to abort):
   1 | # Hello World
   2 | This is my document.
   3 | EOF
[a3f2c1d] edit readme.md
```

## Multi-User Permissions

markdownfs implements full Unix-style ownership and permission enforcement:

### Permission Model

- Every inode has `uid`, `gid`, and `mode` (rwx bits in owner/group/other format)
- Root user (uid=0) bypasses all permission checks
- Permission checks on **every operation**: path traversal (execute), read, write, create, delete
- `ls -l` shows owner and group names:
  ```
  drwxr-xr-x alice    devs          0 Apr 13 10:00 shared/
  -rw-r-----  bob     devs        142 Apr 13 10:05 notes.md
  ```

### Visibility Filtering

Users only see files they have read permission for. This applies to:
- `ls` — hides entries the user can't read
- `tree` — skips subtrees the user can't access
- `find` — omits files the user can't read
- `grep -r` — only searches files the user can read

### Special Bits

- **Setgid** on directories (`chmod 2755`): new files inherit the directory's group, enabling team collaboration
- **Sticky bit** on directories (`chmod 1777`): only the file owner, directory owner, or root can delete entries (like `/tmp`)

### Agent Authentication

AI agents are created with `addagent` and receive a SHA-256 hashed API token:

```
root@markdownfs:/ $ addagent crawler-bot
Agent 'crawler-bot' created (uid=3)
API token (save this — shown only once):
  a1b2c3d4e5f6...
```

### Example: Multi-Tenant Setup

```bash
# As admin, create team structure
addgroup engineering
adduser alice
adduser bob
usermod -aG engineering alice
usermod -aG engineering bob

# Create shared workspace with setgid
mkdir projects
chown root:engineering projects
chmod 2775 projects

# Alice creates a file — automatically inherits 'engineering' group
su alice
touch projects/design.md
write projects/design.md # Architecture Notes

# Bob can access it through group permissions
su bob
cat projects/design.md    # works!
```

## Architecture

```
src/
  auth/          Multi-user identity & permissions
    mod.rs         User, Group types, uid/gid constants
    registry.rs    UserRegistry — CRUD, agent tokens, group membership
    perms.rs       Permission checks (rwx bits, setgid, sticky)
    session.rs     Session — current user context
  cmd/           Command dispatch & pipe execution
    mod.rs         All command implementations + permission enforcement
    parser.rs      Tokenizer & pipeline parser
  fs/            Virtual filesystem core
    mod.rs         VirtualFs — inode management, path resolution, all FS ops
    inode.rs       Inode, InodeKind (File/Directory/Symlink), uid/gid ownership
  store/         Content-addressable object store
    mod.rs         ObjectId (SHA-256), ObjectKind
    blob.rs        BlobStore — deduplicated object storage
    tree.rs        TreeEntry (with uid/gid), TreeObject
    commit.rs      CommitObject (with author tracking)
  vcs/           Version control
    mod.rs         Vcs — commit, log, revert, status
    revert.rs      Tree-to-filesystem reconstruction (preserves ownership)
    snapshot.rs    Filesystem-to-tree serialization (captures ownership)
  io/            I/O utilities
  persist.rs     Disk persistence (atomic save/load, V1/V2 migration)
  error.rs       VfsError enum
  lib.rs         Module declarations
  main.rs        REPL, edit command, login flow
```

### Key Design Decisions

- **In-memory inodes** — `HashMap<InodeId, Inode>` for O(1) lookups. Directories use `BTreeMap` for sorted entries.
- **Content-addressable storage** — Objects are keyed by SHA-256 hash. Identical content is stored once, regardless of how many files reference it.
- **Atomic persistence** — Writes to a temp file, then renames. No partial state on crash.
- **Bincode serialization** — Binary format, faster than JSON/CBOR for known structures.
- **Markdown-only** — `touch` and `edit` enforce `.md` extension. This is intentional — markdownfs is a purpose-built store for structured text.
- **Session-based identity** — Session is held in the REPL, not stored in the filesystem. Passed through the entire dispatch chain.
- **Visibility = permission** — If you can't read it, you can't see it. No information leakage through directory listings.

## Performance

Benchmarks against native filesystem (run with `cargo test --test perf_comparison --release`):

| Operation | markdownfs | Native | Speedup |
|---|---|---|---|
| File creation (10K) | ~microseconds | ~milliseconds | ~50-100x |
| Sequential reads (10K) | ~microseconds | ~milliseconds | ~30-80x |
| Directory listing | ~microseconds | ~milliseconds | ~15-30x |
| Commit (10K files) | ~milliseconds | N/A | — |
| Persistence save | ~milliseconds | N/A | — |

Run the full benchmark suite:

```bash
cargo test --test perf --release -- --nocapture
cargo test --test perf_comparison --release -- --nocapture
```

## Testing

```bash
# All tests (49 tests: 17 unit + 31 integration + 1 perf comparison)
cargo test --test integration --lib --test perf_comparison

# Integration tests (includes permission tests)
cargo test --test integration

# Auth module tests
cargo test auth

# Performance benchmarks
cargo test --test perf --release -- --nocapture

# Native filesystem comparison
cargo test --test perf_comparison --release -- --nocapture
```

## License

MIT
