---
name: revert-proof-commit-first
description: Commit before a regression-proof revert run; restoring with `git checkout -- <dir>` also discards every uncommitted edit in that dir, not just the reverts
metadata:
  type: feedback
---

Before applying the reverts for a regression proof, commit the work (a WIP commit is fine), then restore with `git checkout HEAD -- <paths>`.

**Why:** in ORG-03 round 2 (2026-09-27) the telemetry edits were still uncommitted when I restored a revert run with `git checkout -- crates`. That reset every modified file under `crates/`, including about a dozen files of real work; only untracked new files survived. It was recoverable only because every edit had come from a script or a Write call still in the transcript.

**How to apply:** the order for any revert, run and restore cycle is: `git status` clean except the intended work, then commit, then apply the reverts, then run, then `git checkout HEAD -- <paths>`. Also: heredocs through the Bash tool break on some content with apostrophes (`2's`) and truncate silently ("here-document delimited by end-of-file"). Write edit scripts with the Write tool and run them with `python <file>`. Related: [[lane-sh-masks-cargo-exit-code]].
