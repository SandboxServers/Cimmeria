---
name: interact-range-and-logcapture-traps
description: interact_target_in_range had no same-space check until AT-04; SpaceManager::get_entity searches every space; LogCapture cargo-test flake fixed by a global always-interested subscriber (#891)
metadata:
  type: project
---

- `SpaceManager::get_entity` looks up an id across ALL spaces, and positions are per-space coordinates. So any "is X near Y" check that compares only positions is fooled by a target in another space at nearby coordinates. `interact_target_in_range` had exactly this hole until AT-04 (2026-09-26) added a `get_entity_space_id` comparison. A new proximity gate needs the same check.
- LogCapture-based tests used to fail under plain `cargo test` (one process, many threads) because `tracing` caches callsite interest process-wide: a callsite first hit on a capture-less thread could be cached `never`. Fixed 2026-09-28 (#891): `LogCapture::install` sets a once-per-process always-interested global `Registry` and calls `rebuild_interest_cache()`. Measured on `cimmeria-cell-combat --lib`: 28/40 runs failed before, 0/40 after. If a LogCapture test flakes again under `cargo test`, check whether that test binary sets its own global subscriber first (then the fix is off for it).

**Why:** both cost a debugging round in AT-04.
**How to apply:** when you write a range or proximity gate, compare spaces. When a LogCapture assertion fails only under `cargo test`, rerun with nextest before treating it as a regression.
