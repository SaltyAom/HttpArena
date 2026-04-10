---
title: Implementation Guidelines
---
{{< type-rules production="Must use the framework's standard reverse proxy configuration. The proxy must be a widely-used, production-grade server (Nginx, Caddy, Envoy, HAProxy, Traefik, etc.). No custom protocol handlers or request-level code in the proxy layer. Server must use standard framework middleware for all endpoints." tuned="May optimize proxy configuration (worker counts, buffer sizes, keepalive tuning, connection pooling). May tune the proxy-to-server protocol (h2c, Unix sockets, etc.). Server may use any caching or optimization strategy." engine="No specific rules. May use custom proxy implementations or skip the proxy entirely. Ranked separately from frameworks." >}}

The Gateway-64 test benchmarks a complete HTTP/2 application stack — not just a single server process, but an entire deployment as you'd run it in production. The load generator sends TLS-encrypted HTTP/2 requests to port 8443. What listens on that port — and what happens to the request after it arrives — is entirely up to the entry.

Most entries will use a reverse proxy (Nginx, Caddy, Envoy) that terminates TLS and forwards to an application server. But entries can use **any architecture**: multiple proxies, multiple application servers, caches, connection poolers, load balancers, or a single server handling everything. The only constraint is that port 8443 must accept HTTPS/H2 and the total CPU budget is 64 logical CPUs.

## Architecture

The simplest configuration is a two-service stack: a reverse proxy that handles TLS and static files, and an application server that handles dynamic endpoints.

```
                    TLS (h2)              proxy → server
  ┌──────────┐    ──────────>    ┌───────────┐    ──────>    ┌──────────┐
  │  h2load  │                   │   Proxy   │               │  Server  │
  │  (load   │    port 8443      │  (Nginx,  │    any port   │  (app    │
  │   gen)   │    HTTPS/h2       │  Caddy…)  │    any proto  │  server) │
  └──────────┘                   └───────────┘               └──────────┘
                                 CPU: N of 64                CPU: 64-N
```

But this test supports **any architecture** — the only rule is that port **8443** must accept HTTPS/H2 connections and return correct responses. Everything behind that port is entirely the entry's design choice. You can run one container, two, five, or ten. You can have multiple proxies, multiple application servers, caches, connection poolers, or any combination.

The benchmark script doesn't know or care about your internal architecture. It sends requests to `https://localhost:8443`, measures throughput, and collects CPU/memory stats from all your compose containers combined. How you route, balance, cache, or process those requests is what makes each entry unique.

### Example configurations

**Two-tier** — proxy + server (most common):
```
  h2load :8443 → Nginx (TLS, static) → ASP.NET :8080 (json, db, baseline)
```

**Three-tier** — proxy + cache + server:
```
  h2load :8443 → Nginx (TLS) → Varnish (cache static) → Go :8080 (all endpoints)
```

**Single-tier** — no proxy, server handles everything:
```
  h2load :8443 → Actix-web (TLS, static, json, db, baseline — all 64 CPUs)
```

**Sidecar pattern** — proxy + server + connection pooler:
```
  h2load :8443 → Caddy (TLS) → Node.js :8080 (json, baseline)
                              → PgBouncer :6432 → Postgres (async-db)
```

**Load-balanced** — proxy distributing to multiple server instances:
```
  h2load :8443 → HAProxy (TLS, round-robin) → server-1 :8081 (all endpoints)
                                             → server-2 :8082 (all endpoints)
                                             → server-3 :8083 (all endpoints)
```

**Split proxy** — dedicated proxies for different endpoint types:
```
  h2load :8443 → Nginx (TLS, routing)
                   ├── /static/*   → served directly by Nginx from disk
                   ├── /json       → app-server :8080
                   ├── /baseline2  → app-server :8080
                   └── /async-db   → app-server :8080 → PgBouncer :6432
```

**Multi-server specialization** — different servers for different endpoints:
```
  h2load :8443 → Envoy (TLS, route by path)
                   ├── /static/*   → Nginx :8081 (optimized for static files)
                   ├── /json       → Rust server :8082 (optimized for compute)
                   ├── /baseline2  → Rust server :8082
                   └── /async-db   → Go server :8083 (optimized for async I/O)
```

