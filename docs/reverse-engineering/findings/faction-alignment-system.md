# Faction / Alignment System

> **Session**: Worker Faction (W-faction), 2026-05-13
> **Binary**: SGW.exe (32-bit x86 PE, MSVC 8.0 / VC80)
> **Status**: Initial findings — wire format confirmed, enum values recovered, binary handlers traced

---

## Overview

SGW uses two orthogonal INT8 properties on every `SGWCombatant`-implementing entity to
classify combat allegiance:

- **`Alignment`** (`UINT8`, `CELL_PUBLIC`) — the player's chosen side (Praxis vs. SGU).
  Drives starting-world selection, character-creation filtering, and the `who` command
  output. Fixed at character creation; not changed at runtime under normal gameplay.
- **`faction`** (`UINT8`, `CELL_PUBLIC`) — the entity's current organizational standing.
  The only value that the combat and interaction systems directly test at runtime is
  **10 (hostile)**: faction 10 NPCs are valid attack targets; all others are not.

Both properties are broadcast server→client via single-byte `ClientMethod` messages
(`onAlignmentUpdate` index 24, `onFactionUpdate` index 25) and stored in the `GameBeing`
C++ object. The binary updates these fields and immediately triggers a visual-state refresh
that re-applies interaction descriptors (nameplate, right-click menu) on the entity.

There are also two GM/admin slash-commands — `GiveFaction` and `SetFaction` — that travel
client→server to mutate faction standing.

---

## Faction Inventory

### `EFaction` enum (DB type `resources.EFaction`)

Source: `db/resources/Social/Types/EFaction.sql`

The enum is ordered; the 0-based index is the wire value sent in `onFactionUpdate`.

| Wire value | Enum name | Notes |
|-----------|-----------|-------|
| 0 | `FACTION_None` | Default neutral |
| 1 | `FACTION_SGC` | Stargate Command — used by neutral NPCs in entity_templates (value 1) |
| 2 | `FACTION_Asgard` | |
| 3 | `FACTION_Free_Jaffa` | Hardcoded in `map_loaded.rs` as the player's own faction during world entry |
| 4 | `FACTION_New_Mind_Goauld` | |
| 5 | `FACTION_Op_CORE` | |
| 6 | `FACTION_Loyalist_Jaffa` | |
| 7 | `FACTION_Ra_Jaffa` | |
| 8 | `FACTION_Straegis` | |
| 9 | `FACTION_Furling` | |
| 10 | `FACTION_Burtonol` | **Combat sentinel** — the only value tested by the fight-or-interact gate |
| 11 | `FACTION_TechCon_Group` | |
| 12 | `FACTION_Tollan` | |
| 13 | `FACTION_Svarog` | |
| 14 | `FACTION_Lucia_Red` | |
| 15 | `FACTION_Lucia_Blue` | |
| 16 | `FACTION_Lucia_Green` | |
| 17 | `FACTION_Lucia_Yellow` | |
| 18 | `FACTION_Ancients` | |
| 19 | `FACTION_Sha` | |
| 20 | `FACTION_Bataur` | |
| 21 | `FACTION_Wan_Lumin` | |
| 22 | `FACTION_Sokar` | |
| 23 | `FACTION_Nox` | |
| 24 | `FACTION_Lord_Yu` | |
| 25 | `FACTION_Anubis` | |
| 26 | `FACTION_Vokos_Ancient` | |
| 27 | `FACTION_Vokos_Furling` | |
| 28 | `FACTION_Vokos_Nox` | |
| 29 | `FACTION_Vokos_Asgard` | |
| 30 | `FACTION_Pen_Lai_Barbarians` | |
| 31 | `FACTION_Pen_Lai_Goauld` | |
| 32 | `FACTION_Sokars_Jaffa_Goauld` | |
| 33 | `FACTION_Kull_Tribesmen` | |

