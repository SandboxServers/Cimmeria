# Auto-Cycle (Auto-Fire) Button — Protocol and Behavior Reference

**Status**: Confirmed  
**Confidence**: HIGH (binary + Python canonical source + entity def confirmed)  
**Ghidra anchors**: `ghidra://SGW.exe@0x00d68250`, `ghidra://SGW.exe@0x00e01c90`, `ghidra://SGW.exe@0x00e05fb0`  
**Related files**: `crates/services/src/cell/cell_methods/player/world.rs`, `crates/entity/src/abilities.rs`, `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/AbilityManager.py`, `entities/defs/SGWPlayer.def`

---

## Wire Path — Button to Handler

### 1. Client-side UI binding

The gun-icon button on the bottom-right HUD is handled by **`USGWTargetIndicator`**, the UE3 HUD widget class responsible for the targeting reticle and combat-mode indicators. The RTTI string `".?AV$MemberCallback@XVUSGWTargetIndicator@@P81@AEXPBUEvent_UI_AutoCycle@@PAX@ZU2@@EventSignal@CME@@"` at `ghidra://SGW.exe@0x01e6b538` confirms that `USGWTargetIndicator` subscribes to **`Event_UI_AutoCycle`** — the CME event that carries the toggle state into the client UI layer.

The button press travels via the CME event bus through **`SGWScriptedWindow`** (`GameEventHandler<Event_UI_AutoCycle>` at `ghidra://SGW.exe@0x00ce9b90`) then serialises onto the Mercury wire as **`Event_NetOut_SetAutoCycle`** (`ghidra://SGW.exe@0x019b3e90`).

There is also a slash-command path: **`Event_SlashCmd_toggleAutoCycleAbility`** (`ghidra://SGW.exe@0x01842480`) handled by `SGWTextCommandMgr`, which feeds the same `Event_NetOut_SetAutoCycle` emission. The player could type `/toggleAutoCycleAbility` as a keyboard shortcut alternative.

### 2. Method index: 83 (`setAutoCycle`)

The `Event_NetOut_SetAutoCycle` network dispatch resolves to **cell method index 83**, confirmed by:

- `entities/defs/SGWPlayer.def` lines 701–704: `<setAutoCycle><Exposed/><Arg>INT8 enabled</Arg></setAutoCycle>`
- `crates/services/src/cell/cell_methods/player/constants.rs:21`: `pub const SET_AUTO_CYCLE: u16 = 83;`
- RTTI string `"setAutoCycle"` at `ghidra://SGW.exe@0x019c2e6c`
- `docs/protocol/cell-method-dispatch-table.md:289`: entry 83 confirmed

### 3. Wire format

```
Offset  Size  Type   Field     Description
0       1     uint8  header    methodID | 0x80  (= 0x80 | 83 = 0xD3)
1       1     int8   enabled   0 = disable auto-cycle, 1 = enable auto-cycle
```

**Total payload: 2 bytes** (smallest possible SGWPlayer cell method call).

Source: `docs/reverse-engineering/findings/combat-wire-formats.md` §setAutoCycle, cross-confirmed against `SGWPlayer.def`.

### 4. Server-side handler (Cimmeria)

`crates/services/src/cell/cell_methods/player/world.rs`, `dispatch()` match arm `SET_AUTO_CYCLE`:

```rust
SET_AUTO_CYCLE => {
    if !args.is_empty() {
        let enabled = args[0] != 0;
        if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
            entity.abilities.auto_cycle = enabled;
            if !enabled {
                entity.abilities.auto_cycle_ability_id = None;
            }
        }
    }
    true
}
```

---

## Server-Side Behavior — Original Intent

### The two entry points for auto-cycle

The original design had **two separate ways** to enter auto-cycle mode:

1. **Button press (`setAutoCycle(enabled=1)`)** — the explicit toggle. The client sends method 83 when the player presses the gun-icon button. This sets `BSF_AutoCycling` (bit 1, mask `0x002`) on the player's state field and arms the server-side `autoCycle` flag. It does **not** immediately fire an ability — it only changes the mode for the *next* cooldown expiry.

