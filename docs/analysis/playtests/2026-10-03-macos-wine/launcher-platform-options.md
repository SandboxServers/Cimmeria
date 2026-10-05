# Launcher platform options: shared UI or SwiftUI with Rust

> **Type:** Explanation (research and proposed evaluation, not an accepted ADR)
> **Audience:** Launcher maintainers deciding the next implementation experiment
> **Last updated:** 2026-10-03
> **Companions:** [Playtest handoff](README.md), [launcher design](../../../client/sgw-launcher.md), [Mac guide](../../../guides/macos.md), [build policy](../../../../CLAUDE.md)

## Authorized packaging experiment

On 2026-10-03 the user explicitly authorized a bounded native Mac packaging
proof comparing egui and Tauri. This supersedes the earlier no-build permission
status **for that proof only**. The production Windows-native policy remains;
all compiling Cargo calls still use the build lane. No architecture is selected.
See the [proof runbook](../../../../crates/launcher/prototype-packaging/README.md)
for commands, self-contained first-open criteria, current evidence and the
Windows-native handoff. At this handoff, egui release build and actual-model JS
UAT passed; Tauri compilation and offline/visual verification remain unfinished.

## Recommendation

### Accepted first-open requirement

Confirmed with the product owner on 2026-10-03: after downloading the platform
package, the launcher must open and display its interface without downloading
additional software or requiring separately installed tools. Its UI assets and
required non-system runtime components must ship in that package. Offline opening
must remain usable and explain unavailable network operations.

Selecting Install starts automated game preparation: preflight checks, obtaining
and configuring game prerequisites and the compatibility runtime, downloading
and verifying content, applying patches, and checking readiness for Play.
Players should not need a separate prerequisite-installation guide.

Keeping the installer engine in Rust is **not a hard requirement**. Reuse remains
an engineering tradeoff, not a constraint on the chosen implementation language.
The framework recommendations below are provisional under this product contract.

