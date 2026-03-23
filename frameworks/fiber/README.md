# Fiber

[Fiber](https://github.com/gofiber/fiber) is an Express-inspired Go web framework built on [fasthttp](https://github.com/valyala/fasthttp). It's one of the most popular Go web frameworks (~35k stars) known for extreme performance and a familiar API for developers coming from Node.js/Express.

## Key Features

- Built on fasthttp (zero-allocation HTTP engine)
- Express-like API for ease of use
- Prefork mode for multi-core utilization
- Zero memory allocation routing
- Built-in middleware ecosystem

## Implementation Notes

- Uses Fiber v2 with prefork mode enabled (one process per CPU core)
- Pure Go SQLite via `modernc.org/sqlite` (no CGO)
- Manual compression handling (deflate/gzip) matching HttpArena spec
- Static files pre-loaded into memory at startup
- 25MB body limit for upload endpoint

## Why This Entry Matters

HttpArena already has raw fasthttp and two net/http-based frameworks (Gin, Echo). Fiber completes the Go comparison by showing how a framework built *on top of* fasthttp compares — what's the overhead of Fiber's routing and middleware layer over raw fasthttp? And how does fasthttp-based Fiber compare to net/http-based Gin and Echo?
