using System.Text.Json;
using Microsoft.Data.Sqlite;

static class AppData
{
    public static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase
    };

    public static List<DatasetItem>? DatasetItems;
    public static byte[]? LargeJsonResponse;
    public static Dictionary<string, (byte[] Data, string ContentType)> StaticFiles = new();
    public static SqliteConnection? DbConnection;

    public static void Load()
    {
        LoadDataset();
        LoadLargeDataset();
        LoadStaticFiles();
        OpenDatabase();
    }

    static void LoadDataset()
    {
        var path = Environment.GetEnvironmentVariable("DATASET_PATH") ?? "/data/dataset.json";
        if (!File.Exists(path)) return;
        DatasetItems = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(path), JsonOptions);
    }

    static void LoadLargeDataset()
    {
        var path = "/data/dataset-large.json";
        if (!File.Exists(path)) return;
        var items = JsonSerializer.Deserialize<List<DatasetItem>>(File.ReadAllText(path), JsonOptions);
        if (items == null) return;

        var processed = new List<ProcessedItem>(items.Count);
        foreach (var item in items)
        {
            processed.Add(new ProcessedItem
            {
                Id = item.Id, Name = item.Name, Category = item.Category,
                Price = item.Price, Quantity = item.Quantity, Active = item.Active,
                Tags = item.Tags, Rating = item.Rating,
                Total = Math.Round(item.Price * item.Quantity, 2)
            });
        }
        LargeJsonResponse = JsonSerializer.SerializeToUtf8Bytes(
            new { items = processed, count = processed.Count }, JsonOptions);
    }

    static void LoadStaticFiles()
    {
        var dir = "/data/static";
        if (!Directory.Exists(dir)) return;

        var mimeTypes = new Dictionary<string, string>
        {
            {".css", "text/css"}, {".js", "application/javascript"}, {".html", "text/html"},
            {".woff2", "font/woff2"}, {".svg", "image/svg+xml"}, {".webp", "image/webp"}, {".json", "application/json"}
        };
        foreach (var file in Directory.GetFiles(dir))
        {
            var name = Path.GetFileName(file);
            var ext = Path.GetExtension(file);
            var ct = mimeTypes.GetValueOrDefault(ext, "application/octet-stream");
            StaticFiles[name] = (File.ReadAllBytes(file), ct);
        }
    }

    static void OpenDatabase()
    {
        var path = "/data/benchmark.db";
        if (!File.Exists(path)) return;
        DbConnection = new SqliteConnection($"Data Source={path};Mode=ReadOnly");
        DbConnection.Open();
        using var pragma = DbConnection.CreateCommand();
        pragma.CommandText = "PRAGMA mmap_size=268435456";
        pragma.ExecuteNonQuery();
    }
}
