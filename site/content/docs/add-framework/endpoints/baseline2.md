---
title: HTTP/2 Baseline
---

Required for the `baseline-h2` profile. Served over **HTTPS on port 8443**.

## `GET /baseline2?a=N&b=N`

Same logic as `/baseline11` — parse query parameters and return their sum. Served over HTTP/2 with TLS.

Read the TLS certificate and key from `/certs/server.crt` and `/certs/server.key` (mounted read-only by the benchmark runner).
