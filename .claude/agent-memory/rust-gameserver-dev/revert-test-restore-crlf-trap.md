---
name: revert-test-restore-crlf-trap
description: Restoring a file after a revert-verification test with a multi-line Python str.replace silently no-ops on CRLF sources, so the NEXT revert test runs against a still-broken tree
metadata:
  type: feedback
---

When revert-verifying a guard, restore the file with `git checkout HEAD -- <file>`, never
with a scripted string replace — and re-`grep` the restored line before the next test.

**Why:** most `crates/**/*.rs` in this repo are checked out CRLF. A Python
`open(p, newline='')` read returns `\r\n`, so a replacement pattern written with `\n`
spanning two lines finds nothing and `str.replace` returns the string unchanged — no
exception, no diff, no signal. The *revert* direction usually works (single-line patterns
have no `\n` in them) and the *restore* direction silently fails, so the next revert test
runs against a tree that still has the previous revert in it. Observed on Harset H08: two
`auto_cycle_tick` tests failed during the auto-cycle-sweep revert and looked like the sweep
mattered to them, when in fact the `is_auto_cycle_target_valid` gate had never been put
back.

**How to apply:** revert-verification loop is (1) break one thing, (2) run, (3)
`git checkout HEAD -- <file>`, (4) grep the line back, (5) next. If a revert makes *more*
tests fail than the ones you reasoned about, suspect an un-restored earlier revert before
you conclude the guard is broader than you thought. Checkpoint-commit before the loop
starts so `git checkout HEAD --` has something correct to restore from — see
[[revert-verification-loses-uncommitted-fmt]].
