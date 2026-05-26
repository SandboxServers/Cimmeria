---
name: auto-cycle-findings
description: Auto-cycle/auto-fire system — cell method 83 wire path, server loop model, BSF_AutoCycling, missing Cimmeria implementation
metadata:
  type: project
---

## Auto-Cycle System — Confirmed Findings (2026-05-20)

**Cell method**: 83 (`setAutoCycle`) — `crates/services/src/cell/cell_methods/player/constants.rs:21`  
**Wire format**: 2 bytes — `methodID|0x80` + `int8 enabled`  
**Entity def**: `entities/defs/SGWPlayer.def:701–704`  
**Ghidra**: `ghidra://SGW.exe@0x019c2e6c` (RTTI "setAutoCycle"), `ghidra://SGW.exe@0x019b3e90` (RTTI "Event_NetOut_SetAutoCycle")

### Client UI binding
- `USGWTargetIndicator` subscribes to `Event_UI_AutoCycle` CME event (RTTI at `0x01e6b538`)
- `SGWScriptedWindow::GameEventHandler<Event_UI_AutoCycle>` at `0x00ce9b90`
- Slash-command path: `Event_SlashCmd_toggleAutoCycleAbility` at `0x01842480` → same `Event_NetOut_SetAutoCycle` output
- Button highlight driven by `BSF_AutoCycling` (state field bit 1, mask `0x002`) → `FUN_00e01c90` XOR-delta handler at `0x00e01c90` → `FUN_00e05fb0` (`GameBeing_EmitAutoCycleStateChanged`)

### Server-side loop model (Python original)
The loop is **server-driven, cooldown-gated** — no client participation after the initial toggle.  
`AbilityManager.abilityCooledDown()` in `deprecated/python/cell/AbilityManager.py:965–979`:
1. Cooldown expires → check `self.autoCycle`
2. Find target by `entity().targetId` (server-stored)
3. If target alive: re-call `launchAbility(autoCycleAbility, targetId, autoCycle=True)`
4. If target dead: set `autoCycle=False`, call `stoppedAutoCycling()` → `unsetStateFlag(BSF_AutoCycling)`

### `auto_cycle_ability_id` is set at `launchAbility` time, NOT at `setAutoCycle(1)` time
The button payload does not carry an ability ID. The ability to loop is stored when `launchAbility(autoCycle=True)` commits — i.e., at the first `interact`-driven fire or subsequent re-fires.

### Ability flag gates
- `DoNotActivate_AutoCycle` (512 / `0x200`): `interact` path will not enable auto-cycle for this ability
- `Deactivate_AutoCycle` / `AF_DEACTIVATE_AUTO_CYCLE` (1024 / `0x400`): launching this ability stops auto-cycle immediately

### What Cimmeria is missing
1. **Cooldown-expiry re-fire tick** — `auto_cycle` is stored but never read to re-invoke `handle_use_ability` when cooldown lapses
2. **`auto_cycle_ability_id` never set on first fire** — needs to be stored at `handle_use_ability` success time when `auto_cycle==true`
3. **`BSF_AutoCycling` (bit 1) not set/cleared** — state field is not updated when auto_cycle changes; client button stays un-highlighted
4. **`stoppedAutoCycling` equivalent missing** — no broadcast when loop terminates

### Full doc
`docs/protocol/auto-cycle-button.md`
