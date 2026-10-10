---
name: reference_lab_cli_powershell_traps_2026_10_10
description: PowerShell traps the lab CLI campaign hit (2026-10-10) - one-element unroll before splatting, StrictMode $LASTEXITCODE, REG_SZ from SetEnvironmentVariable, squash merges vs ancestor tests, -Value:-x, ship.py --retire "Permission denied"
metadata:
  type: reference
---

Found while building the `lab` command (campaign `docs/analysis/lab-cli/`, PRs #1334 to #1338, 2026-10-10). Each one cost a review round.

- **One-element unroll before a splat.** A dispatcher with no `param()` block splats `$args[1..]` to the command. `$Rest = if (...) { @($args[1..n]) } else { @() }` unrolls a one-element array to a bare string (an expression's output is enumerated), and `& $cmd @Rest` then splats that string one character at a time: `lab instances status` bound `s`, `t`, ... Assign the array directly: `$Rest = @(); if (...) { $Rest = @(...) }`. Splatting the automatic `$args` also keeps `-Lines 5` named, where a `ValueFromRemainingArguments [string[]]` parameter turns it into positional strings.
- **StrictMode and `$LASTEXITCODE`.** Under `Set-StrictMode -Version Latest`, reading `$LASTEXITCODE` before any native command ran throws. A dispatcher that ends `exit $LASTEXITCODE` must set `$global:LASTEXITCODE = 0` before running the command, so a command that falls off the end (no `exit`, nothing native) reads as success.
- **`[Environment]::SetEnvironmentVariable('Path', ..., 'User')` writes `REG_SZ`.** A `REG_EXPAND_SZ` user `PATH` holding `%VARIABLES%` stops expanding them. Write `HKCU\Environment` through `Microsoft.Win32.Registry` with the existing `GetValueKind`, read with `DoNotExpandEnvironmentNames`, then broadcast `WM_SETTINGCHANGE` ("Environment") yourself (`tools/lab/cli/install-lib.ps1`).
- **Squash merges defeat ancestor checks.** PRs here are squash-merged, so a worktree commit is never an ancestor of `origin/main`, even after its PR merges. "Is the installed copy current?" must compare content: `git diff --quiet <sha> origin/main -- <path>` (`lab doctor`'s last check).
- **A value starting with `-`.** `lab env set KEY -x` binds `-x` as a parameter name. Write `-Value:-x`; the colon form binds it as the value.
- **`ship.py merge --retire` fails with "Permission denied"** when any shell, the coordinator's own included, has its working directory inside the worktree being retired: Windows won't delete a directory a process stands in. Run the merge from the main checkout, and move every shell (and worker) out of the worktree first.

Graduated to `docs/guides/live-research-lab.md` ("The `lab` command") where it is lab behaviour; the PowerShell traps stay here. Related: [[reference_lab_mcp_token_cost_2026_10_10]], [[project_live_research_lab]].
