# Agent Workspace Demo

This guide gives a runnable 7-minute demo for positioning `mdfs` as an agent workspace.

The current demo uses:

- the `markdownfs` CLI (legacy binary name) for one-time setup
- the `mdfs-server` HTTP API as the single writer
- normal shell commands like `curl` and `jq` so the agent can work through familiar CLI tools

## Demo Goal

Show that agents need more than raw filesystem access. They need a persistent workspace they can:

- inspect
- search
- update
- commit
- review
- revert

## Demo Setup

### 1. Initialize a fresh demo data directory

```bash
export MARKDOWNFS_DATA_DIR="$PWD/.demo/incident-workspace"
rm -rf "$MARKDOWNFS_DATA_DIR"
mkdir -p "$MARKDOWNFS_DATA_DIR"
```

### 2. Create the admin user and agent token

Run the CLI once:

```bash
cargo run --release --bin markdownfs
```

Create an admin user when prompted:

```text
Admin username: alice
```

Then set up shared top-level directories and an agent token:

```text
alice@markdownfs:~ $ su root
root@markdownfs:~ $ mkdir -p /incidents/checkout-latency
root@markdownfs:~ $ mkdir -p /runbooks
root@markdownfs:~ $ mkdir -p /memory/agents
root@markdownfs:~ $ chmod 777 /incidents /incidents/checkout-latency /runbooks /memory /memory/agents
root@markdownfs:~ $ addagent incident-bot
Created agent: incident-bot (uid=2)
Token: REPLACE_WITH_REAL_TOKEN
root@markdownfs:~ $ exit
```

Save the token in another terminal:

```bash
export MARKDOWNFS_TOKEN="REPLACE_WITH_REAL_TOKEN"
```

Exit the CLI before starting the HTTP server.

### 3. Start the HTTP server

```bash
MARKDOWNFS_DATA_DIR="$MARKDOWNFS_DATA_DIR" \
MARKDOWNFS_LISTEN=127.0.0.1:3000 \
cargo run --release --bin mdfs-server
```

### 4. Seed the workspace from the example files

In another terminal:

```bash
curl -s -X PUT http://localhost:3000/fs/incidents/checkout-latency/timeline.md \
  -H "Authorization: User alice" \
  --data-binary @examples/incident-workspace/incidents/checkout-latency/timeline.md

curl -s -X PUT http://localhost:3000/fs/incidents/checkout-latency/evidence.md \
  -H "Authorization: User alice" \
  --data-binary @examples/incident-workspace/incidents/checkout-latency/evidence.md

curl -s -X PUT http://localhost:3000/fs/incidents/checkout-latency/hypotheses.md \
  -H "Authorization: User alice" \
  --data-binary @examples/incident-workspace/incidents/checkout-latency/hypotheses.md

curl -s -X PUT http://localhost:3000/fs/runbooks/payment-service.md \
  -H "Authorization: User alice" \
  --data-binary @examples/incident-workspace/runbooks/payment-service.md

curl -s -X PUT http://localhost:3000/fs/memory/agents/researcher.md \
  -H "Authorization: User alice" \
  --data-binary @examples/incident-workspace/memory/agents/researcher.md
```

Create a baseline commit:

```bash
curl -s -X POST http://localhost:3000/vcs/commit \
  -H "Authorization: User alice" \
  -H "Content-Type: application/json" \
  -d '{"message":"seed incident workspace"}' | jq
```

Expected shape:

```json
{
  "hash": "abcd1234",
  "message": "seed incident workspace",
  "author": "alice"
}
```

## 7-Minute Script

### Minute 0-1: Frame the problem

Say:

> Most agent systems still leave behind transcripts. We want a workspace: persistent memory, inspectable files, commits, rollback, and permissioned access.

Show health:

```bash
curl -s http://localhost:3000/health | jq
```

Expected shape:

```json
{
  "status": "ok",
  "version": "0.2.0",
  "commits": 1
}
```

### Minute 1-2: Show workspace state through CLI tools

List the incident folder as the agent:

```bash
curl -s http://localhost:3000/fs/incidents/checkout-latency/ \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" | jq
```

Expected entries:

