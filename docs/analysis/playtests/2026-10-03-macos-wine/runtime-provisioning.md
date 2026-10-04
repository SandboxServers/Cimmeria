# Mac runtime provisioning evidence

> **Type:** explanation
> **Audience:** launcher contributors and release maintainers
> **Last updated:** 2026-10-04
> **Companions:** [implementation plan](launcher-implementation-plan.md), [desktop engine](../../../../crates/launcher/desktop/README.md)
> **Scope:** implementation inputs, not SGW compatibility or distribution clearance.

Source review dated 2026-10-04. WoWSilicon references are pinned to v3.2.2
unless stated otherwise; Apple guidance and the rosettax87 upstream license
reference are not version-pinned.

## Runtime inventory

WoWSilicon v3.2.2 resolves to commit
`5276d92627f26334f6580270eabadf22361d3717`. Its pinned runtime is revision 17,
Wine 11.13 at WineAndAqua commit `37540b5d94ac1c86e2599ef55d7f3a15e3237ce8`.
The `wine-runtime-r17` release asset `WoWSilicon-WineRuntime-r17.tar.xz` is
63,744,856 bytes with SHA-256
`dc67cf0c2dd1e4c1cfaffe924f4737aaa594645b135a7973ac3505c83c70f882`.
Sources: [runtime lock](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/runtime-lock.json),
[artifact lock](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/artifact-lock.json),
[release](https://github.com/WoWSilicon/WoWSilicon/releases/tag/wine-runtime-r17).

That archive is not the entire app payload. D3D9 replacements and x87 accelerators
are separate patch resources. The rosettax87 executable needs its adjacent
`libRuntimeRosettax87` companion. x87sidecar uses a different environment selector.
A pinned Wine archive alone is insufficient for a reproducible SGW runtime.
Sources: [assembly](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/tools/wine-runtime/assemble.sh),
[patch resources](https://github.com/WoWSilicon/WoWSilicon/tree/v3.2.2/Sources/WoWSiliconSwift/Resources/Patching),
[x87 resolution](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/BundledRosettaRuntime.swift).

Implementation recommendation: an immutable, hash-identified runtime directory,
a separately writable prefix exclusive to this game installation, and a complete
inventory of runtime/patch resources. Do not modify the installed WoWSilicon app
or depend on its private profile JSON.

## Candidate launch contract

Use a native process API with a fixed executable, argument vector, working
directory and allowlisted environment. The candidate Wine/helper command uses
`<runtime>/bin/wine`, a Windows helper path, and `<game>/Working/Binaries` as cwd.
Selected DXVK/MoltenVK settings from upstream direct game launch:

| Variable | Candidate value |
|---|---|
| `WINEPREFIX` | Cimmeria-owned exclusive prefix |
| `DYLD_LIBRARY_PATH` | runtime `lib/external` |
| `WINEDLLOVERRIDES` | `d3d9=n` |
| `WINE_LARGE_ADDRESS_AWARE` | `1` |
| `VK_DRIVER_FILES` | runtime MoltenVK ICD manifest, if present |
| `MVK_CONFIG_SYNCHRONOUS_QUEUE_SUBMITS` | `1` |
| `DXVK_ASYNC` | `1` |
| `ROSETTA_X87_PATH` | pinned rosettax87 executable, only when selected |

This is a candidate to validate, not a verified SGW recipe. These variables
describe selected upstream behavior; helper and guest propagation must be tested
before adopting this environment. Upstream's
third-party-launcher path differs: it uses `d3d9=n,b`, adds Chromium flags and
does not explicitly set the selected x87 variable. A working Windows launcher
and saved profile setting therefore do not prove acceleration reaches SGW.
Do not copy those Chromium flags into the game/helper invocation.
Sources: [runtime environment](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/BundledWineRuntime.swift),
[launch paths](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift).

Generic D3D9 patching copies the selected `d3d9.dll` beside the actual game
executable. It does not apply WoW-specific modifications. For SGW the directory
is `Working/Binaries`. Retain `CX_FWD_COMPAT_GL_CTX=1` for the existing egui
fallback ([local bring-up evidence](README.md)); a console helper has no demonstrated need for that window workaround.
Source: [patch service](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/PatchService.swift).

## Prerequisites and distribution gates

Upstream probes Rosetta with `arch -x86_64 /usr/bin/true` and demonstrates Wine
redistributable invocation with `/install /quiet /norestart`. Its modern VC++ DLL
checks do not establish SGW's VC++2005, PhysX2.x, D3DX9 or XInput prerequisites.
Each needs a pinned package, terms review, silent-install contract and success
probe. Installed Mono does not establish that SGW requires it.
Source: [dependency service](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/DependencyService.swift).

Rosetta is system software. Present a user-controlled installation step and
re-probe afterward; do not silently accept terms or copy system Rosetta into a
bundle. Future macOS support must be rechecked against Apple's current policy.
Source: [Apple Rosetta guidance](https://support.apple.com/en-us/102527).

Verified license sources include WoWSilicon app source
[GPLv3](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/LICENSE), the pinned Wine fork
[LGPL2.1-or-later](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/LICENSE),
D9VK [zlib/libpng](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Resources/Patching/d9vk/LICENSE),
x87sidecar [MIT](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Resources/Patching/x87sidecar/LICENSE),
and rosettax87 upstream [MIT](https://github.com/Lifeisawful/rosettax87_jit/blob/main/LICENSE).
The rosettax87 `main` citation is unpinned and does not establish the bundled
accelerator’s provenance. These sources do not clear the assembled bundle. Record exact accelerator provenance,
all external libraries/overlays, notices/corresponding-source obligations,
Microsoft/NVIDIA terms and signing before public distribution. No upstream
Swift implementation has been copied into the launcher by this research.

## Process and extraction ownership

Upstream monitors host processes and may fall back to broad process-name killing.
Cimmeria must not terminate another Wine game. Native code owns the helper host
process and operation ID; the Windows helper owns the guest game handle and
reports lifecycle. A guest PID is not a macOS process handle. Cancel cooperatively
first; any prefix-scoped shutdown requires an exclusive prefix and an explicit
policy for an already-running game. Never use a process-name-wide kill.
Sources: [monitor](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/WineProcessMonitor.swift),
[launch/quit implementation](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift).

The existing `crates/launcher/src/unpack/cab_set.rs` handles a chained MakeCAB
set through Windows FDI. Non-Windows expansion rejects it. Recommend a narrow,
Windows-native-built helper under the pinned Wine adapter for the first Mac
implementation. Require spanning-cabinet, path, cancellation and partial-output
fixtures, then the real archive. Native ZIP/RAR fixture passes do not prove the
retail chained cabinets work under Wine. Fresh-prefix prerequisites, helper
ownership, injection, login and gameplay remain unverified.


## Seed adapter integration status — 2026-10-04

The shared installer now exposes a seed-only extraction adapter with download
cache separate from fresh content staging. It verifies hashes before dispatch
and preserves input/partial output on uncertain extraction. This is an interface
and fixture-tested pipeline, not a configured Wine backend; existing production
callers still use native extraction.

Before selecting a backend, bind runtime/helper identity, cache and staging paths
to durable operation ownership. The adapter must invoke the supervised Windows
helper with native-owned arguments/environment, record host identity before
request dispatch, and distinguish confirmed completion from lost observation.
Recovery/resume must retain the same backend identity and reconcile retained
input/output without inferring Wine guest death from host exit. Runtime/prefix
provisioning, fresh-prefix prerequisites, cabinet-chain fixtures and real archive
validation remain unimplemented integration gates. No Wine runtime was selected
or executed by this packet.


## Runtime artifact inspection — 2026-10-04

Downloaded the pinned `WoWSilicon-WineRuntime-r17.tar.xz` from the
[upstream release](https://github.com/WoWSilicon/WoWSilicon/releases/tag/wine-runtime-r17)
with the GitHub CLI. Its 63,744,856-byte size and SHA-256 match the inventory
above. Archive metadata inspection found 1,850 members under a single
`.wine-runtime` root, with declared expanded file sizes totaling 407,600,761 bytes.
`bin/wine` and `bin/wineserver` have mode `0755`. Of thirteen symlinks, twelve
are under `bin/` and target `wine`; the remaining link is
`lib/wine/x86_64-unix/libvulkan.1.dylib` → `../../external/libvulkan.1.dylib`.

No member filename contains `license`, `copying` or `notice`. This filename check
does not establish that license text is absent from file contents or companion
materials, and does not clear redistribution. The archive was neither extracted
nor executed. Hash and metadata checks establish artifact identity and layout,
not runtime compatibility, successful provisioning or safe link handling by a
future extractor.
