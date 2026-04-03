const http = require('http');
const http2 = require('http2');
const cluster = require('cluster');
const os = require('os');
const fs = require('fs');

const zlib = require('zlib');
const Database = require('better-sqlite3');

const SERVER_HEADERS = { 'server': 'node' };

// Raw dataset for per-request JSON processing
let datasetItems;

// Pre-serialized large dataset for compression endpoint
let largeJsonBuf;

// SQLite prepared statement (per-worker process)
let dbStmt;

// PostgreSQL pool (per-worker process)
let pgPool;

// Pre-loaded static files
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

function sumQuery(url) {
    const q = url.indexOf('?');
    if (q === -1) return 0;
    let sum = 0;
    const qs = url.slice(q + 1);
    let i = 0;
    while (i < qs.length) {
        const eq = qs.indexOf('=', i);
        if (eq === -1) break;
        let amp = qs.indexOf('&', eq);
        if (amp === -1) amp = qs.length;
        const n = parseInt(qs.slice(eq + 1, amp), 10);
        if (n === n) sum += n; // NaN check
        i = amp + 1;
    }
    return sum;
}

const server = http.createServer((req, res) => {
    const url = req.url;
    const q = url.indexOf('?');
    const path = q === -1 ? url : url.slice(0, q);

    if (path === '/pipeline') {
        res.writeHead(200, { 'content-type': 'text/plain', ...SERVER_HEADERS });
        res.end('ok');
    } else if (path === '/json') {
        if (datasetItems) {
            const items = datasetItems.map(d => ({
                id: d.id, name: d.name, category: d.category,
                price: d.price, quantity: d.quantity, active: d.active,
                tags: d.tags, rating: d.rating,
                total: Math.round(d.price * d.quantity * 100) / 100
            }));
            const buf = Buffer.from(JSON.stringify({ items, count: items.length }));
            res.writeHead(200, {
                'content-type': 'application/json',
                'content-length': buf.length,
                ...SERVER_HEADERS
            });
            res.end(buf);
        } else {
            res.writeHead(500);
            res.end('No dataset');
        }
    } else if (path === '/compression') {
        if (largeJsonBuf) {
            const compressed = zlib.gzipSync(largeJsonBuf, { level: 1 });
            res.writeHead(200, {
                'content-type': 'application/json',
                'content-encoding': 'gzip',
                'content-length': compressed.length,
                ...SERVER_HEADERS
            });
            res.end(compressed);
        } else {
            res.writeHead(500);
            res.end('No dataset');
        }
    } else if (path === '/db') {
        if (!dbStmt) {
            res.writeHead(200, { 'content-type': 'application/json', ...SERVER_HEADERS });
            res.end('{"items":[],"count":0}');
        } else {
            let min = 10, max = 50;
            if (q !== -1) {
                const qs = url.slice(q + 1);
                for (const pair of qs.split('&')) {
                    const eq = pair.indexOf('=');
                    if (eq === -1) continue;
                    const k = pair.slice(0, eq), v = pair.slice(eq + 1);
                    if (k === 'min') min = parseFloat(v) || 10;
                    else if (k === 'max') max = parseFloat(v) || 50;
                }
            }
            const rows = dbStmt.all(min, max);
            const items = rows.map(r => ({
                id: r.id, name: r.name, category: r.category,
                price: r.price, quantity: r.quantity, active: r.active === 1,
                tags: JSON.parse(r.tags),
                rating: { score: r.rating_score, count: r.rating_count }
            }));
            const body = JSON.stringify({ items, count: items.length });
            res.writeHead(200, {
                'content-type': 'application/json',
                'content-length': Buffer.byteLength(body),
                ...SERVER_HEADERS
            });
            res.end(body);
        }
    } else if (path === '/async-db') {
        if (!pgPool) {
            res.writeHead(200, { 'content-type': 'application/json', ...SERVER_HEADERS });
            res.end('{"items":[],"count":0}');
        } else {
            let min = 10, max = 50;
            if (q !== -1) {
                const qs = url.slice(q + 1);
                for (const pair of qs.split('&')) {
                    const eq = pair.indexOf('=');
                    if (eq === -1) continue;
                    const k = pair.slice(0, eq), v = pair.slice(eq + 1);
                    if (k === 'min') min = parseFloat(v) || 10;
                    else if (k === 'max') max = parseFloat(v) || 50;
                }
            }
            pgPool.query(
                'SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50',
                [min, max]
            ).then(result => {
                const items = result.rows.map(r => ({
                    id: r.id, name: r.name, category: r.category,
                    price: r.price, quantity: r.quantity, active: r.active,
                    tags: r.tags,
                    rating: { score: r.rating_score, count: r.rating_count }
                }));
                const body = JSON.stringify({ items, count: items.length });
                res.writeHead(200, {
                    'content-type': 'application/json',
                    'content-length': Buffer.byteLength(body),
                    ...SERVER_HEADERS
                });
                res.end(body);
            }).catch(() => {
                res.writeHead(200, { 'content-type': 'application/json', ...SERVER_HEADERS });
                res.end('{"items":[],"count":0}');
            });
        }
    } else if (path === '/baseline2') {
        const body = String(sumQuery(url));
        res.writeHead(200, { 'content-type': 'text/plain', ...SERVER_HEADERS });
        res.end(body);
    } else if (path === '/upload' && req.method === 'POST') {
        let size = 0;
        req.on('data', chunk => size += chunk.length);
        req.on('end', () => {
            res.writeHead(200, { 'content-type': 'text/plain', ...SERVER_HEADERS });
            res.end(String(size));
        });
    } else if (path.startsWith('/static/')) {
        const name = path.slice(8);
        const sf = staticFiles[name];
        if (sf) {
            res.writeHead(200, { 'content-type': sf.ct, 'content-length': sf.buf.length, ...SERVER_HEADERS });
            res.end(sf.buf);
        } else {
            res.writeHead(404);
            res.end();
        }
    } else {
        // /baseline11 — GET or POST
        const querySum = sumQuery(url);
        if (req.method === 'POST') {
            let body = '';
            req.on('data', chunk => body += chunk);
            req.on('end', () => {
                let total = querySum;
                const n = parseInt(body.trim(), 10);
                if (n === n) total += n;
                res.writeHead(200, { 'content-type': 'text/plain', ...SERVER_HEADERS });
                res.end(String(total));
            });
        } else {
            res.writeHead(200, { 'content-type': 'text/plain', ...SERVER_HEADERS });
            res.end(String(querySum));
        }
    }
});

server.keepAliveTimeout = 0;

// HTTP/2 TLS server on port 8443
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
                    res.writeHead(200, { 'content-type': sf.ct, 'content-length': sf.buf.length, 'server': 'node' });
                    res.end(sf.buf);
                } else {
                    res.writeHead(404);
                    res.end();
                }
            } else {
                const sum = sumQuery(url);
                res.writeHead(200, { 'content-type': 'text/plain', 'server': 'node' });
                res.end(String(sum));
            }
        });
        h2server.listen(8443);
    } catch (e) {
        // TLS certs not available, skip H2
    }
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
    loadDataset();
    loadLargeDataset();
    loadStaticFiles();
    loadDatabase();
    loadPgPool();
    server.listen(8080);
    startH2();
}
