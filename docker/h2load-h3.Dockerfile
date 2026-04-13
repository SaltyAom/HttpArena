FROM ubuntu:24.04 AS builder

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates git build-essential pkg-config \
        autoconf automake autotools-dev libtool \
        cmake ninja-build zlib1g-dev libxml2-dev \
        libev-dev libjemalloc-dev libc-ares-dev \
        libjansson-dev libevent-dev libbrotli-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /src

# quictls: OpenSSL fork that exposes the QUIC API ngtcp2 needs
RUN git clone --depth 1 -b openssl-3.3.0-quic1 https://github.com/quictls/openssl quictls \
    && cd quictls \
    && ./config --prefix=/opt/quictls --libdir=lib enable-tls1_3 no-shared \
    && make -j"$(nproc)" \
    && make install_sw

# nghttp3
RUN git clone --depth 1 --recursive https://github.com/ngtcp2/nghttp3 \
    && cd nghttp3 \
    && autoreconf -i \
    && ./configure --prefix=/opt/nghttp3 --enable-lib-only \
    && make -j"$(nproc)" \
    && make install

# ngtcp2 with quictls crypto helper
RUN git clone --depth 1 --recursive https://github.com/ngtcp2/ngtcp2 \
    && cd ngtcp2 \
    && autoreconf -i \
    && PKG_CONFIG_PATH=/opt/quictls/lib/pkgconfig \
       ./configure --prefix=/opt/ngtcp2 \
           --with-openssl --enable-lib-only \
    && make -j"$(nproc)" \
    && make install

# nghttp2 with h2load h3 support
RUN git clone --depth 1 https://github.com/nghttp2/nghttp2 \
    && cd nghttp2 \
    && git submodule update --init \
    && autoreconf -i \
    && PKG_CONFIG_PATH=/opt/quictls/lib/pkgconfig:/opt/nghttp3/lib/pkgconfig:/opt/ngtcp2/lib/pkgconfig \
       LDFLAGS="-Wl,-rpath,/opt/quictls/lib:/opt/nghttp3/lib:/opt/ngtcp2/lib" \
       ./configure --prefix=/opt/nghttp2 \
           --enable-app --enable-http3 \
           --disable-examples --disable-python-bindings \
    && make -j"$(nproc)" \
    && make install

FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
        libev4 libjemalloc2 libc-ares2 libxml2 libjansson4 \
        libevent-2.1-7t64 libbrotli1 zlib1g ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/quictls/lib /opt/quictls/lib
COPY --from=builder /opt/nghttp3/lib /opt/nghttp3/lib
COPY --from=builder /opt/ngtcp2/lib /opt/ngtcp2/lib
COPY --from=builder /opt/nghttp2 /opt/nghttp2
ENV PATH=/opt/nghttp2/bin:$PATH \
    LD_LIBRARY_PATH=/opt/quictls/lib:/opt/nghttp3/lib:/opt/ngtcp2/lib:/opt/nghttp2/lib
ENTRYPOINT ["h2load"]
