---
title: Validation
---

The following checks are executed by `validate.sh` for every framework subscribed to the `json` test.

## Response structure and computed totals

Sends `GET /json` and parses the JSON response. Verifies:

- The response contains exactly **50 items**
- Every item has a `total` field
- Each `total` is correctly computed as `price * quantity`, rounded to 2 decimal places (tolerance: 0.01)

## Content-Type header

Sends `GET /json` and verifies the `Content-Type` response header is `application/json`.
