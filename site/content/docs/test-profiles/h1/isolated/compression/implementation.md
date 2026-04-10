---
title: Implementation Guidelines
---
{{< type-rules production="Must use the framework's built-in compression (gzip or brotli). No pre-compressed caches, no custom compression libraries. The dataset may be loaded and processed to JSON at startup, but compression must happen per request." tuned="May use alternative compression libraries or hardware-accelerated compression. The dataset may be loaded and processed to JSON at startup, but compression must happen per request. No pre-compressed caches." engine="The dataset may be loaded and processed to JSON at startup, but compression must happen per request. No pre-compressed caches." >}}


The Compression profile measures the throughput cost of real-time compression on a large JSON response. The framework chooses its compression algorithm (gzip or brotli) and compression level. Only frameworks with built-in compression support are eligible.

**Connections:** 512, 4,096

## How it works

1. At startup, the server loads `/data/dataset-large.json` - a 6,000-item dataset
2. Processes each item (computes a `total` field), serializes to JSON (~1 MB response)
3. On each `GET /compression` request with `Accept-Encoding: gzip, br`, the server compresses the response using its preferred algorithm and level
4. Returns the compressed JSON with the appropriate `Content-Encoding` header (`gzip` or `br`)

## What it measures

- **Compression throughput** - CPU cost of compressing a 1 MB response per request
- **Compression quality vs. speed tradeoff** - higher compression levels produce smaller responses but cost more CPU per request
- **Compression implementation quality** - framework/server compression performance differences

## Eligibility

A framework must have **built-in compression** — native gzip or brotli support. Custom compression implementations or third-party compression libraries are not permitted for production entries.

Frameworks excluded due to no built-in compression: hyper, Node.js, Express, Deno, Flask.

## Algorithm and level

The framework picks whichever algorithm (gzip or brotli) and compression level it prefers. Both algorithms are valid, and any compression level is allowed. All requests include `Accept-Encoding: gzip, br`, so the framework is free to choose either.

This is a deliberate design choice — the bandwidth-adjusted scoring formula (see [Scoring](#scoring)) heavily rewards smaller responses, so frameworks must balance compression speed against compression ratio. A fast but weak compression level produces high RPS but gets penalized for larger responses. A slow but strong level produces fewer requests but each one is worth more in the adjusted score.

## Expected response

```
GET /compression HTTP/1.1
Accept-Encoding: gzip, br
```

```
HTTP/1.1 200 OK
Content-Type: application/json
Content-Encoding: gzip

(gzip-compressed ~1 MB JSON)
```

Or with brotli:

```
HTTP/1.1 200 OK
Content-Type: application/json
Content-Encoding: br

(brotli-compressed ~1 MB JSON)
```

The response JSON follows the same structure as the [JSON Processing](../../json-processing) endpoint: `{"items": [...], "count": N}`. The `count` field must be dynamically computed from the number of items (e.g. `len(items)`, `items.length`, `items.size()`), not hardcoded to `6000`.

## Scoring

Raw requests per second alone is not a fair metric for compression benchmarks. A framework that compresses less aggressively produces larger responses, reducing CPU cost per request and inflating its RPS - but at the expense of bandwidth efficiency.

To account for this, the compression score adjusts RPS by the **squared bandwidth penalty** — frameworks with larger responses are penalized quadratically:

```
bw_per_req   = bandwidth / rps
ratio        = min(bw_per_req) / bw_per_req
adjusted_rps = rps × ratio²
score        = (adjusted_rps / max(adjusted_rps)) × 100
```

- `bw_per_req` - average bytes transferred per request (higher = worse compression)
- `min(bw_per_req)` - the smallest value across all frameworks (best compressor)
- `ratio²` - **squared** penalty ≤ 1.0; the best compressor gets no penalty, others are reduced quadratically. A framework with 2× the response size of the best compressor loses 75% of its score (not 50% as a linear penalty would give)
- The framework with the highest adjusted RPS scores **100**, others scale down

The squared penalty heavily rewards better compression. This means frameworks must carefully balance their compression level — using the fastest level maximizes RPS but the larger responses get severely penalized. Using a stronger compression level costs CPU but the smaller responses are worth significantly more.

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | `GET /compression` |
| Connections | 512, 4,096 |
| Pipeline | 1 |
| Duration | 5s |
| Runs | 3 (best taken) |
| Compression | Any level (framework's choice) |
| Dataset | 6,000 items, mounted at `/data/dataset-large.json` |
| Uncompressed size | ~1 MB |
