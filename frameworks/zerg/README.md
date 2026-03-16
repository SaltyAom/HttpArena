# zerg — C# io_uring TCP server

[zerg](https://github.com/MDA2AV/zerg) is a low-level TCP server framework for C# built directly on Linux `io_uring`. It provides zero-copy buffer rings, multishot accept/recv, and `DEFER_TASKRUN`/`SINGLE_ISSUER` optimizations — no HTTP abstractions, just raw TCP with async/await.

This entry builds a full HTTP/1.1 server on top of zerg using its `ConnectionPipeReader` adapter for robust buffer management, with manual HTTP parsing and routing.

## What makes it interesting

- **io_uring native:** Direct ring submission via liburing shim — no epoll, no kqueue
- **Zero-copy reads:** Provided buffer rings let the kernel write directly into pre-allocated memory
- **C# without Kestrel:** Shows what .NET can do when you bypass the ASP.NET stack entirely
- **Same language, different I/O:** Direct comparison with `aspnet-minimal` (Kestrel) — same runtime, radically different I/O strategy

## Configuration

- Reactor count = CPU count (one io_uring instance per reactor thread)
- 16KB recv buffers, 16K buffer ring entries per reactor
- SINGLE_ISSUER + DEFER_TASKRUN ring flags for minimal kernel transitions
- PipeReader adapter for correct HTTP pipelining support

## Requirements

- Linux kernel 6.1+ (io_uring provided buffers)
- .NET 10 preview
