# Windows on Arm desktop launcher release (launcher-win-arm64)

> **Type:** Reference
> **Audience:** Launcher contributors, the orchestrator and the owner
> **Last updated:** 2026-10-24
> **Companions:** [Work packets](work-packets.md), [resume card](RESUME.md), [release.md](../../../crates/launcher/desktop/docs/release.md), [updater.md](../../../crates/launcher/desktop/docs/updater.md)

Every launcher release ships a native Windows arm64 desktop launcher beside the x64 one, and each architecture updates itself from the same signed `latest.json`. The owner confirmed the launcher works on their Windows on Arm laptop. Prefix `WA`. Designed 2026-10-24 against `main` `2019922fe`. Line numbers below are from that commit; every packet re-greps its anchors.

## Findings

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | The release builds one Windows zip, `StargateWorlds-Launcher-windows-x64.zip`, on `windows-latest`. `build-desktop.sh verify` requires PE machine `0x8664`. | `tools/launcher-release/build-desktop.sh` l.93, l.215 | WA-02, WA-04 |
| F2 | The launcher EXE embeds three i686 Play pieces: the launch worker and two game DLLs. They stay i686 on Arm, because the game is 32-bit x86 and runs under Windows' x86 emulation. An arm64 job can take the x64 job's staged pieces through `play-pieces-in` and the digest job output, as `mac-pieces` already does. It then compiles no i686 code. | `build-desktop.sh` l.343-381; `launcher-release.yml` `mac-pieces` | WA-04 |
| F3 | The updater's platform key is the constant `WINDOWS_PLATFORM = "windows-x86_64"`. The `policy.rs` allow-list holds only `darwin-aarch64`, `darwin-x86_64` and `windows-x86_64`. `apply.rs` refuses a Windows swap for any other platform. A feed without the build's key gives `Error::Platform`. | `engine/src/storage/updater/release_check.rs:19`, `policy.rs:39-45`, `apply.rs:196-199` | WA-01 |
| F4 | `release_feed sign` signs one zip and writes a one-platform feed; `verify` checks that one platform. | `engine/examples/release_feed.rs` | WA-01, WA-02 |
| F5 | The example's own `#[cfg(test)]` tests never run: `cargo test` runs an example's tests only when the example is declared with `test = true`. | the engine `Cargo.toml` has no `[[example]]` | WA-02 |
| F6 | `publish.sh` uploads one Windows set, and its notes name only the x64 zip. | `tools/launcher-release/publish.sh` | WA-03 |
| F7 | The PR dry run signs a one-zip feed with a throwaway key, inside `release-dry-run`. | `.github/workflows/launcher-desktop.yml` l.190-194 | WA-05 |
| F8 | Hosted `windows-11-arm` runners are free for public repositories, and this one is public. The self-hosted `cimmeria-windows` pool has no Arm machine. | `tools/ci-runners/README.md` § Windows pool | WA-04, WA-05 |
| F9 | `ring` and `aws-lc-sys` need clang to build for `aarch64-pc-windows-msvc`. The frontend lockfile has `@esbuild/win32-arm64` and no native rollup binary, so `npm ci` works on Arm. Whether the hosted image already has clang, Python and rustup is only proved by the first CI run. | crate build notes; `frontend/package-lock.json` | WA-04 |
| F10 | Launcher telemetry already reports `aarch64` (`launcher_summary/schema.rs:141-160`), so the privacy inventory doesn't change. | | none |
| F11 | The bootstrap installer and `setup.ps1` download the x64 zip by name. x64 runs emulated on Arm, so they keep working. Switching them has to wait until a release actually carries the arm64 zip. | `Install-CimmeriaLauncher.ps1:44`, `setup.ps1:152` | WA-F1 |
| F12 | The implementation ledger the doc-update map names is 990 lines, over the 700-line hard cap. | `docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-ledger.md` | D-WA11 |
| F13 | Nothing validates `launcher-release.yml` until it is dispatched: it has only `workflow_dispatch`, and the repo has no actionlint. The reusable arm64 workflow is validated on every PR, because the dry run calls it. The release wiring gets policy tests in WA-04, and its first real test is WA-U2. | | WA-04, WA-U2 |

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-WA1 | Build on the hosted `windows-11-arm` runner, not by cross-compiling `aarch64-pc-windows-msvc` from x64. | Decided | `verify` runs the EXE's embedded-payload probe (cold extraction, reuse, repair), which needs the arm64 EXE to run. A cross-compile can't be smoke-tested. The runner is free here (F8). |
| D-WA2 | Feed key `windows-aarch64` (Tauri's naming), zip `StargateWorlds-Launcher-windows-arm64.zip` (the same pattern as `macos-arm64`), PE machine `0xaa64`. Every x64 name stays as it is. | Decided | Old launchers and links keep working. |
| D-WA3 | The compile-time `target_arch` picks `WINDOWS_PLATFORM`. An x64 build running on Arm keeps updating to x64; there is no automatic cross-grade. | Decided | Pure compile-time, with no runtime OS probe. A cross-grade (IsWow64Process2) is a possible later follow-up. |
| D-WA4 | One `latest.json` carries both Windows keys. The existing `sign` job signs both zips in one `release_feed sign` call. `publish` uploads both sets, and the feed last. | Decided | Backward-compatible: old x64 launchers read only their own key. One feed means one atomic swap on `launcher-current`. |
| D-WA5 | The arm64 build is on the release's critical path: `sign` and `publish` need it. Recovery is "Re-run failed jobs". The `desktop-windows` Play-pieces steps lose `continue-on-error`, and their artifact retention goes from 1 to 7 days. | Decided | A feed without `windows-aarch64` gives every arm64 launcher `Error::Platform`, so after the first arm64 release every feed must carry both keys. The longer retention lets a rerun on a later day still find the pieces. |
| D-WA6 | A new reusable `.github/workflows/launcher-desktop-windows-arm64.yml`. The release calls it as `desktop-windows-arm64`, and the PR dry run as `release-dry-run-arm64`. | Decided | The PR runs exactly the release job, as `launcher-desktop-macos.yml` already does for macOS. |
| D-WA7 | The PR dry run builds arm64 natively, then a new `release-dry-run-feed` job signs and checks the two-zip feed with a throwaway key. The feed step moves out of `release-dry-run`. The new jobs don't gate merges, the same as today's dry run (`ship.py` `CONDITIONAL` lists only `native-engine`). | Decided; gating optional (owner) | It catches arm64 toolchain and feed-format breakage before release day. Making the dry runs required is an owner or admin choice, and doesn't block this campaign. |
| D-WA8 | The arm64 job checks its tools before building: `python`, then `clang` (Chocolatey LLVM when missing), then `rustup` (official `rustup-init` when missing). It pins the default host to `aarch64-pc-windows-msvc` and asserts it after the toolchain action runs. No new third-party action is added, and the release actions stay SHA-pinned. | Decided | F9. The host assert turns a silent x64 toolchain into a clear failure; `verify`'s `0xaa64` check would catch it too, but later. |
| D-WA9 | Declare `[[example]] name = "release_feed"` with `test = true`, so the example's parser and version tests run under `cargo test`. | Decided | F5. |
| D-WA10 | The bootstrap installer and `setup.ps1` stay x64 in this campaign. WA-F1 makes them pick by architecture once a release carries the arm64 zip. | Decided | F11. Changed now, they would 404 until the first arm64 release. |
| D-WA11 | Don't grow the over-cap implementation ledger. This README is the record; WA-06 lists the ledger row as skipped in the PR body, and WA-F2 splits the ledger. | Decided | F12; the hard cap forbids growing a file that is already over 700 lines. |

**Owner decisions:** none block this campaign. The only owner call is optional: whether to make the arm64 dry run (and the existing x64 dry run) a required check (D-WA7).

## Packets

Wave 1 has six packets and ships as **one PR** from the integration worktree `lwa-integration` (branch `launcher-win-arm64/integration`). Merge order into it: WA-01, WA-02, WA-03, WA-04, WA-05, WA-06. That PR's CI holds the first real arm64 compile; if it fails, the orchestrator opens a fix packet from the failed job's log.

| ID | Title | Agent | Depends | Status |
|---|---|---|---|---|
| WA-01 | Engine: two Windows platforms, feed builder and checker | packet-coder | - | Ready |
| WA-02 | `release_feed` CLI, example tests and `build-desktop.sh` arch rows | packet-coder | WA-01 | BlockedDependency |
| WA-03 | `publish.sh`: both Windows zips | packet-coder | - | Ready |
| WA-04 | Reusable arm64 workflow and the release jobs | packet-coder | - | Ready |
| WA-05 | PR dry run: arm64 build and the two-zip feed job | packet-coder | - | Ready |
| WA-06 | Docs | documentation-writer | - | Ready |
| WA-07 | Close-out: status docs, unified UAT, retire | documentation-writer | wave PR merged, campaign review `ship` | BlockedDependency |
| WA-U1 | Owner: the PR's arm64 dry-run zip on the Arm laptop | owner | wave PR CI green | BlockedDependency |
| WA-U2 | Owner: first release, install and Play on Arm | owner | first release after merge | BlockedDependency |
| WA-U3 | Owner: second release, self-update on arm64 and on x64-on-Arm | owner | a second release | BlockedDependency |
| WA-F1 | Follow-up: bootstrap installer and `setup.ps1` pick the arm64 zip on Arm | later campaign | WA-U2 | BlockedDependency |
| WA-F2 | Follow-up: split the over-cap implementation ledger | later docs packet | - | BlockedDependency |

### Owner UAT (no lab use)

| ID | Action | Expected |
|---|---|---|
| WA-U1 | From the wave PR's CI run, download `desktop-launcher-windows-arm64-dry-run`, unzip it on the Arm laptop, run `cimmeria-launcher-desktop.exe`. | The window opens on the first run. Task Manager's Architecture column (Details tab) says ARM64. The main page renders. This is a development build, so it isn't expected to install from the release manifest. |
| WA-U2 | After the first release with this change, download `StargateWorlds-Launcher-windows-arm64.zip` from `launcher-current`, then Install or adopt, and press Play once. | The launcher shows ARM64 in Task Manager. Install or adoption completes. Play starts the game, which runs x86-emulated. The dated release notes show the **Windows on Arm:** paragraph. |
| WA-U3 | After a second release, open the arm64 launcher from WA-U2, and separately an x64 launcher on the same laptop. | Each offers the new version and restarts into it. The arm64 build is still ARM64 and the x64 build is still x64 (D-WA3). |
