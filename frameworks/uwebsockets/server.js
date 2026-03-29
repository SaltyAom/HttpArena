const uWS = require('uWebSockets.js');
const cluster = require('cluster');
const os = require('os');

const SERVER_HEADER = 'uwebsockets';

function sumQuery(qs) {
  if (!qs) return 0;
  let sum = 0;
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
  return sum;
}

function readBody(res, cb) {
  let buffer;
  res.onData((ab, isLast) => {
    const chunk = Buffer.from(ab);
    if (isLast) {
      if (buffer) {
        cb(Buffer.concat([buffer, chunk]).toString());
      } else {
        cb(chunk.toString());
      }
    } else {
      if (buffer) {
        buffer = Buffer.concat([buffer, chunk]);
      } else {
        buffer = Buffer.concat([chunk]);
      }
    }
  });
}

function startServer() {
  const app = uWS.App();

  // /pipeline — lightweight endpoint for pipelining test
  app.get('/pipeline', (res, req) => {
    res.cork(() => {
      res.writeHeader('content-type', 'text/plain');
      res.writeHeader('server', SERVER_HEADER);
      res.end('ok');
    });
  });

  // /baseline2 — GET: sum query params
  app.get('/baseline2', (res, req) => {
    const qs = req.getQuery();
    const body = String(sumQuery(qs));
    res.cork(() => {
      res.writeHeader('content-type', 'text/plain');
      res.writeHeader('server', SERVER_HEADER);
      res.end(body);
    });
  });

  // c1
  // Catch-all GET — /baseline11 etc: sum query params
  app.get('/*', (res, req) => {
    const qs = req.getQuery();
    const body = String(sumQuery(qs));
    res.cork(() => {
      res.writeHeader('content-type', 'text/plain');
      res.writeHeader('server', SERVER_HEADER);
      res.end(body);
    });
  });

  // Catch-all POST — /baseline11 etc: sum query params + body
  app.post('/*', (res, req) => {
    const qs = req.getQuery();
    const querySum = sumQuery(qs);

    let aborted = false;
    res.onAborted(() => { aborted = true; });

    readBody(res, (bodyStr) => {
      if (aborted) return;
      let total = querySum;
      const n = parseInt(bodyStr.trim(), 10);
      if (n === n) total += n;
      res.cork(() => {
        res.writeHeader('content-type', 'text/plain');
        res.writeHeader('server', SERVER_HEADER);
        res.end(String(total));
      });
    });
  });

  app.listen(8080, (listenSocket) => {
    if (listenSocket) {
      console.log(`Worker ${process.pid} listening on port 8080`);
    } else {
      console.error(`Worker ${process.pid} failed to listen on port 8080`);
      process.exit(1);
    }
  });
}

if (cluster.isPrimary) {
  const numCPUs = os.availableParallelism ? os.availableParallelism() : os.cpus().length;
  for (let i = 0; i < numCPUs; i++) cluster.fork();
} else {
  startServer();
}