2. **`interact` against a hostile NPC** — the implicit path. When `interact` resolves to a hostile target, `SGWPlayer.py:1175–1178` sets `BSF_AutoCycling`, picks the weapon ability from the bandolier item events, and calls `abilities.launchAbility(..., autoCycle=True)`. This is the right-click path the user currently experiences.

The `setAutoCycle(enabled=1)` button path is the *explicit override* — the player wants to stay in auto-fire mode without needing to right-click again after the current ability fires.

### The auto-cycle loop — server-driven, cooldown-gated

The key function is `AbilityManager.abilityCooledDown()` in `deprecated/python/cell/AbilityManager.py:965–979`:

```python
def abilityCooledDown(self, ability: Ability):
    del self.cooldownTimers[ability.id]
    self.entity().onAbilityCooledDown(ability)

    if self.autoCycle:
        target = self.entity().space.findEntity(self.entity().targetId)
        if target is None or target.isDead():
            self.autoCycle = False
            self.entity().stoppedAutoCycling()
        else:
            self.launchAbility(self.autoCycleAbility, self.entity().targetId,
                               autoCycle=True, isEntityAbility=False)
```

**The loop is entirely server-driven.** The sequence is:

1. `interact` or `setAutoCycle(1)` + manual `useAbility` → arms `autoCycle=True`, stores `autoCycleAbility`.
2. Ability fires → server starts cooldown timer.
3. Cooldown expires → `abilityCooledDown()` callback fires on the server.
4. Server checks: is `autoCycle` still set? Is the target still alive?
5. If yes: server directly calls `launchAbility` again. No client request. No packet from the client side.
6. Client receives the resulting `onTimerUpdate` + `onSequence` + `onEffectResults` packets from the re-fired ability, same as a manual fire.

The client is a **passive recipient** of the loop. The only round-trip is the initial `setAutoCycle(1)` toggle. Everything after that is server-side timer callbacks re-firing the ability.

### `auto_cycle_ability_id` — what it tracks

`auto_cycle_ability_id` (Rust) / `autoCycleAbility` (Python) stores the `ability` object (Python) or `ability_id` integer (Rust) that is being looped. This is set at `launchAbility(autoCycle=True)` call time and **not** by the client. The client does not send an ability ID with `setAutoCycle` — the `enabled` byte is the entire payload.

When `setAutoCycle(enabled=1)` arrives as a standalone button press (not from `interact`), the ability to cycle is implicitly the last one fired at the current `targetId`. The Python design assumed `setAutoCycle(1)` would arrive *after* a manual `useAbility` had already stored `autoCycleAbility`; the button merely kept the loop running rather than starting it from scratch.

### Ability flag gates

Two ability flags interact with auto-cycle (from `deprecated/python/Atrea/enums.py` and `crates/entity/src/abilities.rs`):

| Flag | Value | Meaning |
|------|-------|---------|
| `DoNotActivate_AutoCycle` / — | `512` (`0x200`) | When this flag is set on an ability, `interact` will NOT set `BSF_AutoCycling` even against a hostile target. |
| `Deactivate_AutoCycle` / `AF_DEACTIVATE_AUTO_CYCLE` | `1024` (`0x400`) | When an ability with this flag fires (via `launchAbility`), auto-cycle is immediately cancelled and `stoppedAutoCycling()` is called. Used for one-shot abilities that should break the loop. |

A manual `useAbility` call (outside `autoCycle` path) also cancels auto-cycle: `AbilityManager.useAbility()` sets `autoCycle = False` on entry (`AbilityManager.py:1019`).

### `stoppedAutoCycling()` and `BSF_AutoCycling` clear

