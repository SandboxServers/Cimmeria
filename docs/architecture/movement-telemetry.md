# Movement Telemetry (what the position path reports)

> **Status**: Shipped in PR **#700** (navmesh observability), amended by
> its review follow-up. Split out of
> [movement-validation.md](movement-validation.md), which keeps the
> *decisions*; **nothing in this document changes one**. The validator
> decides what is accepted and what is snapped back; everything here
> reports on that decision after the fact.

## Where it lives

| Concern | Code |
|---|---|
| Per-entity state, the throttle primitive, the load-time mesh line | [`movement_telemetry/mod.rs`](../../crates/services/src/cell/space_manager/movement_telemetry/mod.rs) |
| Every **hard reject**, all three outcomes | [`movement_telemetry/reject.rs`](../../crates/services/src/cell/space_manager/movement_telemetry/reject.rs) |
| The accepted-position sampler | [`movement_telemetry/position_sample.rs`](../../crates/services/src/cell/space_manager/movement_telemetry/position_sample.rs) |
| Dispatch (outcome → report call → snap-back) | [`base_messages/movement.rs`](../../crates/services/src/cell/service/base_messages/movement.rs) |

Emission lives on the `SpaceManager` rather than in the message handler
because the rows need three things only it can answer: the world name
behind a space id, the navmesh diagnosis for the rejected point, and the
per-entity throttle state. Inlining that into the handler would put a
three-step borrow-ordering dance into the middle of a message loop —
three times over, once per outcome, which is exactly how the accounting
came to be applied to only one of them (see below).

## Every hard reject is counted, whichever outcome it produced

