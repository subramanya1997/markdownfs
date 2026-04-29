# Notebook Examples

These notebooks are runnable walkthroughs for trying `mdfs` from Python/Jupyter.

| Notebook | Shows |
|---|---|
| `01_getting_started_http.ipynb` | Start the HTTP server, write markdown, read it back, search, commit, and inspect history |
| `02_mount_and_start_using_mdfs.ipynb` | Build and run the optional FUSE mount, then use `mdfs` like a local directory |
| `03_user_delegation_to_agents.ipynb` | Create users and an agent, delegate work to the agent, and observe least-privilege behavior |

The notebooks auto-detect the repository root when launched from inside the repo, including from `examples/notebooks/`. If Jupyter starts somewhere else, set `MDFS_REPO_ROOT` to the repository path before running the first cell.

## Prerequisites

- Rust toolchain installed (`rustc` and `cargo`)
- Python 3 Jupyter environment
- For the mount notebook only: a working FUSE/macFUSE setup for your OS. On macOS, install `pkg-config` and macFUSE so `fuser` can find `fuse.pc`, for example `brew install pkg-config && brew install --cask macfuse`.

If Jupyter does not inherit your shell `PATH`, the notebooks will also look for Cargo at `~/.cargo/bin/cargo`, `/opt/homebrew/bin/cargo`, and `/usr/local/bin/cargo`, and for `pkg-config` at common Homebrew/MacPorts locations. You can override discovery with `CARGO=/absolute/path/to/cargo` and `PKG_CONFIG=/absolute/path/to/pkg-config`.

The HTTP and delegation notebooks use only Python's standard library.
