---
title: Composite Score
---

The composite score combines results from multiple test profiles into a single number that reflects overall framework performance. It uses a normalized arithmetic mean with optional resource efficiency factors.

## How it works

### Step 1: Average RPS per profile

For each framework and profile, compute the average RPS across all connection counts. This rewards frameworks that scale well across concurrency levels rather than just their peak.

### Step 2: Normalize per profile

For each profile, normalize against the best-performing realistic framework:

```
rpsScore = (framework_avg_rps / best_avg_rps) × 100
```

This produces a 0–100 value where the top framework scores 100.

### Step 3: Arithmetic mean

The final composite score is the arithmetic mean of per-profile scores across all **scored** profiles:

```
composite = sum(scored_profile_scores) / number_of_scored_profiles
```

Frameworks that don't participate in a scored profile receive 0 for that profile, which lowers their composite proportionally.

## Scored vs reference-only profiles

Not all profiles count toward the composite score. Profiles marked as **scored** contribute to the composite. Reference-only profiles (marked with **\***) are displayed for comparison but do not affect the ranking.

| Profile | Protocol | Scored | Workload |
|---|---|---|---|
| Baseline | HTTP/1.1 | Yes | Mixed GET/POST with query parsing |
| Pipelined | HTTP/1.1 | No (*) | 16 requests batched per connection |
| Short-lived | HTTP/1.1 | Yes | Connections closed after 10 requests |
| JSON | HTTP/1.1 | Yes | Dataset processing and serialization |
| Baseline | HTTP/2 | Yes | Query parsing over TLS with multiplexed streams |
| Static | HTTP/2 | Yes | 20 static files served over TLS with multiplexed streams |
| Baseline | HTTP/3 | Yes | Query parsing over QUIC (UDP) with TLS 1.3 |
| Static | HTTP/3 | Yes | 20 static files served over QUIC (UDP) with TLS 1.3 |

Pipelined is reference-only because not all frameworks support HTTP pipelining.

## Resource efficiency factors

Two optional toggles allow factoring in resource efficiency:

- **CPU efficiency** (1× weight) — measures requests per CPU percent (`rps / cpu%`)
- **Memory efficiency** (0.5× weight) — measures requests per megabyte (`rps / MB`)

These use **efficiency ratios**, not absolute resource usage. A framework that achieves high throughput with low resource consumption scores well. A framework that uses little CPU simply because it is slow does **not** benefit.

### How resource scores are computed

For each profile, compute the efficiency ratio for every framework:

```
cpuEfficiency = rps / cpu%
memEfficiency = rps / memoryMB
```

Normalize against the best efficiency in that profile:

```
cpuScore = (framework_cpuEff / best_cpuEff) × 100
memScore = (framework_memEff / best_memEff) × 100
```

Combine with the RPS score using configured weights:

```
profileScore = (rpsScore × 1 + cpuScore × wCpu + memScore × wMem) / totalWeight
```

Where `totalWeight = 1 + wCpu + wMem`. With both factors active, `totalWeight = 2.5`, so RPS counts 40%, CPU efficiency 40%, and memory efficiency 20%.

### Example

| Framework | RPS | CPU% | Mem (MB) | rps/cpu | rps/MB |
|---|---|---|---|---|---|
| A | 500,000 | 800% | 50 | 625 | 10,000 |
| B | 100,000 | 100% | 20 | 1,000 | 5,000 |

- RPS scores: A = 100, B = 20
- CPU efficiency scores: A = 62.5, B = 100 (B gets more throughput per CPU unit)
- Memory efficiency scores: A = 100, B = 50

With both factors on (`totalWeight = 2.5`):
- A: `(100 + 62.5 + 50) / 2.5 = 85.0`
- B: `(20 + 100 + 25) / 2.5 = 58.0`

Framework A still leads because its raw throughput advantage outweighs B's CPU efficiency, but the gap narrows from 5× to 1.5×.

## Stripped frameworks

Frameworks with type `stripped` are excluded from the composite ranking and from the normalization pool. They can still be compared in individual test profiles. Only `realistic` frameworks — those using standard, production-grade HTTP libraries — are scored here.

## Why this approach

- **Arithmetic mean** — straightforward averaging that doesn't over-penalize a single weak profile
- **Normalization** — each profile contributes equally regardless of absolute RPS scale (baseline at 1M vs JSON at 200K)
- **Efficiency ratios** — resource factors measure throughput per unit of resource, preventing slow frameworks from gaming the score by using fewer absolute resources
- **Average across connections** — each framework is scored on its average RPS across all connection counts, rewarding consistent scaling
