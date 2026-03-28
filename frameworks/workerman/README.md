# Workerman

An asynchronous event driven PHP socket framework. Supports HTTP, Websocket, SSL and other custom protocols.

All in pure PHP code.

https://github.com/walkor/workerman
https://manual.workerman.net/doc/en/
https://www.workerman.net/


## Stack

- **Language:** PHP
- **Engine:** Workerman (event)


## Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/pipeline` | GET | Returns `ok` (plain text) |
| `/baseline11` | GET | Sums query parameter values |
| `/baseline11` | POST | Sums query parameters + request body |
| `/json` | GET | Processes 50-item dataset, serializes JSON |
| `/compression` | GET | Gzip-compressed large JSON response |
| `/db` | GET | SQLite range query with JSON response |
| `/upload` | POST | Receives 1 MB body, returns byte count |
| `/static/{filename}` | GET | Serves preloaded static files |

## Notes

- No chunked requests for now (WIP)
- Chunked responses OK
- Per-worker database connection

