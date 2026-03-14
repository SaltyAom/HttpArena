FROM rust:1-bookworm AS build
WORKDIR /app

RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock* ./
COPY build.rs ./
COPY proto/ proto/
COPY src/ src/

RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app

COPY --from=build /app/target/release/tonic-grpc .

EXPOSE 8080 8443
ENTRYPOINT ["./tonic-grpc"]
