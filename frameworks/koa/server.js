const cluster = require('cluster');
const os = require('os');
const fs = require('fs');
const http2 = require('http2');
const zlib = require('zlib');

const SERVER_NAME = 'koa';

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
    if (query) {
        for (const key of Object.keys(query)) {
            const n = parseInt(query[key], 10);
            if (n === n) sum += n;
        }
    }
    return sum;
}

function readBody(ctx) {
    return new Promise((resolve, reject) => {
        const chunks = [];
        ctx.req.on('data', c => chunks.push(c));
        ctx.req.on('end', () => resolve(Buffer.concat(chunks)));
        ctx.req.on('error', reject);
    });
}

function startWorker() {
    loadDataset();
    loadLargeDataset();
    loadStaticFiles();
    loadDatabase();
    loadPgPool();

    const Koa = require('koa');
    const Router = require('koa-router');

    const app = new Koa();
    app.proxy = false;

    const router = new Router();

    // --- /static/:filename ---
    router.get('/static/:filename', (ctx) => {
        const sf = staticFiles[ctx.params.filename];
        if (sf) {
            ctx.set('server', SERVER_NAME);
            ctx.set('content-type', sf.ct);
            ctx.set('content-length', String(sf.buf.length));
            ctx.body = sf.buf;
        } else {
            ctx.status = 404;
        }
    });

    // --- /pipeline ---
    router.get('/pipeline', (ctx) => {
        ctx.set('server', SERVER_NAME);
        ctx.type = 'text/plain';
        ctx.body = 'ok';
    });

    // --- /baseline11 GET & POST ---
    router.get('/baseline11', (ctx) => {
        const s = sumQuery(ctx.query);
        ctx.set('server', SERVER_NAME);
        ctx.type = 'text/plain';
        ctx.body = String(s);
    });

    router.post('/baseline11', async (ctx) => {
        const querySum = sumQuery(ctx.query);
        const body = (await readBody(ctx)).toString();
        let total = querySum;
        const n = parseInt(body.trim(), 10);
        if (n === n) total += n;
        ctx.set('server', SERVER_NAME);
        ctx.type = 'text/plain';
        ctx.body = String(total);
    });

    // --- /baseline2 ---
    router.get('/baseline2', (ctx) => {
        const s = sumQuery(ctx.query);
        ctx.set('server', SERVER_NAME);
        ctx.type = 'text/plain';
        ctx.body = String(s);
    });

    // --- /json ---
    router.get('/json', (ctx) => {
        if (!datasetItems) {
            ctx.status = 500;
            ctx.body = 'No dataset';
            return;
        }
        const items = datasetItems.map(d => ({
            id: d.id, name: d.name, category: d.category,
            price: d.price, quantity: d.quantity, active: d.active,
            tags: d.tags, rating: d.rating,
            total: Math.round(d.price * d.quantity * 100) / 100
        }));
        const body = JSON.stringify({ items, count: items.length });
        ctx.set('server', SERVER_NAME);
        ctx.set('content-type', 'application/json');
        ctx.set('content-length', String(Buffer.byteLength(body)));
        ctx.body = body;
    });

    // --- /compression ---
    router.get('/compression', (ctx) => {
        if (!largeJsonBuf) {
            ctx.status = 500;
            ctx.body = 'No dataset';
            return;
        }
        const compressed = zlib.gzipSync(largeJsonBuf, { level: 1 });
        ctx.set('server', SERVER_NAME);
        ctx.set('content-type', 'application/json');
        ctx.set('content-encoding', 'gzip');
        ctx.set('content-length', String(compressed.length));
        ctx.body = compressed;
    });

    // --- /db ---
    router.get('/db', (ctx) => {
        if (!dbStmt) {
            ctx.set('server', SERVER_NAME);
            ctx.type = 'application/json';
            ctx.body = '{"items":[],"count":0}';
            return;
        }
        let min = 10, max = 50;
        if (ctx.query.min) min = parseFloat(ctx.query.min) || 10;
        if (ctx.query.max) max = parseFloat(ctx.query.max) || 50;
        const rows = dbStmt.all(min, max);
        const items = rows.map(r => ({
            id: r.id, name: r.name, category: r.category,
            price: r.price, quantity: r.quantity, active: r.active === 1,
            tags: JSON.parse(r.tags),
            rating: { score: r.rating_score, count: r.rating_count }
        }));
        const body = JSON.stringify({ items, count: items.length });
        ctx.set('server', SERVER_NAME);
        ctx.set('content-type', 'application/json');
        ctx.set('content-length', String(Buffer.byteLength(body)));
        ctx.body = body;
    });

    // --- /async-db ---
    router.get('/async-db', async (ctx) => {
        if (!pgPool) {
            ctx.set('server', SERVER_NAME);
            ctx.type = 'application/json';
            ctx.body = '{"items":[],"count":0}';
            return;
        }
        let min = 10, max = 50;
        if (ctx.query.min) min = parseFloat(ctx.query.min) || 10;
        if (ctx.query.max) max = parseFloat(ctx.query.max) || 50;
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
            ctx.set('server', SERVER_NAME);
            ctx.set('content-type', 'application/json');
            ctx.set('content-length', String(Buffer.byteLength(body)));
            ctx.body = body;
        } catch (e) {
            ctx.set('server', SERVER_NAME);
            ctx.type = 'application/json';
            ctx.body = '{"items":[],"count":0}';
        }
    });

    // --- /upload ---
    router.post('/upload', async (ctx) => {
        let size = 0;
        await new Promise((resolve, reject) => {
            ctx.req.on('data', chunk => { size += chunk.length; });
            ctx.req.on('end', resolve);
            ctx.req.on('error', reject);
        });
        ctx.set('server', SERVER_NAME);
        ctx.type = 'text/plain';
        ctx.body = String(size);
    });

    app.use(router.routes());
    app.use(router.allowedMethods());

    app.listen(8080, '0.0.0.0', () => {
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
