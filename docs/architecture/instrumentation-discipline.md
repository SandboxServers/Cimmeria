# Instrumentation Discipline

> **Status**: Convention adopted in issue #482 (2026-06-01). Companion to
> [negative-logging-convention.md](negative-logging-convention.md) which
> covers the *failure-side* discipline. This document covers the
> *success-side* — span placement, event-level rules, metric labels.

## What this document covers

`negative-logging-convention.md` answers "what do you log when an
expectation fails?". This doc answers the inverse: "when you instrument
a *working* code path, where does the span go, what level does the
event get, and which fields are safe to attach?"

The rules pin the implicit guidance scattered across
[`observability.md`](observability.md) (hot-path cost, target catalog)
into one greppable surface so a new contributor doesn't have to grep
the codebase to learn the convention.

Source: issue #482 (`Telemetry & logging instrumentation pass`).

## The four rules

### Rule 1 — Every dispatch entrypoint gets an info-level span

Any function that's the receiving end of a wire-level dispatch (Mercury
message → handler, content-engine action → executor, cell→base channel
message → base handler) gets:

```rust
#[tracing::instrument(
    name = "<system>.<verb>",
    level = "info",
    skip_all,
    fields(player_id, entity_id, space_id),
)]
```

- **`name=`** — `dotted.lowercase` matching the `target` catalog in
  [`observability.md`](observability.md). `world_entry.play_character`,
  `trade.request`, `cover.detection_tick`, `crafting.load`.
- **`level = "info"`** — info, not debug. Dispatch is the analytical
  surface SigNoz operators query against. A handler that's debug-only
  becomes invisible without `RUST_LOG=debug`.
- **`skip_all`** — never let `tracing::instrument` auto-include
  arguments. Inventories, slot lists, full packet bodies, and
  `Arc<Mutex<...>>` handles all panic or balloon the log when
  serialised. Whitelist via `fields(...)`.
