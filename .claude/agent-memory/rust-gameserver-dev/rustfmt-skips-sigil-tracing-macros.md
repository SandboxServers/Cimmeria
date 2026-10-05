---
name: rustfmt-skips-sigil-tracing-macros
description: rustfmt leaves a tracing macro untouched when any field uses a % or ? sigil, so scripted field inserts stay on one line.
metadata:
  type: reference
---

rustfmt formats `tracing::info!(a, b = x, "msg")` like a function call only when every argument parses as an expression. A `%x` or `?x` field (or `target:`-style tokens it can't parse) makes it skip the whole call, so `cargo fmt` passes while a scripted insert like `entity_id, entity_name = ...,` stays on one line, and a single-line call can stay over 100 chars.

**How to apply:** after a scripted field sweep (NT-27, 2026-10-04), reflow the calls you touched to one field per line yourself (keep `// nt:id-only` comments on their field's line), then run `cargo fmt`. Related: [[rustfmt-trailing-line-comment-quirk]], [[shared-scratchpad-name-collisions]] (prefix scratch scripts; a sibling replaced a generic `ed.py` mid-sweep).
