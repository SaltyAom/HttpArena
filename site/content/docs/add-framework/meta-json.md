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
| `type` | `production` for standard framework usage, `tuned` for optimized/non-default configurations, `engine` for bare-metal implementations |
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
| `mixed` | HTTP/1.1 | `/baseline11`, `/json`, `/db`, `/upload`, `/compression`, `/static/*`, `/async-db` |
| `static` | HTTP/1.1 | `/static/*` (port 8080) |
| `tcp-frag` | HTTP/1.1 | `/baseline11` (loopback MTU 69) |
| `async-db` | HTTP/1.1 | `/async-db` (requires `DATABASE_URL` env var) |
| `baseline-h2` | HTTP/2 | `/baseline2` (TLS, port 8443) |
| `static-h2` | HTTP/2 | `/static/*` (TLS, port 8443) |
| `baseline-h3` | HTTP/3 | `/baseline2` (QUIC, port 8443) |
| `static-h3` | HTTP/3 | `/static/*` (QUIC, port 8443) |
| `unary-grpc` | gRPC | `BenchmarkService/GetSum` (h2c, port 8080) |
| `unary-grpc-tls` | gRPC | `BenchmarkService/GetSum` (TLS, port 8443) |
| `echo-ws` | WebSocket | `/ws` echo (port 8080) |

Only include profiles your framework supports. Frameworks missing a profile simply don't appear in that profile's leaderboard.

### async-db

The `async-db` profile requires an async PostgreSQL driver. The benchmark script starts a Postgres sidecar with 100K rows and passes `DATABASE_URL=postgres://bench:bench@localhost:5432/benchmark` to your container. Your framework must:

1. Connect to Postgres using the `DATABASE_URL` environment variable
2. Implement `GET /async-db?min=X&max=Y` that queries: `SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50`
3. Return JSON: `{"items": [...], "count": N}` with nested `rating: {score, count}` and `tags` as a JSON array
4. Return `{"items":[],"count":0}` if the database is unavailable
5. Use lazy connection initialization — retry connecting if Postgres isn't ready at startup
