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
        .unwrap_or_else(num_cpus::get)
}

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use salvo::conn::rustls::{Keycert, RustlsConfig};
use salvo::compression::Compression;
use salvo::http::header::{self, HeaderValue};
use salvo::http::StatusCode;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use rusqlite::Connection;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;

static STATE: OnceLock<AppState> = OnceLock::new();
static PG_POOL: OnceLock<Option<Pool>> = OnceLock::new();
static SERVER_HDR: HeaderValue = HeaderValue::from_static("salvo");

struct AppState {
    dataset: Vec<DatasetItem>,
    json_large_cache: Vec<u8>,
    static_files: HashMap<String, StaticFile>,
    db_available: bool,
}

thread_local! {
    static TL_DB: RefCell<Option<Connection>> = RefCell::new(None);
}

fn get_tl_conn() -> bool {
    TL_DB.with(|cell| {
        let mut opt = cell.borrow_mut();
        if opt.is_none() {
            if let Ok(conn) = Connection::open_with_flags(
                "/data/benchmark.db",
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            ) {
                conn.execute_batch("PRAGMA mmap_size=268435456").ok();
                *opt = Some(conn);
            }
        }
        opt.is_some()
    })
}

struct StaticFile {
    data: Vec<u8>,
    content_type: &'static str,
}

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

fn get_mime(ext: &str) -> &'static str {
    match ext {
        ".css" => "text/css",
        ".js" => "application/javascript",
        ".html" => "text/html",
        ".woff2" => "font/woff2",
        ".svg" => "image/svg+xml",
        ".webp" => "image/webp",
        ".json" => "application/json",
        _ => "application/octet-stream",
    }
}

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn build_json_cache(dataset: &[DatasetItem]) -> Vec<u8> {
    let items: Vec<ProcessedItem> = dataset
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
        .collect();
    let resp = JsonResponse {
        count: items.len(),
        items,
    };
    serde_json::to_vec(&resp).unwrap_or_default()
}

fn load_static_files() -> HashMap<String, StaticFile> {
    let mut files = HashMap::new();
    if let Ok(entries) = std::fs::read_dir("/data/static") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(data) = std::fs::read(entry.path()) {
                let ext = name.rfind('.').map(|i| &name[i..]).unwrap_or("");
                let ct = get_mime(ext);
                files.insert(name, StaticFile { data, content_type: ct });
            }
        }
    }
    files
}

#[handler]
async fn add_server_header(res: &mut Response) {
    res.headers_mut()
        .insert(header::SERVER, SERVER_HDR.clone());
}

#[handler]
async fn pipeline(res: &mut Response) {
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    res.render("ok");
}

#[handler]
async fn baseline11_get(req: &mut Request, res: &mut Response) {
    let sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    res.render(sum.to_string());
}

#[handler]
async fn baseline11_post(req: &mut Request, res: &mut Response) {
    let mut sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    if let Ok(body) = req.payload().await {
        if let Ok(s) = std::str::from_utf8(body) {
            if let Ok(n) = s.trim().parse::<i64>() {
                sum += n;
            }
        }
    }
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    res.render(sum.to_string());
}

#[handler]
async fn baseline2(req: &mut Request, res: &mut Response) {
    let sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    res.render(sum.to_string());
}

#[handler]
async fn json_endpoint(res: &mut Response) {
    let state = STATE.get().unwrap();
    if state.dataset.is_empty() {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
        return;
    }
    let items: Vec<ProcessedItem> = state.dataset.iter().map(|d| ProcessedItem {
        id: d.id,
        name: d.name.clone(),
        category: d.category.clone(),
        price: d.price,
        quantity: d.quantity,
        active: d.active,
        tags: d.tags.clone(),
        rating: RatingOut { score: d.rating.score, count: d.rating.count },
        total: (d.price * d.quantity as f64 * 100.0).round() / 100.0,
    }).collect();
    let resp = JsonResponse { count: items.len(), items };
    let body = serde_json::to_vec(&resp).unwrap_or_default();
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    res.write_body(body).ok();
}

#[handler]
async fn compression(res: &mut Response) {
    let state = STATE.get().unwrap();
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    res.write_body(state.json_large_cache.clone()).ok();
}

#[handler]
async fn upload(req: &mut Request, res: &mut Response) {
    use http_body_util::BodyExt;
    let mut size: usize = 0;
    let body = req.body_mut();
    while let Some(frame) = body.frame().await {
        match frame {
            Ok(f) => {
                if let Some(data) = f.data_ref() {
                    size += data.len();
                }
            }
            Err(_) => break,
        }
    }
    res.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain"));
    res.render(size.to_string());
}

