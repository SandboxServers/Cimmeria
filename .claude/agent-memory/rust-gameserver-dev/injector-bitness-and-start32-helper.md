---
name: injector-bitness-and-start32-helper
description: DLL injection into the 32-bit SGW.exe must run at 32-bit; the x64 launcher uses the i686 sgw-start32 helper; the WOW64 kernel32 export-table resolver does not work on a suspended target
metadata:
  type: project
---

`cimmeria-client-launch::inject::inject_dll` hands the remote thread the injector's own
`kernel32!LoadLibraryW`, valid only at the same bitness. Measured 2026-09-27 (BM-06,
PR #984) against a copy of `SysWOW64\hostname.exe` / `PING.EXE` + `SysWOW64\version.dll`:
from x86_64 the remote call returns `RemoteLoadFailed`; from i686 it loads. `check_bitness`
now refuses a cross-bitness direct injection with `BitnessMismatch`.

Owner decision (2026-09-27): the launcher stays **x86_64**. It injects through
`sgw-start32.exe`, an i686 bin in its own crate `cimmeria-start32` (it links only
`cimmeria-client-launch`; the contract is `cimmeria_client_launch::start32` + the
client-launch README), embedded in the launcher via `CIMMERIA_START32_EXE` and kept at
one stable path `<launcher dir>/sgw-start32.exe` (never %TEMP%, for AV exclusions).

- Do NOT retry the "resolve the WOW64 kernel32 LoadLibraryW from x64" idea: a
  `CREATE_SUSPENDED` WOW64 process has no 32-bit modules yet
  (`EnumProcessModulesEx(LIST_MODULES_32BIT)` returns count=0 for 2 s, measured), and a
  thread a 64-bit process creates there runs in 64-bit mode anyway.
- The helper is its own crate because `winres` in a *library* crate emits
  `rustc-link-lib=resource`, which propagates to dependents: any i686 build linking
  client-launch would inherit the helper's VERSIONINFO and collide with its own.
- `cimmeria-client-launch` + `cimmeria-start32` are hakari-excluded so the i686
  build does not pull reqwest's aws-lc-rs (needs NASM on i686).
- The release build is staged in `tools/launcher-release/build.sh`; launcher-build.yml
  dry-runs it because launcher-release.yml only runs on a release.
- The helper tests need `CIMMERIA_TEST_START32`; they fail when `CI` is set without
  it (non-vacuous), skip locally.
- `cimmeria-lab` (64-bit) and the telemetry DLL launch should call the same helper;
  tracked in issue #985 (the lab bridge was never injected by the supervisor).
  Fixed 2026-09-28: `cimmeria-lab` `process::launch` runs the helper
  (`CIMMERIA_LAB_START32`, else beside the exe). A real-process test needs a
  target that stays up long enough for `RunningProcess::open`: a copied
  SysWOW64 exe without its `en-US\<name>.mui` exits at once (OpenProcess
  error 87); `winver.exe` + its MUI copied as `SGW.exe.mui` works. The old
  same-bitness path leaves the target SUSPENDED on failure: kill it after a
  revert proof.
- A 32-bit exe whose name contains patch/setup/install/update trips UAC installer
  detection (os error 740); the helper name avoids them and embeds `asInvoker`.

**Why:** before this, no launcher build ever injected anything into SGW.exe.
**How to apply:** any new injector from a 64-bit process goes through `start32::run`;
test it with the real WOW64 helper tests. See [[injected-dll-unwind-and-lua-error-rules]],
[[i686-test-exe-uac-installer-detection]].
