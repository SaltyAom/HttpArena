---
title: Static Files (HTTP/3)
---
{{< type-rules production="Must load files from disk on every request. No in-memory caching, no memory-mapped files, no pre-loaded file buffers." tuned="May cache files in memory at startup, use memory-mapped files, pre-rendered response headers, or any caching strategy." engine="No specific rules. Ranked separately from frameworks." >}}


The HTTP/3 Static Files profile serves 20 static files of various types over QUIC, simulating a browser loading page assets over HTTP/3.

**Connections:** 64, 512

## How it works

1. The load generator ([oha](/docs/load-generators)) connects over HTTP/3 (QUIC) on port 8443
2. Cycles through 20 URIs from `requests/static-h2-uris.txt` (same file set as the HTTP/2 static test)
3. Each request fetches a different static file - CSS, JavaScript, HTML, fonts, SVGs, WebP images, and JSON
4. The server returns pre-loaded file contents with the correct `Content-Type`

## What it measures

- **HTTP/3 static asset serving** - mixed content types and sizes over QUIC
- **QUIC multiplexing** - how well the framework handles varied concurrent requests
- **Content-Type handling** - correct MIME type mapping across file types

## Static files

20 files (~360 KB total) pre-loaded into memory at startup:

| Type | Files | Examples |
|------|-------|---------|
| CSS | 5 | `reset.css`, `layout.css`, `theme.css` |
| JavaScript | 5 | `app.js`, `vendor.js`, `router.js` |
| HTML | 2 | `header.html`, `footer.html` |
| Fonts | 2 | `regular.woff2`, `bold.woff2` |
| SVG | 2 | `logo.svg`, `icon-sprite.svg` |
| WebP | 3 | `hero.webp`, `thumb1.webp`, `thumb2.webp` |
| JSON | 1 | `manifest.json` |

## Expected request/response

```
GET /static/logo.svg HTTP/3
```

```
HTTP/3 200 OK
Content-Type: image/svg+xml

(file contents)
```

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | 20 URIs under `/static/*` |
| Connections | 64, 512 |
| Parallelism | 128 per connection |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | oha |
| Port | 8443 (TLS + QUIC) |
