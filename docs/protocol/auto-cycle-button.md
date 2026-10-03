---
title: "Auto-Cycle (Auto-Fire) Button — Protocol and Behavior Reference"
type: reference
audience: engineers
last_updated: 2026-10-03
---

# Auto-Cycle (Auto-Fire) Button — Protocol and Behavior Reference

**Status**: Confirmed  
**Confidence**: HIGH (binary + live-debugger verification + Python canonical source + entity def confirmed)  
**Ghidra anchors**: `ghidra://SGW.exe@0x00aa29c0`, `ghidra://SGW.exe@0x00ad7820`, `ghidra://SGW.exe@0x00e02700`, `ghidra://SGW.exe@0x00e061b0`, `ghidra://SGW.exe@0x00cbbc40`, `ghidra://SGW.exe@0x00e01c90`, `ghidra://SGW.exe@0x00e05fb0`  
**Related files**: `crates/cell-methods/src/cell/cell_methods/player/world/mod.rs`, `crates/cell-combat/src/cell/combat/auto_cycle.rs`, `crates/cell/src/cell/service/ticks/auto_cycle.rs`, `crates/entity/src/abilities/manager.rs`, `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/AbilityManager.py`, `entities/defs/SGWPlayer.def`

---

## Wire Path — Button to Handler

### 1. Client-side trace (verified via live debugger)

The auto-cycle button on the bottom-right HUD is **a CEGUI widget bound to a Lua function**, not a Flash/UnrealScript widget. The Lua function is named `setAutoAttack` (player-facing name); the C side wires it to a Lua-binding shim that constructs the `Event_NetOut_SetAutoCycle` network event directly. Verified by attaching x64dbg to a live SGW client and watching the breakpoint hit on every button click.

```
Lua  setAutoAttack(enabled: bool)           [Lua function bound to the CEGUI widget]
  ↓
0x00aa29c0  Lua_setAutoAttack                [CEGUI Lua binding shim — error string
                                              `"#ferror in function 'setAutoAttack'."`
                                              confirms the binding's Lua name]
  ↓
0x00ad7820  MaybeSendSetAutoCycle(bool)      [RTTI-gates on local controller being a
                                              GameBeing — refuses to send if you have
                                              no live character]
  ↓
0x00e02700  SendSetAutoCycle(bool)           [Allocates Event_NetOut_SetAutoCycle,
                                              sets the "enabled" property to the bool,
                                              emits through CME]
  ↓
0x00e061b0  CME::EventSignal<...>::Emit      [Allocates 0x18-byte TypedEmitInfo,
                                              walks the handler list]
  ↓
0x00cbbc40  TypedEmitInfo<...>::ctor         [Sets dispatch metadata + vftable]
  ↓
(vtable)    SGWNetworkManager::EventHandler  [Per-event serializer]
              <Event_NetOut_SetAutoCycle>::handle
  ↓
0x00d43dc0  shared NetOut byte-writer        [Generic trampoline shared by ~100
                                              NetOut events. Writes `methodID|0x80`
                                              + 1-byte payload to Mercury buffer]
  ↓
Mercury wire → server (cell method 83)
```

### 2. `Event_UI_AutoCycle` is INBOUND (server → client), not outbound

The previous version of this document characterized `Event_UI_AutoCycle` as a step on the OUT path. **That was wrong.** RTTI evidence:

- The only two subscribers to `Event_UI_AutoCycle` are both *listeners*:
  - `USGWTargetIndicator::MemberCallback<Event_UI_AutoCycle>` at `0x01e6b538` — updates the gun-icon button highlight.
  - `SGWScriptedWindow::GameEventHandler<Event_UI_AutoCycle>` at `0x01e1cfc8` — propagates the change to downstream UI.
- The only *emitter* of `Event_UI_AutoCycle` is `FUN_00e05fb0` (`EmitAutoCycleStateChanged`), called *from* `FUN_00e01c90` (the state-field XOR-delta handler) when **the server** toggles `BSF_AutoCycling` (mask `0x002`) on the player's `bStateField`.

So `Event_UI_AutoCycle` is purely a server-driven UI refresh signal. The button press skips it entirely on the way out — the Lua → CEGUI binding constructs `Event_NetOut_SetAutoCycle` directly.

