---
title: "Build system: toolchain, profiles, concurrency and disk"
type: explanation
audience: engineers and agent operators
last_updated: 2026-09-26
---

# Build system: toolchain, profiles, concurrency and disk

> **Status:** Accepted (2026-09-26). Implemented on `build/toolchain-overhaul`, except §6 (cargo-hakari, planned as the last step, after the crate split).
> **Scope:** how Cimmeria's Rust workspace is compiled on developer and agent machines and in CI, and why. The companion [services-crate-split.md](services-crate-split.md) covers the crate layout.

## Context

By September 2026, several Claude sessions and their agents were building the workspace in parallel on one 64 GB Windows workstation. Measured on 2026-09-26:

- **Disk.** About 254 GB of build output: the main checkout's `target\` was 171 GB (312,000 files), twelve worktree target dirs held 62 GB, and sccache held 21 GB. `target\debug\deps` contained 157 distinct builds of `cimmeria-entity` alone. Cargo never deletes old artifacts, and every change of Rust version, feature set or flags leaves another full copy.
- **Two toolchains.** Local builds used whatever stable was installed (1.94), while CI floated on current stable (1.98.1), so clippy was run on both and every crate was compiled twice.
- **Feature churn.** `cargo build -p X` resolves dependency features differently from `--workspace`, so partial builds recompiled dependencies the full build had already built.
- **One huge crate.** `cimmeria-services` was 914 files in a single compilation unit. Editing one file in `cell::content` and rebuilding its test binary took **164.7 s**.
- **Guessed concurrency.** The build lane allowed two builds at once, a number chosen before the linker changed.

Baseline (untouched `main` `f153138b`, Rust 1.98.1, no sccache, `CARGO_BUILD_JOBS=10`, measured with `tools/build-metrics/measure-build.ps1`):

| Measure | Baseline |
|---|---|
| Cold build, gated workspace, all targets | 254.7 s |
| Edit `cell/content/mod.rs`, rebuild services test binary | 164.7 s |
| Edit, then `cargo check -p cimmeria-services` | 57.3 s |
| Peak compiler + linker working set (cold build) | 9.4 GB |
| Lowest free RAM during the cold build | 19.3 GB |
| Target dir after those three builds | 14.2 GB, 9,840 files |

The old "a full link needs ~47 GB" figure predates the switch to `rust-lld` on Windows (already in `.cargo/config.toml`). It no longer applies.

## Decisions

### 1. One pinned toolchain

`rust-toolchain.toml` pins the Rust version (1.98.1). Every CI workflow installs it through `.github/actions/rust-toolchain`, which reads the channel from that file. Local `cargo` picks it up automatically through rustup.

Bumping the version is a deliberate PR: change the file, run the pre-PR checklist, and fix new lints in the same PR. This replaces the old "CI floats on stable, so run `cargo +<newer> clippy` before pushing" gotcha.

### 2. Debug info: line tables for our crates, none for dependencies

- `[profile.dev] debug = "line-tables-only"`: panics and backtraces keep file:line.
- `[profile.dev.package."*"] debug = false` (unchanged), plus `opt-level = 1` for dependencies.
- For a debugger session, use `cargo build --profile dev-debug`. That profile has full debug info and builds into `target/dev-debug/`, so it never invalidates the normal dev build.

### 3. The build lane is part of the repo

`tools/build-lane/lane.sh` is the machine-wide counting semaphore that every agent and worker cargo call goes through. It was previously an untracked script under `%TEMP%`.

