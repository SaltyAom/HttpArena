---
title: Implementation Guidelines
---
{{< type-rules production="All endpoint implementations must follow their respective production rules. No endpoint-specific optimizations that would not be used in production." tuned="May optimize each endpoint independently. Pre-computed responses, custom serializers, and non-default configurations allowed." engine="No specific rules." >}}

The API-4 profile runs a lighter workload than [Mixed](../../mixed/implementation) with the server container constrained to **4 CPUs and 16 GB memory**. Only baseline, JSON, static files, and async database endpoints are tested — heavy endpoints (upload, compression, SQLite DB) are excluded. The load generator uses 4 threads and 256 connections.

**Connections:** 128

## How it differs from Mixed

| Parameter | Mixed | API-4 |
|-----------|-------|--------|
| Server CPUs | Unlimited | 4 |
| Server memory | Unlimited | 16 GB |
| Connections | 4,096 | 128 |
| gcannon threads | 64 | 4 |
| Duration | 15s | 15s |
| Request templates | 14 | 8 |
| Requests per connection | 5 | 5 |
| Upload | Yes | No |
| Compression | Yes | No |
| SQLite DB | Yes | No |

## What it measures

- **Resource-constrained performance** - how well a framework utilizes limited CPU and memory
- **Real-world relevance** - closer to a typical production deployment (4-core VM, limited RAM) than the full-hardware tests
- **Efficiency under contention** - thread pool saturation, memory pressure, and GC behavior when resources are scarce
- **Scaling characteristics** - whether a framework's performance degrades gracefully with fewer resources

## Request mix

- 3x baseline GET (`GET /baseline11?a=1&b=2`)
- 3x JSON processing (`GET /json`)
- 2x async DB query (`GET /async-db?min=10&max=50`)

## Docker constraints

The server container is started with:

```
--cpus=4 --memory=16g --memory-swap=16g
```

If a framework exceeds the 16 GB memory limit, the container will be OOM-killed by Docker.

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoints | `/baseline11`, `/json`, `/async-db` |
| Connections | 256 |
| Pipeline | 1 |
| Requests per connection | 5 (then reconnect with next template) |
| Duration | 15s |
| Runs | 3 (best taken) |
| Templates | 8 (3 baseline GET, 3 JSON, 2 async-db) |
| Server CPU limit | 4 |
| Server memory limit | 16 GB |
| gcannon threads | 4 |
