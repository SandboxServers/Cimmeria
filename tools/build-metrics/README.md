# Build metrics

| Entry point | What it does |
|---|---|
| `measure-build.ps1` | Controlled build measurements from a worktree with an empty target dir: cold build, edit-rebuild loop, `cargo check`, peak memory and target size |

Its `.SYNOPSIS` and `.DESCRIPTION` list the phases and parameters (`Get-Help tools/build-metrics/measure-build.ps1 -Full`). The measurements it produced, and the decisions they drove: [build-system.md](../../docs/architecture/build-system.md).
