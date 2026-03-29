---
title: CI & Runner
---

## PR Commands

Comment on any PR to trigger validation or benchmarks. The framework is auto-detected from changed files, or you can specify it explicitly.

| Command | Description |
|---------|-------------|
| `/validate` | Run the 18-point validation suite |
| `/validate -f <framework>` | Validate a specific framework |
| `/benchmark` | Run all benchmark tests |
| `/benchmark -t <test>` | Run a specific test profile |
| `/benchmark -f <framework> -t <test>` | Run a specific framework and test |
| `/benchmark --save` | Run and save results (updates leaderboard on merge) |

### Flags

- **`-f <framework>`** — Override auto-detection. Use the directory name under `frameworks/` (e.g. `-f actix`, `-f go-fasthttp`).
- **`-t <test>`** — Run a specific test profile (e.g. `-t baseline`, `-t mixed`, `-t async-db`).
- **`--save`** — Save benchmark results to the PR branch. When the PR is merged, results are included in the next site deployment and appear on the leaderboard.

### Comparison with main

After every benchmark run, results are automatically compared against the current published data on main. A delta table is posted in the PR comment showing changes in RPS, latency, CPU, and memory for each connection count. New frameworks with no prior results show "NEW" instead of deltas.

## GitHub Actions

HttpArena uses four GitHub Actions workflows.

### PR Commands (`pr-commands.yml`)

**Trigger:** Comment on a PR containing `/validate` or `/benchmark`.

Parses the command and flags from the comment, detects the framework from changed PR files (or uses the `-f` flag), and either runs validation directly or dispatches the benchmark workflow. Adds a rocket reaction to the comment and posts results when done.

### Benchmark PR (`benchmark-pr.yml`)

**Trigger:** Dispatched by the PR Commands workflow, or manually via workflow dispatch.

Checks out the PR branch, runs the benchmark with optional `--save`, compares results against main using `scripts/compare.sh`, and posts a comment with raw results and a comparison table. If `--save` is used, results are committed and pushed to the PR branch.

### Benchmark (`benchmark.yml`)

**Trigger:** Automatically when a push to `main` modifies files under `frameworks/`, or manually via workflow dispatch.

Detects which frameworks changed and benchmarks them with `--save`. Results are committed to `main` and the site data is rebuilt.

### Deploy Site (`deploy.yml`)

**Trigger:** Automatically when a push to `main` modifies files under `site/`, or manually via workflow dispatch.

Builds the Hugo site and deploys it to GitHub Pages. Runs on GitHub-hosted runners (not the self-hosted benchmark machine).

## Hosted Runner

The Validate, Benchmark, and Benchmark PR workflows run on a **self-hosted runner** — a dedicated 64-core bare-metal machine configured for reproducible benchmarking:

- CPU governor locked to `performance` mode during benchmarks
- Kernel caches dropped between runs
- Docker daemon restarted for clean state
- `somaxconn` and TCP backlog tuned for high connection counts
- `ulimit` set to maximum file descriptors
- Host networking for minimal overhead

Only the Deploy Site workflow uses GitHub-hosted runners.
