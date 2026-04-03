#include <drogon/drogon.h>
#include <sqlite3.h>
#include <thread>
#include <dirent.h>
#include <fstream>
#include <sstream>
#include <cmath>
#include <unistd.h>

using namespace drogon;

// ── Shared data ──

struct Rating { double score; int64_t count; };
struct DataItem {
    int64_t id;
    std::string name, category;
    double price;
    int quantity;
    bool active;
    std::vector<std::string> tags;
    Rating rating;
    double total;
};
static std::vector<DataItem> dataset;

static std::string json_large_response;

struct StaticFile {
    std::string data;
    std::string content_type;
};
static std::unordered_map<std::string, StaticFile> static_files;

static bool db_available = false;

static sqlite3 *openDb()
{
    sqlite3 *h = nullptr;
    if (sqlite3_open_v2("/data/benchmark.db", &h,
                        SQLITE_OPEN_READONLY | SQLITE_OPEN_NOMUTEX, nullptr) != SQLITE_OK) {
        if (h) sqlite3_close(h);
        return nullptr;
    }
    sqlite3_exec(h, "PRAGMA mmap_size=268435456", nullptr, nullptr, nullptr);
    return h;
}

static thread_local sqlite3 *tl_db = nullptr;
static thread_local sqlite3_stmt *tl_stmt = nullptr;

static sqlite3 *getDb()
{
    if (!tl_db) {
        tl_db = openDb();
        if (tl_db) {
            const char *sql = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ?1 AND ?2 LIMIT 50";
            sqlite3_prepare_v2(tl_db, sql, -1, &tl_stmt, nullptr);
        }
    }
    return tl_db;
}

// ── PostgreSQL (async-db) ──

static bool pg_available = false;
static drogon::orm::DbClientPtr pgClient;

static void loadDataset()
{
    const char *path = getenv("DATASET_PATH");
    if (!path) path = "/data/dataset.json";
    std::ifstream f(path);
    if (!f.is_open()) return;
    std::stringstream ss;
    ss << f.rdbuf();
    f.close();
    Json::CharReaderBuilder rb;
    Json::Value root;
    std::string errs;
    std::istringstream is(ss.str());
    Json::parseFromStream(rb, is, &root, &errs);
    if (!root.isArray()) return;

    for (const auto &d : root) {
        DataItem item;
        item.id = d["id"].asInt64();
        item.name = d["name"].asString();
        item.category = d["category"].asString();
        item.price = d["price"].asDouble();
        item.quantity = d["quantity"].asInt();
        item.active = d["active"].asBool();
        if (d["tags"].isArray())
            for (const auto &t : d["tags"]) item.tags.push_back(t.asString());
        item.rating.score = d["rating"]["score"].asDouble();
        item.rating.count = d["rating"]["count"].asInt64();
        item.total = std::round(item.price * item.quantity * 100.0) / 100.0;
        dataset.push_back(std::move(item));
    }
}

static void loadDatasetLarge()
{
    std::ifstream f("/data/dataset-large.json");
    if (!f.is_open()) return;
    std::stringstream ss;
    ss << f.rdbuf();
    f.close();
    Json::CharReaderBuilder rb;
    Json::Value root;
    std::string errs;
    std::istringstream is(ss.str());
    if (!Json::parseFromStream(rb, is, &root, &errs) || !root.isArray()) return;

    Json::Value resp;
    Json::Value items(Json::arrayValue);
    for (const auto &d : root) {
        Json::Value item;
        item["id"] = d["id"]; item["name"] = d["name"];
        item["category"] = d["category"]; item["price"] = d["price"];
        item["quantity"] = d["quantity"]; item["active"] = d["active"];
        item["tags"] = d["tags"]; item["rating"] = d["rating"];
        item["total"] = std::round(d["price"].asDouble() * d["quantity"].asInt() * 100.0) / 100.0;
        items.append(std::move(item));
    }
    resp["items"] = std::move(items);
    resp["count"] = static_cast<int>(root.size());
    Json::StreamWriterBuilder wb;
    wb["indentation"] = "";
    json_large_response = Json::writeString(wb, resp);
}

static void loadStaticFiles()
{
    static const std::unordered_map<std::string, std::string> mime = {
        {".css","text/css"},{".js","application/javascript"},{".html","text/html"},
        {".woff2","font/woff2"},{".svg","image/svg+xml"},{".webp","image/webp"},{".json","application/json"}
    };
    DIR *d = opendir("/data/static");
    if (!d) return;
    struct dirent *e;
    while ((e = readdir(d)) != nullptr) {
        if (e->d_type != DT_REG) continue;
        std::string name(e->d_name);
        std::ifstream f("/data/static/" + name, std::ios::binary);
        if (!f) continue;
        std::ostringstream ss;
        ss << f.rdbuf();
        auto dot = name.rfind('.');
        std::string ext = dot != std::string::npos ? name.substr(dot) : "";
        auto it = mime.find(ext);
        std::string ct = it != mime.end() ? it->second : "application/octet-stream";
        static_files[name] = {ss.str(), ct};
    }
    closedir(d);
}

