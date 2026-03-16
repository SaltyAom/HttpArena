using System.Buffers;
using System.Buffers.Text;
using System.IO.Compression;
using System.IO.Pipelines;
using System.Runtime.CompilerServices;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.Data.Sqlite;
using zerg;
using zerg.Engine;
using zerg.Engine.Configs;
using Zerg.Core;

// ── Data models ──

public class DatasetItem
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    public string Category { get; set; } = "";
    public double Price { get; set; }
    public int Quantity { get; set; }
    public bool Active { get; set; }
    public List<string> Tags { get; set; } = new();
    public RatingInfo Rating { get; set; } = new();
}

public class RatingInfo
{
    public double Score { get; set; }
    public int Count { get; set; }
}

public class ProcessedItem
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    public string Category { get; set; } = "";
    public double Price { get; set; }
    public int Quantity { get; set; }
    public bool Active { get; set; }
    public List<string> Tags { get; set; } = new();
    public RatingInfo Rating { get; set; } = new();
    public double Total { get; set; }
}

[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    GenerationMode = JsonSourceGenerationMode.Default)]
[JsonSerializable(typeof(JsonResponse))]
[JsonSerializable(typeof(DbResponse))]
[JsonSerializable(typeof(List<DatasetItem>))]
partial class AppJsonContext : JsonSerializerContext { }

public class JsonResponse
{
    public List<ProcessedItem> Items { get; set; } = new();
    public int Count { get; set; }
}

public class DbResponse
{
    public List<DbItem> Items { get; set; } = new();
    public int Count { get; set; }
}

public class DbItem
{
    public int Id { get; set; }
    public string Name { get; set; } = "";
    public string Category { get; set; } = "";
    public double Price { get; set; }
    public int Quantity { get; set; }
    public bool Active { get; set; }
    public List<string> Tags { get; set; } = new();
    public DbRating Rating { get; set; } = new();
}

public class DbRating
{
    public double Score { get; set; }
    public int Count { get; set; }
}

// ── Shared app data ──

static class AppData
{
    public static List<DatasetItem> Dataset = new();
    public static byte[] JsonCache = Array.Empty<byte>();
    public static byte[] LargeJsonCache = Array.Empty<byte>();
    public static Dictionary<string, (byte[] Data, string ContentType)> StaticFiles = new();
    public static SqliteConnection? Db;

    public static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public static void Load()
    {
        // Dataset
        var path = Environment.GetEnvironmentVariable("DATASET_PATH") ?? "/data/dataset.json";
        if (File.Exists(path))
        {
            Dataset = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(path), JsonOpts) ?? new();
            JsonCache = BuildJsonCache(Dataset);
        }

        // Large dataset for compression
        var largePath = "/data/dataset-large.json";
        if (File.Exists(largePath))
        {
            var largeItems = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(largePath), JsonOpts) ?? new();
            LargeJsonCache = BuildJsonCache(largeItems);
        }

        // Static files
        if (Directory.Exists("/data/static"))
        {
            var mimeTypes = new Dictionary<string, string>
            {
                {".css","text/css"},{".js","application/javascript"},{".html","text/html"},
                {".woff2","font/woff2"},{".svg","image/svg+xml"},{".webp","image/webp"},{".json","application/json"}
            };
            foreach (var file in Directory.GetFiles("/data/static"))
            {
                var name = Path.GetFileName(file);
                var ext = Path.GetExtension(file);
                var ct = mimeTypes.GetValueOrDefault(ext, "application/octet-stream");
                StaticFiles[name] = (File.ReadAllBytes(file), ct);
            }
        }

        // Database
        var dbPath = "/data/benchmark.db";
        if (File.Exists(dbPath))
        {
            Db = new SqliteConnection($"Data Source={dbPath};Mode=ReadOnly");
            Db.Open();
            using var pragma = Db.CreateCommand();
            pragma.CommandText = "PRAGMA mmap_size=268435456";
            pragma.ExecuteNonQuery();
        }
    }

    static byte[] BuildJsonCache(List<DatasetItem> items)
    {
        var processed = items.Select(d => new ProcessedItem
        {
            Id = d.Id, Name = d.Name, Category = d.Category,
            Price = d.Price, Quantity = d.Quantity, Active = d.Active,
            Tags = d.Tags, Rating = d.Rating,
            Total = Math.Round(d.Price * d.Quantity, 2)
        }).ToList();
        var resp = new JsonResponse { Items = processed, Count = processed.Count };
        return JsonSerializer.SerializeToUtf8Bytes(resp, AppJsonContext.Default.JsonResponse);
    }
}

