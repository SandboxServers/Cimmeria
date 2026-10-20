# T-08e: per-player phase visibility (design note)

Status: **design, offline evidence only**. Nothing here has run against a live client. Marks used:
**BlockedDecision** (owner must choose), **UATPending** (needs the native client), **Proven offline**
(code or RE finding cited). bigworld-engine-advisor, 2026-10-17; reviewer: aoi-witness-broadcast.

## 1. Verdict

**Client-safe: yes.** A phase is expressed only as *AoI membership*. A player who is not in an
entity's phase never has that entity in its witness set, so the client receives exactly the
messages it already receives when the entity is out of range: nothing, or `ENTITY_INVISIBLE (0x0B)`
+ `LEAVE_AOI (0x0C)` when the phase drops, and the normal `CREATE_ENTITY` + createOnClient
cascade when the phase is gained. No new opcode, no new entity method, no `.def` change, no client
patch. The client cannot distinguish "out of phase" from "out of range".

Why that holds, in BigWorld terms: the client's `EntityManager` has no notion of *why* an entity
entered or left its AoI. Stock BigWorld drives `enterAoI`/`leaveAoI` purely from the cell's
witness, and the cell is free to apply any rule it likes (range, LoD, team visibility). CME never
shipped a phasing system that the RE has found, so we are not reconstructing a lost primitive; we
are using the one lever the protocol always gave the server.

Single-cell simplification: Cimmeria runs one cell process per space, so there are no ghost
entities and no cell handoff. A phase filter does not have to be replicated to ghosts or carried
across a cell boundary. If a multi-cell CellApp is ever built, the filter must run wherever the
witness's AoI is computed (the real entity's cell), never on the ghost.

## 2. What the client tolerates (evidence)

Source: `docs/reverse-engineering/findings/client-entity-lifecycle.md` (Proven offline).

