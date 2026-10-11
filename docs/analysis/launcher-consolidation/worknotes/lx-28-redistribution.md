# LX-28 redistribution check and macOS CI design

> Type: explanation and design record. Audience: the launcher-consolidation coordinator, the LX-28 implementer and maintainers.
> Updated: 2026-10-11. Companions: [campaign README](../README.md) (D-LX10, D-LX14), [work packets](../work-packets.md) (LX-13, LX-15, LX-28), [Wine validation](../../../../crates/launcher/desktop/docs/wine-validation.md), [native game launch contract](../../../../crates/launcher/desktop/docs/launch.md), [Mac runtime provisioning evidence](../../playtests/2026-10-03-macos-wine/runtime-provisioning.md), [signed launcher updates](../../../../crates/launcher/desktop/docs/updater.md).

This is the "check first" step of LX-28. It lists every third-party binary the macOS desktop launcher bundles or fetches, records each one's licence and whether a public zip may carry it, checks what ad-hoc signing gets a player on current macOS, and designs the `desktop-macos` release job. The job landed in the LX-28 implementation PR; its deviations from this design are in [Implementation and deviations](#implementation-and-deviations-from-this-design).

Licence findings are an engineering reading of upstream licence files, not legal advice. Anything not checked against a source is marked **unverified**.

## Verdict

**LX-28 can proceed.** Every piece the Mac app needs is either ours, carried in the zip under a permissive licence, or already fetched at first run from its upstream with a pinned size and SHA-256. Nothing required is both unshippable and unfetchable, so there is no stop for the coordinator.

Conditions on the CI job:

1. Ship our five Windows-built pieces and the D9VK `d3d9.dll` in the app; keep `D9VK-LICENSE.txt` beside it.
2. Keep the Wine runtime out of the zip. The launcher already downloads it at first run (`engine/src/mac_runtime/`).
3. Leave the optional x87 accelerator (rosettax87) out. Stock Rosetta is the tested path.
4. Use the `macos-15` arm64 runner, not `macos-14`: GitHub marks macOS 14 images deprecated.
5. Ship macOS self-update as **manual** in LX-28. The engine's macOS apply exists, but the shell disables it and the packaged path has never been run. Enabling `darwin-aarch64` is a small follow-up packet (see [Self-update on macOS](#self-update-on-macos)).
6. The player guide must tell players to drag the app to Applications in Finder before the first open, and then use Open Anyway once.

## What the Mac app carries or fetches

The inventory comes from the code, not the docs: the compile-time pins in `shell/src/host.rs` and `shell/src/host/launch/resources.rs`, the bundle resource map in `shell/tauri.conf.json` (`windows/` and `graphics/`), and the runtime pin in `engine/src/mac_runtime/mod.rs`.

| Piece | Where it comes from | How the app gets it | Pin |
|---|---|---|---|
| `cimmeria-archive-worker.exe` (x86-64 PE) | this repo, engine bin | bundled, `Contents/Resources/windows/` | `CIMMERIA_WINDOWS_HELPER_SHA256` |
| `cimmeria-prerequisite-worker.exe` (x86 PE) | this repo, `cimmeria-runtime-probe` | bundled, `windows/` | `CIMMERIA_PREREQUISITE_HELPER_SHA256` |
| `cimmeria-launch-worker.exe` (x86 PE) | this repo, `cimmeria-runtime-probe` | bundled, `windows/` | `CIMMERIA_LAUNCH_HELPER_SHA256` |
| `cimmeria_client_patches.dll` (x86 PE) | this repo | bundled, `windows/` | `CIMMERIA_CLIENT_PATCHES_SHA256` |
| `cimmeria_client_telemetry.dll`, player build (x86 PE) | this repo | bundled, `windows/` (optional) | `CIMMERIA_CLIENT_TELEMETRY_SHA256` |
| D9VK `d3d9.dll` (x86 PE) | WoWSilicon v3.2.2 resource | bundled, `graphics/` (required on Mac) | `CIMMERIA_D3D9_SHA256` = `033cef34…045e` |
| rosettax87 + `libRuntimeRosettax87` (arm64 Mach-O) | WoWSilicon v3.2.2 resource | optional bundle pair, `graphics/` | `CIMMERIA_ROSETTA_X87_SHA256`, `_LIBRARY_SHA256` |
| WoWSilicon Wine runtime r17 (`.tar.xz`, x86-64 Mach-O tree) | WoWSilicon GitHub release `wine-runtime-r17` | downloaded at first Install into the native cache | size 63,744,856, SHA-256 `dc67cf0c…f882`, plus a full tree digest |
| Rosetta 2 | Apple system software | the player installs it (`softwareupdate --install-rosetta`); the launcher only probes | none |
| PhysX 7.11.13 MSI, VC80 runtime, D3DX9, XInput | the player's own original client archive (`Data/Prerequisites`) | retained from the archive the player downloads from archive.org | per-file SHA-256 |

