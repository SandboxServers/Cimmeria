---
name: revert-proof-mutation-must-be-confirmed
description: A scripted revert proof whose mutation silently fails runs the unmutated tests and reports "ok"; confirm each mutation applied, and never split old/new on "=>" in Rust
metadata:
  type: feedback
---

A scripted revert proof (mutate, run the guard, restore) reports every guard as "passing under the mutation" when the mutation step itself failed, because the tests then run against unmutated code. Seen in SS-C3 (2026-09-27): a Python heredoc inside a bash script lost its `\n` escapes, the mutator died with a SyntaxError, and all seven proofs printed `ok`. A second trap: splitting the `old=>new` mutation argument on `=>` cuts inside any Rust match arm (`X => {`), producing "unclosed delimiter" instead of a mutation.

**Why:** a proof that never mutated anything looks exactly like a guard that does not guard, and it is easy to misread as the latter (or, worse, to skip reading it).

**How to apply:** keep the mutator in its own `.py` file (not a heredoc), have it print `mutated <file>` and exit non-zero on "not found", abort the proof when it fails, and use a separator that cannot occur in Rust (`|||`). Expect FAILED lines; an all-`ok` proof run means check the mutation first. Restore with `git checkout HEAD -- <file>` and `touch` it ([[ai-state-private-and-revert-proof-mtime]]).
