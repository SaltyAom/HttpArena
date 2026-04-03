fn cgroup_cpus() -> usize {
    std::fs::read_to_string("/sys/fs/cgroup/cpu.max")
        .ok()
        .and_then(|s| {
            let mut parts = s.trim().split(' ');
            let quota = parts.next()?;
            if quota == "max" { return None; }
            let period: usize = parts.next()?.parse().ok()?;
            let q: usize = quota.parse().ok()?;
            let cpus = q / period;
            if cpus >= 1 { Some(cpus) } else { None }
        })
        .unwrap_or_else(cgroup_cpus)
}

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use flate2::write::GzEncoder;
use flate2::Compression;
use rocket::data::{Data, ToByteUnit};
use rocket::http::{ContentType, Header, Status};
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::response::{self, Responder, Response};
use rocket::tokio::io::AsyncReadExt;
use rocket::{get, post, routes, State};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ─── Request guard to extract raw query string ───

struct RawQuery(Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RawQuery {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(RawQuery(
            req.uri().query().map(|q| q.as_str().to_string()),
        ))
    }
}

// ─── Custom responder that always sets Server header ───

struct ServerResponse {
    status: Status,
    content_type: ContentType,
    body: Vec<u8>,
    extra_headers: Vec<(&'static str, String)>,
}

impl ServerResponse {
    fn text(body: String) -> Self {
        Self {
            status: Status::Ok,
            content_type: ContentType::Plain,
            body: body.into_bytes(),
            extra_headers: vec![],
        }
    }

    fn json_bytes(body: Vec<u8>) -> Self {
        Self {
            status: Status::Ok,
            content_type: ContentType::JSON,
            body,
            extra_headers: vec![],
        }
    }

    fn with_header(mut self, name: &'static str, value: String) -> Self {
        self.extra_headers.push((name, value));
        self
    }

    fn not_found() -> Self {
        Self {
            status: Status::NotFound,
            content_type: ContentType::Plain,
            body: b"Not Found".to_vec(),
            extra_headers: vec![],
        }
    }

    fn error(msg: &str) -> Self {
        Self {
            status: Status::InternalServerError,
            content_type: ContentType::Plain,
            body: msg.as_bytes().to_vec(),
            extra_headers: vec![],
        }
    }
}

impl<'r> Responder<'r, 'static> for ServerResponse {
    fn respond_to(self, _req: &'r Request<'_>) -> response::Result<'static> {
        let mut builder = Response::build();
        builder
            .status(self.status)
            .header(self.content_type)
            .header(Header::new("Server", "rocket"))
            .sized_body(self.body.len(), Cursor::new(self.body));
        for (name, value) in self.extra_headers {
            builder.header(Header::new(name, value));
        }
        builder.ok()
    }
}

// ─── Data types ───

#[derive(Deserialize, Clone)]
struct Rating {
    score: f64,
    count: i64,
}

#[derive(Deserialize, Clone)]
struct DatasetItem {
    id: i64,
    name: String,
    category: String,
    price: f64,
    quantity: i64,
    active: bool,
    tags: Vec<String>,
    rating: Rating,
}

#[derive(Serialize)]
struct RatingOut {
    score: f64,
    count: i64,
}

#[derive(Serialize)]
struct ProcessedItem {
    id: i64,
    name: String,
    category: String,
    price: f64,
    quantity: i64,
    active: bool,
    tags: Vec<String>,
    rating: RatingOut,
    total: f64,
}

#[derive(Serialize)]
struct JsonResponse {
    items: Vec<ProcessedItem>,
    count: usize,
}

// ─── App state ───

struct StaticFile {
    data: Vec<u8>,
    content_type: String,
}

struct AppState {
    dataset: Vec<DatasetItem>,
    json_large_cache: Vec<u8>,
    static_files: HashMap<String, StaticFile>,
    db_pool: Vec<Mutex<Connection>>,
    db_counter: AtomicUsize,
}

fn process_items(dataset: &[DatasetItem]) -> Vec<ProcessedItem> {
    dataset
        .iter()
        .map(|d| ProcessedItem {
            id: d.id,
            name: d.name.clone(),
            category: d.category.clone(),
            price: d.price,
            quantity: d.quantity,
            active: d.active,
            tags: d.tags.clone(),
            rating: RatingOut {
                score: d.rating.score,
                count: d.rating.count,
            },
            total: (d.price * d.quantity as f64 * 100.0).round() / 100.0,
        })
        .collect()
}

