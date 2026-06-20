# Pet / Companion System — Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (wire formats: .def + binary confirmed); MEDIUM (server lifecycle: minimal Python); LOW (per-instance persistence: no DB schema found)
> **Sources**: `entities/defs/SGWPet.def`; `entities.xml`; `deprecated/python/{base,cell}/SGWPet.py`;
>   `deprecated/python/common/defs/PetCommand.py`; `SGW.exe` Ghidra (`GamePet.cpp`);
>   `crates/services/src/cell/cell_methods/player/social.rs`; `db/resources/AI/Types/EPetStance.sql`;
>   `docs/reverse-engineering/findings/pet-wire-formats.md`
> **Tracking issue**: new (no prior pets issue existed)

## Completeness assessment

`SGWPet` is fully specified (`SGWPet → SGWMob → SGWBeing → SGWEntity`, entity-type index **5**,
`GamePet` struct 436 bytes / `malloc(0x1b4)` @ `0x00c69f90`). The Python server implemented almost
nothing (`cell/SGWPet.py` only has `createOnClient`; `base/SGWPet.py` empty). The three client→server
pet-command stubs in Rust parse correctly but do nothing.

| Layer | Coverage |
|---|---|
| Entity def | 100% |
| Original server (Python) | ~5% |
| Client binary | fully recoverable (`GamePet` class confirmed) |
| Rust server | ~12% (wire skeleton only) |
| **Overall functional** | **~10%** |

## ⚠️ Wire-format corrections to `pet-wire-formats.md`

Two existing-doc errors that will desync the client if implemented as documented:

1. **`onPetStanceList`**: doc says `ARRAY<INT32>` (4B/element). `.def` line 88 declares `ARRAY<INT8>`
   (1B/element). HIGH confidence.
2. **`onPetStanceUpdate`**: doc says `INT32` (5B total). `.def` line 92 declares `INT8` (2B total). HIGH.

## Entity model

`SGWPet` 12 properties (all CELL_PRIVATE/PUBLIC, none PERSISTENT — pet persistence is manual via
`saveToDB`): `ownerID: INT32` (**CELL_PUBLIC**), `ownerBase: MAILBOX` (**CELL_PUBLIC**), `transferXP: FLOAT=1.0`,
`petDespawnTimerId: CONTROLLER_ID`, `abilityToResolve: INT32`, `abilityInformation: PYTHON`,
`toggledAbilities: ARRAY<INT32>` (abilities toggled OFF), `lastOwnerPositionCheck/lastTeleportTime: FLOAT`,
`ownerLastPosition/petLastPosition: VECTOR3`, `petStance: INT8=1`.

`EPetStance`: 0 Passive, 1 Defensive (default), 2 Aggressive.

## Wire messages

### Server → Client (SGWPet entity methods)

- `onPetAbilityList` [client idx 1] — `ARRAY<INT32> abilityIds`. register `0x00d77720`.
- `onPetStanceList` [idx 0] — `ARRAY<INT8>` (see correction). register `0x00d779c0`.
- `onPetStanceUpdate` [idx 2] — `INT8` (see correction). register `0x00d77c60`.

`GamePet` subscribes to all three (RTTI `0x01e261b0`/`0x01e26280`/`0x01e26350`).

### Client → Server (player cell methods — pet commands ride on SGWPlayer, not SGWPet)

- `petInvokeAbility` [88] — `INT32 entityId, INT32 abilityId, INT32 targetId` (12B). `NetOut` `0x019b42a4`.
- `petAbilityToggle` [89] — `INT32 entityId, INT32 abilityId, INT8 toggle` (9B). `NetOut` `0x019b42d8`.
- `petChangeStance` [90] — `INT32 entityId, INT8 stance` (5B). `NetOut` `0x019bc070`.

Rust stub byte widths (12/9/5) **agree** with the .def. `entityId` is the pet's id — ownership must be
verified server-side (spoofing surface).

## Lifecycle

- **Summon**: no `petSummon` method — pets are spawned by **ability resolution**. The ability handler
  creates the SGWPet, sets `ownerID`/`ownerBase`/`abilityToResolve`/`abilityInformation`/`petStance`, spawns
  into AoI, then `createOnClient` → `onPetAbilityList` + `onPetStanceList`. **`Event_NetOut_Summon`
  (`0x019be290`) is the GM `/summon <player>` command — NOT pet summon** (RTTI routes via SGWTextCommandMgr).
- **Despawn**: `petDespawnTimerId` (timer handle) → destroy; or `onOwnerDeath` / `onOwnerLeash`. No explicit
  dismiss message.
- **Follow AI**: poll loop via `lastOwnerPositionCheck`/`ownerLastPosition`/`petLastPosition`; teleport to
  owner when `dist > leash`, rate-limited by `lastTeleportTime`. `onOwnerLeash` forces immediate teleport.
  `onOwnerRespawn(INT8 aShouldDespawn)` decides survive-vs-despawn after owner death.
- **Ability integration**: `toggledAbilities` opt-out list; `sendPetInfoToOwner(MAILBOX, ARRAY<INT32>)`
  pushes stance+ability lists to a new owner.

## Ownership / AoI

`ownerID`/`ownerBase` CELL_PUBLIC → replicated to AoI observers (clients show "X's Pet"). Client supports
**6 party-pet targeting slots** (`Event_Action_TargetPartyPet1..6` @ `0x01840a24`–`0x01840ac4`) — so
`ownerID` must be reliably synced to all party members. `saveToDB(INT32 playerDbId)` persists pet state, but
**no `pets` DB table exists anywhere** (open Q).

## Client UI (confirmed)

`UIPetStance`, `PetCommandAction`/`PetStanceAction`/`PetAbilityAction` action-bar classes; ScrFuncs
`getPetAbilityList`/`getPetCommandList`/`getPetStanceList`/`changePetStance`/`usePetAbility`/`togglePetAbility`;
UI events `UEvent_UI_PetChanged`/`PetStanceChange`/`PetUpdate`.

## Open questions

1. **Pets DB table** — `saveToDB(playerDbId)` exists but no schema; were pets persistent or blob-on-character?
2. Leash teleport threshold distance. → x64dbg.
3. Owner-position poll interval. → x64dbg.
4. Stance change: user-only or also auto-sync? → x64dbg.
5. `onOwnerRespawn` default (despawn vs teleport).
6. `onPetStanceList` element meaning (stance indices vs ability ids). → x64dbg.

## Dynamic-analysis needs (x64dbg)

- Leash threshold / poll interval: watchpoint on `GamePet` `lastTeleportTime` / `lastOwnerPositionCheck`
  fields; capture distance + timestamp deltas.
- BP `0x00d39cb0` (`GamePet` ctor) — construction state (`[0x5b]=5` type index; stance bytes `[0x171..0x173]`
  init `0/0/0xff`).
- BP `register_NetIn_PetStances` (`0x00d779c0`) emit — capture `ARRAY<INT8>` payload (resolves stance-list meaning).
- Inject MercuryLogger and use a summoning ability: capture `createEntity(class_id=5)` + property-sync to
  resolve the wire representation and the `ownerID` sync bytes.

## Ghidra annotations

None applied this session. Recommended renames: `0x00d39cb0`→`GamePet__ctor`,
`0x00c69f90`→`GameEntityFactory_GamePet__Register`, `0x00d66110`→`SGWNetworkManager_PetInvokeAbility_HandlerCtor`,
`0x00d66320`→`…PetChangeStance…`, `0x00d661e0`→`…PetAbilityToggle…`.
