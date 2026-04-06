using System.Buffers;
using System.Text.Json;
using System.Text.Json.Serialization;
using Microsoft.AspNetCore.Http.HttpResults;
using Microsoft.Data.Sqlite;
using Npgsql;


[JsonSerializable(typeof(ResponseDto))]
[JsonSerializable(typeof(DbResponseDto))]
[JsonSerializable(typeof(DbItemDto))]
[JsonSerializable(typeof(List<DatasetItem>))]
[JsonSerializable(typeof(ProcessedItem))]
[JsonSerializable(typeof(RatingInfo))]
[JsonSerializable(typeof(List<string>))]
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase)]
partial class AppJsonContext : JsonSerializerContext { }

static class Handlers
{
    public static int Sum(int a, int b) => a + b;

    public static async ValueTask<int> SumBody(int a, int b, HttpRequest req)
    {
        using var reader = new StreamReader(req.Body);
        return a + b + int.Parse(await reader.ReadToEndAsync());
    }

    public static string Text() => "ok";

    public static async Task<IResult> Upload(HttpRequest req)
    {
        long size = 0;
        var buffer = ArrayPool<byte>.Shared.Rent(65536);
        try
        {
            int read;
            while ((read = await req.Body.ReadAsync(buffer.AsMemory(0, buffer.Length))) > 0)
            {
                size += read;
            }
        }
        finally
        {
            ArrayPool<byte>.Shared.Return(buffer);
        }

        return Results.Text(size.ToString());
    }

    public static Results<JsonHttpResult<ResponseDto>, ProblemHttpResult> Json()
    {
        if (AppData.JsonResponse == null)
            return TypedResults.Problem("Dataset not loaded");

        return TypedResults.Json(AppData.JsonResponse, AppJsonContext.Default.ResponseDto);
    }

    public static IResult Compression()
    {
        if (AppData.LargeJsonResponse == null)
        {
            return Results.StatusCode(500);
        }

        return Results.Bytes(AppData.LargeJsonResponse, "application/json");
    }

    public static Results<JsonHttpResult<DbResponseDto>, ProblemHttpResult> Database(HttpRequest req)
    {
        if (AppData.DbConnection == null)
            return TypedResults.Problem("DB not available");

        ReadPriceRange(req, out var min, out var max);

        using var cmd = AppData.DbConnection.CreateCommand();
        cmd.CommandText = "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN @min AND @max LIMIT 50";
        cmd.Parameters.AddWithValue("@min", min);
        cmd.Parameters.AddWithValue("@max", max);
        using var reader = cmd.ExecuteReader();

        var items = new List<DbItemDto>(50);
        while (reader.Read())
        {
            items.Add(ReadSqliteItem(reader));
        }

        return TypedResults.Json(new DbResponseDto(items, items.Count), AppJsonContext.Default.DbResponseDto);
    }

    public static async Task<JsonHttpResult<DbResponseDto>> AsyncDatabase(HttpRequest req)
    {
        if (AppData.PgDataSource == null)
            return TypedResults.Json(new DbResponseDto([], 0), AppJsonContext.Default.DbResponseDto);

        ReadPriceRange(req, out var min, out var max);

        await using var cmd = AppData.PgDataSource.CreateCommand(
            "SELECT id, name, category, price, quantity, active, tags, rating_score, rating_count FROM items WHERE price BETWEEN $1 AND $2 LIMIT 50");
        cmd.Parameters.AddWithValue(min);
        cmd.Parameters.AddWithValue(max);
        await using var reader = await cmd.ExecuteReaderAsync();

        var items = new List<DbItemDto>(50);
        while (await reader.ReadAsync())
        {
            items.Add(ReadNpgsqlItem(reader));
        }

        return TypedResults.Json(new DbResponseDto(items, items.Count), AppJsonContext.Default.DbResponseDto);
    }

    static void ReadPriceRange(HttpRequest req, out double min, out double max)
    {
        min = 10;
        max = 50;

        if (req.Query.TryGetValue("min", out var minValue) && double.TryParse(minValue, out var parsedMin))
        {
            min = parsedMin;
        }

        if (req.Query.TryGetValue("max", out var maxValue) && double.TryParse(maxValue, out var parsedMax))
        {
            max = parsedMax;
        }
    }

    static DbItemDto ReadSqliteItem(SqliteDataReader reader)
    {
        return new DbItemDto
        {
            Id = reader.GetInt32(0),
            Name = reader.GetString(1),
            Category = reader.GetString(2),
            Price = reader.GetDouble(3),
            Quantity = reader.GetInt32(4),
            Active = reader.GetInt32(5) == 1,
            Tags = JsonSerializer.Deserialize(reader.GetString(6), AppJsonContext.Default.ListString) ?? [],
            Rating = new RatingInfo
            {
                Score = reader.GetDouble(7),
                Count = reader.GetInt32(8)
            }
        };
    }

    static DbItemDto ReadNpgsqlItem(NpgsqlDataReader reader)
    {
        return new DbItemDto
        {
            Id = reader.GetInt32(0),
            Name = reader.GetString(1),
            Category = reader.GetString(2),
            Price = reader.GetDouble(3),
            Quantity = reader.GetInt32(4),
            Active = reader.GetBoolean(5),
            Tags = JsonSerializer.Deserialize(reader.GetString(6), AppJsonContext.Default.ListString) ?? [],
            Rating = new RatingInfo
            {
                Score = reader.GetDouble(7),
                Count = reader.GetInt32(8)
            }
        };
    }

}
