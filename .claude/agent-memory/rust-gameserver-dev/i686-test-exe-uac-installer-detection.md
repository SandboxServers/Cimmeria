---
name: i686-test-exe-uac-installer-detection
description: A 32-bit test exe with "patch" (or install/setup/update) in its name fails with os error 740 under UAC; embed an asInvoker manifest via build.rs rustc-link-arg (not -tests)
metadata:
  type: feedback
---

`cargo nextest run -p cimmeria-patch-wire --target i686-pc-windows-msvc`
failed before any test ran: `The requested operation requires elevation.
(os error 740)`. The test harness is `cimmeria_patch_wire-<hash>.exe`, and
Windows' UAC installer detection treats any **32-bit** exe without a manifest
whose file name contains "patch", "install", "setup" or "update" as an
installer that needs elevation. 64-bit exes are exempt, so the x64 host run
of the same tests is fine, and so is CI (hosted runners run with UAC off).

**Why:** found 2026-09-27 building the Black Market patch crates
(`cimmeria-patch-wire`, `cimmeria-client-patches`). Any future i686 crate
named `*patch*`, `*install*`, `*setup*` or `*update*` hits it.

**How to apply:** a build.rs that, for `CARGO_CFG_TARGET_ARCH == "x86"` and
`CARGO_CFG_TARGET_ENV == "msvc"`, prints
`cargo:rustc-link-arg=/MANIFEST:EMBED` and
`cargo:rustc-link-arg=/MANIFESTUAC:level='asInvoker'`.

- Use plain `rustc-link-arg`. `rustc-link-arg-tests` only reaches `tests/`
  integration targets: on a crate without them cargo rejects the build
  script with "does not have a test target", and it never reaches the lib's
  own unit-test harness.
- On a cdylib the plain form also embeds the manifest in the DLL (resource
  2). That is inert, because it declares no dependent assemblies.
- The i686 target uses MSVC `link.exe` here (`.cargo/config.toml` only
  switches x86_64 to rust-lld), which accepts both flags.

Related: [[build-environment]].
