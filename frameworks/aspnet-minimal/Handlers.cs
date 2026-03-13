using System.Text.Json;

static class Handlers
{
    public static IResult GetBaseline(HttpRequest req)
    {
        return Results.Text(SumQuery(req).ToString());
    }

    public static async Task<IResult> PostBaseline(HttpRequest req)
    {
        int sum = SumQuery(req);
        using var reader = new StreamReader(req.Body);
        var body = await reader.ReadToEndAsync();
        if (int.TryParse(body, out int b))
            sum += b;
        return Results.Text(sum.ToString());
    }

    public static IResult GetBaseline2(HttpRequest req)
    {
        return Results.Text(SumQuery(req).ToString());
    }

    public static IResult Pipeline()
    {
        return Results.Text("ok");
    }

    public static async Task<IResult> Upload(HttpRequest req)
    {
        using var ms = new MemoryStream();
        await req.Body.CopyToAsync(ms);
        return Results.Text(ms.Length.ToString());
    }

    public static IResult Json()
    {
        if (AppData.DatasetItems == null)
            return Results.Problem("Dataset not loaded");

        var items = new List<ProcessedItem>(AppData.DatasetItems.Count);
        foreach (var item in AppData.DatasetItems)
        {
            items.Add(new ProcessedItem
            {
                Id = item.Id, Name = item.Name, Category = item.Category,
                Price = item.Price, Quantity = item.Quantity, Active = item.Active,
                Tags = item.Tags, Rating = item.Rating,
                Total = Math.Round(item.Price * item.Quantity, 2)
            });
        }
        return Results.Json(new { items, count = items.Count });
    }

    public static async Task Compression(HttpContext ctx)
    {
        if (AppData.LargeJsonResponse == null)
        {
            ctx.Response.StatusCode = 500;
            return;
        }
        ctx.Response.ContentType = "application/json";
        await ctx.Response.Body.WriteAsync(AppData.LargeJsonResponse);
    }

    public static IResult StaticFile(string filename)
    {
        if (AppData.StaticFiles.TryGetValue(filename, out var sf))
            return Results.Bytes(sf.Data, sf.ContentType);
        return Results.NotFound();
    }

    public static IResult Database(HttpRequest req)
    {
        if (AppData.DbConnection == null)
            return Results.Problem("DB not available");

        double min = 10, max = 50;
        if (req.Query.ContainsKey("min") && double.TryParse(req.Query["min"], out double pmin))
            min = pmin;
        if (req.Query.ContainsKey("max") && double.TryParse(req.Query["max"], out double pmax))
            max = pmax;

        using var cmd = AppData.DbConnection.CreateCommand();
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
        return Results.Json(new { items, count = items.Count });
    }

    static int SumQuery(HttpRequest req)
    {
        int sum = 0;
        foreach (var (_, values) in req.Query)
            foreach (var v in values)
                if (int.TryParse(v, out int n)) sum += n;
        return sum;
    }
}
