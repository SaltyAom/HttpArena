---
title: Implementation Guidelines
---
{{< type-rules production="Response compression must use the framework's standard middleware. Pre-compressed files on disk are allowed if the framework documents this as the official/recommended approach (e.g., ASP.NET MapStaticAssets, Nginx gzip_static). Binary formats (webp, woff2) should not be compressed." tuned="May cache compressed and uncompressed versions in memory. Pre-compressed files on disk allowed. Must serve uncompressed when Accept-Encoding: gzip is absent." engine="Pre-compressed files on disk allowed. Must respect Accept-Encoding header presence/absence." >}}

The Assets-16 profile is identical to [Assets-4](../../assets-4/implementation) but with the server constrained to **16 CPUs and 32 GB memory** instead of 4 CPUs and 16 GB. This measures how well the framework scales asset serving with more available resources.

All [compression rules](../../assets-4/implementation/#compression-rules) and [caching rules](../../assets-4/implementation/#caching-rules) from Assets-4 apply.

## Docker constraints

The server container is started with:

```
--cpuset-cpus=0-7,64-71 --memory=32g --memory-swap=32g
```

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoints | `/static/*`, `/json` |
| Connections | 1024 |
| Pipeline | 1 |
| Requests per connection | 10 (then reconnect with next template) |
| Duration | 15s |
| Runs | 3 (best taken) |
| Templates | 20 |
| Server CPU limit | 16 |
| Server memory limit | 32 GB |
