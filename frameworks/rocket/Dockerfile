FROM rust:1.88 AS build
WORKDIR /app
COPY Cargo.toml .
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src/ target/release/httparena-rocket* target/release/deps/httparena_rocket*
COPY src ./src
RUN RUSTFLAGS="-C target-cpu=native" cargo build --release

FROM debian:bookworm-slim
COPY --from=build /app/target/release/httparena-rocket /server
## ROCKET_WORKERS is not set — server reads cgroup cpu.max at runtime
EXPOSE 8080
CMD ["/server"]
