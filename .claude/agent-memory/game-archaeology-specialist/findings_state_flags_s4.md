---
name: findings-state-flags-s4
description: Session 4 W-state findings: BSF_* flag master table, FUN_00e01c90 XOR-delta dispatch, BSF_Holster correction, issues #219/#232/#249 root causes
metadata:
  type: project
---

Key addresses recovered and V5-documented in session 4 W-state (2026-05-13):

| Address | Name | Role |
|---------|------|------|
| 0x00e01c90 | GameBeing_OnStateFieldUpdate | CME subscriber for Event_NetIn_onStateFieldUpdate; XOR-delta bit dispatch on bits 0-7 |
| 0x00e7b4c0 | GameBeing_UpdateCombatStanceWeaponSet | BSF_InCombat (bit 3) handler; selects weapon anim set via 2-char archetype key |
| 0x00e79fc0 | GameBeing_BuildCombatAnimKey | Builds combat anim key from BSF_InCombat + melee-range predicates |
| 0x00dff430 | GameBeing_IsInCombat | Predicate: tests pGameBeing+0x158 & 0x08 |
| 0x00dff880 | GameBeing_IsTargetInMeleeRange | 3D distance check vs melee range (pGameBeing+0x150) |
| 0x00dfff70 | GameBeing_UpdateMovementSpeed | Movement speed from BSF_Walking(0x80)/Crouching(0x04)/MovementLock(0x40) |
| 0x00e05fb0 | GameBeing_EmitAutoCycleStateChanged | Pattern B CME emitter for BSF_AutoCycling (bit 1) |
| 0x00e060b0 | GameBeing_EmitStealthStateChanged | Pattern B CME emitter for BSF_InStealth (bit 5) |
| 0x00e05db0 | GameBeing_EmitStateFieldChanged | Unconditional emitter; fires Event_Entity_StateFieldChanged with {entity_id, old, new, delta} |
| 0x00e6e330 | GameBeing_OnDeadStateChanged | BSF_Dead (bit 0) handler: interaction type toggle, pawn actor update |
| 0x00c71790 | GameBeing_GetMovementSpeedTable (pending W0 rename) | Speed constant table singleton |

CRITICAL CORRECTION: BSF_Holster (bit 8, mask 0x100) is NOT dispatched in GameBeing_OnStateFieldUpdate.
All TEST instructions use BL (low byte). Bit 8 requires TEST EBX,0x100 which is absent.
No dedicated holster ClientMethod exists in any .def file.
BSF_Holster may be a state-persistence flag only; animation mechanism unidentified.

Bit dispatch confirmed from assembly at 0x00e01d62:
- bit 0 (0x01) BSF_Dead -> GameBeing_OnDeadStateChanged (00e6e330)
- bit 1 (0x02) BSF_AutoCycling -> GameBeing_EmitAutoCycleStateChanged (00e05fb0)
- bits 2+6+7 (0xC4) BSF_Crouching|MovementLock|Walking -> GameBeing_UpdateMovementSpeed (00dfff70)
- bit 3 (0x08) BSF_InCombat -> GameBeing_UpdateCombatStanceWeaponSet (00e7b4c0)
- bit 4 (0x10) BSF_PlayingMinigame -> FUN_00e31aa0
- bit 5 (0x20) BSF_InStealth -> GameBeing_EmitStealthStateChanged (00e060b0)
- bit 8 (0x100) BSF_Holster -> NOT HANDLED
- unconditional -> GameBeing_EmitStateFieldChanged (00e05db0)

Issue root causes confirmed:
- #219: threat.rs sends EntityMethodCall; messaging.rs::send_entity_method routes players to EntityMethodCall not witness fanout
- #232 Bug A: aoi/create.rs hardcodes 0u32; Bug B: respawn.rs send_entity_method = EntityMethodCall for players
- #249: combatant.rs uses EntityMethodCall with hardcoded method_index 19; AND client has no side effect for bit 8

Findings doc: docs/reverse-engineering/findings/state-flag-broadcast.md
Checkpoint: docs/reverse-engineering/v5-campaign/worker-state.checkpoint.json

**Why**: Assigned W-state in session 4 orchestrated campaign.
**How to apply**: Use these addresses for weapon animation, locomotion, entity state broadcast investigations. BSF_Holster correction is critical.
