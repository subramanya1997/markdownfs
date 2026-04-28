# Agent Workspace Positioning

This guide defines how to position `markdownfs` as an agent-facing product.

## One-Line Positioning

`markdownfs` is a versioned markdown workspace for AI agents: persistent memory, search, permissions, commits, and rollback in one shared surface.

## Category

Use **agent workspace** as the primary category.

Avoid calling it a general cloud filesystem today. The current product is strongest when described as:

- a persistent workspace for agent-produced knowledge
- a control layer for inspectable agent collaboration
- a markdown-native memory surface with version history

## Core Message

> AWS S3 Files gives applications a filesystem. `markdownfs` gives agents a workspace: memory, permissions, search, commits, rollback, and auditability.

## What Problem It Solves

AI agents are good at generating output, but they are still weak at:

- keeping durable working memory between runs
- leaving behind inspectable artifacts instead of opaque chat transcripts
- collaborating safely with humans and other agents
- recovering cleanly from bad changes

`markdownfs` solves those problems with a markdown-only workspace that agents can access through CLI, HTTP, and MCP.

## Narrative Pillars

### Persistent agent memory

Agents can store notes, plans, decisions, status, and evidence in durable markdown files instead of re-deriving everything from prompts.

### Inspectable work

Humans can browse files, search across a workspace, review history, and verify what happened after an agent finishes.

### Reversible collaboration

Built-in commits and revert make it possible to checkpoint good states and recover quickly from bad edits.

### Scoped access

Users, groups, and bearer-token agents make access control part of the workspace model instead of an afterthought.

### Multi-surface access

The same workspace can be used through:

- the `markdownfs` CLI/REPL
- the `markdownfs-server` HTTP API
- the `markdownfs-mcp` MCP server

## Product Boundary

Be explicit about what `markdownfs` is and is not.

### It is today

- a markdown-native virtual filesystem
- a persistent agent workspace
- a search and versioning layer
- a permissioned collaboration surface

### It is not yet

- a sandboxed shell execution platform
- a generic POSIX cloud filesystem
- a replacement for block storage, EFS, or S3 itself

## Positioning Against AWS S3 Files

Use AWS S3 Files as category validation, not as the main antagonist.

### AWS S3 Files is best at

- making S3 look like a filesystem inside AWS
- integrating with AWS compute, IAM, CloudWatch, and CloudTrail
- serving infrastructure teams that want familiar storage semantics

### `markdownfs` should compete on

- inspectable agent memory
- human-reviewable workspace state
- commits and rollback for agent changes
- permissions and multi-actor collaboration
- AI-friendly surfaces like MCP and workflow-oriented HTTP APIs

The story is not “we are a better S3 mount.” The story is “filesystem semantics are table stakes; agent workspace control is the product.”

## Recommended Messaging

### Homepage / README short form

`markdownfs` is a versioned markdown workspace for AI agents. It gives agents durable memory, search, permissions, commits, rollback, HTTP APIs, and MCP tools in one shared surface.

### Demo intro

Agents do not just need files. They need a workspace they can search, update, commit, review, and recover.

### Product deck headline

From transient chat output to durable agent workspaces.

## Language To Avoid

Avoid these claims unless the product changes:

- “Runs arbitrary CLI tools today”
- “General cloud execution layer”
- “Drop-in replacement for S3 Files”
- “Full OS mount for any program”

## Near-Term Evolution

The natural product progression is:

1. Agent workspace
2. Workspace plus run records and provenance
3. Workspace plus sandboxed execution

That progression is credible, easy to demo, and aligned with the current repo capabilities.
