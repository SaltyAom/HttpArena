---
title: Implementation Guidelines
---
{{< type-rules production="Response compression must use the framework's standard middleware. Pre-compressed files on disk are allowed if the framework documents this as the official/recommended approach (e.g., ASP.NET MapStaticAssets, Nginx gzip_static). Binary formats (webp, woff2) should not be compressed." tuned="May cache compressed and uncompressed versions in memory. Pre-compressed files on disk allowed. Must serve uncompressed when Accept-Encoding: gzip is absent." engine="Pre-compressed files on disk allowed. Must respect Accept-Encoding header presence/absence." >}}

The Assets-4 profile serves a mix of static files and JSON responses, where some requests include `Accept-Encoding: gzip` and others do not. The server must compress text-based responses on-the-fly when the header is present, skip compression for binary formats, and serve uncompressed responses when the header is absent. The server container is constrained to **4 CPUs and 16 GB memory**.

## Compression rules

1. **Text-based files** (CSS, JS, HTML, JSON): must be gzip-compressed when `Accept-Encoding: gzip` is present in the request
2. **Binary files** (webp, woff2): must NOT be compressed even when `Accept-Encoding: gzip` is present — these formats are already compressed
3. **SVG files**: server may choose to compress or not (both are accepted)
4. **No compression header**: when `Accept-Encoding: gzip` is absent, responses must always be uncompressed regardless of content type
5. **Pre-compressed files on disk**: allowed for **production** frameworks if the framework documents this as the official/recommended approach (e.g., ASP.NET `MapStaticAssets`, Nginx `gzip_static`). Always allowed for **tuned** and **engine** types.

## Caching rules

- **Production** frameworks must use standard middleware; in-memory caching of compressed variants is allowed if it's the framework's documented approach
- **Tuned** and **Engine** frameworks may cache both compressed and uncompressed versions in memory, and may use pre-compressed files on disk

## Request mix (20 templates)

| Category | Templates | Accept-Encoding: gzip |
|----------|-----------|-----------------------|
| Text files (JS, CSS, HTML) | 5 | Yes |
| JSON (`/json`) | 1 | Yes |
| Text files (JS, CSS, HTML) | 5 | No |
| JSON (`/json`) | 1 | No |
| Binary (webp, woff2) | 2 | Yes (server must skip) |
| Binary (webp, woff2) | 2 | No |
| SVG | 1 | Yes (either accepted) |
| SVG | 1 | No |
| Manifest JSON + CSS | 2 | No |

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
