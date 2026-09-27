# Duel System Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `SGWPlayer.def`, `alias.xml`

---

Defined directly on `SGWPlayer.def`.

### Client → Server

#### `sendDuelChallenge` (Base Method, Exposed) — Issue Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aSquadDuel` | `INT8` | 1B — 1 for squad duel |

#### `sendDuelResponse` (Cell Method, Exposed) — Accept/Decline

| Field | Type | Size |
|-------|------|------|
| `aResponse` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `duelForfeit` (Cell Method, Exposed) — Surrender

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

### Server → Client

#### `onDuelChallenge` — Incoming Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityId` | `INT32` | 4B — challenger entity ID |
| `aSquadList` | `ARRAY<INT32>` | 4B count + N×4B — challenger's squad |

#### `onDuelEntitiesSet` — Duel Participants

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onDuelEntitiesRemove` — Participant Defeated

| Field | Type | Size |
|-------|------|------|
| `aEntityId` | `INT32` | 4B |

#### `onDuelEntitiesClear` — Duel Ended

*(No arguments)* — 1 byte

---

## SS-E1 client evidence (2026-09-27)

> Static Ghidra RE against `SGW.exe` (no debugger), for the social-systems campaign's SS-E1 packet.
> Answers D-Q1–D-Q6 from `docs/analysis/social-systems/work-packets.md` (D.1/D.2/D.6/D.7 from
> `duel-restoration.md`'s open questions). **D-Q5 is the blocking question for SS-D2** and is
> covered first and in full.

### D-Q5 (BLOCKING) — `onDuelEntitiesSet`/`Remove`/`Clear` and `GameBeing::isInteractable` (CLOSED)

The three addresses named in the packet (`0x00d89aa0`, `0x00d89d40`, `0x00d89fe0`) are only the
CME `register_NetIn_*` name-string functions, not the handlers. Each event's actual client
subscriber was traced through the standard CME plumbing: register fn → the event's static
`TypedEmitInfo` object → the bound-listener chain → the concrete `GamePlayer` method, the same
pattern documented in `spec.engine.cme-event-signal`. All three are bound to **`GamePlayer`**
(confirmed by the mangled `MemberCallback<..., GamePlayer, void (GamePlayer::*)(EventType const*,
void*), EventType>` vtable names at `0x019d6bc8`/`0x019d6be4`/`0x019d6c00`), registered together
in `GamePlayer`'s own subscribe-all function `FUN_00e07780`.

**`onDuelEntitiesSet` [151] → `GamePlayer` handler `FUN_00e07250`** (assert string cites
`.\Src\GamePlayer.cpp` line `0x69`): decodes the `ARRAY<INT32> aEntityList`, and for **each** id
in it, **inserts** the id into an ordered container at `GamePlayer + 0x16c` — confirmed by
`FUN_00c6bd20`, a classic MSVC `std::map`/`std::set` red-black-tree insert (the `<0x11>`-sentinel
traversal pattern) — then, for that same id, looks the entity up by id and calls
`GameEntity__unknown_00e6e330(entity, /*flag=*/1)` on it.

**`onDuelEntitiesRemove` [152] → `GamePlayer` handler `FUN_00e07440`** (`GameEntity.cpp` line
`0x79`): decodes a single `INT32 aEntityId`, then **erases** that one id from the **same**
`GamePlayer + 0x16c` container — confirmed by `FUN_00e083a0`, an erase-by-key on the same
tree/map — then does the **identical** `GameEntity__unknown_00e6e330(entity, /*flag=*/1)` call on
that entity.

**This directly contradicts the reading in `aoi.rs:203-211`'s comment** ("`onDuelEntitiesRemove`
… adds to this set"): the decompile shows the opposite — **151 (Set) is the one that adds ids to
the local player's own duel-entity set; 152 (Remove) erases one.**

**`onDuelEntitiesClear` [153] → `GamePlayer` handler `FUN_00e071c0`**: swaps the whole container
(the `+0x16c/+0x170/+0x174` field group) for a freshly-constructed empty one — an O(1) STL-style
clear — then walks the just-emptied former contents and, for each id that had been in it, does a
find-entity-by-id + the identical `GameEntity__unknown_00e6e330(entity, 1)` cleanup (the per-id
callback stub at `0x00e06fb0`, confirmed by disassembly: `FUN_00c66ad0` → `FUN_00dd0de0`
find-by-id → `GameEntity__unknown_00e6e330(entity, 1)`).

**The interactability decision itself never reads the `+0x16c` set.**
`GameEntity__unknown_00e6e330` is a generic "recompute this entity's cached interaction flags"
routine: it calls a virtual predicate on the entity (`this+0x34` vtable slot) and, if true, calls
`FUN_00e719d0(interactionManager, targetAsGameBeing, localPlayerAsGameBeing)` to classify the
interaction, writing the tri-state 0/1/2 result into bits on the entity's own flags field
(`this+0x54`). **`FUN_00e719d0` is a purely generic, per-target-*template* lookup**: it fetches
interaction-rule candidates keyed by the target's own template/type id (`*(target+0xc)`, via a
`map.find`-style lookup, `FUN_00e73190`), then geometrically range-tests the local player's
position against each candidate rule, and returns the matched rule's stored interaction-type value
(or `-1` if none match / out of range). **No reference to `GamePlayer+0x16c` (the duel-entity set)
appears anywhere in this chain.** Membership in the duel-entity set has no bearing on
interactability; only the *act* of calling insert/erase/clear forces a recompute of whatever the
target's rule-driven interactability already is.

**What the server may safely send, and why:**

- **AoI's existing use of 152 (`onDuelEntitiesRemove`) on every interactable NPC works only as a
  side effect of the forced recompute**, not because of set semantics: sending 152 for an id that
  was never inserted (which is always true for NPCs, since AoI never calls 151) is a harmless
  no-op `erase()`, and the recompute call re-derives the NPC's already-correct
  template/range-based interactable flags. This is unrelated to duel bookkeeping.
- **Because the local player's `+0x16c` set only ever contains what 151/152/153 put there, and
  AoI never calls 151, sending 151 (Set) with the two real duel-partner entity ids at duel
  engage, and 153 (Clear) at duel end, is SAFE**: it cannot affect any NPC's interactability
  (the classification never consults the set), and it cannot leave stray NPC ids in the set for
  153 to "clean up" incorrectly, because NPC ids are never in it. **D-SS25's caution can be lifted
  for 151/153**: SS-D2 may implement issue #569's original plan (151 on start, 153 on end) as
  written. AoI's use of 152 should be left exactly as it is — it is not touching the duel set in
  any way that matters.
- The one thing SS-D2 must still get right: 151/153 only ever need to name the **two duelists'
  own player entity ids** (not any NPC), matching what the client actually consumes.

**Correction needed in `crates/cell-world/src/cell/space_manager/aoi.rs:203-211`**: the comment
should say 152 *erases* from the set (and that the fix works via the forced interaction-flags
recompute, not set membership), not that it *adds*. Recommend SS-D2 or a follow-up carries this
comment fix alongside its own change to that area.

Evidence (all `ghidra://SGW.exe@<addr>`): `0x00d89aa0`/`0x00d89d40`/`0x00d89fe0` (register fns,
ruled out as the handlers); MemberCallback vtables `0x019d6bc8`/`0x019d6be4`/`0x019d6c00`;
subscribe chain `0x00e07780` → `0x00e07ea0`/`0x00e07f30`/`0x00e07fc0` → `0x00e07bb0`/`0x00e07c30`/
`0x00e07cb0`; handler bodies `0x00e07250` (Set), `0x00e07440` (Remove), `0x00e071c0` (Clear);
set insert `0x00c6bd20`, set erase `0x00e083a0`; per-id refresh `0x00e6e330`
(`GameEntity__unknown_00e6e330`); classification `0x00e719d0`; Clear's per-id cleanup stub at
`0x00e06fb0` (disassembled directly, not a named function).

