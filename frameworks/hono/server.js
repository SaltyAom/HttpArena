const cluster = require('cluster');
const os = require('os');
const fs = require('fs');
const http2 = require('http2');
const zlib = require('zlib');

const SERVER_NAME = 'hono';

let datasetItems;
let largeJsonBuf;
let dbStmt;
let pgPool;
const staticFiles = {};
const MIME_TYPES = {
    '.css': 'text/css', '.js': 'application/javascript', '.html': 'text/html',
    '.woff2': 'font/woff2', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.json': 'application/json'
};

function loadStaticFiles() {
    const dir = '/data/static';
    try {
        for (const name of fs.readdirSync(dir)) {
            const buf = fs.readFileSync(dir + '/' + name);
            const ext = name.slice(name.lastIndexOf('.'));
            staticFiles[name] = { buf, ct: MIME_TYPES[ext] || 'application/octet-stream' };
        }
    } catch (e) {}
}

function loadDataset() {
    const path = process.env.DATASET_PATH || '/data/dataset.json';
    try {
        datasetItems = JSON.parse(fs.readFileSync(path, 'utf8'));
    } catch (e) {}
}

function loadLargeDataset() {
    try {
        const raw = JSON.parse(fs.readFileSync('/data/dataset-large.json', 'utf8'));
        const items = raw.map(d => ({
            id: d.id, name: d.name, category: d.category,
            price: d.price, quantity: d.quantity, active: d.active,
            tags: d.tags, rating: d.rating,
            total: Math.round(d.price * d.quantity * 100) / 100
        }));
        largeJsonBuf = Buffer.from(JSON.stringify({ items, count: items.length }));
    } catch (e) {}
}

function loadDatabase() {
    try {
        const Database = require('better-sqlite3');
        const db = new Database('/data/benchmark.db', { readonly: true });
        db.pragma('mmap_size=268435456');
        dbStmt = db.prepare('SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50');
    } catch (e) {}
}

function loadPgPool() {
    const dbUrl = process.env.DATABASE_URL;
    if (!dbUrl) return;
    try {
        const { Pool } = require('pg');
        pgPool = new Pool({ connectionString: dbUrl, max: 4 });
    } catch (e) {}
}

function sumQuery(query) {
    let sum = 0;
    for (const key in query) {
        const n = parseInt(query[key], 10);
        if (n === n) sum += n;
    }
    return sum;
}

function parseQueryString(url) {
    const q = url.indexOf('?');
    if (q === -1) return {};
    const result = {};
    const qs = url.slice(q + 1);
    let i = 0;
    while (i < qs.length) {
        const eq = qs.indexOf('=', i);
        if (eq === -1) break;
        let amp = qs.indexOf('&', eq);
        if (amp === -1) amp = qs.length;
        result[decodeURIComponent(qs.slice(i, eq))] = decodeURIComponent(qs.slice(eq + 1, amp));
        i = amp + 1;
    }
    return result;
}