- **`fields(...)`** — only the **correlator fields** an operator would
  use to filter the SigNoz timeline: `player_id`, `entity_id`,
  `space_id`, `peer`, `account_id`. Use the canonical names per
  [negative-logging-convention.md §field-naming-rules](negative-logging-convention.md#field-naming-rules)
  — no `pid` / `eid` aliases.

**The gold-standard reference:**
[`crates/services/src/base/world_entry/play_character.rs:26-31`](../../crates/services/src/base/world_entry/play_character.rs#L26).

### Rule 2 — Every state transition gets a debug-level event with `event = "..."`

Inside a span, a meaningful state transition (NPC entered combat, item
moved between containers, mission task advanced) gets a single
`tracing::debug!` event with an `event = "<short_discriminator>"`
field. The discriminator names the transition, not the function — it
must be greppable across the codebase as a stable token.

```rust
tracing::debug!(
    target: "npc_ai",
    event = "patrol_arrived",
    npc_id,
    target_index,
    delay_secs,
    "NPC AI: patrol → arrived, dwelling"
);
```

- **debug, not info.** State transitions can fire many times per
  second on hot loops; flooding info would blow out the SigNoz log
  retention budget. The dispatcher's info span carries the parent
  context; the per-transition event lives inside it.
- **`event = "..."` mandatory.** A SigNoz query
  `groupBy = event WHERE target = "<system>"` is the discoverability
  surface for state-machine analysis. Without `event`, the operator
  has to grep message bodies — fragile against wording changes.
- **`target:`** — same `dotted.lowercase` system name as the parent
  span. Targets stay in the catalog at
  [`observability.md`](observability.md#stable-target-catalog).

**Reference:** the `npc_ai` state handlers in
[`crates/services/src/cell/service/npc_ai.rs`](../../crates/services/src/cell/service/npc_ai.rs)
— every `patrol_arrived`, `patrol_waypoint_set`, `investigate_routed`,
`follow_routed` event uses this shape.

### Rule 3 — Per-tick decisions inherit the dispatcher span, NEVER add a per-handler span

The cell tick loop calls `npc_ai_fight`, `npc_ai_patrol`,
`npc_ai_wander`, ... once per NPC per tick. At 50 NPCs × 1Hz that's 50
handler invocations per second. **Each of those handlers must NOT add
its own `#[tracing::instrument]`** — the parent dispatcher span at
[`npc_ai.rs:79`](../../crates/services/src/cell/service/npc_ai.rs#L79)
already wraps every call. Adding a span per handler would 2× the span
volume for no diagnostic benefit (the parent already carries
`npc_id`/`ai_state`/`space_id`).

The pattern instead:

```rust
async fn npc_ai_patrol(npc_id: u32, ...) {
    // No #[instrument] — parent dispatcher span wraps us.
    // To record decision context onto the parent span:
    tracing::Span::current().record("decision_outcome", "patrol_dwell");
    ...
}
```

The dispatcher declares
`fields(decision_outcome = tracing::field::Empty)` so the
`Span::current().record(...)` in the handler fills the slot.

**Reference:** the cover-detection tick at
[`crates/services/src/cell/service/ticks/cover.rs:31-36`](../../crates/services/src/cell/service/ticks/cover.rs#L31)
declares `fields(player_count = tracing::field::Empty, events = tracing::field::Empty)`
and the body fills them via `Span::current().record(...)`.

### Rule 4 — Metric labels are enumerated, span/log fields are correlators

The metric system (introduced by [issue #482](https://github.com/SandboxServers/Cimmeria/issues/482))
ships counters and histograms to SigNoz via OTLP. **Metric labels are
not span fields.** The cardinality rules:

| Where | What goes there | Examples |
|---|---|---|
| Metric label | Enumerated low-cardinality string (target ≤ ~30 values) | `outcome`, `reason`, `kind`, `world_name`, `decision_outcome` |
| Span field | High-cardinality correlator | `player_id`, `entity_id`, `space_id`, `peer`, `mission_id` |
| Log field | Same as span field — any correlator | `player_id`, `entity_id`, `rows_affected`, `expected` |

The metric `npc_ai_decisions_total{decision_outcome}` is correctly
labelled by the enum; **adding `entity_id` as a label would explode
the label-set cardinality to one bucket per NPC**, which ClickHouse
handles badly and SigNoz's UI shows as a wall of cardinality warnings.

**Two thresholds — design target vs. hard ceiling.**

- **Target ≤ ~30 distinct values** per label is the *design goal*.
  Picking labels in this range gives readable SigNoz pivot tables and
  predictable ClickHouse merge-tree storage. The values listed in the
  table above all sit comfortably under this.
- **Hard ceiling ~100 distinct values** is the *do-not-cross line*.
  If a label can ever cross this — a per-player counter, a per-entity
  counter, a per-template counter for a content set that may grow
  beyond ~100 templates — the cardinality bound moves from "operator
  unfriendly" to "ClickHouse query-plan blow-up." Move it to a span /
  log field instead.

A label sitting between the target and the hard ceiling (e.g. 50
worlds when we ship more content) is a yellow flag, not a fail —
revisit it during the next instrumentation review.

### Worked example

A `trade.execute` handler that already has the dispatcher span:

```rust
#[tracing::instrument(
    name = "trade.execute",
    level = "info",
    skip_all,
    fields(initiator_player_id, recipient_player_id, total_cash),
)]
async fn execute_trade(...) -> Result<(), TradeError> {
    // ... attempt the atomic swap ...

    // Rule 4: counter label is the enumerated outcome (low-cardinality),
    // never a player_id (which is the span's correlator).
    cimmeria_observability::counter!(
        "trade_swaps_total",
        "outcome" => "completed",
    );
    Ok(())
}
```

The span fields and the counter label are complementary: the span
carries the correlators an operator filters *by* (which trade, whose
trade), the counter aggregates *across* trades by outcome.

## Anti-patterns

- **`tracing::info!` inside a hot tick loop.** Every player movement
  packet, every NPC AI tick, every projectile tick. If the message
  fires more than ~10/sec/process under normal load, it's `debug!`.
- **`#[instrument(level = "info")]` on a helper called from a hot
  loop.** A per-call info span IS a per-call log line; same volume
  budget. Helpers stay un-instrumented and inherit the dispatcher's
  parent span. Add the span at the dispatch entrypoint, not on
  every leaf function.
- **`fields(self)` or `fields(?everything)`** on `#[instrument]`. The
  serialiser will format the whole struct via Debug, which can OOM the
  log pipeline for nested entity / inventory state. Use `skip_all` and
  whitelist explicit fields.
- **Metric label = `entity_id` / `player_id` / `peer`.** Per the
  cardinality rule above — these are span fields, never labels. A
  ClickHouse merge-tree storing a label per entity for every counter
  emission would degrade query performance non-linearly.

## Defensible exceptions

- **One-shot boot-path functions** (navmesh load, content engine init)
  can use info-level events without a span, since they fire once and
  don't accumulate. The convention here is anchored on the dispatch
  pattern, not on every code path.
- **Error / warning logs inside a hot loop**. The level discipline in
  [negative-logging-convention.md](negative-logging-convention.md)
  governs — a `warn!` on an expectation-failure path SHOULD fire from
  inside a hot loop because the failure is rare and player-visible.
  The "every state transition is debug" rule is for the *success-side*
  state machine.

## Regression-guard testing

Per [TESTING.md](../../TESTING.md), any PR that adds or changes a
dispatch-level instrumentation point should include either:

- A `LogCapture` assertion that the expected info-span / debug-event
  fires (see [negative-logging-convention.md §regression-guard-testing](negative-logging-convention.md#regression-guard-testing)).
- A counter-emission test, if the change wires a new metric — verify
  the counter increments by 1 on the labelled path and 0 on the
  un-labelled paths.

Pin both the **level** and the **event discriminator** so a revert
that demotes `event = "patrol_arrived"` to a free-text `info!` trips
the test.

## Related

- [observability.md](observability.md) — OTLP exporter design, target
  catalog, sampler choice, `decision_outcome` enum.
- [negative-logging-convention.md](negative-logging-convention.md) —
  Companion: failure-side rules, `LogCapture` helper, field-naming.
- [TESTING.md](../../TESTING.md) — Test-type picker; regression-guard
  rules.
- [docs/audits/telemetry-audit-2026-06-01.md](../audits/telemetry-audit-2026-06-01.md)
  — Audit that produced this convention, with file:line gaps per
  feature.
