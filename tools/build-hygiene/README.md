# Build hygiene

| Entry point | What it does |
|---|---|
| `sweep.ps1` | Remove stale build artifacts from every Cimmeria target dir on the machine with `cargo-sweep` (`-DryRun` first). Don't run it while anything builds |

The build lane prints this command when free space drops below `LANE_MIN_FREE_GB`; run `rm-worktree --merged` from [build-lane](../build-lane/README.md) first. Background: [build-system.md](../../docs/architecture/build-system.md).
