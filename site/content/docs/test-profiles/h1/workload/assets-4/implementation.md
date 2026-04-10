---
title: Implementation Guidelines
---
{{< type-rules production="Static file compression must use the framework's standard middleware or built-in static file handler — no handmade compression code. Pre-compressed files on disk are allowed if the framework documents this as the official/recommended approach (e.g., ASP.NET MapStaticAssets, Nginx gzip_static). Binary formats (webp, woff2) should not be compressed." tuned="May cache compressed and uncompressed versions in memory. Pre-compressed files on disk allowed. Free to use any compression approach." engine="Pre-compressed files on disk allowed. Must respect Accept-Encoding header presence/absence." >}}

The Assets-4 profile serves a mix of static files and JSON responses. All static file requests include `Accept-Encoding: br;q=1, gzip;q=0.8`. JSON requests (`/json`) do **not** include a compression header — this endpoint tests pure serialization performance. The server container is constrained to **4 CPUs and 16 GB memory**.

## Compression rules

1. **Static files**: all static file requests include `Accept-Encoding: br;q=1, gzip;q=0.8`. Compression is **optional but recommended** for text-based files — frameworks that compress will benefit from reduced I/O
2. **Binary files** (webp, woff2): already compressed formats — servers should skip compression for these
3. **JSON endpoint (`/json`)**: no compression header is sent. The response must be serialized on every request — this tests pure JSON processing throughput
4. **Pre-compressed files on disk** (`.gz`, `.br`): available for all static files. Frameworks that support serving pre-compressed files can use them for zero CPU overhead compression
5. **Production rule**: compression must use the framework's standard middleware or built-in static file handler. No handmade compression code
6. **Tuned rule**: free to use any compression approach

## Caching rules

- **Production** frameworks must use standard middleware; in-memory caching of compressed variants is allowed if it's the framework's documented approach
- **Tuned** and **Engine** frameworks may cache both compressed and uncompressed versions in memory, and may use pre-compressed files on disk

## Request mix (20 templates)

| Category | Templates | Accept-Encoding |
|----------|-----------|-----------------|
| Static text (JS, CSS, HTML) | 10 | `br;q=1, gzip;q=0.8` |
| Static binary (webp, woff2) | 4 | `br;q=1, gzip;q=0.8` (server should skip) |
| Static SVG + logo | 2 | `br;q=1, gzip;q=0.8` |
| Static manifest + CSS | 2 | `br;q=1, gzip;q=0.8` |
| JSON (`/json`) | 2 | None |

## Docker constraints

The server container is started with:

```
--cpuset-cpus=0-3 --memory=16g --memory-swap=16g
```

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoints | `/static/*`, `/json` |
| Connections | 256 |
| Pipeline | 1 |
| Requests per connection | 10 (then reconnect with next template) |
| Duration | 15s |
| Runs | 3 (best taken) |
| Templates | 20 |
| Server CPU limit | 4 |
| Server memory limit | 16 GB |
