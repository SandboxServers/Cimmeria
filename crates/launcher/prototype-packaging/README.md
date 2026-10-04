# Build and inspect the launcher packaging proofs

> **Type:** How-to (throwaway experiment)
> **Audience:** Maintainers comparing native egui and Tauri packaging
> **Last updated:** 2026-10-04
> **Companions:** [Platform research](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-platform-options.md), [playtest handoff](../../../docs/analysis/playtests/2026-10-03-macos-wine/README.md), [build rules](../../../CLAUDE.md), [test policy](../../../TESTING.md)

## Scope and permission

The user explicitly authorized this bounded native Mac packaging experiment.
That exception supersedes the research note's earlier “no Mac build authorized”
status **for this proof only**. It does not introduce a supported production Mac
target, change the Windows-native production build policy, or select a frontend.
Every compiling Cargo invocation still runs through the build lane.

The two shells share the Rust model in `model/`. Its operations are simulations:
there is no game download, installation, patching, repair, uninstall, persistence,
telemetry transmission, Wine provisioning or game launch. A checked telemetry
box only changes an in-memory Boolean and resets on restart.

The proof requirement is **self-contained first open**: after receiving and
extracting the packaged application, the tester should open its window without
Node, npm, Rust, a development server, or an additional runtime download.
Build-time dependency downloads are permitted. macOS system frameworks are
allowed; Windows needs a separately verified runtime packaging strategy.
This requirement concerns opening the launcher proof, not downloading SGW.

## Build on the authorized Mac

From the repository root, with the pinned Rust toolchain, Apple build tools,
Node/npm and the Tauri Cargo CLI available:

```bash
bash crates/launcher/prototype-packaging/build-macos.sh
```

The wrapper builds local TypeScript assets, runs the egui/model release build
through `tools/build-lane/lane.sh`, and invokes the Tauri build inside another
lane slot. Do not run `tauri-build.sh` directly: its `cargo tauri build` also
compiles Rust and depends on the outer lane wrapper.

The packaging step writes to `crates/launcher/prototype-packaging/dist/`:

- `Cimmeria Egui Proof.app` and its ZIP.
- `Cimmeria Tauri Proof.app` and its ZIP, when the Tauri bundle exists.

The egui bundle is ad-hoc signed for local development. Neither artifact is a
notarized distribution release. An artifact appearing in `dist/` alone does not
prove that it opens, passes Gatekeeper on another Mac, or is self-contained.

## Run the shared-model logic UAT

After the model release executable exists, run from the repository root:

```bash
node crates/launcher/prototype-packaging/uat.mjs \
  target/release/packaging-proof-model
```

This JS REPL-style pass drives the **actual Rust model executable** with action
lines and checks its JSON snapshots. It exercises telemetry toggles, notes,
settings, home navigation, simulated install feedback and repair/uninstall
no-action feedback. It also verifies that returning home preserves the current
in-memory telemetry choice.

It does not cover UI bindings, native accessibility, persistence, real
installation or a clean machine. Persistence is deliberately absent from the
proof; do not describe the in-memory assertions as restart-state validation.

## Inspect first-open behavior

For each packaged app:

1. Copy/extract the artifact to a separate location and launch the bundle itself,
   with no frontend development server running. Record OS, architecture, package
   hash and artifact size.
2. Repeat with network access unavailable. Confirm that the initial screen and
   bundled notes render without loading a remote asset or fetching a runtime.
3. Exercise Play/notes navigation, settings, telemetry toggle and simulated
   actions. Check native titlebar controls, resize, keyboard focus and readable
   text. Record visual evidence separately from model-test output.
4. Inspect linked libraries and bundle contents for development-machine paths
   and missing dependencies. A local launch on a developer Mac is useful but
   does not replace a clean-machine test.
5. Restart and confirm that the proof resets, as designed. Record first-open
   errors and any prompts rather than treating ad-hoc signing as notarization.

## Evidence ledger

| Check | Result |
|---|---|
| Native Mac release packages | Both built successfully through the lane; arm64 Mach-O executables |
| Shared-model JS UAT | Passed against the actual Rust executable |
| Compiled TypeScript binding UAT | Passed: queued actions, tabs, settings, consent retention, repair feedback, seven manifest entries |
| Linked libraries | `otool -L` lists only system frameworks and `/usr/lib` imports for both executables |
| Initial egui offline drawing | Observed with `sandbox-exec` network denial and a system-only PATH; not a clean-machine test |
| Initial Tauri window and interactions | Observed: dark UI, bundled notes, telemetry toggle, install simulation, settings and repair feedback |
| User assessment of initial Tauri proof | Responsive, fast opening, preferred over egui; requested better resizing and closer approved styling |
| Revised Tauri styling/resizing | Rebuilt; scrolling content and persistent Install/telemetry footer implemented. Visual review pending |
| Tauri network-denied startup | Inconclusive: process remained alive, but automation could not bind its window; normal launch worked. Do not count as an offline pass |
| Clean-machine installation and distribution trust | Pending |
| Windows-native packaging | Not run; Windows handoff below |