// ── Date header (updated every second) ──

static class DateHelper
{
    private const int PrefixLen = 6;
    private const int DateLen = 29;
    private static byte[] _master = new byte[PrefixLen + DateLen + 4];
    private static byte[] _scratch = new byte[PrefixLen + DateLen + 4];
    private static readonly Timer _timer;

    static DateHelper()
    {
        "Date: "u8.CopyTo(_master);
        "Date: "u8.CopyTo(_scratch);
        _master[PrefixLen + DateLen] = (byte)'\r';
        _master[PrefixLen + DateLen + 1] = (byte)'\n';
        _master[PrefixLen + DateLen + 2] = (byte)'\r';
        _master[PrefixLen + DateLen + 3] = (byte)'\n';
        _scratch[PrefixLen + DateLen] = (byte)'\r';
        _scratch[PrefixLen + DateLen + 1] = (byte)'\n';
        _scratch[PrefixLen + DateLen + 2] = (byte)'\r';
        _scratch[PrefixLen + DateLen + 3] = (byte)'\n';
        SetDate(DateTimeOffset.UtcNow);
        _timer = new Timer(_ => SetDate(DateTimeOffset.UtcNow), null, 1000, 1000);
    }

    private static void SetDate(DateTimeOffset value)
    {
        lock (_scratch)
        {
            Utf8Formatter.TryFormat(value, _scratch.AsSpan(PrefixLen), out _, 'R');
            (_scratch, _master) = (_master, _scratch);
        }
    }

    public static ReadOnlySpan<byte> HeaderBytes => _master;
}

// ── HTTP request parsing ──

readonly struct HttpRequest
{
    public readonly ReadOnlyMemory<byte> Method;
    public readonly ReadOnlyMemory<byte> Path;
    public readonly ReadOnlyMemory<byte> Query;
    public readonly ReadOnlyMemory<byte> Body;
    public readonly int TotalLength;
    public readonly int ContentLength;
    public readonly bool AcceptsGzip;

    public HttpRequest(ReadOnlyMemory<byte> method, ReadOnlyMemory<byte> path,
        ReadOnlyMemory<byte> query, ReadOnlyMemory<byte> body,
        int totalLength, int contentLength, bool acceptsGzip)
    {
        Method = method; Path = path; Query = query; Body = body;
        TotalLength = totalLength; ContentLength = contentLength;
        AcceptsGzip = acceptsGzip;
    }
}

static class HttpParser
{
    // Returns null if buffer doesn't contain a complete request yet
    public static HttpRequest? TryParse(ReadOnlySequence<byte> buffer)
    {
        // Find end of headers
        var span = buffer.IsSingleSegment ? buffer.FirstSpan : buffer.ToArray().AsSpan();
        return TryParse(span, buffer);
    }

    public static HttpRequest? TryParse(ReadOnlySpan<byte> span, ReadOnlySequence<byte> buffer)
    {
        int headerEnd = span.IndexOf("\r\n\r\n"u8);
        if (headerEnd < 0) return null;

        int headersLen = headerEnd + 4;

        // Parse request line: METHOD SP PATH[?QUERY] SP HTTP/x.x\r\n
        int firstSpace = span.IndexOf((byte)' ');
        if (firstSpace < 0) return null;
        var method = span[..firstSpace];

        int pathStart = firstSpace + 1;
        int secondSpace = span[pathStart..].IndexOf((byte)' ');
        if (secondSpace < 0) return null;
        var uri = span[pathStart..(pathStart + secondSpace)];

        ReadOnlySpan<byte> path;
        ReadOnlySpan<byte> query = default;
        int qmark = uri.IndexOf((byte)'?');
        if (qmark >= 0)
        {
            path = uri[..qmark];
            query = uri[(qmark + 1)..];
        }
        else
        {
            path = uri;
        }

        // Parse Content-Length if present
        int contentLength = 0;
        bool acceptsGzip = false;
        var headers = span[..headerEnd];
        int pos = 0;
        while (pos < headers.Length)
        {
            int lineEnd = headers[pos..].IndexOf("\r\n"u8);
            if (lineEnd < 0) break;
            var line = headers[pos..(pos + lineEnd)];
            pos += lineEnd + 2;

            if (line.Length > 16 &&
                (line[0] == (byte)'C' || line[0] == (byte)'c') &&
                (line[8] == (byte)'L' || line[8] == (byte)'l'))
            {
                // Content-Length:
                int colon = line.IndexOf((byte)':');
                if (colon >= 0)
                {
                    var val = line[(colon + 1)..];
                    // Trim leading space
                    while (val.Length > 0 && val[0] == (byte)' ') val = val[1..];
                    if (Utf8Parser.TryParse(val, out int cl, out _))
                        contentLength = cl;
                }
            }
            else if (line.Length > 15 &&
                (line[0] == (byte)'A' || line[0] == (byte)'a') &&
                (line[7] == (byte)'E' || line[7] == (byte)'e'))
            {
                // Accept-Encoding:
                if (line.IndexOf("gzip"u8) >= 0)
                    acceptsGzip = true;
            }
        }

        int totalLen = headersLen + contentLength;
        if (span.Length < totalLen) return null; // body not yet complete

        ReadOnlyMemory<byte> body = default;
        if (contentLength > 0)
        {
            body = buffer.Slice(headersLen, contentLength).ToArray();
        }

        return new HttpRequest(
            method.ToArray(),
            path.ToArray(),
            query.Length > 0 ? query.ToArray() : ReadOnlyMemory<byte>.Empty,
            body,
            totalLen,
            contentLength,
            acceptsGzip);
    }
}

