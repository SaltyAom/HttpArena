import gzip
import json
import os
import sqlite3
import threading
from contextlib import asynccontextmanager

import asyncpg
import orjson
from fastapi import FastAPI, Request, Response

# ── MIME types ────────────────────────────────────────────────────────
MIME_TYPES = {
    ".css": "text/css", ".js": "application/javascript", ".html": "text/html",
    ".woff2": "font/woff2", ".svg": "image/svg+xml", ".webp": "image/webp", ".json": "application/json",
}

# ── Pre-load static files ────────────────────────────────────────────
static_files: dict[str, tuple[bytes, str]] = {}
try:
    for name in os.listdir("/data/static"):
        filepath = os.path.join("/data/static", name)
        if os.path.isfile(filepath):
            with open(filepath, "rb") as f:
                data = f.read()
            ext = os.path.splitext(name)[1]
            ct = MIME_TYPES.get(ext, "application/octet-stream")
            static_files[name] = (data, ct)
except Exception:
    pass

# ── Postgres (async) ─────────────────────────────────────────────────
pg_pool: asyncpg.Pool | None = None
PG_QUERY = (
    "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count "
    "FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50"
)


@asynccontextmanager
async def lifespan(application: FastAPI):
    global pg_pool
    db_url = os.environ.get("DATABASE_URL")
    if db_url:
        try:
            # asyncpg needs postgresql:// not postgres://
            if db_url.startswith("postgres://"):
                db_url = "postgresql://" + db_url[len("postgres://"):]
            max_conn = int(os.environ.get("DATABASE_MAX_CONN", "256"))
            try:
                q, p = open('/sys/fs/cgroup/cpu.max').read().strip().split()
                num_workers = int(q) // int(p) if q != 'max' else len(os.sched_getaffinity(0))
            except Exception:
                num_workers = len(os.sched_getaffinity(0))
            pool_size = max(1, max_conn // num_workers)
            pg_pool = await asyncpg.create_pool(dsn=db_url, min_size=1, max_size=pool_size)
        except Exception:
            pg_pool = None
    yield
    if pg_pool is not None:
        await pg_pool.close()


app = FastAPI(lifespan=lifespan)

# ── Dataset ──────────────────────────────────────────────────────────
dataset_items = None
dataset_path = os.environ.get("DATASET_PATH", "/data/dataset.json")
try:
    with open(dataset_path) as f:
        dataset_items = json.load(f)
except Exception:
    pass

# Large dataset for compression (pre-serialised)
large_json_buf: bytes | None = None
try:
    with open("/data/dataset-large.json") as f:
        raw = json.load(f)
    items = []
    for d in raw:
        item = dict(d)
        item["total"] = round(d["price"] * d["quantity"] * 100) / 100
        items.append(item)
    large_json_buf = orjson.dumps({"items": items, "count": len(items)})
except Exception:
    pass

# ── SQLite (thread-local, sync — runs in threadpool via run_in_executor) ──
db_available = os.path.exists("/data/benchmark.db")
DB_QUERY = (
    "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count "
    "FROM items WHERE price BETWEEN ? AND ? LIMIT 50"
)
_local = threading.local()


def _get_db() -> sqlite3.Connection:
    conn = getattr(_local, "conn", None)
    if conn is None:
        conn = sqlite3.connect("/data/benchmark.db", uri=True)
        conn.execute("PRAGMA mmap_size=268435456")
        conn.row_factory = sqlite3.Row
        _local.conn = conn
    return conn


# ── Helpers ──────────────────────────────────────────────────────────
def _text(body: str | bytes, status: int = 200) -> Response:
    return Response(
        content=body,
        status_code=status,
        media_type="text/plain",
        headers={"Server": "fastapi"},
    )


def _json_resp(body: bytes, status: int = 200, extra_headers: dict | None = None) -> Response:
    headers = {"Server": "fastapi"}
    if extra_headers:
        headers.update(extra_headers)
    return Response(content=body, status_code=status, media_type="application/json", headers=headers)


# ── Routes ───────────────────────────────────────────────────────────
@app.get("/pipeline")
async def pipeline():
    return _text(b"ok")


@app.api_route("/baseline11", methods=["GET", "POST"])
async def baseline11(request: Request):
    total = 0
    for v in request.query_params.values():
        try:
            total += int(v)
        except ValueError:
            pass
    if request.method == "POST":
        body = await request.body()
        if body:
            try:
                total += int(body.strip())
            except ValueError:
                pass
    return _text(str(total))


@app.get("/baseline2")
async def baseline2(request: Request):
    total = 0
    for v in request.query_params.values():
        try:
            total += int(v)
        except ValueError:
            pass
    return _text(str(total))


@app.get("/json")
async def json_endpoint():
    if dataset_items is None:
        return _text("No dataset", 500)
    items = []
    for d in dataset_items:
        item = dict(d)
        item["total"] = round(d["price"] * d["quantity"] * 100) / 100
        items.append(item)
    body = orjson.dumps({"items": items, "count": len(items)})
    return _json_resp(body)


@app.get("/compression")
async def compression_endpoint():
    if large_json_buf is None:
        return _text("No dataset", 500)
    compressed = gzip.compress(large_json_buf, compresslevel=1)
    return _json_resp(compressed, extra_headers={"Content-Encoding": "gzip"})


@app.get("/db")
async def db_endpoint(request: Request):
    if not db_available:
        return _json_resp(b'{"items":[],"count":0}')
    min_val = float(request.query_params.get("min", 10))
    max_val = float(request.query_params.get("max", 50))
    conn = _get_db()
    rows = conn.execute(DB_QUERY, (min_val, max_val)).fetchall()
    items = []
    for r in rows:
        items.append(
            {
                "id": r["id"],
                "name": r["name"],
                "category": r["category"],
                "price": r["price"],
                "quantity": r["quantity"],
                "active": bool(r["active"]),
                "tags": json.loads(r["tags"]),
                "rating": {"score": r["rating_score"], "count": r["rating_count"]},
            }
        )
    body = orjson.dumps({"items": items, "count": len(items)})
    return _json_resp(body)


@app.get("/async-db")
async def async_db_endpoint(request: Request):
    if pg_pool is None:
        return _json_resp(b'{"items":[],"count":0}')
    min_val = float(request.query_params.get("min", 10))
    max_val = float(request.query_params.get("max", 50))
    try:
        rows = await pg_pool.fetch(PG_QUERY, min_val, max_val)
        items = []
        for r in rows:
            items.append(
                {
                    "id": r["id"],
                    "name": r["name"],
                    "category": r["category"],
                    "price": r["price"],
                    "quantity": r["quantity"],
                    "active": r["active"],
                    "tags": json.loads(r["tags"]) if isinstance(r["tags"], str) else r["tags"],
                    "rating": {"score": r["rating_score"], "count": r["rating_count"]},
                }
            )
        body = orjson.dumps({"items": items, "count": len(items)})
        return _json_resp(body)
    except Exception:
        return _json_resp(b'{"items":[],"count":0}')


@app.get("/static/{filename}")
async def static_file(filename: str):
    entry = static_files.get(filename)
    if entry is None:
        return _text(b"Not Found", 404)
    data, ct = entry
    return Response(content=data, status_code=200, media_type=ct, headers={"Server": "fastapi"})


@app.post("/upload")
async def upload_endpoint(request: Request):
    size = 0
    async for chunk in request.stream():
        size += len(chunk)
    return _text(str(size))
