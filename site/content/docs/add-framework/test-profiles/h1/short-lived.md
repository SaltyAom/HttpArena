---
title: Short-lived Connection
---

Same workload as baseline, but each connection is closed and re-established after 10 requests. This forces frequent TCP handshakes.

**Connections:** 512, 4,096

## What it measures

- Socket creation and teardown overhead
- Connection accept rate
- Per-connection memory allocation/deallocation
- Any connection pooling or caching strategies

## Real-world relevance

Many clients (mobile, IoT, load balancers without keepalive) don't maintain long-lived connections. This profile captures how well a framework handles the constant churn of short-lived connections — a common pattern in production environments.
