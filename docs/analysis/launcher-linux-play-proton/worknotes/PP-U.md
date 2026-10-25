# PP-U: Deck UAT (attempt 2, 2026-10-25 18:00-18:15 local)

**Result: blocked at U3.** U1 passes. The PR's AppImage is built with the development manifest key, so it can neither install the published `client.rar` nor play an install made against the real signed release. Every Play row (U3, U4, U5, U8, and the Play half of U2) needs an AppImage built with the real manifest public key, and none can be built from this branch without a maintainer decision (see [Blocker](#blocker)). No Play was run, so the PhysX-under-Proton risk is **still unproven**.

Evidence (gitignored, on the Deck): the worktree's `.claude/reports/launcher-linux-play-proton/pp-u/` (`PP-U-*.png`, `root1-launcher.log`, `root2-launcher.log`). The report is `.claude/reports/launcher-linux-play-proton/pp-u-report.md`.

## Build under test

- Source: draft PR #2032, head `306827aa154e36143a05e88d56a1520dcdac75d6` (integration branch tip).
- AppImage: CI artifact `desktop-launcher-linux-dry-run` of run 40241140189 (`pull_request`, head `306827aa1`), job "desktop release build dry run (Linux) / Linux AppImage" (121316266350), **success**. The owner's view that the Linux job did not run is not borne out: it ran on this head.
- `StargateWorlds-Launcher-linux-x86_64.AppImage`, 88197624 B, SHA-256 `d2f0a8e8f31441499315ce22e641691f6b32c8ed132c6628da39843d12187cf0`, the same as the artifact's own `.sha256`. Launcher version `1970.1.101` ("Development build"), `release_build=true`, `dll_bundled=true`.
- The file left by the stopped first attempt, `~/Downloads/StargateWorlds-Launcher-linux-x86_64.AppImage` (Oct 24, SHA-256 `cc97b9b2…11ff`), does **not** match and was not used.
- Embedded pieces (from `--verify-embedded-payload`), SHA-256:

| Piece | SHA-256 | Matches CI `desktop-play-pieces-dry-run` |
|---|---|---|
| cimmeria-launch-worker.exe | `2d14c790019e63ecebeb7e76d834c99e1f535576da4eae5e169bc504dcfbb425` | yes |
| cimmeria_client_patches.dll | `3af91116fdfe9c2ded879135cb96cdca37f46275eb52520fce3c829e29dc1020` | yes |
| cimmeria_client_telemetry.dll | `376887f124dba872d3d564eb13681bb14cf650224a313451dfd4506365082fae` | yes |
| cimmeria-prerequisite-worker.exe | `b0fa5b10f7475c2afb7dd9c3623e46ac43ff73b0458db213406946e96127a70f` | not in that artifact (embedded by the Linux job) |

No local build was made.

## Setup

