#define H2O_USE_LIBUV 0

#include <h2o.h>
#include <h2o/serverutil.h>
#include <math.h>
#include <netinet/tcp.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <openssl/ssl.h>
#include <sqlite3.h>
#include <yajl/yajl_gen.h>
#include <yajl/yajl_tree.h>

static h2o_globalconf_t globalconf;
static SSL_CTX *ssl_ctx;
static __thread sqlite3 *tl_db = NULL;
static __thread sqlite3_stmt *tl_db_stmt = NULL;
static yajl_val dataset_tree = NULL;
static size_t dataset_count = 0;
static char *json_large_response;
static size_t json_large_response_len;
/* Pre-loaded static files */
#define MAX_STATIC_FILES 32
typedef struct {
    const char *name;
    const char *content_type;
    char *data;
    size_t len;
} static_file_t;
static static_file_t static_files[MAX_STATIC_FILES];
static int static_file_count;

/* Parse query string values and return their sum */
static int64_t sum_query_values(h2o_req_t *req)
{
    if (req->query_at == SIZE_MAX)
        return 0;
    int64_t sum = 0;
    const char *p = req->path.base + req->query_at + 1;
    const char *end = req->path.base + req->path.len;
    while (p < end) {
        const char *eq = memchr(p, '=', end - p);
        if (!eq) break;
        const char *v = eq + 1;
        const char *amp = memchr(v, '&', end - v);
        if (!amp) amp = end;
        char *ep;
        long long n = strtoll(v, &ep, 10);
        if (ep > v && ep <= amp) sum += n;
        p = amp < end ? amp + 1 : end;
    }
    return sum;
}

