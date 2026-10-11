---
name: reference_desktop_launcher_macos_gating_2026_10_11
description: Desktop launcher macOS-only code first compiles in launcher-desktop.yml's native-engine (macos-latest) job; ship.py gates on it since 2026-10-11; the client_maintenance log-capture flake and its fix
metadata:
  type: reference
---

Launcher consolidation, 2026-10-11.

- **macOS-only code is never compiled on the Windows workstation.** The desktop launcher has `cfg(target_os = "macos")` code (Wine paths, `adopted_tests.rs`, the updater's `native_uat.rs`, `libc::kill` probes). Workers build on Windows, so the first compile is the `native-engine (macos-latest)` job in `launcher-desktop.yml`. Two macOS breaks reached PRs that way in one session: an `unsafe { .. } || ..` statement-expression parse error, and a struct literal missing a newly added field (E0063).
- **`ship.py merge` gates on both `native-engine` jobs** when they ran (`CONDITIONAL`, since this campaign's wave-1 close). Before that they were informational, so a macOS break could merge silently. The job has an explicit `name: native-engine (${{ matrix.os }})`, and `test_every_conditional_check_names_a_real_workflow_job` expands the matrix to check the names stay real.
- **Flake, fixed in LX-13 (#1438):** `client_maintenance::tests::a_successful_reset_logs_what_it_deleted` intermittently captured no logs under `cargo test`. That's the single-dispatcher interest trap (see the rust-gameserver-dev note `tracing-capture-single-dispatcher-trap`). A failure of that one test on a PR branch older than #1438 is the flake, not the PR.
- **`migration::tests::desktop_receipts_and_stale_preferences_are_not_overwritten`** failed once on macOS with `Busy` from the legacy `launcher.lock` `try_lock` (2026-10-11, PR #1414), then passed on rerun. Treat a repeat as a real lock race worth fixing.

Related: [[reference_lab_cli_powershell_traps_2026_10_10]].
