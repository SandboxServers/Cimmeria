---
name: dll-boot-testhost-traps
description: Traps met building the sgw-testhost DLL boot harness - console children inherit a piped stdout (start32::run blocks), staged DLLs go stale, hard-coded site counts drift, the lane target dir can be swept
metadata:
  type: reference
---

Found 2026-09-28 building `crates/sgw-testhost` (DLL boot tests via
`sgw-start32`).

- **A console-subsystem child keeps its parent's stdout pipe open.** Windows
  copies a console parent's std handles into a console child even with
  `bInheritHandles=FALSE`. `start32::run` reads the helper with
  `Command::output()` (waits for EOF), so with a console target it returns
  only when the *target* exits: measured 4086 ms piped vs 50 ms redirected
  to a file. SGW.exe is GUI subsystem (PE subsystem 2), so the launcher is
  fine; the test host is `#![windows_subsystem = "windows"]` for this reason.
  Symptom in a test: "connected then RST" / "connection refused" to a
  listener the DLL log says is up (the host had already exited).
- **The tests load the staged copies** (`tools/testhost/stage.sh` into
  `<target>/testhost`), not what cargo just built. Restage after any DLL
  change, or a revert proof tests the old DLL.
- **Don't hard-code a DLL's fingerprint site count** in a test; main added
  patches send-side sites (4 -> 8) within a day. The harness dev-depends
  (Windows-only) on both DLL rlibs and reads `fingerprint::SITES` /
  `CODE_SITES + SLOT_SITES`.
- **Host clippy of a crate that dev-depends on the telemetry rlib** compiles
  telemetry for x86_64 with `-D warnings`, which exposed x86-only dead code
  that the i686-only CI never saw.
- `B:\targets\<worktree>\` subdirs (the i686 tree, `testhost/`) can vanish
  between runs (a sweep on the shared Dev Drive); a "no staged build" skip
  or panic means restage, not a code bug.

Related: [[telemetry-anchor-audit-and-hookgate]].