`SpaceManager::reject_outcome` resolves one refused client position into
`Rejected`, `Recovered` or `CorrectionSuppressed` (the decision table is
in [movement-validation.md](movement-validation.md#correction-termination-snap-back-recovery)).
Those differ only in *what the client is told*. All three are the same
event: a position the validator refused.

The first cut of this module reported only `Rejected`, because that was
the branch the snap-back lived in. `movement_validation_rejects_total`
therefore under-counted exactly the worst case — a player stuck badly
enough to exhaust the correction budget emits `CorrectionSuppressed` at
the full 10 Hz client rate indefinitely and contributed *nothing* to the
reject rate an operator alerts on, while its ERROR row was the one
unthrottled `movement.validation` stream left.

`SpaceManager::account_hard_reject` is now the single seam: one counter
increment, one diagnosis, one throttle decision. Each outcome's row is a
presentation of that same accounting.

| Row | Level | Extra fields | Client is sent |
|---|---|---|---|
| `movement.validation_reject` | WARN | `last_valid_*`, `bounds_min_*` / `bounds_max_*` | `FORCED_POSITION` to `last_valid` |
| `movement.validation_recovered` | WARN | `from_*`, `recovered_*` | `FORCED_POSITION` to `recovered_to` |
| `movement.correction_suppressed` | ERROR | `from_*`, `strikes` | nothing |

The two outcome-specific counters
(`movement_validation_recoveries_total`,
`movement_validation_corrections_suppressed_total`) are a **breakdown
of** `movement_validation_rejects_total`, not alternatives to it.

## Which rows carry `world`

A space id is a runtime allocation (`(cell_id << 16) | local_index`)
that means nothing outside the process that minted it, so a row carrying
only `space_id` cannot be grouped by zone after the fact. `world` is
also the counter label — see
[instrumentation-discipline.md](instrumentation-discipline.md#ruling-world-is-an-approved-label)
for the cardinality ruling.

**Every `movement.validation` row whose space id resolves carries
`world`**, which is all of them but one:

| Row | `world` | Why |
|---|---|---|
| `validation_reject`, `validation_recovered`, `correction_suppressed` | yes | resolved from the reject's `space_id` |
| `space_mismatch` | yes | the **server's** binding — the claimed id is by definition not one this process can resolve |
| `navmesh_gm_bypass`, `speed_warning` | yes | resolved from the entity's space |
| `entity_missing` | **no** | the entity is in no space at all. That is the whole finding: a stale packet arrived after destroy/disconnect, so there is no binding left to resolve a world from |

A world-filtered dashboard therefore sees everything except
`entity_missing`, which is a post-teardown drop rather than a movement
decision.

## The navmesh diagnosis

`reason = "navmesh"` covers four different bugs with four different
owners. A reject with that reason and nothing else cannot tell a mesh
hole from a player clipped into the floor. `NavMesh::diagnose_point`
([navigation](../../crates/entity/src/navigation/mod.rs)) returns the
same boolean `is_point_valid` does — the latter is a thin wrapper over
the former, so they cannot disagree — plus the gate and the distances:

| `gate` | Meaning | Usual owner |
|---|---|---|
| `no_poly_in_extents` | Neither search phase found any polygon. The point is nowhere near the mesh | **Mesh build** — a hole, or a room the builder never covered |
| `horizontal` | A polygon was found, but the point is > `agent_radius * 2` from it in X/Z | Mesh edge / walkable-surface coverage |
| `below_surface` | Within the horizontal gate, > `agent_radius * 2` **below** the surface | Floor-clip / under-terrain; client physics or a bad authoritative write |
| `above_jump_tolerance` | Within the horizontal gate, > `JUMP_HEIGHT_TOLERANCE` (4.0) **above** the surface | Tolerance calibration, or a genuine fly-hack |

`nav_horiz_dist` and `nav_dy` say by *how much* — the difference
between "widen the tolerance" and "rebuild the mesh". They are absent
exactly when `gate = no_poly_in_extents`, because there was no polygon
to measure against; a `0.0` there would read in a query as "right on
the surface".

The diagnosis always describes **the position the client claimed**, on
all three outcomes, so `gate` / `nav_horiz_dist` / `nav_dy` mean the
same thing wherever they appear. It is computed only for
`reason = "navmesh"`: running it for a bounds or teleport reject would
spend two Detour queries per rejected packet on a field nobody can act
on.

**Two-phase reporting.** `diagnose_point` mirrors `is_point_valid`'s
two-phase search and reports phase 2's polygon when it found one, else
phase 1's. The fallback matters: a point clipped below the surface
pushes phase 2's downward-biased search box past the floor entirely, so
phase 2 finds nothing — and reporting `no_poly_in_extents` there would
mislabel a floor-clip as a mesh hole, which are the two failures an
operator most needs to tell apart.

## Which mesh said so — `navmesh_hash`

`navmesh_hash` names the mesh build the point was judged against: the
8-digit short form of an FNV-1a 64 over the whole `.nav` file. It joins
to the `movement.navmesh` `navmesh_loaded` line, which carries the full
hash, the file size, the header counts and the agent parameters. Without
it, a reject from before a mesh rebuild is indistinguishable from one
after it — exactly the reconstruction the September 2026
Castle_CellBlock rebuild had to do by hand from deploy timestamps.

The fingerprint is computed while the loader streams the file (see
[`navigation/fingerprint.rs`](../../crates/entity/src/navigation/fingerprint.rs)),
and is **stable for a given file forever**: shipped `.bug` rows and
historical reject rows carry it, and a change to how it is computed
severs every one of those joins. `navmesh_hash_is_stable_for_the_shipped_meshes`
pins the value for each mesh in `data/spaces/`; its provenance table is
[data/spaces/README.md](../../data/spaces/README.md).

## Throttling

Pattern D in
[negative-logging-convention.md](negative-logging-convention.md#pattern-d--high-frequency-repeat-throttled-with-a-suppressed-count).
Per entity: the first hard reject emits immediately, then at most one
row per second, carrying `suppressed = N` for the rows elided since the
last emission. The measured need was 146,760 reject rows in three days,
103,818 of them (71%) from one entity in Harset stuck against a wall and
re-reporting at 10 Hz.

Two properties that are easy to break:

- **One window per entity, shared by all three outcomes.** They are one
  event seen three ways, and only one can fire per packet. Giving
  `CorrectionSuppressed` its own stream would reintroduce the flood the
  throttle exists for, at ERROR.
- **The counter is incremented before the throttle decision.**
  Suppressing a log line must not suppress the count, or the throttle
  silently deflates the rate an operator alerts on.

The same primitive backs `npc_ai.path_fail` — see
[`npc_ai/path_failure`](../../crates/services/src/cell/service/npc_ai/path_failure/mod.rs),
whose row additionally carries `fallback` (`direct_waypoint` |
`path_unchanged`) because what the handler *did* about the failure is
not derivable from why it failed.

### State release

State is keyed by entity, and released on **every** teardown path:
`destroy_entity` for the per-entity case, and `destroy_space` for the
NPCs that go away with an instance when its last player leaves —
`destroy_entity` never runs for those. Missing the second path leaked
one slot per NPC per instance for the process lifetime and let a
recycled `entity_id` inherit an open throttle window, silently swallowing
the new occupant's *first* occurrence: the one row an incident timeline
most needs. Anything added to `destroy_entity`'s release block belongs
in `destroy_space` too.

## Accepted positions

`movement.position_sample` (DEBUG) samples **accepted** player
positions at ≤ 1 per player per 5 s, and only after ≥ 1 u of movement.
Rejects say where players are stopped; this says where they
successfully walk, which is what turns "somebody fell into a hole" into
a walked-surface map per world. Hooked in `SpaceManager::accept`, so a
rejected position is never sampled, and stamped with the *caller's*
processing instant rather than a fresh `Instant::now()` — both so the
sample carries the time the packet was processed and so the 5 s window
is reachable from the time-injected tests. Budget and level rationale:
[instrumentation-discipline.md](instrumentation-discipline.md#sampled-positive-telemetry-movementposition_sample).

## Known gap: advisory worlds carry no navmesh diagnosis

Everything above hangs off the **reject** path. A world whose
`resources.worlds.navmesh_mode` is `advisory` accepts off-mesh positions
instead of snapping them back, so it produces no
`movement.validation_reject` rows with `reason = "navmesh"`. What it
emits instead is `movement.navmesh` with
`reason = "advisory_off_mesh_accepted"` (`cell::space_manager::client_move`)
— at **TRACE**, level-gated, unthrottled, and carrying only the entity,
space and client position: no `gate`, no `nav_horiz_dist` / `nav_dy`, no
mesh hash. The advisory worlds are the ones with the worst meshes, which
is where the diagnosis is most needed, so finding mesh holes there means
enabling TRACE for that target and joining positions to the mesh by hand.

The reporting seam is deliberately a set of `SpaceManager` methods rather
than inline handler code so a non-rejecting caller can reuse the
diagnosis and the throttle; routing the advisory branch through it is the
obvious follow-up and was left out on purpose. The GM off-navmesh
allowance has the same shape at a smaller scale: `navmesh_gm_bypass` is
unthrottled and carries `world` but not the gate, distances or mesh hash.

## Regression guards

All in
[`space_manager/tests/movement_validation/`](../../crates/services/src/cell/space_manager/tests/movement_validation/):

| File | Guards |
|---|---|
| `telemetry_reject.rs` | `world` and its `unknown` fallback, the gate + distances + mesh hash, the "a bounds reject names no gate" negative, the throttle and its per-entity independence, the diagnosis on **all three** outcomes, the shared window, the `navmesh_loaded` field set |
| `telemetry_sampling.rs` | sample rate, minimum distance, players-only, navmesh state, the injected-clock window, "a rejected move does not sample" |
| `telemetry_lifecycle.rs` | release on `destroy_entity`, release on `destroy_space`, and the recycled-id stale window |

The `LogThrottle` arithmetic itself is pinned in
[`movement_telemetry/tests.rs`](../../crates/services/src/cell/space_manager/movement_telemetry/tests.rs).
Counter *emission* is not observable from a unit test —
`cimmeria_observability` no-ops without an initialised Meter — so the
guards assert on the fields and the throttle bookkeeping the same seam
produces, plus the stable label vocabulary.