#[handler]
async fn db_endpoint(req: &mut Request, res: &mut Response) {
    let state = STATE.get().unwrap();
    if !state.db_available || !get_tl_conn() {
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        res.render("{\"items\":[],\"count\":0}");
        return;
    }
    let min_price: f64 = req.query("min").unwrap_or(10.0);
    let max_price: f64 = req.query("max").unwrap_or(50.0);
    let result = TL_DB.with(|cell| {
        let borrow = cell.borrow();
        let conn = borrow.as_ref().unwrap();
        let mut stmt = conn
            .prepare_cached(
                "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ?1 AND ?2 LIMIT 50",
            )
            .unwrap();
        let rows = stmt.query_map(rusqlite::params![min_price, max_price], |row| {
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
        serde_json::json!({"items": items, "count": items.len()})
    });
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    res.render(result.to_string());
}

#[handler]
async fn pgdb_endpoint(req: &mut Request, res: &mut Response) {
    let pool = match PG_POOL.get().and_then(|p| p.as_ref()) {
        Some(p) => p,
        None => {
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            res.render(r#"{"items":[],"count":0}"#);
            return;
        }
    };
    let min_price: f64 = req.query("min").unwrap_or(10.0);
    let max_price: f64 = req.query("max").unwrap_or(50.0);
    let client = match pool.get().await {
        Ok(c) => c,
        Err(_) => {
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            res.render(r#"{"items":[],"count":0}"#);
            return;
        }
    };
    let stmt = match client.prepare_cached(
        "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50"
    ).await {
        Ok(s) => s,
        Err(_) => {
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            res.render(r#"{"items":[],"count":0}"#);
            return;
        }
    };
    let rows = match client.query(&stmt, &[&min_price, &max_price]).await {
        Ok(r) => r,
        Err(_) => {
            res.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            res.render(r#"{"items":[],"count":0}"#);
            return;
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
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    res.render(result.to_string());
}

#[handler]
async fn static_file(req: &mut Request, res: &mut Response) {
    let state = STATE.get().unwrap();
    let filename: String = req.param("filename").unwrap_or_default();
    if let Some(sf) = state.static_files.get(&filename) {
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(sf.content_type),
        );
        res.write_body(sf.data.clone()).ok();
    } else {
        res.status_code(StatusCode::NOT_FOUND);
    }
}

#[tokio::main]
async fn main() {
    let dataset = load_dataset();

    let large_dataset: Vec<DatasetItem> = match std::fs::read_to_string("/data/dataset-large.json") {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let json_large_cache = build_json_cache(&large_dataset);

    let db_available = Connection::open_with_flags(
        "/data/benchmark.db",
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .is_ok();

    STATE
        .set(AppState {
            dataset,
            json_large_cache,
            static_files: load_static_files(),
            db_available,
        })
        .ok();

    let pg_pool: Option<Pool> = std::env::var("DATABASE_URL").ok().and_then(|url| {
        let pg_config: tokio_postgres::Config = url.parse().ok()?;
        let mgr = Manager::from_config(pg_config, deadpool_postgres::tokio_postgres::NoTls,
            ManagerConfig { recycling_method: RecyclingMethod::Fast });
        let pool_size = (cgroup_cpus() * 4).max(64);
        Pool::builder(mgr).max_size(pool_size).build().ok()
    });
    PG_POOL.set(pg_pool).ok();

    let router = Router::new()
        .hoop(add_server_header)
        .push(Router::with_path("pipeline").get(pipeline))
        .push(
            Router::with_path("baseline11")
                .get(baseline11_get)
                .post(baseline11_post),
        )
        .push(Router::with_path("baseline2").get(baseline2))
        .push(Router::with_path("json").get(json_endpoint))
        .push(Router::with_path("db").get(db_endpoint))
        .push(Router::with_path("async-db").get(pgdb_endpoint))
        .push(
            Router::with_path("compression")
                .hoop(Compression::new().enable_gzip(salvo::compression::CompressionLevel::Fastest))
                .get(compression),
        )
        .push(Router::with_path("upload").post(upload))
        .push(
            Router::with_path("static").push(
                Router::with_path("{filename}").get(static_file),
            ),
        );

    let cert_path = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());

    let has_tls = std::fs::metadata(&cert_path).is_ok() && std::fs::metadata(&key_path).is_ok();

    if has_tls {
        let cert = std::fs::read(&cert_path).expect("Failed to read cert");
        let key = std::fs::read(&key_path).expect("Failed to read key");
        let config = RustlsConfig::new(Keycert::new().cert(cert).key(key));

        let plain = TcpListener::new("0.0.0.0:8080");
        let tls_listener = TcpListener::new("0.0.0.0:8443").rustls(config.clone());
        let quinn_config = config
            .build_quinn_config()
            .expect("Failed to build quinn config");
        let quinn_listener = QuinnListener::new(quinn_config, "0.0.0.0:8443");

        let acceptor = quinn_listener.join(tls_listener).join(plain).bind().await;
        Server::new(acceptor).serve(router).await;
    } else {
        let acceptor = TcpListener::new("0.0.0.0:8080").bind().await;
        Server::new(acceptor).serve(router).await;
    }
}
