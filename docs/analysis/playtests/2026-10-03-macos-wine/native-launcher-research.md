# Native Mac launcher: WoWSilicon integration research

> **Type:** Reference (historical research)
> **Audience:** Launcher contributors researching design provenance
> **Last updated:** 2026-10-04 (historical notice; research dated 2026-10-03)
> **Companions:** [Selected Tauri implementation plan](launcher-implementation-plan.md), [current checkpoint](worknotes/macbook-testing-checkpoint.md)

> **Historical, superseded design proposal:** SwiftUI was evaluated, not selected.
> The implementation uses Tauri with Rust and Effect; follow the linked current
> plan and acceptance checkpoint. Upstream findings below retain their original
> version/date scope and have not been refreshed for this archival checkpoint.

Date: 2026-10-03. Read-only source research against WoWSilicon **v3.2.2**, plus the current Cimmeria checkout. No native launcher, runtime adapter, package, or gameplay was built or tested for this note. The SwiftUI design below is proposed; it requires an explicit exception to Cimmeria's current Windows-only build policy and a native macOS build lane. Existing Windows components remain Windows-native builds through `tools/build-lane/lane.sh`.

## Verified upstream contracts and limits

| Area | Source finding | Design consequence |
| --- | --- | --- |
| Supported product surface | [README](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/README.md) explicitly supports other 32-bit D3D9 games via Non-WoW profiles and bundles customized Wine, RosettaX87, and DX9 translation. | Reuse this tested runtime family, but do not assume SGW compatibility from general D3D9 support. |
| External launch | [LaunchService.swift](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift#L274-L324) exports a shell script carrying launch setup and background log redirection. [ShortcutExportService.swift](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/ShortcutExportService.swift) packages it as an Apple Shortcut with a shell-script action and invokes `shortcuts sign`. | User-facing shortcut export exists. It is a snapshot of paths/settings, not a versioned profile-launch RPC or completion protocol. Apple Shortcut signing is separate from app notarization. |
| Automation API | No launcher CLI, URL-launch scheme, or public profile import API is documented in the README; inspected [App entry](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/WoWSiliconSwiftApp.swift) and [Info.plist](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/Info.plist) expose no such handler. [VersionStore.swift](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Stores/VersionStore.swift) owns `versions.json` persistence. | Treat file manipulation and calls into internal Swift services as unsupported integration, not a stable upstream API. Prefer a pinned, independently tested adapter; seek an upstream launch/export contract later. This is a bounded source inspection, not a claim that every upstream file was searched. |
| Runtime environment | [BundledWineRuntime.swift](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/BundledWineRuntime.swift) resolves bundled `Wine/bin/wine` and `wineserver`, uses `Wine/lib/external` in `DYLD_LIBRARY_PATH`, supplies `WINEPREFIX`, resolves Vulkan manifests, and removes inherited x87 keys. `WOWSILICON_WINE_RUNTIME` is an implementation override. | Calling the `wine` executable alone does not reproduce WoWSilicon's environment. Keep runtime version, resources, prefix, renderer, working directory, and environment together in an immutable launch plan. |
| x87 setup | [BundledRosettaRuntime.swift](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/BundledRosettaRuntime.swift) validates `rosettax87` plus adjacent executable `libRuntimeRosettax87`; game launch explicitly sets `ROSETTA_X87_PATH` (or `X87_SIDECAR_PATH`) in [LaunchService](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift#L459-L487). | Preserve both resources and the explicit game launch environment. Presence of Rosetta 2 alone is not proof that this accelerated x87 path is active. |
| Third-party launcher trap | [Third-party launch](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift#L623-L718) uses builtin-fallback DLL overrides, adds Chromium-style flags, and does not set the x87 environment that normal game launch sets. `makeEnvironment` strips inherited x87 keys. | Do not blindly reuse this path for SGW injection. Whether the helper-spawned SGW child receives and activates the right runtime must be measured. The source difference is verified; an SGW performance or child-process failure is not yet observed. |
| Lifecycle | Integrated launch tracks the spawned shell process; exported shortcuts background the command. `forceQuitWine` invokes `wineserver -k`, then falls back to scanning/killing Wine processes ([LaunchService](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift#L887-L910)). | Neither shortcut completion nor helper exit proves game exit. Own a separate Cimmeria prefix and session tracking; avoid a global force-quit fallback that could affect another game. |

The known process-local `CX_FWD_COMPAT_GL_CTX=1` workaround remains necessary when showing today's Windows egui launcher. A native SwiftUI shell removes that GUI's WGL requirement; it does **not** prove game D3D9 compatibility or fix missing Windows runtimes.

## Design C: SwiftUI shell with shared install core

Optimize the default screen for **Inspect → Prepare → Play**, exposing runtime internals only in diagnostics/settings:

1. **Inspect** returns a structured report: selected install, signed manifest status, disk budget, install/patch state, Wine/runtime pin, Rosetta/x87 assets, legacy prerequisites, and whether another session owns the prefix. This phase does not mutate the install.
2. **Prepare** executes a reviewable, resumable plan with per-step progress, cancellation boundaries, atomic state writes, and a journal. Download/hash verification, manifest policy, install layout, patch selection, and repair semantics belong to a shared Rust core consumed by Windows and Mac UIs.
3. **Play** applies the same launcher setup and injection contract as Windows, then reports preparing/starting/running/exited based on game-session evidence. Errors become an actionable repair step with exportable diagnostics. Native SwiftUI appearance follows macOS; appearance choice is independent of Wine registry preferences.

Use a narrow versioned C ABI (or an out-of-process structured protocol if isolation is preferred) between SwiftUI and the shared core: `inspect`, `plan_prepare`, `execute`, `cancel`, `launch`, event subscription. Put Wine behind a `RuntimeProvider` interface that produces a complete executable/arguments/environment/working-directory/session specification. Do not expose `versions.json` as a Cimmeria storage model or duplicate installer policy in Swift.

Retain Windows helper operations for APIs that are currently Windows-specific. `crates/launcher/src/unpack/cab_set.rs` dispatches multipart CAB expansion to Windows FDI; its non-Windows path rejects it. `crates/launcher/src/unpack/fdi.rs` owns continuation handling via `cabinet.dll`. `crates/client-launch/src/launch.rs` documents suspended process creation, injection, and resume; its non-Windows implementation is a stub. Therefore propose a signed/versioned helper protocol for FDI expansion and a Windows-native injection helper run under Wine. Do not reimplement CAB continuation or bypass `sgw-start32` just to make a native UI compile. Helper paths and Windows/macOS path translation require explicit fixtures and traversal validation.

For a spike, allow discovery of an existing WoWSilicon v3.2.2 install with an isolated Cimmeria prefix and a narrowly pinned adapter. For a white-glove release, choose and own a versioned runtime distribution rather than silently updating or editing another app's profile. Neither runtime-distribution option is currently validated.

## Distribution and licensing findings

WoWSilicon's root [LICENSE](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/LICENSE) is GPLv3. Its [runtime lock](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/runtime-lock.json) lists multiple external libraries and renderer resources. This is an inventory finding, not a determination that copying its Swift services or shipping all runtime components under Cimmeria's current terms is permissible. A release decision needs component-by-component license/source/notices review, including the origin and redistribution terms of `rosettax87` and `libRuntimeRosettax87`; the root license alone does not establish those binaries' provenance.

Upstream's [Makefile](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Makefile#L71-L108) optionally codesigns the app when an identity is supplied; the README documents an unsigned-app quarantine workaround. The inspected release path does not establish a notarized, hardened-runtime-compatible package. A signed Sparkle feed or Apple Shortcut is not that evidence.

Apple requires Developer ID signing and hardened runtime for notarization; JIT and dynamic-library loading can require scoped runtime exceptions. The precise exceptions needed by Wine/x87 have **not** been determined here. Test the full signed distribution, not only development launches, and verify clean-machine Gatekeeper launch with the game child. Sources: [Apple distribution guide](https://help.apple.com/xcode/mac/current/en.lproj/dev033e997ca.html), [Hardened Runtime](https://developer.apple.com/documentation/security/hardened-runtime), [notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

## Release gates and principal risks

- **Highest technical risk:** helper → suspended SGW → injected DLLs → resumed child under Wine, with accelerated x87 and the chosen D3D9 backend actually active. Establish a tiny end-to-end spike before extracting the full installer core.
- **Highest integration risk:** no verified stable WoWSilicon automation API; the third-party path differs materially from game launch. Version-pin and fixture-test the adapter, and fail clearly on unsupported runtimes.
- **Highest distribution risk:** unsigned runtime behavior does not predict hardened/notarized behavior, and bundled binary provenance needs review before promising a one-download experience.
- **Install correctness risk:** FDI continuation, DOS timestamps, path translation/case, prerequisites, launcher setup and patch injection must preserve Windows semantics. Add contract and fixture tests at the shared-core/helper boundary.
- **Human gate:** a fresh Mac completes Prepare, reaches login, enters the world, opens Black Market, and retains usable input/audio/performance after relaunch. Test signed production packaging, dark/light appearance, cancellation/resume, process lifetime, and no interference with another Wine session. No such UAT was performed for this design.
