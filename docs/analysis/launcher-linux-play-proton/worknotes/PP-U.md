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

# PP-U: Deck UAT (attempt 3, 2026-10-25 22:45-23:20 local, real-key AppImage)

**Result: FAIL at the install step (U3, U8).** U1 passes and the release manifest now verifies, which clears the attempt-2 blocker. But the real-key AppImage cannot unpack the genuine release seed `client.rar`: both install paths (a local archive and the live download) stop at the start of the unpack with `RAR error: File header damaged`. The archive itself is good. Without an install no Play row can run, so the PhysX-under-Proton risk is **still unproven**.

Evidence (gitignored, on the Deck): the worktree's `.claude/reports/launcher-linux-play-proton/pp-u2/` (`PP-U2-*.png`, `root1-launcher.log`, `rootb-launcher.log`, `unrar-l.txt`). The report is `.claude/reports/launcher-linux-play-proton/pp-u-report-2.md`.

## Build under test

- workflow_dispatch run 40244390639, head `b7262377b`, job "desktop release build dry run (Linux, release manifest key) / Linux AppImage" (121325410198), **success**, `dev_manifest_key: false`. The CI log shows `embedded payload verified: 4 files`.
- Artifact `desktop-launcher-linux-real-key-dry-run`, downloaded into a new empty folder outside the repo. `StargateWorlds-Launcher-linux-x86_64.AppImage`, 88197624 B, SHA-256 `be084fa60542d9b85c4053c0ecf3278d387e628116bfe82f8303d2adc681133a`; `sha256sum -c` against the artifact's `.sha256` is OK.
- Launcher `version="1970.1.101"`, `release_build=true`, `dll_bundled=true`; the window title is "Cimmeria Launcher — development".
- Embedded pieces (from `--verify-embedded-payload <cache>`), SHA-256 prefixes: prerequisite worker `926ffc65…a855`, client patches `724668ac…5460`, client telemetry `c2720c5d…cf73`, launch worker `351dea3a…5460`. They differ from attempt 2, as expected after PP-08 and PP-08W.

## Setup

- Desktop Mode (KDE Plasma, Wayland, Xwayland `:0`), launcher run with `GDK_BACKEND=x11`, driven with `xdotool` as in attempt 2.
- Disk: 135 GB free on `/home`, enough for the 3.85 GB seed and its expansion.
- **Root 1 (fresh):** XDG data, config and cache under `~/pp-u2-20261025/root/`. Install from a local archive ("Use a downloaded game archive…" → `client.rar`, 4135724034 B, SHA-256 `7ba97ed2…4f07`, the release seed).
- **Root B (fresh):** the same layout under `~/pp-u2-20261025/rootb/`. "Install Cimmeria" downloads the seed from the live release.
- The real install (`~/.local/share/app.cimmeria.launcher.desktop`) was not touched.

## Rows

| Row | Result | Evidence |
|---|---|---|
| U1 | **PASS** | `--appimage-extract` exit 0; the binary is an x86-64 ELF; 0 `.exe`/`.dll` files in the AppDir; `--verify-embedded-payload <cache>` prints `embedded payload verified: 4 files`, exit 0. Startup logs `payload_cache_ready … files=4`. |
| Release manifest (attempt-2 blocker) | **PASS** | Both roots log `release_fetched` ("fetched and verified"). Root 1 logs `seed_archive_picked bytes=4135724034`. |
| U8, copy phase | **PASS** | "Switching to your archive…" shows at once, then "Copying your archive… 1.85 GB of 3.85 GB · 47.7 MB/s · about 43 s left" (`PP-U2-20`, `PP-U2-23`). `install_phase_finished phase="copy" … elapsed_ms=72673`. |
| U8, extraction | **FAIL** | `03:55:54.171Z seed_unpack_started check="hashed_while_copied" bytes=4135724034`, then 11 ms later `03:55:54.182Z install_stage_failed reason="unpack" unpack_kind="rar"` and `install_stage_detail … detail=Unpack error: RAR error: File header damaged`, then `install_failed duration_ms=72763`. The UI shows "Installation stopped. Partial files are preserved…" (`PP-U2-24`). |
| U3, install (download path) | **FAIL** | Root B: "Downloading verified game content…" (`PP-U2-32`); `04:13:08.852Z seed_unpack_started check="sha256"` after a 749 s download; `04:13:12.005Z install_stage_failed reason="unpack" unpack_kind="rar"`, the same `File header damaged` detail; `install_failed duration_ms=753923` (`PP-U2-33`). |
| U2, Proton list | **PASS** (attempt 2) | Not re-run; the list needs no install. |
| U2, which Proton Automatic picks | **NEEDS-OWNER** | By the D-PP4 ranking it should be GE-Proton11-11. Not confirmed: that needs a Play log. |
| U2, no-Proton copy | **BLOCKED** | It only shows with an installed game. |
| U3 Play: prefix, DirectX pin, PhysX, window, seeding (441 B), second Play refused | **BLOCKED** | No install. PhysX under Proton is still unproven. |
| U3, frame cap 30/60 (`DXVK_FRAME_RATE`) | **BLOCKED** | The options were checked in attempt 2; the environment needs a Play. |
| Telemetry tag `desktop-launcher/linux/proton` | **BLOCKED** | No Play, so no game session. The code (`session_tags`) gives `["desktop-launcher", "linux", "proton"]` for a Proton plan. |
| U4, U5 | **BLOCKED** | They need an install or a Play. |
| U6, U7 | Not run | As in attempt 2. |

