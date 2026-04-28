# Checkout Latency Evidence

## Signals

- `checkout_api_latency_p95`: 2.8 s at peak, baseline 480 ms
- `payment_service_timeout_rate`: 7.4%, baseline < 0.5%
- `checkout_retry_rate`: 3.1x baseline
- Regional spread: visible in `us-east-1` and `us-west-2`

## Log Excerpts

```text
ERROR payment confirmation request exceeded timeout budget
ERROR upstream call to payment-service retried 3 times
WARN checkout request waiting on payment confirmation
```

## Relevant Files

- `runbooks/payment-service.md`
- `memory/agents/researcher.md`

## Notes

- Evidence suggests the issue is downstream of checkout and near payment confirmation.
- No signs yet of database saturation on the checkout side.