// ── Response writing ──

static class HttpResponse
{
    static ReadOnlySpan<byte> ServerHeader => "Server: zerg\r\n"u8;

    public static void WriteText(Connection conn, ReadOnlySpan<byte> body, int statusCode = 200)
    {
        Span<byte> lenBuf = stackalloc byte[16];
        Utf8Formatter.TryFormat(body.Length, lenBuf, out int lenWritten);

        conn.Write(statusCode == 200
            ? "HTTP/1.1 200 OK\r\n"u8
            : "HTTP/1.1 404 Not Found\r\n"u8);
        conn.Write(ServerHeader);
        conn.Write("Content-Type: text/plain\r\nContent-Length: "u8);
        conn.Write(lenBuf[..lenWritten]);
        conn.Write("\r\n"u8);
        conn.Write(DateHelper.HeaderBytes);
        conn.Write(body);
    }

    public static void WriteJson(Connection conn, byte[] body, bool compress, bool acceptsGzip)
    {
        if (compress && acceptsGzip && body.Length > 256)
        {
            using var ms = new MemoryStream();
            using (var gz = new GZipStream(ms, CompressionLevel.Fastest, true))
                gz.Write(body);
            var compressed = ms.ToArray();

            Span<byte> lenBuf = stackalloc byte[16];
            Utf8Formatter.TryFormat(compressed.Length, lenBuf, out int lenWritten);

            conn.Write("HTTP/1.1 200 OK\r\n"u8);
            conn.Write(ServerHeader);
            conn.Write("Content-Type: application/json\r\nContent-Encoding: gzip\r\nContent-Length: "u8);
            conn.Write(lenBuf[..lenWritten]);
            conn.Write("\r\n"u8);
            conn.Write(DateHelper.HeaderBytes);
            conn.Write(compressed);
        }
        else
        {
            Span<byte> lenBuf = stackalloc byte[16];
            Utf8Formatter.TryFormat(body.Length, lenBuf, out int lenWritten);

            conn.Write("HTTP/1.1 200 OK\r\n"u8);
            conn.Write(ServerHeader);
            conn.Write("Content-Type: application/json\r\nContent-Length: "u8);
            conn.Write(lenBuf[..lenWritten]);
            conn.Write("\r\n"u8);
            conn.Write(DateHelper.HeaderBytes);
            conn.Write(body);
        }
    }

    public static void WriteBytes(Connection conn, byte[] body, string contentType)
    {
        Span<byte> lenBuf = stackalloc byte[16];
        Utf8Formatter.TryFormat(body.Length, lenBuf, out int lenWritten);

        conn.Write("HTTP/1.1 200 OK\r\n"u8);
        conn.Write(ServerHeader);
        conn.Write("Content-Type: "u8);
        conn.Write(Encoding.UTF8.GetBytes(contentType));
        conn.Write("\r\nContent-Length: "u8);
        conn.Write(lenBuf[..lenWritten]);
        conn.Write("\r\n"u8);
        conn.Write(DateHelper.HeaderBytes);
        conn.Write(body);
    }

