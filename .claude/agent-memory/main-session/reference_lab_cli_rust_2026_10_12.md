---
name: reference_lab_cli_rust_2026_10_12
description: lab.exe (the Rust lab command, crates/lab-cli) as built 2026-10-12 - install layout, no fallback except the seven run_script wrappers, flag rewriting, tests isolate CIMMERIA_LAB_HOME, JSON by Serialize structs
metadata:
  type: reference
---

The `lab` command is `lab.exe`, crate `cimmeria-lab-cli` in `crates/lab-cli` (campaign `docs/analysis/lab-cli-rust/`, 2026-10-11 to 2026-10-12, packets LX-01 to LX-19; LX-20, the live check on the workstation, is owed). Operating guide: `docs/guides/live-research-lab.md` ("The `lab` command").

- **Layout.** `lab.exe` is `%LOCALAPPDATA%\cimmeria-lab\bin\lab.exe`, next to `cimmeria-lab.exe`, copied by `tools/lab/install.ps1` (and `lab install --from <worktree>`). `lab setup` copies the PowerShell it still runs to `cli\` (`install`, `setup`, `local-setup` and their libraries, with `daemon.ps1`, `instances.ps1`, `labd-lib.ps1`). There is no `lab.cmd` shim (setup deletes an old one) and no `lab.ps1`. `CIMMERIA_LAB_CLI_DIR` overrides the scripts folder; `CIMMERIA_LAB_HOME` overrides the lab home and is for tests only (`daemon.ps1` ignores it).
- **No fallback.** The strangler fallback to a PowerShell dispatcher ended at LX-18. Only `start`, `stop`, `restart`, `instances`, `install`, `setup` and `local-setup` start PowerShell, through `fallback::run_script`: `--runs-per-lease` becomes `-RunsPerLease`, `--value=-x` becomes `-Value:-x`, an empty `--x=` becomes `-X` plus an empty token, tokens after a bare `--` stay; the script's exit code passes through (255 when it has none); a missing `pwsh` or script is exit 1 with a message.
- **Flag rewriting.** `argv::rewrite` turns PowerShell spellings (`-Rows`, `-PlanOnly`, `-Value:-x`) into clap's `--rows`, `--plan-only`, `--value=-x` before parsing, so old briefs keep working. Every `lab ...` line in a help page's Examples must parse (a test).
- **Plain mode.** Colour and glyphs need a terminal, `NO_COLOR` unset or empty, and `TERM` not `dumb`; otherwise output is the PowerShell command's plain lines. `--json` and exit codes do not depend on it.
- **Tests never reach a real daemon.** Any test that runs the binary uses `tests/support`'s `isolated_lab_home()`: `CIMMERIA_LAB_HOME` set, the token read from the process environment (never HKCU), a `labd.pid` that names a closed port. End-to-end `uat` tests run against a scripted stub daemon.
- **JSON.** Objects the CLI builds are `#[derive(Serialize)]` structs in PowerShell's key order, because the workspace `serde_json` has no `preserve_order`; daemon answers pass through as `Value` (already sorted). Strings escape U+0085, U+2028 and U+2029 as `ConvertTo-Json -Compress` does (a custom `Formatter`).
- **CI.** A new workspace crate must be classified in `crates/server/src/logging/target_scan_tests.rs` or the merge queue fails.

Open: the LX-20 live check; the owner's confirmation of D-LX1; one PR from `lab-cli-rust/integration` to `main`. Related: [[reference_lab_cli_powershell_traps_2026_10_10]].