/* GET /pipeline — return "ok" (zero-copy static response) */
static int on_pipeline(h2o_handler_t *h, h2o_req_t *req)
{
    static h2o_iovec_t body = {H2O_STRLIT("ok")};
    (void)h;
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = body.len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* GET|POST /baseline11 — sum query params (+ body for POST) */
static int on_baseline11(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    int64_t sum = sum_query_values(req);
    if (h2o_memis(req->method.base, req->method.len, H2O_STRLIT("POST"))
        && req->entity.len > 0) {
        const char *p = req->entity.base;
        const char *end = p + req->entity.len;
        while (p < end && *p <= ' ') p++;
        char *ep;
        long long n = strtoll(p, &ep, 10);
        if (ep > p) sum += n;
    }
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)sum);
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    h2o_iovec_t body = h2o_iovec_init(buf, len);
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* GET /baseline2 — sum query params */
static int on_baseline2(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    int64_t sum = sum_query_values(req);
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%lld", (long long)sum);
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    h2o_iovec_t body = h2o_iovec_init(buf, len);
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* GET /json — serialize JSON dataset per-request from parsed tree */
static int on_json(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    if (!dataset_tree) {
        h2o_send_error_500(req, "Error", "No dataset", 0);
        return 0;
    }

    yajl_gen gen = yajl_gen_alloc(NULL);
    yajl_gen_map_open(gen);
    yajl_gen_string(gen, (const unsigned char *)"items", 5);
    yajl_gen_array_open(gen);

    for (size_t i = 0; i < dataset_count; i++) {
        yajl_val item = YAJL_GET_ARRAY(dataset_tree)->values[i];
        if (!YAJL_IS_OBJECT(item)) continue;

        const char *p_id[] = {"id", NULL};
        const char *p_name[] = {"name", NULL};
        const char *p_cat[] = {"category", NULL};
        const char *p_price[] = {"price", NULL};
        const char *p_qty[] = {"quantity", NULL};
        const char *p_active[] = {"active", NULL};
        const char *p_tags[] = {"tags", NULL};
        const char *p_rating[] = {"rating", NULL};

        yajl_val vid = yajl_tree_get(item, p_id, yajl_t_number);
        yajl_val vname = yajl_tree_get(item, p_name, yajl_t_string);
        yajl_val vcat = yajl_tree_get(item, p_cat, yajl_t_string);
        yajl_val vprice = yajl_tree_get(item, p_price, yajl_t_number);
        yajl_val vqty = yajl_tree_get(item, p_qty, yajl_t_number);
        yajl_val vactive = yajl_tree_get(item, p_active, yajl_t_any);
        yajl_val vtags = yajl_tree_get(item, p_tags, yajl_t_array);
        yajl_val vrating = yajl_tree_get(item, p_rating, yajl_t_object);

        double price = vprice ? YAJL_GET_DOUBLE(vprice) : 0;
        long long qty = vqty ? YAJL_GET_INTEGER(vqty) : 0;
        double total = round(price * (double)qty * 100.0) / 100.0;

        yajl_gen_map_open(gen);

        yajl_gen_string(gen, (const unsigned char *)"id", 2);
        yajl_gen_integer(gen, vid ? YAJL_GET_INTEGER(vid) : 0);

        yajl_gen_string(gen, (const unsigned char *)"name", 4);
        const char *name = vname ? YAJL_GET_STRING(vname) : "";
        yajl_gen_string(gen, (const unsigned char *)name, strlen(name));

        yajl_gen_string(gen, (const unsigned char *)"category", 8);
        const char *cat = vcat ? YAJL_GET_STRING(vcat) : "";
        yajl_gen_string(gen, (const unsigned char *)cat, strlen(cat));

        yajl_gen_string(gen, (const unsigned char *)"price", 5);
        yajl_gen_double(gen, price);

        yajl_gen_string(gen, (const unsigned char *)"quantity", 8);
        yajl_gen_integer(gen, qty);

        yajl_gen_string(gen, (const unsigned char *)"active", 6);
        yajl_gen_bool(gen, vactive ? YAJL_IS_TRUE(vactive) : 0);

        yajl_gen_string(gen, (const unsigned char *)"tags", 4);
        yajl_gen_array_open(gen);
        if (vtags) {
            for (size_t j = 0; j < YAJL_GET_ARRAY(vtags)->len; j++) {
                yajl_val t = YAJL_GET_ARRAY(vtags)->values[j];
                if (YAJL_IS_STRING(t)) {
                    const char *s = YAJL_GET_STRING(t);
                    yajl_gen_string(gen, (const unsigned char *)s, strlen(s));
                }
            }
        }
        yajl_gen_array_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"rating", 6);
        yajl_gen_map_open(gen);
        if (vrating) {
            const char *ps[] = {"score", NULL};
            const char *pc[] = {"count", NULL};
            yajl_val vs = yajl_tree_get(vrating, ps, yajl_t_number);
            yajl_val vc = yajl_tree_get(vrating, pc, yajl_t_number);
            yajl_gen_string(gen, (const unsigned char *)"score", 5);
            yajl_gen_double(gen, vs ? YAJL_GET_DOUBLE(vs) : 0);
            yajl_gen_string(gen, (const unsigned char *)"count", 5);
            yajl_gen_integer(gen, vc ? YAJL_GET_INTEGER(vc) : 0);
        }
        yajl_gen_map_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"total", 5);
        yajl_gen_double(gen, total);

        yajl_gen_map_close(gen);
    }

    yajl_gen_array_close(gen);
    yajl_gen_string(gen, (const unsigned char *)"count", 5);
    yajl_gen_integer(gen, (long long)dataset_count);
    yajl_gen_map_close(gen);

    const unsigned char *buf;
    size_t len;
    yajl_gen_get_buf(gen, &buf, &len);

    h2o_iovec_t body = h2o_strdup(&req->pool, (const char *)buf, len);
    yajl_gen_free(gen);

    h2o_generator_t hgen;
    memset(&hgen, 0, sizeof(hgen));
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = body.len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("application/json"));
    h2o_start_response(req, &hgen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* GET /static/<filename> — serve pre-loaded static files */
static int on_static(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    /* path is /static/<filename>, extract filename after "/static/" (8 chars) */
    if (req->path_normalized.len <= 8) {
        h2o_send_error_404(req, "Not Found", "Not Found", 0);
        return 0;
    }
    const char *fname = req->path_normalized.base + 8;
    size_t fname_len = req->path_normalized.len - 8;

    for (int i = 0; i < static_file_count; i++) {
        size_t nlen = strlen(static_files[i].name);
        if (nlen == fname_len && memcmp(static_files[i].name, fname, nlen) == 0) {
            h2o_generator_t gen;
            memset(&gen, 0, sizeof(gen));
            h2o_iovec_t body = h2o_iovec_init(static_files[i].data, static_files[i].len);
            req->res.status = 200;
            req->res.reason = "OK";
            req->res.content_length = static_files[i].len;
            h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                           NULL, static_files[i].content_type, strlen(static_files[i].content_type));
            h2o_start_response(req, &gen);
            h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
            return 0;
        }
    }
    h2o_send_error_404(req, "Not Found", "Not Found", 0);
    return 0;
}

