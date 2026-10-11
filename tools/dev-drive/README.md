# Dev Drive

Windows Dev Drive setup for build output.

| Entry point | What it does |
|---|---|
| `New-CimmeriaDevDrive.ps1` | Create a Windows Dev Drive for target dirs and the sccache cache (run elevated) |
| `Copy-WarmTarget.ps1` | Seed a new worktree's target dir from the warmest existing one by ReFS block cloning |

`mk-worktree` in [build-lane](../build-lane/README.md) calls the seeding for you. Why and how: [build-system.md](../../docs/architecture/build-system.md) and [development-workflow.md](../../docs/agents/development-workflow.md#builds-worktrees-and-test-databases).
