# Linux / Steam Deck launcher: feasibility assessment

> **Type:** Explanation (spike brief) · **Audience:** Engineers running a Steam Deck spike · **Updated:** 2026-10-24
> **Companions:** [macOS Wine runtime provisioning](../playtests/2026-10-03-macos-wine/runtime-provisioning.md), [launcher platform options](../playtests/2026-10-03-macos-wine/launcher-platform-options.md), [Wine validation](../../../crates/launcher/desktop/docs/wine-validation.md), [launch](../../../crates/launcher/desktop/docs/launch.md)

This is a **spike brief** for a Steam Deck session (`claude --agent hands-on`). It is not a campaign: there is no ledger, no packets and no commitment to ship.

**Evidence status.** Everything below comes from reading code and docs. Nothing was compiled, run, or tried on Linux, SteamOS or Proton. Every claim about runtime behavior under Proton is untested. Paths below are relative to `crates/launcher/desktop` (written `D/`) unless they start with `docs/`, `crates/` or `tools/`.

## What exists today

- **The engine builds and tests on Linux.** [`D/docs/launcher-summaries.md`](../../../crates/launcher/desktop/docs/launcher-summaries.md) (lines 420-436) says the engine tests run locally on Linux through the lane. `SummaryOs` already has a `Linux` variant (`D/engine/src/storage/launcher_summary/schema.rs`).
- **The Tauri shell has never been built on Linux.** `launcher-summaries.md:433` says the shell does not build on the Linux development host (no GTK or WebKit), so shell code is first compiled on Windows and macOS.
- **macOS code is cfg-gated.** `mac_runtime` and `mac_wine` compile only under `cfg(target_os = "macos")` (`D/engine/src/lib.rs:54-58`). A Linux backend would be a sibling module, not a change to these.

## The macOS pattern to reuse

macOS spawns a pinned Wine loader that runs the same i686 `cimmeria-launch-worker.exe`, passing a `Z:\` guest path. The worker creates `SGW.exe` suspended, injects the DLLs, then resumes it (`D/engine/src/storage/launch/wine.rs:34-58`, `D/engine/src/mac_wine/paths.rs`). The request lists the client patches DLL first, then the telemetry DLL.

- The macOS-only environment (MoltenVK ICD, Rosetta x87, `DYLD_*`, `CX_FWD_COMPAT_GL_CTX`) is built at `wine.rs:66-96`. It would drop out on Linux. Keep `WINEDLLOVERRIDES=d3d9=n` and `DXVK_FRAME_RATE`.
- DXVK's d3d9 would come from the Proton runtime instead of a pinned bundle.
- CI shape: `.github/workflows/launcher-desktop-macos.yml` and `tools/launcher-release/build-desktop-macos.sh` consume Windows-built pieces (the worker and DLLs) as an artifact. A Linux job could follow the same shape.

## Blocking sites

| Site | What blocks Linux |
|---|---|
| `D/engine/src/storage/launch/mod.rs:188-200` | `Native` backend requires `cfg!(windows)`; `Wine` requires `cfg!(target_os = "macos")` |
| `D/engine/src/storage/launch/worker.rs:317-353` | `Ownership::Wine` and `wine::prepare` are macOS-only; other hosts return `Corrupt` |
| `D/engine/src/storage/install_worker/wine/` and `install_worker/mod.rs` | Wine extraction backend path |
| `D/engine/src/storage/launch/supervisor.rs:60`, `D/engine/src/helper_supervisor/mod.rs:107` | `creation_flags(0x08000000)` is `cfg(windows)`; Linux needs no flag but needs a process-lifetime check |
| `D/engine/src/storage/runtime_setup/windows_physx/` vs `D/engine/src/mac_wine/prerequisites/` | PhysX 2.6.3 is found via registry `HKLM\Software\Ageia Technologies` (`D/docs/physx-probe.md:24`); the macOS prefix installer does this inside Wine, and Linux needs the same |
| `D/shell/src/webview2.rs`, called at `D/shell/src/main.rs:542` | WebView2 check (already `cfg(windows)`; Linux needs webkit2gtk instead) |
| `D/engine/src/archive_worker/mod.rs:170` | `cfg(windows)` for CAB/RAR handling |
| `D/engine/src/storage/updater/apply.rs:50-82` | Only macOS and Windows targets; every other OS returns `Error::Platform` |
| `D/engine/src/storage/updater/policy.rs:41` | Accepted platforms are `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`; no `linux-x86_64` |
| `D/engine/src/storage/updater/release_check.rs:19` | Only `WINDOWS_PLATFORM` is defined |
| `D/engine/src/storage/adoption/` | `cfg(unix)` sites (for example `direct_path_ops.rs`) were written for macOS, including `RENAME_SWAP`; whether they behave on Linux filesystems is unverified |

I did not re-check `apply.rs:188,197` individually; treat them as part of the same updater gap.

## Existing docs and code

- `docs/analysis/playtests/2026-10-03-macos-wine/` (`runtime-provisioning.md`, `launcher-platform-options.md`).
- `D/docs/wine-validation.md`, `D/docs/launch.md`.
- `crates/client-patches/src/strstream_underflow/` fixes `msvcp80` `strstreambuf::underflow` under Wine. Expect it to matter on Proton too.
- Searching the launcher docs found **no** mention of Linux, Proton or Steam Deck.

## Tauri on Linux

`bundle.targets` is `"all"` in `D/shell/tauri.conf.json`. A Linux build needs `webkit2gtk-4.1` and GTK3 development packages. CI has no Linux build job today: the `ubuntu-latest` job in `launcher-desktop.yml` only runs shellcheck on release scripts.

## Estimate

- **(a) Patch, update, configure and start the game through Wine or Proton: moderate**, roughly the size of the macOS slice. Compile the shell on Linux. Add a Linux Wine backend (runtime discovery, prefix, loader, environment), reusing `wine.rs`. Open the launch and install gates. Take DXVK from the runtime. Generalize prerequisite installs.
- **(b) Full parity: larger.** Add a `linux-x86_64` updater key and an apply path, packaging and signing, a webkit2gtk CI job, and Deck handling for gamescope and the controller.

## Hard parts and unknowns

- **DLL injection under Proton.** It works under the macOS Wine; unverified on Proton. Wineserver and container sandboxing may differ.
- **Prefix choice.** Steam's `compatdata` prefix versus the private prefix with explicit `Z:` mapping that the macOS code assumes.
- **Wine version differences.** `msvcp80` behavior and manifests may differ between Proton and the pinned macOS Wine.
- **Process-lifetime tracking.** When the Wine loader execs elsewhere, tracking broke on macOS (with rosettax87). Proton's launcher scripts may do the same.

## Spike checklist (Steam Deck)

Needs your own client copy, Desktop Mode, and awareness that the SteamOS root is read-only (install tools under `$HOME`, use Flatpak or a container). The repo's PowerShell-only rules assume Windows; this spike is exempt, and it should record every command it ran.

1. Can Wine or Proton start `SGW.exe` against a Cimmeria server?
2. Can the existing i686 `cimmeria-launch-worker.exe` start it suspended, inject `cimmeria_client_patches.dll` and the telemetry DLL, and resume it under Proton?
3. Which prefix works: Steam `compatdata` or a private one?
4. Which PhysX and MSVC runtime installs does the prefix need?

Record the results as a worknote beside this file, marking each item tested or untested.
