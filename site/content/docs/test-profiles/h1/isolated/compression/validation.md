---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `compression` test.

## Content-Encoding header

Sends `GET /compression` with `Accept-Encoding: gzip, br` and verifies the response includes a `Content-Encoding` header with either `gzip` or `br`. The framework chooses which algorithm to use.

## Response content

Sends `GET /compression` with `Accept-Encoding: gzip, br`, decompresses the response, and parses the JSON. Verifies:

- The response contains exactly **6,000 items**
- Every item has a `total` field

## Compressed size

Sends `GET /compression` with `Accept-Encoding: gzip, br` and checks the raw response size. The compressed response must be **under 500 KB** (the uncompressed JSON is ~1 MB, so effective compression should reduce it significantly).

## Per-request compression

Sends `GET /compression` **without** `Accept-Encoding` and verifies the response does **not** include a `Content-Encoding` header. If the server returns compressed data without the client requesting it, this indicates a pre-compressed cache rather than per-request compression, which is not allowed for any framework type.
