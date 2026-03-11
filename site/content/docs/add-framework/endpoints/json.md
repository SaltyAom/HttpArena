---
title: JSON Processing
---

Required for the `json` and `mixed` profiles.

## `GET /json`

Load `/data/dataset.json` at startup (50 items). For each request, compute `total = price * quantity` (rounded to 2 decimals) for every item and return:

```json
{
  "items": [
    {"id": 1, "name": "Alpha Widget", "category": "electronics", "price": 29.99, "quantity": 5, "active": true, "tags": ["fast", "new"], "rating": {"score": 4.2, "count": 127}, "total": 149.95}
  ],
  "count": 50
}
```

Response must have `Content-Type: application/json`.