| Case | Client behaviour | Safe? |
|---|---|---|
| Entity leaves while in range | `leaveAoI` `0x00dd2800` decrements the enter count; at 0 the entity is destroyed (model, target selection, nameplate) | Yes. Same as walk-away |
| Same entity id re-enters | `enterAoI` `0x00dd24f0` bumps the count; `createEntity` `0x00dd2270` enters world when count >= 1 | Yes, **provided a LeftAoI separated the two Enters** |
| Enter twice with no Leave | a second CREATE for an id already in world trips the `getEnterCount() > 0` assert path (double create) | **No.** Invariant I1 below |
| Method arrives for an entity not in world | queued by `EntityManager` until the entity enters | Harmless, but the server must not rely on it (I3) |
| Targeted/selected entity leaves | entity destroyed, selection dropped client-side | Yes. Server mirrors with `drop_target_if(.., "target_left_aoi")` (#844) |
| `requestEntityUpdate (0x07)` | sent on every non-player enter; server ignores ids in the witness set, refuses others | Yes. Doubles as anti-probe (§5.3) |

Production already exercises leave-then-re-enter of the same id: every walk-away/walk-back past
the 150 m / 175 m hysteresis, and the NPC respawn re-create in
`crates/cell/src/cell/service/ticks/npc_respawn/mod.rs` (LeftAoI, then `introduction_events`,
same entity id). Phase toggles add no new client path. (Proven offline.)

Ruled out: `ENTITY_INVISIBLE` alone plus `onVisible(1)` (method 8), the ring-transport fade in
`crates/wire/src/mercury/aoi/leave.rs::build_entity_invisible`. The client keeps the entity
record, so it stays targetable by tab-cycle and keeps receiving method calls. That is a visual
trick, not isolation.

### Invariants the implementation must hold

- **I1. No double Enter.** For each (witness, entity) pair, wire events alternate Enter, Leave,
  Enter. The witness set is the single source of truth; a phase change only adds or removes from
  it through the existing AoI diff, never by emitting `EnteredAoI` directly.
- **I2. Leave before re-create.** A phase gain while the entity is already in the witness set is
  a no-op on the wire.
- **I3. No fan-out to non-witnesses.** Every entity-method and property broadcast already goes
  through witness sets (`get_witnesses_of`, base `BroadcastToWitnesses` in
  `crates/base-session/src/base/helpers/witness_broadcast.rs`). An out-of-phase player is not a
  witness, so it gets nothing. Code that bypasses witness sets (a scan over `space.players`) must
  apply the phase test itself (§5).
- **I4. Ordering under deferred AoI.** During a cinematic hold the base defers AoI events
  (`crates/base-session/src/base/deferred_aoi_lifecycle.rs`). `cancel_enter_leave_pairs` drops an
  Enter followed by Leave of the same id; `lifecycle_segments` keeps Leave(X) then Enter(X) as two
  ordered segments. A phase toggle inside a hold is therefore safe. (Proven offline.)
- **I5. Players are never phased in v1** (BlockedDecision D-PH1). Player-to-player visibility,
  chat, groups, trade and duels stay untouched.

## 3. Data model

### 3.1 Entity side

`CellEntity` (`crates/entity/src/cell_entity/entity_struct.rs`) gains:

```rust
/// Phase this entity lives in. `None` = visible to everyone (every entity today).
pub phase: Option<PhaseKey>,
```

`PhaseKey` is a newtype over a short ASCII string (`[a-z0-9_]{1,64}`), defined in
`crates/entity/src/cell_entity/phase.rs` (new). It is set at spawn from the spawn row (§3.4) or by
a content `spawn_entity` action carrying a phase. It never changes after spawn in v1.

### 3.2 Player side

```rust
/// Phases this player is in. Empty for every player today.
pub phases: HashSet<PhaseKey>,
```

Rule (BlockedDecision D-PH2, recommended option A):

- **A (recommended): show-in-phase.** A phased entity is visible to a player only if the player
  holds that phase. An unphased entity is visible to everyone.
- B: also support "hide-in-phase" (an entity visible only to players *not* in the phase, for a
  before/after world swap). Adds a `PhaseRule::{Show, Hide}` on the entity. Not needed by 621.

Visibility predicate, the one function every gate calls:

```rust
// crates/cell-world/src/cell/space_manager/phase.rs (new)
impl SpaceManager {
    /// Whether `viewer` may see, target or touch `other` under phasing.
    /// Players are never phased (I5); unphased entities are always visible.
    pub fn phase_visible(&self, viewer: &CellEntity, other: &CellEntity) -> bool;
}
// Free-function form for code holding two &CellEntity and no SpaceManager:
pub fn phase_visible(viewer: &CellEntity, other: &CellEntity) -> bool;
```

NPC viewers (an NPC "seeing" another entity) always return true in v1: NPC AI does not compute
witness AoI, and the aggro gate in §5.4 handles the NPC-to-player direction.

### 3.3 Persistence across relog and world transfer

New schema file **`db/sgw/Players/Tables/sgw_player_phases.sql`** (not a `db/scripts` migration;
copies the `sgw_player_tutorials.sql` pattern):

```sql
CREATE TABLE sgw_player_phases (
    player_id  integer NOT NULL,
    phase_key  character varying(64) NOT NULL,
    set_at     timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT sgw_player_phases_pkey PRIMARY KEY (player_id, phase_key)
);
```

- FK `player_id -> sgw_player(player_id) ON DELETE CASCADE` goes in `db/sgw/_foreign_keys.sql`
  (pattern: `sgw_player_content_cooldown`).
- Include `\ir sgw/Players/Tables/sgw_player_phases.sql` in `db/database.sql` next to line 451
  (`sgw_player_tutorials.sql`).
- **Load:** the base reads the rows at world entry, beside
  `crates/base-world-entry/src/base/world_entry/shown_tutorials.rs`, and ships them in
  `InitPlayerState` (`crates/wire/src/cell/messages/base_to_cell.rs`, new field
  `phases: Vec<String>` next to `shown_tutorials`). The cell hydrates `CellEntity.phases` in the
  player-init handler (`crates/cell/src/cell/service/base_messages/player_init/`).
- **Write:** cell to base `CellToBaseMsg::SetPlayerPhase { player_id, phase, member: bool }`,
  answered by `BaseToCellMsg::PlayerPhaseRecorded { phase, outcome }` with outcome
  `Inserted | Removed | Unchanged | Refused`. Mirrors `RecordTutorialShown` / `TutorialRecorded`
  in `crates/wire/src/cell/messages/tutorial_content.rs`. The base handler lives beside the
  tutorial arm in `crates/base-world-entry/src/base/world_entry/cell_dispatch/mod.rs`.
- **Order:** the cell applies the change to `CellEntity.phases` and recomputes AoI immediately
  (the player sees the result at once). The DB write is asynchronous. A `Refused` reply logs
  `phase.persist_refused` at WARN and leaves memory as is for the session; a relog then shows the
  DB truth. This is the same trade the tutorial set makes.
- **World transfer:** phases are per character, not per world, so they survive a cross-world
  teleport and relog because `InitPlayerState` re-ships them. Phased entities are spawned per
  world, so a `svarog_vault` phase is inert outside world 19. (BlockedDecision D-PH4 offers
  per-world scoping instead.)
- Alternative with no table (BlockedDecision D-PH3, option B): derive the phase from mission state
  (`in_phase(svarog_vault)` = mission 621 active and step 2112 reached, 2766 not done). No
  schema, no stuck state possible, but every phase needs bespoke derivation code and the
  content-engine verbs in §7 become read-only. Recommendation: option A (table), because it is
  generic and T-09 needs explicit clearing anyway.

### 3.4 Spawn-side phase

`db/resources/Worlds/Tables/spawnlist.sql` gains a nullable column `phase_key character
varying(64)` (seed-table schema in `db/resources`, not a migration). NULL for every existing row.
The spawner reads it into `SpawnRecord` (`crates/cell-catalog/src/cell/spawner/`) and stamps
`CellEntity.phase` at spawn. BlockedDecision D-PH5: column on `spawnlist` (recommended) versus a
phase tag convention on the existing `tag` column (no schema change, but overloads `tag`, which
content chains already use for `despawn_entity` / `set_visible` lookups).

## 4. The filter seam

**File:** `crates/cell-world/src/cell/space_manager/aoi.rs`, function `compute_player_aoi`.
The candidate loop already skips non-introducible entities:

```rust
if !other.is_introducible() { continue; }
// add:
if !phase::phase_visible(witness, other) { continue; }
```

placed before the distance test, so an out-of-phase entity is never a candidate. The existing
diff then does the rest: an entity in the old witness set and not in the new one becomes
`LeftAoI`; the reverse becomes `EnteredAoI` with `introduction_events` / `push_introduction`
(the full cascade). The `aoi.entity_enter` / `aoi.entity_leave` telemetry and the
`aoi_introductions_total`, `aoi_leaves_total` and `aoi_witness_set_size` metrics fire unchanged.

**Immediate recompute.** After a phase change the cell calls
`SpaceManager::compute_aoi_changes_for_player(player_id)` (same file) and forwards its events the
way the AoI tick does, so the safe appears without waiting for the next tick. New entry point:

```rust
// crates/cell-world/src/cell/space_manager/phase.rs
impl SpaceManager {
    /// Add or remove `phase` for `player_id`, then recompute that player's AoI.
    /// Returns the AoI events to forward; empty when membership did not change.
    pub fn set_player_phase(&mut self, player_id: u32, phase: &PhaseKey, member: bool)
        -> Result<Vec<AoiEvent>, PhaseError>;
}
```

`PhaseError::{PlayerMissing, NotAPlayer}`. Logs `phase.changed` at INFO with `player_id`,
`player_name`, `phase`, `member`, `entered`, `left` counts.

**Leave side effects come free.** The diff already calls
`target_lifetime::drop_target_if(.., "target_left_aoi")`, so a player targeting the safe when the
phase is cleared loses the target server-side as the client destroys it. Despawn and
`disconnect_entity` (`space_manager/entities.rs`) scrub witness sets from the entity side and are
phase-agnostic.

**Respawn.** `npc_respawn/mod.rs` re-creates through `get_witnesses_of`, i.e. only players who
already hold the entity. Out-of-phase players never do, so respawn needs no change. (Proven
offline by reading; P1 adds a test.)

**Do not touch** the reverse lookup `get_witnesses_of` / `has_witness` in
`space_manager/queries.rs`; it is correct by construction once witness sets are filtered.

## 5. Refusing interaction, combat and abilities

The witness set gates the wire. The handlers below act on a client-supplied entity id and do
**not** consult the witness set, so a modified client could still name the safe's id. Each gets a
`phase_visible` check. A shared helper keeps the log shape uniform:

```rust
// crates/cell-world/src/cell/space_manager/phase.rs
pub fn refuse_out_of_phase(space_mgr: &SpaceManager, actor_id: u32, target_id: u32,
                           surface: &'static str);   // WARN, target "phase", event phase.refused
```

| # | Surface | File / function | Gate position | Refusal on wire | Telemetry |
|---|---|---|---|---|---|
| 5.1 | Interact (right-click / use) | `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs::handle_interact` | **first**, before the faction-10 hostile reroute to `right_click_attack` and before `interact_target_in_range` | none (same silence as a missing target) | `interaction.out_of_phase` + `phase.refused surface=interact` |
| 5.2 | useAbility | `crates/cell-combat/src/cell/abilities/use_ability/fire_los.rs::fire_line_of_sight` (new `FireLos::OutOfPhase`, checked beside `FireLos::OtherSpace`) | in `handle_use_ability` with the #906 other-space check | `onErrorCode(0, ability_id, 0)` (CONDITION_FEEDBACK_INVALID_ENTITY), reusing `other_space_error_args` | `cast_refused reason=target_out_of_phase`; `RefusalReason::TargetOutOfPhase` in `crates/cell-combat/.../metrics/mod.rs` beside `TargetOtherSpace` |
| 5.3 | requestEntityUpdate | `crates/cell/src/cell/service/base_messages/request_entity_update.rs::refusal_cause` | new cause before `outside_aoi` | none (already refused) | existing WARN `reason=not_in_witness_aoi cause=out_of_phase` |
| 5.4 | NPC aggro on players | `crates/cell-combat/src/cell/service/npc_ai/aggro_gates.rs::evaluate_candidate` (+ `assist.rs::evaluate_assister`) | after `gm_ignores_aggro` | n/a | new `AggroReject::OutOfPhase`, logged like `GmIgnored` |
| 5.5 | Splash / area damage | `crates/cell-combat/src/cell/abilities/damage_apply/ammo_splash.rs::splash_targets` and any other collector that scans `space.players` or the grid | filter candidates | n/a | `phase.refused surface=splash` at DEBUG (per-tick volume) |
| 5.6 | Dialog and chain anchors | `crates/cell-content/src/cell/content/dialog/mod.rs` (the witness reads near lines 305 and 479) and `step_anchor.rs` (near line 100) | where the speaker or anchor is resolved | none | `phase.refused surface=dialog` / `surface=anchor` |
| 5.7 | Loot of a phased corpse/bag | the loot open handler (`OpenLoot` action and client loot request) | target resolution | existing loot refusal path | `phase.refused surface=loot` |
| 5.8 | GM console targeting | `target_in_view` callers in cell-console | none; `target_in_view` reads the witness set and is already correct | n/a | none |

5.5 to 5.7 matter only if a phased entity is an NPC or drops loot. For 621 the phased entity is a
non-combat prop, so 5.1, 5.2 and 5.3 are the load-bearing gates; the rest go in for correctness
because the primitive is generic. Spatial chat (`broadcast_to_witnesses` in cell-console) is
player-to-player and unaffected by I5.

Negative-logging convention (Pattern C, `docs/architecture/negative-logging-convention.md`):
every refusal is logged with actor and target identity fields, never silently dropped.

## 6. Two-player semantics

Players A and B stand together at the Curia. A has phase `svarog_vault`; B does not.

- A sees the safe; B does not. B's witness set never contains it, so B's client has no record.
- A sees B and B sees A (I5). B watching A sees A interact with empty air. That is the accepted
  cost of phasing in every MMO of the era and matches the 621 text ("hidden safe").
- B names the safe's id: interact is refused (5.1), useAbility gets `onErrorCode` (5.2),
  requestEntityUpdate is refused with `cause=out_of_phase` (5.3).
- Property or method broadcasts *from* the safe (an open animation, a state change) reach A only:
  fan-out is through witness sets (I3).
- A clears the phase: A gets ENTITY_INVISIBLE + LEAVE_AOI for the safe; B gets nothing.
- Group play: phases are per character. A grouped B must reach step 2112 and use their own device
  to see the safe. Whether a group shares phases is BlockedDecision D-PH6 (recommended: no, v1).

## 7. Content-engine verbs

`crates/content-engine/src/actions.rs` (templates: `ShowTutorial` near line 530, `SetVisible`):

```rust
SetPhase { phase: String },    // JSON: {"type":"set_phase","phase":"svarog_vault"}
ClearPhase { phase: String },  // JSON: {"type":"clear_phase","phase":"svarog_vault"}
```

`crates/content-engine/src/conditions/mod.rs` (template: `TutorialShown`, lines 169 and 420):

```rust
InPhase { phase: String, #[serde(default = "default_true")] present: bool },
// JSON: {"type":"in_phase","phase":"svarog_vault"} / {"...","present":false}
```

- Context: `crates/content-engine/src/context.rs` gains `phases: Option<HashSet<String>>` and
  `with_phases(..)` beside `shown_tutorials` / `with_shown_tutorials`. `InPhase` evaluated with
  `phases == None` fails closed and WARNs, exactly like `TutorialShown`.
- Loader validation: phase strings must match `[a-z0-9_]{1,64}`; a bad key rejects the chain at
  load with the loader's existing error type.
- Executor: `crates/cell-content/src/cell/content/executor/phase.rs` (new, beside
  `executor/tutorial.rs`) calls `SpaceManager::set_player_phase`, forwards the AoI events, and
  sends `SetPlayerPhase` to the base. Actor is always the triggering player; a chain with no
  player actor logs `phase.no_actor` and does nothing.
- `spawn_entity` (`actions.rs` line 112) gains optional `phase: Option<String>` so a chain can
  spawn a phased entity without a seed column.

Telemetry names (all under target `phase`): `phase.changed`, `phase.refused`,
`phase.persist_refused`, `phase.no_actor`, `phase.loaded` (DEBUG, at InitPlayerState with the
count). Metric: `phase_changes_total{member}`.

## 8. Minimal mission-621 use (T-09 wires it; T-08e only provides the verbs)

Steps from `db/resources/Missions/Seed/mission_steps.sql` lines 5671-5679, all `step_enabled`
false today: 2110 Head to the Curia; 2111 Obtain a phase device from Svarog; 2112 Use the Phase
Device to find the hidden safe; 2766 Obtain the Gate Address and Activation Code from within
Svarog's safe; 4103 Bring them back to Harset.

1. Spawn row: `curia_wall_safe` prop in world 19 with `phase_key = 'svarog_vault'` (§3.4).
   **UATPending / HELD**: no world-19 coordinate yet (`tollana-placement-contract.csv`, T-01).
2. 2111: Svarog's dialog grants the phase device item (existing `grant_item`).
3. 2112: chain on `on_item_use { item_id: <device> }` with condition mission 621 at step 2112 and
   `in_phase(svarog_vault, present=false)`: action `set_phase(svarog_vault)` then
   `advance_step(621, 2112)`. Effects 2336/2337 from T-01 (§105-111 of
   `T-01-source-map-contract.md`) are the device's visual and can be applied with `apply_effect`.
4. 2766: chain on `on_interact_tag { entity_tag: "curia_wall_safe" }` with `in_phase(svarog_vault)`:
   grant the address/code item, advance the step. The interact gate (5.1) already refuses
   out-of-phase players; the condition is belt and braces.
5. **No stuck phase:** `clear_phase(svarog_vault)` on 2766 completion, and chains on
   `on_mission_abandoned(621)` and `on_mission_completed(621)`. A relog mid-step keeps the phase
   (table), so the player can still open the safe. BlockedDecision D-PH7 decides whether 2766
   completion or 621 completion clears it (recommended: 2766, the safe has no further use).
6. Prerequisite skipping: the device-use chain checks step 2112; the safe chain checks the phase,
   which only step 2112 grants. No path reaches 2766 without 2112.

## 9. Packets

Each packet is sized at about 40k working tokens for **rust-gameserver-dev** (not Haiku). Every
packet: one worktree and one test database, CRLF preserved in docs, files under 500 lines (split
along seams), tests that fail when the change is reverted, docs rows from
`docs/agents/doc-update-map.md` listed in the report. Lane commands run from the packet worktree
(`Set-Location <wt>;` in each call).

### PH-01: phase model + AoI filter + immediate recompute

- Files: `crates/entity/src/cell_entity/phase.rs` (new: `PhaseKey`, validation),
  `entity_struct.rs` and `construction.rs` (fields, default empty),
  `crates/cell-world/src/cell/space_manager/phase.rs` (new: `phase_visible`,
  `set_player_phase`, `refuse_out_of_phase`, `PhaseError`), `space_manager/aoi.rs` (one filter
  line in `compute_player_aoi`), `space_manager/mod.rs` (module).
- Tests (`crates/cell-world/src/cell/space_manager/tests/phase.rs`, new):
  - unit: `phase_visible` truth table (unphased, phased+member, phased+non-member, player
    target never phased);
  - AoI: phased prop 10 m from player, no phase, no `EnteredAoI`; `set_player_phase(true)` yields
    exactly one `EnteredAoI`; second `set_player_phase(true)` yields none (I1/I2); `false` yields
    one `LeftAoI` and drops a target on it;
  - two-player: A in phase, B not, same spot; only A's witness set holds the prop; B's tick over
    50 iterations never introduces it;
  - respawn: prop re-create reaches A only (drive the respawn path or assert `get_witnesses_of`).
- Revert proof: delete the filter line in `aoi.rs`; the no-phase and two-player tests fail.
- Lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-world` and
  `-p cimmeria-entity`.

### PH-02: refusal gates and telemetry

- Depends on PH-01.
- Files: `interact.rs::handle_interact` (5.1); `use_ability/fire_los.rs`, `use_ability/handle.rs`,
  combat `metrics/mod.rs` (5.2); `request_entity_update.rs::refusal_cause` (5.3);
  `npc_ai/aggro_gates.rs`, `npc_ai/assist.rs` (5.4); `damage_apply/ammo_splash.rs` (5.5);
  cell-content `dialog/mod.rs`, `step_anchor.rs` (5.6); loot resolution (5.7). Before editing,
  grep `space.players` and `get_witnesses_of` across `crates/cell*` and list every collector that
  bypasses witness sets in the report; gate each or justify skipping it.
- Tests: one per surface: out-of-phase interact does not reach `fire_interact_tag` and logs
  `interaction.out_of_phase`; useAbility returns `false` and emits `onErrorCode(0, id, 0)` bytes
  (assert the encoded method call); `refusal_cause` returns `out_of_phase`; a hostile phased NPC
  ignores an out-of-phase player and aggroes an in-phase one; splash skips the out-of-phase
  player. Use `tracing-test` or the repo's log-capture helper for event names.
- Revert proof: drop the gate in `handle_interact` and in `fire_line_of_sight`; those tests fail.
- Lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-cell-methods`, then
  `-p cimmeria-cell-combat`, `-p cimmeria-cell`, `-p cimmeria-cell-interactions`,
  `-p cimmeria-cell-content`.

### PH-03: persistence (table, hydration, record/recorded)

- Depends on PH-01. Blocked on D-PH3 (table versus derived) and D-PH4 (world scoping).
- Files: `db/sgw/Players/Tables/sgw_player_phases.sql` (new), `db/sgw/_foreign_keys.sql`,
  `db/database.sql` (include beside line 451); `crates/wire/src/cell/messages/phase.rs` (new:
  `SetPlayerPhase`, `PlayerPhaseRecorded`, outcome enum), `base_to_cell.rs`
  (`InitPlayerState.phases`), `cell_to_base.rs`, `cell_to_base_kind.rs`, `messages/mod.rs`;
  `crates/base-world-entry/src/base/world_entry/player_phases.rs` (new: load + upsert/delete,
  beside `shown_tutorials.rs`), `world_entry/cell_dispatch/mod.rs` (arm),
  `world_entry_appearance/client_ready/player_init_row.rs` (ship the set); cell player-init
  handler (hydrate). Lab fixture: add phases to `crates/lab/src/uat/fixture.rs` and
  `snapshot.rs` only if the tutorial fields are mirrored there (they are).
- Tests: wire round-trip for both messages and the new `InitPlayerState` field (old payload
  without the field must still decode if the codec is versioned; otherwise update every
  constructor); base live-DB test (repo's live-DB harness, see the `lane-build` skill):
  insert, duplicate insert is `Unchanged`, delete is `Removed`, row survives a simulated relog
  and comes back in `InitPlayerState`; cell test: hydrated phases make the prop enter on the
  first AoI tick after player init (relog persistence); world-transfer test: phases survive a
  `CrossWorldTeleport` re-init.
- Revert proof: remove the hydration line; the relog test fails.
- Lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-wire`, then
  `-p cimmeria-base-world-entry`, `-p cimmeria-cell`. Live-DB tests per the `lane-build` skill.

### PH-04: content-engine verbs + spawn phase

- Depends on PH-01; executor persistence call depends on PH-03 (stub to a log until it lands).
  Spawn column blocked on D-PH5.
- Files: `crates/content-engine/src/actions.rs` (`SetPhase`, `ClearPhase`, `SpawnEntity.phase`),
  `conditions/mod.rs` (`InPhase`), `context.rs` (`phases`, `with_phases`), loader validation;
  `crates/cell-content/src/cell/content/executor/phase.rs` (new), executor dispatch,
  `mission_context.rs` (populate `phases`); `db/resources/Worlds/Tables/spawnlist.sql`
  (`phase_key`), `crates/cell-catalog/src/cell/spawner/` (read column, stamp entity).
- Tests: serde round-trip for the three verbs; loader rejects `"Svarog Vault"`; `InPhase` with
  `phases == None` fails closed; chain replay test (pattern:
  `chain_replay_tests/mission_622_core_tutorial.rs`): item use runs `set_phase`, the prop enters,
  `on_interact_tag` passes `in_phase`, `clear_phase` makes it leave; spawner test: a row with
  `phase_key` produces an entity with `phase = Some(..)`, NULL produces `None`.
- Revert proof: make the executor a no-op; the chain replay test fails at the prop entering.
- Lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-content-engine`, then
  `-p cimmeria-cell-content`, `-p cimmeria-cell-catalog`.

### PH-05: wire-format and two-client proof

- Depends on PH-01..04.
- Files: `crates/wire/src/mercury/aoi/tests.rs` (or a sibling `phase_tests.rs`): toggle produces
  `0x0B` + `0x0C` (WORD_LENGTH 8, entity id, cacheStamp 0), re-gain produces a fresh
  `CREATE_ENTITY` and cascade, never two creates in a row for one id;
  `crates/wireclient/tests/it/two_client_phase_visibility.rs` (new, pattern:
  `two_client_castle_visibility.rs`): two wire clients in world 19 (or a test world), A runs the
  phase chain, assert A decodes CREATE for the prop and B never does; A clears, A decodes LEAVE.
  Base two-player test beside
  `crates/base-world-entry/.../tests_dispatch_arms/two_player_visibility.rs`.
- Revert proof: remove the PH-01 filter line; B decodes the prop's CREATE and the test fails.
- Lane: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-wire`, then
  `-p cimmeria-wireclient`, `-p cimmeria-base-world-entry`.

PH-01 first; PH-02, PH-03 and PH-04 can run in parallel after it (separate worktrees, PH-04 stubs
the base write); PH-05 last. Mission-621 seed wiring (§8) stays in T-09.

## 10. Owner decisions (BlockedDecision)

| Id | Question | Recommendation |
|---|---|---|
| D-PH1 | May players be phased (players invisible to each other)? | No in v1 (I5) |
| D-PH2 | Show-in-phase only, or also hide-in-phase? | Show only; add Hide when content needs it |
| D-PH3 | Persist phases in `sgw_player_phases`, or derive from mission state? | Table |
| D-PH4 | Phases per character (survive world transfer) or per world? | Per character |
| D-PH5 | `spawnlist.phase_key` column, or a tag convention? | Column |
| D-PH6 | Do group members share phases? | No in v1 |
| D-PH7 | Clear `svarog_vault` on step 2766 or on mission 621 completion? | 2766, plus abandon/complete |

## 11. Live-client checks (UATPending)

- U-PH1: the safe appears on `set_phase` with no client assert or log error, is interactable, and
  disappears on `clear_phase`; repeat the toggle 5 times (I1 under a real client).
- U-PH2: target the safe, clear the phase; selection and nameplate drop cleanly.
- U-PH3: two clients side by side; B never sees the safe, sees A, sees no stray effect.
- U-PH4: relog mid-step 2112..2766; the safe is visible on the first frame after load.
- U-PH5: toggle the phase during a cinematic hold (I4) and confirm ordered delivery.
- Placement itself (the world-19 coordinate) is HELD on T-01 and must be lab-confirmed.

## 12. Fallback (O14-21)

If U-PH1 shows the client does not tolerate the toggle (no evidence suggests it will not; it is
the respawn path), the O14-21 fallback is a **per-player instanced room**: step 2112 teleports the
player into a private instance of the vault (existing instance and `CrossWorldTeleport` /
`Teleport` machinery), where the safe is an ordinary unphased prop, and 2766 completion teleports
them back. It preserves 621's objective and two-player isolation with no AoI change, at the cost
of a load screen and an instance definition. It needs client and authority review per O14-21 and
no client patch.
