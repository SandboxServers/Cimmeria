---
title: "Build system: toolchain, profiles, concurrency and disk"
type: explanation
audience: engineers and agent operators
last_updated: 2026-10-03
---

# Build system: toolchain, profiles, concurrency and disk

> **Status:** Accepted and implemented (2026-09-26, `build/toolchain-overhaul`). The sccache and disk fixes of §3–§5 followed on 2026-09-28 (#1023), and the lane's quiet output for agents (§3) on 2026-10-03 (#957). Measured results are in [Results](#results) and [sccache and disk results](#sccache-and-disk-results-2026-09-28).
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
- **Job log:** every job appends one JSON line to `%LOCALAPPDATA%\cimmeria-build\metrics\jobs.jsonl`. The line records the start time, the wait for a slot, the run time, the exit code, the worktree and commit, the settings (jobs, incremental, Dev Drive or local target, sccache), how many other builds were running, the lowest free RAM during the job, the sccache hits and misses, the free disk at the start (`disk_free_gb`), the incremental sessions pruned after it (`pruned_mb`), and whether the output was summarised (`quiet`, with the `log` path). `LANE_METRICS=0` turns it off. See §10 for the report.
- **Quiet output for agents.** When the lane's stdout is not a terminal, which is how an agent's Bash tool runs it, the command's whole output goes to a log, `%LOCALAPPDATA%\cimmeria-build\logs\<worktree>\<time>-<pid>.log`, and stdout gets only a summary that [`lane_summary.py`](../../tools/build-lane/lane_summary.py) extracts from it: `[lane] status=ok|failed exit=N ran=Ns`, the test counts, each compiler error (up to 14 lines of it, 8 errors), each failing test with its panic message, warnings as one line each, the failures file (every error and failing test in full) and the log path. A failure it doesn't recognise prints the log's last 25 lines instead, so a failure is never only an exit code. The log path is also printed to stderr before the command runs, so a caller whose tool times out still has it. nextest runs with `--status-level fail --show-progress=none --failure-output final`, through its `NEXTEST_*` variables, which a caller can still set. `LANE_VERBOSE=1` keeps the full output, and so does `CI=true`, so a CI step that goes through the lane keeps its log; CI's live-DB jobs call `tools/test-live-db.sh` directly anyway. A terminal sees no change. Each worktree keeps its newest `LANE_LOG_KEEP` logs (default 20), none older than `LANE_LOG_DAYS` days (default 7), and `rm-worktree.sh` deletes a retired worktree's logs. A passing `cargo test -p cimmeria-commands` went from 2,524 characters of output (8,946 on a cold build) to 267, and `cargo nextest run -p cimmeria-wire` from 36,765 to about 270; a failing test from 2,522 to about 710 ([#957](https://github.com/SandboxServers/Cimmeria/issues/957), TP-02).
- **Compiler cache:** sccache caches third-party crates in one shared cache, for every worktree. `RUSTC_WRAPPER` is not sccache itself but [`sccache-wrap.rs`](../../tools/build-lane/sccache-wrap.rs), which the lane compiles with plain `rustc` on first use into `%LOCALAPPDATA%\cimmeria-build\bin\sccache-wrap\<source hash>\sccache.exe`. The wrapper removes `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` and `CARGO_BUILD_BUILD_DIR` from sccache's environment, then runs it. It exists because sccache hashes every `CARGO_*` variable into a Rust cache key (all but `CARGO_MAKEFLAGS`, `CARGO_REGISTRIES_*`, `CARGO_BUILD_JOBS` and `CARGO_ENCODED_RUSTFLAGS`). With each worktree's own `CARGO_TARGET_DIR` in the key, the same `tokio` got a different key in every worktree: 124 hits against 66,934 misses on 2026-09-27. Cargo has already read the variable when it runs the wrapper, and rustc gets every path it needs as an argument (`--out-dir`, `-L`, `--extern`), which sccache leaves out of the key. See the [results](#sccache-and-disk-results-2026-09-28).
  - **What still misses across worktrees, on purpose.** A crate that reads `OUT_DIR` at compile time (`include!(concat!(env!("OUT_DIR"), ...))`) lists it in its dep-info, and sccache hashes its value, which names the worktree's target dir. So its build is never replayed into another worktree, where a baked-in path such as utoipa-swagger-ui's would point at the wrong target dir ([sccache#2870](https://github.com/mozilla/sccache/issues/2870)). `serde`, `serde_core` and `thiserror` are such crates, and every crate built on them misses as well, because their metadata differs per worktree. That is about 22% of the third-party crates.
  - **`SCCACHE_BASEDIRS`** is set to the Dev Drive target root and the main checkout. sccache 0.18 reads it once, when its server starts, and applies it only to C/C++ compiles (none of ours go through sccache today), so it does nothing for Rust. The wrapper is the Rust fix.
- **Incremental builds everywhere; sccache for third-party crates only:** workspace crates build incrementally (the dev profile's default) in the main checkout and in every worktree, and sccache passes them through uncached. Worktrees first built with `CARGO_INCREMENTAL=0`, so that sccache could hand a worker the workspace crates it didn't touch. It never did. sccache hashes a crate's `CARGO_MANIFEST_DIR`, the worktree's own path for workspace crates, so one worktree's crates can't hit in another: over the first 100 lane jobs the hit rate was 0%. Meanwhile the edit loop lost its incremental reuse. Re-enabling incremental cut an edit-then-`cargo check` of `cimmeria-cell-content` and `cimmeria-services` from 16.6–17.7 s to 5.5 s. sccache refuses to run when `CARGO_INCREMENTAL` is set to anything but `0`, so the lane drops sccache when a caller sets it.
- **Stale incremental sessions are pruned after each job.** rustc keeps the session it started from next to the one it just wrote, in every unit dir under `<target>/<profile>/incremental/`, and deletes it only when that unit next compiles. It only ever loads the newest finished session, so the older one is dead weight: 3.0 of 6.7 GB of one worktree's incremental dir. After the command, the lane deletes each unit's older sessions and any `-working` session more than an hour old (a compile that died), unless another lane job is building in the same worktree. This never makes cargo or rustc rebuild anything. `LANE_PRUNE=0` turns it off.
- **Low-disk guard.** Before running the command, the lane checks the free space on the target dir's drive. Below `LANE_MIN_FREE_GB` (default 10), it prunes the worktree's stale sessions, and if that isn't enough it exits with code 28 and a message that names `rm-worktree.sh --merged` and `sweep.ps1`. Without the guard, cargo runs until the disk is full and fails part-way with "os error 112", as every agent's build did twice on 2026-09-28. `LANE_MIN_FREE_GB=0` turns it off.
- **Per-worktree target dirs:** each worktree keeps its own target dir. Cargo locks a target (or `build-dir`) for the whole build, so a shared one would serialise every worktree. Fine-grained locking is nightly-only and was reported deadlocking in September 2026 ([cargo#17508](https://github.com/rust-lang/cargo/issues/17508)).

`reload-db.sh` and `live-db-test.sh` live alongside it and give each worktree its own test database. `mk-worktree.sh` creates a buildable worktree, and `rm-worktree.sh` retires one once its PR merges: target dir, `external/` junction, worktree, branch and test database. Per-worktree target dirs make that cleanup part of the design. Left behind, they filled the 150 GB Dev Drive on 2026-09-26. `rebase-pr.sh` rebases a PR branch onto `origin/main` with git alone, in a throwaway worktree that checks nothing out, so a failed rebase leaves the branch, its worktree and its build cache untouched; it resolves only generated doc blocks (main's side) and `Cargo.lock` (`cargo update --workspace`), and reports any other conflict for a worker ([development-workflow.md](../agents/development-workflow.md#worker-lifetime-and-notifications)). It doesn't compile, so it doesn't take a lane slot. `ship.sh` puts the end of a packet into one call with one line of output, because plumbing calls run one per request in large contexts were 11.2% of agent spend ([retro cost study](../analysis/token-usage/retro-cost-study.md)). `ship.sh pr` commits, pushes and opens the PR, and refuses to run anywhere but a registered worktree on a feature branch. `ship.sh merge` waits for the gating checks, calls `rebase-pr.sh` only when GitHub reports the PR behind or conflicting, merges, and retires the worktree through `rm-worktree.sh`. Neither `rm-worktree.sh` (without `--prune`) nor `rebase-pr.sh` runs `git worktree prune`: it deletes every entry whose path the pruning git can't resolve, and is the first suspect for the worktree entries that vanished on 2026-10-03.

### 4. Dev Drive for build output (optional, recommended)

`tools/dev-drive/New-CimmeriaDevDrive.ps1` creates a dynamically sized VHDX formatted as a Windows Dev Drive. It must run from an elevated PowerShell. It also sets `CIMMERIA_TARGET_ROOT` and `CIMMERIA_SCCACHE_DIR` for the user, and the lane then places target dirs and the sccache cache on that drive.

- **Defender:** Defender scans a trusted Dev Drive asynchronously (performance mode). To confirm it, open Windows Security → **Virus & threat protection → Manage settings → Dev Drive protection**, which lists each volume. The Dev Drive should say "Asynchronous scanning is on". Don't rely on `(Get-MpPreference).PerformanceModeStatus`: it is not a per-volume reading, and on this machine it read `1` ("Disabled") while that page reported the drive as scanned asynchronously.
- **Block cloning:** ReFS copies files within the volume by cloning blocks. `Copy-WarmTarget.ps1`, called by `mk-worktree.sh`, seeds a new worktree's target dir from a warm one in seconds, at almost no disk cost.
- **What seeding reuses:** only third-party crates. Cargo keys workspace crates by their source path, so those rebuild once per worktree.
- **What seeding drops:** build-script output that names the source target dir. Cargo rewrites a unit's own `OUT_DIR` in its `output` file when the target dir moves, but not other paths: a `cargo:` line naming another unit's out dir (aws-lc-rs passes on aws-lc-sys's), or generated Rust that bakes in a path (utoipa-swagger-ui). Those would point at the source worktree, and at nothing once it is retired; two workspace clippy runs failed that way on 2026-09-27 (#962). `Copy-WarmTarget.ps1` deletes such a unit's build dir and fingerprint, and cargo reruns its build script.

Source code stays where it is.

### 5. Cleanup is scheduled, not ad hoc

`tools/build-hygiene/sweep.ps1` trims every target dir on the machine (the main checkout, `.claude/worktrees/*` and the Dev Drive). It:

- keeps only artifacts built by the pinned toolchain, and drops artifacts unused for `-Days` (default 14), through `cargo-sweep`;
- prunes incremental caches, which cargo-sweep does not touch: the stale sessions of §3 in every unit dir, and whole unit dirs not compiled for `-IncrementalHours` (default 24). Neither makes cargo rebuild anything: a unit without a cache compiles from scratch the next time it changes;
- deletes feature variants that no build has read for `-VariantHours` (default 24). A variant is one build unit, `.fingerprint\<package>-<hash>` with its files in `deps\` and `build\`: the crate built with another feature set, profile or dependency graph. Every rebase that changes the workspace-hack or `Cargo.lock` leaves a full set behind; one worktree held four test executables of `cimmeria-cell-world`, one per dependency graph it had been built against that day. Cargo reads a unit's fingerprint files in every build that includes it, so their last-access time says when a build last used it. The most recently used variant of each package is always kept, and the step is skipped when the volume doesn't record last-access times. A deleted variant that is needed again is rebuilt, from sccache for third-party crates;
- optionally removes Dev Drive target dirs whose worktree is gone (`-RemoveOrphans`), and the old in-worktree target dirs once builds have moved to the Dev Drive (`-RemoveLegacyTargets`).

`-DryRun` reports what it would free per target dir, and `-Only <worktree>` limits it to named worktrees. It skips a target dir a lane job is building in, but can't see builds outside the lane, so don't run it while anything else builds.

On 2026-09-26 this freed 84.7 GB of stale artifacts and 70.6 GB of incremental caches from the main checkout alone. The per-worktree figures for the 2026-09-28 additions are in the [results](#sccache-and-disk-results-2026-09-28).

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

### 12. CI caches are saved from main only

GitHub keeps 10 GB of Actions cache per repository and evicts the oldest entries above that. In September 2026 the repo held 12.85 GB: every PR saved five Rust caches of 350–480 MB (clippy, build, test, test-live-db, coverage), which only that PR could read. Main's caches were evicted before PRs could restore them, and 3 of 5 jobs in one sampled run logged `No cache found`. A cache miss turned a 3-minute build job into a 7.6-minute one.

- Every `Swatinem/rust-cache` step in `test.yml`, `launcher-build.yml`, `client-patches-build.yml` and `client-telemetry-build.yml` sets `save-if: github.ref == 'refs/heads/main'`. PRs restore main's cache and never write their own. rust-cache doesn't cache workspace crates anyway, so a PR's own cache bought nothing that main's didn't.
- Those workflows cancel superseded runs only on PRs. Runs on `main` finish, because a cancelled run doesn't save its cache.
- `build-and-test` builds `--all-targets` and then runs nextest on the same artifacts. That's one workspace compile where there used to be two, on separate runners with separate caches.
- A `changes` job skips the Rust jobs on PRs that change only Markdown, `.claude/` or images under `docs/`. Tests read `docs/protocol/`, so it always counts as code.
- Coverage is two jobs that run at the same time, `coverage-workspace` and `coverage-live-db`, instead of one 9.5-minute job that ran both passes back to back. Each uploads its own report and Codecov merges them (`after_n_builds: 2`). A third job would only repeat the instrumented compile, which is most of each job's time.

## Consequences

- One toolchain, one set of artifacts, and CI clippy equals local clippy.
- Target dirs stop growing without bound, as long as the sweep runs.
- Agent worktrees share compiled third-party crates through sccache (78% hits in a fresh worktree, §3) and need a Dev Drive only for the extra Defender and cloning gains.
- Bumping Rust is now a visible, reviewable change instead of something CI does silently.
- Once §6 lands, a `workspace-hack` dependency appears in every crate's manifest. `cargo hakari manage-deps` maintains it; don't edit it by hand.

## Results

Measured on 2026-09-26 with `tools/build-metrics/measure-build.ps1`, on the same machine (i9-13900KF, 64 GB) and Rust 1.98.1. The baseline is main at `f153138b`, before the overhaul. The final measurement is this branch at `7aa6f9fd`. Both runs:

- started from a fresh worktree with an empty target dir;
- ran under `lane.sh --exclusive`, so nothing else was building;
- had sccache off, incremental on and `CARGO_BUILD_JOBS=10`.

The final run's target dir was on the Dev Drive. That is part of what is being measured.

| | Baseline | Final | Change |
|---|---|---|---|
| Cold build: `cargo build --workspace --all-targets`, gated set | 254.7 s | 146.2 s | −43% |
| Edit `cell/content/mod.rs`, then `cargo test -p cimmeria-services --no-run` (the baseline's loop) | 164.7 s | 16.3 s | −90% |
| The same edit, then `cargo check -p cimmeria-services` | 57.3 s | 11.5 s | −80% |
| The same edit, then `cargo test -p cimmeria-cell-content --no-run` (the loop now: test the crate you changed) | n/a | 17.7 s | |
| The same edit, then `cargo check -p cimmeria-cell-content` (first check after a cold `build`) | n/a | 23.2 s | |
| Peak working set of the build processes, cold | 9.4 GB | 2.9 GB | −69% |
| Lowest free RAM during the cold build | 19.3 GB | 28.1 GB | |
| Target dir after all phases | 14.2 GB | 7.1 GB | −50% |

- **Baseline memory figures.** The baseline's memory samples were re-parsed after the fix to the harness's thousands-separator bug, so its `summary.json` memory fields are wrong; the corrected figures are the ones above.
- **Why the edit loop dropped the most.** An edit in content now rebuilds content and the five crates above it, not a ~240k-line monolith. It rebuilds them incrementally, and no single rustc holds more than about 40k lines.
- **Why the cold build dropped less.** It still compiles every crate. The gains there come from:
  - parallel crates;
  - `line-tables-only` debug info;
  - the dependency dedupe;
  - the Dev Drive.

  The workspace-hack's cost to the critical path (§6) is included in the figure.
- **What the harness doesn't isolate:**
  - The incremental A/B in §3: an edit, then a `check` of content and services, took 17 s with incremental off and 5.5 s with it on.
  - The hakari churn in §6: a `-p` check after a workspace check rebuilt up to 36 third-party crates before, and none after.
  - The lane's job log, in real agent work on the last waves: the median `cargo check` fell from 27.7 s to 17.8 s, and the median scoped `nextest` from 40.6 s to 23.8 s.
- **Lane slots.** A cold build now peaks at about 3 GB. The job log never saw less than 27 GB free with four builds running, so four slots of `cores / 4` jobs each stay the default. The CPU, not memory, is now the limit on more.

The raw samples, `summary.json` files and cargo `--timings` reports stay in the measuring session's scratchpad. To re-measure, run the harness from a fresh worktree, using the same flags as the table.

## sccache and disk results (2026-09-28)

Measured for #1023 on the same machine, while other agents were building, so run times are noisy. The hit counts are exact: the runs used a private sccache server (`SCCACHE_SERVER_PORT`) with an empty cache of its own, so no other build touched them. Every run is a line in the lane's job log.

### sccache hits

Before the fix, the lane log recorded 2,536 hits against 72,958 misses (3.4%) on 2026-09-26, and 124 against 66,934 (0.2%) on 2026-09-27.

The controlled runs used four fresh worktrees with empty target dirs on the Dev Drive, run one after another. Each ran `cargo check -p cimmeria-common`, which sends the workspace-hack's third-party crates through sccache: 308 compiles.

| Run | Lane | Hits | Misses | Hit rate | Run time |
|---|---|---|---|---|---|
| 1st worktree, empty cache | before (main's `lane.sh`) | 0 | 308 | 0% | 77.3 s |
| 2nd worktree | before | 0 | 308 | 0% | 63.5 s |
| 3rd worktree | after | 0 | 308 | 0% | 56.9 s |
| 4th worktree | after | 241 | 67 | **78%** | 37.7 s |

The 3rd run misses everything because the key no longer contains the target dir, so the entries runs 1 and 2 wrote don't match. The 4th run shows what a new worktree gets once any other worktree has built the same crates.

A freshly seeded worktree gets the same rate. Its target dir was seeded with `Copy-WarmTarget.ps1` from a check-only target dir, so `cargo build -p cimmeria-common` had to compile the 171 rlibs the seed lacked. Another worktree had already run the same build. The seeded worktree scored 132 hits and 39 misses (77%) and ran in 21.4 s, against 31.4 s in the worktree that filled the cache.

The misses are `serde`, `serde_core` and `thiserror`, which hash `OUT_DIR` (§3), and 36 others, mostly crates built on them, such as `tokio`, `hyper`, `sqlx-core`, `chrono` and `url`.

### Disk

`sweep.ps1 -DryRun` over the Dev Drive target dirs that no lane job was building in at the time, with the default windows:

| Target dir | Before | After | Freed: incremental | Freed: variants |
|---|---|---|---|---|
| main checkout | 23.9 GB | 20.1 GB | 3.8 GB | 0 |
| worktree A (campaign, active all day) | 12.5 GB | 9.5 GB | 3.0 GB | 0 |
| worktree B (agent) | 9.0 GB | 7.4 GB | 1.6 GB | 0 |
| worktree C (agent) | 8.9 GB | 7.9 GB | 1.0 GB | 0 |
| **Total** | **54.3 GB** | **44.9 GB** | **9.4 GB (−17%)** | 0 |

- **Incremental.** Nearly all of the saving is the stale sessions of §3, about 45% of each incremental dir. The lane now removes those after every job, so a worktree no longer builds them up between sweeps.
- **Variants.** None of these dirs held a variant that no build had read for 24 hours. With `-VariantHours 6`, worktree A would have gone from 12.5 to 5.4 GB and the main checkout from 23.9 to 14.4 GB. The window stays at 24 hours so that a variant an agent still uses isn't rebuilt from scratch.
- **A real run on this PR's worktree.** After building `cimmeria-cell-world` and its tests twice with `LANE_PRUNE=0`, `sweep.ps1 -Only <worktree>` took the target dir from 2.2 to 1.8 GB, and its incremental dir from 1.02 to 0.65 GB. The next edit-then-`cargo check` still built incrementally (2.1 s). With pruning on, the lane removed 109 MB and 62 MB after two edit-then-check jobs.

#962's other workstream A item, measuring `debug = 0` or `strip = "debuginfo"` for test builds, is not part of this change.

## Standalone desktop launcher validation

The macOS/Windows launcher implementation in
[`crates/launcher/desktop/`](../../crates/launcher/desktop/README.md) is a scoped
exception to the Windows-only application build convention. Each OS builds
natively, using the pinned toolchain and build lane; Windows cross-compilation
remains unsupported. The root Cargo workspace and its generated graph do not
include this nested workspace. `.github/workflows/launcher-desktop.yml` invokes
its manifest explicitly on native Mac and Windows runners, including a
headless JS/Effect-to-Rust persistence UAT. These checks do not establish
packaged-webview behavior, game compatibility or self-contained first open.
