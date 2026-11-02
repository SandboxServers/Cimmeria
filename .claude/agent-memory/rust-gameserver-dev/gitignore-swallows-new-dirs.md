---
name: gitignore-swallows-new-dirs
description: Repo .gitignore has unanchored directory rules that silently hide new source directories from git add; verify with git status --ignored after any file-to-directory split
metadata:
  type: project
---

Splitting `foo.rs` into `foo/mod.rs` can silently produce an **untracked,
unstaged, never-pushed** directory, because the repo `.gitignore` carries
unanchored directory rules that match at any depth.

The one that bit CA04 was a bare `server/` rule, intended for the
root-level PostgreSQL runtime data directory. It matched
`crates/services/src/minigame/server/` and hid a whole split from
`git add -A`. That rule is gone from the current `.gitignore`, but generic
directory names can hit any unanchored rule again, so check explicitly.

**How to apply:** after any `foo.rs` → `foo/` split, or any new source
directory whose basename is a generic word (`server`, `logs`, `data`,
`build`, `dist`, `target`, `bin`, `temp`), run:

```bash
git status --short --ignored <path> | grep '^!!'
git check-ignore -v <path>/mod.rs
```

`git check-ignore -v` prints the offending `.gitignore` line and number.
A clean `git status --short` is NOT sufficient — an ignored directory does
not appear there at all, so the split looks committed when it is not.

These are Git Bash commands; in PowerShell, use `Select-String` in place of
`grep`. Before broadening or anchoring a shared rule, confirm what it currently
catches. If the only hit is your new directory, anchoring is safe.

Related: [[build-environment]].

INVALIDATED WHEN: never; the method holds while .gitignore has unanchored directory rules
checked: 2026-11-01