fn build_json_cache(dataset: &[DatasetItem]) -> Vec<u8> {
    let items = process_items(dataset);
    let resp = JsonResponse {
        count: items.len(),
        items,
    };
    serde_json::to_vec(&resp).unwrap_or_default()
}

fn gzip_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn load_static_files() -> HashMap<String, StaticFile> {
    let mime_types: HashMap<&str, &str> = [
        (".css", "text/css"),
        (".js", "application/javascript"),
        (".html", "text/html"),
        (".woff2", "font/woff2"),
        (".svg", "image/svg+xml"),
        (".webp", "image/webp"),
        (".json", "application/json"),
    ]
    .into();
    let mut files = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/data/static") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(data) = std::fs::read(entry.path()) {
                let ext = name.rfind('.').map(|i| &name[i..]).unwrap_or("");
                let ct = mime_types.get(ext).unwrap_or(&"application/octet-stream");
                files.insert(
                    name,
                    StaticFile {
                        data,
                        content_type: ct.to_string(),
                    },
                );
            }
        }
    }
    files
}

fn open_db_pool(count: usize) -> Vec<Mutex<Connection>> {
    let db_path = "/data/benchmark.db";
    if !std::path::Path::new(db_path).exists() {
        return Vec::new();
    }
    (0..count)
        .filter_map(|_| {
            let conn = Connection::open_with_flags(
                db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .ok()?;
            conn.execute_batch("PRAGMA mmap_size=268435456").ok();
            Some(Mutex::new(conn))
        })
        .collect()
}

fn parse_query_sum(query: &str) -> i64 {
    let mut sum: i64 = 0;
    for pair in query.split('&') {
        if let Some(val) = pair.split('=').nth(1) {
            if let Ok(n) = val.parse::<i64>() {
                sum += n;
            }
        }
    }
    sum
}

fn parse_query_param(query: &str, name: &str) -> Option<f64> {
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix(name).and_then(|s| s.strip_prefix('=')) {
            if let Ok(n) = v.parse() {
                return Some(n);
            }
        }
    }
    None
}

// ─── Routes ───

#[get("/pipeline")]
fn pipeline() -> ServerResponse {
    ServerResponse::text("ok".to_string())
}

#[get("/baseline11")]
fn baseline11_get(raw: RawQuery) -> ServerResponse {
    let sum = raw.0.as_deref().map(parse_query_sum).unwrap_or(0);
    ServerResponse::text(sum.to_string())
}

#[post("/baseline11", data = "<body>")]
async fn baseline11_post(raw: RawQuery, body: Data<'_>) -> ServerResponse {
    let mut sum = raw.0.as_deref().map(parse_query_sum).unwrap_or(0);
    if let Ok(bytes) = body.open(25.mebibytes()).into_bytes().await {
        if let Ok(s) = std::str::from_utf8(&bytes) {
            if let Ok(n) = s.trim().parse::<i64>() {
                sum += n;
            }
        }
    }
    ServerResponse::text(sum.to_string())
}

#[get("/baseline2")]
fn baseline2(raw: RawQuery) -> ServerResponse {
    let sum = raw.0.as_deref().map(parse_query_sum).unwrap_or(0);
    ServerResponse::text(sum.to_string())
}

#[get("/json")]
fn json_endpoint(state: &State<Arc<AppState>>) -> ServerResponse {
    if state.dataset.is_empty() {
        return ServerResponse::error("No dataset");
    }
    let items = process_items(&state.dataset);
    let resp = JsonResponse {
        count: items.len(),
        items,
    };
    ServerResponse::json_bytes(serde_json::to_vec(&resp).unwrap_or_default())
}

#[get("/compression")]
fn compression_endpoint(state: &State<Arc<AppState>>) -> ServerResponse {
    if state.json_large_cache.is_empty() {
        return ServerResponse::error("No dataset");
    }
    let compressed = gzip_compress(&state.json_large_cache);
    ServerResponse {
        status: Status::Ok,
        content_type: ContentType::JSON,
        body: compressed,
        extra_headers: vec![],
    }
    .with_header("Content-Encoding", "gzip".to_string())
}

