# Lab tooling

Scripts that install, run and drive the Live Research Lab: the `cimmeria-lab` supervisor, its shared daemon, and the `lab` command.

| Entry point | What it does |
|---|---|
| `install.ps1` | Build the lab binaries from a worktree through the build lane and install them (`-DryRun` changes nothing) |
| `daemon.ps1` | Run the shared supervisor (`cimmeria-lab --http`) as the `CimmeriaLabDaemon` scheduled task: `install`, `start`, `stop`, `restart`, `status`, `uninstall` |
| `cli/` | The PowerShell the `lab` command still runs: `install.ps1`, `setup.ps1`, `local-setup.ps1`, their `*-lib.ps1` and `common.ps1`; `lab.exe` (`crates/lab-cli/`) is the command itself |
| `instances.ps1` | Write the account files for the extra lab clients and report each instance |
| `labd-headers.ps1` | The `.mcp.json` `headersHelper` that prints the daemon's bearer header |

`labd-lib.ps1`, `cli/common.ps1` and `cli/*-lib.ps1` are dot-sourced libraries; `test-*.ps1` are their tests, which `lab.yml` runs. The [tools catalog](../README.md#tools-catalog) lists each script.

Guides: [live-research-lab.md](../../docs/guides/live-research-lab.md) (install, the daemon, the `lab` command, parallel clients) and [automated-uat.md](../../docs/guides/automated-uat.md) (`lab uat`, goldens, calibration).
