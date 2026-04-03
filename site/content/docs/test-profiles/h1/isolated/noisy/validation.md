---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `noisy` test.

## Valid baseline request

Sends `GET /baseline11?a=13&b=42` and verifies the response body is `55`. Confirms the server handles valid requests correctly in the context of noise testing.

## Bad HTTP method

Sends a request with an invalid HTTP method (`GETT`) to `/baseline11?a=1&b=1`. Verifies the server returns a **4xx status code** (400 or 405). The server must not crash or return a 5xx error.

## Nonexistent path

Sends `GET /this/path/does/not/exist` and verifies the server returns **HTTP 404**.

## Post-noise recovery

After the noise requests above, sends another valid request with randomized parameters `GET /baseline11?a={random}&b={random}` and verifies the correct sum is returned. This confirms the server did not crash or enter a broken state after handling malformed traffic.
