---
name: lab-cli-child-test-daemon-isolation
description: tools/lab command tests in a child pwsh can still reach the real lab daemon - the token is a User env var; isolate with -StatusFile seams plus a closed-port labd.pid
metadata:
  type: project
---

`Get-LabStatus` (tools/lab/cli/common.ps1) reads `CIMMERIA_LAB_DAEMON_TOKEN` from the **User** environment scope first, and the bind from `labd.pid` (default `127.0.0.1:8779`). Pointing `CIMMERIA_LAB_HOME` at a temp folder therefore does not stop a child `pwsh -File <command>.ps1` from querying the workstation's running daemon.

**Why:** found writing LW-04 (`lab watch`) command tests, 2026-10-11; a test hitting the real daemon is flaky and breaks the "no packet uses the lab" rule.

**How to apply:** in command tests, pass the command's file seam (`-StatusFile`) for every non-usage case, and also write `labd.pid` = `{"pid":1,"bind":"127.0.0.1:9"}` into the temp home (test-ops.ps1 does the same). `Get-UatRoot` also honours `$env:CIMMERIA_LAB_UAT_DIR`, so set it to an empty temp folder for "no batch" cases. A live-loop command run with redirected stdout should be tested via `Start-Process -PassThru` + `WaitForExit(20000)`, never a bare capture that can hang CI. See [[lab-cli-powershell-traps]].

**Revert proofs are the dangerous case** (2026-10-11, CH-08 review): a "exits 2 before the daemon is asked" check run as a child `pwsh -File uat.ps1 ... -Leases 2` was harmless while the guard held, but with the guard reverted the child planned and started real `lab_uat_run`s on two lab instances (stopped by a lease takeover). Write such checks dot-sourced with every daemon function (`Get-LabToken`, `Get-LabStatus`, `Get-LabdEnv`, `Get-LabMcpUrl`, `New-McpSession`) stubbed to throw, and run revert proofs under `Start-Process` + `WaitForExit` with the original saved to the scratchpad first (a killed shell skips `finally`). After an escaped run: `lab_lease_status`, then take over and release its lease.
