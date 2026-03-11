using System.Net;
using System.Security.Cryptography.X509Certificates;
using System.Text.Json;
using GenHTTP.Api.Content;
using GenHTTP.Api.Protocol;
using GenHTTP.Engine.Kestrel;
using GenHTTP.Modules.Compression;
using GenHTTP.Modules.Functional;
using GenHTTP.Modules.IO;
using GenHTTP.Modules.Layouting;
using Microsoft.Data.Sqlite;

// JSON options
var jsonOptions = new JsonSerializerOptions
{
    PropertyNameCaseInsensitive = true,
    PropertyNamingPolicy = JsonNamingPolicy.CamelCase
};

// Load small dataset — keep raw items for per-request processing
var datasetPath = Environment.GetEnvironmentVariable("DATASET_PATH") ?? "/data/dataset.json";
List<DatasetItem>? datasetItems = null;
if (File.Exists(datasetPath))
{
    datasetItems = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(datasetPath), jsonOptions);
}

// Load large dataset for compression — pre-serialize to bytes
byte[]? largeJsonBytes = null;
var largePath = "/data/dataset-large.json";
if (File.Exists(largePath))
{
    var largeItems = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(largePath), jsonOptions);
    if (largeItems != null)
    {
        var processed = largeItems.Select(d => new ProcessedItem
        {
            Id = d.Id, Name = d.Name, Category = d.Category,
            Price = d.Price, Quantity = d.Quantity, Active = d.Active,
            Tags = d.Tags, Rating = d.Rating,
            Total = Math.Round(d.Price * d.Quantity, 2)
        }).ToList();
        largeJsonBytes = JsonSerializer.SerializeToUtf8Bytes(new { items = processed, count = processed.Count }, jsonOptions);
    }
}

// Pre-load static files
var staticFileMap = new Dictionary<string, (byte[] Data, string ContentType)>();
var staticDir = "/data/static";
if (Directory.Exists(staticDir))
{
    var mimeTypes = new Dictionary<string, string>
    {
        {".css", "text/css"}, {".js", "application/javascript"}, {".html", "text/html"},
        {".woff2", "font/woff2"}, {".svg", "image/svg+xml"}, {".webp", "image/webp"}, {".json", "application/json"}
    };
    foreach (var file in Directory.GetFiles(staticDir))
    {
        var name = Path.GetFileName(file);
        var ext = Path.GetExtension(file);
        var ct = mimeTypes.GetValueOrDefault(ext, "application/octet-stream");
        staticFileMap[name] = (File.ReadAllBytes(file), ct);
    }
}

// Open SQLite database
SqliteConnection? dbConn = null;
var dbPath = "/data/benchmark.db";
if (File.Exists(dbPath))
{
    dbConn = new SqliteConnection($"Data Source={dbPath};Mode=ReadOnly");
    dbConn.Open();
    using var pragma = dbConn.CreateCommand();
    pragma.CommandText = "PRAGMA mmap_size=268435456";
    pragma.ExecuteNonQuery();
}

// Helper: sum query parameters
static int SumQuery(IRequest request)
{
    int sum = 0;
    foreach (var (_, value) in request.Query)
    {
        if (int.TryParse(value, out int n))
            sum += n;
    }
    return sum;
}

// Helper: build a response from a byte array
static IResponse ByteResponse(IRequest request, byte[] data, string contentType)
{
    return request.Respond()
        .Content(new MemoryStream(data))
        .Type(new FlexibleContentType(contentType))
        .Length((ulong)data.Length)
        .Header("Server", "genhttp")
        .Build();
}

