---
name: auto-cycle-findings
description: Auto-cycle/auto-fire system — cell method 83 wire path, server loop model, BSF_AutoCycling, missing Cimmeria implementation
metadata:
  type: project
---

Auto-cycle (cell method 83 `setAutoCycle`, wire: `methodID|0x80` + `int8 enabled`; `docs/protocol/auto-cycle-button.md` is the full doc). Method for the next question about it:
- Client: `USGWTargetIndicator` subscribes to `Event_UI_AutoCycle`; the slash path `toggleAutoCycleAbility` reaches the same `Event_NetOut_SetAutoCycle`. The button highlight is driven by `BSF_AutoCycling` (state bit 1, mask 0x002) through the XOR-delta handler `0x00e01c90`.
- Server model (from the original Python): the loop is server-driven and cooldown-gated. On cooldown expiry, re-launch the stored auto-cycle ability at the stored target; if the target is dead, clear the flag and unset `BSF_AutoCycling`.
- Trap: the button payload carries no ability id. The ability to loop is stored when `launchAbility(autoCycle=True)` commits, not at `setAutoCycle(1)`.
- Ability flags: `DoNotActivate_AutoCycle` (0x200) stops `interact` enabling it; `Deactivate_AutoCycle` (0x400) stops it on launch.
- The Rust side lives in `crates/cell/src/cell/service/ticks/auto_cycle.rs`; read it before trusting any older "missing" claim.
