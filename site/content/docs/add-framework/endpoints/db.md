---
title: Database
---

Required for the `db` and `mixed` profiles.

## `GET /db?min=N&max=N`

Open `/data/benchmark.db` (a SQLite database with 100K rows) at startup in read-only mode. For each request, query items where `price BETWEEN min AND max` (default 10–50), limit 50 rows.

Return JSON with parsed tags and structured rating:

```json
{
  "items": [
    {"id": 1, "name": "Widget", "category": "electronics", "price": 29.99, "quantity": 5, "active": true, "tags": ["fast"], "rating": {"score": 4.2, "count": 127}}
  ],
  "count": 42
}
```

The `tags` column is stored as a JSON string in the database — parse it into an array. The `active` column is an integer (0/1) — return it as a boolean. The `rating_score` and `rating_count` columns should be returned as a nested `rating` object.

Response must have `Content-Type: application/json`.
