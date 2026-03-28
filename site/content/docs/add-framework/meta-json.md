---
title: meta.json
---

Create a `meta.json` file in your framework directory:

```json
{
  "display_name": "your-framework",
  "language": "Go",
  "engine": "net/http",
  "type": "framework",
  "description": "Short description of the framework and its key features.",
  "repo": "https://github.com/org/repo",
  "enabled": true,
  "tests": ["baseline", "pipelined", "limited-conn", "json", "upload", "compression", "noisy", "mixed", "baseline-h2", "static-h2"]
}
```

## Fields

| Field | Description |
|-------|-------------|
| `display_name` | Name shown on the leaderboard |
| `language` | Programming language (e.g., `Go`, `Rust`, `C#`, `Java`) |
| `engine` | HTTP server engine (e.g., `Kestrel`, `Tomcat`, `hyper`) |
| `type` | `framework` for production-ready frameworks, `engine` for bare-metal/engine-level implementations |
| `description` | Shown in the framework detail popup on the leaderboard |
| `repo` | Link to the framework's source repository |
| `enabled` | Set to `false` to skip this framework during benchmark runs |
| `tests` | Array of test profiles this framework participates in |

## Available test profiles

| Profile | Protocol | Required endpoints |
|---------|----------|--------------------|
| `baseline` | HTTP/1.1 | `/baseline11`, `/pipeline` |
| `pipelined` | HTTP/1.1 | `/pipeline` |
| `limited-conn` | HTTP/1.1 | `/baseline11` |
| `json` | HTTP/1.1 | `/json` |
| `upload` | HTTP/1.1 | `/upload` |
| `compression` | HTTP/1.1 | `/compression` |
| `noisy` | HTTP/1.1 | `/baseline11` |
| `mixed` | HTTP/1.1 | `/baseline11`, `/json`, `/db`, `/upload`, `/compression` |
| `static` | HTTP/1.1 | `/static/*` (port 8080) |
| `baseline-h2` | HTTP/2 | `/baseline2` (TLS, port 8443) |
| `static-h2` | HTTP/2 | `/static/*` (TLS, port 8443) |
| `baseline-h3` | HTTP/3 | `/baseline2` (QUIC, port 8443) |
| `static-h3` | HTTP/3 | `/static/*` (QUIC, port 8443) |
| `unary-grpc` | gRPC | `BenchmarkService/GetSum` (h2c, port 8080) |
| `unary-grpc-tls` | gRPC | `BenchmarkService/GetSum` (TLS, port 8443) |
| `echo-ws` | WebSocket | `/ws` echo (port 8080) |

Only include profiles your framework supports. Frameworks missing a profile simply don't appear in that profile's leaderboard.
