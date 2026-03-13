---
title: Baseline
---

The primary throughput benchmark. Each connection sends one request at a time over persistent keep-alive connections with no CPU restrictions.

**Connections:** 512, 4,096, 16,384

## Workload

A mix of three request types, rotated across connections:

- `GET /baseline11?a=13&b=42` — query parameter parsing, response: sum of values
- `POST /baseline11?a=13&b=42` with Content-Length body — query params + body parsing
- `POST /baseline11?a=13&b=42` with chunked Transfer-Encoding body — chunked decoding

This exercises the full HTTP handling path: request line parsing, header parsing, query string extraction, body reading (both Content-Length and chunked), integer arithmetic, and response serialization.

## What it measures

- Raw request throughput under ideal conditions
- Full HTTP parsing pipeline performance
- Keep-alive connection handling efficiency
- How frameworks scale with increasing connection counts

