---
title: Implementation Guidelines
---
{{< type-rules production="Same rules as baseline - standard framework configuration with no TCP tuning." tuned="Same as baseline tuned rules. The MTU change is applied externally by the benchmark runner." engine="No specific rules." >}}


Stress test with extreme TCP fragmentation - loopback MTU forced to 69 bytes, splitting every packet into ~29-byte payload segments. Uses 8 diverse request templates designed to maximize parser stress.

**Connections:** 512, 4,096, 16,384
**Requests per connection:** 2 (rotates through all templates)

## How it works

Before the load generator starts, the benchmark script sets the loopback interface MTU to 69:

```bash
sudo ip link set lo mtu 69
```

This is the lowest practical MTU (IP minimum is 68). With 20-byte IP headers and 20-byte TCP headers, each TCP segment carries only **29 bytes of payload**. The MTU is restored to 65,536 after the test.

Each connection sends 2 requests then reconnects, rotating to the next template. This ensures all 8 templates are exercised evenly across all connections.

## Request templates

8 templates designed to stress different aspects of HTTP parsing under fragmentation:

| Template | Size | Segments | Purpose |
|----------|------|----------|---------|
| **GET** (×2) | 355B | 13 | Browser-like headers (User-Agent, Accept, etc.) |
| **POST** | 360B | 13 | Content-Length body across segments |
| **Chunked** | 379B | 14 | `\r\n\r\n` header boundary splits at segment edge |
| **Noise** | 843B | 30 | 500-character junk path (expects 404) |
| **Cookie bomb** | 2,097B | 73 | 2KB cookie header parsed across 73 segments |
| **30 Headers** | 380B | 14 | 30 tiny headers, boundaries everywhere |
| **Body29** | 146B | 6 | Body fills exactly one segment (29 bytes) |

## What it measures

- TCP segment reassembly efficiency under extreme fragmentation
- HTTP parser robustness when headers, bodies, and boundaries split across reads
- Syscall overhead when the kernel processes many small packets
- Cookie and multi-header parsing across fragmented buffers
- Chunked transfer-encoding with split `\r\n\r\n` boundaries
- Framework resilience to junk routes under fragmentation

## Expected behavior

Frameworks typically see 60-90% throughput reduction compared to baseline (MTU 65,536). Some frameworks collapse at high connection counts (4,096+) as the kernel TCP stack gets overwhelmed by the volume of tiny segments. The stacked bar in the results shows which template types each framework handles well vs which cause errors.

## Parameters

| Parameter | Value |
|-----------|-------|
| Endpoint | `/baseline11` (GET + POST) |
| Loopback MTU | 69 bytes (29 bytes payload per segment) |
| Templates | 8 (GET×2, POST, Chunked, Noise, Cookie, 30 Headers, Body29) |
| Connections | 512, 4,096, 16,384 |
| Requests/connection | 2 (rotate on reconnect) |
| Pipeline | 1 |
| Duration | 5s |
| Runs | 3 (best taken) |
| Load generator | gcannon |
