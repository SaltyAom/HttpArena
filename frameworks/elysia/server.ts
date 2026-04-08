import { Elysia } from "elysia";
import { Database } from "bun:sqlite";
import { readFileSync } from "fs";

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

// Build route handlers as a reusable plugin
function addRoutes(app: Elysia) {
  return app
    .get("/pipeline", () => new Response("ok", { headers: { "content-type": "text/plain" } }))

    .all("/baseline11", async ({ query, request }) => {
      let querySum = 0;
      for (const v of Object.values(query))
        querySum += parseInt(v as string, 10) || 0;
      if (request.method === "POST") {
        const body = await request.text();
        let total = querySum;
        const n = parseInt(body.trim(), 10);
        if (!isNaN(n)) total += n;
        return new Response(String(total), { headers: { "content-type": "text/plain" } });
      }
      return new Response(String(querySum), { headers: { "content-type": "text/plain" } });
    })

    .get("/json", () => {
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
    })

    .get("/compression", ({ request }) => {
      const ae = request.headers.get("accept-encoding") || "";
      if (ae.includes("gzip")) {
        const compressed = Bun.gzipSync(largeJsonBuf, { level: 1 });
        return new Response(compressed, {
          headers: {
            "content-type": "application/json",
            "content-encoding": "gzip",
            "content-length": String(compressed.length),
          },
        });
      }
      return new Response(largeJsonBuf, {
        headers: {
          "content-type": "application/json",
          "content-length": String(largeJsonBuf.length),
        },
      });
    })

    .get("/db", ({ query }) => {
      if (!dbStmt) return new Response("DB not available", { status: 500 });
      try {
        const min = parseFloat(query.min as string) || 10;
        const max = parseFloat(query.max as string) || 50;
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
      } catch (e: any) {
        return new Response(e.message || "db error", { status: 500 });
      }
    })

    .get("/async-db", async ({ query }) => {
      if (!pgPool) {
        return new Response('{"items":[],"count":0}', {
          headers: { "content-type": "application/json" },
        });
      }
      const min = parseFloat(query.min as string) || 10;
      const max = parseFloat(query.max as string) || 50;
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
    })

    .post("/upload", async ({ request }) => {
      let size = 0;
      if (request.body) {
        for await (const chunk of request.body) {
          size += chunk.byteLength;
        }
      }
      return new Response(String(size), {
        headers: { "content-type": "text/plain" },
      });
    })

    .get("/static/:filename", async ({ params: { filename } }) => {
      const file = Bun.file(`${STATIC_DIR}/${filename}`);
      if (await file.exists()) {
        const ext = filename.slice(filename.lastIndexOf("."));
        return new Response(file, {
          headers: { "content-type": MIME_TYPES[ext] || "application/octet-stream" },
        });
      }
      return new Response("Not found", { status: 404 });
    })

    // Catch-all for unknown routes
    .all("*", () => new Response("Not found", { status: 404 }));
}

// HTTP server on port 8080
const httpApp = addRoutes(new Elysia());
httpApp.listen({ port: 8080, reusePort: true });
