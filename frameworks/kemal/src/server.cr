require "kemal"
require "json"
require "compress/gzip"
require "sqlite3"
require "db"
require "pg"


# ---------------------------------------------------------------------------
# Startup: load datasets once
# ---------------------------------------------------------------------------

DATASET_PATH       = ENV.fetch("DATASET_PATH", "/data/dataset.json")
LARGE_DATASET_PATH = "/data/dataset-large.json"
DB_PATH            = "/data/benchmark.db"
STATIC_DIR         = "/data/static"
DB_QUERY           = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50"

# Process a raw JSON array of items → add "total" field
def process_items(raw : Array(JSON::Any)) : Array(JSON::Any)
  raw.map do |item|
    obj = item.as_h.dup
    price = item["price"].as_f
    quantity = item["quantity"].as_i
    obj["total"] = JSON::Any.new((price * quantity * 100).round / 100)
    JSON::Any.new(obj)
  end
end

# Small dataset – processed per-request like Flask does
DATASET_ITEMS = begin
  if File.exists?(DATASET_PATH)
    Array(JSON::Any).from_json(File.read(DATASET_PATH))
  else
    nil
  end
rescue
  nil
end

# Large dataset – pre-process at startup, compress per-request
LARGE_PAYLOAD = begin
  if File.exists?(LARGE_DATASET_PATH)
    raw = Array(JSON::Any).from_json(File.read(LARGE_DATASET_PATH))
    items = process_items(raw)
    {items: items, count: items.size}.to_json
  else
    nil
  end
rescue
  nil
end

DB_AVAILABLE = File.exists?(DB_PATH)

# Load static files into memory at startup
MIME_TYPES = {
  ".css"   => "text/css",
  ".js"    => "application/javascript",
  ".html"  => "text/html",
  ".woff2" => "font/woff2",
  ".svg"   => "image/svg+xml",
  ".webp"  => "image/webp",
  ".json"  => "application/json",
}

STATIC_FILES = begin
  files = Hash(String, {data: Bytes, content_type: String}).new
  if Dir.exists?(STATIC_DIR)
    Dir.each_child(STATIC_DIR) do |name|
      path = "#{STATIC_DIR}/#{name}"
      next unless File.file?(path)
      data = File.read(path).to_slice
      ext = File.extname(name)
      ct = MIME_TYPES.fetch(ext, "application/octet-stream")
      files[name] = {data: data, content_type: ct}
    end
  end
  files
end

PG_QUERY = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50"

PG_POOL_SIZE = begin
  cpus = System.cpu_count.to_i
  if File.exists?("/sys/fs/cgroup/cpu.max")
    parts = File.read("/sys/fs/cgroup/cpu.max").strip.split(' ')
    if parts[0] != "max" && parts.size == 2
      cg = parts[0].to_i // parts[1].to_i
      cpus = cg if cg >= 1 && cg < cpus
    end
  end
  {cpus * 4, 64}.max
end

class PgPool
  @@db : DB::Database? = nil

  def self.get : DB::Database?
    return @@db if @@db
    if url = ENV["DATABASE_URL"]?
      separator = url.includes?('?') ? '&' : '?'
      pg_url = "#{url}#{separator}max_pool_size=#{PG_POOL_SIZE}&max_idle_pool_size=#{PG_POOL_SIZE}"
      @@db = DB.open(pg_url)
    end
    @@db
  rescue
    nil
  end
end

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

macro server_header(env)
  {{env}}.response.headers["Server"] = "kemal"
end

# Thread-local (fiber-local) DB connections
class DBPool
  @@connections = Hash(UInt64, DB::Database).new

  def self.get : DB::Database
    fid = Fiber.current.object_id
    @@connections[fid] ||= DB.open("sqlite3://#{DB_PATH}?mode=ro")
  end
end

# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------

get "/pipeline" do |env|
  server_header(env)
  env.response.content_type = "text/plain"
  "ok"
end

get "/baseline11" do |env|
  server_header(env)
  total = 0_i64
  env.params.query.each do |_key, value|
    total += value.to_i64 rescue next
  end
  env.response.content_type = "text/plain"
  total.to_s
end

post "/baseline11" do |env|
  server_header(env)
  total = 0_i64
  env.params.query.each do |_key, value|
    total += value.to_i64 rescue next
  end
  body = env.request.body.try(&.gets_to_end)
  if body && !body.empty?
    begin
      total += body.strip.to_i64
    rescue
    end
  end
  env.response.content_type = "text/plain"
  total.to_s