#[get("/db")]
fn db_endpoint(state: &State<Arc<AppState>>, raw: RawQuery) -> ServerResponse {
    let query = raw.0.as_deref().unwrap_or("");
    let min = parse_query_param(query, "min").unwrap_or(10.0);
    let max = parse_query_param(query, "max").unwrap_or(50.0);

    if state.db_pool.is_empty() {
        return ServerResponse::error("Database not available");
    }

    let idx = state.db_counter.fetch_add(1, Ordering::Relaxed) % state.db_pool.len();
    let conn = state.db_pool[idx].lock().unwrap();
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ?1 AND ?2 LIMIT 50",
        )
        .unwrap();
    let rows = stmt.query_map(rusqlite::params![min, max], |row| {
        Ok(serde_json::json!({
            "id": row.get::<_, i64>(0)?,
            "name": row.get::<_, String>(1)?,
            "category": row.get::<_, String>(2)?,
            "price": row.get::<_, f64>(3)?,
            "quantity": row.get::<_, i64>(4)?,
            "active": row.get::<_, i64>(5)? == 1,
            "tags": serde_json::from_str::<serde_json::Value>(&row.get::<_, String>(6)?).unwrap_or_default(),
            "rating": serde_json::json!({
                "score": row.get::<_, f64>(7)?,
                "count": row.get::<_, i64>(8)?
            })
        }))
    });
    let items: Vec<serde_json::Value> = match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    };
    let result = serde_json::json!({"items": items, "count": items.len()});
    ServerResponse::json_bytes(serde_json::to_vec(&result).unwrap_or_default())
}

#[get("/async-db")]
async fn pgdb_endpoint(raw: RawQuery, pg_pool: &State<Option<Pool>>) -> ServerResponse {
    let pool = match pg_pool.as_ref() {
        Some(p) => p,
        None => {
            return ServerResponse::json_bytes(br#"{"items":[],"count":0}"#.to_vec());
        }
    };
    let query = raw.0.as_deref().unwrap_or("");
    let min: f64 = parse_query_param(query, "min").unwrap_or(10.0);
    let max: f64 = parse_query_param(query, "max").unwrap_or(50.0);
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            return ServerResponse::json_bytes(br#"{"items":[],"count":0}"#.to_vec());
        }
    };
    let stmt = match client.prepare_cached(
        "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50"
    ).await {
        Ok(s) => s,
        Err(_) => {
            return ServerResponse::json_bytes(br#"{"items":[],"count":0}"#.to_vec());
        }
    };
    let rows = match client.query(&stmt, &[&min, &max]).await {
        Ok(r) => r,
        Err(_) => {
            return ServerResponse::json_bytes(br#"{"items":[],"count":0}"#.to_vec());
        }
    };
    let items: Vec<serde_json::Value> = rows.iter().map(|row| {
        serde_json::json!({
            "id": row.get::<_, i32>(0) as i64,
            "name": row.get::<_, &str>(1),
            "category": row.get::<_, &str>(2),
            "price": row.get::<_, f64>(3),
            "quantity": row.get::<_, i32>(4) as i64,
            "active": row.get::<_, bool>(5),
            "tags": row.get::<_, serde_json::Value>(6),
            "rating": {
                "score": row.get::<_, f64>(7),
                "count": row.get::<_, i32>(8) as i64,
            }
        })
    }).collect();
    let result = serde_json::json!({"items": items, "count": items.len()});
    ServerResponse::json_bytes(serde_json::to_vec(&result).unwrap_or_default())
}

#[post("/upload", data = "<body>")]
async fn upload(body: Data<'_>) -> ServerResponse {
    let mut stream = body.open(25.mebibytes());
    let mut buf = [0u8; 65536];
    let mut size: usize = 0;
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => size += n,
            Err(_) => break,
        }
    }
    ServerResponse::text(size.to_string())
}

