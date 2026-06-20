# Duel System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire format: binary RTTI + register fns); MEDIUM (lifecycle: reconstructed from .def, no server impl); LOW (timers/range/abort)
> **Sources**: `SGW.exe` Ghidra; `deprecated/python/{base,cell}/SGWDuelMarker.py`; `deprecated/python/{base,cell}/SGWPlayer.py`;
>   `entities/defs/SGWPlayer.def`; `entities/defs/SGWDuelMarker.def`; `python/Atrea/enums.py`;
>   `crates/services/src/cell/cell_methods/player/social.rs`; `docs/reverse-engineering/findings/duel-wire-formats.md`
> **Tracking issue**: replaces #70

## Completeness assessment

The duel system was **never implemented server-side** in the original game either — both
`SGWDuelMarker.py` files are skeletons (`__init__` + `super()`), and the SGWPlayer duel handlers are
`pass`. The client side is fully shipped and confirms `duel-wire-formats.md` with **no corrections
needed**. This is consistent with SGW's pre-launch cancellation.

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

Inherits `SGWSpawnableEntity`. Client entity-type index **2** (confirmed `FUN_00c67420` registration
order: Account=0, SGWSpawnableEntity=1, SGWDuelMarker=2, …). entities.xml type id 6.
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
   `GENERICPROPERTY_PvPFlag=4` to value 1 (fan out to AoI witnesses), `onDuelEntitiesSet` [151].
4. **Combat**: PvP flag active; both can damage each other.
5. **Resolution**: on death/forfeit/teleport/disconnect/range → `duelEntityDefeat(mailbox, reason)` →
   marker `onEntityDefeated` → `onDuelEntitiesRemove` [152] → when empty: `onDuelEntitiesClear` [153] +
   reset PvP flag + destroy marker.

**PvP flag**: `GENERICPROPERTY_PvPFlag = 4` via `onEntityProperty(4, INT32)`. Current Rust sends `(4,0)`
at world entry only (`world_data.rs`); no setter to 1 / no duel-time fanout exists.

## Open questions

1. DuelTimer (type 14) — server-started or pure client countdown? No server dispatch site found.
2. `duelAbort` semantics on decline/timeout/disconnect — no impl anywhere.
3. Squad-duel scope — can any member challenge, or leader only? (`aSquadDuel` flag client-controlled.)
4. `duelDetectorID` CONTROLLER_ID — implies a trigger-region arena boundary (→ EDUEL_DEFEAT_Range), always
   0 in practice; likely planned-not-wired.
5. `sendDuelResponse` byte: confirm 1=accept (standard Lua truthy). → x64dbg.

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
