---
title: Static Files
---

Required for the `static-h2` and `static-h3` profiles. Served over **HTTPS on port 8443**.

## `GET /static/{filename}`

Serve 20 pre-loaded static files from `/data/static/`. Files should be loaded into memory at startup and served with the correct `Content-Type`:

| Extension | Content-Type |
|-----------|-------------|
| `.css` | `text/css` |
| `.js` | `application/javascript` |
| `.html` | `text/html` |
| `.woff2` | `font/woff2` |
| `.svg` | `image/svg+xml` |
| `.webp` | `image/webp` |
| `.json` | `application/json` |

Return `404` for missing files.
