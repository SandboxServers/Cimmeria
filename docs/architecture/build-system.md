---
title: "Build system: toolchain, profiles, concurrency and disk"
type: explanation
audience: engineers and agent operators
last_updated: 2026-09-26
---

# Build system: toolchain, profiles, concurrency and disk

> **Status:** Accepted (2026-09-26). Implemented on `build/toolchain-overhaul`, except §6 (cargo-hakari, planned as the last step, after the crate split) and §9 (the crate split, in progress).
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

- **Slots:** `LANE_SLOTS`, defaulting to `%LOCALAPPDATA%\cimmeria-build\lane\SLOTS` (2). `--exclusive` takes every slot.
- **Jobs:** `CARGO_BUILD_JOBS` defaults to 10 per build.
- **Compiler cache:** sccache is the `RUSTC_WRAPPER` when installed, with one shared cache.
- **No incremental cache in linked worktrees:** worktrees build with `CARGO_INCREMENTAL=0`. sccache cannot cache incremental compilations, so this lets a worker reuse every workspace crate it didn't touch, and it removes the incremental cache, the largest part of a warm target dir. The main checkout keeps incremental builds.
- **Per-worktree target dirs:** each worktree keeps its own target dir. Cargo locks a target (or `build-dir`) for the whole build, so a shared one would serialise every worktree. Fine-grained locking is nightly-only and was reported deadlocking in September 2026 ([cargo#17508](https://github.com/rust-lang/cargo/issues/17508)).

`reload-db.sh` and `live-db-test.sh` live alongside it and give each worktree its own test database. `mk-worktree.sh` creates a buildable worktree.

### 4. Dev Drive for build output (optional, recommended)

`tools/dev-drive/New-CimmeriaDevDrive.ps1` creates a dynamically sized VHDX formatted as a Windows Dev Drive. It must run from an elevated PowerShell. It also sets `CIMMERIA_TARGET_ROOT` and `CIMMERIA_SCCACHE_DIR` for the user, and the lane then places target dirs and the sccache cache on that drive.

- **Defender:** a Dev Drive is a ReFS volume that Defender scans in performance mode.
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

### 6. Feature unification with cargo-hakari (planned)

A generated `workspace-hack` crate (`cargo hakari generate` / `manage-deps`) will make every workspace crate request the same feature set for shared dependencies, so `-p` builds reuse what `--workspace` built and the reverse. A CI step will verify the hack is up to date. It is generated after the crate split, which rewrites most manifests.

Cargo's built-in equivalent (`resolver.feature-unification`, [cargo#14774](https://github.com/rust-lang/cargo/issues/14774)) is still nightly-only.

### 7. One integration-test binary per crate

Every integration test file under `tests/` is its own binary that relinks the whole dependency tree. Crates with several files now use `tests/it/main.rs` with one module per former file (content-engine, wireclient, navmesh-extractor: 23 binaries became 3). New integration tests go in `tests/it/`.

### 8. Fewer and deduplicated dependencies

The September pass removed about 30 unused dependencies and collapsed duplicate versions: 341 → 332 unique crates in the gated workspace, and the services dependency tree went from 277 to 258 crates. Duplicates that remain are pinned by third-party crates; the reasons are recorded in `.claude/agent-memory/rust-gameserver-dev/dependency-dedupe-blockers.md`.

To find who pulls an old version, run `cargo tree -i <crate>@<version>`. Run `cargo machete` before adding a dependency cleanup PR.

### 9. The services crate is being split (in progress)

See [services-crate-split.md](services-crate-split.md) for the plan and the per-wave status.

### 10. Measure before changing concurrency

`tools/build-metrics/measure-build.ps1` measures the cold build, the edit loop, `cargo check`, peak memory and target size. Run it from a worktree with an empty target dir, under `lane.sh --exclusive`. Re-measure before changing the lane's slot count or `CARGO_BUILD_JOBS`.

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