When auto-cycle stops (target dead, manual override, `Deactivate_AutoCycle` flag), the server calls `SGWPlayer.stoppedAutoCycling()` (`SGWPlayer.py:1084–1088`), which calls `unsetStateFlag(BSF_AutoCycling)`. The state-field change is broadcast to the client, causing `FUN_00e01c90` (address `ghidra://SGW.exe@0x00e01c90`) to fire with delta bit 1 set, which calls `FUN_00e05fb0` (`ghidra://SGW.exe@0x00e05fb0`). That function emits the `Event_UI_AutoCycle` CME event, which notifies `USGWTargetIndicator` to un-highlight the button.

### `autoCycleTimerID` entity property

`SGWPlayer.def:183–187` defines a `CELL_PRIVATE` `CONTROLLER_ID` property `autoCycleTimerID`. This is a BigWorld timer handle — the Python server stored the timer reference here so it could cancel the pending cooldown callback (e.g., on death or target-lost). It is server-private; the client never sees it.

### Target acquisition — server-side, not client-driven

The server re-fires at `self.entity().targetId`, the server-stored target entity ID (set on `interact` or `setTargetID`). The client does not send a target with the auto-cycle re-fires. When the client sends `setAutoCycle(1)` explicitly, it is the player's way of saying "keep firing at whatever target I last attacked" — the server already knows the target from the prior `interact` call.

---

## State-Field Handshake

`BSF_AutoCycling` (bit 1, mask `0x002`) in the `bStateField` INT32 property is the client-visible signal. The client's `FUN_00e01c90` XOR-delta handler at `ghidra://SGW.exe@0x00e01c90` tests `delta & 0x002` and calls `FUN_00e05fb0` on any transition. This emits `Event_UI_AutoCycle` into the CME bus, which `USGWTargetIndicator` and `SGWScriptedWindow` subscribe to — driving the button highlight state and any combat-mode cursor change.

No explicit `onTimerUpdate` handshake is required to start or stop the loop. The loop itself drives `onTimerUpdate` on every ability fire (the cooldown timer packet is part of the normal ability-fire sequence, not specific to auto-cycle).

---

## Implementation Guidance for Cimmeria

The current Cimmeria implementation handles `setAutoCycle(enabled)` correctly at the **message-receipt layer** — it stores `auto_cycle` and clears `auto_cycle_ability_id` on disable. What is missing is the **cooldown-expiry re-fire loop** that makes auto-cycle actually shoot. Specifically:

### What is missing

1. **The cooldown-expiry auto-cycle check.** When `handle_use_ability` starts a cooldown in `use_ability.rs`, it does not check `auto_cycle` at cooldown expiry. The ability's cooldown timer expires inside `start_ability_cooldown` / `AbilityManager::cleanup_expired` but there is no callback that re-invokes `handle_use_ability` when the timer lapses and `auto_cycle == true`.

2. **`auto_cycle_ability_id` is never set on enable.** `SET_AUTO_CYCLE` with `enabled=1` does not set `auto_cycle_ability_id` — it only sets the flag. The ability ID needs to be stored at the time the *first* `useAbility` fires with auto-cycle intent, not at `setAutoCycle(1)`. For the `interact` path this would be the bandolier weapon ability; for the button-press path it is the ability that was last fired.

3. **`BSF_AutoCycling` is never set in the Rust path.** `setAutoCycle(1)` stores `auto_cycle = true` on `AbilityManager` but does not set bit 1 of `entity.state_field`. The state-field bit drives the client button highlight and `USGWTargetIndicator` update. Without it the button has no visual feedback.

4. **`stoppedAutoCycling()` equivalent is missing.** When the loop terminates (target dead, explicit disable, `AF_DEACTIVATE_AUTO_CYCLE` ability fires), the server must clear `BSF_AutoCycling` from the state field and broadcast the change so the client un-highlights the button.

### Recommended implementation sketch

The cleanest approach, matching the original Python model:

**A. At `use_ability.rs` fire-time:** when the call succeeds and `entity.abilities.auto_cycle == true`, store the fired `ability_id` in `entity.abilities.auto_cycle_ability_id`. This is the "lock in the cycled ability" step.

**B. In the per-entity tick** (wherever cooldown completion is checked — the same tick that handles `reload_complete_at`, `pending_reload_at`, etc.): after a cooldown entry for an ability expires, check:

