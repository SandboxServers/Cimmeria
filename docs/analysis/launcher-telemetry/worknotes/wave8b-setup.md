# Wave 8b setup

Date: 2026-10-21

Base: tip of `launcher-telemetry/light-adoption` (worktree `lt-light-integration`), verified
`c4126f495a1a49279238bdcae849a365afa8072b` (matches expected).

| Worktree | Path | Branch |
|---|---|---|
| la-03 | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\la-03` | `launcher-telemetry/la-03-incomplete-ledger` |
| la-09 | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\la-09` | `launcher-telemetry/la-09-light-plan` |
| la-12 | `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\la-12` | `launcher-telemetry/la-12-index-compare` |

Each worktree has `external\` as a directory junction to
`C:\Users\Steve\source\projects\Cimmeria\external` (same as la-05). HEAD = base sha, working
tree clean at creation. Nothing staged; do not stage `launcher_targets.txt`, `dist` or `node_modules`.
