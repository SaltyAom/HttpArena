# HttpArena

HTTP framework benchmark platform.

18 test profiles. 64-core dedicated hardware. Same conditions for every framework.

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
| Resilience | Noisy, Mixed | Malformed requests, concurrent endpoints |
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

## AI Agents

HttpArena uses autonomous AI agents to help with PR reviews, community engagement, and benchmark auditing.

### BennyFranciscus

The primary maintainer agent. Benny runs three cron jobs:

- **PR Review** — Monitors open PRs and issues on MDA2AV/HttpArena. Reviews diffs, responds to comments, addresses requested changes, and triggers benchmark runs when asked. Tracks commitments in a memory file to ensure follow-through.
- **Mentions** — Watches GitHub notifications for @BennyFranciscus mentions. Replies with technical context and actual benchmark data. Can trigger benchmark workflows directly from PR comments.

### jerrythetruckdriver

The benchmark auditor. Cotton is an io_uring specialist who keeps HttpArena fair. Three active cron jobs:

- **Cheater Audit** — Audits framework implementations for cheating or non-compliance with test profile specs. Checks if anyone is gaming the benchmark by bypassing framework APIs, using undocumented flags, or swapping in exotic libraries.
- **PR Review** — Reviews open PRs that add or modify frameworks. Checks diffs for rule violations and implementation issues.
- **Framework Audit** — Deep audit of existing framework entries for implementation rule violations.


## Contributing

- [Add a new framework](https://www.http-arena.com/docs/add-framework/)
- Improve an existing implementation — open a PR modifying files under `frameworks/<name>/`
- [Open an issue](https://github.com/MDA2AV/HttpArena/issues)
- Comment on any open issue or PR
