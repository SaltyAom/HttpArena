FROM ubuntu:24.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update -yqq && apt-get install -yqq software-properties-common >/dev/null

RUN LC_ALL=C.UTF-8 add-apt-repository ppa:ondrej/php >/dev/null && \
    apt-get update -yqq > /dev/null && apt-get upgrade -yqq >/dev/null && \
    apt-get install -yqq wget git libxml2-dev systemtap-sdt-dev \
                    zlib1g-dev libpcre3-dev libargon2-dev libsodium-dev libkrb5-dev \
                    php8.4-cli php8.4-dev libphp8.4-embed php8.4-sqlite3 php8.4-pgsql >/dev/null

ARG NGINX_VERSION=1.28.3

RUN git clone -b v0.0.30 --single-branch --depth 1 https://github.com/rryqszq4/ngx-php.git >/dev/null

RUN wget -q http://nginx.org/download/nginx-${NGINX_VERSION}.tar.gz && \
    tar -zxf nginx-${NGINX_VERSION}.tar.gz && \
    cd nginx-${NGINX_VERSION} && \
    export PHP_LIB=/usr/lib && \
    ./configure --user=www --group=www \
            --prefix=/nginx \
            --with-ld-opt="-Wl,-rpath,$PHP_LIB" \
            --with-http_ssl_module \
            --with-http_v2_module \
            --with-http_v3_module \
            --with-http_gzip_static_module \
            --add-module=/ngx-php/third_party/ngx_devel_kit \
            --add-module=/ngx-php >/dev/null && \
    make -j$(nproc) >/dev/null && make install >/dev/null

RUN echo "opcache.jit=tracing" >> /etc/php/8.4/embed/conf.d/90-override.ini

COPY . .

RUN chmod +x /compress.sh
RUN /compress.sh

EXPOSE 8080 8443/tcp 8443/udp

ENTRYPOINT ["/nginx/sbin/nginx", "-c", "/nginx.conf"]
