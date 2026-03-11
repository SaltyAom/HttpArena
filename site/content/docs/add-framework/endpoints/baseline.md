---
title: Baseline
---

Required for the `baseline`, `limited-conn`, `noisy`, and `mixed` profiles.

## `GET/POST /baseline11?a=N&b=N`

Parse query parameters `a` and `b`, compute their sum, and return it as plain text.

For POST requests, the body contains an additional integer to add. The server must accept both `Content-Length` and `chunked Transfer-Encoding` bodies.

```
GET /baseline11?a=13&b=42 → "55"
POST /baseline11?a=13&b=42 (body: "20") → "75"
```

Response must have `Content-Type: text/plain`.
