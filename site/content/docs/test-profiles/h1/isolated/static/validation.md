---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `static` test.

## Content-Type headers

Verifies correct `Content-Type` headers for representative file types:

- `GET /static/reset.css` — expects `Content-Type: text/css`
- `GET /static/app.js` — expects `Content-Type: application/javascript`
- `GET /static/manifest.json` — expects `Content-Type: application/json`

Note: `text/javascript` is accepted as equivalent to `application/javascript` per RFC 9239.

## File size verification

Requests all 20 static files and compares the response size (`Content-Length` or download size) against the actual file size on disk. All 20 files must match exactly:

`reset.css`, `layout.css`, `theme.css`, `components.css`, `utilities.css`, `analytics.js`, `helpers.js`, `app.js`, `vendor.js`, `router.js`, `header.html`, `footer.html`, `regular.woff2`, `bold.woff2`, `logo.svg`, `icon-sprite.svg`, `hero.webp`, `thumb1.webp`, `thumb2.webp`, `manifest.json`

## 404 for nonexistent file

Sends `GET /static/nonexistent.txt` and verifies the server returns **HTTP 404**.
