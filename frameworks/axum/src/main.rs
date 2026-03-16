use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use bytes::Bytes;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;

static SERVER_HDR: HeaderValue = HeaderValue::from_static("axum");

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
    (0..count)
        .map(|_| {
            let conn = Connection::open_with_flags(
                "/data/benchmark.db",
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .expect("Failed to open database");
            conn.execute_batch("PRAGMA mmap_size=268435456").ok();
            Mutex::new(conn)
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

fn server_header(mut resp: Response) -> Response {
    resp.headers_mut()
        .insert(header::SERVER, SERVER_HDR.clone());
    resp
}

async fn pipeline() -> Response {
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            "ok",
        )
            .into_response(),
    )
}

async fn baseline11_get(
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Response {
    let sum = raw_query.as_deref().map(parse_query_sum).unwrap_or(0);
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            sum.to_string(),
        )
            .into_response(),
    )
}

async fn baseline11_post(
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
    body: Bytes,
) -> Response {
    let mut sum = raw_query.as_deref().map(parse_query_sum).unwrap_or(0);
    if let Ok(s) = std::str::from_utf8(&body) {
        if let Ok(n) = s.trim().parse::<i64>() {
            sum += n;
        }
    }
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            sum.to_string(),
        )
            .into_response(),
    )
}

async fn baseline2(
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Response {
    let sum = raw_query.as_deref().map(parse_query_sum).unwrap_or(0);
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            sum.to_string(),
        )
            .into_response(),
    )
}

async fn json_endpoint(State(state): State<Arc<AppState>>) -> Response {
    if state.dataset.is_empty() {
        return server_header(
            (StatusCode::INTERNAL_SERVER_ERROR, "No dataset").into_response(),
        );
    }
    let items: Vec<ProcessedItem> = state
        .dataset
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
    let body = serde_json::to_vec(&resp).unwrap_or_default();
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            body,
        )
            .into_response(),
    )
}

async fn compression(State(state): State<Arc<AppState>>) -> Response {
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            state.json_large_cache.clone(),
        )
            .into_response(),
    )
}

async fn db_endpoint(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
) -> Response {
    let query = raw_query.as_deref().unwrap_or("");
    let mut min: f64 = 10.0;
    let mut max: f64 = 50.0;
    for pair in query.split('&') {
        if let Some(v) = pair.strip_prefix("min=") {
            if let Ok(n) = v.parse() {
                min = n;
            }
        } else if let Some(v) = pair.strip_prefix("max=") {
            if let Ok(n) = v.parse() {
                max = n;
            }
        }
    }

    // Round-robin across DB connections
    let idx = state.db_counter.fetch_add(1, Ordering::Relaxed) % state.db_pool.len().max(1);
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
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            result.to_string(),
        )
            .into_response(),
    )
}

async fn upload(body: Bytes) -> Response {
    server_header(
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            body.len().to_string(),
        )
            .into_response(),
    )
}

async fn static_file(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Response {
    if let Some(sf) = state.static_files.get(&filename) {
        server_header(
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, sf.content_type.as_str())],
                sf.data.clone(),
            )
                .into_response(),
        )
    } else {
        server_header(StatusCode::NOT_FOUND.into_response())
    }
}

fn load_tls_config() -> Option<rustls::ServerConfig> {
    let cert_path = std::env::var("TLS_CERT").unwrap_or_else(|_| "/certs/server.crt".to_string());
    let key_path = std::env::var("TLS_KEY").unwrap_or_else(|_| "/certs/server.key".to_string());
    let cert_file = std::fs::File::open(&cert_path).ok()?;
    let key_file = std::fs::File::open(&key_path).ok()?;
    let certs: Vec<_> = rustls_pemfile::certs(&mut io::BufReader::new(cert_file))
        .filter_map(|r| r.ok())
        .collect();
    let key = rustls_pemfile::private_key(&mut io::BufReader::new(key_file)).ok()??;
    let mut config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .ok()?;
    config.alpn_protocols = vec![b"h2".to_vec()];
    Some(config)
}

#[tokio::main]
async fn main() {
    let dataset = load_dataset();
    let large_dataset: Vec<DatasetItem> =
        match std::fs::read_to_string("/data/dataset-large.json") {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
    let json_large_cache = build_json_cache(&large_dataset);

    let workers = num_cpus::get();
    let state = Arc::new(AppState {
        dataset,
        json_large_cache,
        static_files: load_static_files(),
        db_pool: open_db_pool(workers),
        db_counter: AtomicUsize::new(0),
    });

    let app = Router::new()
        .route("/pipeline", get(pipeline))
        .route("/baseline11", get(baseline11_get).post(baseline11_post))
        .route("/baseline2", get(baseline2))
        .route("/json", get(json_endpoint))
        .route("/compression", get(compression))
        .route("/db", get(db_endpoint))
        .route("/upload", post(upload))
        .route("/static/{filename}", get(static_file))
        .layer(CompressionLayer::new())
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");

    // Spawn TLS server if certs available
    if let Some(tls_config) = load_tls_config() {
        let tls_app = app.clone();
        let tls_addr = SocketAddr::from(([0, 0, 0, 0], 8443));
        tokio::spawn(async move {
            axum_server::bind_rustls(tls_addr, axum_server::tls_rustls::RustlsConfig::from_config(Arc::new(tls_config)))
                .serve(tls_app.into_make_service())
                .await
                .expect("TLS server failed");
        });
    }

    axum::serve(listener, app).await.expect("Server failed");
}
