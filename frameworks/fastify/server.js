const cluster = require('cluster');
const os = require('os');
const fs = require('fs');
const zlib = require('zlib');

const SERVER_NAME = 'fastify';

// --- Shared data (loaded per-worker) ---
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

function startWorker() {
    loadDataset();
    loadLargeDataset();
    loadStaticFiles();
    loadDatabase();
    loadPgPool();

    const Fastify = require('fastify');
    const app = Fastify({ logger: false, bodyLimit: 50 * 1024 * 1024 });

    // Register raw body parsers so req.body is available without manual stream reading
    app.addContentTypeParser('text/plain', { parseAs: 'string' }, (req, body, done) => done(null, body));
    app.addContentTypeParser('application/octet-stream', { parseAs: 'buffer' }, (req, body, done) => done(null, body));
    app.addContentTypeParser('*', { parseAs: 'buffer' }, (req, body, done) => done(null, body));

    // Register shared routes (baseline, static)
    registerSharedRoutes(app);

    // --- /pipeline ---
    app.get('/pipeline', (req, reply) => {
        reply.header('server', SERVER_NAME).type('text/plain').send('ok');
    });

    // --- /json ---
    app.get('/json', (req, reply) => {
        if (!datasetItems) {
            reply.code(500).send('No dataset');
            return;
        }
        const items = datasetItems.map(d => ({
            id: d.id, name: d.name, category: d.category,
            price: d.price, quantity: d.quantity, active: d.active,
            tags: d.tags, rating: d.rating,
            total: Math.round(d.price * d.quantity * 100) / 100
        }));
        const buf = Buffer.from(JSON.stringify({ items, count: items.length }));
        reply
            .header('server', SERVER_NAME)
            .header('content-type', 'application/json')
            .header('content-length', buf.length)
            .send(buf);
    });

    // --- /compression ---
    app.get('/compression', (req, reply) => {
        if (!largeJsonBuf) {
            reply.code(500).send('No dataset');
            return;
        }
        const compressed = zlib.gzipSync(largeJsonBuf, { level: 1 });
        reply
            .header('server', SERVER_NAME)
            .header('content-type', 'application/json')
            .header('content-encoding', 'gzip')
            .header('content-length', compressed.length)
            .send(compressed);
    });

    // --- /db ---
    app.get('/db', (req, reply) => {
        if (!dbStmt) {
            reply.header('server', SERVER_NAME).type('application/json').send('{"items":[],"count":0}');
            return;
        }
        let min = 10, max = 50;
        if (req.query.min) min = parseFloat(req.query.min) || 10;
        if (req.query.max) max = parseFloat(req.query.max) || 50;
        const rows = dbStmt.all(min, max);
        const items = rows.map(r => ({
            id: r.id, name: r.name, category: r.category,
            price: r.price, quantity: r.quantity, active: r.active === 1,
            tags: JSON.parse(r.tags),
            rating: { score: r.rating_score, count: r.rating_count }
        }));
        const body = JSON.stringify({ items, count: items.length });
        reply
            .header('server', SERVER_NAME)
            .header('content-type', 'application/json')
            .header('content-length', Buffer.byteLength(body))
            .send(body);
    });

    // --- /async-db ---
    app.get('/async-db', async (req, reply) => {
        if (!pgPool) {
            reply.header('server', SERVER_NAME).type('application/json').send('{"items":[],"count":0}');
            return;
        }
        let min = 10, max = 50;
        if (req.query.min) min = parseFloat(req.query.min) || 10;
        if (req.query.max) max = parseFloat(req.query.max) || 50;
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
            reply
                .header('server', SERVER_NAME)
                .header('content-type', 'application/json')
                .header('content-length', Buffer.byteLength(body))
                .send(body);
        } catch (e) {
            reply.header('server', SERVER_NAME).type('application/json').send('{"items":[],"count":0}');
        }
    });

    // --- /upload --- (streaming: bypass Fastify body parsing via encapsulated plugin)
    app.register(function (instance, opts, done) {
        instance.removeAllContentTypeParsers();
        instance.addContentTypeParser('*', function (request, payload, done) {
            done(null);
        });
        instance.post('/upload', (req, reply) => {
            let size = 0;
            req.raw.on('data', chunk => { size += chunk.length; });
            req.raw.on('end', () => {
                reply.header('server', SERVER_NAME).type('text/plain').send(String(size));
            });
        });
        done();
    });

    // Start HTTP/1.1 server
    app.listen({ port: 8080, host: '0.0.0.0' }).then(() => {
        // Also start HTTP/2 server on 8443
        startH2();
    });
}

// --- Shared route handlers (used by both H1 and H2 instances) ---
function registerSharedRoutes(app) {
    app.get('/static/:filename', (req, reply) => {
        const sf = staticFiles[req.params.filename];
        if (sf) {
            reply
                .header('server', SERVER_NAME)
                .header('content-type', sf.ct)
                .header('content-length', sf.buf.length)
                .send(sf.buf);
        } else {
            reply.code(404).send();
        }
    });

    app.get('/baseline11', (req, reply) => {
        const s = sumQuery(req.query);
        reply.header('server', SERVER_NAME).type('text/plain').send(String(s));
    });

    app.post('/baseline11', (req, reply) => {
        const querySum = sumQuery(req.query);
        const body = typeof req.body === 'string' ? req.body : (req.body ? req.body.toString() : '');
        let total = querySum;
        const n = parseInt(body.trim(), 10);
        if (n === n) total += n;
        reply.header('server', SERVER_NAME).type('text/plain').send(String(total));
    });

    app.get('/baseline2', (req, reply) => {
        const s = sumQuery(req.query);
        reply.header('server', SERVER_NAME).type('text/plain').send(String(s));
    });
}

function startH2() {
    const certFile = process.env.TLS_CERT || '/certs/server.crt';
    const keyFile = process.env.TLS_KEY || '/certs/server.key';
    try {
        const Fastify = require('fastify');
        const h2app = Fastify({
            logger: false,
            http2: true,
            https: {
                cert: fs.readFileSync(certFile),
                key: fs.readFileSync(keyFile),
                allowHTTP1: false,
            },
            bodyLimit: 50 * 1024 * 1024,
        });

        h2app.addContentTypeParser('text/plain', { parseAs: 'string' }, (req, body, done) => done(null, body));
        h2app.addContentTypeParser('application/octet-stream', { parseAs: 'buffer' }, (req, body, done) => done(null, body));
        h2app.addContentTypeParser('*', { parseAs: 'buffer' }, (req, body, done) => done(null, body));

        registerSharedRoutes(h2app);
        h2app.listen({ port: 8443, host: '0.0.0.0' });
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
    startWorker();
}
