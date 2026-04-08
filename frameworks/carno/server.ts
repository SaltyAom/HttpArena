import "reflect-metadata";
import { Carno, Controller, Get, Post, Ctx, Context, Use, CompressionMiddleware } from "@carno.js/core";
import { Database } from "bun:sqlite";
import { readFileSync } from "fs";

// Inject reusePort so multiple workers can share port 8080
const _serve = Bun.serve.bind(Bun);
Bun.serve = (opts: any) => _serve({ ...opts, reusePort: true });

const MIME_TYPES: Record<string, string> = {
  ".css": "text/css", ".js": "application/javascript", ".html": "text/html",
  ".woff2": "font/woff2", ".svg": "image/svg+xml", ".webp": "image/webp", ".json": "application/json",
};

// Load datasets
const datasetItems: any[] = JSON.parse(readFileSync("/data/dataset.json", "utf8"));

const largeData = JSON.parse(readFileSync("/data/dataset-large.json", "utf8"));
const largeItems = largeData.map((d: any) => ({
  id: d.id, name: d.name, category: d.category,
  price: d.price, quantity: d.quantity, active: d.active,
  tags: d.tags, rating: d.rating,
  total: Math.round(d.price * d.quantity * 100) / 100,
}));
const largeJsonBuf = Buffer.from(JSON.stringify({ items: largeItems, count: largeItems.length }));

// Open SQLite database read-only
let dbStmt: any = null;
for (let attempt = 0; attempt < 3 && !dbStmt; attempt++) {
  try {
    const db = new Database("/data/benchmark.db", { readonly: true });
    db.exec("PRAGMA mmap_size=268435456");
    dbStmt = db.prepare("SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50");
  } catch (e) {
    console.error(`SQLite open attempt ${attempt + 1} failed:`, e);
    if (attempt < 2) Bun.sleepSync(50);
  }
}

// PostgreSQL pool for async-db
let pgPool: any = null;
{
  const dbUrl = process.env.DATABASE_URL;
  if (dbUrl) {
    try {
      const { Pool } = require("pg");
      pgPool = new Pool({ connectionString: dbUrl, max: 4 });
    } catch (_) {}
  }
}

const STATIC_DIR = "/data/static";

const gzipMiddleware = new CompressionMiddleware({
  threshold: 0,
  encodings: ["gzip"],
  gzipLevel: 1,
});

@Controller()
class BenchController {
  @Get("/pipeline")
  pipeline() {
    return new Response("ok", { headers: { "content-type": "text/plain" } });
  }

  @Get("/baseline11")
  baselineGet(@Ctx() ctx: Context) {
    const query = ctx.query;
    let sum = 0;
    for (const v of Object.values(query))
      sum += parseInt(v as string, 10) || 0;
    return new Response(String(sum), { headers: { "content-type": "text/plain" } });
  }

  @Post("/baseline11")
  async baselinePost(@Ctx() ctx: Context) {
    const query = ctx.query;
    let querySum = 0;
    for (const v of Object.values(query))
      querySum += parseInt(v as string, 10) || 0;
    const body = await ctx.req.text();
    let total = querySum;
    const n = parseInt(body.trim(), 10);
    if (!isNaN(n)) total += n;
    return new Response(String(total), { headers: { "content-type": "text/plain" } });
  }

  @Get("/json")
  json() {
    const items = datasetItems.map((d: any) => ({
      id: d.id, name: d.name, category: d.category,
      price: d.price, quantity: d.quantity, active: d.active,
      tags: d.tags, rating: d.rating,
      total: Math.round(d.price * d.quantity * 100) / 100,
    }));
    const body = JSON.stringify({ items, count: items.length });
    return new Response(body, {
      headers: { "content-type": "application/json", "content-length": String(Buffer.byteLength(body)) },
    });
  }

  @Get("/compression")
  @Use(gzipMiddleware)
  compression() {
    return new Response(largeJsonBuf, {
      headers: {
        "content-type": "application/json",
        "content-length": String(largeJsonBuf.length),
      },
    });
  }

  @Get("/db")
  db(@Ctx() ctx: Context) {
    if (!dbStmt) return new Response("DB not available", { status: 500 });
    const min = parseFloat(ctx.query.min as string) || 10;
    const max = parseFloat(ctx.query.max as string) || 50;
    const rows = dbStmt.all(min, max) as any[];
    const items = rows.map((r: any) => ({
      id: r.id, name: r.name, category: r.category,
      price: r.price, quantity: r.quantity, active: r.active === 1,
      tags: JSON.parse(r.tags),
      rating: { score: r.rating_score, count: r.rating_count },
    }));
    const body = JSON.stringify({ items, count: items.length });
    return new Response(body, {
      headers: { "content-type": "application/json", "content-length": String(Buffer.byteLength(body)) },
    });
  }

  @Get("/async-db")
  async asyncDb(@Ctx() ctx: Context) {
    if (!pgPool) {
      return new Response('{"items":[],"count":0}', {
        headers: { "content-type": "application/json" },
      });
    }
    const min = parseFloat(ctx.query.min as string) || 10;
    const max = parseFloat(ctx.query.max as string) || 50;
    try {
      const result = await pgPool.query(
        "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50",
        [min, max]
      );
      const items = result.rows.map((r: any) => ({
        id: r.id, name: r.name, category: r.category,
        price: r.price, quantity: r.quantity, active: r.active,
        tags: r.tags,
        rating: { score: r.rating_score, count: r.rating_count },
      }));
      const body = JSON.stringify({ items, count: items.length });
      return new Response(body, {
        headers: { "content-type": "application/json", "content-length": String(Buffer.byteLength(body)) },
      });
    } catch (e) {
      return new Response('{"items":[],"count":0}', {
        headers: { "content-type": "application/json" },
      });
    }
  }

  @Post("/upload")
  async upload(@Ctx() ctx: Context) {
    let size = 0;
    const body = ctx.req.body;
    if (body) {
      for await (const chunk of body) {
        size += chunk.byteLength;
      }
    }
    return new Response(String(size), { headers: { "content-type": "text/plain" } });
  }

  @Get("/static/:filename")
  async staticFile(@Ctx() ctx: Context) {
    const filename = ctx.params.filename;
    const file = Bun.file(`${STATIC_DIR}/${filename}`);
    if (await file.exists()) {
      const ext = filename.slice(filename.lastIndexOf("."));
      return new Response(file, {
        headers: { "content-type": MIME_TYPES[ext] || "application/octet-stream" },
      });
    }
    return new Response("Not found", { status: 404 });
  }
}

const app = new Carno({ disableStartupLog: true, validation: false });
app.controllers([BenchController]);
app.listen(8080);

console.log("Carno.js running on port 8080");