## Findings

- F-U3 (**FAIL**, install): the real-key CI AppImage fails to unpack the genuine release `client.rar` on both install paths, so the launcher cannot install the game on Linux.
  - The archive is good. The staged copy (`.tmp-seed-7ba97ed2cb94.download`) is byte-identical to the original (`cmp` 0) in root 1. In root B its SHA-256 is the release's `7ba97ed2…4f07`. System `unrar` 7.23 lists 180 entries (3939121104 B) and `unrar t` reports `All OK`. The archive is RAR 1.5 format (RAR 4 family), stored (`-m0`), with an archive comment, a recovery record and the lock flag.
  - The error is `UnrarError` `(BadData, Read)` from the `unrar` crate 0.5.8 (a `read_header` failure). It comes within 11 ms of the unpack start, so it is at the first headers, in the listing pass or the first processing header, not partway through the data.
  - The same seed installed on this Deck on 2026-10-25 13:24-13:33 (PP-S1 test root) with a local **debug** build (`version="0.1.0"`, `release_build=false`): download, hash, `RAR holds an installer cabinet set; expanding it cabinets=4 files=5983`, `install_finished outcome="ok"`.
  - Suspects (not diagnosed, no code changed): (a) the CI release build of the bundled UnRAR (`unrar_sys` 0.5.8, unchanged in `Cargo.lock` since #986): release optimisation or the CI toolchain or container; (b) a launcher change between that debug build and `b7262377b`. In `unpack/rar.rs` the only change is PP-20a's `sink.report_index(listed)` in the listing loop (`8f928efac`). Other unpack-path changes are PP-25 `def6862b9`, PP-15 `da11b72a2` and the wave-1B fixes `279969d1e`. A test that extracts a stored RAR with an archive comment, a recovery record and the lock flag in a release build would separate (a) from (b). The fixtures in `test_fixtures::write_stored_rar4` may not cover these.
- F-U4: after "Remove partial files" the button reads "Retry installation" (`PP-U2-26`). The archive choice ("Using client.rar (verified)") persists across a launcher restart (`PP-U2-28`).

# PP-U: discrimination runs (ruling 1, step 1; 2026-10-25 23:25-23:46 local)

**Result: 1a passes, 1b fails, 1c fails.** The same seed (`~/Downloads/sgw-seed/client.rar`, 4135724034 B, SHA-256 `7ba97ed2…4f07`) was installed through "Use a downloaded game archive…" by three launchers, each in its own fresh root under `~/pp-u3-20261025/`. Yesterday's local debug build unpacks it. The CI-built binary run outside the AppImage fails with `RAR error: File header damaged`, the same as the AppImage did in attempt 3. Main's released AppImage fails at the unpack stage too. This is the pattern of ruling row O2. The ruling on it is the architect's.

Evidence (gitignored, on the Deck): `.claude/reports/launcher-linux-play-proton/pp-u3/` (`rootC-`, `rootE-`, `rootD-launcher.log`, the `*-environ.txt` and `*-maps-so.txt` files, `shots/`). The report is `.claude/reports/launcher-linux-play-proton/pp-u-discriminate.md`.

## Setup

- Desktop Mode, Xwayland `:0`, `GDK_BACKEND=x11`, driven with `xdotool` after a KWin script raised the launcher and moved it under the pointer, as in attempts 2-3.
- Every root has its own `XDG_DATA_HOME`, `XDG_CONFIG_HOME` and `XDG_CACHE_HOME`. The real install and the attempt-3 roots were not touched.
- Run 1.0: the idle root B launcher's `environ` (secret-like lines removed) and `.so` map lines were saved to `pp-u3/rootb-*.txt`, then the launcher was stopped (SIGTERM). It had to stop: the single-instance guard would have handed every new launch to it.
- In all three builds, picking the archive starts the install by itself; Install was not pressed.

## Runs

| Run | Binary | Root | Result | Log lines |
|---|---|---|---|---|
| 1a | `target/desktop/debug/cimmeria-launcher-desktop`, built 08:20 on 10-25 (`version="0.1.0"`, `release_build=false`) | `rootC` | **PASS** | `04:33:04.70Z install_started`, `04:36:26.28Z Unpacking seed into …`, `04:39:01.10Z install_phase phase="extraction"`. `.tmp-unpack/Data/` held 9 files: `DATA1.CAB` (1073741824 B, complete), `DATA2.CAB` part-written, plus `DATA.DDF`, `DATA.INF` and `DATA.RPT`. UI: "Extracting game content… 3%". No `install_stage_failed`. Stopped (SIGTERM) at 04:40 |
| 1b | the attempt-3 AppImage's inner `usr/bin/cimmeria-launcher-desktop` (SHA-256 `64398a24…508f`), from a new `--appimage-extract` of the verified AppImage (`be084fa6…133a`), started without AppRun | `rootE` | **FAIL** | `04:43:16.532340Z seed_unpack_started check="hashed_while_copied" bytes=4135724034`, then `04:43:16.538685Z install_stage_failed reason="unpack" unpack_kind="rar"`, `install_stage_detail … detail=Unpack error: RAR error: File header damaged`, `install_failed duration_ms=68101`. That is 6 ms after the unpack start, and `.tmp-unpack` was never created |
| 1c | main's release AppImage, tag `launcher-20261024-c22f36c` (`version="2026.10.2401"`, `release_build=true`) | `rootD` | **FAIL** | `04:45:10.651Z Unpacking seed into …`, then `04:45:14.701Z install_stage_failed reason="unpack"`, `install_failed duration_ms=68965`. `state/install-result.json` has `"outcome":"install_failed"`. UI: "Installation stopped. Partial files are preserved…". This version logs no `install_stage_detail` and no `unpack_kind`, so the error text cannot be read. As in 1b, `.tmp-unpack` was never created |

- 1b without AppRun:
  - Started from the root, it aborts: WebKit's patched helper path `././/lib/x86_64-linux-gnu/webkit2gtk-4.1/WebKitNetworkProcess` is relative to the working directory.
  - With the working directory set to `squashfs-root/usr`, `WebKitWebProcess` cannot find `libwebkit2gtk-4.1.so.0`.
  - It ran with the working directory `squashfs-root/usr` and `LD_LIBRARY_PATH=squashfs-root/usr/lib` only. None of AppRun's GTK hook variables were set (`APPDIR`, `APPIMAGE`, `GDK_PIXBUF_MODULE_FILE`, `GIO_MODULE_DIR`, `GSETTINGS_SCHEMA_DIR`, `GTK_*`, `GST_*`, `PYTHON*`, `PERLLIB` are all absent from `1b-environ.txt`), and the squashfs was not mounted.
- **The extracted CI binary fails the same way as the AppImage.** The failure is in the binary, not in the AppImage runtime (AppRun's environment).
- Main's release: commit `c22f36c6197a20bd658b5bd73e541ded3463ea78` (the #2020 merge itself, so not older than the native Linux install), run 40136981222. `StargateWorlds-Launcher-linux-x86_64.AppImage`, 86436344 B, SHA-256 `cc97bc9b2908415f15a98e512eb3c630b58759a714bbea180a3de052714511ff`. `sha256sum -c` against the release's `.sha256` asset is OK, and the hash matches GitHub's asset digest. It was downloaded with `gh release download` into the new empty folder `~/pp-u3-20261025/release/`.

## Compiler, glibc and libunrar facts

- **Lane compiler on the Deck** (built 1a): the host has no `c++` on the system PATH. `~/.bashrc` puts `~/.local/opt/cimmeria-cc-bin` first. Its `c++` is a shim that drops `-stdlib=libc++` and runs conda-forge **g++ 16.2.0** (`~/.local/opt/cimmeria-cc`), with mold 2.40.4 as the linker. The 1a binary's `.comment` has `GCC: (conda-forge gcc 16.2.0-7)`, `GCC: (Ubuntu 13.3.0-6ubuntu2~24.04.1)`, `GCC: (GNU) 14.3.1 (Red Hat)`, `clang 22.1.0-rc2`, `rustc 1.98.1` and `mold 2.40.4`. Its RUNPATH is `~/.local/opt/cimmeria-cc/lib`, so it loads conda's `libstdc++`.
- **CI compiler** (1b, the attempt-3 build): `.comment` has `GCC: (Ubuntu 11.4.0-1ubuntu1~22.04.3)`, `GCC: (Ubuntu 12.3.0-1ubuntu1~22.04.3)`, `clang 22.1.0-rc2`, `rustc 1.98.1` and `mold 1.0.3`. Its RUNPATH is `$ORIGIN/../lib`.
- **glibc**:
  - Deck host: SteamOS glibc 2.43.
  - The `cimmeria-dev` container (Ubuntu 24.04, where 1a runs): glibc 2.39. The 1a binary needs `GLIBC_2.39` and `libwebkit2gtk-4.1`, which the host lacks, so 1a ran inside that container (`podman exec`), as the PP-S1 debug launcher did.
  - The CI binary needs at most `GLIBC_2.34` and `GLIBCXX_3.4.29`.
  - The AppImage (root B), 1b and 1c all load the host's `/usr/lib/libc.so.6`, `libstdc++.so.6.0.35` and `libgcc_s.so.1`. None of these is bundled.
- **libunrar**: none on the host, in the container or in the AppDir, and none in any process map. UnRAR (`unrar_sys` 0.5.8, vendored) is linked statically into the launcher binary. The system `unrar` 7.23 CLI is a separate program.
- Caveat: 1a and 1b differ in the C++ compiler **and** in the userland they ran on (the Ubuntu 24.04 container with conda `libstdc++`, against the SteamOS host). This run does not separate the two.
