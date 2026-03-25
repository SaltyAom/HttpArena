use ntex::http::header::{ContentEncoding, CONTENT_TYPE, SERVER};
use ntex::util::{Bytes, BytesMut};
use ntex::web::{self, App, BodyEncoding, HttpRequest, HttpResponse};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

static SERVER_NAME: &str = "ntex";

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
}

struct WorkerDb(RefCell<Option<Connection>>);

fn load_dataset() -> Vec<DatasetItem> {
    let path = std::env::var("DATASET_PATH").unwrap_or_else(|_| "/data/dataset.json".to_string());
    match std::fs::read_to_string(&path) {
        Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn process_items(dataset: &[DatasetItem]) -> Vec<u8> {
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

async fn pipeline() -> HttpResponse {
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "text/plain")
        .body("ok")
}

async fn baseline11_get(req: HttpRequest) -> HttpResponse {
    let sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "text/plain")
        .body(sum.to_string())
}

async fn baseline11_post(
    req: HttpRequest,
    mut body: web::types::Payload,
) -> Result<HttpResponse, web::error::PayloadError> {
    let mut sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    let mut buf = BytesMut::new();
    while let Some(chunk) = ntex::util::stream_recv(&mut body).await {
        buf.extend_from_slice(&chunk?);
    }
    if let Ok(s) = std::str::from_utf8(&buf) {
        if let Ok(n) = s.trim().parse::<i64>() {
            sum += n;
        }
    }
    Ok(HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "text/plain")
        .body(sum.to_string()))
}

async fn baseline2(req: HttpRequest) -> HttpResponse {
    let sum = req.uri().query().map(parse_query_sum).unwrap_or(0);
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "text/plain")
        .body(sum.to_string())
}

async fn upload(body: Bytes) -> HttpResponse {
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "text/plain")
        .body(body.len().to_string())
}

async fn json_endpoint(state: web::types::State<Arc<AppState>>) -> HttpResponse {
    if state.dataset.is_empty() {
        return HttpResponse::InternalServerError().body("No dataset");
    }
    let body = process_items(&state.dataset);
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "application/json")
        .body(body)
}

async fn compression(state: web::types::State<Arc<AppState>>) -> HttpResponse {
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "application/json")
        .encoding(ContentEncoding::Gzip)
        .body(state.json_large_cache.clone())
}

async fn db_endpoint(req: HttpRequest, db: web::types::State<WorkerDb>) -> HttpResponse {
    let min: f64 = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|p| p.strip_prefix("min=").and_then(|v| v.parse().ok()))
        })
        .unwrap_or(10.0);
    let max: f64 = req
        .uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|p| p.strip_prefix("max=").and_then(|v| v.parse().ok()))
        })
        .unwrap_or(50.0);
    let borrow = db.0.borrow();
    let conn = match borrow.as_ref() {
        Some(c) => c,
        None => {
            return HttpResponse::InternalServerError().body("Database not available");
        }
    };
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
    HttpResponse::Ok()
        .header(SERVER, SERVER_NAME)
        .header(CONTENT_TYPE, "application/json")
        .body(result.to_string())
}

async fn static_file(
    state: web::types::State<Arc<AppState>>,
    path: web::types::Path<String>,
) -> HttpResponse {
    let filename = path.into_inner();
    if let Some(sf) = state.static_files.get(&filename) {
        HttpResponse::Ok()
            .header(SERVER, SERVER_NAME)
            .header(CONTENT_TYPE, sf.content_type.as_str())
            .body(sf.data.clone())
    } else {
        HttpResponse::NotFound().finish()
    }
}

#[ntex::main]
async fn main() -> std::io::Result<()> {
    let dataset = load_dataset();

    let large_dataset: Vec<DatasetItem> =
        match std::fs::read_to_string("/data/dataset-large.json") {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
    let json_large_cache = process_items(&large_dataset);

    let state = Arc::new(AppState {
        dataset,
        json_large_cache,
        static_files: load_static_files(),
    });

    let workers = num_cpus::get();

    web::server(async move || {
        let worker_db = Connection::open_with_flags(
            "/data/benchmark.db",
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .ok()
        .map(|conn| {
            conn.execute_batch("PRAGMA mmap_size=268435456").ok();
            conn
        });
        let db_state = WorkerDb(RefCell::new(worker_db));

        App::new()
            .middleware(web::middleware::Compress::default())
            .state(state.clone())
            .state(db_state)
            .state(web::types::PayloadConfig::new(25 * 1024 * 1024))
            .route("/pipeline", web::get().to(pipeline))
            .route("/baseline11", web::get().to(baseline11_get))
            .route("/baseline11", web::post().to(baseline11_post))
            .route("/baseline2", web::get().to(baseline2))
            .route("/upload", web::post().to(upload))
            .route("/json", web::get().to(json_endpoint))
            .route("/compression", web::get().to(compression))
            .route("/db", web::get().to(db_endpoint))
            .route("/static/{filename}", web::get().to(static_file))
    })
    .workers(workers)
    .backlog(4096)
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