**Critical note**: Wire value 10 is `FACTION_Burtonol` by the enum ordering but is
used everywhere in the codebase with the comment "hostile". This is the only faction
value that the combat system tests (see below). Whether the label is meaningful or
incidental is unknown — `FACTION_Burtonol` may have been chosen as a convenient
"enemy" bucket, or the enum may have been reordered after the code was written.

### Faction values observed in `entity_templates` seed data

From `db/resources/Entities/Seed/entity_templates.sql` (153 mob-class rows):

| Faction value | Count | Interpretation |
|--------------|-------|----------------|
| 10 | 83 | Hostile — combat NPCs |
| 1 | 56 | SGC — neutral/friendly NPCs (vendors, quest givers) |
| 0 | 7 | None — inert world objects or beings with no faction |
| 3 | 2 | Free_Jaffa — used by a small number of NPCs |
| NULL | 12 | No faction set (spawnable props, world objects) |

---

## Alignment Property Semantics

### `EAlignment` enum (DB type `resources.EAlignment`)

Source: `db/resources/Archetypes/Types/EAlignment.sql`

| Wire value | Enum name | Notes |
|-----------|-----------|-------|
| 0 | `ALIGNMENT_Undefined` | Default; not used by playable characters |
| 1 | `ALIGNMENT_Praxis` | Player faction; starts in `Castle_CellBlock` |
| 2 | `ALIGNMENT_SGU` | Player faction; starts in `SGC_W1` |
| 3 | `ALIGNMENT_Side_01` | Reserved; not assigned to any chardef |
| 4 | `ALIGNMENT_Side_02` | Reserved; not assigned to any chardef |
| 5 | `ALIGNMENT_Side_03` | Reserved; not assigned to any chardef |
| 6 | `ALIGNMENT_End` | Sentinel; not a valid game value |

DB constraint: `alignment_sanity CHECK ((alignment >= 0) AND (alignment <= 5))`
(`db/sgw/Players/Tables/sgw_player.sql` line 39).

Alignment is fixed at character creation from the `char_creation` table and stored in
`sgw_player.alignment`. It is sent to the client every time the player's own entity is
initialized (world entry phase 9, direct method 0x98).

The `who` GM slash-command (SGWPlayer CellMethods) transmits a `WSTRING` alignment
parameter to describe the player, but this is a display string, not the numeric enum.

---

## Wire Format

Both `onAlignmentUpdate` and `onFactionUpdate` share an identical single-byte wire layout.

### `onAlignmentUpdate` (SGWCombatant ClientMethod index 24, direct method byte 0x98)

```
Offset  Size  Type   Field      Description
     0     1  INT8   alignment  EAlignment enum value
```

Total: 1 byte.

### `onFactionUpdate` (SGWCombatant ClientMethod index 25, direct method byte 0x99)

```
Offset  Size  Type   Field    Description
     0     1  INT8   faction  EFaction enum ordinal value
```

Total: 1 byte.

**Source**: `entities/defs/interfaces/SGWCombatant.def` lines 182–188,
confirmed by binary string `"faction"` at `0x019d63ac` and
`"alignment"` at `0x019d6364`, referenced from `GameBeing.cpp` handler functions.

---

## Client-Side Handler — `GameBeing.cpp`

Two symmetric functions handle the two events. Both follow the same pattern:

### `onFactionUpdate` handler — `FUN_00e02280` (confirmed `GameBeing.cpp:0x3f6` = line 1014)

1. Calls `FUN_00d434d0(pEventData, "faction", &this->mFaction)` — extracts the INT8 value
   and stores it at `this+0x134` (i.e., `GameBeing::mFaction`).
2. Asserts on extraction failure (debug build string `"aEvent->getProperty<int8>(\"faction\", mFaction)"`
   at `0x019d63c8`).
3. Unconditionally calls `GameBeing_OnDeadStateChanged(this, &1, ESI)`.

