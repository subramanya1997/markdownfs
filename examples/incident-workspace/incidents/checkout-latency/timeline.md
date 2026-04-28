# Checkout Latency Timeline

## Summary

This incident workspace tracks the April 24 checkout latency investigation.

## Timeline

- 13:41 UTC: Checkout p95 rose from 480 ms to 2.8 s.
- 13:43 UTC: Alert fired for `checkout-latency-p95`.
- 13:45 UTC: Support reported intermittent payment confirmation delays.
- 13:48 UTC: On-call confirmed elevated retry volume between checkout and payment-service.
- 13:52 UTC: Latest payment-service rollout identified as the most recent production change.
- 13:57 UTC: Search started across prior incident notes and runbooks.

## Open Questions

- Did the payment-service rollout increase upstream timeout or retry pressure?
- Are errors concentrated in one region or all regions?
- Did a dependency degrade, or is this an application-level regression?
