# Player-ghost AoI cascade

> **Last updated**: 2026-09-19
> **Audience**: Engineers touching AoI introduction, the `createOnClient`
> cascade, `ConnectedClientState`, or anything that decides what one player
> sees of another
> **Type**: Architecture decision + reference for the cascade contents
> **Owner**: AoI / entity lifecycle
> **Companions**: [mercury-bundle.md](mercury-bundle.md) (the bundle the
> deferred path appends into),
> [first-login-cinematic-aoi-hold.md](first-login-cinematic-aoi-hold.md) (the
> hold that stretches the deferred window on a first login),
> [negative-logging-convention.md](negative-logging-convention.md)
> (the seam `aoi.player_ghost_incomplete` follows),
> [observability.md](observability.md) (target catalog),
> [state-field-bits.md](state-field-bits.md) (what `onStateFieldUpdate`
> carries), [../gap-analysis.md](../gap-analysis.md) §8

## TL;DR

In a shared (non-instanced) world — Castle, Harset — two players in the same
space now actually see each other: body, gear, nameplate, level, alignment,
and live combat/death state. Before this change a player entering another
player's AoI got the **NPC** `createOnClient` cascade with nothing in it: no
`BeingAppearance`, no name, placeholder stats, `stateField = 0`.

Four pieces make that work:

1. A dedicated **player-ghost cascade** composer that mirrors the 2009
   Python witness cascade for an `SGWPlayer`.
2. A **split of data ownership** between cell and base, joined **at emit
   time** rather than when the cell fires the event.
3. An **introduction gate** that keeps a player's cell entity out of
   everybody's AoI until it is actually a player, because introduction is
   one-shot.
4. A **base-to-cell witness broadcast** so a rebuilt `BeingAppearance`
   (weapon draw, holster, gear change) or a level-up reaches the players
   already watching, not just the player it belongs to.

