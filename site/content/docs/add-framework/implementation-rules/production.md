---
title: "$ Production"
weight: 1
---

The most restrictive category. Production entries measure what the framework gives you out of the box, the way developers actually use it in real applications.

## Use framework-level APIs

If a framework provides a documented, high-level way to accomplish a task, the benchmark implementation **must** use it. Bypassing the framework to hand-roll a faster solution is not permitted.

{{< tabs items="Good,Bad" >}}

{{< tab >}}
```python
# Use the framework's built-in parameter binding
@app.get("/baseline")
def baseline(a: int, b: int):
    return str(a + b)
```
{{< /tab >}}

{{< tab >}}
```python
# Manually parse query string for speed
@app.get("/baseline")
def baseline(request):
    qs = request.url.query.encode()
    a = fast_parse_int(qs, b"a=")
    b = fast_parse_int(qs, b"b=")
    return custom_serialize(a + b)
```
{{< /tab >}}

{{< /tabs >}}

## Settings must be production-documented

Non-default configuration is allowed **only if the framework's official production deployment guide recommends it**. If there is no official documentation recommending a setting for production use, it does not belong in the benchmark.

**Allowed:**
- GC settings recommended in production deployment guides
- Worker/thread counts matching available CPU cores
- Connection pool sizes for the environment

**Not allowed:**
- Undocumented flags found by reading framework source code
- Experimental or unstable options that trade safety for speed
- Settings that disable buffering, validation, or error handling

## Use standard libraries and drivers

If the ecosystem has a well-established, production-grade library for a task (database driver, JSON serializer), use it. Experimental or hand-rolled alternatives solely for benchmark performance are not permitted.

**Exception:** If the framework itself bundles or officially recommends a specific library, that library is acceptable.

## Static files must be read from disk

For static file tests, production entries must read files from disk on every request. No in-memory caching, no memory-mapped files, no pre-loaded file buffers.

## Deployment-environment tuning

Adapting to the benchmark hardware is permitted:
- Setting worker count to match CPU cores
- Configuring connection pool sizes
- Adjusting memory limits for the container

The boundary is: **adapt to the environment, do not exploit it.**
