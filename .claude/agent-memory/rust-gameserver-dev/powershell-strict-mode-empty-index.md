---
name: powershell-strict-mode-empty-index
description: Lab CLI PowerShell under Set-StrictMode -Version Latest - `@(...)[0]` on an empty list throws, so pick with Select-Object -First 1; test scratch git repos lose files to add -A/reset --hard
metadata:
  type: reference
---

Found writing `lab calibrate` / `lab uat -Heal` (SV-12, 2026-10-11).

- `Set-StrictMode -Version Latest` (every `tools/lab/cli` script) makes an out-of-bounds index throw: `@($list | Where-Object {...})[0]` on no match is an exception, not `$null`. Inside a `try` it surfaces as a misleading error ("Index was outside the bounds"). Use `... | Select-Object -First 1`.
- `Invoke-McpTool` (`uat-lib.ps1`) already throws `"<tool> failed: ..."`; prefixing again doubles it. Only a timeout lacks the name.
- A scratch git repo in a test: keep fixture files (diffs) outside the repo folder; `git add -A` then `git reset --hard` deletes them.
- `git apply` of an LF diff onto a CRLF worktree file under `core.autocrlf=true` works (apply renormalizes), so a calibrate diff applies to this checkout.

Related: [[reference_lab_cli_powershell_traps_2026_10_10]].
