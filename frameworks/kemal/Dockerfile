# Build stage
FROM crystallang/crystal:1.14-alpine AS builder

WORKDIR /app

# Install SQLite dev headers
RUN apk add --no-cache sqlite-dev sqlite-static

# Install dependencies
COPY shard.yml shard.lock ./
RUN shards install --production

# Copy source and build
COPY src/ src/
RUN crystal build src/server.cr -o server --release --no-debug

# Runtime stage — need same libc as builder (musl on Alpine)
FROM alpine:3.20

RUN apk add --no-cache ca-certificates libgcc gc libevent sqlite-libs pcre2

WORKDIR /app
COPY --from=builder /app/server /app/server

EXPOSE 8080

CMD ["/app/server"]