### `onAlignmentUpdate` handler — `FUN_00e02180` (confirmed `GameBeing.cpp:0x3ef` = line 1007)

1. Calls `FUN_00d434d0(pEventData, "alignment", &this->mAlignment)` — extracts the INT8
   value and stores it at `this+0x135` (i.e., `GameBeing::mAlignment`).
2. Asserts on extraction failure (debug build string `"aEvent->getProperty<int8>(\"alignment\", mAlignment)"`
   at `0x019d6378`).
3. Unconditionally calls `GameBeing_OnDeadStateChanged(this, &1, ESI)`.

**`GameBeing` field layout (confirmed offsets)**:

| Offset | Field | Type | Notes |
|--------|-------|------|-------|
| `+0x134` | `mFaction` | INT8 | Stored by `onFactionUpdate` handler |
| `+0x135` | `mAlignment` | INT8 | Stored by `onAlignmentUpdate` handler |
| `+0x158` | `bStateField` | UINT32 | State flags (BSF_Dead, BSF_InCombat, etc.) |

**Why `GameBeing_OnDeadStateChanged` on a faction update?** The name is misleading. The
function is a general client visual-state refresh that propagates changes to the UE3
interaction descriptor on the entity (nameplate color, right-click menu). Both faction and
alignment updates trigger it with `pParam = &1` (non-null, meaning "do full refresh").
Source: `GameBeing_OnDeadStateChanged @ 0x00e6e330` — traced to UE3 interaction update
event `Event_Entity_InteractionUpdate`.

---

## Combat / Interaction Gate — Faction 10 is the Hostile Sentinel

The client has a single hard check on faction:

```
FUN_00e719d0: walk entity list → if faction == 10 AND alive → treat as attack target
```

In Cimmeria (`crates/services/src/cell/cell_methods/player/interaction.rs` line 49):

```rust
let is_hostile = space_mgr.get_entity(target_entity_u32).is_some_and(|t| {
    !t.is_player && t.faction == 10 && !is_dead_state(t.state_field)
});
```

And in AoE dispatch (`crates/services/src/cell/abilities/dispatch.rs` line 113):

```rust
const HOSTILE_FACTION: u8 = 10;
// skip NPCs that are not faction 10
if npc.faction != HOSTILE_FACTION { continue; }
```

There is no friendly/neutral/hostile matrix in the binary. The system is binary:
**faction 10 = attack target; everything else = non-target**. The `EFaction` enum's
named entries do not form a relationship graph in the client; they are just identifiers
that the server assigns.

---

## World-Entry Sequence

During `setupPlayer` / `map_loaded`, the server emits:

| Phase | Method | Index | Wire byte | Value sent |
|-------|--------|-------|-----------|------------|
| 9 | `onAlignmentUpdate` | 24 | `0x98` | Player's `alignment` from `sgw_player` (UINT8) |
| 10 | `onFactionUpdate` | 25 | `0x99` | **Hardcoded 3** (`FACTION_Free_Jaffa`) |

Source: `crates/services/src/mercury/world_data/map_loaded.rs` lines 212–215.

The player's own faction is hardcoded to 3 at login. This matches what was captured from
the live server during protocol recording. The `faction` column in `sgw_player` does not
exist — faction is not persisted for players; only NPCs carry it from `entity_templates`.

---

## AoI Enter — NPC Faction Delivery

When a new entity enters the player's AoI, `onAlignmentUpdate` and `onFactionUpdate` are
sent as part of the creation bundle (items 10–11 in the AoI entity-create sequence):

```rust
// crates/services/src/mercury/aoi/create.rs lines 80–81, 166–174
let align = npc_data.map_or(0u8, |d| d.alignment);
let fac   = npc_data.map_or(0u8, |d| d.faction);
// ...
append_entity_method(&mut body, method_idx::ON_ALIGNMENT_UPDATE, entity_id, &[align]);
append_entity_method(&mut body, method_idx::ON_FACTION_UPDATE, entity_id, &[fac]);
```

