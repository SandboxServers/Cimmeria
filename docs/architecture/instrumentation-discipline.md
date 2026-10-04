# Instrumentation Discipline

> **Last updated**: 2026-10-04
> **Status**: Convention adopted in issue #482 (2026-06-01); Rule 5
> (stable player identity on every log) added 2026-09-17; Rule 6 (every
> ID paired with its name) added 2026-10-04. Companion to
> [negative-logging-convention.md](negative-logging-convention.md) which
> covers the *failure-side* discipline. This document covers the
> *success-side* — span placement, event-level rules, metric labels.

## What this document covers

`negative-logging-convention.md` answers "what do you log when an
expectation fails?". This doc answers the inverse: "when you instrument
a *working* code path, where does the span go, what level does the
event get, and which fields are safe to attach?"

The rules pin the implicit guidance scattered across
[`observability.md`](observability.md) (hot-path cost) and its target catalog
into one greppable surface so a new contributor doesn't have to grep
the codebase to learn the convention.

Source: issue #482 (`Telemetry & logging instrumentation pass`).

## The six rules

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
[`crates/base-world-entry/src/base/world_entry/play_character.rs:26-31`](../../crates/base-world-entry/src/base/world_entry/play_character.rs#L26).

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
  [`observability-target-catalog.md`](observability-target-catalog.md).

**Reference:** the `npc_ai` state handlers in
[`crates/cell-combat/src/cell/service/npc_ai/mod.rs`](../../crates/cell-combat/src/cell/service/npc_ai/mod.rs)
— every `patrol_arrived`, `patrol_waypoint_set`, `investigate_routed`,
`follow_routed` event uses this shape.

### Rule 3 — Per-tick decisions inherit the dispatcher span, NEVER add a per-handler span

The cell tick loop calls `npc_ai_fight`, `npc_ai_patrol`,
`npc_ai_wander`, ... once per NPC per tick. At 50 NPCs × 1Hz that's 50
handler invocations per second. **Each of those handlers must NOT add
its own `#[tracing::instrument]`** — the parent dispatcher span at
[`npc_ai/mod.rs:79`](../../crates/cell-combat/src/cell/service/npc_ai/mod.rs#L79)
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
[`crates/cell/src/cell/service/ticks/cover.rs:31-36`](../../crates/cell/src/cell/service/ticks/cover.rs#L31)
declares `fields(player_count = tracing::field::Empty, events = tracing::field::Empty)`
and the body fills them via `Span::current().record(...)`.

### Rule 4 — Metric labels are enumerated, span/log fields are correlators

The metric system (introduced by [issue #482](https://github.com/SandboxServers/Cimmeria/issues/482))
ships counters and histograms to SigNoz via OTLP. **Metric labels are
not span fields.** The cardinality rules:

| Where | What goes there | Examples |
|---|---|---|
| Metric label | Enumerated low-cardinality string (target ≤ ~30 values) | `outcome`, `reason`, `kind`, `world`, `decision_outcome` |
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

**Declaring a label as an enum.** `cimmeria_observability::metric_label!`
declares a label as a `Copy` enum with `ALL` and `label()`, so a call
site passes a variant and cannot invent a value, and a test can compare
`ALL` with the reasons the code logs. The ability metrics
(`crates/cell-combat/src/cell/abilities/metrics/`) use it for every
label and pin each set in `metrics/tests.rs`.

#### Ruling: `world` is an approved label

`movement_validation_rejects_total`, `npc_path_fail_total` and
`spawner.npc_respawn`'s counters all carry `world`. **~24 shipped
worlds**, comfortably inside the ≤ ~30 design target, and the
cross-product with the other labels on those counters stays in the low
hundreds of series.

It earns the slot rather than merely fitting it. Before September 2026
the movement-reject counter was labelled by `reason` alone, and the
reject log carried `space_id` with no world — so answering "which
zone's navmesh is rejecting players?" meant exporting rows and joining
space ids to world names by hand against a table that only exists
inside the running process. That join is what made the Castle_CellBlock
navmesh investigation a manual exercise. A world label is the
difference between a dashboard panel and an archaeology session.

If the world count ever crosses ~50 (a content expansion, or
per-instance labelling — **never** label by instance id, that is
unbounded), revisit. The `gate` label on the same counter is 5 values
and fixed by the [`NavGate`](../../crates/entity/src/navigation/verdict.rs)
enum; adding a gate is a deliberate change to that enum, not
open-ended growth.

#### Sampled positive telemetry: `movement.position_sample`

The accepted-position sampler is the one place this codebase emits a
**success-side** row on a per-packet path, so it is worth stating the
budget explicitly.

| Question | Answer |
|---|---|
| Level | **DEBUG.** It is a sampled hot-path row, matching the sibling `movement.player`. Not INFO: per Rule 1, info is for dispatch entrypoints, and per the anti-patterns below, anything that can fire more than ~10/sec/process under normal load is debug. DEBUG *is* exported to SigNoz (the `movement.player` / `npc_ai.tick` saved views query it), so this is not a decision to hide the data |
| Rate | 1 row per player per **5 s**, and only after **≥ 1 u** of movement. A standing player emits nothing |
| Volume | 720 rows/hour per actively-moving player. At 20 concurrent: **14,400 rows/hour**. For scale, the reject stream this landed alongside was running at ~2,000 rows/hour from a *single* stuck entity before it was throttled |
| Why sample at all | Rejects say where players are *stopped*. Nothing said where they successfully walk, so a navmesh hole was only visible once somebody fell into it. Accepted positions grouped by world are the walked-surface map that makes the hole visible first |
| Why not NPCs | They outnumber players by an order of magnitude in a populated zone, and `movement.npc` / `npc_ai.tick` already cover them |

The interval and distance constants are pinned by a test
(`position_sample_budget_matches_the_documented_rate`) so loosening
either one trips CI against the figures quoted here.

### Rule 5 — Every log describing player activity carries `account_id` + `player_id`

An `entity_id` is **not an identity**. It is a recycled per-space slot
integer: when a player disconnects their id returns to the pool and a
later connection can be handed the same number minutes later. A log
filtered on `entity_id` answers "what happened in this slot", not
"what did this player do".

So any log event describing something a *player* did — a movement
reject, a GM console command, a teleport, an AoI drop, a session
lifecycle transition — must carry both:

| Field | Source | Stability |
|---|---|---|
| `account_id` | `account.account_id` | Stable across reconnects **and** character switches |
| `player_id` | `sgw_player.player_id` | Stable for the life of a character session |

`entity_id` stays alongside them. This is additive — it is still the
right correlator for per-space and AoI debugging, and for NPCs it is
the *only* one.

Rule 6 pairs each ID with its name, and `PlayerIdentity` carries both
names (NT-02): `account_name` (the login) and `player_name` (the
character). A line that emits the identity emits all four fields, and
`entity_name` next to `entity_id`.

#### These go on the log EVENT, not only on a span

This is the part that is easy to get wrong, because the local
`RUST_LOG` output makes a span-only approach look like it works.

The OTLP log pipeline is
[`opentelemetry-appender-tracing`](../../crates/server/src/otel.rs)'s
`OpenTelemetryTracingBridge`. It converts each `tracing` event into one
OpenTelemetry log record carrying **that event's own fields**, plus
`trace_id` / `span_id` for correlation. It does **not** walk the
ancestor span chain and copy span fields onto the log record. A field
that exists only on a parent span is therefore invisible to a SigNoz
**Logs** query — which is the surface operators actually use to answer
"show me everything account 6 did".

Two further reasons a span-only approach cannot work here:

1. **Spans do not cross the base↔cell boundary.** The two halves of the
   server are separate tokio tasks joined by
   `mpsc::Sender<BaseToCellMsg>` / `CellToBaseMsg`
   ([`orchestrator.rs`](../../crates/services/src/orchestrator.rs)). A
   span entered on the base side is not in scope when the cell task
   later dequeues the message — so movement validation, console
   dispatch, travel, and entity lifecycle could never inherit it.
2. **The candidate parent spans are `level = "debug"`.**
   `base.datagram`, `base.encrypted_datagram`, `base.player_method` and
   `cell.dispatch` are all debug-level, so under the default info
   filter they are not recorded at all and any field on them vanishes.

Spans **may** also declare the fields (`world_entry.play_character`
already does, and it is a useful correlator in the Traces view) — but
a span declaration never substitutes for the event field.

#### Pass `Option`s through; never `unwrap_or(0)`

Both fields are typed `Option` and are handed to `tracing` **as
`Option`s**:

```rust
let id = space_mgr.player_identity(entity_id);
tracing::warn!(
    target: "movement.validation",
    entity_id,
    entity_name = id.player_name,   // Option<&str>
    account_id = id.account_id,     // Option<u32>
    account_name = id.account_name, // Option<&str>
    player_id = id.player_id,       // Option<i32>
    player_name = id.player_name,   // Option<&str>
    reason = reason_label,
    "movement.validation_reject: ..."
);
```

`tracing`'s `impl<T: Value> Value for Option<T>` records **nothing**
when the value is `None`. So a player's line carries `account_id=6`
and an NPC's line carries no `account_id` key at all.

Do not "helpfully" unwrap to a sentinel. `account_id = 0` is
indistinguishable from a real account in a query and matches every NPC
in the store; `"None"` pollutes the field's value set the same way.
Absence is the correct encoding for "this entity has no account", and
the guards in
[`identity_propagation.rs`](../../crates/cell/src/cell/service/base_messages/tests/identity_propagation.rs)
assert the fields are **absent** for NPCs precisely so this shortcut
trips CI.

#### Where identity comes from

Two resolvers, one per side of the server. Never re-derive it inline.

| Side | Resolver | Backing state |
|---|---|---|
| Cell | `SpaceManager::player_identity(entity_id)` | `CellEntity::account_id` / `::player_id`, names from `::log_names` |
| Base | `base::session_identity::identity_for_entity(connected, entity_to_addr, entity_id)`, or `session_identity(&client)` when the session is in hand | `ConnectedClientState::account_id` / `::active_player_id`, names from `::account_name` / `::player_name` |

Both return `PlayerIdentity::UNKNOWN` (every field `None`) for an NPC
or an unresolvable id, so the caller emits nothing.

The names are `Option<&'static str>`, interned once per distinct name
(`cimmeria_entity::name_intern`), so `PlayerIdentity` stays `Copy`. A
blank name, or one longer than 64 bytes, interns to `None`. On the cell
the names are interned when they are stamped (`CreateEntity`,
`InitPlayerState`) into `CellEntity::log_names`, so
`player_identity` is a plain copy with no lock and no hashing.
`log_names` is for logs only: the game's `character_name` is still set
by `InitPlayerState` alone, so name lookups such as `.goto` see a
loading player exactly as before.

The cell entity is identity-stamped **at birth**, from
`BaseToCellMsg::CreateEntity`, not at `InitPlayerState`.
`InitPlayerState` arrives only after `onClientReady`; stamping there
would leave the multi-second world-entry window — and the fresh entity
a gate-travel builds in the destination world — un-attributable.
`InitPlayerState` still re-asserts the pair as a belt-and-braces path
for any create route that skips the stamp. `CreateEntity` carries the
two names as well (`account_name`, `player_name`), into `log_names`,
so the same window names the player, not only the IDs.

#### Naming when an actor acts on someone else

GM commands move *other* players. `account_id` / `player_id` always
name the **caller**, so a single `account_id = N` filter returns
everything that account did. The subject gets its own prefixed key
(`subject_player_id`, `target_player_id`) next to the existing
`entity` / `target` field.

#### Resolve late, and before teardown

- **Late**: resolve inside the branch that actually logs, not at
  function entry. `handle_entity_move` runs ~10 Hz per active player
  and the accepted path must not pay for a lookup it never emits.
- **Before teardown**: `destroy_entity` / `disconnect_entity` remove
  the entity, so identity must be snapshotted at the top of the
  function. A lookup at the log statement resolves to `UNKNOWN` — and
  the session-closing line is the one an incident timeline needs most.

#### Scope

NPC-only logs do not get forced identity — they have no account, and
`PlayerIdentity::UNKNOWN` already encodes that correctly. Auth and
character-select logs already carried `account_id` before this rule and
are unchanged.

### Rule 6 — Every ID field is paired with its name

An ID tells a query which rows belong together. It tells a human
nothing. `ability_id=880 target=4123 space_id=12` sends whoever reads
it — you at 2am in SigNoz, a teammate in Discord, an agent spending
tokens — off to look up three numbers by hand. So every ID a log line
or a Discord message carries has its name next to it, resolved by
ordinary code when the line is written:

```text
ability_id=880 ability_name="Staff Blast" target=4123 target_name="Jaffa Guard"
space_id=12 world="Castle_CellBlock" reason=out_of_range
```

The rule was added by the
[named-telemetry campaign](../analysis/named-telemetry/README.md),
which also builds the lookups it needs (NT-01, NT-02) and the CI scan
that enforces it (NT-03).

#### The pairing contract

- **Pair, don't replace.** The ID stays: it is the join key, and two
  objects can share a name. The name goes next to it.
- **Same prefix.** `ability_id` pairs with `ability_name`,
  `target_player_id` with `target_player_name`. A reader who sees one
  key can predict the other.
- **Absent when unresolved.** Pass the name as an `Option<&str>` and
  let `tracing` drop the field when it is `None`, exactly as Rule 5
  does for `account_id`. Never write `"unknown"`, `""` or `"None"`:
  each one becomes a value in SigNoz's value set, matches every other
  unresolved row, and hides the fact that the name is missing. A
  missing name is a signal — a `template_id` with no `template_name`
  points at a seed hole — so let it show.
- **Seed placeholders are unresolved.** `NO ITEM NAME`,
  `UNUSED DIALOGUE` and their kin are not names. The NameBook (NT-01)
  loads them as `None`, so the field is left out.
- **On the event, never only on a span.** Same reason as Rule 5: the
  OTLP bridge exports only the event's own fields, and so does the
  Discord layer (see the anti-pattern below).
- **Resolve late, and before teardown.** Same as Rule 5. Look the name
  up inside the branch that logs, and snapshot it at the top of a
  function that destroys the entity.

#### Naming an entity ID

Entity IDs are recycled slots, so they are named from `SpaceManager`,
never the NameBook (NT-02):

| Call | Returns |
|---|---|
| `entity_label(entity_id)` | `Option<&str>`: the character name for a player, the `name_id` text for an NPC |
| `entity_names(entity_id)` | `EntityNames { entity_name, template_id, template_name }`, all three fields an NPC line carries (D-NT5). A player gets only `entity_name` |
| `entity_label_at(space_id, entity_id, at)` | The label of whoever held the slot at server time `at` (a `SystemTime`). Lifetimes are half-open, `[created_at, destroyed_at)`. The live entity answers when `at` is at or after its `created_at`; otherwise a departed-entity ring answers when `at` falls inside a departed occupant's lifetime. `None` otherwise, so a reused slot is never named after the wrong occupant |

**`at` is server time.** Lifetimes are stamped with the server's wall
clock, and two occupants of a slot can be seconds apart. A caller naming
a client row (NT-40) maps the row's time onto the server clock first,
from the server's receive time or a per-session clock offset. Passing a
client timestamp raw lets clock skew name the wrong occupant, and lets a
client choose the name by choosing the time.

Each space keeps two departed rings, one for players and one for NPCs,
each holding the entities destroyed in it for 10 minutes, up to 4,096
rows, whichever is smaller. They are separate because NPC despawns
outnumber player departures by orders of magnitude: in one ring they
would push a departed player out long before its 10 minutes. The rings
live on `SpaceManager`, so a destroyed instance's NPCs and its last
player stay nameable after the instance is gone. `destroy_entity` and
`destroy_space` record into them; `destroy_entity` snapshots the names
at its top, next to the identity snapshot.

#### Which key gets which name

**The default rule.** A field whose key ends in `_id` pairs with the
same prefix ending in `_name`: `witness_id` → `witness_name`,
`owner_id` → `owner_name`, `npc_id` → `npc_name`,
`subject_player_id` → `subject_player_name`. The bare entity keys
`target`, `attacker`, `entity` and `witness` pair with the key plus
`_name`: `target` → `target_name`.

**Exceptions.** The table lists only the keys where the name key or
the lookup differs from the default. Each row also says where the name
comes from, so every call site resolves it the same way.

| ID key | Name key | Source |
|---|---|---|
| `entity_id`, `target`, `attacker` (any key holding an entity ID) | `entity_name`, `target_name`, `attacker_name` | A player: the character name. An NPC: its player-facing `name_id` text, **and** the line also carries the `template_id` + `template_name` pair (D-NT5). Live entities only, through `SpaceManager` and the departed-entity ring (NT-02), never the NameBook: entity IDs are recycled slots |
| `template_id` | `template_name` | `entity_templates.template_name` |
| `item_id` (instance), `item_type_id` | `item_name` | `items.name`, looked up by the item's type. A logged `item_id` is often an instance ID; resolve it to its type first |
| `ability_id` | `ability_name` | `abilities.name` |
| `effect_id` | `effect_name` | `effects.name` |
| `mission_id`, `step_id`, `objective_id` | `mission_name`, `step_name`, `objective_name` | `missions.mission_label`; step and objective display text |
| `dialog_id`, `dialog_set_id`, `speaker_id` | `dialog_name`, `dialog_set_name`, `speaker_name` | `dialogs.name`, `dialog_sets.name`, `speakers.name` |
| `space_id`, `world_id` | `world` | `SpaceManager` (a space's world) and the `Worlds` table |
| `account_id` | `account_name` | The session's login name |
| `player_id` | `player_name` | The session, or `sgw_player` |
| `org_id` | `org_name` | Organizations |
| `archetype` | `archetype_name` | `archetype_name()` |
| `error_code`, `moniker_id` | `error_name`, `moniker_name` | `error_texts.moniker_name`, `monikers.name`. `error_name` is reserved for `error_texts`: a code from another vocabulary names its domain, as the Black Market's `error_id` pairs with `bm_error` (the `BMError` variant) |
| A bitflag word (`state_field`, `flags`, `interaction_flags`, `recipient_flags`, `from_mask`, …) | `<key>_names` | The flag set's `FlagSet` table (`cimmeria_common::flag_names`), rendered `A\|B\|C` with unknown bits as one hex remainder; see [negative-logging-convention.md § Field naming rules](negative-logging-convention.md#field-naming-rules). Not on rows exported per packet or per tick (NT-31) |
| `opcode`, `msg_id` | `msg_name` | The NT-30 Mercury message table (system frames such as `AUTHENTICATE` included; `wire-log` already uses `msg_name`) |
| `method_id`, `method_index` | `method_name` | The NT-30 method table, per entity type (clientIndex). A `msg_id` in an entity-method range carries both `msg_name` and `method_name` |

**Generic keys name their domain first.** `type_id` and `design_id` are not item keys everywhere: `abilities.type_id`
is logged in the spawner, and the GM console uses `design_id` for mission and template input. A key that
doesn't say what it identifies can't be paired by rule, so a sweep renames it to its domain key
(`item_type_id`, `ability_id`, `mission_id`, `template_id`) and then pairs that. Until it is renamed it
counts as unpaired in NT-03's baseline.

`space_id` pairs with `world`, not `world_name`. `world` is already the
key on about 70 log sites against about 20 for `world_name`, it is the
key [negative-logging-convention.md](negative-logging-convention.md#field-naming-rules)
asks for, and it is the approved metric label (Rule 4), so a dashboard
and a log query filter on the same key.

**Where existing keys disagree, sweeps converge on one:**

| Keep | Retire | Why |
|---|---|---|
| `player_name` | `character_name` | It follows the default rule from `player_id`. As log fields, `player_name` is on 5 sites and `character_name` on 1. Struct fields such as `CellEntity::character_name` keep their names; this is about the log key |
| `world` | `world_name` | See above |
| `npc_name` (with `npc_id`), `entity_name` (with `entity_id`) | `name` holding an NPC or template name | A bare `name` key says nothing about what it names. `name = %record.template_name` in the spawn path becomes `template_name` |

A key that fits neither the default rule nor the exceptions table needs
an exemption (below), or NT-03's scan fails the build.

#### Exemption: `// nt:id-only <reason>`

Some IDs have no name to pair: a correlation token, a generated
session ID, a row ID in a table with no name column. Mark the field's
line with a comment saying why. The dev-session token mint in
`crates/admin-api/src/routes/dev_session/handlers.rs` is one:

```rust
tracing::info!(
    session_id = %session_id, // nt:id-only generated UUID, nothing to name
    session_kind = claims.session_kind(),
    "Minted dev-session telemetry token"
);
```

The marker exempts exactly one field, so the field goes on its own
line: a marked line that holds two ID fields, in one call or two, fails
the build. The reason is mandatory and has to read as one: at least two
words or 10 characters (`x`, `-` and `TODO` fail). The marker must open
the comment; a comment that only mentions `nt:id-only` exempts nothing.

#### Names are never metric labels

Rule 4 stands. An item, ability or player name has as many values as
the ID it names, so it is a log field, never a label. `world` stays the
one approved name-shaped label, under Rule 4's ruling.
Declare a label with `cimmeria_observability::metric_label!` (Rule 4)
and a call site can't pass a name into it at all: the label's values
are a fixed enum, not a string.

#### Discord

Discord messages follow the same pairing, with three extra rules. The
restoration team reads Discord, but only developers on the VPN can
reach SigNoz, so each message has to stand on its own.

- **Every object renders as `Name (#id)`:** `Staff Blast (#880)`,
  `steve (#6)`. An unresolved name renders as the ID alone, `#880`.
  Account login names stay in Discord (D-NT2): the server is private
  and team-only. Player IPs and whisper text stay hidden, as today.
- **No links to SigNoz, the admin API or any other VPN-only host.**
  Most readers can't open them. NT-11 adds a test that fails on any
  such link anywhere in an embed.
- **`trace_id` as plain text.** An error embed may carry the
  `trace_id` as a short footer line a developer can paste into SigNoz
  (D-NT3). It is text, never a link.

The Discord tracing layer folds each `x_id` + `x_name` pair into one
embed field (NT-11), so pairing doesn't push other fields past the
embed's field cap.

#### Scope

Every log event (`trace!` to `error!`, and `event!`) and every Discord message that carries an ID, at any
level. Span fields (`*_span!`, `#[instrument(fields(...))]`) are out of scope: names on a span reach
neither the log record nor Discord (see the anti-pattern below), so NT-03 doesn't scan span constructors.
A span may still carry a name for the Traces view.
NPC lines are in scope too: unlike Rule 5's identity, an NPC has a
name. Existing lines converge through the campaign's system sweeps
(NT-20 to NT-27), and NT-03's baseline only shrinks.

The scan is `unpaired_id_fields_only_shrink` in
`crates/server/src/logging/unpaired_id_tests/`, and the baseline is
`crates/server/src/logging/unpaired_id_baseline.txt` (unpaired fields
per file). A sweep that pairs fields lowers the baseline in the same
PR with `NT_BASELINE_BLESS=1 cargo nextest run -p cimmeria-server
unpaired_id`. The scan applies the exceptions table by suffix and keeps
the prefix (`dest_space_id` pairs with `dest_world`), and a dotted key
pairs under the same path (`npc.template_id` with `npc.template_name`).

How the scan reads the code and the baseline:

- **Test code is skipped:** test files, and anything under `#[cfg(test)]`
  or `#[cfg(any(test, ...))]` / `#[cfg(all(test, ...))]`, down to a single
  field or match arm.
- **Wrappers count.** A `macro_rules!` wrapper that forwards `$(...)`
  into an event macro has each of its call sites in the same file judged
  as an event, with the wrapper's fixed fields counted toward pairing.
  A wrapper with no call site in its file fails the build unless its
  forwarding line carries a marker. So does renaming an event macro in a
  `use tracing::... as ...` import.
- **Lists of IDs are out of scope.** `effect_ids`, `target_player_ids`
  and other `*_ids` keys aren't ID-shaped under the default rule, and the
  scan doesn't count them.
- **The baseline only shrinks in total.** Its `# total N` line is the sum
  of the per-file rows. A bless (`NT_BASELINE_BLESS=1`) accepts any
  per-file change, a moved or split file included, while that total
  doesn't rise, and refuses otherwise; it panics under `CI`. A scanner
  change that finds more fields is the one legitimate rise: empty the
  file and bless, and the `# total` line shows the rise in review.
- **What the ratchet can't see:** pairing one field and adding a new
  unpaired one in the same file keeps that file's count, so it passes.
  Review catches that, not the scan.

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
- **Identity on the span only.** A field that lives on a parent span
  is not on the exported log record — `opentelemetry-appender-tracing`
  does not flatten ancestor span fields — so a SigNoz Logs filter
  never sees it. Per Rule 5, put `account_id`/`player_id` on the
  event.
- **`account_id = id.account_id.unwrap_or(0)`.** A `0` sentinel is
  indistinguishable from a real account in a query and matches every
  NPC. Pass the `Option` through and let the field be omitted.
- **Re-deriving identity inline** (hand-rolling the
  `entity_to_addr` → `connected` two-step, or reading
  `entity.player_id` without `account_id`). Use
  `SpaceManager::player_identity` / `session_identity::identity_for_entity`
  so every call site emits the same two field names.
- **Names only on a span.** Rule 6's names are invisible anywhere a
  human reads a log line if they sit only on a parent span. The OTLP
  bridge copies only the event's own fields into the log record (Rule 5
  § "These go on the log EVENT"), and the Discord tracing layer's
  `DiscordLayer::on_event` (`crates/discord/src/layer/mod.rs`) reads
  only the event's fields too. A span-level name helps the Traces view
  and nothing else. Put the name on the event.
- **`name = x.as_deref().unwrap_or("")`.** An empty string is a value:
  it matches every other unresolved row and hides the missing name.
  Pass the `Option` through, as Rule 6 says. Several `npc_name` sites
  do this today; the NPC sweep (NT-25) fixes them.
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

- [observability.md](observability.md) — OTLP exporter design, sampler
  choice. Its target catalog and the `decision_outcome` enum are in
  [observability-target-catalog.md](observability-target-catalog.md).
- [negative-logging-convention.md](negative-logging-convention.md) —
  Companion: failure-side rules, `LogCapture` helper, field-naming.
- [TESTING.md](../../TESTING.md) — Test-type picker; regression-guard
  rules.
- [docs/audits/telemetry-audit-2026-06-01.md](../audits/telemetry-audit-2026-06-01.md)
  — Audit that produced this convention, with file:line gaps per
  feature.