The next experiment should prove offline first-open from release packages on
clean Windows and Apple Silicon environments before investing in the full UI.
For Tauri, evaluate bundled fixed-version WebView2 on Windows; an online
bootstrapper cannot satisfy the requirement. An offline WebView2 installer avoids
downloads but adds a runtime-install step, whereas a bundled runtime better fits
the desired immediate opening. macOS uses its system WKWebView. These are
documented packaging paths, not validated Cimmeria packages: see
[Windows distribution](https://v2.tauri.app/distribute/windows-installer/) and
[webview providers](https://v2.tauri.app/reference/webview-versions/).

### Provisional framework recommendation

A shared launcher can run natively on Apple Silicon and Windows and can plausibly
meet the desired responsiveness. SwiftUI is not a prerequisite for native
execution or native GPU rendering. No implementation here has been benchmarked
against that goal yet.

Keep **egui/eframe with wgpu and Metal on macOS** as the leading candidate: it
preserves the existing Rust interface and avoids introducing another frontend
language. Evaluate **Tauri 2 with TypeScript/CSS and a shared Rust core** as the
challenger when faithful reproduction of the approved HTML design is the main
priority. Choose **SwiftUI with shared Rust behavior** only if measured benefits
in accessibility and macOS interaction conventions justify maintaining two
frontends. Do not rewrite the installer in Swift merely to obtain a SwiftUI
window.

This recommendation is an experiment order, not a platform commitment. The
current repo permits Windows-native builds and has no Mac product target.
**The packaging proof above has a scoped exception; obtain explicit approval
before expanding it into a production Mac implementation.** Windows builds continue through `tools/build-lane/lane.sh`.
This research does not amend those rules or authorize cross-compilation.

For the immediate tester, continue the existing Windows launcher under
WoWSilicon using the saved `CX_FWD_COMPAT_GL_CTX=1` setting. A new native launcher
is a separate product effort and is not necessary to clear that window blocker.

## Evidence and its limits

Repository inspection used production source at `ce488734d1f3e35abc9cc67fd880f047dba581e1`.
The approved visual reference is [the prototype at e70b076a9](https://github.com/SandboxServers/Cimmeria/tree/e70b076a9/crates/launcher/prototype-macos).
Upstream documentation was checked on 2026-10-03; URLs containing `latest` may
change. No comparative binaries, latency traces or memory measurements were made.

| Evidence class | What it establishes | What it does not establish |
|---|---|---|
| Observed on the tester's Mac | The existing Windows launcher renders with the saved WGL workaround; the manifest loads | Installation, injection, login, gameplay, or native-launcher performance |
| Current repository inspection | Windows dependencies, preparation ownership, process boundaries, existing renderer configuration | The cost of implementing or maintaining a complete port |
| Primary framework documentation | Supported backends, APIs, accessibility hooks, testing mechanisms | Our application's latency, resource use, usability, or release reliability |
| Historical project decision | The rationale recorded when Tauri was replaced by egui | A current benchmark or a permanent rejection of all webview interfaces |
| Engineering inference | An experiment worth testing and likely maintenance tradeoffs | A measured winner or an approved architecture |

The historical anchor matters: [PR #343](https://github.com/SandboxServers/Cimmeria/pull/343)
replaced a Tauri 2 launcher with egui, emphasizing a roughly 5 MB executable at
that time and the absence of a JavaScript toolchain. That figure is **historical**,
not today's launcher size; the PR is not a controlled responsiveness comparison.
Reintroducing Tauri should therefore explain the product benefit that justifies
reversing the toolchain simplification.

## What “native and responsive” should mean

Separate three questions before choosing a framework:

- **Native execution:** the launcher's own executable runs as arm64 on macOS,
  rather than running the Windows launcher through Wine. Shared source produces
  separate per-OS artifacts, not one executable that runs unchanged everywhere.
- **Native rendering and controls:** Metal renders a custom interface, a webview
  renders HTML, or SwiftUI supplies platform controls. These are different UI
  mechanisms; none alone guarantees responsiveness.
- **Native interaction:** keyboard focus, text selection, menus, accessibility,
  file dialogs, system appearance, and window behavior follow Mac expectations.

A native egui/wgpu application satisfies the first question and uses a native
Metal backend for the second. It still needs deliberate work on the third.
SwiftUI starts with useful platform conventions, but custom views can lose
semantics and heavy main-thread work can still cause hangs. Apple's
[SwiftUI performance guidance](https://developer.apple.com/documentation/xcode/understanding-and-improving-swiftui-performance)
explicitly treats unnecessary updates and main-thread work as problems to fix.

WoWSilicon remains necessary for the **Windows game**, independently of the
launcher frontend. A native launcher neither ports SGW nor proves that its
D3D9 renderer, old runtimes, or injected patches work.

## Comparison

| Dimension | Shared egui/eframe + wgpu | Shared Tauri 2 + TS/CSS + Rust | SwiftUI + shared Rust |
|---|---|---|---|
| Existing implementation reuse | Highest UI reuse; renderer and platform work remain | Retains Rust behavior after extraction; replaces UI | Retains Rust behavior after extraction; adds a Mac UI alongside Windows |
| Approved custom visual design | Recreate design with Rust layout and painting | Direct fit for an HTML/CSS design; still integrate real state | Recreate design with SwiftUI styling and layout |
| Native execution | Native Rust executable | Native host and platform webview | Native SwiftUI shell and native Rust library/helper |
| Graphics | wgpu supports Metal on Mac | Platform webview composition | SwiftUI platform rendering |
| Mac conventions | Implement and test custom-widget behavior | HTML semantics plus platform integration; test the actual webview | Standard controls provide a useful baseline; custom design still needs testing |
| Accessibility | Enable AccessKit and verify real semantics | Semantic DOM and webview accessibility need platform checks | Standard controls supply defaults; custom elements require explicit semantics |
| Toolchain burden | Rust plus Mac packaging/signing | Rust, JS/TS dependencies and tooling, platform webviews | Rust, Swift/Xcode, interface bindings or a process protocol, two frontends |
| Performance confidence today | Unmeasured | Unmeasured | Unmeasured |
| Primary risk | Assuming existing Rust code is already portable | Reversing prior simplification without enough product value | Duplicated UI behavior and release work |

Sources: [wgpu backends](https://docs.rs/wgpu/latest/wgpu/),
[eframe features](https://docs.rs/eframe/0.35.0/eframe/),
[Tauri architecture](https://v2.tauri.app/concept/architecture/),
[SwiftUI ButtonStyle](https://developer.apple.com/documentation/swiftui/buttonstyle),
[SwiftUI accessibility](https://developer.apple.com/documentation/swiftui/accessibility-fundamentals).
These document capabilities, not comparative performance results.

For Tauri, the UI runs in [WKWebView on macOS and WebView2 on Windows](https://v2.tauri.app/concept/process-model/).
That permits a shared UI, but requires checking both engines rather than assuming
a Chrome prototype proves Mac rendering. Keep content verification and file
operations in Rust. Its [command documentation](https://v2.tauri.app/develop/calling-rust/)
puts ordinary synchronous commands on the main thread; CPU-heavy work needs a
worker even when invoked from an async command. Its [channels](https://v2.tauri.app/develop/calling-frontend/)
support streaming progress; coalesce updates instead of sending a message for
every byte or log line. Ship local UI assets, render manifest text as data, and
limit privileged commands using [capabilities](https://v2.tauri.app/security/capabilities/).

Tauri also supplies documented [Mac bundling](https://v2.tauri.app/distribute/macos-application-bundle/),
[Windows installer/WebView2 provisioning](https://v2.tauri.app/distribute/windows-installer/)
and [signed application updates](https://v2.tauri.app/plugin/updater/). Adopting its
updater is a migration from the current `.exe` release contract, not a drop-in
change to the signed game-content manifest. They are separate update systems.

**Slint** is a credible fourth option: declarative custom UI with Rust behavior
and [Metal-capable and software renderers](https://docs.slint.dev/latest/docs/slint/guide/backends-and-renderers/backend_winit/).
It introduces another UI implementation and requires an explicit
[license selection](https://github.com/slint-ui/slint/blob/master/LICENSE.md).
Consider it if both shortlisted approaches fail a requirement; there is no
measured benefit here that justifies a third prototype immediately. **Iced**
likewise adds a rewrite; its [README](https://github.com/iced-rs/iced) still calls
it experimental and [accessibility work](https://github.com/iced-rs/iced/issues/552)
requires investigation. Neither is dismissed as inherently slow.

[Electron](https://www.electronjs.org/docs/latest/) embeds Chromium and Node.js;
[Flutter desktop](https://docs.flutter.dev/platform-integration/desktop) supports
Windows and macOS. Neither is ruled out as incapable or slow. They add a runtime
or toolchain and a frontend rewrite without an identified advantage over this
shortlist's reuse of existing Rust behavior, so defer them unless requirements
change.

The current launcher selects `glow`, disables eframe default features, and does
not enable `accesskit`. A proposed native wgpu/Metal build must select and test
its own features; it is not evidence that today's executable already has those
backends or accessibility support. A wgpu fallback is also not synonymous with
a software renderer.

## The porting work lives below the window

Changing frontend does not remove these repository boundaries. The existing
`sgw-start32` helper already exposes a language-neutral CLI with an `ok pid`
response and inherited environment, but exits after resuming the game. That is
not a serialized, long-lived interface to the entire launcher worker:


| Current boundary | Consequence for any native Mac shell |
|---|---|
| `crates/launcher/src/main.rs` constructs the GUI directly | There is no ready-to-use headless launcher command interface |
| `crates/launcher/src/worker/messages.rs` defines in-process messages | A subprocess interface needs an explicitly versioned serialized protocol |
| `crates/launcher/src/app/mod.rs` owns configuration and pre-launch preparation | Extract shared orchestration; a new frontend must not independently reconstruct launch rules |
| `crates/launcher/src/unpack/cab_set.rs` uses Windows FDI for the installer cabinet set | Supply a verified portable CAB path or keep extraction in a Windows worker under Wine |
| `crates/client-launch` implements Windows launch and injection | Keep a Windows helper for the game; do not replace injection with an ordinary host process spawn |
| Windows helper reports a Windows PID | Model operation ownership and exit observation; do not treat a Wine guest PID as a macOS PID |
| Launcher state and update assumptions include executable-adjacent files | Define writable Mac state paths and signed-bundle update behavior |
| Repair and uninstall are proposed product flows | Specify ownership and recovery semantics; do not present mock buttons as implemented operations |

A useful shared boundary would accept an intent, publish progress and a resulting
snapshot, and support explicit cancellation. It should own installation truth,
launch preparation, and recovery. The UI should render that truth, not infer it
from a progress percentage or reimplement it independently.

Current UI-path inspection also identifies work to measure before blaming a
renderer: `app/view.rs` calls synchronous `refresh_install_state` every two
seconds; `app/mod.rs` calls `client_setup::prepare` during launch preparation;
and event draining loops through available events without a bounded batch.
These are investigation candidates, **not measured performance defects**. A new
frontend that preserves the same blocking work could preserve the same stalls.

The exact API is still a design task. Candidates such as `inspect`,
`perform(intent)`, and `cancel(operation)` illustrate a narrow boundary; they are
not existing repo APIs. Persisted operations, compatibility versions, error
classification, and file ownership need tests before a second frontend relies
on them.

CAB compatibility deserves an early proof: the real seed is a multi-cabinet
installer set, not simply a ZIP whose decompressor can be swapped. Test against
the actual format and expected extracted tree, including files spanning cabinets,
before claiming a native extractor is interchangeable with FDI.

## How SwiftUI could reuse Rust

Swift is the language; SwiftUI is Apple's UI framework. Apple's
[SwiftUI platform scope](https://developer.apple.com/documentation/technologyoverviews/swiftui)
is Apple platforms, so this approach does not provide the same SwiftUI frontend
on Windows. Shared Rust behavior still needs separate Windows and Mac views.

There are three materially different approaches:

1. **SwiftUI with a Windows worker under Wine.** Add a headless worker and a
   versioned protocol. Existing Windows extraction and injection stay near their
   implementation. The window is native; background work still depends on Wine.
2. **SwiftUI with a native Rust library through UniFFI.** Generated bindings keep
   business logic in Rust. The Windows game helper remains a separate concern.
3. **SwiftUI with a native Rust subprocess.** A serialized protocol trades FFI
   integration for process lifecycle and protocol maintenance; helper failures
   can be isolated from the window.

[UniFFI](https://mozilla.github.io/uniffi-rs/latest/) is designed for sharing Rust
logic with other languages. Its [Swift documentation](https://mozilla.github.io/uniffi-rs/latest/swift/overview.html)
describes generated Swift types, headers and module maps, but library shipping
remains the application's responsibility. It also records Swift 6 concurrency
limitations. [Async interop](https://mozilla.github.io/uniffi-rs/latest/futures.html)
does not automatically supply cancellation. Pin the Rust toolchain, UniFFI
version, generated bindings and Swift language mode together and test explicit
cancellation across the chosen boundary.

All three approaches still require moving orchestration out of the existing
GUI. SwiftUI is not a shortcut around that work. An all-Swift installer rewrite
would add a second implementation of download verification, patch order,
recovery, and launch preparation without evidence that those algorithms cause
the current UI problem.

## Proposed evaluation gates — not measurements

Compare the same narrow workflow and visual design on the same hardware, with
the same synthetic worker events and content workload. Use release builds and
record framework versions, device, display refresh rate and power mode.
Collect distributions and worst observed stalls, not one favorable screenshot.

| Gate | Proposed acceptance signal |
|---|---|
| Button acknowledgement | Visible feedback within 100 ms; no wait for network or disk completion |
| Interaction while busy | At 60 Hz, work fits the 16.7 ms frame budget; measure scroll/resize hitches and frame-time distribution |
| Hashing and extraction | No synchronous bulk work on the UI thread; interactions remain available during sustained work |
| Startup | Measure process start to usable window separately from manifest/update completion; offline startup remains usable |
| Idle | Quiescent when unchanged; inspect CPU, wakeups and energy, including hidden/minimized behavior |
| Memory | Report the whole launcher process tree; count webview/helper processes and report Wine/game memory separately |
| Cancellation | Immediate acknowledgement, eventual worker cancellation, honest terminal state and restart-safe recovery |
| Accessibility | Keyboard-only completion, visible focus, VoiceOver/Narrator semantics, text scaling and reduced motion; retain approved dark-only appearance |
| Persistence | Restart restores the actual operation/install state without presenting partially applied content as ready |

These numbers are proposed project gates, not documented guarantees from any
framework. Agree on precise tolerances for frame outliers, cold startup and idle
resource use before using them to choose a winner. A slower network must not
make a framework appear to have a slower window.

Apple's [responsiveness guidance](https://developer.apple.com/documentation/xcode/improving-app-responsiveness)
warns that asynchronous code can still execute on the main actor. “Uses async”
is therefore not proof that hashing or extraction cannot block a SwiftUI view.
The same separation applies to Rust UI threads and Tauri command handlers.

## Verification layers

Shared orchestration tests should exercise interrupted downloads, integrity
failures, disk errors, ordered patches, cancellation, restart recovery and safe
path handling. Pick tests using [TESTING.md](../../../../TESTING.md), with
fixtures that independently establish the expected result.

For every meaningful frontend change, perform the repository's JS REPL-style
logic UAT in addition to ordinary tests. For a TS state model, exercise the real
reducer/persistence logic. For Rust or Swift, a JS harness can exercise a real
serialized worker protocol if one exists. Do not translate production behavior
into a separate JS imitation and call that coverage. If no meaningful JS seam
exists, state that limitation, identify the native logic tests performed, and
retain visual/manual UAT as a separate obligation.

Tauri has a testing distinction worth preserving: the current
[WebDriver guide](https://v2.tauri.app/develop/tests/webdriver/) describes macOS
automation through an embedded WebdriverIO server. Native `tauri-driver` alone
still does not supply macOS support. Select and verify the documented mechanism
rather than repeating the older blanket claim that Mac automation is unavailable.
Browser-only testing of the HTML prototype does not test the packaged webview,
Rust commands, updater or operating-system integration.

Whichever frontend wins, test fresh installation, upgrade and recovery on clean
machines. Human Mac UAT still covers appearance, focus, dialogs, VoiceOver and
the real SGW login/world/Black Market flow. A successful native window or mocked
worker cannot substitute for that game test.

## Distribution and maintenance decision

Both native approaches need app-bundle layout, signing, hardened runtime,
notarization and release validation. Apple's
[notarization guide](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
and [packaging guide](https://developer.apple.com/documentation/xcode/packaging-mac-software-for-distribution)
make distribution a separate engineering activity from drawing the UI.

WoWSilicon runtime redistribution rights, entitlements and a clean first-run
provisioning flow remain unverified. Decide whether the product manages a
runtime or discovers an external installation before promising a one-click Mac
installer. The frontend selection does not settle this obligation.

The inspected [WoWSilicon v3.2.2 runtime environment](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/BundledWineRuntime.swift)
includes prefix and library/renderer configuration. Its
[game launch path](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Sources/WoWSiliconSwift/Services/LaunchService.swift)
adds x87 setup that its third-party-launcher path does not explicitly add.
This source difference is not an observed SGW failure, but it means calling
`wine` alone is not a verified reproduction of the complete launch environment.
No stable public launch/profile API was identified in the inspected entry points;
use a version-pinned adapter rather than treating internal profile JSON as an API.
The root [GPLv3 license](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/LICENSE)
does not by itself establish redistribution terms for every binary in the
[runtime inventory](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/runtime-lock.json).
Component provenance and final signed-package behavior remain release gates.

Before implementation, name the owners of Windows and Mac releases, signing
credentials, runtime updates and frontend parity. With two frontends, require
one shared behavioral contract and explicit parity tests. With Tauri, accept the
JS dependency/tooling lifecycle. With egui, accept the work needed for the
approved design and native interaction details.

Stop or change direction when evidence warrants it:

- Keep native work within the authorized packaging-proof scope; stop before
  expanding production platform support without an explicit policy decision.
- Stop a frontend candidate if it cannot meet agreed accessibility or
  responsiveness gates without disproportionate maintenance work.
- Stop native extraction if the real CAB fixture cannot be handled correctly;
  assess a Windows worker rather than silently changing install semantics.
- Stop release promises if runtime distribution, signed updates, or clean-machine
  recovery remain unresolved.
- Do not fund a second frontend solely for an assumed speed improvement. Require
  a demonstrated interaction or accessibility benefit that the shared UI cannot
  reasonably deliver.

The next decision is a bounded comparison of shared egui/wgpu and Tauri against
the approved design, after policy approval and a shared worker contract sketch.
Bring SwiftUI into that comparison when Mac conventions are a concrete product
requirement. Record the measured choice in an ADR only after the evidence exists.

## Closer comparison: egui/wgpu versus Tauri

Tauri permits a [plain TypeScript frontend](https://v2.tauri.app/start/frontend/);
a spike need not add React, SSR or a shipped Node server. The repo already uses
React 19, TypeScript, Vite 6 and Tauri 2 in its other frontends (see
`frontend/package.json` and `tools/ContentEditor/ui/package.json`), so this restores a
stack to the **launcher**, rather than introducing it to the entire project.
A small React/TS/Vite implementation may fit that convention; maintainer
familiarity has not been established. The archived `tools/SGWLauncher` is not the
production reference.

Neither candidate dominates every requirement. **egui minimizes implementation
change and keeps one Rust toolchain; Tauri favors CSS design iteration and offers
packaging integrations.** The user has not chosen either. Compare these actual
benefits against their maintenance costs instead of treating “native” or “web”
as a performance verdict.

### Translate the approved design deliberately

| Design element | egui/wgpu approach | Tauri approach |
|---|---|---|
| Hero art and gradient treatment | Load a licensed static texture; compose panels and overlays with egui painting. Bake complex static effects into assets where appropriate | Use the same licensed asset with CSS gradients, overlays and responsive layout |
| Typography | Configure font families, sizes and spacing; optionally bundle licensed fonts | Start from the prototype’s system-font stack; optionally bundle licensed fonts |
| Primary action and progress | Style standard buttons and progress widgets while preserving enabled/focus semantics | Style semantic `button` and progress elements; retain disabled state, labels and keyboard behavior |
| Play and Patch Notes tabs; gear settings | Rust view composition with selected-tab/modal state | Components or targeted DOM updates with selected-tab/modal state |
| Text fields and dialogs | Standard widgets plus a native dialog adapter | Semantic form controls plus native dialog plugin |
| Window chrome | Use the operating system's actual titlebar/window controls | Use native window decoration or a tested platform titlebar integration |

Neither route requires reproducing the prototype's decorative traffic lights as
fake buttons. Native window controls must move, minimize and close the real
window correctly. A polished image is not sufficient evidence for keyboard
focus, resize hit targets, drag regions or accessibility.

The prototype currently uses system fonts; no downloadable font was approved.
If you choose a bundled font, licensing is a release requirement: permission to view a font
on a website does not establish permission to bundle it in a desktop app.
Record the font and hero-image licenses with the assets, including redistribution
terms. Use comparable fonts and the same approved assets in the comparison so a typography
mismatch is not mistaken for a framework limitation.

Native file dialogs are available to both candidates. Rust's
[`rfd`](https://docs.rs/rfd/latest/rfd/) provides native file/message dialogs;
Tauri supplies a [dialog plugin](https://v2.tauri.app/plugin/dialog/). Test the
selected integration (including rfd’s macOS main-thread/app-context guidance)
for cancellation, Unicode and spaced paths, initial
location, and parent-window behavior. A plugin's existence does not prove the
launcher has integrated those flows correctly.

### One Rust engine, two possible presentation boundaries

The engine should own installation truth, operation lifecycle, cancellation,
telemetry consent and game lifecycle. It publishes snapshots and accepts intents.
A frontend may own the selected tab, expanded notes and an open modal; it must
not decide that files are installed merely because an animation reached 100%.

With egui, the UI can consume typed Rust snapshots and send typed intents through
in-process channels. The engine can remain independent of egui so its behavior
is testable without creating a window. This is the smaller change from the
existing worker pattern, although moving preparation out of `app` remains work.

With Tauri, async commands carry intents and a channel carries snapshots or
progress events across a serialized boundary. Define schema versions, operation
IDs and error types; validate commands in Rust. Keep the webview a presentation
client of that interface. Tauri documents [Rust commands](https://v2.tauri.app/develop/calling-rust/)
and [Rust-to-frontend channels](https://v2.tauri.app/develop/calling-frontend/);
they supply transport mechanisms, not an installation state model.

The approved prototype's `state.mjs` is a simulation. Its useful state scenarios
can become contract fixtures, but its synthetic install/game status must not
become a second authority when wired to the real launcher. Similarly, replacing
a whole view with `innerHTML` on every progress update risks losing input focus,
selection and transient form state. Use targeted updates or stable components
and verify focus during repeated worker events.

### Keep both interfaces idle when nothing changes

An egui interface does not need to repaint continuously while idle: request
repaint for relevant events and animations. Tauri likewise should not run a
permanent animation loop or repeatedly rebuild the DOM when the snapshot has
not changed. The current egui UI already has a worker-event wakeup and repaints
every 33 ms while installing; preserve event-driven wakeups and measure queue floods,
rather than assuming it always renders continuously. Measure CPU and wakeups in
visible, background and minimized states.

As a **proposed starting point**, coalesce ordinary download/hash progress to
roughly 5–10 updates per second, while delivering errors, cancellation and
terminal transitions immediately. This is not a measured optimum or a product
requirement. Keep animation timing separate from worker progress and tune using
real traces so coalescing does not hide a completed or failed operation.

Send compact metadata across the boundary: operation ID, stage, counts and
status. Do not serialize archives, extracted file contents or enormous log
buffers over frontend IPC. Rust owns download, hashing, extraction and bounded
log storage. Move synchronous launch preparation off the UI thread in either
candidate; neither CSS nor Metal fixes blocking filesystem work.

### Verify semantics before judging screenshots

[`egui_kittest` 0.35](https://docs.rs/egui_kittest/0.35.0/egui_kittest/)
provides semantic interaction tests and wgpu-backed snapshot testing. That makes
“click Install, observe busy state, cancel, observe terminal state” testable
without substituting pixel coordinates for behavior. It does not automatically
cover the packaged app, native dialogs, VoiceOver or the Wine helper.

For Tauri, combine state/contract tests with packaged-webview automation using
the platform-appropriate mechanism described above. Keep embedded WebDriver
plugins test-only and verify they are absent from shipped artifacts; browser
mode with mocked invokes does not prove WKWebView or real IPC behavior.
Both candidates still need
visual checks at different window sizes and scaling, keyboard-only operation,
and real Mac accessibility checks. Run the same scenarios against the same
engine fixtures and record what each layer does not cover.

### Migrate in reversible stages

1. Extract and test authoritative Rust orchestration while retaining the
   existing egui launcher as the working product.
2. Agree on snapshots, intents, cancellation and restart semantics. Add a thin
   adapter for the selected experiment; do not fork installer logic.
3. Implement the approved core screen, settings and notes with real engine
   events. Compare appearance, focus, idle behavior and busy-state responsiveness.
4. Prove install, update, launch, consent, error and recovery parity before
   replacing the maintained frontend or publishing a Mac artifact.

A Tauri candidate should be a fresh adapter to current Rust behavior, not a
resurrection of the historical Tauri launcher and its obsolete assumptions.
Tauri's [bundler](https://v2.tauri.app/distribute/) and
[updater plugin](https://v2.tauri.app/plugin/updater/) can reduce integration work,
but still need signing, permissions, failure recovery and release ownership.
The updater expects SemVer and platform-specific URLs/signatures; current
`launcher-YYYYMMDD-sha` release tags are not directly compatible. Design version
mapping, asset naming, existing `.exe` migration and state adoption. Do not run
the legacy swap updater and Tauri updater concurrently.
Launcher-app update signatures are separate from the game's signed content
manifest; preserve both trust boundaries rather than treating one as the other.

Tauri also adds an IPC permission boundary: [custom commands](https://v2.tauri.app/security/capabilities/)
need explicit restriction and Rust-side validation of paths and process inputs;
plugin scopes alone do not constrain arbitrary commands. Use a bundled frontend,
an explicit [CSP](https://v2.tauri.app/security/csp/), narrow operations and
schema-checked DTOs, rather than a general shell/filesystem bridge.

Favor egui if it reproduces the required design and passes interaction gates
with modest changes. Favor Tauri if its demonstrably better design workflow or
platform integrations outweigh the added frontend/protocol maintenance while
meeting the same gates. Neither choice is justified by invented FPS or memory
ratios; the deciding evidence is the bounded experiment and its upkeep cost.