function startWorker() {
    loadDataset();
    loadLargeDataset();
    loadStaticFiles();
    loadDatabase();
    loadPgPool();

    const { Hono } = require('hono');
    const { serve } = require('@hono/node-server');

    const app = new Hono();

    // --- /static/:filename ---
    app.get('/static/:filename', (c) => {
        const sf = staticFiles[c.req.param('filename')];
        if (sf) {
            c.header('server', SERVER_NAME);
            c.header('content-type', sf.ct);
            c.header('content-length', String(sf.buf.length));
            return c.body(sf.buf);
        }
        return c.text('Not found', 404);
    });

    // --- /pipeline ---
    app.get('/pipeline', (c) => {
        c.header('server', SERVER_NAME);
        return c.text('ok');
    });

    // --- /baseline11 GET & POST ---
    app.get('/baseline11', (c) => {
        const query = parseQueryString(c.req.raw.url);
        const s = sumQuery(query);
        c.header('server', SERVER_NAME);
        return c.text(String(s));
    });

    app.post('/baseline11', async (c) => {
        const query = parseQueryString(c.req.raw.url);
        const querySum = sumQuery(query);
        const body = await c.req.text();
        let total = querySum;
        const n = parseInt(body.trim(), 10);
        if (n === n) total += n;
        c.header('server', SERVER_NAME);
        return c.text(String(total));
    });

    // --- /baseline2 ---
    app.get('/baseline2', (c) => {
        const query = parseQueryString(c.req.raw.url);
        const s = sumQuery(query);
        c.header('server', SERVER_NAME);
        return c.text(String(s));
    });

    // --- /json ---
    app.get('/json', (c) => {
        if (!datasetItems) {
            return c.text('No dataset', 500);
        }
        const items = datasetItems.map(d => ({
            id: d.id, name: d.name, category: d.category,
            price: d.price, quantity: d.quantity, active: d.active,
            tags: d.tags, rating: d.rating,
            total: Math.round(d.price * d.quantity * 100) / 100
        }));
        const buf = Buffer.from(JSON.stringify({ items, count: items.length }));
        c.header('server', SERVER_NAME);
        c.header('content-type', 'application/json');
        c.header('content-length', String(buf.length));
        return c.body(buf);
    });

    // --- /compression ---
    app.get('/compression', (c) => {
        if (!largeJsonBuf) {
            return c.text('No dataset', 500);
        }
        const compressed = zlib.gzipSync(largeJsonBuf, { level: 1 });
        c.header('server', SERVER_NAME);
        c.header('content-type', 'application/json');
        c.header('content-encoding', 'gzip');
        c.header('content-length', String(compressed.length));
        return c.body(compressed);
    });

    // --- /db ---
    app.get('/db', (c) => {
        if (!dbStmt) {
            c.header('server', SERVER_NAME);
            c.header('content-type', 'application/json');
            return c.body('{"items":[],"count":0}');
        }
        const query = parseQueryString(c.req.raw.url);
        let min = 10, max = 50;
        if (query.min) min = parseFloat(query.min) || 10;
        if (query.max) max = parseFloat(query.max) || 50;
        const rows = dbStmt.all(min, max);
        const items = rows.map(r => ({
            id: r.id, name: r.name, category: r.category,
            price: r.price, quantity: r.quantity, active: r.active === 1,
            tags: JSON.parse(r.tags),
            rating: { score: r.rating_score, count: r.rating_count }
        }));
        const body = JSON.stringify({ items, count: items.length });
        c.header('server', SERVER_NAME);
        c.header('content-type', 'application/json');
        c.header('content-length', String(Buffer.byteLength(body)));
        return c.body(body);
    });

    // --- /async-db ---
    app.get('/async-db', async (c) => {
        if (!pgPool) {
            c.header('server', SERVER_NAME);
            c.header('content-type', 'application/json');
            return c.body('{"items":[],"count":0}');
        }
        const query = parseQueryString(c.req.raw.url);
        let min = 10, max = 50;
        if (query.min) min = parseFloat(query.min) || 10;
        if (query.max) max = parseFloat(query.max) || 50;
        try {
            const result = await pgPool.query(
                'SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50',
                [min, max]
            );
            const items = result.rows.map(r => ({
                id: r.id, name: r.name, category: r.category,
                price: r.price, quantity: r.quantity, active: r.active,
                tags: r.tags,
                rating: { score: r.rating_score, count: r.rating_count }
            }));
            const body = JSON.stringify({ items, count: items.length });
            c.header('server', SERVER_NAME);
            c.header('content-type', 'application/json');
            c.header('content-length', String(Buffer.byteLength(body)));
            return c.body(body);
        } catch (e) {
            c.header('server', SERVER_NAME);
            c.header('content-type', 'application/json');
            return c.body('{"items":[],"count":0}');
        }
    });

    // --- /upload ---
    app.post('/upload', async (c) => {
        let size = 0;
        const reader = c.req.raw.body?.getReader();
        if (reader) {
            while (true) {
                const { done, value } = await reader.read();
                if (done) break;
                size += value.byteLength;
            }
        }
        c.header('server', SERVER_NAME);
        return c.text(String(size));
    });

    // Start HTTP/1.1 via @hono/node-server
    serve({
        fetch: app.fetch,
        port: 8080,
        hostname: '0.0.0.0',
    }, () => {
        startH2();
    });
}

function startH2() {
    const certFile = process.env.TLS_CERT || '/certs/server.crt';
    const keyFile = process.env.TLS_KEY || '/certs/server.key';
    try {
        const opts = {
            cert: fs.readFileSync(certFile),
            key: fs.readFileSync(keyFile),
            allowHTTP1: false,
        };
        const h2server = http2.createSecureServer(opts, (req, res) => {
            const url = req.url;
            const q = url.indexOf('?');
            const p = q === -1 ? url : url.slice(0, q);
            if (p.startsWith('/static/')) {
                const name = p.slice(8);
                const sf = staticFiles[name];
                if (sf) {
                    res.writeHead(200, { 'content-type': sf.ct, 'content-length': sf.buf.length, 'server': SERVER_NAME });
                    res.end(sf.buf);
                } else {
                    res.writeHead(404);
                    res.end();
                }
            } else {
                let sum = 0;
                if (q !== -1) {
                    const qs = url.slice(q + 1);
                    let i = 0;
                    while (i < qs.length) {
                        const eq = qs.indexOf('=', i);
                        if (eq === -1) break;
                        let amp = qs.indexOf('&', eq);
                        if (amp === -1) amp = qs.length;
                        const n = parseInt(qs.slice(eq + 1, amp), 10);
                        if (n === n) sum += n;
                        i = amp + 1;
                    }
                }
                res.writeHead(200, { 'content-type': 'text/plain', 'server': SERVER_NAME });
                res.end(String(sum));
            }
        });
        h2server.listen(8443);
    } catch (e) {}
}

function getCPUCount() {
    try {
        const max = require('fs').readFileSync('/sys/fs/cgroup/cpu.max', 'utf8').trim();
        const [quota, period] = max.split(' ');
        if (quota !== 'max') {
            const cgroup = Math.floor(Number(quota) / Number(period));
            if (cgroup >= 1) return cgroup;
        }
    } catch {}
    return os.availableParallelism ? os.availableParallelism() : os.cpus().length;
}

if (cluster.isPrimary) {
    const numCPUs = getCPUCount();
    for (let i = 0; i < numCPUs; i++) cluster.fork();
} else {
    startWorker();
}
