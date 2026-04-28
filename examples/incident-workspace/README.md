# Incident Workspace Example

This example provides seed markdown content for the agent workspace demo.

## Workspace Tree

```text
examples/incident-workspace/
├── incidents/
│   └── checkout-latency/
│       ├── evidence.md
│       ├── hypotheses.md
│       └── timeline.md
├── memory/
│   └── agents/
│       └── researcher.md
└── runbooks/
    └── payment-service.md
```

## What This Is For

Use these files to demo:

- agent memory
- cross-file search with `grep`-style or MCP search
- human-reviewable workspace state
- commit and revert workflows

## Suggested Demo Flow

1. Load or recreate these files in a fresh `markdownfs` data directory.
2. Ask an agent to inspect the runbook and memory before writing new findings.
3. Have the agent add `root-cause.md` and `remediation.md`.
4. Commit the good state.
5. Introduce a bad edit and revert it.