static int64_t sumQuery(const HttpRequestPtr &req)
{
    int64_t sum = 0;
    for (auto &[k, v] : req->parameters()) {
        try { sum += std::stoll(v); } catch (...) {}
    }
    return sum;
}

// ── Controller ──

class BenchmarkCtrl : public drogon::HttpController<BenchmarkCtrl>
{
public:
    METHOD_LIST_BEGIN
    ADD_METHOD_TO(BenchmarkCtrl::pipeline,    "/pipeline",         Get);
    ADD_METHOD_TO(BenchmarkCtrl::json,        "/json",             Get);
    ADD_METHOD_TO(BenchmarkCtrl::compression, "/compression",      Get);
    ADD_METHOD_TO(BenchmarkCtrl::baseline2,   "/baseline2",        Get);
    ADD_METHOD_TO(BenchmarkCtrl::upload,      "/upload",           Post);
    ADD_METHOD_TO(BenchmarkCtrl::baseline11,  "/baseline11",       Get, Post);
    ADD_METHOD_TO(BenchmarkCtrl::dbEndpoint,  "/db",               Get);
    ADD_METHOD_TO(BenchmarkCtrl::asyncDb,    "/async-db",         Get);
    ADD_METHOD_TO(BenchmarkCtrl::staticFile,  "/static/{1}",       Get);
    METHOD_LIST_END

    void pipeline(const HttpRequestPtr &req,
                  std::function<void(const HttpResponsePtr &)> &&callback)
    {
        auto resp = HttpResponse::newHttpResponse();
        resp->setBody("ok");
        resp->setContentTypeCode(CT_TEXT_PLAIN);
        callback(resp);
    }

    void json(const HttpRequestPtr &req,
              std::function<void(const HttpResponsePtr &)> &&callback)
    {
        if (!dataset.empty()) {
            Json::Value respJson;
            Json::Value items(Json::arrayValue);
            for (const auto &d : dataset) {
                Json::Value item;
                item["id"] = static_cast<Json::Int64>(d.id);
                item["name"] = d.name;
                item["category"] = d.category;
                item["price"] = d.price;
                item["quantity"] = d.quantity;
                item["active"] = d.active;
                Json::Value tags(Json::arrayValue);
                for (const auto &t : d.tags) tags.append(t);
                item["tags"] = std::move(tags);
                Json::Value rating;
                rating["score"] = d.rating.score;
                rating["count"] = static_cast<Json::Int64>(d.rating.count);
                item["rating"] = std::move(rating);
                item["total"] = d.total;
                items.append(std::move(item));
            }
            respJson["items"] = std::move(items);
            respJson["count"] = static_cast<int>(dataset.size());
            Json::StreamWriterBuilder wb;
            wb["indentation"] = "";
            auto resp = HttpResponse::newHttpResponse();
            resp->setBody(Json::writeString(wb, respJson));
            resp->setContentTypeCode(CT_APPLICATION_JSON);
            callback(resp);
        } else {
            auto resp = HttpResponse::newHttpResponse();
            resp->setStatusCode(k500InternalServerError);
            resp->setBody("No dataset");
            callback(resp);
        }
    }

    void compression(const HttpRequestPtr &req,
                     std::function<void(const HttpResponsePtr &)> &&callback)
    {
        if (!json_large_response.empty()) {
            auto resp = HttpResponse::newHttpResponse();
            resp->setBody(json_large_response);
            resp->setContentTypeCode(CT_APPLICATION_JSON);
            callback(resp);
        } else {
            auto resp = HttpResponse::newHttpResponse();
            resp->setStatusCode(k500InternalServerError);
            resp->setBody("No dataset");
            callback(resp);
        }
    }

    void baseline2(const HttpRequestPtr &req,
                   std::function<void(const HttpResponsePtr &)> &&callback)
    {
        auto resp = HttpResponse::newHttpResponse();
        resp->setBody(std::to_string(sumQuery(req)));
        resp->setContentTypeCode(CT_TEXT_PLAIN);
        callback(resp);
    }

    void upload(const HttpRequestPtr &req,
                std::function<void(const HttpResponsePtr &)> &&callback)
    {
        const auto &body = req->body();
        auto resp = HttpResponse::newHttpResponse();
        resp->setBody(std::to_string(body.size()));
        resp->setContentTypeCode(CT_TEXT_PLAIN);
        callback(resp);
    }

