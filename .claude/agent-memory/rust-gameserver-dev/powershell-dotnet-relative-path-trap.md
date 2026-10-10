---
name: powershell-dotnet-relative-path-trap
description: [IO.File]::* with a relative path resolves against the process cwd (the main checkout), not the PowerShell `cd` location, so a worktree edit lands in main
metadata:
  type: feedback
---

In the PowerShell tool, `cd <worktree>; [IO.File]::WriteAllText('crates/README.md', ...)`
writes the **main checkout's** file: .NET resolves relative paths against
`[Environment]::CurrentDirectory`, which `cd`/`Set-Location` does not change. Happened
2026-10-10 (caught by `git status` in the worktree showing no change; reverted in main).

**Why:** other sessions work in the main checkout; a stray write there is someone else's dirty tree.
**How to apply:** always pass absolute paths to `[IO.File]`/`[IO.Path]` calls, or use the
Edit tool. After any scripted edit, `git status` the intended worktree to confirm it landed.
See [[concurrent-claude-sessions]].
