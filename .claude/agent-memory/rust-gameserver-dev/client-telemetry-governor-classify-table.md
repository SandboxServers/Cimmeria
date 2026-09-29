---
name: client-telemetry-governor-classify-table
description: Every DLL event passes the governor (queue::governed_channel); a new hook's target defaults to Budgeted unless it gets a row in governor/classify.rs RULES; server priority list mirrors the must-keep families
metadata:
  type: project
---

Since 2026-09-29 (PR for the owner's "throttle but keep fidelity" direction)
`Producer::try_emit` in `crates/client-telemetry/src/queue.rs` admits every
event through `governor::Governor` before it takes a ring slot.

- The only place that branches on target names is `RULES` in
  `crates/client-telemetry/src/governor/classify.rs`. A new hook whose target
  has no row is **Budgeted** (burst 20, 2/s, then rolled up). A new
  high-rate hook needs a `hot(...)` row. A new must-keep family needs a
  `keep(...)` row **and** the matching prefix in `PRIORITY_PREFIXES` in
  `crates/admin-api/src/routes/telemetry/session_budget.rs`, or the
  server-side runaway guard can cut what the client promised to keep.
- warn/error level and non-happy outcome fields override every row.
- `no_rule_is_shadowed_by_an_earlier_one` fails if a new prefix row swallows
  a later exact row. Put specific rows first.
- Conservation invariant (volume test): rows + `repeat_count` + rollup
  `count` = events raised, per target. A change that absorbs events without
  counting them breaks `the_measured_mix_shrinks_by_90_percent...`.
- `queue::channel()` (ungoverned) still exists for tests and any caller that
  wants the raw path. `boot.rs` uses `governed_channel`.

**Why:** 86% of an hour's rows were `client.engine.sequence_tick`, and before
the governor, a hot burst could fill the ring and push out entity-lifecycle
events. Ring drops were also counted but never reported. The
`client.telemetry.health` event now reports them.

**How to apply:** whenever you add or rename a `client.*` target, check
which class it lands in (`classify::classify`) and add a row plus a test in
`governor/tests/classify.rs` if the default is wrong.
Related: [[client-telemetry-index-and-upload-traps]].
