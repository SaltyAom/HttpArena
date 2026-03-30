# frozen_string_literal: true

require 'bundler/setup'
Bundler.require(:default)

require 'zlib'

class App < Roda
  # Load dataset
  dataset_path = ENV.fetch('DATASET_PATH', '/data/dataset.json')
  if File.exist?(dataset_path)
    opts[:dataset_items] = JSON.parse(File.read(dataset_path))
  end

  # Large dataset for compression
  large_path = '/data/dataset-large.json'
  if File.exist?(large_path)
    raw = JSON.parse(File.read(large_path))
    items = raw.map do |d|
      d.merge('total' => (d['price'] * d['quantity'] * 100).round / 100.0)
    end
    opts[:large_json_payload] = JSON.generate({ 'items' => items, 'count' => items.length })
  end

  # SQLite
  opts[:db_available] = File.exist?('/data/benchmark.db')

  DB_QUERY = 'SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN ? AND ? LIMIT 50'

  plugin :default_headers, 'Server' => 'roda'
  plugin :halt
  plugin :streaming

  route do |r|
    r.root { 'ok' }

    r.is 'pipeline' do
      response[RodaResponseHeaders::CONTENT_TYPE] = 'text/plain'
      'ok'
    end

    r.is('baseline11') { handle_baseline11 }

    r.is 'baseline2' do
      total = 0
      request.GET.each do |_k, v|
        total += v.to_i
      end
      response[RodaResponseHeaders::CONTENT_TYPE] = 'text/plain'
      total.to_s
    end

    r.is 'json' do
      dataset = opts[:dataset_items]
      r.halt 500, 'No dataset' unless dataset
      items = dataset.map do |d|
        d.merge('total' => (d['price'] * d['quantity'] * 100).round / 100.0)
      end
      response[RodaResponseHeaders::CONTENT_TYPE] = 'application/json'
      JSON.generate({ 'items' => items, 'count' => items.length })
    end

    r.is 'compression' do
      payload = opts[:large_json_payload]
      r.halt 500, 'No dataset' unless payload
      sio = StringIO.new
      gz = Zlib::GzipWriter.new(sio, 1)
      gz.write(payload)
      gz.close
      response[RodaResponseHeaders::CONTENT_TYPE] = 'application/json'
      response[RodaResponseHeaders::CONTENT_ENCODING] = 'gzip'
      sio.string
    end

    r.is 'upload' do
      size = 0
      buf = request.body
      while (chunk = buf.read(65536))
        size += chunk.bytesize
      end
      size.to_s
    end

    r.is 'db' do
      unless opts[:db_available]
        response[RodaResponseHeaders::CONTENT_TYPE] = 'application/json'
        return '{"items":[],"count":0}'
      end
      min_val = (request.params['min'] || 10).to_i
      max_val = (request.params['max'] || 50).to_i
      db = get_db
      rows = db.execute(DB_QUERY, [min_val, max_val])
      items = rows.map do |row|
        {
          'id' => row['id'], 'name' => row['name'], 'category' => row['category'],
          'price' => row['price'], 'quantity' => row['quantity'], 'active' => row['active'] == 1,
          'tags' => JSON.parse(row['tags']),
          'rating' => { 'score' => row['rating_score'], 'count' => row['rating_count'] }
        }
      end
      response[RodaResponseHeaders::CONTENT_TYPE] = 'application/json'
      JSON.generate({ 'items' => items, 'count' => items.length })
    end
  end

  def handle_baseline11
    total = 0
    request.GET.each do |_k, v|
      total += v.to_i
    end
    if request.post?
      request.body.rewind
      body_str = request.body.read.strip
      total += body_str.to_i
    end
    response[RodaResponseHeaders::CONTENT_TYPE] = 'text/plain'
    total.to_s
  end

  def get_db
    Thread.current[:roda_db] ||= begin
      db = SQLite3::Database.new('/data/benchmark.db', readonly: true)
      db.execute('PRAGMA mmap_size=268435456')
      db.results_as_hash = true
      db
    end
  end
end
