# Auto-Cycle (Auto-Fire) Button — Protocol and Behavior Reference

**Status**: Confirmed  
**Confidence**: HIGH (binary + live-debugger verification + Python canonical source + entity def confirmed)  
**Ghidra anchors**: `ghidra://SGW.exe@0x00aa29c0`, `ghidra://SGW.exe@0x00ad7820`, `ghidra://SGW.exe@0x00e02700`, `ghidra://SGW.exe@0x00e061b0`, `ghidra://SGW.exe@0x00cbbc40`, `ghidra://SGW.exe@0x00e01c90`, `ghidra://SGW.exe@0x00e05fb0`  
**Related files**: `crates/services/src/cell/cell_methods/player/world.rs`, `crates/services/src/cell/combat/auto_cycle.rs`, `crates/services/src/cell/service/ticks.rs`, `crates/entity/src/abilities.rs`, `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/AbilityManager.py`, `entities/defs/SGWPlayer.def`

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

## Implementation in Cimmeria

The loop is fully wired as of #341. The code lives in five locations:

| File | What it owns |
|---|---|
| `crates/entity/src/abilities.rs` | `AbilityManager` fields: `auto_cycle` (flag), `auto_cycle_ability_id` (loop's committed ability), `last_fired_ability_id` (player's most recent fire — persists across loop on/off cycles, used by the immediate-fire path). |
| `crates/entity/src/cell_entity/mod.rs` | `current_target_id` field — the player's live cursor selection, written by `setTargetID` (cell method 0). The auto-cycle tick + death sweep read this as the LIVE target instead of stashing one at arm-time. |
| `crates/services/src/cell/combat/state.rs` | `BSF_AUTO_CYCLING` constant (mask `0x002`, bit 1). |
| `crates/services/src/cell/combat/auto_cycle.rs` | Lifecycle primitives: `arm_auto_cycle`, `clear_auto_cycle`, `clear_auto_cycle_for_target`. Manipulate `BSF_AUTO_CYCLING` with **raw `\|=` / `&= !mask` ops** (NOT the ref-counted `set_state_flag` / `unset_state_flag` helpers — see "Bit management" below). All three return `Some(new_state_field)` only when the bit actually transitioned. |
| `crates/services/src/cell/cell_methods/being.rs` | `SET_TARGET_ID` handler (cell method 0) — persists the target id to `current_target_id` on the player entity so the auto-cycle tick can read it as the live re-fire target. |
| `crates/services/src/cell/cell_methods/player/world.rs` | `SET_AUTO_CYCLE` handler: enable sets the flag AND lights `BSF_AUTO_CYCLING` immediately AND fires immediately if `(last_fired_ability_id, current_target_id)` are both Some; disable drops the stash, clears the BSF bit, and broadcasts `onStateFieldUpdate`. |
| `crates/services/src/cell/abilities/use_ability.rs` | Manual-override gate at function entry (different ability ⇒ clear loop), arm/AF_DEACTIVATE branch at commit time, AND stashes `last_fired_ability_id` on every commit regardless of `auto_cycle` state. |
| `crates/services/src/cell/service/ticks.rs` | `auto_cycle_tick` — every 100 ms AoI tick, scans armed players and re-invokes `handle_use_ability` against the LIVE `current_target_id`. Cursor switches mid-loop redirect automatically; target deselect (`current_target_id = None`) or dead/missing target clears the loop. Cooldown gate is the rate-limiter. |
| `crates/services/src/cell/abilities/death.rs` | `apply_death_transition` calls `clear_auto_cycle_for_target` so every player auto-firing at the dying entity gets their loop cleared (matches against LIVE `current_target_id`, not an arm-time stash). **Plus** clears the dying player's OWN auto-cycle — prevents the loop from auto-resuming on respawn. |

### Bit management — raw ops, NOT the ref-counted helpers

`BSF_AUTO_CYCLING` uses raw `|=` and `&= !mask` ops, deliberately bypassing the ref-counted `set_state_flag` / `unset_state_flag` API on `CellEntity`. Mirrors how `BSF_IN_COMBAT` is handled in `combat::threat` — both are single-source flags where exactly one module (this one) arms and clears the bit.

Using the ref-counted helpers would be a correctness bug: every tick-driven re-fire re-enters `arm_auto_cycle`, `set_state_flag` would bump the per-flag counter from 1 to 2, 3, 4 …, and the single decrement in `clear_auto_cycle` would only bring it back to N-1 — leaving the bit stuck set forever and suppressing every disable/death/manual-override broadcast. This failure mode was observed in #341 playtest (server logs showed `auto-cycle: armed` firing on first commit, then **zero** `death: clearing player auto-cycle loop` lines despite the target dying and the player getting un-aggroed cleanly). Pinned by `clear_after_n_arms_still_transitions_bit_and_broadcasts`.

### Loop semantics (what the tests pin)

