---
weight: 4
title: gRPC
---

gRPC test profiles measure framework performance using Protocol Buffers over cleartext HTTP/2 (h2c). The server listens on **port 8080** and implements the `BenchmarkService` defined in `benchmark.proto`.

{{< cards >}}
  {{< card link="unary" title="Unary" subtitle="Single unary RPC throughput — GetSum request/response over h2c." icon="globe-alt" >}}
{{< /cards >}}
