---
title: Static Files (HTTP/2)
---
{{< type-rules production="Must load files from disk on every request. No in-memory caching, no memory-mapped files, no pre-loaded file buffers." tuned="May cache files in memory at startup, use memory-mapped files, pre-rendered response headers, or any caching strategy." engine="No specific rules. Ranked separately from frameworks." >}}


Serves 20 static files of various types and sizes over HTTP/2 with TLS, simulating a realistic browser page load with multiplexed streams.

**Connections:** 64, 256, 1,024
**Concurrent streams per connection:** 100

## Workload

The load generator ([h2load](https://nghttp2.org/documentation/h2load-howto.html)) requests 20 static files in a round-robin pattern across multiplexed streams:

- **CSS** (5 files, 1.2–12 KB): `reset.css`, `layout.css`, `theme.css`, `components.css`, `utilities.css`
- **JavaScript** (4 files, 3.2–35 KB): `analytics.js`, `helpers.js`, `app.js`, `vendor.js`, `router.js`
- **HTML** (2 files, 1.1–1.5 KB): `header.html`, `footer.html`
- **Fonts** (2 files, 32–38 KB): `regular.woff2`, `bold.woff2`
- **SVG** (2 files, 4.5–8 KB): `logo.svg`, `icon-sprite.svg`
- **Images** (3 files, 18–85 KB): `hero.webp`, `thumb1.webp`, `thumb2.webp`
- **JSON** (1 file, 0.9 KB): `manifest.json`

Total payload: ~325 KB across 20 files.

## What it measures

- Static file serving throughput over HTTP/2
- HTTP/2 multiplexing efficiency with varied response sizes
- Content-Type handling for different file types
- Memory-mapped or pre-loaded file serving performance
- TLS overhead with realistic mixed payloads

## Expected request/response

```
GET /static/reset.css HTTP/2
```

```
HTTP/2 200 OK
Content-Type: text/css

(file contents)
```

```
GET /static/app.js HTTP/2
```

```
HTTP/2 200 OK
Content-Type: application/javascript

(file contents)
```

## How it differs from baseline-h2

| | Baseline (HTTP/2) | Static Files (HTTP/2) |
|---|---|---|
| Endpoint | Single `GET /baseline2` | 20 different `/static/*` URIs |
| Response size | ~2 bytes | 0.9–85 KB (varied) |
| Content types | `text/plain` | CSS, JS, HTML, fonts, SVG, WebP, JSON |
| h2load mode | Single URI | Multi-URI (`-i` flag, round-robin) |

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | 20 URIs under `/static/*` |
| Connections | 64, 256, 1,024 |
| Streams per connection | 100 (`-m 100`) |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | h2load with `-i` (multi-URI) |