All of these are valid as long as:
1. Port 8443 accepts HTTPS/H2 and returns correct responses for all four endpoint types
2. The total CPU allocation across all services equals exactly 64 logical CPUs
3. All compose settings follow the [required settings](#required-compose-settings)

## Docker Compose

Every gateway entry is orchestrated via **Docker Compose**. The entry provides a `compose.gateway.yml` file that defines the full stack — all services, their images, CPU pinning, volumes, and networking. The benchmark script builds, starts, and tears down this compose stack for each test run.

This approach gives entries full control over their deployment architecture while keeping the benchmark infrastructure simple and consistent.

### How the benchmark script uses your compose file

The benchmark script runs your compose file through a well-defined lifecycle:

1. **Build phase** (once per framework):
   ```bash
   CERTS_DIR=... DATA_DIR=... DATABASE_URL=... \
     docker compose -f compose.gateway.yml -p httparena-<framework> build
   ```

2. **Start phase** (once per connection count):
   ```bash
   CERTS_DIR=... DATA_DIR=... DATABASE_URL=... \
     docker compose -f compose.gateway.yml -p httparena-<framework> up -d
   ```

3. **Wait** — the script polls `https://localhost:8443` until it responds (up to 30 seconds)

4. **Benchmark** — h2load runs against port 8443 for 5 seconds, repeated 3 times (best taken)

5. **Stats collection** — during each run, `docker stats` polls all containers and sums CPU/memory usage across the entire stack

6. **Teardown** (after each connection count):
   ```bash
   docker compose -f compose.gateway.yml -p httparena-<framework> down --remove-orphans
   ```

### Environment variables

The benchmark script exports three environment variables before every compose command. Your compose file **must** use these — do not hardcode paths.

| Variable | Value | Example |
|----------|-------|---------|
| `CERTS_DIR` | Absolute path to TLS certificate directory. Contains `server.crt` and `server.key`. | `/home/user/HttpArena/certs` |
| `DATA_DIR` | Absolute path to the data directory. Contains `static/`, `dataset.json`, `dataset-large.json`, `benchmark.db`. | `/home/user/HttpArena/data` |
| `DATABASE_URL` | Postgres connection string for the async-db endpoint. The Postgres sidecar is started automatically by the benchmark script before your compose stack. | `postgres://bench:bench@localhost:5432/benchmark` |

Use `${CERTS_DIR}`, `${DATA_DIR}`, and `${DATABASE_URL}` in your compose file to reference these:

```yaml
volumes:
  - ${CERTS_DIR}:/certs:ro           # TLS certificates
  - ${DATA_DIR}/static:/data/static:ro  # Static files
  - ${DATA_DIR}/dataset.json:/data/dataset.json:ro  # JSON dataset
environment:
  - DATABASE_URL=${DATABASE_URL}     # Postgres connection
```

### Required compose settings

Every service in your compose file **must** include these settings:

| Setting | Value | Why |
|---------|-------|-----|
| `network_mode: host` | All services | Docker bridge networking adds measurable latency. Host networking ensures services communicate at native speed via localhost. |
| `cpuset` | CPU range string | Pins the service to specific CPU cores. See [CPU allocation](#cpu-allocation) for the rules. |
| `security_opt: [seccomp:unconfined]` | All services | Required for io_uring and other low-level syscalls that Docker's default seccomp profile blocks. |
| `ulimits.memlock: -1` | All services | Allows memory locking for performance-critical operations. |
| `ulimits.nofile: { soft: 1048576, hard: 1048576 }` | All services | Raises the file descriptor limit to handle many concurrent connections. |

### Example: two-tier stack (proxy + server)

This is the most common configuration — an Nginx proxy serving static files and forwarding dynamic requests to the application server.

```yaml
services:
  proxy:
    build: ./proxy
    network_mode: host
    cpuset: "0-7,64-71"
    ulimits:
      memlock: -1
      nofile:
        soft: 1048576
        hard: 1048576
    security_opt:
      - seccomp:unconfined
    volumes:
      - ${CERTS_DIR}:/certs:ro
      - ${DATA_DIR}/static:/data/static:ro
    depends_on:
      - server

  server:
    build:
      context: ../../
      dockerfile: frameworks/my-framework/Dockerfile
    network_mode: host
    cpuset: "8-31,72-95"
    ulimits:
      memlock: -1
      nofile:
        soft: 1048576
        hard: 1048576
    security_opt:
      - seccomp:unconfined
    environment:
      - DATABASE_URL=${DATABASE_URL}
      - DATABASE_MAX_CONN=256
    volumes:
      - ${DATA_DIR}/dataset.json:/data/dataset.json:ro
      - ${DATA_DIR}/dataset-large.json:/data/dataset-large.json:ro
```

In this example:
- **Proxy** (Nginx) gets 8 physical cores (16 logical CPUs: `0-7,64-71`). It handles TLS termination, serves `/static/*` directly from disk, and forwards `/json`, `/async-db`, and `/baseline2` to the server on localhost:8080.
- **Server** gets 24 physical cores (48 logical CPUs: `8-31,72-95`). It listens on port 8080 (HTTP/1.1, no TLS) and handles all dynamic endpoints.
- Total: 16 + 48 = 64 logical CPUs.

### Example: three-tier stack (proxy + cache + server)

A more complex setup with a caching layer between the proxy and server:

```yaml
services:
  proxy:
    build: ./proxy
    network_mode: host
    cpuset: "0-3,64-67"
    ulimits:
      memlock: -1
      nofile: { soft: 1048576, hard: 1048576 }
    security_opt: [seccomp:unconfined]
    volumes:
      - ${CERTS_DIR}:/certs:ro
    depends_on: [cache, server]

  cache:
    image: varnish:7
    network_mode: host
    cpuset: "4-7,68-71"
    ulimits:
      memlock: -1
      nofile: { soft: 1048576, hard: 1048576 }
    security_opt: [seccomp:unconfined]
    volumes:
      - ./varnish.vcl:/etc/varnish/default.vcl:ro
      - ${DATA_DIR}/static:/data/static:ro

  server:
    build:
      context: ../../
      dockerfile: frameworks/my-framework/Dockerfile
    network_mode: host
    cpuset: "8-31,72-95"
    ulimits:
      memlock: -1
      nofile: { soft: 1048576, hard: 1048576 }
    security_opt: [seccomp:unconfined]
    environment:
      - DATABASE_URL=${DATABASE_URL}
    volumes:
      - ${DATA_DIR}/dataset.json:/data/dataset.json:ro
      - ${DATA_DIR}/dataset-large.json:/data/dataset-large.json:ro
```

In this example: proxy (8 CPUs) → cache (8 CPUs) → server (48 CPUs) = 64 total.

### Example: single-tier (no proxy)

The server handles TLS directly with all 64 CPUs:

```yaml
services:
  server:
    build:
      context: ../../
      dockerfile: frameworks/my-framework/Dockerfile
    network_mode: host
    cpuset: "0-31,64-95"
    ulimits:
      memlock: -1
      nofile: { soft: 1048576, hard: 1048576 }
    security_opt: [seccomp:unconfined]
    environment:
      - DATABASE_URL=${DATABASE_URL}
    volumes:
      - ${CERTS_DIR}:/certs:ro
      - ${DATA_DIR}/static:/data/static:ro
      - ${DATA_DIR}/dataset.json:/data/dataset.json:ro
      - ${DATA_DIR}/dataset-large.json:/data/dataset-large.json:ro
```

This tests whether a framework's built-in TLS handling can compete with dedicated reverse proxies. The server must listen on port 8443 with HTTPS/H2.

### Build context

Compose resolves `build` paths relative to the compose file's location. Since `compose.gateway.yml` lives in `frameworks/<name>/`, these paths work:

| What you need | How to reference it |
|--------------|-------------------|
| Proxy Dockerfile in `proxy/` | `build: ./proxy` |
| Server Dockerfile in same dir | `build: .` |
| Server Dockerfile needing repo root (e.g., to copy shared data) | `build: { context: ../../, dockerfile: frameworks/<name>/Dockerfile }` |
| Pre-built image | `image: nginx:1.27-alpine` |

## CPU allocation

The test uses **64 logical CPUs** — 32 physical cores (0-31) plus their SMT/HyperThreading siblings (64-95). The entry's `compose.gateway.yml` defines how these are split between services using the `cpuset` directive.

### CPU topology rules

CPUs are always allocated in **physical core pairs** — each allocation includes both the physical core and its SMT sibling. This ensures consistent performance characteristics and avoids scenarios where two services share a physical core.

The benchmark machine has 32 physical cores (0-31) with SMT siblings (64-95), for a total of **64 logical CPUs**. Entries must allocate all 64 CPUs across their services. Example splits:

| Services | Proxy cpuset | Server cpuset | Total logical CPUs |
|----------|-------------|---------------|-------------------|
| No proxy | — | 0-31,64-95 | 64 |
| 2 cores proxy | 0-1,64-65 | 2-31,66-95 | 4 + 60 = 64 |
| 4 cores proxy | 0-3,64-67 | 4-31,68-95 | 8 + 56 = 64 |
| 8 cores proxy | 0-7,64-71 | 8-31,72-95 | 16 + 48 = 64 |
| 3-tier (4+4+24) | 0-3,64-67 | 4-7,68-71 + 8-31,72-95 | 8 + 8 + 48 = 64 |

**Rules:**
- Every allocated core must include both the physical core **and** its SMT sibling (e.g., core 3 → CPUs 3 and 67)
- The total across all services must equal exactly 64 logical CPUs (0-31 + 64-95)
- No two services may share a physical core
- More complex configurations (proxy + app + cache + sidecar) are allowed — just follow the pairing and total rules

### Understanding SMT pairing

On the benchmark machine, each physical core has two logical CPUs — the core itself and its SMT (Simultaneous Multi-Threading / HyperThreading) sibling. The mapping is:

- Physical core 0 → logical CPUs **0** and **64**
- Physical core 1 → logical CPUs **1** and **65**
- Physical core 15 → logical CPUs **15** and **79**
- Physical core 31 → logical CPUs **31** and **95**

The general formula: physical core **N** maps to logical CPUs **N** and **N+64**.

When you allocate cores, you must always allocate **both siblings**. This is because:
1. Two threads sharing a physical core compete for execution resources (ALU, cache, branch predictor)
2. If service A gets CPU 3 and service B gets CPU 67, they share the same physical core — causing unpredictable performance interference
3. Allocating both siblings to the same service ensures clean isolation between services

**Correct** — proxy gets physical cores 0-7 (16 logical CPUs):
```
cpuset: "0-7,64-71"
```

**Incorrect** — splits a physical core across services:
```
# BAD: CPU 3 and 67 are siblings on the same physical core
proxy:  cpuset: "0-3"         # gets core 3 without its sibling (67)
server: cpuset: "4-31,64-95"  # gets sibling 67 without core 3
```

**Incorrect** — doesn't allocate all 64 CPUs:
```
# BAD: only 32 CPUs allocated, 32 wasted
proxy:  cpuset: "0-3,64-67"     # 8 CPUs
server: cpuset: "4-15,68-79"    # 24 CPUs — should be "4-31,68-95"
```

### Choosing the right split

The optimal split depends on what each service does:

- **Lightweight proxies** (Nginx forwarding only, no static files) — 2-4 physical cores is usually enough
- **TLS-terminating proxies** with heavy traffic — 4-8 physical cores. TLS handshakes and encryption are CPU-intensive, especially with many concurrent HTTP/2 streams
- **Proxies serving static files** — may need more cores if the file serving itself is a bottleneck
- **No proxy entries** — get all 64 logical CPUs but must handle TLS, static files, and all endpoints in a single process
- **Three-tier stacks** — allocate most cores to the service doing the heaviest work (usually the application server)

### Common mistakes

1. **Not allocating all 64 CPUs** — every CPU in the 0-31 + 64-95 range must be assigned to exactly one service. Unassigned CPUs are wasted.
2. **Splitting SMT siblings across services** — CPUs N and N+64 must always go to the same service.
3. **Using `cpus` instead of `cpuset`** — the `cpus` directive limits how much CPU time a container gets but doesn't pin to specific cores. Use `cpuset` for deterministic, reproducible pinning.
4. **Forgetting `network_mode: host`** — Docker bridge networking adds measurable latency and throughput overhead. All services must use host networking.
5. **Hardcoding paths** — always use `${CERTS_DIR}`, `${DATA_DIR}`, and `${DATABASE_URL}`. The benchmark machine's paths differ from yours.

## meta.json configuration

```json
{
  "display_name": "my-framework + nginx",
  "language": "C#",
  "type": "production",
  "engine": "kestrel",
  "description": "ASP.NET minimal API behind Nginx reverse proxy.",
  "repo": "https://github.com/dotnet/aspnetcore",
  "tests": ["gateway-64"]
}
```

No special fields are needed beyond the standard meta.json — the compose file handles all orchestration. A framework may subscribe to both gateway-64 and other tests (e.g., `baseline`, `json`), in which case the standard Dockerfile is used for non-gateway tests and the compose file for gateway-64.

## Directory structure

```
frameworks/my-framework/
  compose.gateway.yml   # Docker Compose defining the full stack
  Dockerfile            # Server image
  meta.json
  proxy/
    Dockerfile          # Proxy image (e.g., Nginx with custom config)
    nginx.conf          # Proxy configuration
```

For entries without a proxy, the `proxy/` directory is not needed — the compose file just defines a single `server` service.

For entries that only participate in gateway-64 (not other tests), the top-level `Dockerfile` is only used by the compose file's `server` service build.

## Proxy requirements

### Inbound (load generator to proxy)

- **Port 8443** — the proxy (or server, if no proxy) must listen on this port for HTTPS
- **TLS 1.2+** with HTTP/2 via ALPN negotiation
- **TLS certificate** — use the shared certificates mounted at `/certs/server.crt` and `/certs/server.key`

### Outbound (proxy to server)

The proxy-to-server protocol is the **entry's choice**. Common options, from simplest to most optimized:

| Protocol | Pros | Cons | Example |
|----------|------|------|---------|
| **HTTP/1.1 over TCP** | Simplest to configure, widely supported | No multiplexing, one request per connection at a time | Nginx `proxy_pass http://127.0.0.1:8080` |
| **HTTP/1.1 with keepalive pool** | Reuses connections, avoids TCP handshake per request | Still no multiplexing | Nginx `upstream` with `keepalive 256` |
| **h2c (HTTP/2 cleartext)** | Multiplexing without TLS overhead | Some frameworks don't support h2c | Nginx `proxy_pass http://127.0.0.1:8080` with `proxy_http_version 2.0` |
| **HTTP/2 with TLS** | Full multiplexing + encryption | Double TLS overhead (client→proxy and proxy→server) | Nginx `proxy_pass https://127.0.0.1:8443` |
| **Unix domain sockets** | Lowest latency, no TCP overhead | Both services must share a filesystem (host networking makes this easy) | Nginx `proxy_pass http://unix:/tmp/app.sock` |

The benchmark measures end-to-end throughput from the load generator's perspective. How the proxy communicates with the server is an implementation detail — but the choice matters for performance.

### Request routing

The proxy must forward all paths to the backend. The proxy may also serve some paths directly (e.g., static files from disk) — this is a valid and common architectural choice.

| Path | Description | Can proxy serve directly? |
|------|-------------|--------------------------|
| `/static/*` | Static file serving | Yes — proxy can serve from `/data/static/` mounted volume |
| `/json` | JSON processing (compute-bound) | No — requires application logic |
| `/async-db?min=N&max=M` | Async database query | No — requires database connection |
| `/baseline2?a=N&b=M` | Query parameter sum | No — requires application logic |

Serving static files directly from the proxy (e.g., Nginx `location /static/ { alias /data/static/; }`) is a common production pattern and is **allowed for all framework types**.

## Endpoints

The stack must serve four endpoint types. Whether the proxy or server handles each one is the entry's choice — the load generator doesn't care what happens behind port 8443.

### `/static/*` — Static files

Serves static files from the dataset (CSS, JS, HTML, fonts, images, JSON). The files are available at `/data/static/` inside any container that mounts `${DATA_DIR}/static`. Pre-compressed versions (`.gz`, `.br`) are also available on disk.

All requests include `Accept-Encoding: br;q=1, gzip;q=0.8`. Compression is optional but recommended for text-based files — frameworks that compress will benefit from reduced I/O. See the [Static Files compression rules](/docs/test-profiles/h1/isolated/static/implementation/#compression) for details.

**Who serves this?** The proxy can serve these directly from disk (e.g., Nginx `location /static/ { alias /data/static/; }` with `gzip_static on; brotli_static on;`), or the proxy can forward to the server. Serving from the proxy is typically faster since it avoids the proxy-to-server hop for static content.

### `/json` — JSON processing

Loads a 50-item dataset, computes `total = price * quantity` for each item, and returns a JSON response (~10 KB). This is compute-bound and must be handled by the application server.

Refer to the [JSON Processing implementation](/docs/test-profiles/h1/isolated/json-processing/implementation) for the expected request/response format.

### `/async-db` — Async database query

Queries a Postgres database with `min` and `max` price range parameters and returns up to 50 items as JSON. This is I/O-bound and must be handled by the application server.

The `DATABASE_URL` environment variable is set in the compose environment and provides the Postgres connection string. The Postgres sidecar is started automatically by the benchmark script before the compose stack.

Refer to the [Async Database implementation](/docs/test-profiles/h1/isolated/async-database/implementation) for the query, expected response structure, and connection pooling requirements.

### `/baseline2` — Query parameter sum

Parses query parameters `a` and `b`, returns their sum as plain text. This is the same endpoint used in the [Baseline (HTTP/2) test](/docs/test-profiles/h2/baseline-h2/implementation) — a lightweight request that measures raw proxy forwarding overhead with minimal server-side computation.

## Workload

The load generator ([h2load](https://nghttp2.org/documentation/h2load-howto.html)) requests 20 URIs in a round-robin pattern across multiplexed HTTP/2 streams:

| Category | URIs | Count | Weight |
|----------|------|-------|--------|
| Static files | `/static/reset.css`, `layout.css`, `theme.css`, `components.css`, `utilities.css`, `analytics.js`, `helpers.js`, `app.js`, `vendor.js`, `router.js`, `header.html`, `footer.html` | 12 | 60% |
| JSON | `/json` | 3 | 15% |
| Async DB | `/async-db?min=10&max=50` | 3 | 15% |
| Baseline | `/baseline2?a=1&b=1` | 2 | 10% |

The mix is weighted toward static files (60%) with JSON (15%), database queries (15%), and lightweight baseline requests (10%), approximating a realistic application's traffic pattern. The leaderboard shows a colored breakdown of how many requests each category handled.

## What it measures

- **Reverse proxy overhead** — how much throughput is lost by adding a proxy layer vs. direct server access
- **TLS termination efficiency** — proxy vs. application-level TLS handling
- **HTTP/2 multiplexing through a proxy** — whether the proxy can efficiently forward multiplexed streams
- **Mixed workload throughput** — static files, compute-bound JSON, and I/O-bound database queries competing for the same CPU budget
- **Architecture decisions** — proxy selection, proxy-to-server protocol, CPU allocation strategy
- **End-to-end stack performance** — the combined efficiency of all services as a production-like deployment unit

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoints | `/static/*`, `/json`, `/async-db`, `/baseline2` |
| Connections | 256, 1,024 |
| Streams per connection | 100 (`-m 100`) |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | h2load with `-i` (multi-URI round-robin) |
| Total CPU budget | 64 logical (32 physical + 32 SMT) |
| Memory limit | Unlimited |
| Port | 8443 (HTTPS/H2) |
| Orchestration | Docker Compose (`compose.gateway.yml`) |
