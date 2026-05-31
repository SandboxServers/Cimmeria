---
name: read-wstring-offset-semantic
description: read_wstring returns BYTES CONSUMED, not the new absolute offset — always use += not =
metadata:
  type: feedback
---

`crates/services/src/mercury/mod.rs::read_wstring(buf, offset) -> Result<(String, usize), _>` returns `(decoded_string, bytes_consumed)`. The second tuple element is the number of bytes the WSTRING occupied — `4 + char_count*2` — NOT the new absolute offset into `buf`.

Correct pattern when chaining multiple `read_wstring` calls:

```rust
let mut offset = 1;  // e.g., after a 1-byte channel tag
let (target, n) = read_wstring(payload, offset)?;
offset += n;          // accumulate
let (text, _) = read_wstring(payload, offset)?;
```

Incorrect pattern (was the bug in `base/dispatch.rs::SEND_PLAYER_COMMUNICATION` before #425):

```rust
offset = new_offset;  // drops every byte before this WSTRING from the running offset
```

**Why:** Wired #65 (speaker_flags) tests revealed dispatch never reached the cell-forward branch — the second `read_wstring` was reading from offset 4 instead of 5 (channel byte forgotten), saw garbage where the text WSTRING length should have been, and silently returned Ok(()). Pre-existing bug, no test exercised it before.

**How to apply:** Audit any future `read_wstring` chains in the codebase. If you see `offset = result.1` followed by another `read_wstring(buf, offset)`, that's the bug shape — fix to `+=`. The function signature could be tightened (return the new absolute offset instead of bytes-consumed) but changing semantics would be a wider refactor; for now, just use `+=`.
