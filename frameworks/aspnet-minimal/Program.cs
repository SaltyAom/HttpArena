using System.Text.Json;
using System.Security.Cryptography.X509Certificates;
using Microsoft.AspNetCore.Server.Kestrel.Core;

var builder = WebApplication.CreateBuilder(args);
builder.Logging.ClearProviders();

var certPath = Environment.GetEnvironmentVariable("TLS_CERT") ?? "/certs/server.crt";
var keyPath = Environment.GetEnvironmentVariable("TLS_KEY") ?? "/certs/server.key";
var hasCert = File.Exists(certPath) && File.Exists(keyPath);

builder.WebHost.ConfigureKestrel(options =>
{
    // HTTP/1.1 on port 8080
    options.ListenAnyIP(8080, lo =>
    {
        lo.Protocols = HttpProtocols.Http1;
    });

    // HTTPS + HTTP/2 + HTTP/3 on port 8443
    if (hasCert)
    {
        options.ListenAnyIP(8443, lo =>
        {
            lo.Protocols = HttpProtocols.Http1AndHttp2AndHttp3;
            lo.UseHttps(X509Certificate2.CreateFromPemFile(certPath, keyPath));
        });
    }
});

var app = builder.Build();

app.Use(async (ctx, next) =>
{
    ctx.Response.Headers["Server"] = "aspnet-minimal";
    await next();
});

// Load dataset at startup
var datasetPath = Environment.GetEnvironmentVariable("DATASET_PATH") ?? "/data/dataset.json";
List<DatasetItem>? datasetItems = null;
if (File.Exists(datasetPath))
{
    var json = File.ReadAllText(datasetPath);
    datasetItems = JsonSerializer.Deserialize<List<DatasetItem>>(json);
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

app.MapGet("/static/{filename}", (string filename) =>
{
    if (staticFileMap.TryGetValue(filename, out var sf))
        return Results.Bytes(sf.Data, sf.ContentType);
    return Results.NotFound();
});

app.MapGet("/pipeline", () => Results.Text("ok"));

app.MapGet("/baseline11", (HttpRequest req) =>
{
    int sum = SumQuery(req);
    return Results.Text(sum.ToString());
});

app.MapPost("/baseline11", async (HttpRequest req) =>
{
    int sum = SumQuery(req);

    using var reader = new StreamReader(req.Body);

    var body = await reader.ReadToEndAsync();

    if (int.TryParse(body, out int b))
        sum += b;

    return Results.Text(sum.ToString());
});

app.MapGet("/baseline2", (HttpRequest req) =>
{
    int sum = SumQuery(req);
    return Results.Text(sum.ToString());
});

app.MapGet("/json", () =>
{
    if (datasetItems == null)
        return Results.Problem("Dataset not loaded");

    var responseItems = new List<ProcessedItem>(datasetItems.Count);
    foreach (var item in datasetItems)
    {
        responseItems.Add(new ProcessedItem
        {
            Id = item.Id,
            Name = item.Name,
            Category = item.Category,
            Price = item.Price,
            Quantity = item.Quantity,
            Active = item.Active,
            Tags = item.Tags,
            Rating = item.Rating,
            Total = Math.Round(item.Price * item.Quantity, 2)
        });
    }

    return Results.Json(new { items = responseItems, count = responseItems.Count });
});

app.Run();

static int SumQuery(HttpRequest req)
{
    int sum = 0;
    foreach (var (_, values) in req.Query)
        foreach (var v in values)
            if (int.TryParse(v, out int n)) sum += n;
    return sum;
}

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
