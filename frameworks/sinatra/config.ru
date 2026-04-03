require_relative 'app'

# Rack middleware to handle unknown HTTP methods before Puma/Sinatra
class MethodGuard
  KNOWN = %w[GET POST PUT DELETE PATCH HEAD OPTIONS TRACE CONNECT].freeze

  def initialize(app)
    @app = app
  end

  def call(env)
    if KNOWN.include?(env['REQUEST_METHOD'])
      @app.call(env)
    else
      [405, { 'content-type' => 'text/plain', 'server' => 'sinatra' }, ['Method Not Allowed']]
    end
  end
end

# Middleware to handle POST /upload directly — bypasses Rack/Sinatra param
# parsing which chokes on large binary POST bodies with no Content-Type header
# (Rack tries to URL-decode the binary body, fails on invalid %-encoding)
class UploadHandler
  def initialize(app)
    @app = app
  end

  def call(env)
    if env['REQUEST_METHOD'] == 'POST' && env['PATH_INFO'] == '/upload'
      input = env['rack.input']
      input.rewind
      bytes = 0
      while (chunk = input.read(4096))
        bytes += chunk.bytesize
      end
      [200, { 'content-type' => 'text/plain', 'server' => 'sinatra' }, [bytes.to_s]]
    else
      @app.call(env)
    end
  end
end

use MethodGuard
use UploadHandler
run App