/* POST /upload — return byte count of request body */
static int on_upload(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    if (!h2o_memis(req->method.base, req->method.len, H2O_STRLIT("POST"))
        || req->entity.len == 0) {
        h2o_send_error_400(req, "Bad Request", "POST with body required", 0);
        return 0;
    }
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%zu", req->entity.len);
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    h2o_iovec_t body = h2o_iovec_init(buf, len);
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("text/plain"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* GET /compression — return pre-serialized large JSON (gzip handled by h2o) */
static int on_compression(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    if (!json_large_response) {
        h2o_send_error_500(req, "Error", "No dataset", 0);
        return 0;
    }
    h2o_generator_t gen;
    memset(&gen, 0, sizeof(gen));
    h2o_iovec_t body = h2o_iovec_init(json_large_response, json_large_response_len);
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = json_large_response_len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("application/json"));
    h2o_start_response(req, &gen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* SQLite DB — open per-thread for lock-free concurrent reads */
static int g_db_available = 0;

static void load_db_check(void)
{
    /* Just verify the DB file is accessible; actual open happens per-thread */
    sqlite3 *tmp = NULL;
    if (sqlite3_open_v2("/data/benchmark.db", &tmp,
                        SQLITE_OPEN_READONLY, NULL) == SQLITE_OK) {
        g_db_available = 1;
        sqlite3_close(tmp);
    }
}

static void load_db_thread(void)
{
    if (!g_db_available) return;
    if (sqlite3_open_v2("/data/benchmark.db", &tl_db,
                        SQLITE_OPEN_READONLY, NULL) != SQLITE_OK) {
        tl_db = NULL;
        return;
    }
    sqlite3_exec(tl_db, "PRAGMA mmap_size=268435456", NULL, NULL, NULL);
    sqlite3_prepare_v2(tl_db,
        "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count "
        "FROM items WHERE price BETWEEN ? AND ? LIMIT 50",
        -1, &tl_db_stmt, NULL);
}

/* GET /db — query SQLite by price range and return JSON array */
static int on_db(h2o_handler_t *h, h2o_req_t *req)
{
    (void)h;
    if (!tl_db || !tl_db_stmt) {
        h2o_send_error_500(req, "Error", "DB not loaded", 0);
        return 0;
    }

    /* Parse min and max from query string (defaults: 10.0, 50.0) */
    double min_price = 10.0;
    double max_price = 50.0;
    if (req->query_at != SIZE_MAX) {
        const char *p = req->path.base + req->query_at + 1;
        const char *end = req->path.base + req->path.len;
        while (p < end) {
            const char *eq = memchr(p, '=', end - p);
            if (!eq) break;
            const char *v = eq + 1;
            const char *amp = memchr(v, '&', end - v);
            if (!amp) amp = end;
            if ((eq - p) == 3 && memcmp(p, "min", 3) == 0) {
                char *ep;
                double d = strtod(v, &ep);
                if (ep > v) min_price = d;
            } else if ((eq - p) == 3 && memcmp(p, "max", 3) == 0) {
                char *ep;
                double d = strtod(v, &ep);
                if (ep > v) max_price = d;
            }
            p = amp < end ? amp + 1 : end;
        }
    }

    sqlite3_bind_double(tl_db_stmt, 1, min_price);
    sqlite3_bind_double(tl_db_stmt, 2, max_price);

    yajl_gen gen = yajl_gen_alloc(NULL);
    yajl_gen_map_open(gen);
    yajl_gen_string(gen, (const unsigned char *)"items", 5);
    yajl_gen_array_open(gen);

    int row_count = 0;
    while (sqlite3_step(tl_db_stmt) == SQLITE_ROW && row_count < 50) {
        int db_id = sqlite3_column_int(tl_db_stmt, 0);
        const char *name = (const char *)sqlite3_column_text(tl_db_stmt, 1);
        const char *category = (const char *)sqlite3_column_text(tl_db_stmt, 2);
        double price = sqlite3_column_double(tl_db_stmt, 3);
        int quantity = sqlite3_column_int(tl_db_stmt, 4);
        int active = sqlite3_column_int(tl_db_stmt, 5);
        const char *tags_str = (const char *)sqlite3_column_text(tl_db_stmt, 6);
        double rating_score = sqlite3_column_double(tl_db_stmt, 7);
        int rating_count = sqlite3_column_int(tl_db_stmt, 8);

        yajl_gen_map_open(gen);

        yajl_gen_string(gen, (const unsigned char *)"id", 2);
        yajl_gen_integer(gen, db_id);

        yajl_gen_string(gen, (const unsigned char *)"name", 4);
        const char *n = name ? name : "";
        yajl_gen_string(gen, (const unsigned char *)n, strlen(n));

        yajl_gen_string(gen, (const unsigned char *)"category", 8);
        const char *cat = category ? category : "";
        yajl_gen_string(gen, (const unsigned char *)cat, strlen(cat));

        yajl_gen_string(gen, (const unsigned char *)"price", 5);
        yajl_gen_double(gen, price);

        yajl_gen_string(gen, (const unsigned char *)"quantity", 8);
        yajl_gen_integer(gen, quantity);

        yajl_gen_string(gen, (const unsigned char *)"active", 6);
        yajl_gen_bool(gen, active ? 1 : 0);

        yajl_gen_string(gen, (const unsigned char *)"tags", 4);
        yajl_gen_array_open(gen);
        if (tags_str) {
            char errbuf[1024];
            yajl_val tags = yajl_tree_parse(tags_str, errbuf, sizeof(errbuf));
            if (tags && YAJL_IS_ARRAY(tags)) {
                for (size_t j = 0; j < YAJL_GET_ARRAY(tags)->len; j++) {
                    yajl_val t = YAJL_GET_ARRAY(tags)->values[j];
                    if (YAJL_IS_STRING(t)) {
                        const char *s = YAJL_GET_STRING(t);
                        yajl_gen_string(gen, (const unsigned char *)s, strlen(s));
                    }
                }
            }
            if (tags) yajl_tree_free(tags);
        }
        yajl_gen_array_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"rating", 6);
        yajl_gen_map_open(gen);
        yajl_gen_string(gen, (const unsigned char *)"score", 5);
        yajl_gen_double(gen, rating_score);
        yajl_gen_string(gen, (const unsigned char *)"count", 5);
        yajl_gen_integer(gen, rating_count);
        yajl_gen_map_close(gen);

        yajl_gen_map_close(gen);
        row_count++;
    }

    sqlite3_reset(tl_db_stmt);

    yajl_gen_array_close(gen);
    yajl_gen_string(gen, (const unsigned char *)"count", 5);
    yajl_gen_integer(gen, row_count);
    yajl_gen_map_close(gen);

    const unsigned char *buf;
    size_t len;
    yajl_gen_get_buf(gen, &buf, &len);

    h2o_iovec_t body = h2o_strdup(&req->pool, (const char *)buf, len);
    yajl_gen_free(gen);

    h2o_generator_t hgen;
    memset(&hgen, 0, sizeof(hgen));
    req->res.status = 200;
    req->res.reason = "OK";
    req->res.content_length = body.len;
    h2o_add_header(&req->pool, &req->res.headers, H2O_TOKEN_CONTENT_TYPE,
                   NULL, H2O_STRLIT("application/json"));
    h2o_start_response(req, &hgen);
    h2o_send(req, &body, 1, H2O_SEND_STATE_FINAL);
    return 0;
}

/* Load all static files from /data/static/ into memory */
static void load_static_files(void)
{
    static const struct { const char *name; const char *ct; } entries[] = {
        {"reset.css",       "text/css"},
        {"layout.css",      "text/css"},
        {"theme.css",       "text/css"},
        {"components.css",  "text/css"},
        {"utilities.css",   "text/css"},
        {"analytics.js",    "application/javascript"},
        {"helpers.js",      "application/javascript"},
        {"app.js",          "application/javascript"},
        {"vendor.js",       "application/javascript"},
        {"router.js",       "application/javascript"},
        {"header.html",     "text/html"},
        {"footer.html",     "text/html"},
        {"regular.woff2",   "font/woff2"},
        {"bold.woff2",      "font/woff2"},
        {"logo.svg",        "image/svg+xml"},
        {"icon-sprite.svg", "image/svg+xml"},
        {"hero.webp",       "image/webp"},
        {"thumb1.webp",     "image/webp"},
        {"thumb2.webp",     "image/webp"},
        {"manifest.json",   "application/json"},
    };
    int n = sizeof(entries) / sizeof(entries[0]);
    for (int i = 0; i < n && static_file_count < MAX_STATIC_FILES; i++) {
        char path[256];
        snprintf(path, sizeof(path), "/data/static/%s", entries[i].name);
        FILE *f = fopen(path, "rb");
        if (!f) continue;
        fseek(f, 0, SEEK_END);
        long sz = ftell(f);
        fseek(f, 0, SEEK_SET);
        char *data = malloc(sz);
        if (!data) { fclose(f); continue; }
        fread(data, 1, sz, f);
        fclose(f);
        static_files[static_file_count].name = entries[i].name;
        static_files[static_file_count].content_type = entries[i].ct;
        static_files[static_file_count].data = data;
        static_files[static_file_count].len = sz;
        static_file_count++;
    }
    printf("Loaded %d static files\n", static_file_count);
}

static h2o_pathconf_t *register_handler(h2o_hostconf_t *host, const char *path,
                              int (*fn)(h2o_handler_t *, h2o_req_t *))
{
    h2o_pathconf_t *pc = h2o_config_register_path(host, path, 0);
    h2o_handler_t *h = h2o_create_handler(pc, sizeof(*h));
    h->on_req = fn;
    return pc;
}

static void setup_host(h2o_hostconf_t *host)
{
    register_handler(host, "/pipeline", on_pipeline);
    register_handler(host, "/baseline11", on_baseline11);
    register_handler(host, "/baseline2", on_baseline2);
    register_handler(host, "/json", on_json);
    register_handler(host, "/static", on_static);
    register_handler(host, "/db", on_db);
    register_handler(host, "/upload", on_upload);
    h2o_pathconf_t *pc_compress = register_handler(host, "/compression", on_compression);
    h2o_compress_args_t compress_args = {.min_size = 100, .gzip = {.quality = 1}, .brotli = {.quality = -1}};
    h2o_compress_register(pc_compress, &compress_args);
}

/* Load dataset.json — parse into tree, keep in memory for per-request serialization */
static void load_dataset(void)
{
    const char *path = getenv("DATASET_PATH");
    if (!path) path = "/data/dataset.json";
    FILE *f = fopen(path, "r");
    if (!f) return;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc(sz + 1);
    if (!data) { fclose(f); return; }
    fread(data, 1, sz, f);
    data[sz] = 0;
    fclose(f);

    char errbuf[1024];
    yajl_val tree = yajl_tree_parse(data, errbuf, sizeof(errbuf));
    free(data);
    if (!tree || !YAJL_IS_ARRAY(tree)) {
        if (tree) yajl_tree_free(tree);
        return;
    }

    dataset_tree = tree;
    dataset_count = YAJL_GET_ARRAY(tree)->len;
    printf("Loaded dataset: %zu items\n", dataset_count);
}

/* Load dataset-large.json and pre-serialize the /compression response with yajl */
static void load_dataset_large(void)
{
    FILE *f = fopen("/data/dataset-large.json", "r");
    if (!f) return;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char *data = malloc(sz + 1);
    if (!data) { fclose(f); return; }
    fread(data, 1, sz, f);
    data[sz] = 0;
    fclose(f);

    char errbuf[1024];
    yajl_val tree = yajl_tree_parse(data, errbuf, sizeof(errbuf));
    free(data);
    if (!tree || !YAJL_IS_ARRAY(tree)) {
        if (tree) yajl_tree_free(tree);
        return;
    }

    yajl_gen gen = yajl_gen_alloc(NULL);
    yajl_gen_map_open(gen);
    yajl_gen_string(gen, (const unsigned char *)"items", 5);
    yajl_gen_array_open(gen);

    size_t count = YAJL_GET_ARRAY(tree)->len;
    for (size_t i = 0; i < count; i++) {
        yajl_val item = YAJL_GET_ARRAY(tree)->values[i];
        if (!YAJL_IS_OBJECT(item)) continue;

        const char *p_id[] = {"id", NULL};
        const char *p_name[] = {"name", NULL};
        const char *p_cat[] = {"category", NULL};
        const char *p_price[] = {"price", NULL};
        const char *p_qty[] = {"quantity", NULL};
        const char *p_active[] = {"active", NULL};
        const char *p_tags[] = {"tags", NULL};
        const char *p_rating[] = {"rating", NULL};

        yajl_val vid = yajl_tree_get(item, p_id, yajl_t_number);
        yajl_val vname = yajl_tree_get(item, p_name, yajl_t_string);
        yajl_val vcat = yajl_tree_get(item, p_cat, yajl_t_string);
        yajl_val vprice = yajl_tree_get(item, p_price, yajl_t_number);
        yajl_val vqty = yajl_tree_get(item, p_qty, yajl_t_number);
        yajl_val vactive = yajl_tree_get(item, p_active, yajl_t_any);
        yajl_val vtags = yajl_tree_get(item, p_tags, yajl_t_array);
        yajl_val vrating = yajl_tree_get(item, p_rating, yajl_t_object);

        double price = vprice ? YAJL_GET_DOUBLE(vprice) : 0;
        long long qty = vqty ? YAJL_GET_INTEGER(vqty) : 0;
        double total = round(price * (double)qty * 100.0) / 100.0;

        yajl_gen_map_open(gen);

        yajl_gen_string(gen, (const unsigned char *)"id", 2);
        yajl_gen_integer(gen, vid ? YAJL_GET_INTEGER(vid) : 0);

        yajl_gen_string(gen, (const unsigned char *)"name", 4);
        const char *name = vname ? YAJL_GET_STRING(vname) : "";
        yajl_gen_string(gen, (const unsigned char *)name, strlen(name));

        yajl_gen_string(gen, (const unsigned char *)"category", 8);
        const char *cat = vcat ? YAJL_GET_STRING(vcat) : "";
        yajl_gen_string(gen, (const unsigned char *)cat, strlen(cat));

        yajl_gen_string(gen, (const unsigned char *)"price", 5);
        yajl_gen_double(gen, price);

        yajl_gen_string(gen, (const unsigned char *)"quantity", 8);
        yajl_gen_integer(gen, qty);

        yajl_gen_string(gen, (const unsigned char *)"active", 6);
        yajl_gen_bool(gen, vactive ? YAJL_IS_TRUE(vactive) : 0);

        yajl_gen_string(gen, (const unsigned char *)"tags", 4);
        yajl_gen_array_open(gen);
        if (vtags) {
            for (size_t j = 0; j < YAJL_GET_ARRAY(vtags)->len; j++) {
                yajl_val t = YAJL_GET_ARRAY(vtags)->values[j];
                if (YAJL_IS_STRING(t)) {
                    const char *s = YAJL_GET_STRING(t);
                    yajl_gen_string(gen, (const unsigned char *)s, strlen(s));
                }
            }
        }
        yajl_gen_array_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"rating", 6);
        yajl_gen_map_open(gen);
        if (vrating) {
            const char *ps[] = {"score", NULL};
            const char *pc[] = {"count", NULL};
            yajl_val vs = yajl_tree_get(vrating, ps, yajl_t_number);
            yajl_val vc = yajl_tree_get(vrating, pc, yajl_t_number);
            yajl_gen_string(gen, (const unsigned char *)"score", 5);
            yajl_gen_double(gen, vs ? YAJL_GET_DOUBLE(vs) : 0);
            yajl_gen_string(gen, (const unsigned char *)"count", 5);
            yajl_gen_integer(gen, vc ? YAJL_GET_INTEGER(vc) : 0);
        }
        yajl_gen_map_close(gen);

        yajl_gen_string(gen, (const unsigned char *)"total", 5);
        yajl_gen_double(gen, total);

        yajl_gen_map_close(gen);
    }

    yajl_gen_array_close(gen);
    yajl_gen_string(gen, (const unsigned char *)"count", 5);
    yajl_gen_integer(gen, (long long)count);
    yajl_gen_map_close(gen);

    const unsigned char *buf;
    size_t len;
    yajl_gen_get_buf(gen, &buf, &len);
    json_large_response = malloc(len);
    memcpy(json_large_response, buf, len);
    json_large_response_len = len;

    yajl_gen_free(gen);
    yajl_tree_free(tree);
    printf("Loaded large dataset: %zu bytes JSON\n", json_large_response_len);
}

/* Create listener socket with SO_REUSEPORT */
static int create_listener(int port)
{
    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);

    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return -1;

    int on = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &on, sizeof(on));
    setsockopt(fd, SOL_SOCKET, SO_REUSEPORT, &on, sizeof(on));
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof(on));
    setsockopt(fd, IPPROTO_TCP, TCP_QUICKACK, &on, sizeof(on));

    int defer = 10;
    setsockopt(fd, IPPROTO_TCP, TCP_DEFER_ACCEPT, &defer, sizeof(defer));

    int qlen = 4096;
    setsockopt(fd, IPPROTO_TCP, TCP_FASTOPEN, &qlen, sizeof(qlen));

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) { close(fd); return -1; }
    if (listen(fd, 4096) < 0) { close(fd); return -1; }
    return fd;
}