// Build the handler tree
var api = Inline.Create()
    .Get("/pipeline", (IRequest request) =>
    {
        return request.Respond()
            .Content(Resource.FromString("ok").Build())
            .Type(new FlexibleContentType("text/plain"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Get("/baseline11", (IRequest request) =>
    {
        int sum = SumQuery(request);
        return request.Respond()
            .Content(Resource.FromString(sum.ToString()).Build())
            .Type(new FlexibleContentType("text/plain"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Post("/baseline11", async (IRequest request) =>
    {
        int sum = SumQuery(request);
        if (request.Content != null)
        {
            using var reader = new StreamReader(request.Content);
            var body = await reader.ReadToEndAsync();
            if (int.TryParse(body.Trim(), out int b))
                sum += b;
        }
        return request.Respond()
            .Content(Resource.FromString(sum.ToString()).Build())
            .Type(new FlexibleContentType("text/plain"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Get("/baseline2", (IRequest request) =>
    {
        int sum = SumQuery(request);
        return request.Respond()
            .Content(Resource.FromString(sum.ToString()).Build())
            .Type(new FlexibleContentType("text/plain"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Get("/json", (IRequest request) =>
    {
        if (datasetItems == null)
            return request.Respond().Status(500, "No dataset").Build();
        var processed = new List<ProcessedItem>(datasetItems.Count);
        foreach (var d in datasetItems)
        {
            processed.Add(new ProcessedItem
            {
                Id = d.Id, Name = d.Name, Category = d.Category,
                Price = d.Price, Quantity = d.Quantity, Active = d.Active,
                Tags = d.Tags, Rating = d.Rating,
                Total = Math.Round(d.Price * d.Quantity, 2)
            });
        }
        var json = JsonSerializer.Serialize(new { items = processed, count = processed.Count }, jsonOptions);
        return request.Respond()
            .Content(Resource.FromString(json).Build())
            .Type(new FlexibleContentType("application/json"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Get("/compression", (IRequest request) =>
    {
        if (largeJsonBytes == null)
            return request.Respond().Status(500, "No dataset").Build();
        return ByteResponse(request, largeJsonBytes, "application/json");
    })
    .Post("/upload", async (IRequest request) =>
    {
        using var ms = new MemoryStream();
        if (request.Content != null)
            await request.Content.CopyToAsync(ms);
        uint crc = Crc32Helper.Compute(ms.GetBuffer().AsSpan(0, (int)ms.Length));
        var hex = crc.ToString("x8");
        return request.Respond()
            .Content(Resource.FromString(hex).Build())
            .Type(new FlexibleContentType("text/plain"))
            .Header("Server", "genhttp")
            .Build();
    })
    .Get("/db", (IRequest request) =>
    {
        if (dbConn == null)
            return request.Respond().Status(500, "DB not available").Build();

        double min = 10, max = 50;
        if (request.Query.TryGetValue("min", out var minStr) && double.TryParse(minStr, out double pmin))
            min = pmin;
        if (request.Query.TryGetValue("max", out var maxStr) && double.TryParse(maxStr, out double pmax))
            max = pmax;

        using var cmd = dbConn.CreateCommand();
        cmd.CommandText = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN @min AND @max LIMIT 50";
        cmd.Parameters.AddWithValue("@min", min);
        cmd.Parameters.AddWithValue("@max", max);
        using var reader = cmd.ExecuteReader();
        var items = new List<object>();
        while (reader.Read())
        {
            items.Add(new
            {
                id = reader.GetInt32(0),
                name = reader.GetString(1),
                category = reader.GetString(2),
                price = reader.GetDouble(3),
                quantity = reader.GetInt32(4),
                active = reader.GetInt32(5) == 1,
                tags = JsonSerializer.Deserialize<List<string>>(reader.GetString(6)),
                rating = new { score = reader.GetDouble(7), count = reader.GetInt32(8) },
            });
        }
        var json = JsonSerializer.Serialize(new { items, count = items.Count }, jsonOptions);
        return request.Respond()
            .Content(Resource.FromString(json).Build())
            .Type(new FlexibleContentType("application/json"))
            .Header("Server", "genhttp")
            .Build();
    });

// Static file handler — register each file as a sub-route in a layout
var staticLayout = Layout.Create();
foreach (var (name, (data, contentType)) in staticFileMap)
{
    var fileData = data;
    var fileContentType = contentType;
    staticLayout.Add(name, Inline.Create()
        .Get((IRequest request) => ByteResponse(request, fileData, fileContentType)));
}

var layout = Layout.Create()
    .Add(api)
    .Add("static", staticLayout);

// TLS configuration
var certPath = Environment.GetEnvironmentVariable("TLS_CERT") ?? "/certs/server.crt";
var keyPath = Environment.GetEnvironmentVariable("TLS_KEY") ?? "/certs/server.key";
var hasCert = File.Exists(certPath) && File.Exists(keyPath);

var host = Host.Create()
    .Handler(layout)
    .Compression(CompressedContent.Default());

host.Bind(IPAddress.Any, 8080);

if (hasCert)
{
    var cert = X509Certificate2.CreateFromPemFile(certPath, keyPath);
    host.Bind(IPAddress.Any, 8443, cert, enableQuic: true);
}

await host.RunAsync();

// --- Data models ---

class DatasetItem
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

class ProcessedItem
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

class RatingInfo
{
    public double Score { get; set; }
    public int Count { get; set; }
}

static class Crc32Helper
{
    private static readonly uint[][] T = new uint[8][];
    static Crc32Helper()
    {
        for (int s = 0; s < 8; s++) T[s] = new uint[256];
        for (uint i = 0; i < 256; i++)
        {
            uint c = i;
            for (int j = 0; j < 8; j++)
                c = (c >> 1) ^ (0xEDB88320u & (0u - (c & 1u)));
            T[0][i] = c;
        }
        for (uint i = 0; i < 256; i++)
            for (int s = 1; s < 8; s++)
                T[s][i] = (T[s-1][i] >> 8) ^ T[0][T[s-1][i] & 0xFF];
    }
    public static uint Compute(ReadOnlySpan<byte> data)
    {
        uint crc = 0xFFFFFFFF;
        int i = 0;
        while (i + 8 <= data.Length)
        {
            uint a = (uint)(data[i] | (data[i+1] << 8) | (data[i+2] << 16) | (data[i+3] << 24)) ^ crc;
            uint b = (uint)(data[i+4] | (data[i+5] << 8) | (data[i+6] << 16) | (data[i+7] << 24));
            crc = T[7][a & 0xFF] ^ T[6][(a >> 8) & 0xFF]
                ^ T[5][(a >> 16) & 0xFF] ^ T[4][a >> 24]
                ^ T[3][b & 0xFF] ^ T[2][(b >> 8) & 0xFF]
                ^ T[1][(b >> 16) & 0xFF] ^ T[0][b >> 24];
            i += 8;
        }
        while (i < data.Length)
            crc = (crc >> 8) ^ T[0][(crc ^ data[i++]) & 0xFF];
        return crc ^ 0xFFFFFFFF;
    }
}
