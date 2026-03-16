FROM debian:bookworm-slim AS build
RUN apt-get update && apt-get install -y wget xz-utils && \
    wget -q https://ziglang.org/download/0.14.0/zig-linux-x86_64-0.14.0.tar.xz && \
    tar xf zig-linux-x86_64-0.14.0.tar.xz && \
    mv zig-linux-x86_64-0.14.0 /usr/local/zig
ENV PATH="/usr/local/zig:$PATH"
WORKDIR /app
COPY build.zig build.zig.zon ./
COPY src ./src
RUN zig build -Doptimize=ReleaseFast

FROM debian:bookworm-slim
COPY --from=build /app/zig-out/bin/blitz /server
ENV BLITZ_URING=1
EXPOSE 8080
CMD ["/server"]