- Desktop Mode (KDE Plasma, Wayland, Xwayland `:0`), launcher run with `GDK_BACKEND=x11`.
- **Root 1 (fresh):** `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and `XDG_CACHE_HOME` under `~/pp-u-20261025/`. No prefix, no install.
- **Root 2:** a `cp -a` of the PP-S1 test root (`~/cimmeria-launcher-test-20261025/xdg-data/…`, `content_prepared`, no `proton/`) into `~/pp-u-20261025/root2/`. The original was not modified.
- The real install (`~/.local/share/app.cimmeria.launcher.desktop`) was not touched.
- **Process stopped:** the PP-S1 debug launcher (`target/desktop/debug/cimmeria-launcher-desktop`, started 08:21, idle since 08:36, test root only) still held the single-instance lock, so a new launcher would only have focused it. It was stopped with SIGTERM. No files were changed.
- Driving: `xdotool` cannot warp the pointer while it is over a Wayland window. The launcher was moved under the pointer with a read-only KWin script, then driven with `xdotool` (pointer coordinates scaled by 1/1.35).

## Rows

| Row | Result | Evidence |
|---|---|---|
| U1 | **PASS** | `--appimage-extract` exit 0. `usr/bin/cimmeria-launcher-desktop` is an x86-64 ELF. `--verify-embedded-payload` prints `embedded payload verified: 4 files`, exit 0 (also in the CI log). `find squashfs-root -iname '*.exe' -o -iname '*.dll'` finds 0 files. Startup logs `payload_cache_ready … files=4`. |
| U2, Proton list | **PASS** (data); the default needs the owner | Settings → Proton for Play shows Automatic, GE-Proton11-11, GE-Proton7-38, experimental-11.0-20261014-x86_64, proton-11.0-2e-x86_64, proton-10.0-4b, proton-9.0-4f, proton-8.0-5d, proton-7.0-6f (`PP-U-26`, `PP-U-27`). That is every Proton of F-04 plus the SD card's 10.0 and 11.0, in D-PP4 order. Escape left the choice at Automatic (`PP-U-28`). **NEEDS-OWNER:** the UI does not say which Proton Automatic resolves to. |
| U2, no-Proton copy | **BLOCKED** | `protonInstallText` only shows with an installed game (`launch-view.ts:44,56`); no install is possible (see U3). |
| U3 | **BLOCKED** | Root 1: "Use a downloaded game archive…" → `client.rar` → `seed_archive_pick_refused reason="verify_failed"`, after `release_fetch_failed reason="signature"`. The UI shows "The signed game release failed verification, so your archive was not checked…" (`PP-U-12`, below the fold). Root 2: `invoke_failed area="launch" code="corrupt_state"`, and Play is disabled with "The launcher's saved state is damaged" (`PP-U-20`). |
| U3, frame-cap options | **PASS** (options only) | Settings: Frame rate limit offers "30 fps (cooler, quieter)" (default) and "60 fps", with no Match display (`PP-U-29`), as wave3 ruling 1 says. `DXVK_FRAME_RATE` was not checked (no Play). |
| U3 Play, prefix, DirectX, PhysX, window, seeding | **BLOCKED** | Not run. PhysX under Proton is still unproven. |
| U4, U5, U8 | **BLOCKED** | They need an install or a Play. |
| U6 | Not run | A follow-up (OD-2). |
| U7 | Not run here | The packet-coder's row. `uat:launch` cannot run on Linux (FU-14). |

## Blocker

- **Cause (code, by design):** the PR dry run builds Linux with `dev_manifest_key: true` (`launcher-desktop.yml:395` → `launcher-desktop-linux.yml:136`, key `197f6b23…8d61`). The live release manifest is signed with the real key, so `fetch_release` fails `signature`. Stored release evidence is re-verified with the embedded key on every load (`release_evidence/mod.rs:104`), so an install made with the real key reads as `corrupt_state` under the dev key.
- **No CI path:** `launcher-desktop.yml` has no `workflow_dispatch`; `launcher-desktop-linux.yml` is `workflow_call` only; `launcher-release.yml` builds Linux only on `refs/heads/main` (`:661`).
- **No local path here:** a local build needs `LAUNCHER_MANIFEST_PUBKEY_HEX` (a repository secret) and the build lane, and `pwsh` is not installed on the Deck. A direct `cargo` call is refused by the hook.
- **Options for the owner:**
  - (a) Supply the release public key hex and allow a local AppImage build from `306827aa1`. The Linux lane is unvalidated; the i686 pieces come from the run 40241140189 artifacts.
  - (b) A workflow change (a `workflow_dispatch` input, or a non-dev-key Linux dry run) on the branch, then dispatch it.
  - (c) Test against a release signed with the dev key, served from a local endpoint. This needs code, because the URL is fixed.

## Findings beyond the rows

- F-U1: no defect found in the PR code. The block is a test-environment gap: the PR's Linux AppImage cannot be UAT'd for Play against the real release. The ledger's plan ("PP-U on the PR's AppImage, artifact desktop-launcher-linux-dry-run", RESUME) does not work as written.
- F-U2: the archive-refused feedback is below the fold on the 620x688 window (`PP-U-11` shows no change; `PP-U-12`, after scrolling, shows the message). Whether that meets "visible feedback on the first press" is **NEEDS-OWNER**.
