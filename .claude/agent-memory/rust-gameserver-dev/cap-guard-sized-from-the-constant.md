---
name: cap-guard-sized-from-the-constant
description: A size-cap regression test whose payload is built from the cap constant (CAP + 1) still passes when the constant is loosened; size the payload with a literal.
metadata:
  type: feedback
---

A guard for "input over the cap is refused" that builds its input as
`vec![..; MAX_X as usize + 1]` follows the constant: raise `MAX_X` back to the
old, looser value and the test sends a bigger payload and still passes. Found
2026-10-10 by mutation on the telemetry chunk decompression cap (the mutation
256 MiB passed until the payload became a literal 9 MiB).

**Why:** TESTING.md requires a regression guard to fail with the fix
reverted; a cap fix is often "the constant got smaller", which a
constant-sized payload cannot see.

**How to apply:** size the payload with a literal just over the intended cap,
and `assert!(payload.len() > MAX_X)` beside it so a later cap raise fails
loudly. Keep payloads that compress well (blank lines) so the test stays fast.
Related: [[mutation-restore-mtime-trap]].