### D-Q1 (D.1) — countdown duration (CLOSED — no client constant)

`DuelTimerStart_handler` (`0x00cd65a0`) forwards the wire `Event_UI_DuelTimerStart(float
duration)` value unmodified through one more native hop (`FUN_00cc19a0` → `FUN_00ccd720`, a
generic native→Lua event-argument marshal) straight into `DuelMod.showDuelTimerText(time)`
(already read by the earlier research report, which counts down `N, N-1, …, 1` with a beep each
second). **No hardcoded client duration constant exists anywhere in this chain** — the countdown
length is entirely whatever float value the server puts on the wire. D-SS18's provisional 5-second
countdown is therefore project policy, not a recoverable number; any value can be chosen without
a wire-format concern. (The server-side `onTimerUpdate`/`ETimerUpdateType.DuelTimer=14` dispatch
was not re-traced this session — it is already recorded in agent memory
`timer-system-extended.md` and did not need re-deriving.)

### D-Q2 (D.2) — range constants (UNRESOLVED)

Did not find a challenge-range or in-duel leash-range constant this session (time budget). D-SS19's
20-unit/40-unit provisional values remain project policy pending a future pass;
`MAX_INTERACT_DISTANCE = 5` (`deprecated/python/common/Constants.py:125`) is still only an
inference-by-analogy from other proximity checks, not a duel-specific confirmation.

### D-Q3 (D.6) — death vs. duel-end branch (UNRESOLVED — moot for implementation)

Did not trace a client-side death-vs-duel-end branch this session. This is now moot for
implementation purposes: D-SS20 (non-lethal clamp at 1 HP, no death/loot/respawn) is an
**owner-approved** decision regardless of what the original client would have done, so SS-D3
should proceed on D-SS20 as written.

### D-Q4 (D.7) — `GENERICPROPERTY_PvPFlag` (PARTIAL — important correction)

**`SGWPlayer.def` declares a real, dedicated `pvpFlag` property, not just the generic-property
side-channel the existing restoration doc assumes:**

```xml
<pvpFlag>
    <Type>  INT8  </Type>
    <Default>  0  </Default>
    <Flags>  CELL_PUBLIC  </Flags>
</pvpFlag>
```

