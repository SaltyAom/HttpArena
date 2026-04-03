---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `mixed` test (which exercises the `/db` endpoint).

## Response structure

Sends `GET /db?min=10&max=50` and parses the JSON response. Verifies:

- **Item count** is between 1 and 50 (inclusive)
- Every item has a nested `rating` object with a `score` field
- Every item has a `tags` field that is an array (parsed from JSON string, not raw text)
- Every item has an `active` field that is a boolean (`true`/`false`, not integer `0`/`1`)

## Content-Type header

Sends `GET /db?min=10&max=50` and verifies the `Content-Type` response header is `application/json`.

## Anti-cheat: empty range

Sends `GET /db?min=9999&max=9999` (a price range with no matching items) and verifies the response has `count` equal to `0`. This detects hardcoded responses or implementations that ignore the query parameters.
