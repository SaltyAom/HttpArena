using Microsoft.AspNetCore.Mvc;

using System.Text.Json;

[ApiController]
public class TestController : ControllerBase
{

    [HttpGet("/pipeline")]
    public string Pipeline() => "ok";

    [HttpGet("/baseline11")]
    public int Sum([FromQuery] int a, [FromQuery] int b) => a + b;

    [HttpPost("/baseline11")]
    public async Task<int> SumBody([FromQuery] int a, [FromQuery] int b)
    {
        using var reader = new StreamReader(Request.Body);
        return a + b + int.Parse(await reader.ReadToEndAsync());
    }

    [HttpGet("/baseline2")]
    public int Baseline2([FromQuery] int a, [FromQuery] int b) => a + b;

    [HttpPost("/upload")]
    public async Task<IActionResult> Upload()
    {
        long size = 0;
        var buffer = new byte[65536];
        int read;
        while ((read = await Request.Body.ReadAsync(buffer)) > 0)
            size += read;

        return Content(size.ToString());
    }

    [HttpGet("/json")]
    public IActionResult Json()
    {
        if (AppData.DatasetItems == null)
            return Problem("Dataset not loaded");

        var items = new List<ProcessedItem>(AppData.DatasetItems.Count);
        foreach (var item in AppData.DatasetItems)
        {
            items.Add(new ProcessedItem
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
        return Ok(new { items, count = items.Count });
    }

    [HttpGet("/compression")]
    public IActionResult Compression()
    {
        if (AppData.LargeJsonResponse == null)
            return StatusCode(500);

        return File(AppData.LargeJsonResponse, "application/json");
    }

    [HttpGet("/db")]
    public IActionResult Database([FromQuery] double min = 10, [FromQuery] double max = 50)
    {
        if (AppData.DbConnection == null)
            return Problem("DB not available");

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
        return Ok(new { items, count = items.Count });
    }

    [HttpGet("/async-db")]
    public async Task<IActionResult> AsyncDatabase([FromQuery] double min = 10, [FromQuery] double max = 50)
    {
        if (AppData.PgDataSource == null)
            return Ok(new { items = Array.Empty<object>(), count = 0 });

        await using var cmd = AppData.PgDataSource.CreateCommand(
            "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50");
        cmd.Parameters.AddWithValue(min);
        cmd.Parameters.AddWithValue(max);
        await using var reader = await cmd.ExecuteReaderAsync();

        var items = new List<object>();
        while (await reader.ReadAsync())
        {
            items.Add(new
            {
                id = reader.GetInt32(0),
                name = reader.GetString(1),
                category = reader.GetString(2),
                price = reader.GetDouble(3),
                quantity = reader.GetInt32(4),
                active = reader.GetBoolean(5),
                tags = JsonSerializer.Deserialize<List<string>>(reader.GetString(6)),
                rating = new { score = reader.GetDouble(7), count = reader.GetInt32(8) },
            });
        }
        return Ok(new { items, count = items.Count });
    }

}