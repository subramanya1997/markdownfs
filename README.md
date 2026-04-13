# mdvfs

A high-performance, in-memory virtual file system built in Rust. mdvfs supports Unix-like commands, Git-style versioning with content-addressable storage, disk persistence, and multi-user permissioning — designed as a new-age database for AI agents and multi-tenant systems.

Only Markdown (`.md`) files are supported by design.

## Features

- **Unix-like CLI** — `ls`, `cd`, `mkdir`, `touch`, `cat`, `rm`, `mv`, `cp`, `grep`, `find`, `tree`, `chmod`, `ln -s`, and more
- **Pipe support** — chain commands: `grep "TODO" notes/ | head -5 | wc -l`
- **Inline editor** — `edit file.md` opens a multi-line editor with auto-commit
- **Git-style versioning** — `commit`, `log`, `revert`, `status` with full snapshot history
- **Content-addressable storage** — SHA-256 deduplication (10K identical files = 1 stored blob)
- **Disk persistence** — atomic save/load with bincode serialization, survives restarts
- **Multi-user permissions** — uid/gid ownership, rwx bits, setgid inheritance, sticky bit, token-based agent auth
- **Blazing fast** — in-memory HashMap-based inodes, ~130x average speedup over native filesystem operations

## Quick Start

```bash
cargo build --release
cargo run --release
```

On first run:

```
mdvfs v0.1.0 — Markdown Virtual File System
Type 'help' for available commands, 'exit' to quit.

mdvfs:/ $
```

State is automatically saved to `.vfs/state.bin` on exit and restored on next launch.

## Commands

### File Operations

| Command | Description |
|---|---|
| `ls [-l] [path]` | List directory contents |
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
| `cp <src> <dst>` | Copy file |
| `stat <path>` | Show file metadata (inode, mode, uid, gid, timestamps) |
| `tree [path]` | Directory tree view |
| `find [path] [-name pattern]` | Find files by glob pattern |
| `grep [-r] <pattern> [path]` | Search file contents (regex) |
| `head [-n N]` | First N lines (pipe) |
| `tail [-n N]` | Last N lines (pipe) |
| `wc [-l\|-w\|-c]` | Count lines/words/bytes (pipe) |
| `chmod <mode> <path>` | Change permissions (octal) |
| `ln -s <target> <link>` | Create symbolic link |
| `echo <text>` | Print text |

### Version Control

| Command | Description |
|---|---|
| `commit <message>` | Snapshot current filesystem state |
| `log` | Show commit history |
| `revert <hash>` | Revert to a previous commit |
| `status` | Show current state summary |

### Pipes

Commands can be chained with `|`:

```
mdvfs:/ $ echo "# My Notes" | write notes.md
mdvfs:/ $ cat notes.md | grep "Notes" | wc -l
1
mdvfs:/ $ find . -name *.md | head -5
```

### The `edit` Command

`edit` opens an interactive multi-line editor and auto-commits on save:

```
mdvfs:/ $ edit readme.md
Enter new content (type EOF on a blank line to finish, CANCEL to abort):
   1 | # Hello World
   2 | This is my document.
   3 | EOF
[a3f2c1d] edit readme.md
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
    mod.rs         All command implementations
    parser.rs      Tokenizer & pipeline parser
  fs/            Virtual filesystem core
    mod.rs         VirtualFs — inode management, path resolution, all FS ops
    inode.rs       Inode, InodeKind (File/Directory/Symlink)
  store/         Content-addressable object store
    mod.rs         ObjectId (SHA-256), ObjectKind
    blob.rs        BlobStore — deduplicated object storage
    tree.rs        TreeEntry, TreeObject (directory snapshots)
    commit.rs      CommitObject (snapshot metadata)
  vcs/           Version control
    mod.rs         Vcs — commit, log, revert, status
    revert.rs      Tree-to-filesystem reconstruction
    snapshot.rs    Filesystem-to-tree serialization
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
- **Markdown-only** — `touch` and `edit` enforce `.md` extension. This is intentional — mdvfs is a purpose-built store for structured text.

## Performance

Benchmarks against native filesystem (run with `cargo test --test perf_comparison --release`):

| Operation | mdvfs | Native | Speedup |
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

## Multi-User Permissions

mdvfs implements Unix-style ownership and permissions:

- Every inode has `uid`, `gid`, and `mode` (rwx bits)
- Root user (uid=0) bypasses all permission checks
- **Setgid** on directories: new files inherit the directory's group
- **Sticky bit**: only the file owner, directory owner, or root can delete
- **Agent authentication**: AI agents get SHA-256 hashed API tokens for programmatic access
- **Groups**: users can belong to multiple groups for shared directory access

## Testing

```bash
# Unit + integration tests (44 tests)
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
