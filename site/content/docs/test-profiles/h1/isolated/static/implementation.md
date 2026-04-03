---
title: Implementation Guidelines
---
{{< type-rules production="Must load files from disk on every request. No in-memory caching, no memory-mapped files, no pre-loaded file buffers." tuned="May cache files in memory at startup, use memory-mapped files, pre-rendered response headers, or any caching strategy." engine="No specific rules." >}}


Serves 20 static files of various types and sizes over HTTP/1.1, simulating a realistic page load with diverse file types and sizes.

**Connections:** 4,096, 16,384

## Workload

The load generator ([gcannon](https://github.com/MDA2AV/gcannon)) requests 20 static files in a round-robin pattern using raw HTTP/1.1 request templates:

- **CSS** (5 files, 1.2-12 KB): `reset.css`, `layout.css`, `theme.css`, `components.css`, `utilities.css`
- **JavaScript** (5 files, 3.2-35 KB): `analytics.js`, `helpers.js`, `app.js`, `vendor.js`, `router.js`
- **HTML** (2 files, 1.1-1.5 KB): `header.html`, `footer.html`
- **Fonts** (2 files, 32-38 KB): `regular.woff2`, `bold.woff2`
- **SVG** (2 files, 4.5-8 KB): `logo.svg`, `icon-sprite.svg`
- **Images** (3 files, 18-85 KB): `hero.webp`, `thumb1.webp`, `thumb2.webp`
- **JSON** (1 file, 0.9 KB): `manifest.json`

Total payload: ~325 KB across 20 files.

## What it measures

- Static file serving throughput over HTTP/1.1
- Content-Type handling for different file types
- Memory-mapped or pre-loaded file serving performance
- Response efficiency with varied payload sizes

## Expected request/response

```
GET /static/reset.css HTTP/1.1
Host: localhost:8080
```

```
HTTP/1.1 200 OK
Content-Type: text/css

(file contents)
```

```
GET /static/app.js HTTP/1.1
Host: localhost:8080
```

```
HTTP/1.1 200 OK
Content-Type: application/javascript

(file contents)
```

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | 20 URIs under `/static/*` |
| Connections | 4,096, 16,384 |
| Pipeline | 1 |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | gcannon with `--raw` (multi-template) |