```rust
if entity.abilities.auto_cycle {
    if let Some(cycle_id) = entity.abilities.auto_cycle_ability_id {
        if target is still alive {
            // re-invoke handle_use_ability with cycle_id and stored target_id
        } else {
            entity.abilities.auto_cycle = false;
            entity.abilities.auto_cycle_ability_id = None;
            // clear BSF_AutoCycling and broadcast state field
        }
    }
}
```

**C. `BSF_AutoCycling` state-field management:**

- Set bit 1 of `entity.state_field` when `auto_cycle` goes `true` (both from `setAutoCycle(1)` and from `interact` kicking off a cycle).
- Clear bit 1 and broadcast `onStateFieldUpdate` when the loop stops.
- The existing `set_state_flag` / `broadcast_state_field` infrastructure already handles this for `BSF_InCombat`; the same pattern applies.

**D. `AF_DEACTIVATE_AUTO_CYCLE` gate in `use_ability.rs`:** check the ability's `flags & AF_DEACTIVATE_AUTO_CYCLE` (constant `1024`, already defined as `cimmeria_entity::abilities::AF_DEACTIVATE_AUTO_CYCLE`) when launching. If set, cancel auto-cycle before firing.

**E. `interact` path:** the Rust `interact` handler should set `auto_cycle = true` and `auto_cycle_ability_id` to the weapon ability (matching `SGWPlayer.py:1176–1178`). This is the normal entry path — the standalone button is the override.

### What does NOT need to change

- `SET_AUTO_CYCLE` disable handling is correct: it clears both `auto_cycle` and `auto_cycle_ability_id`.
- `USE_ABILITY` should continue to cancel auto-cycle when called directly (matching Python `AbilityManager.useAbility:1019`).
- Wire format is correct: 1-byte `int8 enabled` payload.

---

## Open Questions

| # | Question | Evidence needed |
|---|----------|-----------------|
| OQ-1 | Does `setAutoCycle(1)` as a standalone button press (without a prior `interact`) require a prior target to be set, or does it effectively no-op until the player attacks something? | Check client Lua/SWF to see if the button is only enabled when a hostile is targeted. The Python code implies it would fire into `autoCycleAbility = None` and do nothing useful — `launchAbility(None, ...)` would crash — so presumably the button is only clickable when already in combat. |
| OQ-2 | What was `startAutoCycleAbility` (a base method in `SGWPlayer.def:694`) called by, and how does it differ from `setAutoCycle`? It has no args — is it a server-to-client signal or a server-internal trigger? | No Python implementation found in the deprecation tree. The def entry has no `<Exposed/>` tag, so it was a server-to-server or internal call, not a client RPC. Needs Python base-side search or Ghidra for the handler. |
| OQ-3 | Does the client send `setAutoCycle(0)` when the user clicks the button again to toggle off, or does it rely solely on `BSF_AutoCycling` clearing? | The wire format supports it (enabled=0 is a valid message). The Python `setAutoCycle` handles both. Assuming symmetric toggle — confirmed by `SGWPlayer.def` which shows a separate `stopAutoCycle` internal method alongside `setAutoCycle`. |

---

## Summary for Relaying to the User

The gun-icon button is confirmed to be the **auto-cycle / auto-fire toggle** (`setAutoCycle`, cell method 83). Pressing it sends a 2-byte packet to the server (`methodID + enabled:int8`). When enabled, the server is supposed to automatically re-fire the player's current weapon ability at their current target every time that ability's cooldown expires — no further input required. The client just receives the same ability-fire packets it would from a manual shot. The loop stops when the target dies, when the player manually fires a different ability, or when they press the button again to disable it. Currently the Cimmeria server correctly receives and stores the flag but does not have the cooldown-expiry re-fire tick that actually drives the loop, so the button has no effect in gameplay. The fix requires hooking the cooldown-completion tick to check the auto_cycle flag and re-invoke `handle_use_ability` when it's set.
