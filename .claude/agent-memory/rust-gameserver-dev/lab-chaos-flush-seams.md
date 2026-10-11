---
name: lab-chaos-flush-seams
description: Lab-chaos flush modes (CH-07) - tests cannot install the lab_chaos registry, phase-1 record/cursor arithmetic, where the flush shape lives
metadata:
  type: project
---

- Only `crates/server/src/main.rs` may call `lab_chaos::install`, and only
  allowlisted files may name `lab_chaos::current` (`lab_chaos_callers`
  source scan, comments included). A crate's tests therefore lend their own
  `LabChaos::new()` through a `cfg(test)` thread-local seam in the reading
  file (`flush_chaos::lend_registry`); `#[tokio::test]` runs on the test
  thread, so the seam reaches the flush. Never write the `current` needle in
  a test file or doc comment outside the allowlist.
- Phase-1 record = CREATE_ENTITY (11) + UPDATE_AVATAR (26) = 37 bytes. The
  client's bundle cursor is body offset + 1, so record j's create is at
  cursor `1 + L + 37j`, its avatar update at `12 + L + 37j` (L = filler).
- Fillers: `02 0A` (2), voiceData `33 nn nn`+zeros (3..=36), both (37, 38).
  No 1-byte message exists; residue L=1 uses L=38 on record j-1.
- The phase bundle send lives in `cell_dispatch/intro_bundles.rs` with a
  `Phase1Shape` (default = ordinary flush); chaos only changes the shape.
- Never sleep in a flush: the base receive loop awaits `onClientReady` /
  `cancelMovie` inline, so a sleep stalls every session. `Delay` is a
  cinematic AoI hold (`cinematic_aoi_hold/chaos_delay.rs`), begun under the
  lock that opens the pre-ready gate, drained from a spawned timer. With
  `start_paused`, `elapsed() == ZERO` after the handler proves no wait.
- Pinning "unchanged" bytes: `git restore --source=<old sha> -- <file>`,
  run a capture test through the lane, `git checkout -- <file>` back, touch
  it; hash decrypted packets (FNV-1a) with their lengths. See
  `tests_flush_default_bytes.rs`.
- The filler messages are one switch, `flush_chaos::FILLER_POLICY`.

**Why:** CH-07 cost a design detour on each of these.
**How to apply:** any later lab-chaos packet touching the flush or adding
a registry reader. See [[testing-patterns-index]].
