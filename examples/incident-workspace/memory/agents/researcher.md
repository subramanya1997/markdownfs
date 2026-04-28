# Research Agent Memory

## Operating Notes

- Search runbooks and prior memory before drafting a new incident summary.
- Prefer writing findings as markdown with explicit headings and bullets.
- When evidence is weak, label claims as hypotheses instead of conclusions.

## Recalled Patterns

- Checkout incidents are often secondary effects, not root causes.
- Payment-service deploys have previously introduced timeout regressions.
- Support tickets mentioning delayed confirmation usually correlate with upstream retries.

## Useful Queries

- `ERROR`
- `timeout`
- `retry`
- `root cause`
