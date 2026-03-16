# Axum

[Axum](https://github.com/tokio-rs/axum) is a web application framework built by the [Tokio](https://tokio.rs/) team. It leverages Tokio, Tower, and Hyper to provide an ergonomic and modular framework for building async Rust web services.

## Key Features

- Built on top of `hyper` and `tokio` — the Rust async ecosystem standards
- Tower middleware compatibility — reuse the entire Tower ecosystem
- Extractor-based request handling — type-safe, composable
- Macro-free routing (though macros are available)
- First-class WebSocket, SSE, and multipart support

## Implementation Notes

- Uses Axum 0.8 with Tokio multi-threaded runtime
- Rustls for TLS/HTTP2 support via `axum-server`
- `tower-http` compression layer for gzip
- Pre-opened SQLite connection pool (one per CPU core) with mmap enabled
- Static files and dataset loaded into memory at startup
- Compiled with `-O3`, thin LTO, single codegen unit
