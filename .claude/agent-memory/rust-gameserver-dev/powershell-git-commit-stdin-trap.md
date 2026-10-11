---
name: powershell-git-commit-stdin-trap
description: In the PowerShell tool, `git commit -F - @'...'@` does not pipe the here-string to stdin; git treats it as a pathspec and commits nothing
metadata:
  type: feedback
---

`git commit -q -F - @'<msg>'@` in the PowerShell tool fails with "pathspec '<msg>' did not match any file(s)": the here-string is passed as an argument, not stdin. Nothing is committed (the `git add` before it still stages).

**Why:** hit on FX-02 (2026-10-10) while committing a long message; cost one retry.

**How to apply:** write the message to a file in the scratchpad and `git commit -F <path>`, or use `git commit -m @'...'@` (the documented PowerShell form). Related: [[worktree-shell-and-external-binary-tests]].
