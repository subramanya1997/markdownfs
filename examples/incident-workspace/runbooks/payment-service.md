# Payment Service Runbook

## Purpose

Use this runbook when checkout degradation appears tied to payment confirmation or upstream payment-service timeouts.

## Known Failure Pattern

If checkout latency spikes immediately after a payment-service deploy, inspect timeout and retry changes first.

## Triage Steps

1. Check whether a payment-service deploy occurred in the last 30 minutes.
2. Compare timeout budget and retry configuration with the previous release.
3. Check confirmation error rate and queue depth.
4. If customer impact is high, prepare rollback while root cause is still under investigation.

## Prior Learning

On 2026-03-15, checkout latency increased after payment-service raised a confirmation timeout and retried upstream requests too aggressively. Rolling back reduced p95 within minutes.
