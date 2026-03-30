---
title: "# Tuned"
weight: 2
---

Tuned entries have more freedom. They can use non-default configurations, experimental flags, and optimizations that go beyond standard framework usage.

## What is allowed

- Alternative JSON serializers (simd-json, sonic-json, etc.)
- Custom buffer sizes and TCP socket options
- Experimental or unstable framework flags
- Pre-computed responses and response caching
- Memory-mapped files and in-memory static file caching
- Custom thread pools and worker configurations
- Non-default GC settings without documentation requirement
- Framework-specific performance flags not recommended for production

## What is still required

- Must use the framework's HTTP server (not a raw socket replacement)
- Must implement all endpoint specs correctly
- Must pass the validation suite
- The framework dependency must be a real, published framework

## When to choose Tuned

If your submission uses any setting or optimization that a typical production team would not use, classify it as tuned. If in doubt, start with tuned — it is easier to move to production later than to justify non-standard optimizations.