    public static void Write404(Connection conn)
    {
        conn.Write("HTTP/1.1 404 Not Found\r\n"u8);
        conn.Write(ServerHeader);
        conn.Write("Content-Length: 9\r\n"u8);
        conn.Write(DateHelper.HeaderBytes);
        conn.Write("Not Found"u8);
    }

    public static void Write500(Connection conn, string msg)
    {
        var body = Encoding.UTF8.GetBytes(msg);
        Span<byte> lenBuf = stackalloc byte[16];
        Utf8Formatter.TryFormat(body.Length, lenBuf, out int lenWritten);

        conn.Write("HTTP/1.1 500 Internal Server Error\r\n"u8);
        conn.Write(ServerHeader);
        conn.Write("Content-Type: text/plain\r\nContent-Length: "u8);
        conn.Write(lenBuf[..lenWritten]);
        conn.Write("\r\n"u8);
        conn.Write(DateHelper.HeaderBytes);
        conn.Write(body);
    }
}

// ── Route handling ──

static class Router
{
    public static void Handle(Connection conn, in HttpRequest req)
    {
        var path = req.Path.Span;

        if (path.SequenceEqual("/pipeline"u8))
            HandlePipeline(conn);
        else if (path.SequenceEqual("/baseline11"u8))
            HandleBaseline11(conn, req);
        else if (path.SequenceEqual("/baseline2"u8))
            HandleBaseline2(conn, req);
        else if (path.SequenceEqual("/json"u8))
            HandleJson(conn);
        else if (path.SequenceEqual("/compression"u8))
            HandleCompression(conn, req);
        else if (path.SequenceEqual("/db"u8))
            HandleDb(conn, req);
        else if (path.SequenceEqual("/upload"u8))
            HandleUpload(conn, req);
        else if (path.Length > 8 && path[..8].SequenceEqual("/static/"u8))
            HandleStatic(conn, path);
        else
            HttpResponse.Write404(conn);
    }

    static void HandlePipeline(Connection conn)
    {
        HttpResponse.WriteText(conn, "ok"u8);
    }

    static void HandleBaseline11(Connection conn, in HttpRequest req)
    {
        long sum = SumQuery(req.Query.Span);

        // POST: add body value
        if (req.Method.Span.SequenceEqual("POST"u8) && req.Body.Length > 0)
        {
            var bodyStr = Encoding.UTF8.GetString(req.Body.Span).Trim();
            if (long.TryParse(bodyStr, out long bval))
                sum += bval;
        }

        HttpResponse.WriteText(conn, Encoding.UTF8.GetBytes(sum.ToString()));
    }

    static void HandleBaseline2(Connection conn, in HttpRequest req)
    {
        long sum = SumQuery(req.Query.Span);
        HttpResponse.WriteText(conn, Encoding.UTF8.GetBytes(sum.ToString()));
    }

    static void HandleJson(Connection conn)
    {
        if (AppData.JsonCache.Length == 0)
        {
            HttpResponse.Write500(conn, "Dataset not loaded");
            return;
        }
        HttpResponse.WriteJson(conn, AppData.JsonCache, false, false);
    }

    static void HandleCompression(Connection conn, in HttpRequest req)
    {
        if (AppData.LargeJsonCache.Length == 0)
        {
            HttpResponse.Write500(conn, "Dataset not loaded");
            return;
        }
        HttpResponse.WriteJson(conn, AppData.LargeJsonCache, true, req.AcceptsGzip);
    }

    static void HandleDb(Connection conn, in HttpRequest req)
    {
        if (AppData.Db == null)
        {
            HttpResponse.Write500(conn, "Database not available");
            return;
        }

        double min = 10, max = 50;
        var query = req.Query.Span;
        if (query.Length > 0)
        {
            var qs = Encoding.UTF8.GetString(query);
            foreach (var pair in qs.Split('&'))
            {
                if (pair.StartsWith("min=") && double.TryParse(pair[4..], out double pmin))
                    min = pmin;
                else if (pair.StartsWith("max=") && double.TryParse(pair[4..], out double pmax))
                    max = pmax;
            }
        }

        lock (AppData.Db)
        {
            using var cmd = AppData.Db.CreateCommand();
            cmd.CommandText = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN @min AND @max LIMIT 50";
            cmd.Parameters.AddWithValue("@min", min);
            cmd.Parameters.AddWithValue("@max", max);
            using var reader = cmd.ExecuteReader();

            var items = new List<DbItem>();
            while (reader.Read())
            {
                items.Add(new DbItem
                {
                    Id = reader.GetInt32(0),
                    Name = reader.GetString(1),
                    Category = reader.GetString(2),
                    Price = reader.GetDouble(3),
                    Quantity = reader.GetInt32(4),
                    Active = reader.GetInt32(5) == 1,
                    Tags = JsonSerializer.Deserialize<List<string>>(reader.GetString(6)) ?? new(),
                    Rating = new DbRating { Score = reader.GetDouble(7), Count = reader.GetInt32(8) }
                });
            }

            var resp = new DbResponse { Items = items, Count = items.Count };
            var body = JsonSerializer.SerializeToUtf8Bytes(resp, AppJsonContext.Default.DbResponse);
            HttpResponse.WriteJson(conn, body, false, false);
        }
    }

