---
name: rustfmt trailing line-comment continuation
description: When a statement has a trailing `// ...` comment and the next line is also `// ...`, rustfmt re-indents the second comment to align with the first, even if the second comment is logically attached to the next statement.
metadata:
  type: feedback
---

## What happens

```rust
buf.extend_from_slice(&0_u32.to_le_bytes()); // border_size
// cs, ch, bmin, bmax — pad with zeros.
buf.extend_from_slice(&[0u8; 4 * 8]);
```

`cargo fmt` rewrites to:

```rust
buf.extend_from_slice(&0_u32.to_le_bytes()); // border_size
                                             // cs, ch, bmin, bmax — pad with zeros.
buf.extend_from_slice(&[0u8; 4 * 8]);
```

The block comment gets sucked into the trailing-comment column of the previous statement. Looks awful and breaks readability.

## Fix

Insert a blank line between the trailing line-comment and the next standalone comment:

```rust
buf.extend_from_slice(&0_u32.to_le_bytes()); // border_size

// Pad zeros for cs, ch, bmin, bmax.
buf.extend_from_slice(&[0u8; 4 * 8]);
```

Or move the explanatory comment to *before* the trailing-comment statement so they're not adjacent.

**Why this matters:** I hit this on PR #426 (navmesh-extractor) — the test buffer-construction code has lots of `extend_from_slice` calls with trailing `// nverts`, `// npolys`, etc. labels. Any standalone block comment after one of those gets pulled into the alignment column.

**How to apply:** when writing tests that build up byte buffers with field-label comments on each `extend_from_slice` line, separate explanatory block comments from the labelled lines with a blank line. Catch in CI's `cargo fmt -- --check`.
