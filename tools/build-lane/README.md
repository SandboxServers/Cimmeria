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
| `commit-paths.ps1` | Commit exactly the named files in one worktree for `ops` (`-Paths a,b` comma list, `-Trailers 'x;;y'`, optional `-ExpectBranch`; refuses absolute paths, `.`, `-A`, globs, directories, paths below a junction, any already-staged file (renames included) and an add that would stage an unnamed file (a deleted directory name); whitespace errors refuse only where the repo sets `core.whitespace` or a whitespace attribute, else they warn; unstages again on a failed check or hook, so the index ends empty) |
| `merge-noff.ps1` | Merge one local branch into the current branch of one worktree with `--no-ff` (optional `-ExpectBranch`); a conflict is aborted; `up-to-date` when nothing moves; refuses while a rebase, merge, cherry-pick or revert is in progress; `branch.<b>.mergeOptions` is overridden |
| `branch-at.ps1` | Create one local branch at a full 40-hex SHA for `ops`; never overwrites a branch, never checks out, refuses ambiguous names (`@`, `refs/..`, `<remote>/..`, `tags/..`, a leading `+`, hex-hash-like names, an existing tag name, any name that already resolves such as `stash`) |
| `push-branch.ps1` | Push the worktree's current branch to origin for `ops` (`-u`; `-Force` needs `-ExpectRemoteSha <40-hex>` and runs `--force-with-lease=refs/heads/<b>:<sha>` only); refuses main, master, other branches and names not starting with a letter, digit or `_`; pushes the full `refs/heads/<b>` refspec |
| `rebase-onto.ps1` | Rebase the current branch onto a commit (40-hex `-Onto`, optional 40-hex `-Upstream` for `--onto`; refused when it is HEAD or when the rebase would drop commits; ignores `rebase.updateRefs`) on a clean tree for `ops`, keeping merge commits; a conflict is aborted and exits 1; refuses while a rebase, merge, cherry-pick or revert is in progress |

`lane_summary.py` writes the agent summary, `lane-slots.ps1` holds the slot protocol, and `test_*.py` / `test-*.ps1` are their tests. The [tools catalog](../README.md#tools-catalog) has every script's summary.

The ops git verbs (`commit-paths`, `merge-noff`, `branch-at`, `push-branch`, `rebase-onto`) each have a `test-<name>.ps1` that builds a throwaway repository, worktree and local bare remote under the temp directory; none touches the network, a real remote or `cargo`, and CI does not run them, so run them after any change to those scripts:

```powershell
Get-ChildItem tools/build-lane/test-*.ps1 | ForEach-Object { pwsh -NoProfile -File $_.FullName }
```

Each prints one line per check and exits non-zero when one fails.

How to use them day to day: [development-workflow.md](../../docs/agents/development-workflow.md#builds-worktrees-and-test-databases). Why the lane exists, with measurements: [build-system.md](../../docs/architecture/build-system.md).
