---
name: finding-source-scan-lexer-masking
description: Source-scan tests (NT-03 unpaired-ID scan) mask strings/comments before parsing macro calls; Git Bash `sed -i` silently converts CRLF docs to LF
metadata:
  type: project
---

The NT-03 unpaired-ID scan (`crates/server/src/logging/unpaired_id_tests/`) masks comments and
string/char literals to spaces (offsets and newlines kept) before finding `info!(` etc. Without
that, doc-comment examples and `")"` inside messages break call extraction. Mutation-checked
2026-10-04: six reverts (pairing, exemption, reason check, exception table, span filter, message
break) each fail a named fixture test. First baseline: 6,400 unpaired of 6,456 ID fields, 2,674
calls; `player_id`+`account_id` (Rule 5 identity) are ~30%.

Gotcha: in Git Bash, `sed -i` on a CRLF `docs/**/*.md` or `.rs` file rewrote it LF. Restore with a
python `newline=''` round-trip; `git diff --stat` hides it because autocrlf normalizes, `file` shows it.

**Why:** a future scan reviewer needs to know the masking is load-bearing; the sed gotcha cost a fix-up.
**How to apply:** when reviewing or extending any source-scan test, check it masks literals; edit CRLF
docs with a newline-preserving tool, then confirm with `file`. See [[workflow-revert-audit]].