    void baseline11(const HttpRequestPtr &req,
                    std::function<void(const HttpResponsePtr &)> &&callback)
    {
        int64_t sum = sumQuery(req);
        if (req->method() == Post) {
            const auto &body = req->body();
            if (!body.empty()) {
                try { sum += std::stoll(std::string(body)); } catch (...) {}
            }
        }
        auto resp = HttpResponse::newHttpResponse();
        resp->setBody(std::to_string(sum));
        resp->setContentTypeCode(CT_TEXT_PLAIN);
        callback(resp);
    }

    void dbEndpoint(const HttpRequestPtr &req,
                    std::function<void(const HttpResponsePtr &)> &&callback)
    {
        if (!db_available || !getDb() || !tl_stmt) {
            auto resp = HttpResponse::newHttpResponse();
            resp->setBody("{\"items\":[],\"count\":0}");
            resp->setContentTypeCode(CT_APPLICATION_JSON);
            callback(resp);
            return;
        }
        double minPrice = 10.0, maxPrice = 50.0;
        for (auto &[k, v] : req->parameters()) {
            if (k == "min") { try { minPrice = std::stod(v); } catch (...) {} }
            else if (k == "max") { try { maxPrice = std::stod(v); } catch (...) {} }
        }
        Json::Value respJson;
        Json::Value items(Json::arrayValue);
        sqlite3_reset(tl_stmt);
        sqlite3_bind_double(tl_stmt, 1, minPrice);
        sqlite3_bind_double(tl_stmt, 2, maxPrice);
        while (sqlite3_step(tl_stmt) == SQLITE_ROW) {
            Json::Value item;
            item["id"] = static_cast<Json::Int64>(sqlite3_column_int64(tl_stmt, 0));
            item["name"] = reinterpret_cast<const char *>(sqlite3_column_text(tl_stmt, 1));
            item["category"] = reinterpret_cast<const char *>(sqlite3_column_text(tl_stmt, 2));
            item["price"] = sqlite3_column_double(tl_stmt, 3);
            item["quantity"] = static_cast<Json::Int64>(sqlite3_column_int64(tl_stmt, 4));
            item["active"] = sqlite3_column_int(tl_stmt, 5) == 1;
            const char *tagsStr = reinterpret_cast<const char *>(sqlite3_column_text(tl_stmt, 6));
            Json::CharReaderBuilder rb;
            Json::Value tags;
            std::string errs;
            std::istringstream tis(tagsStr ? tagsStr : "[]");
            Json::parseFromStream(rb, tis, &tags, &errs);
            item["tags"] = tags;
            Json::Value rating;
            rating["score"] = sqlite3_column_double(tl_stmt, 7);
            rating["count"] = static_cast<Json::Int64>(sqlite3_column_int64(tl_stmt, 8));
            item["rating"] = rating;
            items.append(std::move(item));
        }
        respJson["items"] = std::move(items);
        respJson["count"] = static_cast<int>(respJson["items"].size());
        Json::StreamWriterBuilder wb;
        wb["indentation"] = "";
        auto resp = HttpResponse::newHttpResponse();
        resp->setBody(Json::writeString(wb, respJson));
        resp->setContentTypeCode(CT_APPLICATION_JSON);
        callback(resp);
    }

    void asyncDb(const HttpRequestPtr &req,
                 std::function<void(const HttpResponsePtr &)> &&callback)
    {
        static const std::string empty_resp = "{\"items\":[],\"count\":0}";
        if (!pg_available || !pgClient) {
            auto resp = HttpResponse::newHttpResponse();
            resp->setBody(empty_resp);
            resp->setContentTypeCode(CT_APPLICATION_JSON);
            callback(resp);
            return;
        }
        double minPrice = 10.0, maxPrice = 50.0;
        for (auto &[k, v] : req->parameters()) {
            if (k == "min") { try { minPrice = std::stod(v); } catch (...) {} }
            else if (k == "max") { try { maxPrice = std::stod(v); } catch (...) {} }
        }
        pgClient->execSqlAsync(
            "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50",
            [callback](const drogon::orm::Result &result) {
                Json::Value respJson;
                Json::Value items(Json::arrayValue);
                for (const auto &row : result) {
                    Json::Value item;
                    item["id"] = static_cast<Json::Int64>(row["id"].as<int32_t>());
                    item["name"] = row["name"].as<std::string>();
                    item["category"] = row["category"].as<std::string>();
                    item["price"] = row["price"].as<double>();
                    item["quantity"] = static_cast<Json::Int64>(row["quantity"].as<int32_t>());
                    item["active"] = row["active"].as<bool>();
                    Json::CharReaderBuilder rb;
                    Json::Value tags;
                    std::string errs;
                    std::string tagsStr = row["tags"].as<std::string>();
                    std::istringstream tis(tagsStr);
                    Json::parseFromStream(rb, tis, &tags, &errs);
                    item["tags"] = tags;
                    Json::Value rating;
                    rating["score"] = row["rating_score"].as<double>();
                    rating["count"] = static_cast<Json::Int64>(row["rating_count"].as<int32_t>());
                    item["rating"] = rating;
                    items.append(std::move(item));
                }
                respJson["items"] = std::move(items);
                respJson["count"] = static_cast<int>(result.size());
                Json::StreamWriterBuilder wb;
                wb["indentation"] = "";
                auto resp = HttpResponse::newHttpResponse();
                resp->setBody(Json::writeString(wb, respJson));
                resp->setContentTypeCode(CT_APPLICATION_JSON);
                callback(resp);
            },
            [callback](const drogon::orm::DrogonDbException &e) {
                auto resp = HttpResponse::newHttpResponse();
                resp->setBody("{\"items\":[],\"count\":0}");
                resp->setContentTypeCode(CT_APPLICATION_JSON);
                callback(resp);
            },
            minPrice, maxPrice);
    }

