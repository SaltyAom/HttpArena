---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `baseline` or `limited-conn` test.

## GET with query parameters

Sends `GET /baseline11?a=13&b=42` and verifies the response body is `55` (sum of `a` and `b`).

## POST with Content-Length body

Sends `POST /baseline11?a=13&b=42` with body `20` and `Content-Type: text/plain`. Verifies the response body is `75` (sum of `a`, `b`, and body).

## POST with chunked Transfer-Encoding

Sends `POST /baseline11?a=13&b=42` with body `20` and `Transfer-Encoding: chunked`. Verifies the response body is `75`.

## Anti-cheat: randomized query parameters

Generates random values for `a` and `b` (100-999), sends `GET /baseline11?a={a}&b={b}`, and verifies the response matches the expected sum. This detects hardcoded responses.

## Anti-cheat: POST body cache detection

Sends two POST requests with different random body values to the same endpoint (`/baseline11?a=13&b=42`). Verifies each response reflects the correct sum for that specific body. This detects response caching or hardcoded POST handling.

- Request 1: body=`{random1}` — expects `13 + 42 + random1`
- Request 2: body=`{random2}` — expects `13 + 42 + random2`