#[get("/static/<filename>")]
fn static_file(state: &State<Arc<AppState>>, filename: &str) -> ServerResponse {
    if let Some(sf) = state.static_files.get(filename) {
        ServerResponse {
            status: Status::Ok,
            content_type: ContentType::parse_flexible(&sf.content_type)
                .unwrap_or(ContentType::Binary),
            body: sf.data.clone(),
            extra_headers: vec![],
        }
    } else {
        ServerResponse::not_found()
    }
}

// ─── Catchers ───

#[rocket::catch(404)]
fn not_found(_req: &Request<'_>) -> ServerResponse {
    ServerResponse::not_found()
}

#[rocket::catch(405)]
fn method_not_allowed(_req: &Request<'_>) -> ServerResponse {
    ServerResponse {
        status: Status::MethodNotAllowed,
        content_type: ContentType::Plain,
        body: b"Method Not Allowed".to_vec(),
        extra_headers: vec![],
    }
}

// ─── Build Rocket instance ───

fn build_rocket(
    state: Arc<AppState>,
    pg_pool: Option<Pool>,
    figment: rocket::figment::Figment,
) -> rocket::Rocket<rocket::Build> {
    rocket::custom(figment)
        .manage(state)
        .manage(pg_pool)
        .mount(
            "/",
            routes![
                pipeline,
                baseline11_get,
                baseline11_post,
                baseline2,
                json_endpoint,
                compression_endpoint,
                db_endpoint,
                pgdb_endpoint,
                upload,
                static_file,
            ],
        )
        .register("/", rocket::catchers![not_found, method_not_allowed])
}

// ─── Main ───

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workers = std::env::var("ROCKET_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(cgroup_cpus);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .enable_all()
        .build()?;

    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let dataset = load_dataset();
    let large_dataset: Vec<DatasetItem> =
        match std::fs::read_to_string("/data/dataset-large.json") {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
    let json_large_cache = build_json_cache(&large_dataset);

    let workers = std::env::var("ROCKET_WORKERS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(cgroup_cpus);
    let state = Arc::new(AppState {
        dataset,
        json_large_cache,
        static_files: load_static_files(),
        db_pool: open_db_pool(workers),
        db_counter: AtomicUsize::new(0),
    });

    let pg_pool: Option<Pool> = std::env::var("DATABASE_URL").ok().and_then(|url| {
        let pg_config: tokio_postgres::Config = url.parse().ok()?;
        let mgr = Manager::from_config(pg_config, deadpool_postgres::tokio_postgres::NoTls,
            ManagerConfig { recycling_method: RecyclingMethod::Fast });
        let pool_size = (cgroup_cpus() * 4).max(64);
        Pool::builder(mgr).max_size(pool_size).build().ok()
    });

    let cert_path = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    let has_tls =
        std::path::Path::new(&cert_path).exists() && std::path::Path::new(&key_path).exists();

    // HTTP server on port 8080
    let http_figment = rocket::Config::figment()
        .merge(("address", "0.0.0.0"))
        .merge(("port", 8080u16))
        .merge(("workers", workers))
        .merge(("log_level", "off"))
        .merge(("limits.data-form", "25 MiB"))
        .merge(("limits.bytes", "25 MiB"));

    if has_tls {
        // TLS server on port 8443
        let tls_figment = rocket::Config::figment()
            .merge(("address", "0.0.0.0"))
            .merge(("port", 8443u16))
            .merge(("workers", workers))
            .merge(("log_level", "off"))
            .merge(("limits.data-form", "25 MiB"))
            .merge(("limits.bytes", "25 MiB"))
            .merge(("tls.certs", &cert_path))
            .merge(("tls.key", &key_path));

        let tls_rocket = build_rocket(state.clone(), pg_pool.clone(), tls_figment);
        let http_rocket = build_rocket(state.clone(), pg_pool.clone(), http_figment);

        // Launch both concurrently
        let tls_handle = tokio::spawn(async move {
            let _ = tls_rocket.launch().await;
        });
        let http_handle = tokio::spawn(async move {
            let _ = http_rocket.launch().await;
        });

        let _ = tokio::try_join!(tls_handle, http_handle);
    } else {
        let http_rocket = build_rocket(state.clone(), pg_pool.clone(), http_figment);
        let _ = http_rocket.launch().await;
    }

    Ok(())
}
