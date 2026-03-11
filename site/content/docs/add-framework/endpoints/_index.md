---
title: Endpoints
---

Your framework must implement endpoints depending on which test profiles it participates in. All endpoints are served on **port 8080** (HTTP/1.1) unless noted otherwise.

Data files are **mounted automatically** by the benchmark runner — your Dockerfile does not need to include them. The following paths are available inside the container at runtime:

| Path | Description |
|------|-------------|
| `/data/dataset.json` | 50-item dataset for `/json` |
| `/data/dataset-large.json` | 6000-item dataset for `/compression` |
| `/data/benchmark.db` | SQLite database (100K rows) for `/db` |
| `/data/static/` | 20 static files for `/static/*` |
| `/certs/server.crt`, `/certs/server.key` | TLS certificate and key for HTTPS/H2/H3 |

{{< cards >}}
  {{< card link="baseline" title="Baseline" subtitle="GET/POST /baseline11 — query param sum, required for baseline, limited-conn, noisy, and mixed." icon="lightning-bolt" >}}
  {{< card link="pipeline" title="Pipeline" subtitle="GET /pipeline — fixed 'ok' response for pipelined benchmark." icon="fast-forward" >}}
  {{< card link="json" title="JSON Processing" subtitle="GET /json — compute derived fields from 50-item dataset, return JSON." icon="document-text" >}}
  {{< card link="upload" title="Upload" subtitle="POST /upload — ingest up to 20 MB binary, return CRC32 checksum." icon="cloud-upload" >}}
  {{< card link="db" title="Database" subtitle="GET /db — SQLite range query, return structured JSON." icon="database" >}}
  {{< card link="compression" title="Compression" subtitle="GET /compression — serve ~1 MB JSON with gzip compression." icon="archive" >}}
  {{< card link="static" title="Static Files" subtitle="GET /static/* — serve 20 pre-loaded files over HTTPS." icon="document" >}}
  {{< card link="baseline2" title="HTTP/2 Baseline" subtitle="GET /baseline2 — same as baseline, served over H2 with TLS." icon="globe-alt" >}}
  {{< card link="h3" title="HTTP/3" subtitle="QUIC-based endpoints — /baseline2 and /static/* over HTTP/3." icon="wifi" >}}
{{< /cards >}}
