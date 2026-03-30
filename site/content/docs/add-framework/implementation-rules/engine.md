---
title: "* Engine"
weight: 3
---

Engine entries are bare-metal HTTP implementations — raw sockets, custom parsers, low-level I/O. They are not frameworks and are ranked separately.

## What qualifies as an engine

- Raw TCP socket servers with custom HTTP parsing
- Minimal HTTP libraries without routing or middleware
- Direct io_uring or epoll implementations
- Web servers (nginx, h2o) without application framework layers

## Rules

- Must implement the endpoint spec correctly
- Must pass the validation suite
- No restrictions on implementation approach
- Ranked separately from production and tuned entries
- Only participates in connection-level tests (baseline, pipelined, limited-conn) and protocol tests (H2, H3, gRPC, WebSocket) by default
