FROM golang:1.24-bookworm AS build
RUN go install github.com/caddyserver/xcaddy/cmd/xcaddy@latest
WORKDIR /app
COPY go.mod handler.go ./
RUN xcaddy build v2.9.1 --with httparena-handler=/app

FROM debian:bookworm-slim
COPY --from=build /app/caddy /usr/local/bin/caddy
COPY Caddyfile /etc/caddy/Caddyfile
EXPOSE 8080 8443/tcp 8443/udp
ENV GOGC=400
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile"]