/* Accept callback */
static void on_accept(h2o_socket_t *listener, const char *err)
{
    if (err) return;
    h2o_accept_ctx_t *ctx = listener->data;
    h2o_socket_t *sock;
    while ((sock = h2o_evloop_socket_accept(listener)) != NULL)
        h2o_accept(ctx, sock);
}

/* Worker thread: own event loop + listeners */
static void *worker_run(void *arg)
{
    (void)arg;
    load_db_thread();
    h2o_evloop_t *loop = h2o_evloop_create();
    h2o_context_t ctx;
    h2o_context_init(&ctx, loop, &globalconf);

    /* HTTP/1.1 on port 8080 */
    h2o_accept_ctx_t accept_http;
    memset(&accept_http, 0, sizeof(accept_http));
    accept_http.ctx = &ctx;
    accept_http.hosts = globalconf.hosts;

    int fd = create_listener(8080);
    if (fd >= 0) {
        h2o_socket_t *sock = h2o_evloop_socket_create(loop, fd,
                                                       H2O_SOCKET_FLAG_DONT_READ);
        sock->data = &accept_http;
        h2o_socket_read_start(sock, on_accept);
    }

    /* HTTPS/H2 on port 8443 */
    h2o_accept_ctx_t accept_ssl;
    if (ssl_ctx) {
        memset(&accept_ssl, 0, sizeof(accept_ssl));
        accept_ssl.ctx = &ctx;
        accept_ssl.hosts = globalconf.hosts;
        accept_ssl.ssl_ctx = ssl_ctx;

        int fd_ssl = create_listener(8443);
        if (fd_ssl >= 0) {
            h2o_socket_t *sock = h2o_evloop_socket_create(loop, fd_ssl,
                                                           H2O_SOCKET_FLAG_DONT_READ);
            sock->data = &accept_ssl;
            h2o_socket_read_start(sock, on_accept);
        }
    }

    while (h2o_evloop_run(loop, INT32_MAX) == 0)
        ;
    return NULL;
}

