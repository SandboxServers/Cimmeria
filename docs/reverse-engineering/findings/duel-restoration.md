# Duel System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire format: binary RTTI + register fns); MEDIUM (lifecycle: reconstructed from .def, no server impl); LOW (timers/range/abort)
> **Sources**: `SGW.exe` Ghidra; `deprecated/python/{base,cell}/SGWDuelMarker.py`; `deprecated/python/{base,cell}/SGWPlayer.py`;
>   `entities/defs/SGWPlayer.def`; `entities/defs/SGWDuelMarker.def`; `python/Atrea/enums.py`;
>   `crates/cell-methods/src/cell/cell_methods/player/social.rs`; `docs/reverse-engineering/findings/duel-wire-formats.md`
> **Tracking issue**: replaces #70

## Completeness assessment

The duel system was **never implemented server-side** in the original game either — both
`SGWDuelMarker.py` files are skeletons (`__init__` + `super()`), and the SGWPlayer duel handlers are
`pass`. The client side is fully shipped and, **as of this 2026-06-20 pass**, confirmed
`duel-wire-formats.md` with no corrections needed — this is now a historical statement, not a
current one: the SS-E1 pass (2026-09-27) later revised `duel-wire-formats.md` (the `pvpFlag` /
`GENERICPROPERTY_PvPFlag` correction, and D-Q5's `onDuelEntitiesSet`/`Remove`/`Clear` closure), so
treat that doc, not this line, as the up-to-date word on the wire format. This is consistent with
SGW's pre-launch cancellation.

| Aspect | % |
|---|---|
| Wire format recovery | ~90% (all shapes confirmed by RTTI names) |
| State machine / server logic | ~2% (two stub handlers) |
| Client-send fanout (onDuelChallenge/EntitiesSet/Remove/Clear) | ~5% (constants only) |
| `SGWDuelMarker` entity | 0% |
| **Overall functional** | **~5%** |

## Wire messages (all confirmed by binary RTTI)

### Client → Server (NetOut)

- `sendDuelChallenge` [base] — `WSTRING aPlayerName` + `INT8 aSquadDuel`. RTTI `0x01df5e18`, register
  `0x00cbee10`, param name `aSquadDuel` @ `0x019afa18`.
- `sendDuelResponse` [cell, CM 102] — `INT8 aResponse` (0=decline,1=accept). Emitter `FUN_00aeafb0`
  (alloc 0xC, set bool→INT8 `aResponse`); Lua bridge `0x00aab030`.
- `duelForfeit` [cell, CM 103] — no args. RTTI `0x01df5e94`, register `0x00cbefc0`.
- `Event_SlashCmd_SetPVP` (`/pvp`) — `INT8 aPvPValue`. `FUN_00e5d450` emits 1-byte payload. Distinct from
  the server-driven duel PvP flag.

### Server → Client (client methods on SGWPlayer)

- `onDuelChallenge` [143] — `INT32 aEntityId` + `ARRAY<INT32> aSquadList`. UI handler `0x00ce3a30` →
  `FUN_00cc18b0` iterates squad list.
- `onDuelEntitiesSet` [151] — `ARRAY<INT32> aEntityList`. register `0x00d89aa0`.
- `onDuelEntitiesRemove` [152] — `INT32 aEntityId`. register `0x00d89d40`.
- `onDuelEntitiesClear` [153] — no args. register `0x00d89fe0`.
- `Event_UI_DuelTimerStart` — `float duration` (handler `0x00ce3a50` → `FUN_00cd65a0(float*)`). **Addition
  not in duel-wire-formats.md.**

## SGWDuelMarker entity

Inherits `SGWSpawnableEntity`. Client entity-type index **6**, matching `entities.xml`'s row order
(SGWSpawnableEntity=0, SGWBeing=1, SGWPlayer=2, SGWGmPlayer=3, SGWMob=4, SGWPet=5,
SGWDuelMarker=6), per the project's standard "wire typeID = clientIndex" rule. **Corrected
2026-09-27 (SS-E1, audit A-47)** — the earlier "index 2" reading of `FUN_00c67420`'s registration
order was wrong; `entities.xml`'s row order is the higher-confidence source.
Properties (CELL_PRIVATE): `duelDetectorID: CONTROLLER_ID = 0`, `duelEntities: ARRAY<MAILBOX>`.
CellMethod: `onEntityDefeated(INT32 entity_id)`. 0/1 methods implemented anywhere.

## State machine (`python/Atrea/enums.py`)

`EDUEL_STATE_*`: None=0, ResponsePending=1, Challenged=2, StartPending=3, Engaged=4.
`EDUEL_DEFEAT_*`: Health=1, LeftSquad=2, Connection=3, Range=4, Teleport=5, InDuel=6, Forfeit=7.
DuelTimer type=14, PvPTimer type=15.

SGWPlayer.def internal cell methods (none implemented): `duelChallenge`, `duelResponse`,
`duelEntityDefeat`, `startSquadDuel`, `duelAbort`, `onDuelDefeat`, `registerDuelMarker`, `startDuel`.

## Lifecycle (reconstructed; MEDIUM confidence)

1. **Challenge**: `/duel <name>` → `sendDuelChallenge(name, squadFlag)` → base resolves name → cell
   `duelChallenge(challengerMailbox, squadMailboxes)` → `onDuelChallenge` [143] + `Event_UI_DuelTimerStart`.
2. **Response**: accept/decline → `sendDuelResponse` [102] → `duelResponse`.
3. **Arena setup**: spawn `SGWDuelMarker`, `registerDuelMarker` + `startDuel` on participants, set
   the PvP flag to 1 and fan it out to AoI witnesses (**provisional**: the vehicle, `GENERICPROPERTY_PvPFlag=4` or the `pvpFlag` property, is unresolved; see open question 7), `onDuelEntitiesSet` [151].
4. **Combat**: PvP flag active; both can damage each other.
5. **Resolution**: on death/forfeit/teleport/disconnect/range → `duelEntityDefeat(mailbox, reason)` →
   marker `onEntityDefeated` → `onDuelEntitiesRemove` [152] → when empty: `onDuelEntitiesClear` [153] +
   reset PvP flag + destroy marker.

**PvP flag (provisional; the vehicle is unresolved, see open question 7)**: one candidate is `GENERICPROPERTY_PvPFlag = 4` via `onEntityProperty(4, INT32)`. Current Rust sends `(4,0)`
at world entry only (`world_data.rs`); no setter to 1 / no duel-time fanout exists.

**Correction 2026-09-27 (SS-E1, D-Q4)**: `SGWPlayer.def` also declares a dedicated `pvpFlag`
property (`INT8`, default 0, `CELL_PUBLIC`) plus internal cell methods `setPvPFlag(INT8 flagValue,
INT8 shouldDoStrikeTeamLogic)` and `startPvPTimer(INT8 flagValue, FLOAT timeLength)`. In SGW, `CELL_PUBLIC`
maps only to `DATA_GHOSTED` (between CellApps) and is not a client-distribution flag (`docs/drafts/spec/entity-property-sync.md:199,221`), so the declaration alone does **not** show that the client receives this property. It is an **unresolved possibility**, not the established PvP-flag vehicle: the receiver and update path for `pvpFlag` has not been traced. Also not
resolved: whether the client's generic-property dispatch has a live case for ordinal 4 at all, or
how client UI reads `pvpFlag` once synced. See `duel-wire-formats.md`'s SS-E1 section for detail.

## Open questions

1. DuelTimer (type 14) — server-started or pure client countdown? No server dispatch site found.
   **Partially closed 2026-09-27 (SS-E1, D-Q1)**: the client applies no hardcoded duration constant
   anywhere between `Event_UI_DuelTimerStart` and the Lua countdown display — whatever float the
   server sends is shown verbatim. See `duel-wire-formats.md`'s SS-E1 section.
2. `duelAbort` semantics on decline/timeout/disconnect — no impl anywhere.
3. Squad-duel scope — can any member challenge, or leader only? (`aSquadDuel` flag client-controlled.)
   The seeded text moniker 875 ("SVR must be leader to challenge") answers this for squad duels:
   leader only (see the 2026-09-27 dueling research report §1.9; out of SS-E1's scope, squad duels
   are deferred).
4. `duelDetectorID` CONTROLLER_ID — implies a trigger-region arena boundary (→ EDUEL_DEFEAT_Range), always
   0 in practice; likely planned-not-wired.
5. `sendDuelResponse` byte: confirm 1=accept (standard Lua truthy). → x64dbg.
6. **`onDuelEntitiesSet`/`Remove`/`Clear` and `GameBeing::isInteractable` — CLOSED 2026-09-27
   (SS-E1, D-Q5, blocking for SS-D2)**: full decompile in `duel-wire-formats.md`'s SS-E1 section.
   151 inserts into, 152 erases from, a `GamePlayer`-local `std::set<int32_t>` of duel-entity ids;
   153 clears it. The interactability computation never reads this set — it is a generic
   per-target-template + range lookup. Sending 151 at duel start and 153 at duel end is safe and
   does not affect NPC interactability; `aoi.rs:203-211`'s comment has the add/erase direction
   backwards and should be corrected.
7. `GENERICPROPERTY_PvPFlag` vs. the client's actual PvP-flag consumption — **still open
   (new candidate found 2026-09-27 (SS-E1, D-Q4))**: see the "PvP flag" correction above. `pvpFlag` and the generic-property
   channel are both still candidates: neither the receiver path nor client-side consumption has been traced.

## Dynamic-analysis needs (x64dbg)

- **D.1** BP `0x00cd65a0` — capture `Event_UI_DuelTimerStart` float (response-window duration).
- **D.2** Range limit — trigger a duel, walk away; find the periodic check that sends `duelEntityDefeat`
  with `EDUEL_DEFEAT_Range=4`.
- **D.3** BP `0x00d694d0` (SGWNetworkManager DuelChallenge handler) — recover the `sendDuelChallenge` base
  msg_id; also watch register at `0x00cbee10`.
- **D.4** BP at `register_NetIn_onDuelChallenge` return (`0x00d89800`) — confirm empty `aSquadList` is
  `00 00 00 00` (count=0) for 1v1.
- **D.5** BP `0x00aeafb0` — confirm `sendDuelResponse` true=Accept via the Lua bridge `0x00aab030`.

## Ghidra annotations

None applied this session. Recommended renames: `FUN_00c67420`→`Entity_RegisterAllTypes`,
`FUN_00aeafb0`→`sendDuelResponse_emit`, `FUN_00e5d450`→`SetPVP_emit`, `FUN_00cd65a0`→`DuelTimerStart_handler`,
`FUN_00cc18b0`→`onDuelChallenge_ui_invoke`, `0x00aab030`→`Lua_duelResponse_bridge`.