```json
{
  "path": "/incidents/checkout-latency",
  "entries": [
    {"name": "evidence.md", "kind": "file"},
    {"name": "hypotheses.md", "kind": "file"},
    {"name": "timeline.md", "kind": "file"}
  ]
}
```

Show the whole tree:

```bash
curl -s http://localhost:3000/tree \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN"
```

### Minute 2-3: Let the agent inspect evidence before writing

Read the runbook:

```bash
curl -s http://localhost:3000/fs/runbooks/payment-service.md \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN"
```

Search for prior timeout and retry signals:

```bash
curl -s "http://localhost:3000/search/grep?pattern=timeout|retry&recursive=true" \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" | jq
```

Expected result shape:

```json
{
  "results": [
    {
      "file": "runbooks/payment-service.md",
      "line_num": 7,
      "line": "If checkout latency spikes immediately after a payment-service deploy, inspect timeout and retry changes first."
    }
  ],
  "count": 1
}
```

Narration:

> The agent is using plain CLI tools against the workspace. Today that means `curl` and `jq`; later this becomes the `mdfs` CLI wrapper.

### Minute 3-4: Create new agent output

Write a root-cause summary:

```bash
cat <<'EOF' | curl -s -X PUT http://localhost:3000/fs/incidents/checkout-latency/root-cause.md \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" \
  --data-binary @-
# Root Cause

The most likely root cause is a payment-service timeout and retry regression introduced by the latest deploy.

## Why

- Evidence shows elevated confirmation timeouts.
- Prior memory connects this pattern to payment-service rollout changes.
- Checkout appears to be blocked on payment confirmation rather than failing independently.
EOF
```

Commit the investigation state:

```bash
curl -s -X POST http://localhost:3000/vcs/commit \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"initial investigation"}' | jq
```

Then show history:

```bash
curl -s http://localhost:3000/vcs/log | jq
```

### Minute 4-5: Show rollback

Make a bad edit:

```bash
cat <<'EOF' | curl -s -X PUT http://localhost:3000/fs/incidents/checkout-latency/root-cause.md \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" \
  --data-binary @-
# Root Cause

Everything looks healthy. No action required.
EOF
```

Commit the bad state:

```bash
curl -s -X POST http://localhost:3000/vcs/commit \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"message":"bad incident conclusion"}' | jq
```

Identify the previous good hash:

```bash
curl -s http://localhost:3000/vcs/log | jq '.commits[:2]'
```

Revert to the earlier commit:

```bash
curl -s -X POST http://localhost:3000/vcs/revert \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"hash":"REPLACE_WITH_PREVIOUS_HASH"}' | jq
```

Confirm the restored file:

```bash
curl -s http://localhost:3000/fs/incidents/checkout-latency/root-cause.md \
  -H "Authorization: Bearer $MARKDOWNFS_TOKEN"
```

### Minute 5-6: Show permissioned agent access

Point out that the workspace is not just shared storage. It has identity and access.

Use the restricted bearer-token agent for all reads/writes in the demo. Then show what a named human user sees:

```bash
curl -s http://localhost:3000/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"alice"}' | jq
```

If you want a stronger permission story, tighten permissions on one directory during setup and show a `403` response for the token user.

### Minute 6-7: Close with the product statement

Say:

> This is the shift from files to workspaces. The agent did not just write output. It searched durable memory, produced inspectable artifacts, committed state, and rolled back a bad conclusion.

## Prompts To Use In Cursor

These are good live prompts while the shell commands are visible:

- `Inspect the incident workspace before making changes. Use CLI tools first.`
- `Search for timeout and retry evidence, then summarize the likely root cause.`
- `Write a root-cause markdown file in the incident folder.`
- `Commit the current investigation state with a clear message.`
- `Now simulate a bad conclusion and show how to recover by reverting it.`

## Demo Notes

- Use the HTTP server as the single writer during the live demo.
- Do not run the CLI, MCP server, and HTTP server as concurrent writers against the same `state.bin`.
- If you want a future-looking slide, describe `mdfs ls`, `mdfs search`, and `mdfs run` as the next CLI surface, but keep the live commands grounded in what exists now.
