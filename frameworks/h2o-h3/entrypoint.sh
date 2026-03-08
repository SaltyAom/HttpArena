#!/bin/sh
set -e

NPROC=$(nproc)

# Preprocess dataset.json → response.json (add total field, wrap in {items,count})
if [ -f /data/dataset.json ]; then
    jq '{ items: [.[] | . + { total: ((.price * .quantity * 100 | round) / 100) }], count: length }' \
        /data/dataset.json > /tmp/response.json
fi

# Generate h2o.conf
cat > /tmp/h2o.conf << EOF
num-threads: ${NPROC}

listen:
  port: 8080

listen: &ssl_listen
  port: 8443
  ssl:
    certificate-file: /certs/server.crt
    key-file: /certs/server.key

listen:
  <<: *ssl_listen
  type: quic

hosts:
  default:
    paths:
      "/pipeline":
        mruby.handler: |
          Proc.new { [200, {"content-type" => "text/plain"}, ["ok"]] }

      "/baseline11":
        mruby.handler: |
          Proc.new do |env|
            sum = 0
            qs = env["QUERY_STRING"]
            if qs
              qs.split("&").each do |pair|
                _k, v = pair.split("=", 2)
                sum += v.to_i if v
              end
            end
            if env["REQUEST_METHOD"] == "POST"
              body = env["rack.input"] ? env["rack.input"].read : ""
              body = body.strip
              sum += body.to_i if body.length > 0
            end
            [200, {"content-type" => "text/plain"}, [sum.to_s]]
          end

      "/baseline2":
        mruby.handler: |
          Proc.new do |env|
            sum = 0
            qs = env["QUERY_STRING"]
            if qs
              qs.split("&").each do |pair|
                _k, v = pair.split("=", 2)
                sum += v.to_i if v
              end
            end
            [200, {"content-type" => "text/plain"}, [sum.to_s]]
          end

      "/json":
        file.file: /tmp/response.json
        header.add: "content-type: application/json"

      "/static":
        file.dir: /data/static
EOF

exec h2o -c /tmp/h2o.conf
