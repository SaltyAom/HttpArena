## Description



---

**PR Commands** — comment on this PR to trigger (requires collaborator approval):

| Command | Description |
|---------|-------------|
| `/benchmark -f <framework>` | Run all benchmark tests |
| `/benchmark -f <framework> -t <test>` | Run a specific test |
| `/benchmark -f <framework> --save` | Run and save results (updates leaderboard on merge) |

Always specify `-f <framework>`. Results are automatically compared against the current leaderboard.

---

<details>
<summary><strong>Run benchmarks locally</strong></summary>

You can validate and benchmark your framework on your own machine using the lite scripts — no CPU pinning, fixed connections, all load generators run in Docker.

**Linux:**
```bash
./scripts/validate.sh <framework>
./scripts/benchmark-lite.sh <framework> baseline
./scripts/benchmark-lite.sh --load-threads 4 <framework>
```

**Windows / macOS (Docker Desktop):**
```bash
./scripts/validate-windows.sh <framework>
./scripts/benchmark-lite-windows.sh <framework> baseline
./scripts/benchmark-lite-windows.sh --load-threads 4 <framework>
```

The `-docker` variants use a Docker bridge network instead of `--network host` (which doesn't work on Docker Desktop).

**Requirements:** Docker Engine. Load generators (gcannon, h2load) are built as Docker images on first run. gcannon source is expected at `../gcannon` (override with `GCANNON_SRC`).

</details>