    void staticFile(const HttpRequestPtr &req,
                    std::function<void(const HttpResponsePtr &)> &&callback,
                    const std::string &filename)
    {
        auto it = static_files.find(filename);
        if (it != static_files.end()) {
            auto resp = HttpResponse::newHttpResponse();
            resp->setBody(it->second.data);
            resp->setContentTypeString(it->second.content_type);
            callback(resp);
        } else {
            auto resp = HttpResponse::newHttpResponse();
            resp->setStatusCode(k404NotFound);
            callback(resp);
        }
    }
};

// ── Main ──

int main()
{
    loadDataset();
    loadDatasetLarge();
    loadStaticFiles();
    {
        sqlite3 *test = openDb();
        if (test) { db_available = true; sqlite3_close(test); }
    }
    {
        const char *dbUrl = getenv("DATABASE_URL");
        if (dbUrl) {
            // Parse postgres://user:pass@host:port/dbname
            std::string s(dbUrl);
            auto se = s.find("://");
            if (se != std::string::npos) {
                s = s.substr(se + 3);
                std::string user, pass, host = "localhost", port = "5432", dbname;
                auto at = s.find('@');
                if (at != std::string::npos) {
                    auto up = s.substr(0, at);
                    s = s.substr(at + 1);
                    auto c = up.find(':');
                    if (c != std::string::npos) { user = up.substr(0, c); pass = up.substr(c + 1); }
                    else user = up;
                }
                auto sl = s.find('/');
                if (sl != std::string::npos) { dbname = s.substr(sl + 1); s = s.substr(0, sl); }
                auto c = s.find(':');
                if (c != std::string::npos) { host = s.substr(0, c); port = s.substr(c + 1); }
                else host = s;
                int nconns = std::thread::hardware_concurrency();
                {
                    std::ifstream cg("/sys/fs/cgroup/cpu.max");
                    if (cg.is_open()) {
                        std::string quota_s, period_s;
                        if (cg >> quota_s >> period_s && quota_s != "max") {
                            int cg_cpus = std::stoi(quota_s) / std::stoi(period_s);
                            if (cg_cpus >= 1 && cg_cpus < nconns) nconns = cg_cpus;
                        }
                    }
                }
                if (nconns < 4) nconns = 4;
                pgClient = drogon::orm::DbClient::newPgClient(
                    "host=" + host + " port=" + port + " dbname=" + dbname +
                    " user=" + user + " password=" + pass, nconns);
                pg_available = true;
            }
        }
    }

    app().setLogLevel(trantor::Logger::kWarn);
    {
        int threads = std::thread::hardware_concurrency();
        std::ifstream cg("/sys/fs/cgroup/cpu.max");
        if (cg.is_open()) {
            std::string quota_s, period_s;
            if (cg >> quota_s >> period_s && quota_s != "max") {
                int cg_cpus = std::stoi(quota_s) / std::stoi(period_s);
                if (cg_cpus >= 1 && cg_cpus < threads) threads = cg_cpus;
            }
        }
        app().setThreadNum(threads);
    }
    app().setClientMaxBodySize(25 * 1024 * 1024);
    app().setIdleConnectionTimeout(0);
    app().setKeepaliveRequestsNumber(0);
    app().setGzipStatic(false);
    app().setServerHeaderField("drogon");
    app().addListener("0.0.0.0", 8080);

    const char *cert = getenv("TLS_CERT");
    const char *key = getenv("TLS_KEY");
    if (!cert) cert = "/certs/server.crt";
    if (!key) key = "/certs/server.key";
    if (access(cert, R_OK) == 0 && access(key, R_OK) == 0)
        app().addListener("0.0.0.0", 8443, true, cert, key);

    app().enableGzip(true);

    app().run();
    return 0;
}