What it does not use: no D3DMetal and nothing from Apple's Game Porting Toolkit, no DXVK build other than the D9VK DLL above, no separate MoltenVK copy (MoltenVK comes inside the Wine runtime), no fonts (the frontend ships none; Wine's own fonts are inside the runtime), and no prefix template (prefixes are created on the player's Mac). A search of `crates/launcher/desktop/` for `d3dmetal`, `gptk` and "Game Porting" finds nothing.

### Contents of the Wine runtime

WoWSilicon's [runtime lock](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/runtime-lock.json) and [assembly script](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/tools/wine-runtime/assemble.sh) say what r17 is: Wine 11.13 from the [WineAndAqua fork](https://github.com/WineAndAqua/wine/tree/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8) at `37540b5d`, plus overlays:

- MTLd3D v0.7.0 (`d3d9.dll`, `mtld3d.dll`, `mtld3d.so`, `mtld3d.conf`), a Direct3D 9 on Metal layer. The [release notes](https://github.com/WoWSilicon/WoWSilicon/releases/tag/wine-runtime-r17) say only "Reverted MTLd3D from v0.10.0 to v0.7.0". Our launcher does not use it: Play sets `WINEDLLOVERRIDES=d3d9=n` and stages D9VK beside `SGW.exe`, so the native DLL in the game directory loads instead of the runtime's.
- `lib/external/`: MoltenVK, SDL2, FreeType, GMP, GnuTLS, Nettle/Hogweed, libidn2, libintl, p11-kit, libpng, libtasn1, libunistring.
- `libvulkan.1.dylib` is a symlink to the bundled MoltenVK. Play points `VK_DRIVER_FILES` at the runtime's `MoltenVK_icd.json`.

The archive carries no licence or notice files (a filename check on 2026-10-04 found none; [provisioning evidence](../../playtests/2026-10-03-macos-wine/runtime-provisioning.md#runtime-artifact-inspection--2026-10-04)).

## Per-component licence and recommendation

| Component | Licence (source) | May a public zip carry it? | Recommendation |
|---|---|---|---|
| Our five Windows pieces | the repository's own licence | Yes. | **Ship in the zip**, built and pinned in the same workflow run. |
| D9VK `d3d9.dll` | zlib/libpng, Philip Rebohle and Joshua Ashton ([pinned upstream notice](https://github.com/WoWSilicon/WoWSilicon/blob/5276d92627f26334f6580270eabadf22361d3717/Sources/WoWSiliconSwift/Resources/Patching/d9vk/LICENSE), kept as `shell/resources/graphics/D9VK-LICENSE.txt`) | Yes. zlib allows binary redistribution with no source offer; the notice must stay with source copies, and we keep it with the binary too. | **Ship in the zip** with `D9VK-LICENSE.txt`. CI downloads the pinned blob and checks its SHA-256. **Unverified:** which DXVK/D9VK source revision WoWSilicon built it from; zlib does not require us to know, but the release notes should say "unmodified D9VK binary from WoWSilicon v3.2.2". |
| Wine (WineAndAqua fork) | LGPL-2.1-or-later ([LICENSE at the pinned commit](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/LICENSE)) | Only with LGPL obligations: the licence text, and the corresponding source (or a written offer) for the exact build. | **Download at first run from upstream with the pinned hash** (implemented). The launcher does not convey the binary; WoWSilicon does. |
| MTLd3D v0.7.0 (inside the runtime) | zlib ([athei/mtld3d LICENSE](https://github.com/athei/mtld3d/blob/main/LICENSE); the v0.7.0 tag was not checked, **unverified**) | Yes, but we do not need to. | Comes with the runtime download; not used by Play. |
| MoltenVK (inside the runtime) | Apache-2.0 ([KhronosGroup/MoltenVK](https://github.com/KhronosGroup/MoltenVK/blob/main/LICENSE); not re-fetched for this note) | Yes, with the licence and NOTICE. | Comes with the runtime download. |
| GnuTLS, libtasn1, libintl (inside the runtime) | LGPL-2.1-or-later (upstream project licences; **unverified** for the exact builds) | Only with LGPL obligations. | Comes with the runtime download. |
| GMP, Nettle/Hogweed, libidn2, libunistring (inside the runtime) | LGPL-3.0-or-later or GPL-2.0-or-later, dual (upstream; **unverified** for these builds) | Only with LGPL/GPL obligations. | Comes with the runtime download. |
| SDL2, libpng, FreeType, p11-kit (inside the runtime) | zlib; libpng; FreeType Licence (or GPL-2.0); BSD-3-Clause (upstream; **unverified** for these builds) | Yes, with attribution. | Comes with the runtime download. |
| rosettax87 + `libRuntimeRosettax87` | MIT upstream ([Lifeisawful/rosettax87](https://github.com/Lifeisawful/rosettax87/blob/main/LICENSE), archived, pointing to a WineAndAqua fork that did not resolve on 2026-10-11; [rosettax87_jit](https://github.com/Lifeisawful/rosettax87_jit/blob/main/LICENSE)). Both hook Apple's Rosetta at run time from their own source (`rosetta_loader`, `rosetta_runtime`); they do not contain Rosetta. | Licence-wise yes, with the MIT notice. But WoWSilicon's copy ([resource folder](https://github.com/WoWSilicon/WoWSilicon/tree/v3.2.2/Sources/WoWSiliconSwift/Resources/Patching/rosettax87)) has no LICENSE file and no build record, so its provenance is **unverified**. | **Leave out of LX-28.** Also it breaks launch supervision: the Wine loader hands off to a detached `rosettax87` and exits, so Play records `unknown` after about 11 s ([project memory](../../../../.claude/agent-memory/main-session/reference_desktop_game_telemetry_2026_10_04.md)). Its loader wants `com.apple.security.cs.debugger` ([entitlements](https://github.com/Lifeisawful/rosettax87_jit/blob/main/rosetta_loader/entitlements.plist)), and an arm64 Mach-O in `Resources/` would need its own signature. A later packet can add it with a source build of our own. |
| Rosetta 2 | Apple system software | No. | **Not ours to ship.** The player installs it; the launcher probes it, as [Apple's Rosetta guidance](https://support.apple.com/en-us/102527) intends. |
| PhysX MSI, VC80, D3DX9, XInput | Microsoft and NVIDIA redistributable terms | We do not redistribute them. | **Unchanged:** taken from the player's own copy of the original client archive, as on Windows. |
| Apple D3DMetal / GPTK | Apple licence | n/a | **Not used.** Not assessed. |

### The Wine runtime: why fetch, and the risk

Fetching keeps us out of the LGPL distributor role: no source mirror, no written offer, no notice bundle for 1,850 files. The download is already pinned three ways (exact size, archive SHA-256, full tree digest) and runs before any install work, so a tampered or replaced asset fails closed.

The risk is availability. If WoWSilicon deletes or replaces the `wine-runtime-r17` asset, every new Mac install fails at the runtime step; existing caches keep working. GitHub release assets can be deleted by their owner at any time. Two fallbacks, neither needed for LX-28:

- **Mirror** the identical archive as an asset on a Cimmeria release, keep the same hashes, and add the mirror URL as a second source. This makes us a distributor: the mirror release must carry the LGPL texts and the corresponding source (the WineAndAqua commit and the `lib/external` library sources), or a written offer, and the Apache-2.0 NOTICE for MoltenVK.
- **Build our own runtime** from the same lock with `assemble.sh`. Same obligations, plus a build pipeline.

Recommend recording the dependency in `docs/prerequisites.md` and the player guide ("the first install downloads the Wine runtime, 64 MB, from the WoWSilicon project on GitHub") and opening a low-priority issue for the mirror.

## Gatekeeper and ad-hoc signing

**Finding:** an ad-hoc signed, quarantined app downloaded from GitHub can be opened on macOS 15 after one trip to System Settings > Privacy & Security > Open Anyway and a login password; after that the app is an exception and opens normally. Behaviour on macOS 26 and 27 for ad-hoc apps specifically is **unverified** and is what acceptance step 13 proves.

Evidence:

- Apple's current guide: open System Settings > Privacy & Security, click Open Anyway, which "is available for about an hour after you try to open the app", enter the login password, and the app "is saved as an exception" ([Apple: open a Mac app from an unknown developer](https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac)).
- macOS 15 removed the Control-click Open override, so Open Anyway is the only path in the UI ([AppleInsider](https://appleinsider.com/inside/macos-sequoia/tips/whats-changed-in-runtime-protection-for-macos-sequoia), [Cult of Mac](https://www.cultofmac.com/864843/macos-sequoia-removes-handy-shortcut-bypass-gatekeeper-security/), [troz.net test of an unsigned app on Sequoia](https://troz.net/post/2024/sequoia_app_permissions/)).
- Apple silicon will not run native arm64 code without a signature; an ad-hoc signature is enough, and translated x86-64 code may run with none ([Apple Platform Security](https://support.apple.com/guide/security/secebb113be1/web), [Eclectic Light](https://eclecticlight.co/2020/08/22/apple-silicon-macs-will-require-signed-code/)).
- Tauri's guide says ad-hoc signing (`signingIdentity: "-"`) is for Apple silicon apps from the internet and that it "does not prevent MacOS from requiring users to whitelist the installation in their Privacy & Security settings" ([Tauri: macOS code signing](https://v2.tauri.app/distribute/sign/macos/)).
- An app with no valid signature, or one modified after signing, is reported as "damaged" and Open Anyway is not always offered ([macheim PR #11](https://github.com/lofcgi/macheim/pull/11)). So the CI job must verify the seal after packaging, and the zip must be made with `ditto`, which keeps the bundle intact.
- Tahoe 26.2 and later are reported to enforce Gatekeeper more strictly ([macos-tahoe.com](https://macos-tahoe.com/blog/app-is-damaged-cant-be-opened-macos-tahoe-gatekeeper-quarantine-fix-2026/)); this is a secondary source, so treat it as a reason to run step 13 on the newest macOS available.

**Translocation.** A quarantined app opened from the folder it was downloaded into runs from a random read-only path, until the player moves it in Finder ([Eclectic Light: what causes App Translocation](https://eclecticlight.co/2023/05/09/what-causes-app-translocation)). The launcher keeps its state in Tauri's app data directory (`app_data_dir()`, under `~/Library/Application Support/`), so Install and Play should work from a translocated copy, but that is **unverified**: the bundle path changes on every launch, and nobody has checked whether a saved plan records it. The self-updater's bundle exchange would fail on the read-only path. The guide should say: unzip, drag "Stargate Worlds Launcher" to Applications in Finder, then open it.

**Nested code.** Only one file in the bundle is Mach-O: the launcher binary in `Contents/MacOS/`. The Windows PEs and `d3d9.dll` in `Contents/Resources/` are data to the code signature: the bundle seal covers them, and the launcher rechecks each against its compiled SHA-256. They need no signatures of their own. `codesign --force --deep -s -` therefore signs one binary and seals the rest; `--deep` is harmless here.

**The Wine runtime needs no signature from us.** It is not in the bundle. Our process downloads it with `reqwest` and extracts it with `/usr/bin/tar`; a non-sandboxed app's files get no quarantine attribute unless its Info.plist sets `LSFileQuarantineEnabled` ([Apple](https://developer.apple.com/documentation/bundleresources/information-property-list/lsfilequarantineenabled); the page body did not render for this check, so the default is **unverified** from that source), so Gatekeeper never assesses the runtime. Its Mach-O files are x86-64 and run under Rosetta, which needs no signature. This is the configuration that already runs on a maintainer's Mac (macOS 26.6.1). The step 13 UAT should still run `xattr -l` on the runtime cache once to confirm no quarantine attribute.

**Hardened runtime stays off.** It is required only for notarization, which D-LX14 leaves out. With it off, the launcher may spawn Wine and Wine may map unsigned code without entitlements.

**Privacy prompts after updates.** An ad-hoc signature has no team identity, so macOS ties privacy grants to the exact binary. A rebuilt app asks again for folder access (already seen in native UAT: [project memory](../../../../.claude/agent-memory/main-session/reference_launcher_game_update_review.md)). Players will see the Documents or Downloads prompt again after each new version; the guide should say so.

## CI job design

### Runner

`runs-on: macos-15` (arm64). The [runner images list](https://github.com/actions/runner-images) on 2026-10-11 marks both macOS 14 images deprecated, `macos-15` is arm64 and current, and `macos-latest` is macOS 26 and moves without notice. LX-28's text says `macos-14`; change it to `macos-15` in the PR and in `work-packets.md`. The existing `launcher-desktop.yml` matrix uses `macos-latest` for tests, which is fine there.

### Windows pieces: from the Windows job, as an artifact

The Mac app needs **five** Windows pieces, the Windows zip only three. `tools/launcher-release/build-desktop.sh helpers` builds the launch worker and the two DLLs. Add a `mac-helpers` stage (or extend `helpers` behind a flag) that also builds:

- `cargo build --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --bin cimmeria-archive-worker --release --target-dir target/desktop` (x86-64, the host target on the Windows runner)
- `cargo build --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-runtime-probe --bin cimmeria-prerequisite-worker --release --target i686-pc-windows-msvc --target-dir target/desktop`

In `desktop-windows` (as LX-15 leaves it), after `verify`:

1. Run `mac-helpers`.
2. Write `mac-pieces.sha256` (one `<sha256>  <file>` line per piece) and publish it as a job output, so the trusted digests reach the Mac job by a channel separate from the files.
3. `actions/upload-artifact` the five files plus `mac-pieces.sha256` as `desktop-mac-windows-pieces-<tag>`, retention 1 day.

A separate `desktop-mac-pieces` Windows job is cleaner if `desktop-windows` gets long, but it rebuilds the same three pieces; one job with one more stage is simpler.

### The `desktop-macos` job

`needs: [release-tag job, desktop-windows]`; `if: needs.desktop-windows.result == 'success'`; same checkout ref as `desktop-windows`.

| Step | Command |
|---|---|
| Toolchain | `./.github/actions/rust-toolchain` (aarch64-apple-darwin is the host), `actions/setup-node@v6` with Node 24 |
| Tauri CLI | `cargo install tauri-cli --version <exact 2.x> --locked`, pinned to match the `tauri` crate in `crates/launcher/desktop/Cargo.lock` (2.12.1 today). [updater.md](../../../../crates/launcher/desktop/docs/updater.md) already requires a pinned CLI |
| Fetch Windows pieces | `actions/download-artifact` into `mac-pieces/`; compare `shasum -a 256` of each file with the `desktop-windows` job output, fail on any difference |
| Fetch D9VK | `curl -fsSL -o crates/launcher/desktop/shell/resources/graphics/d3d9.dll https://raw.githubusercontent.com/WoWSilicon/WoWSilicon/5276d92627f26334f6580270eabadf22361d3717/Sources/WoWSiliconSwift/Resources/Patching/d9vk/d3d9.dll`, then require size 3,399,680 and SHA-256 `033cef34aafc409075c764684cd97751903f5850b4f8b9d9c09eb5b6e882045e` (pinned in the workflow, as in [graphics/README.md](../../../../crates/launcher/desktop/shell/resources/graphics/README.md)) |
| Stage | `python3 crates/launcher/desktop/tools/stage-helper.py mac-pieces/<file> --kind <archive\|prerequisite\|launch\|client-patches\|client-telemetry> --sha256 <digest> --revision "$GITHUB_SHA"` for each piece; export the variable each run prints. Export `CIMMERIA_D3D9_SHA256`. Leave both rosettax87 variables unset |
| Frontend | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend` and `npm run build --prefix …` |
| Build | from `crates/launcher/desktop/shell`: `CARGO_TARGET_DIR=$GITHUB_WORKSPACE/target/desktop cargo tauri build --bundles app -- --locked` (release profile, not `--debug`). The build step alone gets `LAUNCHER_MANIFEST_PUBKEY_HEX`, `CIMMERIA_LAUNCHER_TAG`, `CIMMERIA_LAUNCHER_BUILD_EPOCH`, `CIMMERIA_LAUNCHER_VERSION`, `CIMMERIA_BUILD_BRANCH`, `CIMMERIA_BUILD_GIT_SHA`, `CIMMERIA_DESKTOP_SOURCE_REVISION` and `LAUNCHER_LOG_SAS_URL`, exactly as the Windows build; plus `LAUNCHER_UPDATER_PUBKEY` only once the Mac updater is enabled |
| Sign | `codesign --force --deep --sign - "target/desktop/release/bundle/macos/Stargate Worlds Launcher.app"`; no `--options runtime`, no entitlements |
| Verify | `codesign --verify --deep --strict --verbose=2 <app>`; `codesign -dv <app>` shows `Signature=adhoc` and no `runtime` flag; `lipo -archs Contents/MacOS/<exe>` is `arm64`; each file in `Contents/Resources/windows/` and `graphics/d3d9.dll` matches its staged digest, and each digest appears in the executable (the Windows `verify` stage's checks); the telemetry DLL lacks the lab-bridge marker; `Info.plist` `CFBundleShortVersionString` equals `CIMMERIA_LAUNCHER_VERSION` (needs LX-17) |
| Package | `ditto -c -k --sequesterRsrc --keepParent "<app>" artifacts/StargateWorlds-Launcher-macos-arm64.zip`; then unzip to a temporary folder and repeat `codesign --verify --deep --strict` on the copy; write `StargateWorlds-Launcher-macos-arm64.zip.sha256` in the Windows `.sha256` format |
| Upload | `actions/upload-artifact` (30 days); `gh release upload "$TAG"` the zip and `.sha256` to the dated release; replace the same two assets on `launcher-current` the way LX-15 does for Windows; append one release-notes line: arm64 only, ad-hoc signed, Open Anyway once, drag to Applications first, updates are manual |

The zip is an extra asset on a release the Windows job already made, as the desktop job is today: if the Mac job fails, the dated release and the Windows assets stand.

### Dry run on pull requests

Add the same job to `launcher-desktop.yml` as `desktop release build dry run (macOS)`, on PRs that touch the desktop launcher. It needs the five Windows pieces too, so the Windows dry-run job runs `mac-helpers` and uploads the artifact, and the Mac dry run builds without a tag, manifest key or upload, then runs every Verify and Package check.

### Secrets

The Mac job needs no new secrets. It uses the same `LAUNCHER_MANIFEST_PUBKEY_HEX` and `LAUNCHER_LOG_SAS_URL` as Windows, and later the same Minisign key (D-LX9) to sign the updater archive. No Apple account, certificate or keychain.

## Self-update on macOS

The LX-13 zip swap is Windows only. macOS has a different, older apply path in the engine, and the shell turns it off.

- **Engine: implemented.** `engine/src/storage/updater/apply.rs` has a `Target::Mac` that takes the running bundle from `current_exe()`, extracts a gzip tar with exactly one `.app` root (`bundle.rs`), checks the new bundle's identifier, executable and `CFBundleShortVersionString` against the installed bundle and the offer, swaps the bundles atomically with `renamex_np(RENAME_SWAP)` (`apply/process.rs`), keeps the old one as a sibling backup and relaunches with `--launcher-update-restart <pid>`. Fixture tests cover the exchange, rollback and recovery on macOS.
- **Policy: accepted.** `Config::new` already accepts `darwin-aarch64`.
- **Shell: disabled.** `shell/src/host/updater/mod.rs` passes `cfg!(windows).then_some("windows-x86_64")`, so every Mac build reports Disabled.

A `darwin-aarch64` feed entry can work. It needs:

1. The shell change: platform `darwin-aarch64` under `cfg!(all(target_os = "macos", target_arch = "aarch64"))`, and `LAUNCHER_UPDATER_PUBKEY` on the Mac build step.
2. A second Mac asset, `StargateWorlds-Launcher-macos-arm64.app.tar.gz`, made with `COPYFILE_DISABLE=1 tar --no-mac-metadata -czf … -C <bundle dir> "Stargate Worlds Launcher.app"`. Without those switches macOS tar adds `._` AppleDouble members, and `._Stargate Worlds Launcher.app` is a second root that `bundle::extract` refuses. Sign it with Minisign, trusted comment `version:<YYYY.M.D>`, as LX-15 signs the Windows zip.
3. The feed must list both platforms. If each platform job writes `latest.json`, the later one overwrites the earlier. Move feed writing into a small `feed` job that `needs` both platform jobs, runs when the Windows job succeeded, and adds `darwin-aarch64` only when the Mac job succeeded.
4. `CFBundleShortVersionString` must equal the feed version (D-LX8, via LX-17).
5. The installed app must not be translocated and its folder must be writable by the player: `/Applications` is, for an admin account.
6. A native Mac UAT of a real packaged update N-1 to N, like acceptance step 11. Nobody has run the Mac apply on a packaged app.

Recommendation: LX-28 publishes the Mac zip with updates **manual**, and `crates/launcher/desktop/docs/release.md` and the guide say so. Items 1 to 6 go in a follow-up packet ("LX-28b: macOS self-update"), so the first Mac release does not wait on a Mac update UAT.

## Implementation and deviations from this design

The implementation PR follows the design above, with the stages in `tools/launcher-release/build-desktop-macos.sh` and the two Windows-side stages (`mac-helpers`, `mac-pieces`) in `build-desktop.sh`. It deviates in six places:

1. **Publishing moved to its own job.** The design's Upload step ran `gh release upload` in `desktop-macos`. That job runs npm and a cargo-installed Tauri CLI, and LX-15 keeps every job that runs build code on a read-only token. So `desktop-macos` uploads an artifact only, and a `publish-macos` job (ubuntu, `contents: write`, no build, no third-party action) checks the zip against `desktop-macos`' digest output and attaches it.
2. **One digest for the pieces.** `desktop-windows` outputs the SHA-256 of `mac-pieces.sha256`, not the list itself. The Mac job checks the list against that one value, then each piece against the list through `stage-helper.py`. Same trust chain, one output.
3. **The Mac steps in `desktop-windows` are `continue-on-error`.** A failure there fails `desktop-macos` (empty output), never the Windows release. The PR dry run has no `continue-on-error`, so a broken Mac stage still fails the PR.
4. **`launcher-current` gets the Mac zip only when `publish` refreshed it** (a signed, first-of-the-day release), the same rule as the Windows zip, so both stable links name the same release. When the Mac build fails, `launcher-current` keeps the previous Mac zip.
5. **`publish-macos` waits for `egui-macos-wine`**, whatever its result: both rewrite the dated release's notes, and parallel read-modify-write edits could drop a paragraph.
6. **The Info.plist version check accepts the base version for a same-day build.** The executable must carry the full `YYYY.M.D-N`; Info.plist's `CFBundleShortVersionString` may be that or `YYYY.M.D`, because whether the bundler keeps a SemVer prerelease there was not checked. The dry run stamps a plain `YYYY.M.D`, so it proves the exact match only for that form.

What only a real release proves: the Tauri CLI 2.12.1 build on `macos-15` with the production manifest key; that the D9VK raw URL still serves the pinned bytes (the dry run checks this on every PR); the `publish-macos` uploads and notes edit; and everything in acceptance step 13.

## Docs the LX-28 implementation PR touches

- `crates/launcher/desktop/docs/release.md`: replace "macOS is deferred" with the Mac job, the asset names, ad-hoc signing and manual updates.
- `.github/workflows/launcher-release.yml`: replace the "Deferred: the desktop launcher for macOS" comment block; items 1 to 3 there (Developer ID, notarization, hardened runtime) become "out of scope by D-LX14".
- `docs/analysis/launcher-consolidation/work-packets.md` LX-28: runner `macos-15`; the Mac needs five Windows pieces; updater follow-up.
- `docs/analysis/launcher-consolidation/acceptance.md` step 13: add "drag to Applications in Finder first" and "check `xattr -l` on the runtime cache".
- LX-22 player docs: `docs/guides/macos.md` gets Rosetta, drag to Applications, Open Anyway once, the 64 MB runtime download from WoWSilicon, the folder-access prompt after updates, and manual updates.
- [runtime-provisioning.md](../../playtests/2026-10-03-macos-wine/runtime-provisioning.md) "Prerequisites and distribution gates": link this note as the distribution decision.
