---
name: reference_powershell_alias_traps
description: PowerShell helper functions named like built-in aliases (R, Rd, Rm, Cat...) never run; the alias wins, and `Rd` deleted files during a doc sweep
metadata:
  type: reference
---

In PowerShell an alias resolves before a function of the same name. During the
LX-24 memory sweep (2026-10-11) a helper named `Rd` (read a file) was really
`Remove-Item` and deleted every note it was handed; one named `R` was
`Invoke-History` and failed with "A positional parameter cannot be found". The
deletions were unstaged, so `git checkout -- $(git ls-files --deleted)` restored
them losslessly.

**How to apply:** give scripted edit helpers verb-noun names that are not aliases
(`Get-NoteText`, `Edit-NoteText`), pass `-ErrorAction Stop`, and run
`git status --short` before committing a scripted doc sweep. Check a short name
with `Get-Alias <name>` first. Other sweep traps: [[reference_build_rule_copies]].