The local release ZIPs measured 4,542,178 bytes (egui) and 2,367,291 bytes
(Tauri). These Mac sizes exclude any game compatibility runtime. They say
nothing about Windows size with bundled WebView2. Both are local development
artifacts, not signed/notarized distribution releases.

No comparative startup timing, memory or frame-performance measurements were
collected. The user's responsiveness assessment is qualitative. Tauri is the
preferred direction for the next iteration, not a completed architecture decision.
Rust remains optional for the eventual installer engine; this proof shares a
Rust model only to keep both shells' simulated behavior comparable. Effect is
not included because it does not resolve first-open packaging.

The revised layout borrows the previously approved prototype's dark palette,
gate motif, typography, tab styling and cyan action. It uses a responsive hero
and one scrolling content area, with action/consent outside that area. No A/B/C
variant was explicitly selected; this is an adaptation, not final design signoff.
Game actions remain simulations, and the proof has no install-to-Play lifecycle.

Run the additional binding check after building TypeScript:

```bash
node crates/launcher/prototype-packaging/ui-uat.mjs \
  target/release/packaging-proof-model
```

It executes the compiled TypeScript with a minimal DOM test adapter and delegates
state changes to the real model executable. It does not test CSS layout, WebKit
IPC transport, VoiceOver, persistence, or installation. Before distribution,
review the rebuilt app at its minimum 480×560 window, default 620×700, and a
larger window; verify notes/settings scroll while Install and consent remain
reachable. Desktop automation must be requested and explicitly approved by the
user before opening, focusing, resizing, or interacting with windows.

## Hand off the Windows proof

Build on Windows rather than cross-compiling from this Mac. Route all compiling
Cargo calls, including `cargo tauri build`, through `tools/build-lane/lane.sh`.
The current Mac wrapper and bundle target are not a Windows packaging recipe.

For Tauri, configure a **fixed WebView2 runtime bundled at build time** and
package the intended architecture. Do not rely on an online bootstrapper or an
already-installed Evergreen runtime to satisfy self-contained first open.
The runtime's redistribution terms, servicing and added package size need
explicit review. See [Tauri Windows installer guidance](https://v2.tauri.app/distribute/windows-installer/)
and [Microsoft's WebView2 distribution guide](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution).
This proof's current `tauri.conf.json` does not yet configure that Windows bundle.

Inspect the PE dependency tree for **both** applications, including Visual C++
CRT and other native DLL requirements. Choose and verify an appropriate
redistribution or linking strategy; a Rust executable is not automatically a
self-contained executable. Test each final package on a clean Windows machine
offline, with no developer tools or preinstalled WebView2 assumption.

Keep the Windows results separate from Mac evidence. Even a successful proof on
both operating systems would establish packaging feasibility, not a production
installer, game compatibility, signed updater, or accepted launcher architecture.

### Windows operator checklist

1. Check out `prototype/launcher-packaging-proof` on a Windows-native build host
   and follow the repository's pinned toolchain/build-lane setup. Use Git Bash
   with the native MSVC toolchain configured, not WSL compilation.
2. In `crates/launcher/prototype-packaging/tauri`, run `npm ci --ignore-scripts`
   and `npm run build`. Return to the repository root.
3. Build the egui/model executables through the lane:

   ```bash
   bash tools/build-lane/lane.sh cargo build --locked \
     --manifest-path crates/launcher/prototype-packaging/Cargo.toml \
     --target-dir target --release \
     -p cimmeria-egui-proof -p packaging-proof-model
   ```

4. Acquire the appropriate fixed WebView2 runtime from Microsoft **on the build
   host**. Record version, architecture, source and SHA-256; extract it into a
   build-only directory. Create a Windows Tauri config override with an `nsis`
   bundle target and `bundle.windows.webviewInstallMode` set to
   `{"type":"fixedRuntime","path":"<extracted-runtime-directory>"}`.
   Resolve the path according to the Tauri configuration reference. Keep the
   runtime artifact out of Git. This configuration is untested here.
5. Inside a lane-controlled shell wrapper, set `CARGO_TARGET_DIR` to the root
   `target` directory, change to the proof's `tauri` directory, then run
   `cargo tauri build --config <windows-override.json> --bundles nsis -- --locked`.
   Invoke that wrapper with `bash tools/build-lane/lane.sh bash <wrapper.sh>`.
   Do not use the Mac-only `--bundles app` wrapper.
6. Run both JS UAT scripts with `target/release/packaging-proof-model.exe`.
   Inspect imports using `dumpbin /DEPENDENTS` and resolve required CRT DLLs
   through verified static linking or packaging. Record any lane-provided CRT
   flags; the Mac Tauri build emitted a deprecated `STATIC_VCRUNTIME` warning,
   which is not evidence that Windows CRT packaging works.
7. On a clean supported Windows VM, disconnect networking **before first open**
   and verify both packages without Rust, Node, developer tools, or a preinstalled
   WebView2 assumption. Include unpack/install steps and any runtime installer
   prompts in the result. Record hashes, package sizes, OS and architecture.
8. Report screenshots and failure logs, then reconnect only for the separate
   game-installation workstream. No game prerequisites are downloaded by this proof.
