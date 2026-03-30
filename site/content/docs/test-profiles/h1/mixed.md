---
title: Mixed Workload
---
{{< type-rules production="All endpoint implementations must follow their respective production rules. No endpoint-specific optimizations that would not be used in production." tuned="May optimize each endpoint independently. Pre-computed responses, custom serializers, and non-default configurations allowed." engine="No specific rules." >}}


The Mixed Workload profile measures overall framework performance under a diverse, realistic request mix that exercises multiple code paths simultaneously - baseline request handling, JSON serialization, database queries, file uploads, and gzip compression.

**Connections:** 4,096, 16,384

## How it works

1. The load generator (gcannon) opens 4,096 or 16,384 connections across 64 threads
2. Each connection is assigned a request template from a pool of 10 templates:
   - 3× baseline GET (`GET /baseline11?a=1&b=2`)
   - 2× baseline POST (`POST /baseline11` with body)
   - 1× JSON processing (`GET /json`)
   - 1× SQLite DB query (`GET /db?min=10&max=50`)
   - 1× file upload (`POST /upload` with 1 MB body)
   - 2× gzip compression (`GET /compression` with `Accept-Encoding: gzip`)
3. Each connection sends 5 requests of its assigned type, then disconnects and **reconnects with the next template type** (round-robin rotation)
4. This rotation ensures all frameworks face a roughly even distribution of request types - fast connections cycle through all templates rather than staying on one type

## Expected request/response

Each template type uses the same endpoint format as its standalone profile:

```
GET /baseline11?a=1&b=2 HTTP/1.1
→ 3

GET /json HTTP/1.1
→ {"items": [...], "count": 50}

GET /db?min=10&max=50 HTTP/1.1
→ {"items": [...], "count": N}

POST /upload HTTP/1.1
Content-Length: 1048576
→ 1048576

GET /compression HTTP/1.1
Accept-Encoding: gzip
→ (gzip-compressed JSON)
```

## Template rotation

Without rotation, connections assigned to lightweight endpoints (baseline: ~2 byte response) would complete hundreds of reconnect cycles, while heavy endpoints (compression: ~234 KB response) complete only a few. This would skew the results - frameworks efficient at heavy endpoints would paradoxically show lower total RPS.

With rotation, each connection cycles through all 10 templates on successive reconnects. Fast connections rotate faster, spreading their requests across all endpoint types. The result is a much more balanced distribution across all frameworks.

## What it measures

- **Overall throughput** - how well a framework handles diverse concurrent workloads
- **Code path diversity** - performance when multiple subsystems (routing, JSON, DB, I/O, compression) are active simultaneously
- **Resource contention** - thread pool saturation, memory pressure, and CPU scheduling under mixed load
- **Endpoint balance** - whether a framework handles all request types efficiently or only excels at simple ones

## Weighted scoring

Raw requests per second is not a fair metric for mixed workloads because different request types have vastly different computational costs. A framework that handles more compression or database requests is doing significantly more work per request than one that mostly serves baseline responses.

The mixed score weights each request by its computational cost:

```
weighted = baseline × 0.15 + json × 1 + db × 10 + upload × 10 + compression × 7
score    = (weighted / max(weighted)) × 100
```

| Request type | Weight | Rationale |
|-------------|--------|-----------|
| Baseline | 0.15 | Trivial - parse query params, add integers, return 2 bytes |
| JSON | 1 | Moderate - iterate 50 items, compute fields, serialize ~10 KB |
| DB | 10 | Heavy - SQLite range query, parse tags, build JSON response |
| Upload | 10 | Heavy - receive 1 MB body, return byte count, memory management |
| Compression | 7 | Heavy - gzip-compress ~1 MB response on the fly |

The framework with the highest weighted total scores **100**, others scale proportionally.

## Request mix visualization

The leaderboard displays a **stacked bar** for each framework showing the actual distribution of completed requests by type. Hovering reveals exact counts and percentages. This makes it transparent how each framework distributed its work across endpoint types.

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoints | `/baseline11`, `/json`, `/db`, `/upload`, `/compression` |
| Connections | 4,096, 16,384 |
| Pipeline | 1 |
| Requests per connection | 5 (then reconnect with next template) |
| Duration | 15s |
| Runs | 3 (best taken) |
| Templates | 10 (3 baseline GET, 2 baseline POST, 1 JSON, 1 DB, 1 upload, 2 compression) |
