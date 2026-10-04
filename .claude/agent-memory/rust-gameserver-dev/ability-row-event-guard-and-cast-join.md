---
name: ability-row-event-guard-and-cast-join
description: Every ability-target log row needs `event` (source-scan guard in server logging tests); base client_sent joins to a cast by payload fields, not cast_id, because EntityMethodCall has ~400 constructors
metadata:
  type: project
---

Learned 2026-10-04 fixing colo smoke-test telemetry gaps (Heal Focus cast).

**Every row under `abilities`, `abilities.*`, `vitals`, `base.entity_method`
must carry `event = ...`.** `crates/server/src/logging/abilities_event_field_tests.rs`
scans `IN_PROCESS_CRATES` source and fails on a bare one (42 were bare before).
A `LogCapture` over a cast cannot catch rows the fixture never reaches, so
the source scan is the real guard.

**Rows logged before the launch mints `effect_seq` have no cast_id.** The
cast scope (`enter_cast_scope`) opens only at fire; launch-side rows must take
the id explicitly or be logged after `next_effect_id()` in `handle.rs`.
Anything at fire can read `space_mgr.current_cast_id()`.

**Do not add a field to `CellToBaseMsg::EntityMethodCall`** for telemetry:
~416 struct-literal constructors. The base's `client_sent` row instead decodes
the payload (`base-world-entry` `cell_dispatch::method_join`) under the same
field names the cell's `wire_sent` row uses; `complete_at` (same f32 widened
to f64 on both sides) disambiguates two presses of one ability.

Related: [[observability-test-and-throttle-traps]], [[tracing-span-fields-not-on-log-records]].
