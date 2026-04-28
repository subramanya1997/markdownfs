# Demo Readiness

This guide decides which product gaps matter before the first public agent-workspace demo.

## Recommendation

Do **not** try to ship the full execution-layer roadmap before the first demo.

The first demo should be about:

- persistent workspace state
- shell-friendly access through HTTP plus CLI tools
- search across markdown memory
- commits and revert
- permission-aware access

That is already a strong story.

## Priority Decisions

### Ship before demo

#### 1. CLI wrapper

This is the highest-value addition if there is time for code work before a polished demo.

Why:

- It matches how agents naturally operate.
- It makes the product feel like a real workspace tool instead of a collection of endpoints.
- It simplifies the live story from `curl` calls to `mdfs ls`, `mdfs grep`, `mdfs commit`.

#### 2. Better status/diff visibility

Some way to answer “what changed?” improves trust immediately.

Minimum acceptable version:

- a `status`-style command or endpoint that summarizes current workspace state
- a simple `diff` endpoint or rendered comparison against the last commit

Why:

- It makes rollback more compelling.
- It helps humans inspect agent work before accepting it.

### Nice to have before demo

#### 3. Agent-scoped MCP auth

This matters because the current MCP server runs as `root`.

Why it is not mandatory for the first demo:

- The first demo can safely use HTTP as the single writer and use bearer tokens there.
- The product story still works if MCP is presented as an early integration surface.

Why it should follow soon after:

- Root-scoped MCP weakens the control-plane narrative.
- Scoped identity is important for enterprise credibility.

### Do not block the first demo on these

#### 4. Run records

Run records are valuable, but they are phase-two material.

Why they can wait:

- The first demo already proves the workspace concept without a job model.
- The product does not currently execute arbitrary shell commands inside the workspace.

#### 5. Sandboxed execution

Do not block the first demo on a general job runner.

Why:

- It is a larger product and security surface.
- It changes the claim from “workspace layer” to “execution platform.”
- It introduces policy, output capture, scheduling, and isolation concerns.

## Recommended Demo Scope

### Demo now

- HTTP server
- bearer-token agent access
- shell commands like `curl` and `jq`
- search, write, commit, log, revert
- seed incident workspace

### Demo soon after

- `mdfs` CLI wrapper
- status/diff surface
- semantic search over indexed markdown

### Demo later

- run records
- sandboxed execution
- real cloud-hosted workspace gateway

## Short Version

If only two things get implemented before the polished demo, they should be:

1. `mdfs` CLI wrapper
2. status/diff visibility

Everything else can follow without weakening the first positioning story.