Not yet validated with two real game clients — see
[Known gaps](#known-gaps) and the [UAT checklist](#two-client-uat-checklist).

## Context

### Root cause 1 — the cascade was NPC-shaped

Phase 1 of an AoI introduction (`CREATE_ENTITY` + `UPDATE_AVATAR`) is
class-agnostic and was already correct. Phase 2 — the `createOnClient()`
cascade that fills in everything the client renders — had exactly one
implementation, `compose_create_entity_cascade_body` in
[`mercury/aoi/create.rs`](../../crates/services/src/mercury/aoi/create.rs),
written against `NpcAoIData`: template-sourced faction, alignment, name id,
flags, and a `level` the cell holds. A player has none of that. The cell
passes `npc_data: None` for a player, so a witness received an entity with a
body it could not composite, no `onBeingNameUpdate`, no public stats, and
`stateField = 0` — a player who was dead, crouched or in combat when you
walked up looked idle and upright.

The legacy server did not have this problem because its cascade was a
method-resolution chain on the entity itself:
`SGWSpawnableEntity.createOnClient` → `SGWBeing.createOnClient` →
`SGWPlayer.createOnClient`
([`SGWSpawnableEntity.py:125-140`](../../deprecated/python/cell/SGWSpawnableEntity.py),
[`SGWBeing.py:491-514`](../../deprecated/python/cell/SGWBeing.py),
[`SGWPlayer.py:575-581`](../../deprecated/python/cell/SGWPlayer.py)). An
`SGWPlayer` picked up the player-specific tail for free.

### Root cause 2 — players were introduced during their own load window

A player's `CellEntity` exists from `CreateEntity`, which is the whole time
that client is downloading and loading the map. Two later messages fill it
in: `ConnectEntity` flips `is_player`, and `InitPlayerState` lands the
archetype and seeds archetype stats.

In a shared world, every nearby player's AoI tick runs during that window.
It saw a live cell entity at a real position and introduced it — as a blank,
NPC-shaped entity. And because `compute_player_aoi` marks the witness set
**on introduction**, the entity is never re-introduced once the real state
arrives. The witness was stuck with the placeholder until they left AoI and
came back, or relogged.

This is a shared-world-only failure. It never showed up in Castle Cellblock
solo testing, which is why it survived this long.

## Decision

### 1. Split the cascade's inputs by who owns them

Nothing owns the whole cascade, so stop pretending one side does.

| Half | Owner | Why it lives there |
|---|---|---|
| Live state — `stateField`, target, public stats, ammo type | **Cell** | The cell is the authority for combat state, health and targeting. It changes every tick |
| Identity — name, level, archetype, alignment, `BeingAppearance` args, tint args | **Base** | The base owns `ConnectedClientState`, and `refresh_player_appearance` ([`methods/inventory/appearance.rs:36`](../../crates/services/src/base/world_entry/methods/inventory/appearance.rs)) keeps `cached_appearance_args` current across every equip, holster and bandolier change |

The cell ships its half on
`CellToBaseMsg::EnteredAoI { player_data: Option<PlayerAoIData> }`
([`cell/messages/data.rs`](../../crates/services/src/cell/messages/data.rs)),
built by `PlayerAoIData::from_entity`. `Some` exactly when the entering
entity is a player; NPCs keep carrying `NpcAoIData` and are untouched.

`player_alignment` is a new field on `ConnectedClientState` — the alignment
was previously only ever read into the owning client's `mapLoaded` body, so
nothing cached it for a third party to read.

### 2. Join at emit time, not at event time

[`base/world_entry/cell_dispatch/player_ghost.rs`](../../crates/services/src/base/world_entry/cell_dispatch/player_ghost.rs)
does the join. `resolve_identity` reads the observee's session out from under
the `connected` lock; `compose_cascade_body` picks the player-ghost cascade
when both halves are present and the NPC/bare cascade otherwise.

The join happens when the packet is composed, which matters on the deferred
path. A witness still loading its own map has its `EnteredAoI` buffered in
[`base/deferred_aoi.rs`](../../crates/services/src/base/deferred_aoi.rs) for
seconds, and the observee can holster a weapon or swap armour in that time.
Joining at buffer time would ship a stale body. Both call sites use the same
function: `aoi::entered_aoi` (standalone packets) and
`deferred_flush::flush_deferred_aoi` (phase-2 bundle).

On a character's **first** login that window is longer still. The
first-login cinematic AoI hold keeps every entity introduction — players
included — in the same buffer until the intro movie ends, so a witness who
sits through `Cine-SGWLogo` joins identity up to 16 seconds after the cell
fired the event. Emit-time joining is what makes that safe: the observee can
equip, holster and change bandolier slots throughout the movie and the
witness still gets the current body. See
[first-login-cinematic-aoi-hold.md](first-login-cinematic-aoi-hold.md).

Locks are released before any send — `entered_aoi` resolves the identity
into an owned `PlayerGhostIdentity` up front, so no `connected` lock is held
across a `.await`.

### 3. Gate introduction on the entity actually being a player

`CellEntity::is_introducible()`
([`entity/cell_entity/witness_aoi.rs`](../../crates/entity/src/cell_entity/witness_aoi.rs)):

```rust
self.account_id.is_none() || (self.is_player && self.archetype_id.is_some())
```

`compute_player_aoi` ([`cell/space_manager/aoi.rs`](../../crates/services/src/cell/space_manager/aoi.rs))
skips a non-introducible entity entirely — it never enters `current_aoi`, so
the witness set is not marked and the entity is introduced properly on a
later tick.

`account_id` is the discriminator rather than `is_player` precisely because
`is_player` is still `false` for the window being guarded. `account_id` is
stamped at `CreateEntity` and is `None` for every server-spawned entity, so
NPCs and props short-circuit on the first clause and are never gated.

### 4. Keep existing witnesses current: base-built updates fan out through the cell

Introduction is only half of "players see each other". The base also owns
state that changes *after* introduction — above all the `BeingAppearance` it
rebuilds from the database on every equip, holster, draw and bandolier slot
change. `refresh_player_appearance` sent that rebuild to the player's **own**
client only; a player who drew a weapon in front of you stayed holstered on
your screen.

The base does not know who is looking: witness sets live on the cell, and a
second copy on the base would be one more thing to leak on disconnect. So the
base hands the finished args to the cell with
`BaseToCellMsg::BroadcastToWitnesses { entity_id, method_index, args }`
([`base/helpers/witness_broadcast.rs`](../../crates/services/src/base/helpers/witness_broadcast.rs)),
and the cell fans them out through `send_entity_method_to_witnesses` — the
same path every cell-originated state change already uses — producing one
`CellToBaseMsg::WitnessEntityMethod` per observer. The bytes broadcast are
the bytes cached in `cached_appearance_args`, so a player already watching
and a player who arrives a second later converge on one appearance.

Level-ups take the same route. `handle_grant_xp` bundles XP, level and
training points to the levelling player's **own** client —
`send_bundle_to_witness_reliable` is single-recipient despite its name — so
when a grant crosses a level boundary it also broadcasts one `onLevelUpdate`
carrying the **final** level, as the legacy `SGWBeing.setLevel` did
(`SGWBeing.py:684-685`). One message even on a multi-level catch-up grant:
a witness renders a level, not the per-level training-point ceremony. The
session's `player_level` is updated in the same handler, so players who
arrive later get the same number from the introduction cascade.

## The cascade

Composed by `compose_player_ghost_cascade_body` in
[`mercury/aoi/player_ghost.rs`](../../crates/services/src/mercury/aoi/player_ghost.rs).
Every method is encoded against `IDBASE_SGW_PLAYER` (61). Order matches the
legacy chain top to bottom.

| # | Method (`method_idx`) | Args | Owner | Legacy reference |
|---|---|---|---|---|
| 1 | `ON_KISMET_EVENT_SET_UPDATE` (9) | `PLAYER_KISMET_EVENT_SET_ID` = 1025 | constant | `SGWSpawnableEntity.py:128-131`, value from `SGWPlayer.py:440` |
| 2 | `BEING_APPEARANCE` (26) | `cached_appearance_args` verbatim | base | `SGWBeing.py:491-493` |
| 3 | `ON_ENTITY_TINT` (10) | `cached_tint_args` verbatim | base | `SGWBeing.py:494` |
| 4 | `INTERACTION_TYPE` (3) | `0u64` | constant | `SGWSpawnableEntity.py:133` |
| 5 | `ON_ENTITY_FLAGS` (4) | `0u64` | constant | `SGWSpawnableEntity.py:136` |
| 6 | `ON_VISIBLE` (8) | `1u8` | constant | `SGWSpawnableEntity.py:137-138` |
| 7 | `ON_LEVEL_UPDATE` (15) | `player_level` | base | `SGWBeing.py:501` |
| 8 | `ON_TARGET_UPDATE` (16) | `PlayerAoIData::target_id` | cell | `SGWBeing.py:502` |
| 9 | `ON_BEING_NAME_UPDATE` (17) | `player_name` as a wstring | base | `SGWBeing.py:503-504` |
| 10 | `ON_ALIGNMENT_UPDATE` (24) | `player_alignment` | base | `SGWBeing.py:505` |
| 11 | `ON_FACTION_UPDATE` (25) | `PLAYER_FACTION` = 3 | constant | `SGWBeing.py:506`, value from `setupPlayer` |
| 12 | `ON_STATE_FIELD_UPDATE` (19) | `PlayerAoIData::state_field` | cell | `SGWBeing.py:507` |
| 13 | `ON_ARCHETYPE_UPDATE` (23) | `player_archetype` | base | `SGWBeing.py:511-512` |
| 14 | `ON_STAT_BASE_UPDATE` (21) | `StatList::serialize_public_base()` | cell | `SGWBeing.py:514` (`sendStats`) |
| 15 | `ON_STAT_UPDATE` (20) | `StatList::serialize_public()` | cell | `SGWBeing.py:514` (`sendStats`) |
| 16 | `ON_ENTITY_PROPERTY` (7) | `GENERICPROPERTY_AmmoTypeId` (3) + active bandolier ammo | cell | `SGWPlayer.py:579-581` |

Conditional sends, each mirroring a legacy branch:

- **2 and 3** are skipped together when the session has no cached
  appearance. A tint with no body is meaningless, so it is never sent alone —
  same shape as `createAppearanceOnClient`'s `if self.bodySet and
  self.components`.
- **9** is skipped when the name is empty (`if self.beingName != ""`).
- **13** is skipped for `ARCHETYPE_Any` (0) — `if self.archetype !=
  Atrea.enums.ARCHETYPE_Any`.
- **14 and 15** are skipped when the serialized list is just its 4-byte
  zero-count prefix; Python's `sendStats` sends nothing for an empty list.

Two methods the legacy chain emits that this one does not:

- `onExtraNameUpdate` is **deliberately** absent. It was commented out in
  `SGWPlayer.py:577-578` with the note that it overwrites the displayed name
  of every player on the witness's client. Sending it would undo method 9.
- `GENERICPROPERTY_DatabaseId` and `onBeingNameIDUpdate` do not apply:
  players have no template `speakerId`, and a player's name is a string, so
  `beingNameId` is 0. Both legacy branches would be false.

`PLAYER_KISMET_EVENT_SET_ID` and `PLAYER_FACTION` are shared constants: the
owning client's `mapLoaded` body
([`mercury/world_data/map_loaded.rs`](../../crates/services/src/mercury/world_data/map_loaded.rs))
now reads them instead of its own literals, so what you see of yourself and
what others see of you cannot drift.

## Alternatives considered

**Cell-side only — push appearance into `CellEntity`.** Give the cell entity
a copy of the `BeingAppearance` args and compose the whole cascade in
`mercury/aoi/create.rs`, no new base module. Rejected: the base already owns
the appearance cache and already has the code that keeps it fresh
(`refresh_player_appearance` runs on every equip / holster / bandolier
change). Mirroring it onto the cell entity means a second copy that can go
stale, and the staleness window is exactly the deferred-AoI window this
design has to get right. It also puts inventory-derived data in the cell for
no other reason.

**Base-side only — let the base own the whole cascade.** Cache state field,
health and target on `ConnectedClientState` alongside the identity fields.
Rejected: those are cell-authoritative and change per tick. The base would
be caching a value the cell is already broadcasting, and a witness would be
introduced against a stale copy — reintroducing the class of bug this change
exists to fix, just with fresher-looking data.

**Introduce the placeholder and re-introduce later.** Instead of gating,
introduce the loading player immediately and send a corrective cascade once
`InitPlayerState` lands. Rejected: introduction is one-shot by design — the
witness set is what makes it one-shot — and the correction would need a
second, different code path fanned out to every witness who saw the
placeholder. Skipping is a three-line guard; re-introduction is a protocol.

## Consequences

- **A player is invisible to others for the length of their own map load.**
  That is the intended trade. The alternative is being visible as a
  nameless, bodyless blank, permanently.
- **The cell → base message grew a field.** `EnteredAoI` now carries
  `player_data`, and so does `DeferredAoiMsg::EnteredAoI`. Both are
  in-process Rust enums, not wire messages — no protocol change.
- **The witness's view of a player is a snapshot.** It is correct at
  introduction and is then maintained by the existing per-change fan-outs
  (appearance rebroadcast, combat/death state, position). Anything with no
  fan-out of its own does not update until re-introduction — see
  [Known gaps](#known-gaps).
- **Packet shape is unchanged.** Still phase 1 + phase 2, still one cascade
  packet per entity on the live path and one appended body on the deferred
  bundle. The transaction-state rule from
  [mercury-bundle.md](mercury-bundle.md) applies unchanged: safe alongside
  other entities' cascades, never in the same bundle as this entity's
  phase 1.
- **NPCs are byte-identical to before.** `player_data: None` routes to
  `compose_create_entity_cascade_body` exactly as it did, and emits no
  warning.

## Observability

Two new negative-log targets, following
[negative-logging-convention.md](negative-logging-convention.md) Pattern C
(lookup miss). Emitted from `player_ghost::resolve_identity`.

| `target` | Level | `reason` | Meaning |
|---|---|---|---|
| `aoi.player_ghost_incomplete` | WARN | `observee_session_unresolved` | The cell says a player entered AoI but the base cannot resolve its session. Also carries `addr_resolved` to separate "no entity→addr mapping" from "no session at that addr". Falls back to the bare cascade |
| `aoi.player_ghost_incomplete` | WARN | `no_cached_appearance` | Session resolved but has no cached `BeingAppearance`. The ghost is still introduced with name, level and stats — a named entity with no body beats nothing — and gets a body on the next appearance rebroadcast |

| `aoi.witness_broadcast_failed` | WARN | `cell_channel_closed` | `broadcast_to_witnesses` could not reach the cell loop (Pattern A, a formerly silent send). Carries `entity_id` and `method_index`. Other players keep the stale view of the entity until it re-enters their AoI |

The two `aoi.player_ghost_incomplete` rows carry `witness_id` and `entity_id`, so a single query names both
ends of a failed introduction. The row is catalogued in
[observability.md](observability.md) alongside `aoi.create_emit` and
`aoi.create_send_failed`.

## Test coverage

| Suite | What it pins |
|---|---|
| `mercury::aoi::player_ghost::tests::*` | Wire format. `cascade_emits_identity_appearance_and_live_state_in_legacy_order` decodes the body method-by-method against the expected index/arg pairs — it fails if the cascade regresses to the bare shape. `cascade_skips_absent_optionals_but_still_makes_the_ghost_visible` pins every conditional branch. `composed_body_matches_the_standalone_packet_body` is the compose↔build byte-equivalence guard the bundle path requires |
| `base::world_entry::cell_dispatch::player_ghost::tests::*` | The join. A fan-out byte test drives `aoi::entered_aoi` end to end and asserts the packet the **witness** receives carries the **observee's** identity, and that both packets go to the witness and never to the observee. A second test mutates `cached_appearance_args` between two composes to prove the read is at emit time. Two negative-log guards cover both `reason` values via `LogCapture`; one guard asserts the NPC path is byte-unchanged and silent |
| `cell::space_manager::tests::aoi_player_intro::*` | The gate. `loading_player_is_introduced_once_and_only_after_init` is the regression guard — a player created but not yet initialised produces no `EnteredAoI`, and exactly one is produced after init. `player_observee_carries_its_live_state` and `npc_observee_is_introduced_immediately_with_npc_data` pin the two branches of `player_data` |
| `cell::service::base_messages::tests::broadcast_to_witnesses::fans_out_to_witnessing_players_only`, `inventory::appearance::tests::refresh_player_appearance_asks_the_cell_to_fan_out_to_witnesses`, `base::helpers::witness_broadcast::tests::*` | The post-introduction fan-out. Three players in one shared space: the rebuilt `BeingAppearance` reaches the player standing next to the observee, not the observee's own client and not the player across the map, and is flagged `entity_is_player` so it encodes on the SGWPlayer idbase. The base side asserts the broadcast carries exactly the bytes it cached. The closed-channel WARN and the silent no-cell-service case are both pinned |
| `progression::level_up_fanout_tests::*` (live-DB) | The level-up fan-out. A grant that crosses several boundaries hands the cell exactly one `onLevelUpdate` carrying the level that was persisted; a grant that crosses none sends nothing, so ordinary kill XP does not spam every witness. Live-DB because the level is only computed on the persisted-grant path |
| `cell_entity::tests::is_introducible_gates_players_until_connected_and_initialised` | The predicate itself, across all four states: NPC, created-only, connected-not-initialised, fully initialised |

## Known gaps

- **Not validated with two real game clients.** Everything above is pinned
  by unit, wire-format, fan-out byte and negative-log tests plus the legacy
  Python reference. Nobody has stood two accounts next to each other in
  Castle or Harset yet. That is the outstanding step — see the checklist
  below. Until it passes, treat player-to-player visibility as `NT`, not
  `CW`.
- **GMs are introduced as plain players.** `connect_entity` stamps
  `class_id = 0x02` (`SGWPlayer`) for every player
  ([`cell/space_manager/entities.rs:337`](../../crates/services/src/cell/space_manager/entities.rs)),
  never `0x03` (`SGWGmPlayer`). Left alone on purpose: the witness method
  encoding assumes `IDBASE_SGW_PLAYER` for every player ghost, and
  `SGWGmPlayer`'s idbase has not been verified. Changing the class id
  without that verification would break the ghost's method dispatch.
- **Alignment, faction, archetype and name changes do not reach existing
  witnesses.** The legacy setters fan these out alongside the level
  (`SGWBeing.py:636-637`, `647-648`, `658-659`, `669-670`). Cimmeria has no
  runtime path that changes any of the four on a live player today, so the
  introduction-time value stays right; whoever adds one (a rename, a faction
  swap) must send it through `broadcast_to_witnesses` as the level-up does.
- **`onMeleeRangeUpdate` is not sent.** `SGWBeing.createOnClient` emits it
  when `meleeRange != 0` (`SGWBeing.py:509-510`). The Rust ghost cascade
  omits it. Whether a player ever carries a non-zero `meleeRange` is
  **unverified** — if they do, melee reach may read wrong on the witness.
- **The one-shot introduction still has no repair path.** If a witness
  misses or mis-renders an introduction for any other reason — including the
  still-open [invisible-entity-until-relog defect](../gap-analysis.md) —
  nothing re-sends it. This change narrows the window that produces bad
  introductions; it does not add a way to correct one.

## Two-client UAT checklist

Run in Castle or Harset (shared worlds). Two accounts, two clients. Watch
the server log for `aoi.player_ghost_incomplete` throughout — any WARN is a
failure even if the visuals look right.

1. **Both arrive, then look at each other.** Each client sees the other's
   body with the correct armour and weapon, the correct nameplate name, and
   a plausible level.
2. **Movement relays.** One walks a circuit; the other sees continuous
   movement, correct facing, no rubber-banding and no stuck-at-spawn.
3. **Appearance changes rebroadcast.** One holsters, unholsters, and swaps a
   weapon or armour piece. The other sees each change without relogging.
4. **Logout removes the ghost.** One logs out; the other sees the entity
   disappear rather than freeze in place.
5. **Arrival order A — walk-up.** A is standing in the zone, B gates in and
   walks over. B sees A fully, and A sees B fully.
6. **Arrival order B — the reverse.** Restart both, then have A gate in
   while B is already standing there, so the deferred-AoI path drives at
   least one of the two introductions. Both views must still be complete.
   This is the case that regressed most easily during development.
7. **Live state at introduction.** One is mid-combat (or dead) when the
   other arrives. The arriving client sees the combat state / corpse pose
   immediately, not an idle upright avatar that only corrects on the next
   state change.
8. **Load-window gate.** One relogs while the other watches from nearby.
   The relogging player must be absent while loading and then appear
   complete — never as a nameless blank that stays blank.