    static void HandleUpload(Connection conn, in HttpRequest req)
    {
        HttpResponse.WriteText(conn, Encoding.UTF8.GetBytes(req.ContentLength.ToString()));
    }

    static void HandleStatic(Connection conn, ReadOnlySpan<byte> path)
    {
        var filename = Encoding.UTF8.GetString(path[8..]);
        if (AppData.StaticFiles.TryGetValue(filename, out var sf))
            HttpResponse.WriteBytes(conn, sf.Data, sf.ContentType);
        else
            HttpResponse.Write404(conn);
    }

    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    static long SumQuery(ReadOnlySpan<byte> query)
    {
        if (query.IsEmpty) return 0;
        long sum = 0;
        var qs = Encoding.UTF8.GetString(query);
        foreach (var pair in qs.Split('&'))
        {
            var eq = pair.IndexOf('=');
            if (eq >= 0 && long.TryParse(pair[(eq + 1)..], out long n))
                sum += n;
        }
        return sum;
    }
}

// ── Connection handler ──

static class ConnectionHandler
{
    internal static async Task HandleAsync(Connection connection)
    {
        var reader = new ConnectionPipeReader(connection);

        try
        {
            while (true)
            {
                var result = await reader.ReadAsync();
                if (result.IsCompleted || result.IsCanceled)
                    break;

                var buffer = result.Buffer;
                bool wrote = false;

                while (buffer.Length > 0)
                {
                    var req = HttpParser.TryParse(buffer);
                    if (req == null) break;

                    Router.Handle(connection, req.Value);
                    wrote = true;
                    buffer = buffer.Slice(req.Value.TotalLength);
                }

                reader.AdvanceTo(buffer.Start, buffer.End);

                if (wrote)
                    await connection.FlushAsync();
            }
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine($"Connection error: {ex.Message}");
        }

        reader.Complete();
    }
}

// ── Entry point ──

AppData.Load();

int reactorCount = Environment.ProcessorCount;
if (args.Length > 0 && int.TryParse(args[0], out int rc))
    reactorCount = rc;

Console.WriteLine($"zerg HttpArena server starting on :8080 with {reactorCount} reactors");

var engine = new Engine(new EngineOptions
{
    Ip = "0.0.0.0",
    Port = 8080,
    Backlog = 65535,
    ReactorCount = reactorCount,
    AcceptorConfig = new AcceptorConfig(
        RingFlags: 0,
        SqCpuThread: -1,
        SqThreadIdleMs: 100,
        RingEntries: 8 * 1024,
        BatchSqes: 4096,
        CqTimeout: 100_000_000,
        IPVersion: IPVersion.IPv6DualStack
    ),
    ReactorConfigs = Enumerable.Range(0, reactorCount).Select(_ => new ReactorConfig(
        RingFlags: (1u << 12) | (1u << 13), // SINGLE_ISSUER | DEFER_TASKRUN
        SqCpuThread: -1,
        SqThreadIdleMs: 100,
        RingEntries: 8 * 1024,
        RecvBufferSize: 16 * 1024,
        BufferRingEntries: 16 * 1024,
        BatchCqes: 4096,
        MaxConnectionsPerReactor: 8 * 1024,
        CqTimeout: 1_000_000,
        ConnectionBufferRingEntries: 32,
        IncrementalBufferConsumption: false
    )).ToArray()
});

engine.Listen();

var cts = new CancellationTokenSource();

try
{
    while (engine.ServerRunning)
    {
        var connection = await engine.AcceptAsync(cts.Token);
        if (connection is null) continue;
        _ = ConnectionHandler.HandleAsync(connection);
    }
}
catch (OperationCanceledException) { }

Console.WriteLine("Server stopped.");
