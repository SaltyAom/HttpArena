---
title: Pipeline
---

Required for the `pipelined` profile.

## `GET /pipeline`

Return a fixed `ok` response (exactly 2 bytes, `text/plain`). Used for the pipelined benchmark — should be as lightweight as possible.

Response must have `Content-Type: text/plain`.