There is also a slash-command path: **`Event_SlashCmd_toggleAutoCycleAbility`** (`ghidra://SGW.exe@0x01842480`) handled by `SGWTextCommandMgr`, which constructs the same outbound event. The player can type `/toggleAutoCycleAbility` as a keyboard alternative to the gun-icon button.

### 2. Method index: 83 (`setAutoCycle`)

The `Event_NetOut_SetAutoCycle` network dispatch resolves to **cell method index 83**, confirmed by:

- `entities/defs/SGWPlayer.def` lines 701–704: `<setAutoCycle><Exposed/><Arg>INT8 enabled</Arg></setAutoCycle>`
- `crates/wire/src/cell/cell_methods/player/constants.rs:21`: `pub const SET_AUTO_CYCLE: u16 = 83;`
- RTTI string `"setAutoCycle"` at `ghidra://SGW.exe@0x019c2e6c`
- `docs/protocol/client-method-dispatch-table.md:289`: entry 83 confirmed

### 3. Wire format

```
Offset  Size  Type   Field     Description
0       1     uint8  header    methodID | 0x80  (= 0x80 | 83 = 0xD3)
1       1     int8   enabled   0 = disable auto-cycle, 1 = enable auto-cycle
```

**Total payload: 2 bytes** (smallest possible SGWPlayer cell method call).

Source: `docs/reverse-engineering/findings/combat-wire-formats.md` §setAutoCycle, cross-confirmed against `SGWPlayer.def`.

### 4. Server-side handler (Cimmeria)

`crates/cell-methods/src/cell/cell_methods/player/world/mod.rs`, `dispatch()` match arm `SET_AUTO_CYCLE`:

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

Two ability flags interact with auto-cycle (from `deprecated/python/Atrea/enums.py` and `crates/entity/src/abilities/manager.rs`):

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

## Implementation in Cimmeria

The loop is fully wired. The code lives in ten locations:

