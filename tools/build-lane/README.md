# Build lane

The machine-wide build lane every agent `cargo` call goes through, and the worktree and PR scripts built around it. Each `.sh` script has a PowerShell 7 `.ps1` twin with the same options and output; both share the lane's slots.

| Entry point | What it does |
|---|---|
| `lane.sh` / `lane.ps1` | Run a compiling `cargo` command in a lane slot (`--exclusive` takes every slot), with a per-worktree target dir and sccache; prints a summary when stdout is not a terminal |
| `lane_stats.py` | Report on the lane's job log (`--recent N`, `--html`, `--csv`) |
| `mk-worktree.sh` / `.ps1` | Create a buildable worktree under `.claude/worktrees/` |
| `rm-worktree.sh` / `.ps1` | Retire a merged worktree: target dir, test database, branch (`--merged` for all) |
| `reload-db.sh` / `.ps1`, `live-db-test.sh` / `.ps1` | Reload the worktree's `sgw_<worktree>` database; run the live-DB tier against it in a slot |
| `ship.sh` / `.ps1` (`ship.py`) | `pr -C <worktree> -m <msg>` commits, pushes and opens the PR; `merge <PR> --retire <name>` merges and retires |
| `rebase-pr.sh` / `.ps1` (`rebase_pr.py`) | Rebase a PR branch onto `origin/main`, resolving only mechanical conflicts |

`lane_summary.py` writes the agent summary, `lane-slots.ps1` holds the slot protocol, and `test_*.py` / `test-*.ps1` are their tests. The [tools catalog](../README.md#tools-catalog) has every script's summary.

How to use them day to day: [development-workflow.md](../../docs/agents/development-workflow.md#builds-worktrees-and-test-databases). Why the lane exists, with measurements: [build-system.md](../../docs/architecture/build-system.md).
