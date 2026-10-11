---
name: powershell-ctrl-c-test-pipeline-stopped
description: Testing Ctrl+C in a PowerShell lab CLI - throw PipelineStoppedException from a stub in a child pwsh; no catch sees it, only finally runs, and it ends the whole script
metadata:
  type: reference
---

Found on SV-12's review fixes (lab-spec-vocab, 2026-10-11), `tools/lab/cli/test-uat.ps1`.

- `throw [System.Management.Automation.PipelineStoppedException]::new()` behaves like Ctrl+C: no `catch` runs, not even `catch [PipelineStoppedException]`, every `finally` on the way out runs, and the whole script stops silently (exit 0, nothing after it executes, including the caller's own try/catch).
- So a function that wraps a call in `try { } catch { record the failure }` does not swallow Ctrl+C; there is no need to rethrow it.
- To test "Ctrl+C mid-loop still writes the summary": run the code in a child `pwsh -NoProfile -File <temp script>` whose stub throws it on the Nth call, then read the files the `finally` wrote. In-process, the throw ends the test script itself.
- A generic `throw` from a stub is not a substitute: the code under test catches it as an ordinary call failure.

Related: [[powershell-strict-mode-empty-index]].
