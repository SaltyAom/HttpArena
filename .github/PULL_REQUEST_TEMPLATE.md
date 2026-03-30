## Description



---

**PR Commands** — comment on this PR to trigger:

| Command | Description |
|---------|-------------|
| `/validate -f <framework>` | Run the 18-point validation suite |
| `/benchmark -f <framework>` | Run all benchmark tests |
| `/benchmark -f <framework> -t <test>` | Run a specific test |
| `/benchmark -f <framework> --save` | Run and save results (updates leaderboard on merge) |

Always specify `-f <framework>`. Results are automatically compared against the current leaderboard.
