---
title: Compression
---

Required for the `compression` and `mixed` profiles.

## `GET /compression`

Load `/data/dataset-large.json` at startup (6000 items). Compute `total` for each item (same as `/json`), and return the response with `Content-Type: application/json`.

The server **must** support gzip compression — when the client sends `Accept-Encoding: gzip`, the response should be gzip-compressed. Only frameworks with built-in gzip support are eligible.
