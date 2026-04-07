# HttpArena

[![Discord](https://discordapp.com/api/guilds/1177529388229734410/widget.png?style=shield)](https://discord.com/invite/H84B5ZqDXR)

HTTP framework benchmark platform.

20 test profiles. 64-core dedicated hardware. Same conditions for every framework.

[View Leaderboard](https://www.http-arena.com/) | [Documentation](https://www.http-arena.com/docs/) | [Add a Framework](https://www.http-arena.com/docs/add-framework/)

---

## PR Commands

Tag **@BennyFranciscus** on your PR for help with implementation or benchmark questions.

| Command | Description |
|---------|-------------|
| `/validate -f <framework>` | Run the 18-point validation suite |
| `/benchmark -f <framework>` | Run all benchmark tests |
| `/benchmark -f <framework> -t <test>` | Run a specific test |
| `/benchmark -f <framework> --save` | Run and save results (updates leaderboard on merge) |
| `/benchmark -f <framework> -t <test> --save` | Run specific test and save results |

Always specify `-f <framework>`. Results are automatically compared against the current leaderboard.

---

## Test Profiles

| Category | Profiles | Description |
|----------|----------|-------------|
| Connection | Baseline (512-16K), Pipelined, Limited | Performance scaling with connection count |
| Workload | JSON, Compression, Upload, Database, Async DB | Serialization, gzip, streaming I/O, SQLite queries, async Postgres |
| Resilience | Noisy, Mixed, API-4 | Malformed requests, concurrent endpoints, resource-constrained (4 CPU, 16 GB) |
| Protocol | HTTP/2, HTTP/3, gRPC, WebSocket | Multi-protocol support |

## Run Locally

```bash
git clone https://github.com/MDA2AV/HttpArena.git
cd HttpArena

./scripts/validate.sh <framework>            # correctness check
./scripts/benchmark.sh <framework>           # all profiles
./scripts/benchmark.sh <framework> baseline  # specific profile
./scripts/benchmark.sh <framework> --save    # save results
```

## Contributing

- [Add a new framework](https://www.http-arena.com/docs/add-framework/)
- Improve an existing implementation — open a PR modifying files under `frameworks/<name>/`
- [Open an issue](https://github.com/MDA2AV/HttpArena/issues)
- Comment on any open issue or PR

### Framework Maintainers

Add your GitHub username to the `maintainers` array in your framework's `meta.json` to get notified when someone opens a PR that touches your framework:

```json
"maintainers": ["your-github-username"]
```
