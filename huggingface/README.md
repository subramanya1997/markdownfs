---
title: MarkdownFS
emoji: 📝
colorFrom: indigo
colorTo: purple
sdk: docker
app_port: 7860
pinned: false
license: mit
short_description: A versioned markdown filesystem with a REST API for agents.
---

# MarkdownFS on Hugging Face

MarkdownFS is a high-performance concurrent markdown database with built-in
versioning, permissions, and a REST API. This Space hosts the `mdfs-server`
binary so any agent or app can use it as durable, inspectable workspace memory.

## Endpoints

Base URL: `https://<your-username>-<space-name>.hf.space`

| Method | Path                  | Purpose                          |
|--------|-----------------------|----------------------------------|
| GET    | `/health`             | Health check                     |
| GET    | `/fs/{path}`          | Read file or list directory      |
| PUT    | `/fs/{path}`          | Write file or create directory   |
| DELETE | `/fs/{path}`          | Delete file or directory         |
| GET    | `/search/grep`        | Grep across markdown             |
| GET    | `/search/find`        | Find by name                     |
| GET    | `/tree/{path}`        | Directory tree                   |
| POST   | `/vcs/commit`         | Commit current state             |
| GET    | `/vcs/log`            | Commit history                   |
| POST   | `/vcs/revert`         | Revert to a commit               |

See full docs at the project repo.

## Quick examples

```bash
SPACE=https://your-username-markdownfs.hf.space

# write a file
curl -X PUT "$SPACE/fs/notes/hello.md" \
  -H "Content-Type: text/markdown" \
  --data-binary "# Hello from MarkdownFS"

# read it back
curl "$SPACE/fs/notes/hello.md"

# commit
curl -X POST "$SPACE/vcs/commit" \
  -H "Content-Type: application/json" \
  -d '{"message": "first note"}'
```

## Storage

By default the Space uses ephemeral container storage at
`/home/mdfs/data`. **State is lost when the Space restarts.**

For durable storage, either:
- Enable Hugging Face **Persistent Storage** for this Space and point
  `MARKDOWNFS_DATA_DIR` at the mounted volume, or
- Periodically sync the data directory to an HF Dataset repo.

## Configuration

Override via Space → Settings → Variables:

| Variable                       | Default            |
|--------------------------------|--------------------|
| `MARKDOWNFS_DATA_DIR`          | `/home/mdfs/data`  |
| `MARKDOWNFS_LISTEN`            | `0.0.0.0:7860`     |
| `MARKDOWNFS_AUTOSAVE_SECS`     | `5`                |
| `MARKDOWNFS_MAX_FILE_SIZE`     | `10485760`         |
| `RUST_LOG`                     | `markdownfs=info`  |
