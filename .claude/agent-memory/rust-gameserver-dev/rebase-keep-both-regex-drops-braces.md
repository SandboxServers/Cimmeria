---
name: rebase-keep-both-regex-drops-braces
description: Resolving append-at-end conflicts with a scripted "keep both sides" regex can drop a closing brace or duplicate a doc fragment; read the result before continuing the rebase.
metadata:
  type: feedback
---

When two parallel packets both append to the same list (`mod` lines, fixture constructors, doc paragraphs) and the rebase conflicts, a regex that concatenates `ours + theirs` looks safe but is not. Seen 2026-09-27 on ORG-08 over ORG-09 in `handlers/tests/mod.rs`: the conflict hunk started inside ORG-09's `fn org09` body, so the concatenation lost its closing `}` and duplicated a half-sentence of the module doc.

**Why:** git picks hunk boundaries by line similarity, not by syntax, so "theirs" can begin mid-item.

**How to apply:**
- After any scripted resolution, `sed -n` the whole merged region and `cargo check` before `git rebase --continue`.
- For docs with long single-line table rows (`observability.md`), take `--ours` (main) and re-apply your edit script with the new anchor. Don't merge the row text.

Related: [[stacked-branch-rebase-traps]], [[python-write-mangles-utf8-and-crlf]].