/* Initialize TLS for HTTP/2 */
static void init_tls(void)
{
    const char *cert = getenv("TLS_CERT");
    const char *key = getenv("TLS_KEY");
    if (!cert) cert = "/certs/server.crt";
    if (!key) key = "/certs/server.key";
    if (access(cert, R_OK) != 0 || access(key, R_OK) != 0) return;

    ssl_ctx = SSL_CTX_new(TLS_server_method());
    SSL_CTX_set_min_proto_version(ssl_ctx, TLS1_2_VERSION);
    h2o_ssl_register_alpn_protocols(ssl_ctx, h2o_http2_alpn_protocols);

    if (SSL_CTX_use_certificate_file(ssl_ctx, cert, SSL_FILETYPE_PEM) != 1 ||
        SSL_CTX_use_PrivateKey_file(ssl_ctx, key, SSL_FILETYPE_PEM) != 1) {
        SSL_CTX_free(ssl_ctx);
        ssl_ctx = NULL;
    }
}

int main(void)
{
    signal(SIGPIPE, SIG_IGN);
    load_dataset();
    load_dataset_large();
    load_static_files();
    load_db_check();
    init_tls();

    h2o_config_init(&globalconf);
    globalconf.server_name = h2o_iovec_init(H2O_STRLIT("h2o"));

    /* Register host for HTTP (8080) */
    h2o_hostconf_t *host_http = h2o_config_register_host(
        &globalconf, h2o_iovec_init(H2O_STRLIT("default")), 8080);
    setup_host(host_http);

    /* Register host for HTTPS (8443) */
    if (ssl_ctx) {
        h2o_hostconf_t *host_ssl = h2o_config_register_host(
            &globalconf, h2o_iovec_init(H2O_STRLIT("default")), 8443);
        setup_host(host_ssl);
    }

    int nthreads = sysconf(_SC_NPROCESSORS_ONLN);
    if (nthreads < 1) nthreads = 1;

    for (int i = 1; i < nthreads; i++) {
        pthread_t t;
        pthread_create(&t, NULL, worker_run, NULL);
    }

    worker_run(NULL);
    return 0;
}