end

get "/baseline2" do |env|
  server_header(env)
  total = 0_i64
  env.params.query.each do |_key, value|
    total += value.to_i64 rescue next
  end
  env.response.content_type = "text/plain"
  total.to_s
end

get "/json" do |env|
  server_header(env)
  if items_raw = DATASET_ITEMS
    items = process_items(items_raw)
    env.response.content_type = "application/json"
    {items: items, count: items.size}.to_json
  else
    env.response.status_code = 500
    "No dataset"
  end
end

get "/compression" do |env|
  server_header(env)
  if payload = LARGE_PAYLOAD
    env.response.content_type = "application/json"
    env.response.headers["Content-Encoding"] = "gzip"
    Compress::Gzip::Writer.open(env.response, level: Compress::Gzip::BEST_SPEED) do |gz|
      gz.print payload
    end
    nil
  else
    env.response.status_code = 500
    "No dataset"
  end
end

get "/db" do |env|
  server_header(env)
  env.response.content_type = "application/json"

  unless DB_AVAILABLE
    next %({"items":[],"count":0})
  end

  min_val = (env.params.query["min"]?.try(&.to_f) || 10.0)
  max_val = (env.params.query["max"]?.try(&.to_f) || 50.0)

  db = DBPool.get
  items = [] of Hash(String, JSON::Any)

  db.query(DB_QUERY, min_val, max_val) do |rs|
    rs.each do
      item = Hash(String, JSON::Any).new
      item["id"] = JSON::Any.new(rs.read(Int64))
      item["name"] = JSON::Any.new(rs.read(String))
      item["category"] = JSON::Any.new(rs.read(String))
      item["price"] = JSON::Any.new(rs.read(Float64))
      item["quantity"] = JSON::Any.new(rs.read(Int64))
      item["active"] = JSON::Any.new(rs.read(Int64) != 0)
      tags_raw = rs.read(String)
      item["tags"] = JSON::Any.new(Array(JSON::Any).from_json(tags_raw))
      rating_score = rs.read(Float64)
      rating_count = rs.read(Int64)
      item["rating"] = JSON::Any.new({"score" => JSON::Any.new(rating_score), "count" => JSON::Any.new(rating_count)})
      items << item
    end
  end

  {items: items, count: items.size}.to_json
end

get "/async-db" do |env|
  server_header(env)
  env.response.content_type = "application/json"

  unless pg = PgPool.get
    next %({"items":[],"count":0})
  end

  min_val = (env.params.query["min"]?.try(&.to_f) || 10.0)
  max_val = (env.params.query["max"]?.try(&.to_f) || 50.0)

  begin
    items = [] of Hash(String, JSON::Any)

    pg.query(PG_QUERY, min_val, max_val) do |rs|
      rs.each do
        item = Hash(String, JSON::Any).new
        item["id"] = JSON::Any.new(rs.read(Int32).to_i64)
        item["name"] = JSON::Any.new(rs.read(String))
        item["category"] = JSON::Any.new(rs.read(String))
        item["price"] = JSON::Any.new(rs.read(Float64))
        item["quantity"] = JSON::Any.new(rs.read(Int32).to_i64)
        item["active"] = JSON::Any.new(rs.read(Bool))
        item["tags"] = rs.read(JSON::Any)
        rating_score = rs.read(Float64)
        rating_count = rs.read(Int32).to_i64
        item["rating"] = JSON::Any.new({"score" => JSON::Any.new(rating_score), "count" => JSON::Any.new(rating_count)})
        items << item
      end
    end

    {items: items, count: items.size}.to_json
  rescue
    %({"items":[],"count":0})
  end
end

get "/static/:filename" do |env|
  server_header(env)
  filename = env.params.url["filename"]
  if sf = STATIC_FILES[filename]?
    env.response.content_type = sf[:content_type]
    env.response.write(sf[:data])
    nil
  else
    env.response.status_code = 404
    "Not Found"
  end
end

post "/upload" do |env|
  server_header(env)
  size = 0
  if body_io = env.request.body
    buf = Bytes.new(65536)
    while (n = body_io.read(buf)) > 0
      size += n
    end
  end
  env.response.content_type = "text/plain"
  size.to_s
end

# ---------------------------------------------------------------------------
# Server config
# ---------------------------------------------------------------------------

Kemal.config.port = 8080
Kemal.config.env = "production"
Kemal.config.shutdown_message = false
Kemal.config.logging = false

Kemal.run
