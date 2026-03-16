# blitz ⚡

A Zig micro web framework built for raw speed. Competes in [HttpArena](https://github.com/MDA2AV/HttpArena).

**Repo:** [github.com/BennyFranciscus/blitz](https://github.com/BennyFranciscus/blitz)

## Architecture

### io_uring Backend (default for benchmarks)

- **Acceptor + Reactor model** — dedicated acceptor thread distributes fds to N reactor threads via lock-free SPSC queues
- **Multishot accept** — single SQE auto-re-arms for new connections
- **Kernel buffer ring** — zero-SQE buffer recycling via shared memory
- **send_zc** — zero-copy send (kernel 6.0+) with auto-probe fallback
- **SINGLE_ISSUER + DEFER_TASKRUN** — reduced kernel overhead per io_uring op
- **Pre-allocated connection pools** — O(1) acquire/release per connection
- **Body discard mode** — uploads >64KB counted without buffering (zero malloc for upload bodies)

### Epoll Backend (fallback)

- Edge-triggered, per-thread event loops with SO_REUSEPORT
- Write WouldBlock handling — registers EPOLLOUT instead of closing, prevents reconnect cascading

### Shared

- **Zero-copy parsing** — request headers/body are slices into the read buffer
- **Response fast path** — pre-computed prefix constants for 200 OK text/plain and application/json
- **Pipeline batching** — multiple requests parsed per read, coalesced writes
- **Radix-trie router** — static, `:param`, `*wildcard` with per-route middleware
- **Pre-computed responses** — benchmark endpoints build full HTTP at startup

## Framework Features

- Radix-trie router with path params and wildcards
- Global + per-route middleware (short-circuit capable)
- Route groups with prefix concatenation
- Comptime JSON serializer (zero-alloc)
- Query string parser with typed access
- Request body parsing (URL-encoded, multipart/form-data)
- Cookie support (RFC 6265)
- Redirect helpers
- Static file serving with MIME detection
- WebSocket support (RFC 6455)
- Structured request logging (text/JSON)
- Gzip/deflate response compression
- Graceful shutdown with connection draining
- 218 unit tests

## Building

```bash
zig build -Doptimize=ReleaseFast
```

Set `BLITZ_URING=1` to use io_uring backend (default in Docker).
Listens on port 8080, one worker thread per CPU core.
