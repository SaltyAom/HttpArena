---
title: Validation
---

The mixed workload uses 7 endpoints. Subscribing to the `mixed` test automatically triggers validation for all of them, even if the individual tests are not listed in `meta.json`:

- `/baseline11` — [Baseline validation](../../../isolated/baseline/validation) (GET, POST, chunked POST, anti-cheat)
- `/json` — [JSON Processing validation](../../../isolated/json-processing/validation) (structure, totals, Content-Type)
- `/db` — [Database Query validation](../../../isolated/database/validation) (structure, Content-Type, empty range)
- `/upload` — [Upload validation](../../../isolated/upload/validation) (byte count, random anti-cheat)
- `/compression` — [Compression validation](../../../isolated/compression/validation) (Content-Encoding, content, size, per-request)
- `/static/*` — [Static Files validation](../../../isolated/static/validation) (Content-Types, file sizes, 404)
- `/async-db` — [Async Database validation](../../../isolated/async-database/validation) (structure, Content-Type, empty range)

The Postgres sidecar is started automatically when `mixed` is in the test list, and all required data volumes (`dataset-large.json`, `benchmark.db`, `static/`) are mounted.