| File | What it owns |
|---|---|
| `crates/entity/src/abilities/manager.rs` | `AbilityManager` fields: `auto_cycle` (flag), `auto_cycle_ability_id` (loop's committed ability), `last_fired_ability_id` (player's most recent fire — persists across loop on/off cycles, used by the immediate-fire path). |
| `crates/entity/src/cell_entity/mod.rs` | `current_target_id` field — the player's live cursor selection, written by `setTargetID` (cell method 0). The auto-cycle tick + death sweep read this as the LIVE target instead of stashing one at arm-time. |
| `crates/cell-combat/src/cell/combat/state.rs` | `BSF_AUTO_CYCLING` constant (mask `0x002`, bit 1). |
| `crates/cell-combat/src/cell/combat/auto_cycle.rs` | Lifecycle primitives: `arm_auto_cycle`, `clear_auto_cycle`, `clear_auto_cycle_for_target`. Manipulate `BSF_AUTO_CYCLING` with **raw `\|=` / `&= !mask` ops** (NOT the ref-counted `set_state_flag` / `unset_state_flag` helpers — see "Bit management" below). All three return `Some(new_state_field)` only when the bit actually transitioned. |
| `crates/cell-methods/src/cell/cell_methods/being.rs` | `SET_TARGET_ID` handler (cell method 0) — persists the target id to `current_target_id` on the player entity so the auto-cycle tick can read it as the live re-fire target. A hostile right-click (`player/interaction/interact.rs`) writes the same field, because the client sends no `setTargetID` for it. |
| `crates/cell-methods/src/cell/cell_methods/player/world/mod.rs` | `SET_AUTO_CYCLE` handler: enable sets the flag AND lights `BSF_AUTO_CYCLING` immediately, stashes the loop ability (`last_fired_ability_id`, or the active weapon's `EVENT_ITEM_RANGED` ability when nothing has fired yet), AND fires immediately if that ability and `current_target_id` are both Some; disable drops the stash, clears the BSF bit, and broadcasts `onStateFieldUpdate`. |
| `crates/cell-combat/src/cell/abilities/use_ability/mod.rs` | Manual-override gate at function entry (different ability ⇒ clear loop), arm/AF_DEACTIVATE branch at commit time, AND stashes `last_fired_ability_id` on every commit regardless of `auto_cycle` state. |
| `crates/cell/src/cell/service/ticks/auto_cycle.rs` | `auto_cycle_tick` — every 100 ms AoI tick, scans armed players and re-invokes `handle_use_ability` against the LIVE `current_target_id`. Cursor switches mid-loop redirect automatically. Out of range or on cooldown skips silently and keeps the loop armed. No target, a missing, dead or surrendered target, a target in another space, an NPC the player may not attack, or a player who is not the duel opponent clears the loop and logs `auto_cycle_tick: clearing loop` with a `reason`. |
| `crates/cell-combat/src/cell/abilities/auto_cycle_state.rs` | `send_auto_cycle_state`, the one exit for every `BSF_AUTO_CYCLING` transition: broadcasts `onStateFieldUpdate` to the player. The bit is not saved; every login and respawn starts with the loop off. See [state-field-bits.md](../architecture/state-field-bits.md#persistence-across-relogs). |
| `crates/cell-combat/src/cell/abilities/death.rs` | `apply_death_transition` calls `clear_auto_cycle_for_target` so every player auto-firing at the dying entity gets their loop cleared (matches against LIVE `current_target_id`, not an arm-time stash). **Plus** clears the dying player's OWN auto-cycle — prevents the loop from auto-resuming on respawn. |

### Bit management — raw ops, NOT the ref-counted helpers

`BSF_AUTO_CYCLING` uses raw `|=` and `&= !mask` ops, deliberately bypassing the ref-counted `set_state_flag` / `unset_state_flag` API on `CellEntity`. Mirrors how `BSF_IN_COMBAT` is handled in `combat::threat` — both are single-source flags where exactly one module (this one) arms and clears the bit.

Using the ref-counted helpers would be a correctness bug: every tick-driven re-fire re-enters `arm_auto_cycle`, `set_state_flag` would bump the per-flag counter from 1 to 2, 3, 4 …, and the single decrement in `clear_auto_cycle` would only bring it back to N-1 — leaving the bit stuck set forever and suppressing every disable/death/manual-override broadcast. Observable symptoms: server logs show `auto-cycle: armed` firing on first commit, then **zero** `death: clearing player auto-cycle loop` lines despite the target dying and the player getting un-aggroed cleanly. Pinned by `clear_after_n_arms_still_transitions_bit_and_broadcasts`.

### Loop semantics (what the tests pin)

- **Enable (button press):** sets `auto_cycle = true` AND lights `BSF_AUTO_CYCLING` immediately so the button highlights on the very first press. **Phase 2: if the player has a target selected (`current_target_id`) AND has a loop ability (`last_fired_ability_id`, or the active weapon's ranged ability when nothing has fired this session), the button press ALSO fires that ability immediately at the target** — the MMO auto-attack feel. Pins: `set_auto_cycle_enable_lights_bsf_and_broadcasts` (base behavior), `set_auto_cycle_enable_fires_immediately_when_target_and_last_ability_set` (immediate-fire path), `set_auto_cycle_enable_does_not_fire_without_last_ability_or_weapon` / `set_auto_cycle_enable_does_not_fire_without_target` (degradation paths).
- **Duplicate enable presses (CEGUI fires the Lua function 3-4× per click, observed within ~150µs):** idempotent — the bit-transition gate suppresses re-broadcast AND the immediate-fire path is gated on the same transition so duplicates don't refire. Pin: `set_auto_cycle_enable_spam_does_not_re_broadcast`.
- **First commit while armed:** `arm_auto_cycle` stashes ability. BSF was already set by enable so no second broadcast fires. Pin: `auto_cycle_first_commit_arms_loop_and_broadcasts_state_field`.
- **Tick-driven re-fire:** every 100 ms, eligible players (armed, cursor target alive, cooldown clear) get a re-invocation of `handle_use_ability` against `current_target_id`. Pins: `auto_cycle_tick_refires_when_cooldown_clear`, `auto_cycle_tick_skips_when_on_cooldown`.
- **Cursor target switch mid-loop:** the tick reads `current_target_id` LIVE — switching cursor from enemy A to enemy B redirects the next re-fire to B with zero loop disruption. Pin: `auto_cycle_tick_refires_at_live_current_target`.
- **Target deselect (`setTargetID(0)`):** clears the loop (no point firing at "no target"). Pin: `auto_cycle_tick_clears_loop_when_target_deselected`.
- **Same-ability manual fire:** does NOT break the loop — right-clicking the same weapon at a new target just commits a manual shot; the tick keeps cycling at `current_target_id`. Pin: `same_ability_manual_fire_does_not_break_loop`.
- **Different-ability manual fire:** breaks the loop on entry. Pin: `manual_fire_of_different_ability_cancels_auto_cycle`.
- **`AF_DEACTIVATE_AUTO_CYCLE` flag (mask `0x400`):** breaks the loop after commit so one-shot specials don't auto-repeat. Pin: `af_deactivate_auto_cycle_clears_loop_on_commit`.
- **Target death:** the death-transition burst sweeps every player whose `current_target_id` matches the dying entity. Pins: `target_sweep_clears_every_player_cycling_at_dying_target`, `target_sweep_follows_live_target_after_switch` (a player who switched cursor after arming is NOT cleared by the original target's death).
- **Dying player's own loop:** if the dying entity is itself an auto-cycling player, their own flag + ability stash + BSF clear in the same death burst — otherwise the loop would auto-resume on respawn. Pin: `dying_player_own_auto_cycle_clears_and_broadcasts`.
- **Target despawn (no death message):** the tick's secondary sweep catches missing target ids. Pin: `auto_cycle_tick_clears_loop_when_target_missing`.
- **Explicit disable (`setAutoCycle(0)`):** clears flag + ability stash + BSF, broadcasts. Pin: `set_auto_cycle_disable_clears_stash_and_bsf`.
- **Duplicate disable presses:** idempotent — same transition-gate pattern as enable. Pin: `set_auto_cycle_disable_spam_does_not_re_broadcast`.
- **Login and respawn start off:** the bit is never saved, so world entry leaves `auto_cycle` false and the button unlit, and a respawn clears it with the combat bits. Pins: `login_leaves_auto_cycle_off_and_the_button_unlit` (`player_init/tests/auto_cycle_starts_off.rs`), `same_world_respawn_replays_hotbar_active_slot_journal_and_state_field`.
- **Press at a target behind a wall:** the press's immediate shot sends the one no-line-of-sight notice (39) and marks it sent, so the tick's NA31 gate stays silent until the line clears. Before 2026-10-03 the first tick repeated it and chat showed the line twice. Pin: `press_at_a_target_behind_a_wall_notifies_once` (`ticks/auto_cycle_press_tests.rs`).

### `current_target_id` vs `last_fired_ability_id` — Phase 2 fields

Phase 2 added two server-side player state fields that didn't exist before. They live independently of the auto-cycle loop state and survive its on/off cycles:

- **`current_target_id: Option<i32>`** on `CellEntity` — written by `setTargetID` (cell method 0). Every cursor selection on the client updates this. `setTargetID(0)` clears to `None`. The auto-cycle tick reads it LIVE on every re-fire; the death sweep filters against it. Mirrors python's `self.entity().targetId` live read in `abilityCooledDown`.

  **Target lifetime (#844).** The server also drops the stored target when the target leaves the holder's witness set (the AoI diff), is destroyed (every teardown runs through `destroy_entity`), or respawns (`npc_respawn`, with `onTargetUpdate(0)` to the client), and drops the killer's stored target in the death burst beside the `onTargetUpdate(0)` reticle drop. The killer's clear runs after the auto-cycle death sweep, which finds the killer's loop through this field. Nobody else's selection changes on a death, so a looter keeps the corpse selected. A target leaving view also stops an armed loop: the tick reads `None` and clears it. GM target resolution refuses a stored target that is not the caller or in its witness set. Code: `crates/cell-world/src/cell/space_manager/target_lifetime.rs`. Clearing on respawn was confirmed on 2026-09-28 (decision recorded on #844): the respawned NPC is a new life at a new place under the same id, which is how the colo `.summon` incident happened. Whether the client itself sends `setTargetID(0)` when its target leaves view is unverified.
- **`last_fired_ability_id: Option<i32>`** on `AbilityManager` — stashed on every `handle_use_ability` commit, regardless of `auto_cycle` state. Persists for the whole session: never cleared on loop stop, death, or respawn. Stale values are harmless — the immediate-fire path routes through the normal `handle_use_ability` validation, which rejects abilities the player no longer has (e.g. after a weapon unequip) and leaves the loop BSF-armed for the next manual fire to refresh. The `SET_AUTO_CYCLE(1)` immediate-fire path uses this as a heuristic for "what ability would the player fire?" since the wire payload doesn't carry an ability id. Distinct from `auto_cycle_ability_id`, which is the LOOP'S committed ability and clears on stop.

### Known gaps / follow-ups

- **The `interact` path arming auto-cycle.** Python `SGWPlayer.py:1175-1178` had `interact` against a hostile NPC set `BSF_AutoCycling` and call `launchAbility(autoCycle=True)` implicitly. Cimmeria's `interact` fires the weapon's `items_event_sets` ranged ability directly but does not arm auto-cycle. Since 2026-10-03 it also writes the player's `current_target_id` beside the `onTargetUpdate`, so a loop armed after a right-click fires at that NPC (see the observed trace below).
- **`DoNotActivate_AutoCycle` ability flag (mask `0x200`).** Only meaningful on the `interact` path (it suppresses the implicit auto-cycle arming when interacting with a hostile). Will land alongside the interact-path work.

### Observed button failure and telemetry recipe (2026-09-29 colo)

The shipped client Lua at `Working/SGWGame/Content/UI/Core/AutoAttack/AutoAttack.lua`
has two actions. Clicking `AutoAttack_AutoAttackButton` calls only
`setAutoAttack(not AutoAttackMod.autoEnabled)`; it does not acquire a target or
choose a weapon ability. `Actions.AttackTarget` (the **T** binding) calls
`targetNextEnemy()` first if `Unit.Target` does not exist, then makes the same
toggle call. `autoEnabled` changes only on `Events.AutoCycle` from the server,
which also changes the button overlay. The Lua therefore makes the icon more
dependent on an existing target than T. These are client-file facts; no new
binary inference is needed beyond the verified method-83 path above.

In a fresh CellBlock-to-Castle playthrough, the client logged ten
`client.net.out` `setAutoCycle` sends. All ten reached `cimmeria-server` as
`setAutoCycle enabled=true`; none was a disable. At 18:49:48 UTC, hostile
right-click launched ability 579 (`Pistol Auto Attack`). The first toggle
arrived at 18:49:49, with no earlier `setTargetID` in that player session.
At 18:50:13 the tick cleared the loop with `target_id=0`. Later, the client
explicitly sent `setTargetID(0)` at 19:09:55, then enabled auto-cycle 0.26 s
later; the tick again cleared with `target_id=0` at 19:10:06. A later Castle
press at 19:24:38 cleared at 19:24:39 with the same reason. The right-click
route continued to launch weapon abilities independently. These observations
explain the reported pattern without claiming every auto-cycle failure has
the same cause.

Another button press at 19:20:07 had a selected but non-hostile NPC. The
auto-cycle driver did run: in the 19:20:40–19:20:46 slice it logged 62
`auto_cycle_tick: re-firing` attempts at that target and 62
`useAbility rejected -- player single-target ability against a non-hostile
target` results. The loop only cleared when its target became 0 at 19:20:46.
This is a second distinct "button lit but no damage" path. The tick's
validity filter catches dead, missing, surrendered, and unauthorized player
targets, but it does not pre-filter friendly NPCs; `handle_use_ability` rejects
them downstream on every tick.

**Fixed 2026-10-03.** The hostile right-click now writes `current_target_id`,
so the first signature (clear with `target_id=0` after a right-click) no
longer occurs unless the player really has no target. The tick stops a loop
aimed at an NPC the player may not attack on its first pass (`reason =
not_hostile`), so the friendly-target loop no longer reaches
`handle_use_ability`. A press before any shot this session stashes the
active weapon's ranged ability, so the loop has something to fire. Pins:
`hostile_right_click_records_current_target`,
`auto_cycle_tick_clears_loop_on_non_hostile_npc_target`,
`set_auto_cycle_enable_without_prior_shot_stashes_weapon_ability`.

To recognize this in another playthrough, correlate the same client session
and player entity across these records, using `ts_ms` as the client event time
because upload can lag:

1. On `cimmeria-client`, filter `client_target = 'client.net.out'` and
   `fields CONTAINS 'setAutoCycle'` or `fields CONTAINS 'setTargetID'`.
   `setAutoCycle` uses `msg_id=61, sub_index=22` in this capture. This hook
   proves the client attempted the RPC, but records neither the enabled byte
   nor the `setTargetID` argument.
2. On `cimmeria-server`, search `setAutoCycle` and inspect its `enabled`
   attribute; search `setTargetID` and inspect `target_id`; search
   `auto_cycle_tick: clearing loop` and inspect `target_id` and `reason`
   (`auto_cycle_tick: target gone or disengaged` before 2026-10-03).
   A `setAutoCycle enabled=true` followed by a clear with `target_id=0` is
   the observed target-loss signature. `useAbility: launched` beside
   `interact: targeting hostile NPC for combat` is a direct right-click
   shot, not proof of an auto-cycle re-fire. `auto_cycle_tick: re-firing`
   must be paired with a committed `useAbility: launched`; repeated
   `useAbility rejected -- ... non-hostile target` instead identifies the
   friendly-target loop seen in this capture.
3. `client.state.field_update` confirms the client entered its state-field
   handler but currently carries no flag value. The client CME event hook
   also does not reliably provide an `Events.AutoCycle` delivery record.
   Use the server's BSF transition and the visible button in a targeted UAT
   to settle whether the overlay changed.

Since 2026-10-03 the enable event `setAutoCycle` carries
`current_target_id`, `last_fired_ability_id`, `weapon_ability_id` and the
stashed `auto_cycle_ability_id` (0 means none), and the next line,
`setAutoCycle: enable decision`, names what the press did: `fire`,
`on_cooldown`, `no_target`, `no_ability` or `already_armed`. The tick's clear
event names its `reason`: `no_target`, `target_gone`, `other_space`, `dead`,
`surrendered`, `not_hostile` or `not_duel_opponent`. The September 29
deployment had none of these fields.

---

## Open Questions

| # | Question | Evidence needed |
|---|----------|-----------------|
| OQ-1 | Does the CEGUI button widget gate clicks on having a hostile targeted, or does it accept clicks unconditionally? | Live-debugger evidence: clicking the button reaches the outbound emit (`0x00e02700`) even with no target. Cimmeria handles the empty-target case at TWO independent checks: (a) the `setAutoCycle(1)` immediate-fire path skips when `current_target_id.is_none()` (zip of two Options returns None); (b) the `auto_cycle_tick` driver clears the loop + un-lights BSF when `current_target_id.is_none()` (treats deselect same as dead/despawned target). A press with no target lights the button, and the next tick (within 100 ms) clears the loop with `reason = no_target`, so the button flashes and goes dark. Behavior is correct either way; the Lua-side gate is a UX nicety, not a correctness requirement. |
| OQ-2 | What was `startAutoCycleAbility` (a base method in `SGWPlayer.def:694`) called by, and how does it differ from `setAutoCycle`? It has no args — is it a server-to-client signal or a server-internal trigger? | No Python implementation found in the deprecation tree. The def entry has no `<Exposed/>` tag, so it was a server-to-server or internal call, not a client RPC. Currently unused in Cimmeria; revisit if a future feature needs it. |
| OQ-3 | Does the client send `setAutoCycle(0)` when the user toggles off, or does it rely solely on `BSF_AutoCycling` clearing? | Confirmed via live debugger: every button click hits the outbound emit regardless of state, and the byte argument toggles between `0` and `1`. The wire is symmetric — the client always sends the new value rather than relying on a server-side toggle. |

---

## Summary

The gun-icon button is the **auto-cycle / auto-fire toggle** (`setAutoCycle`, cell method 83). Pressing it sends a 2-byte packet (`methodID|0x80 + int8 enabled`). After a weapon ability has committed, the server re-fires that ability at the live `current_target_id` on cooldown expiry — no further client input required. The toggle alone carries neither a target nor an ability ID. A successful repeat produces the same `onTimerUpdate` + `onSequence` + `onEffectResults` packets as a manual shot, indistinguishable on the wire.

The loop stops on: target death (death-transition sweep), target despawn (tick's defensive sweep), manual fire of a different ability (entry gate in `handle_use_ability`), an `AF_DEACTIVATE_AUTO_CYCLE`-flagged ability firing (commit-time gate), or explicit `setAutoCycle(0)` (button toggle off).

Cimmeria implements the full server-side loop. The end-to-end handshake the client expects (`BSF_AutoCycling` toggling bit 1 of `bStateField` to drive the button highlight) is in place — verified against the live binary's state-field dispatcher at `ghidra://SGW.exe@0x00e01c90`.
