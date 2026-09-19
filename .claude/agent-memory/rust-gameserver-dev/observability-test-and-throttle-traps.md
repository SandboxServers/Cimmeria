---
name: observability-test-and-throttle-traps
description: Traps when adding/testing telemetry in cell code — counters are unobservable in tests, CWD-relative navmesh paths make lifecycle log branches untestable, create_entity leaves is_player=false, and the LogThrottle/suppressed-count pattern
metadata:
  type: project
---

Learned while landing `feat/navmesh-logging` (navmesh observability,
2026-09-19). All verified against the code at that commit.

**Metric emission cannot be asserted in a unit test.**
`cimmeria_observability`'s macros no-op until `init()` installs a
global Meter, which no test does. So "test the counter labels" means
testing the *label-derivation* — the `unknown` / `n/a` fallback
constants and any `&'static str` label vocabulary — not the emission.
Say so explicitly in a PR report rather than implying coverage.

**Why:** a reviewer asked for counter-label tests and the honest
answer is that only half of it is testable.

**How to apply:** pin label constants with a plain `assert_eq!`, and
where the same value also lands on the log event (e.g. `world`), let
the LogCapture assertion cover it.

---

**A log line inside `create_space_instance` is untestable as written.**
`lifecycle.rs` builds `data/spaces/<world>.nav` relative to the process
CWD; under `cargo test -p cimmeria-services` the CWD is `crates/services`
and the fixture lives at `../../data/spaces/`. The success branch
therefore never fires in a test.

**How to apply:** extract the emission into a free function taking the
already-parsed data (`log_navmesh_loaded(space_id, world, &fingerprint)`)
and call *that* from the test with a `NavMesh::load("../../data/...")`.
Bonus: it also shrinks the diff in a file other sessions are editing.

---

**`create_entity` leaves `is_player = false`.** `connect_entity` is what
stamps it (plus `space.players`). Any test for player-gated behaviour
must call `connect_entity`, and any production gate should check both
signals the way `despawn` does — `entity.is_player || space.players.contains(&id)`.

---

**Throttled-log pattern** (now Pattern D in
`docs/architecture/negative-logging-convention.md`): first occurrence
emits immediately, then <= 1 per window carrying `suppressed = N`; the
**counter increments on every occurrence** including suppressed ones.
Shared primitive is `LogThrottle` in
`crates/services/src/cell/space_manager/movement_telemetry/`, keyed by
entity and released in `destroy_entity`. Use `saturating_duration_since`
— plain `Instant` subtraction panics on a rewound sample.

Two guards are mandatory and the second is the one that gets forgotten:
the burst (N occurrences -> 1 row, then `suppressed = N-1`) and
*independence* (a second entity's first occurrence is not swallowed).
Without per-entity keying a throttle is strictly worse than none.

---

**Borrow ordering in these helpers.** `world_name_for_space` returns a
`&str` borrowed from `&self`, and the throttle needs `&mut self`.
Resolve every immutable lookup into owned values (`.to_string()`,
`.map(str::to_owned)`) *before* the throttle call, or split the `&mut`
into its own one-line method. Same trap at the five `npc_ai` call
sites: hoist `PathFailReason::for_missing_path(space_mgr, id)` into a
`let` before passing `space_mgr` as `&mut`.
