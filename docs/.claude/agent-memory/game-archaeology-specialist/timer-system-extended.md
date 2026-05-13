---
name: timer-system-extended
description: Event_NetIn_TimerUpdate has 8 subscribers (not 5); type map including newly-found types 6, 14, 16 and dual-handler type 1; CooldownManager has no type-gating branch.
metadata:
  type: project
---

## Event_NetIn_TimerUpdate — Complete Subscriber Map

**8 subscribers total** (docs previously said 5). Resolved by W-misc-gaps session, 2026-05-13.

### Complete Timer Type Map

| Type (dec) | Handler | Function | Notes |
|------------|---------|----------|-------|
| 0 | CooldownManager | `CooldownManager_HandleOnTimerUpdate` (0x00ea6af0) | warmup start |
| 1 | CooldownManager + GameEntityManager | CM: same; GEM: `FUN_00c68110` (0x00c68110) | cooldown start; GEM handles entity arrival/AoI |
| 2 | CooldownManager | same | warmup end |
| 3 | CooldownManager | same | cooldown end |
| 4 | CooldownManager | same | pass-through (no explicit type branch) |
| 5 | EffectSet | `EffectSet_HandleOnTimerUpdate` (0x00e09160) | active effect duration |
| 6 | DialogController | `FUN_00d26380` (0x00d26380) | NPC interaction countdown timer |
| 7 | CooldownManager | same | pass-through |
| 8 | CooldownManager | same | pass-through |
| 9 | MissionSet | `MissionSet_HandleOnTimerUpdate` (0x00d18a30) | timer start/reset |
| 10 | MissionSet | same | progress timer |
| 11 | MissionSet | same | completion timer |
| 12 | GameBeing | `GameBeing_HandleOnTimerUpdate_Reload` (0x00e02380) | weapon reload |
| 13 | GameBeing | same | deployment reload |
| 14 | GameProxyPlayer/SGWBeing | `SGWBeing_onBigWorldTimeComplete` (0x00dec9e0) | BigWorld time-complete |
| 16 | SGW::Crafting | `FUN_00e47800` (0x00e47800) | crafting job timer |

### Critical implementation note
`CooldownManager_HandleOnTimerUpdate` has **NO type-based early-return**. It processes all types
that pass the `SourceID == this->entityId` gate. The "types 0-3 only" claim in the prior doc was
wrong. Types 4, 7, 8 are ability cooldown sub-states not explicitly branched in the handler.

### New subscriber constructors
- DialogController ctor: `FUN_00d26850` (0x00d26850) — registers timer handler
- SGW::Crafting ctor: `FUN_00e49850` (0x00e49850) — registers timer handler  
- GameEntityManager ctor: `FUN_00c69120` (0x00c69120) — registers timer handler
- GameProxyPlayer uses `SGWBeing_RegisterCallbacks` (0x00df3ab0) / `SGWMob_RegisterCallbacks` (0x00df3cc0)

### GameEntityManager MemberCallback is non-standard
Takes 3 data params (not 2), allocates 0x10 bytes (not 0x0C). Ctor: `FUN_00c6aaa0` (0x00c6aaa0).
