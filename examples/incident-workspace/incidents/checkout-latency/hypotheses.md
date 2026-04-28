# Checkout Latency Hypotheses

## Ranked Hypotheses

### 1. Payment-service timeout regression

Latest rollout likely changed timeout handling or retry behavior, causing checkout to block on confirmation.

### 2. Partial regional dependency issue

A dependency behind payment-service may be degraded in multiple regions, increasing retries.

### 3. Queue depth amplification

Retries may have increased queue depth enough to create user-visible latency even after the initial trigger.

## TODO

- Confirm what changed in the latest payment-service rollout.
- Compare timeout and retry settings before and after deploy.
- Draft mitigation plan if rollback is the safest first response.