Both values are loaded from `entity_templates.alignment` and `entity_templates.faction`
in the database.

---

## Disguise System — `disguiseFaction`

The `SGWBeing` interface has a `disguiseFaction` property (`INT8`, `CELL_PUBLIC`,
default `-1`, at `entities/defs/interfaces/SGWBeing.def` lines 91–95).

The `enableDisguise` cell method accepts a `bool faction` parameter (arg 3, `INT8`).
When `disguiseFaction != -1`, the server presumably overrides the entity's visible faction
for other clients — allowing a player or NPC to appear to belong to a different faction
without actually changing their real allegiance.

**The disguise-faction interaction with the combat gate is not confirmed**. Whether
disguised faction changes combat targetability on the server side is unknown.

---

## GM / Admin Commands

### `GiveFaction` (`gmGiveFaction`)

- **Route**: Client slash-command → `Event_SlashCmd_GiveFaction` → SGWTextCommandMgr →
  `Event_NetOut_GiveFaction` → server `gmGiveFaction` cell method on SGWGmPlayer.
- **Addresses**:
  - Slash-command CME emitter vtable: `0x01843804`
  - `Event_SlashCmd_GiveFaction` ctor: `FUN_00593600 @ 0x00593600`
  - `Event_SlashCmd_GiveFaction` dispatch (`vfunc_2`): `Event_SlashCmd_GiveFaction__vfunc_2 @ 0x00593810`
  - `register_NetOut_GiveFaction @ 0x00d96a30` (returns string `"Event_NetOut_GiveFaction"`)
  - Server handler: `gmGiveFaction` cell method on SGWGmPlayer, `0x00d96a30` registration
- **Wire**: No `.def` entry for `gmGiveFaction` was found in `SGWGmPlayer.def`. The command
  appears in the event-net-mapping table (`docs/analysis/event-net-mapping.md` line 482)
  but the `.def` source is not yet confirmed for the argument list.
- **Interpretation**: Likely awards faction reputation points (hence "give" vs. "set").

### `SetFaction` (`gmSetFaction`)

- **Route**: Client slash-command → `Event_SlashCmd_SetFaction` → SGWTextCommandMgr →
  `Event_NetOut_SetFaction` → server `gmSetFaction` cell method on SGWGmPlayer.
- **Addresses**:
  - Slash-command CME emitter vtable: `0x01843820`
  - `Event_SlashCmd_SetFaction` ctor: `FUN_00593880 @ 0x00593880`
  - `Event_SlashCmd_SetFaction` dispatch (`vfunc_2`): `Event_SlashCmd_SetFaction__vfunc_2 @ 0x00593a90`
  - `register_NetOut_SetFaction @ 0x00d96cd0` (returns string `"Event_NetOut_SetFaction"`)
  - Server handler: `gmSetFaction` cell method on SGWGmPlayer, `0x00d96cd0` registration
- **Wire**: No `.def` entry found in SGWGmPlayer.def. See open questions.
- **Interpretation**: Directly sets faction value (hard override vs. incremental).

Both commands are routed through the standard CME SlashCmd → NetOut pattern. The
`SGWTextCommandMgr` holds `MemberCallback` instances bound to both:
`MemberCallbackRtti_SlashCmd_GiveFaction__SGWTextCommandMgr @ 0x00c99ea0` and
`MemberCallbackRtti_SlashCmd_SetFaction__SGWTextCommandMgr @ 0x00c99f20`.

---

## Archetype Faction Defaults (Character Creation)

During character creation, `alignment` is derived from `CharDefId` via the
`char_creation` table / `chardef_lookup` function. The 23 playable CharDefs (IDs 1–23)
split exclusively into:

