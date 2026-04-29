# Version Control

mdfs has built-in Git-style version control. You can snapshot the entire filesystem, view history, and revert to any previous state — all in microseconds.

## How It Works

Under the hood, mdfs uses content-addressable storage (the same approach as Git):

- **Blobs** store raw file content, identified by SHA-256 hash
- **Trees** store directory snapshots (list of entries with names, types, permissions)
- **Commits** point to a root tree, a parent commit, timestamp, message, and author

Identical content is stored only once. If 10,000 files have the same content, there's only one blob in the store.

## Committing

Snapshot the entire filesystem:

```
alice@markdownfs:/ $ commit initial project setup
[a1b2c3d4] initial project setup
```

The output shows the short hash (first 8 hex characters) and your commit message. If you omit the message, it defaults to "snapshot":

```
alice@markdownfs:/ $ commit
[e5f6a7b8] snapshot
```

The `edit` command auto-commits after every save with the message "edit \<path\>".

### What Gets Committed

A commit captures:
- Every file and directory in the filesystem
- File contents
- Directory structure
- Permission bits (mode), ownership (uid/gid) for each entry
- Symlink targets

User accounts and groups are **not** part of the commit — they're stored separately and preserved across reverts.

## Viewing History

```
alice@markdownfs:/ $ log
e5f6a7b8 2025-04-13 11:45:00 alice    update api docs
c3d4e5f6 2025-04-13 11:30:00 alice    add search feature notes
a1b2c3d4 2025-04-13 10:30:00 alice    initial project setup
```

Commits are shown newest-first. Each line shows:
- **Hash** — 8-character prefix of the SHA-256 commit ID
- **Timestamp** — when the commit was created
- **Author** — who created the commit
- **Message** — the commit description

## Checking Status

```
alice@markdownfs:/ $ status
On commit e5f6a7b8
Objects in store: 47
Files: 12, Total size: 4280 bytes
```

If no commits have been made yet:

```
alice@markdownfs:/ $ status
No commits yet.
Files: 12, Total size: 4280 bytes
```

## Reverting

Restore the filesystem to a previous commit:

```
alice@markdownfs:/ $ revert a1b2c3d4
Reverted to a1b2c3d4
```

You only need enough characters of the hash to uniquely identify the commit. If the prefix is ambiguous, you'll get an error.

### What Revert Does

1. Finds the commit matching the hash prefix
2. Reconstructs the entire filesystem from that commit's tree
3. Restores all files with their original content, permissions, and ownership
4. Updates HEAD to point to the reverted commit

### What Revert Preserves

- **User accounts and groups** are untouched — a revert never deletes users
- **The commit history itself** remains intact — you can still see all commits in `log`

### A Note on the Working Directory

After a revert, the CLI's working directory (`pwd`) is reset to `/`. If the path you were in still exists in the reverted state, simply `cd` back to it:

```
alice@markdownfs:~/project $ revert a1b2c3d4
Reverted to a1b2c3d4
alice@markdownfs:/ $ cd /home/alice/project
alice@markdownfs:~/project $
```

## Practical Workflow

Here's a typical version-controlled workflow:

```
# Set up a project
alice@markdownfs:/ $ mkdir -p project/docs
alice@markdownfs:/ $ touch project/readme.md
alice@markdownfs:/ $ write project/readme.md # My Project v1.0
alice@markdownfs:/ $ touch project/docs/api.md
alice@markdownfs:/ $ write project/docs/api.md # API v1

# Commit the initial state
alice@markdownfs:/ $ commit version 1.0 release
[a1b2c3d4] version 1.0 release

# Make changes
alice@markdownfs:/ $ write project/readme.md # My Project v2.0 with breaking changes
alice@markdownfs:/ $ write project/docs/api.md # API v2 — new endpoints
alice@markdownfs:/ $ touch project/docs/changelog.md
alice@markdownfs:/ $ write project/docs/changelog.md # Changelog

## v2.0
- Breaking API changes
- New endpoints

# Commit the update
alice@markdownfs:/ $ commit version 2.0 release
[e5f6a7b8] version 2.0 release

# Oops — v2.0 has issues, revert to v1.0
alice@markdownfs:/ $ revert a1b2c3d4
Reverted to a1b2c3d4

# Verify — we're back to v1.0
alice@markdownfs:/ $ cat project/readme.md
# My Project v1.0

alice@markdownfs:/ $ tree project
project/
├── docs/
│   └── api.md
└── readme.md

# The changelog.md is gone — it didn't exist in v1.0
```

## Content Deduplication

The content-addressable store automatically deduplicates data:

```
# Create 100 files with identical content
alice@markdownfs:/ $ mkdir templates
# (... create 100 identical template files ...)

alice@markdownfs:/ $ commit templates
[f1e2d3c4] templates

alice@markdownfs:/ $ status
On commit f1e2d3c4
Objects in store: 3
Files: 100, Total size: 5000 bytes
```

100 identical files produce only 3 objects: 1 blob (shared content), 1 tree (directory listing), and 1 commit. This keeps the store compact regardless of how much content is duplicated.

## Performance

Version control operations are extremely fast because everything is in-memory:

| Operation | Typical Time |
|---|---|
| Commit (10,000 files) | ~3 ms |
| Revert (5,000 files) | ~4 ms |
| Sequential commits (100 commits, 100 files) | ~33 µs per commit |

There's no disk I/O during commit or revert — the entire object store lives in memory and is persisted separately by the auto-save system.
