---
name: lab-cli-json-capture-and-parallel-stubs
description: test-uat.ps1 traps - uat.ps1 -Json writes via [Console]::Out (capture with SetOut), and stubs never reach the ForEach-Object -Parallel lanes
metadata:
  type: reference
---

`tools/lab/cli/uat.ps1` prints its `-Json` object with `[Console]::Out.WriteLine`, so a test that dot-sources it and calls `Invoke-LabUat` cannot read that output from the pipeline. Swap in a `[IO.StringWriter]` with `[Console]::SetOut(...)`, restore it in `finally`, then `ConvertFrom-Json`. This is `Invoke-UatChaosCall` in `test-uat.ps1` (lab-chaos CH-08, 2026-10-11).

Functions stubbed in the dot-sourcing scope (`Invoke-McpTool`, `New-McpSession`) cover the pre-flight and plan call only. The batch lanes run in `ForEach-Object -Parallel` runspaces that re-dot-source `common.ps1`/`uat-lib.ps1`, so the stubs are not there. To keep a real-run test from driving anything, make the stubbed plan call throw (exit 3), and test lane-side logic as uat-lib functions instead.

**How to apply:** CLI tests of `lab uat` paths past the plan go through uat-lib helpers, not through `Invoke-LabUat`. See also [[lab-calibrate-sweep-test-seams]].
