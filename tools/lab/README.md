# Lab tooling

Scripts that install and run the Live Research Lab: the `cimmeria-lab` supervisor and its shared daemon. The `lab` command itself is `lab.exe`, written in Rust ([`crates/lab-cli`](../../crates/lab-cli/)); `install.ps1` builds it and installs it to `%LOCALAPPDATA%\cimmeria-lab\bin\` (no shim), and the scripts below are the ones it still runs.

| Entry point | What it does |
|---|---|
| `install.ps1` | Build the lab binaries from a worktree through the build lane and install them (`-DryRun` changes nothing) |
| `daemon.ps1` | Run the shared supervisor (`cimmeria-lab --http`) as the `CimmeriaLabDaemon` scheduled task: `install`, `start`, `stop`, `restart`, `status`, `uninstall` |
| `cli/` | The PowerShell behind `lab install`, `lab setup` and `lab local-setup`: `install.ps1`, `setup.ps1`, `local-setup.ps1`, their `*-lib.ps1` and `common.ps1`. `lab setup` copies them, with `daemon.ps1`, `instances.ps1` and `labd-lib.ps1`, to `%LOCALAPPDATA%\cimmeria-lab\cli\`, and `lab.exe` runs them |
| `instances.ps1` | Write the account files for the extra lab clients and report each instance |
| `labd-headers.ps1` | The `.mcp.json` `headersHelper` that prints the daemon's bearer header |

`labd-lib.ps1`, `cli/common.ps1` and `cli/*-lib.ps1` are dot-sourced libraries; `test-*.ps1` are their tests, which `lab.yml` runs. The [tools catalog](../README.md#tools-catalog) lists each script.

Guides: [live-research-lab.md](../../docs/guides/live-research-lab.md) (install, the daemon, the `lab` command, parallel clients) and [automated-uat.md](../../docs/guides/automated-uat.md) (`lab uat`, goldens, calibration).
