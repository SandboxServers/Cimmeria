---
name: macOS Wine launcher WGL forward-compatible context
description: Tested environment workaround for launcher startup on WoWSilicon 3.2.2; game UAT remains open.
type: reference
---

# Launcher WGL startup — 2026-10-03

The Windows release `launcher-20260929-0d71e26` failed twice under
WoWSilicon 3.2.2 (Wine reports 11.13) on Apple Silicon. With `WINEDEBUG=+wgl`,
the trace showed a core OpenGL 3.3 request (attributes 0x9126=1,
0x2091=3, 0x2092=3), followed by:

```text
OS X only supports forward-compatible 3.2+ contexts
Failed to create context using default context attributes ... [2095] OS Error 8341
extension to create ES context with wgl is not present
```

The ES message is the fallback failure, not the initial request. The release's
Cargo.lock pins eframe 0.35.0 and glutin 0.32.3. eframe first tries default
context attributes, then GLES; glutin defaults desktop GL to core 3.3 and
its WGL implementation does not add the forward-compatible flag.

The pinned WoWSilicon Wine fork already implements `CX_FWD_COMPAT_GL_CTX=1`:
it adds that flag before the check. Retesting the same executable and prefix
with only that environment variable added eliminated both context failures,
reached the launcher update check (started by LauncherApp::new), and left the
process running. The tester then supplied a screenshot confirming a rendered
launcher, manifest schema 1 with seven patches, and an up-to-date launcher
status. The UI was light despite macOS dark appearance. The seed remained
uninstalled. Installation, login, injection, and gameplay remain unverified.

WoWSilicon's Options -> Environment accepts one KEY=VALUE per line. Recommend
this per-profile setting before changing the Cimmeria renderer or Wine build.
The setting was tested as a process environment variable, not persisted into
the profile. The helper-library environment normally supplied by WoWSilicon
should be retained for real play; the direct diagnostic command emitted
FreeType warnings.

## Evidence

- [Pinned Wine implementation](https://github.com/WineAndAqua/wine/blob/37540b5d94ac1c86e2599ef55d7f3a15e3237ce8/dlls/winemac.drv/opengl.c#L2134-L2152)
- [WoWSilicon runtime pin](https://github.com/WoWSilicon/WoWSilicon/blob/v3.2.2/Packaging/WineRuntime/runtime-lock.json)
- [eframe context sequence](https://github.com/emilk/egui/blob/0.35.0/crates/eframe/src/native/glow_integration.rs#L1060-L1088)
- [glutin WGL attributes](https://github.com/rust-windowing/glutin/blob/v0.32.3/glutin/src/api/wgl/context.rs#L78-L189)
- [Mac prerequisite issue](https://github.com/SandboxServers/Cimmeria/issues/1150) and [client runtimes](https://github.com/SandboxServers/Cimmeria/issues/1121)

The existing Mac guide's claim that OpenGL avoids launcher compatibility
problems needs correction. A Windows-copied install must include Cimmeria's
on-disk setup and server list; system runtimes do not travel with the tree.
Direct SGW.exe launch bypasses the launcher's DLL injection. It is a login
smoke workaround, not equivalent to the full supported launcher flow.