- **Enable (button press):** sets `auto_cycle = true` AND lights `BSF_AUTO_CYCLING` immediately so the button highlights on the very first press. **Phase 2: if the player has a target selected (`current_target_id`) AND has fired any ability in this session (`last_fired_ability_id`), the button press ALSO fires that ability immediately at the target** — the MMO auto-attack feel. Pins: `set_auto_cycle_enable_lights_bsf_and_broadcasts` (base behavior), `set_auto_cycle_enable_fires_immediately_when_target_and_last_ability_set` (immediate-fire path), `set_auto_cycle_enable_does_not_fire_without_last_ability` / `set_auto_cycle_enable_does_not_fire_without_target` (degradation paths).
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

### `current_target_id` vs `last_fired_ability_id` — Phase 2 fields

Phase 2 added two server-side player state fields that didn't exist before. They live independently of the auto-cycle loop state and survive its on/off cycles:

- **`current_target_id: Option<i32>`** on `CellEntity` — written by `setTargetID` (cell method 0). Every cursor selection on the client updates this. `setTargetID(0)` clears to `None`. The auto-cycle tick reads it LIVE on every re-fire; the death sweep filters against it. Mirrors python's `self.entity().targetId` live read in `abilityCooledDown`.
- **`last_fired_ability_id: Option<i32>`** on `AbilityManager` — stashed on every `handle_use_ability` commit, regardless of `auto_cycle` state. Persists across loop on/off cycles (only reset on respawn). The `SET_AUTO_CYCLE(1)` immediate-fire path uses this as a heuristic for "what ability would the player fire?" since the wire payload doesn't carry an ability id. Distinct from `auto_cycle_ability_id`, which is the LOOP'S committed ability and clears on stop.

### What was NOT implemented (out of scope for #341)

- **The `interact` path arming auto-cycle.** Python `SGWPlayer.py:1175-1178` had `interact` against a hostile NPC set `BSF_AutoCycling` and call `launchAbility(autoCycle=True)` implicitly. Cimmeria's `interact` does not do this yet — the explicit `setAutoCycle(1)` button is currently the only entry point. With Phase 2's immediate-fire, the button + a target selection now produces the same end-result UX without the interact-path arming. Could still be wired for parity.
- **`DoNotActivate_AutoCycle` ability flag (mask `0x200`).** Only meaningful on the `interact` path (it suppresses the implicit auto-cycle arming when interacting with a hostile). Will land alongside the interact-path work.
- **Out-of-range tick spam.** When the player walks out of range mid-loop, every tick re-fire fails range validation inside `handle_use_ability` and emits `onErrorCode(OutsideWeaponRange)` to the player. At a 0.5s cooldown that's an error packet every ~600ms while out of range. The tick should ideally range-pre-check and silently skip rather than retry+fail+error. Minor — affects UX, not correctness.

---

## Open Questions

| # | Question | Evidence needed |
|---|----------|-----------------|
| OQ-1 | Does the CEGUI button widget gate clicks on having a hostile targeted, or does it accept clicks unconditionally? | Live-debugger evidence: clicking the button reaches the outbound emit (`0x00e02700`) even with no target — Cimmeria handles the empty-target case server-side (the loop arms but the driver tick skips re-firing into an invalid target). Behavior is correct either way; the Lua-side gate is a UX nicety, not a correctness requirement. |
| OQ-2 | What was `startAutoCycleAbility` (a base method in `SGWPlayer.def:694`) called by, and how does it differ from `setAutoCycle`? It has no args — is it a server-to-client signal or a server-internal trigger? | No Python implementation found in the deprecation tree. The def entry has no `<Exposed/>` tag, so it was a server-to-server or internal call, not a client RPC. Currently unused in Cimmeria; revisit if a future feature needs it. |
| OQ-3 | Does the client send `setAutoCycle(0)` when the user toggles off, or does it rely solely on `BSF_AutoCycling` clearing? | Confirmed via live debugger: every button click hits the outbound emit regardless of state, and the byte argument toggles between `0` and `1`. The wire is symmetric — the client always sends the new value rather than relying on a server-side toggle. |

---

## Summary

The gun-icon button is the **auto-cycle / auto-fire toggle** (`setAutoCycle`, cell method 83). Pressing it sends a 2-byte packet (`methodID|0x80 + int8 enabled`). When enabled, the server re-fires the player's current weapon ability at the stashed target every time the cooldown expires — no further client input required. The client receives the same `onTimerUpdate` + `onSequence` + `onEffectResults` packets a manual shot would produce, indistinguishable on the wire.

The loop stops on: target death (death-transition sweep), target despawn (tick's defensive sweep), manual fire of a different ability (entry gate in `handle_use_ability`), an `AF_DEACTIVATE_AUTO_CYCLE`-flagged ability firing (commit-time gate), or explicit `setAutoCycle(0)` (button toggle off).

As of #341 the full server-side loop is implemented and tested. The end-to-end handshake the client expects (`BSF_AutoCycling` toggling bit 1 of `bStateField` to drive the button highlight) is in place — verified against the live binary's state-field dispatcher at `ghidra://SGW.exe@0x00e01c90`.
