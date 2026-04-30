# Installation

MarkdownFS ships as a Rust core with three transports and two SDKs. Pick the surface that fits.

## Run the server

### Docker

```bash
docker run -p 7860:7860 \
  -e MARKDOWNFS_LISTEN=0.0.0.0:7860 \
  -v $PWD/data:/home/mdfs/data \
  ghcr.io/subramanya1997/markdownfs:latest
```

### From source (Rust)

```bash
git clone https://github.com/subramanya1997/markdownfs
cd markdownfs
cargo build --release
./target/release/mdfs-server
```

### Hugging Face Space

One-click deploy to a free Space — see the [Hugging Face guide](huggingface.md).

## Install an SDK

=== "TypeScript (Bun)"

    ```bash
    bun add markdownfs
    ```

    Also works with npm/pnpm/yarn. Requires Node 18+.

=== "Python (uv)"

    ```bash
    uv add markdownfs
    ```

    Or `pip install markdownfs`. Requires Python 3.9+.

=== "Rust"

    ```toml
    [dependencies]
    markdownfs = "0.2"
    ```

## Use the MCP server

Add to your Claude Desktop / Cursor config:

```json
{
  "mcpServers": {
    "markdownfs": {
      "command": "mdfs-mcp"
    }
  }
}
```
