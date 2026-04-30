# Deployment

## Hugging Face Space (recommended for trying it)

The simplest way to host MarkdownFS publicly. See the [Hugging Face guide](getting-started/huggingface.md).

## Docker

```bash
docker build -t markdownfs -f huggingface/Dockerfile .
docker run -d -p 7860:7860 \
  -e MARKDOWNFS_LISTEN=0.0.0.0:7860 \
  -v $PWD/data:/home/mdfs/data \
  markdownfs
```

## Self-hosted (systemd)

```ini
[Unit]
Description=MarkdownFS
After=network.target

[Service]
Environment=MARKDOWNFS_LISTEN=0.0.0.0:3000
Environment=MARKDOWNFS_DATA_DIR=/var/lib/markdownfs
ExecStart=/usr/local/bin/mdfs-server
Restart=on-failure
User=markdownfs

[Install]
WantedBy=multi-user.target
```

## Configuration

| Variable                       | Default            | Notes                            |
|--------------------------------|--------------------|----------------------------------|
| `MARKDOWNFS_DATA_DIR`          | current directory  | Where state is persisted         |
| `MARKDOWNFS_LISTEN`            | `127.0.0.1:3000`   | Bind address                     |
| `MARKDOWNFS_AUTOSAVE_SECS`     | `5`                | Auto-save interval               |
| `MARKDOWNFS_AUTOSAVE_WRITES`   | `100`              | Auto-save after N writes         |
| `MARKDOWNFS_MAX_FILE_SIZE`     | `10485760` (10 MB) | Per-file cap                     |
| `MARKDOWNFS_MAX_INODES`        | `1000000`          | Filesystem capacity              |
| `MARKDOWNFS_MAX_DEPTH`         | `256`              | Max directory depth              |
| `MARKDOWNFS_COMPAT_TARGET`     | `markdown`         | File-type compatibility target   |
| `RUST_LOG`                     | `markdownfs=info`  | Logging filter                   |

## Persistence

MarkdownFS persists state via atomic bincode snapshots in `MARKDOWNFS_DATA_DIR`. Snapshots happen on auto-save, on `vcs/commit`, and on graceful shutdown. Mount durable storage (a Docker volume, an HF Persistent Storage volume, etc.) at that path.