`CELL_PUBLIC` marks this as an ordinary entity property declared through the **standard
entity-property-change mechanism** (the same declaration style as every other `CELL_PUBLIC`
field, per `spec.protocol.entity-property-sync`) — a different mechanism in kind from the small
`EGenericProperty` array/`onEntityProperty(propId, value)` side-channel that
`GENERICPROPERTY_PvPFlag = 4` belongs to. **This is a declaration-level contrast only — it is
not evidence that witnesses actually receive `pvpFlag` updates.** As the next paragraph notes,
SGW's `CELL_PUBLIC` maps only to `DATA_GHOSTED`, and client delivery on this field is unproven
until the receiver and update path are traced.
`SGWPlayer.def` also declares two internal (non-`Exposed`) cell methods immediately
next to it:

```xml
<setPvPFlag>
    <Arg> INT8 </Arg> <!-- flag value -->
    <Arg> INT8 </Arg> <!-- should do strike team logic -->
</setPvPFlag>

<startPvPTimer>
    <Arg> INT8 </Arg>  <!-- flag value -->
    <Arg> FLOAT </Arg> <!-- time length -->
</startPvPTimer>
```

This is a new discovery not reflected in any existing finding doc. It is an **unresolved possibility**, not evidence of a client-facing fanout. In SGW, `CELL_PUBLIC` maps only to `DATA_GHOSTED` (`docs/drafts/spec/entity-property-sync.md:221`; `crates/entity/src/cell_entity/entity_struct.rs:79-80`), so the declaration does not show that witnesses receive `pvpFlag`. Until the receiver and update path is traced, the candidates remain `pvpFlag` and the
`GENERICPROPERTY_PvPFlag`/`onEntityProperty` mechanism `crates/wire/src/mercury/world_data/
map_loaded.rs:334` currently uses to send `(4, 0)` once at world entry. `setPvPFlag`'s second
argument ("should do strike team logic") also connects this property to the *world/organization*
PvP opt-in system (`OrganizationMember.def`'s `pendingPvPTimers`/`pvpOrganizationLeaveResponse`),
not only to duels — consistent with `duel-wire-formats.md`'s existing note that the two PvP
mechanisms are easy to conflate.

**Not resolved this session**: (a) whether the client's generic-property dispatch even has a live
case for ordinal 4 today (i.e. whether `GENERICPROPERTY_PvPFlag` is dead code or a second,
redundant path), and (b) how the client's targeting/nameplate/cursor code actually reads
`pvpFlag` once synced. **Recommend SS-D2 verify which of the two mechanisms (the `pvpFlag`
property vs. the generic-property channel) the client actually reacts to before building on
`map_loaded.rs`'s existing assumption** — this may change the fan-out code, not just the
`duel-restoration.md` write-up.

Evidence: `entities/defs/SGWPlayer.def` (`pvpFlag`, `setPvPFlag`, `startPvPTimer` — file:line, not
a Ghidra address; `pvpFlag` and `setPvPFlag` sit between `knownPetAbilities`/`isBankingOverride`
and `tradeCancel`/`startPvPTimer` respectively in the current def).

### D-Q6 — how the client renders duel text monikers 872–880 (UNRESOLVED — one candidate ruled out)

Checked `onErrorCode` (`SGWPlayer.def`: `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID`,
bound to a free function keyed on `Communicator*`, confirmed via the `MemberCallback`/
`FreeCallback` vtable at `0x019b9884`) as the leading candidate named in the packet — **ruled
out**: its argument shape (`EErrorCodeSystem` + a subsystem-instance id + `EConditionHandlerFeedback`
code) matches ability/condition-system feedback, not a flat "render moniker N" lookup, and I found
no evidence of a generic string-table resolution keyed by a bare int in the time available.

Given that mail's own client-side validation errors (M-Q1, above) are **all literal hardcoded
WSTRINGs**, never moniker ids, and `SGWPlayer.def` has no dedicated "render duel moniker N" wire
method, the more likely mechanism — **not independently confirmed this session** — is that the
(never-built) original duel server would have sent these `texts.sql` strings as plain WSTRING
feedback over an existing generic channel (`onPlayerCommunication` on `CHAN_feedback`, matching
the GM-feedback and mail-error convention already in use elsewhere), rather than through a
client-side numeric-moniker resolution. **Recommend SS-D1/D2/D3 send these as literal feedback
text** (as the ledger's own packet descriptions already do, e.g. "text 877") rather than build a
moniker-id wire path pending stronger evidence.

## Correction to this file (A-47)

The `SGWDuelMarker` entity type index in "SGWDuelMarker entity" below is wrong; see
`duel-restoration.md`'s corrected entry (audit A-47, SS-E1 2026-09-27): the client entity-type
index is **6**, matching `entities.xml`'s row count, not 2. The `FUN_00c67420` registration-order
read that produced "2" was not re-verified this session; `entities.xml`'s row order plus the
project's standing "wire typeID = clientIndex" rule (`CLAUDE.md`,
`reference_entity_typeid_clientindex`) is the higher-confidence source and is treated as correct.