| Alignment | Int value | Starting world | CharDef IDs |
|-----------|-----------|----------------|-------------|
| `ALIGNMENT_Praxis` | 1 | `Castle_CellBlock` | 1,3,5,7,10,11,13,15,17,19,20,22 |
| `ALIGNMENT_SGU` | 2 | `SGC_W1` | 2,4,6,8,9,12,14,16,18,21,23 |

Archetype/alignment pairings from `chardef_lookup` (Cimmeria `chardef.rs`):

| CharDef range | Archetype | Alignment |
|---------------|-----------|-----------|
| 1/11 Soldier M/F | `ARCHETYPE_Soldier` (1) | Praxis=1, SGU=2 |
| 2/12 Soldier M/F | `ARCHETYPE_Soldier` (1) | SGU=2, SGU=2 |
| 3/13 Commando | `ARCHETYPE_Commando` (2) | Praxis=1, Praxis=1 |
| 4/14 Commando | `ARCHETYPE_Commando` (2) | SGU=2, SGU=2 |
| 5/15 Archeologist | `ARCHETYPE_Archeologist` (4) | Praxis=1, Praxis=1 |
| 6/16 Archeologist | `ARCHETYPE_Archeologist` (4) | SGU=2, SGU=2 |
| 7/17 Jaffa | `ARCHETYPE_Jaffa` (8) | Praxis=1, Praxis=1 |
| 8/18 Shol'va | `ARCHETYPE_Sholva` (7) | SGU=2, SGU=2 |
| 9 Asgard | `ARCHETYPE_Asgard` (5) | SGU=2 |
| 10/19 Goa'uld | `ARCHETYPE_Goauld` (6) | Praxis=1, Praxis=1 |
| 20/22 Scientist | `ARCHETYPE_Scientist` (3) | Praxis=1, Praxis=1 |
| 21/23 Scientist | `ARCHETYPE_Scientist` (3) | SGU=2, SGU=2 |

No players have `ALIGNMENT_Undefined` (0) or the reserved Side_01–03 (3–5) values.
`faction` is not stored for players in the DB; players always receive faction 3 on login.

---

## Hostile / Friendly / Neutral Matrix

The binary implements **no matrix**. There are two combat roles:

| Role | Condition | Examples |
|------|-----------|---------|
| Hostile (attack target) | `faction == 10` AND `!dead` | Combat NPCs, drones |
| Non-hostile (no auto-attack) | `faction != 10` | Vendors, quest givers, fauna, players |

Players cannot be targeted by the faction-10 gate (guarded by `!is_player` check).
PvP, if implemented, would bypass this gate entirely.

The `EFaction` enum's 33 named factions exist for server-side content and mission logic
(dialog filters, loot tables, mission rewards with `ShowFactionChangeIcon` flag) but are
opaque to the client combat system. The client only reads the raw INT8 and stores it.

---

## Client Visualization

Faction and alignment changes both ultimately call `GameBeing_OnDeadStateChanged @ 0x00e6e330`
with `pParam = &1`. That function:

1. Recalculates the entity's combat role by calling into the combat graph
   (`FUN_00e719d0 @ 0x00e719d0`) which uses actual position/AoI data, not faction.
2. Updates bit-flags at `this+0x50/0x54` (entity render-state bitmask).
3. Fires `Event_Entity_InteractionUpdate` — which drives the nameplate right-click menu.

There is no nameplate color function identified in this session. The "color" of the
nameplate is likely driven by the interaction type / state flags at `+0x50/0x54` via
the UE3 UI system, not by a direct faction→color lookup. This remains an open question.

---

## Relation to State-Flag Broadcast

The `faction` and `alignment` properties are `CELL_PUBLIC` — they are part of the BigWorld
property synchronization stream and are broadcast to all entities in AoI (not just the
owning client). The state-flag broadcast system (`state-flag-broadcast.md`) covers the
separate `bStateField` INT32 property.

