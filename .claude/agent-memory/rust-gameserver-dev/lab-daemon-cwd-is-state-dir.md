---
name: lab-daemon-cwd-is-state-dir
description: The installed lab daemon runs from %LOCALAPPDATA%\cimmeria-lab (exe in labd\, cwd = state dir), so any "walk up to tools/lab/" lookup is inert there; check the state dir first.
metadata:
  type: project
---

The installed lab daemon (`tools/lab/daemon.ps1`) runs its own exe copy from
`%LOCALAPPDATA%\cimmeria-lab\labd\` with the state dir as its working
directory. A config lookup that walks up from cwd or the exe to find a
checkout's `tools/lab/<file>` never finds anything in that deployment.

**Why:** CL-13's ini overrides first shipped with only the walk-up and were
inert under the installed daemon (review finding, 2026-10-11).

**How to apply:** for any new lab supervisor config file, search
`CIMMERIA_LAB_*` env, then `crate::daemon::config::state_dir()`, then the
checkout walk-up (see `supervisor/ini_overrides.rs::overrides_path_from`).
Env changes reach the daemon via `lab env set` + `lab restart`.
