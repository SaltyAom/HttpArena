---
title: Noisy (Resilience)
---

The Noisy profile measures how well a framework maintains throughput when valid requests are interleaved with malformed, invalid, and adversarial traffic. All frameworks that support the baseline test are eligible.

**Connections:** 512, 4,096, 16,384

## How it works

1. The load generator sends requests in round-robin from a mixed pool of 5 raw request files
2. **2 valid requests** — standard `GET` and `POST` to `/baseline11` (same as baseline)
3. **3 noise requests** — each designed to trigger a different error-handling path:

| Noise type | What it sends |
|---|---|
| Bad path | `GET /this/path/does/not/exist` — nonexistent route |
| Bad Content-Length | `Content-Length: 999` with no body — forces timeout/error handling |
| Binary noise | 256 bytes of random binary data — not valid HTTP at all |

4. Only **2xx responses** (from valid requests) count toward RPS — all error responses are ignored in scoring

## What it measures

- **Error-handling overhead** — how much CPU the framework spends parsing and rejecting bad requests
- **Connection resilience** — whether malformed requests corrupt connection state or cause cascading failures
- **Throughput under adversarial load** — the practical impact of garbage traffic on valid request processing
- **Parser robustness** — handling of edge cases like binary data, oversized headers, and protocol violations

## Scoring

Standard RPS scoring — only 2xx responses count. The framework with the highest valid RPS scores 100, others scale proportionally. Since 60% of requests are noise (3 out of 5), the theoretical maximum RPS is about 40% of the baseline test.

Frameworks that reject bad requests quickly and efficiently with minimal overhead will score closer to their baseline throughput ratio. Frameworks where error handling is expensive or where bad requests disrupt connection state will see a larger gap.

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | `GET/POST /baseline11` + noise |
| Connections | 512, 4,096, 16,384 |
| Pipeline | 1 |
| Duration | 5s |
| Runs | 3 (best taken) |
| Valid request ratio | 2/5 (40%) |
| Noise types | 3 (bad path, bad content-length, binary) |
