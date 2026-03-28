---
title: Implementation Rules
weight: 5
---

These rules exist to keep HttpArena results meaningful and representative of real-world framework performance. They apply to all **framework-type** submissions and serve as a reference during PR reviews.

{{< callout type="info" >}}
**Framework vs Engine entries:** HttpArena distinguishes between _framework_ entries (Express, Flask, Actix-web, etc.) and _engine_ entries (raw Node.js HTTP, CPython `http.server`, etc.). These rules apply to framework entries, where the goal is to measure what the framework gives you out of the box. Engine entries have more latitude since they _are_ the low-level layer — there is no higher-level API to bypass.
{{< /callout >}}

## Benchmark the framework as people use it

All of the rules below follow one principle: **benchmark the framework the way developers actually use it in production.** If a typical team shipping a production API wouldn't make a particular choice, it doesn't belong in an HttpArena submission.

## Use framework-level APIs

If a framework provides a documented, high-level way to accomplish a task, the benchmark implementation **must** use it.

Bypassing the framework to hand-roll a faster solution is not permitted. HttpArena measures framework performance, not custom low-level implementations.

**Example — route parameter parsing:**

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

**Why:** Users want to see how their framework performs with its own routing, serialization, and middleware. If the framework's built-in serializer is slow, that is valuable information. Bypassing it removes that signal and misrepresents actual framework performance.

## Settings must be production-documented

Non-default configuration is allowed **only if the framework's official production deployment guide recommends it**. If there is no official documentation recommending a setting for production use, it does not belong in the benchmark.

For example, you might want to adjust GC settings of Java or .NET applications as recommended in their production deployment guide, or set worker/thread counts to match available CPU cores.

**Not allowed:**
- Undocumented flags found by reading framework source code
- Experimental or unstable options that trade safety for speed
- Settings that disable buffering, validation, or error handling

Any non-default setting should be traceable to the framework's official production deployment documentation. If a reviewer asks for a reference, the submitter should be able to provide one.

## Use standard libraries and drivers

If the ecosystem has a well-established, production-grade library for a task (database driver, JSON serializer, HTTP client), use it. Bringing in an experimental or hand-rolled alternative solely because it performs better in microbenchmarks is not permitted.

**Example:** If every production application in a given language uses `libpq` bindings for Postgres, substituting an experimental zero-copy driver that nobody ships to production is not appropriate.

**Exception:** If the framework itself bundles or officially recommends a specific library, that library is acceptable.

## Deployment-environment tuning

Adapting to the benchmark hardware is permitted:

- Setting worker count to match CPU cores
- Configuring connection pool sizes
- Adjusting memory limits for the container

The boundary is: **adapt to the environment, don't exploit it.**
