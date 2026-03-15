# blitz ⚡

A blazing-fast HTTP/1.1 server written in Zig, built to compete in [HttpArena](https://github.com/MDA2AV/HttpArena).

## Design

- **Language:** Zig — C-level performance with better ergonomics
- **I/O:** epoll with edge-triggered notifications
- **Threading:** SO_REUSEPORT multi-threading (one accept socket per core, no lock contention)
- **Parsing:** Zero-copy HTTP request parsing
- **Responses:** Pre-computed static responses, pipeline batching
- **Memory:** Arena-style per-connection buffers, minimal heap allocations in hot path

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns "ok" — pipelining benchmark |
| `/baseline11` | GET/POST | Query param sum, optional body parsing |
| `/baseline2` | GET | Query param sum (H2 baseline) |
| `/json` | GET | Pre-computed JSON dataset response |
| `/upload` | POST | Returns body byte count |
| `/static/{file}` | GET | Pre-loaded static files |

## Building

```bash
zig build -Doptimize=ReleaseFast
```

## Running

```bash
./zig-out/bin/blitz
```

The server listens on port 8080 and spawns one worker thread per available CPU core.