- **Slots:** `LANE_SLOTS`, else the number in `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` (4 on the 64 GB development machine, where the job log showed at least 27 GB free with four builds running), else 2. `--exclusive` takes every slot.
- **Jobs:** `CARGO_BUILD_JOBS` defaults to the core count divided by the slot count (at least 4), so a full lane doesn't oversubscribe the CPU.
- **Job log:** every job appends one JSON line to `%LOCALAPPDATA%\cimmeria-build\metrics\jobs.jsonl`. The line records the start time, the wait for a slot, the run time, the exit code, the worktree and commit, the settings (jobs, incremental, Dev Drive or local target, sccache), how many other builds were running, the lowest free RAM during the job, and the sccache hits and misses. `LANE_METRICS=0` turns it off. See §10 for the report.
- **Compiler cache:** sccache is the `RUSTC_WRAPPER` when installed, with one shared cache.
- **Incremental builds everywhere; sccache for third-party crates only:** workspace crates build incrementally (the dev profile's default) in the main checkout and in every worktree, and sccache passes them through uncached. Worktrees first built with `CARGO_INCREMENTAL=0`, so that sccache could hand a worker the workspace crates it didn't touch. It never did. sccache keys a crate by its absolute path, so one worktree's crates can't hit in another: over the first 100 lane jobs the hit rate was 0%. Meanwhile the edit loop lost its incremental reuse. Re-enabling incremental cut an edit-then-`cargo check` of `cimmeria-cell-content` and `cimmeria-services` from 16.6–17.7 s to 5.5 s. sccache refuses to run when `CARGO_INCREMENTAL` is set to anything but `0`, so the lane drops sccache when a caller sets it.
- **Per-worktree target dirs:** each worktree keeps its own target dir. Cargo locks a target (or `build-dir`) for the whole build, so a shared one would serialise every worktree. Fine-grained locking is nightly-only and was reported deadlocking in September 2026 ([cargo#17508](https://github.com/rust-lang/cargo/issues/17508)).

`reload-db.sh` and `live-db-test.sh` live alongside it and give each worktree its own test database. `mk-worktree.sh` creates a buildable worktree.

### 4. Dev Drive for build output (optional, recommended)

`tools/dev-drive/New-CimmeriaDevDrive.ps1` creates a dynamically sized VHDX formatted as a Windows Dev Drive. It must run from an elevated PowerShell. It also sets `CIMMERIA_TARGET_ROOT` and `CIMMERIA_SCCACHE_DIR` for the user, and the lane then places target dirs and the sccache cache on that drive.

- **Defender:** Defender scans a trusted Dev Drive asynchronously (performance mode). To confirm it, open Windows Security → **Virus & threat protection → Manage settings → Dev Drive protection**, which lists each volume. The Dev Drive should say "Asynchronous scanning is on". Don't rely on `(Get-MpPreference).PerformanceModeStatus`: it is not a per-volume reading, and on this machine it read `1` ("Disabled") while that page reported the drive as scanned asynchronously.
- **Block cloning:** ReFS copies files within the volume by cloning blocks. `Copy-WarmTarget.ps1`, called by `mk-worktree.sh`, seeds a new worktree's target dir from a warm one in seconds, at almost no disk cost.
- **What seeding reuses:** only third-party crates. Cargo keys workspace crates by their source path, so those rebuild once per worktree.

Source code stays where it is.

### 5. Cleanup is scheduled, not ad hoc

`tools/build-hygiene/sweep.ps1` runs `cargo-sweep` over every target dir on the machine (the main checkout, `.claude/worktrees/*` and the Dev Drive). It does four things:

- keeps only artifacts built by the pinned toolchain;
- drops artifacts unused for `-Days` (default 14);
- prunes stale incremental caches, which cargo-sweep does not touch;
- optionally removes Dev Drive target dirs whose worktree is gone (`-RemoveOrphans`), and the old in-worktree target dirs once builds have moved to the Dev Drive (`-RemoveLegacyTargets`).

Don't run it while something is building.

On 2026-09-26 this freed 84.7 GB of stale artifacts and 70.6 GB of incremental caches from the main checkout alone.

### 6. Feature unification with cargo-hakari

`crates/workspace-hack` (`cimmeria-workspace-hack`) is generated by `cargo hakari` from [.config/hakari.toml](../../.config/hakari.toml). Every gated workspace crate depends on it, so every crate requests the same feature set for shared third-party dependencies. As a result, `-p` builds reuse what `--workspace` built, and the reverse.

- **Why it matters here:** after the split, agents mostly build single crates. Before hakari, a `cargo check -p <crate>` after a warm workspace check recompiled up to 36 third-party crates only because the feature sets differed: wire 36, base-session 32, server 32, cell-world 12. With the workspace-hack every one of those is 0. For example, `-p cimmeria-wire` went from 11.9 s to 2.2 s and `-p cimmeria-base-session` from 12.2 s to 3.4 s.
- **Scope:** the platforms are `x86_64-pc-windows-msvc` (development) and `x86_64-unknown-linux-gnu` (CI). The six crates CI leaves out of the gated workspace (the GUI apps, the client-telemetry cdylib and the lab supervisor) are traversal- and final-excluded. Their GUI-only features would otherwise reach every crate and break the Linux runners.
- **Maintenance:** after adding or changing a dependency, run `cargo hakari generate && cargo hakari manage-deps --yes` and commit the result. The CI build job runs `cargo hakari generate --diff`, `cargo hakari manage-deps --dry-run` and `cargo hakari verify`, and fails when either file is stale.
- **Cost:** every crate now waits for the workspace-hack's dependencies (tokio, hyper, sqlx and the rest) on a cold build, even a small crate that needs none of them. The results table below measures the cold build with it in place.
- The crate graph hides the workspace-hack, because it would add an edge from every member.

Cargo's built-in equivalent (`resolver.feature-unification`, [cargo#14774](https://github.com/rust-lang/cargo/issues/14774)) is still nightly-only.

### 7. One integration-test binary per crate

Every integration test file under `tests/` is its own binary that relinks the whole dependency tree. Crates with several files now use `tests/it/main.rs` with one module per former file (content-engine, wireclient, navmesh-extractor: 23 binaries became 3). New integration tests go in `tests/it/`; mercury's `LossyTransport` round trips, which named only mercury, moved there from `crates/services/tests/` in the last wave of the services split.

### 8. Fewer and deduplicated dependencies

The September pass removed about 30 unused dependencies and collapsed duplicate versions: 341 → 332 unique crates in the gated workspace, and the services dependency tree went from 277 to 258 crates. Duplicates that remain are pinned by third-party crates; the reasons are recorded in `.claude/agent-memory/rust-gameserver-dev/dependency-dedupe-blockers.md`.

To find who pulls an old version, run `cargo tree -i <crate>@<version>`. Run `cargo machete` before adding a dependency cleanup PR.

### 9. The services crate is split

`cimmeria-services` is a facade over 18 service crates (2026-09-26). See [services-crate-split.md](services-crate-split.md) for the plan, the record of each wave and the final layout.

### 10. Measure before changing concurrency

`tools/build-metrics/measure-build.ps1` measures the cold build, the edit loop, `cargo check`, peak memory and target size. Run it from a worktree with an empty target dir, under `lane.sh --exclusive`. Re-measure before changing the lane's slot count or `CARGO_BUILD_JOBS`.

The harness gives controlled numbers for one moment. For day-to-day trends, the lane's job log (§3) records every real build, and `tools/build-lane/lane_stats.py` reports on it:

```bash
python tools/build-lane/lane_stats.py                    # last 14 days: by kind, by day, Dev Drive vs local, slowest jobs
python tools/build-lane/lane_stats.py --kind check --match cimmeria-services --days 60
python tools/build-lane/lane_stats.py --recent 20        # the last 20 jobs
python tools/build-lane/lane_stats.py --html lane.html   # run time over time, one chart per kind
python tools/build-lane/lane_stats.py --csv jobs.csv     # every field, for a spreadsheet
```

The log mixes very different jobs (a one-crate `cargo check` and a workspace `nextest` are both "a build"), so compare like with like with `--kind` and `--match`. A job's sccache hits and misses come from the shared sccache server, so they include any build that overlapped it; `busy_at_start` records how many did. The lowest free RAM is sampled every 2 seconds, so it is the figure to watch before raising the slot count.

### 11. No WSL builds

Development builds run natively on Windows (PowerShell or Git Bash, driven by Claude Code). The WSL cross-compile path and its memory rules are retired.

- CI still builds on Linux runners, so the `x86_64-unknown-linux-gnu` linker settings in `.cargo/config.toml` stay.
- The bootstrap module's WSL branches stay for contributors who run setup there.

## Consequences

- One toolchain, one set of artifacts, and CI clippy equals local clippy.
- Target dirs stop growing without bound, as long as the sweep runs.
- Agent worktrees share compiled crates through sccache and need a Dev Drive only for the extra Defender and cloning gains.
- Bumping Rust is now a visible, reviewable change instead of something CI does silently.
- Once §6 lands, a `workspace-hack` dependency appears in every crate's manifest. `cargo hakari manage-deps` maintains it; don't edit it by hand.

## Results

Filled in by the final measurement of this overhaul (same harness, same machine): see the table at the end of this section once it lands.