`onAlignmentUpdate` and `onFactionUpdate` are **ClientMethod calls**, not property sync —
they are RPC-style one-shot messages triggered by the server, distinct from the BigWorld
property-delta broadcast. The binary stores them in `mFaction / mAlignment` fields
(`this+0x134 / this+0x135`) on `GameBeing`, which are separate from the BigWorld property
mirror used for initial sync.

---

## Cross-References

| Topic | Document |
|-------|----------|
| Wire format of the 1-byte messages | `docs/reverse-engineering/findings/combat-wire-formats.md` §onAlignmentUpdate / onFactionUpdate |
| World-entry sequence (phases 9–10) | `docs/reverse-engineering/findings/world-entry-pipeline.md` table row 9–10 |
| Method index constants | `crates/services/src/cell/client_methods/combatant.rs` |
| AoI NPC delivery | `crates/services/src/mercury/aoi/create.rs` lines 166–174 |
| Hostile combat gate | `crates/services/src/cell/cell_methods/player/interaction.rs` |
| AoE hostile filter | `crates/services/src/cell/abilities/dispatch.rs` |
| CME EventSignal architecture | `docs/reverse-engineering/findings/cme-event-signal.md` |
| State-flag broadcast | `docs/reverse-engineering/findings/state-flag-broadcast.md` |
| Ability resolution | `docs/reverse-engineering/findings/ability-resolution-pipeline.md` |
| Dialog faction filters | `db/resources/Dialogs/Tables/dialog_set_maps.sql` columns `alignments`, `factions` |
| Mission `ShowFactionChangeIcon` | `db/resources/_triggers.sql` (missions table) |
| Content-engine faction condition | `crates/content-engine/src/conditions.rs` `FactionCheck` variant |

---

## Open Questions

1. **GiveFaction vs. SetFaction argument list** — Neither `gmGiveFaction` nor `gmSetFaction`
   appears in `SGWGmPlayer.def`. The commands are mapped in `event-net-mapping.md` but the
   exact parameters (target entity ID? faction ID? amount?) are not confirmed. Binary
   addresses `0x00d96a30` and `0x00d96cd0` are the registration stubs only.

2. **Nameplate color mapping** — `GameBeing_OnDeadStateChanged` fires
   `Event_Entity_InteractionUpdate` which drives some visual state, but the exact
   nameplate color logic (red for hostile, green for friendly, etc.) was not traced to a
   function in this session. Likely in the UE3 UI/HUD layer.

3. **Faction 3 (Free_Jaffa) for players** — The `map_loaded.rs` comment says
   "hardcoded 3 (from setupPlayer)". It is not confirmed whether the original server
   Python actually sent a constant 3 or derived it from player data. If 3 was meaningful
   (distinguishing SGU-side players from Praxis-side players), the hardcode may be wrong.

4. **Disguise-faction interaction** — `SGWBeing.disguiseFaction` (INT8, default -1) is
   not traced. Whether it overrides `mFaction` on the receiving client (and thus changes
   combat targetability) is unknown.

5. **EFaction numeric mapping** — The wire values are the 0-based ordinal position in the
   `EFaction` SQL ENUM, confirmed by the entity_templates seed data showing `faction=10`
   for hostile mobs. Postgres ENUM ordinal ordering is confirmed. However, there is no
   binary enum table traced — the claim rests on the SQL schema + seed data convergence.

6. **Side_01 / Side_02 / Side_03 alignments** — These are in the EAlignment schema but
   have no CharDef assignments. They may be placeholders for future content or for
   non-player factions (Goa'uld / Jaffa enemy squads that have "alignment" in a broader
   story sense).

7. **Content-engine `FactionCheck` condition** — `crates/content-engine/src/conditions.rs`
   has a `FactionCheck { faction: String, relation: FactionRelation }` condition type.
   The `relation` enum and how it maps to the numeric faction values is not documented.
   This powers mission/dialog gating by faction standing — a separate reputation system
   from the simple hostile/neutral binary in combat.
