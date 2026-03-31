# aspnet-minimal

Minimal ASP.NET Core HTTP server using .NET 10 with Kestrel and MVC routing.

## Stack

- **Language:** C# / .NET 10 (Alpine)
- **Framework:** ASP.NET Core MVC
- **Engine:** Kestrel
- **Build:** Self-contained publish, `aspnet:10.0-alpine` runtime

## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns `ok` (plain text) |
| `/baseline11` | GET | Sums query parameter values |
| `/baseline11` | POST | Sums query parameters + request body |
| `/baseline2` | GET | Sums query parameter values (HTTP/2 variant) |
| `/json` | GET | Processes 50-item dataset, serializes JSON |
| `/compression` | GET | Gzip-compressed large JSON response |
| `/db` | GET | SQLite range query with JSON response |
| `/upload` | POST | Receives 1 MB body, returns byte count |
| `/static/{filename}` | GET | Serves preloaded static files with MIME types |

## Notes

- HTTP/1.1 on port 8080, HTTP/1+2+3 on port 8443
- Logging disabled (`ClearProviders()`) for throughput
- Response compression middleware (gzip, fastest level)
- HTTP/2 tuned: 256 max streams, 2 MB connection window
- Split into Program.cs, Handlers.cs, AppData.cs, Models.cs
