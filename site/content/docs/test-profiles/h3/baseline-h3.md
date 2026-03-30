---
title: Baseline (HTTP/3)
---
{{< type-rules production="Must use the framework native QUIC/HTTP3 support with default configuration." tuned="May tune QUIC parameters, congestion control, and UDP buffer sizes." engine="No specific rules. Ranked separately from frameworks." >}}


The HTTP/3 Baseline profile tests raw throughput over QUIC, the UDP-based transport protocol that powers HTTP/3.

**Connections:** 64, 512

## How it works

1. The load generator ([oha](/docs/load-generators)) connects to the server over HTTP/3 (QUIC) on port 8443
2. Sends `GET /baseline2?a=1&b=1` requests with 128 parallel requests per connection
3. The server parses query parameters and returns the sum

## What it measures

- **QUIC transport performance** - UDP-based connection handling
- **HTTP/3 framing overhead** - compared to HTTP/1.1 and HTTP/2
- **TLS 1.3 integration** - QUIC mandates encryption
- **Framework QUIC implementation quality** - varies significantly across frameworks

## Expected request/response

```
GET /baseline2?a=1&b=1 HTTP/3
```

```
HTTP/3 200 OK
Content-Type: text/plain

2
```

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | `GET /baseline2?a=1&b=1` |
| Connections | 64, 512 |
| Parallelism | 128 per connection |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | oha |
| Port | 8443 (TLS + QUIC) |

## Notes

- HTTP/3 support is not universal - only frameworks with native QUIC support participate
- Results may show higher variance than HTTP/1.1 and HTTP/2 due to oha limitations
- See the [oha load generator docs](/docs/load-generators) for known issues
