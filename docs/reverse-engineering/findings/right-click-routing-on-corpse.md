# Right-click Routing: Why Corpses Fail to Open Loot

> **Status**: Resolved — see Resolution section.
> **Source**: Ghidra static analysis + live debugger trace via x32dbg.
> **Audience**: Future Cimmeria reverse-engineers debugging client-server interaction issues.

## Resolution (TL;DR)

The bug was **server-side, not client-side**. None of the static-analysis hypotheses below (`+0x30/+0x31` gate bytes, `actor+0x1b4` entity ID, `onDuelEntitiesRemove` interactable-set hack) actually mattered.

**What was happening**: the Cimmeria server's `interact` cell-method handler at [crates/services/src/cell/cell_methods/player/interaction.rs](../../../crates/services/src/cell/cell_methods/player/interaction/mod.rs) checked whether the target was a hostile NPC (`!is_player && faction == 10`) and **redirected to `useAbility` instead of running `handle_interact`**. The check didn't consider whether the NPC was dead. So:

- Alive hostile guard → reroute to `useAbility` ✓ correct
- **Dead hostile guard → reroute to `useAbility`** ✗ — the lootable corpse's `interact` request was silently turned into combat, and the loot pipeline was never reached.

**Fix**: gate the reroute on `!is_dead_state(target.state_field)`. Dead corpses fall through to `handle_interact`, which sees `interaction_type = Some(Loot)` and dispatches `onLootDisplay`.

```rust
// crates/services/src/cell/cell_methods/player/interaction.rs:37-49
let is_hostile = space_mgr.get_entity(target_entity_u32)
    .map_or(false, |t| {
        !t.is_player
            && t.faction == 10
            && !crate::cell::combat::is_dead_state(t.state_field)
    });
```

**How we found it**: x32dbg attached to live SGW.exe + a log breakpoint at `FUN_00e84b20` (the interact firer wrapper). The breakpoint confirmed the client *was* sending `interact` for the corpse with the right entity ID, comparison conditions passed, and `Event_NetOut_Interact` was being constructed. That eliminated client-side suspects and pointed the search at the server. A `grep` for the dispatch site revealed the hostile-NPC reroute.

The static analysis below remains useful tribal knowledge for understanding the SGW client's pick + interact pipeline, but **its conclusion was wrong**. We documented it here as a record of the investigation, not as ground truth.

---

> **Source**: Ghidra static analysis of SGW.exe
> **Date**: 2026-04-30
> **Confidence**: HIGH for the gate location and callers; **WRONG** for the *cause* of the gate failing — the gate was never actually the cause. See Resolution above.

## Decision graph (player presses RMB on an entity)

```
Event_Action_MouseLook (RMB release)
  │
  ▼
ASGWController_Player::onMouseLook  (FUN_00e85860)
  │  guards: bit0 of this+0x498 == 0 (released, not press)
  │          |delta_x| < 5px && |delta_y| < 5px
  ▼
FUN_00e84b20  (interact firer wrapper)
  │  guards: global UI flags ok
  │          target != self (target.entityId != pawn->entityId)
  │  target = FUN_00e84860(player_controller)
  │  if target != null:
  ▼
FUN_00def4b0  → constructs Event_NetOut_Interact → wire send "interact"
```

If `FUN_00e84860` returns null, interact is silently dropped. The right-click then has nothing else to do — it's a release event. The visible "right-click fires useAbility" we see in our combat.log is the **auto-attack loop** (`FUN_00e3cd90`, called via vtable on a tick), not the click itself.

## The gate — `FUN_00e84860`

Resolves the picked target:

1. Raycast / cursor pick → AActor at the cursor.
2. `FUN_00e85e80(actor)` — filters by team/group membership; rejects actors not in our allowed-target set.
3. Reads `actor + 0x1b4` → **BigWorld entity ID**.
4. `FUN_00dd0de0(EntityManager, entityId)` — std::map find returning the `GameEntity*`.
5. `__RTDynamicCast(...)` to `GameEntity`.
6. **`FUN_00e68570(GameEntity*)` is the gate.** If it returns 0, the resolver returns 0 → interact dropped.

## The gate predicate — `FUN_00e68570`

```c
undefined4 __fastcall FUN_00e68570(int param_1) {
  if (*(char *)(param_1 + 0x31) != '\0' && *(char *)(param_1 + 0x30) != '\0') return 1;
  return 0;
}
```

Two boolean bytes on `GameEntity`. **Both** must be non-zero to pass.

## What sets the bytes

Subscriptions are registered in `GameEntity::vfunc_5` (subscribe) and unregistered in `GameEntity::vfunc_6` (unsubscribe).

| Byte | Set by | Triggered by |
|---|---|---|
| `+0x30 = 1` | `FUN_00e68a30` | `Event_NetIn_onEntityMove` handler `FUN_00e6f080` (reads `locationX/Y/Z`, `velocityX/Y/Z`, `yaw/pitch/roll` event properties) |
| `+0x31 = 1` | `LAB_00e6db70` (12-byte stub: `MOV BYTE PTR [ECX+0x31], 1; CALL FUN_00e688c0; RET 8`) | `Event_NetIn_onVisible` handler |

**Confirmed via the `MemberCallback` template instantiation classes** (named after RTTI annotator ran):
- `MemberCallback<…, GameEntity, GameEntity::*(Event_NetIn_onVisible const*, void*), Event_NetIn_onVisible>` constructed at `FUN_00e70370`.
- The `Event_NetIn_onEntityMove` handler `FUN_00e6f080` reads property name strings `locationX/Y/Z`, etc. — proves the event identity.

No writers found that set either byte to 0 except `std::map`/`std::set` tree-balancing code on unrelated nodes (false positives from the byte-pattern search). Once set, both bytes stay set for the life of the `GameEntity` object on the client.

## Implications for the Rust server

The gate requires the client to have received **both**:
1. A method-call `onEntityMove` (method index 2) — currently we **don't send** this. We send the optimized [`build_avatar_update`](../../../crates/services/src/mercury/aoi/update.rs#L22) packet (BASEMSG `0x10` `UPDATE_AVATAR_NO_ALIAS_FULL_POS_YPR`) which uses the position-stream protocol and may go through a different client path.
2. A method-call `onVisible(1)` — we do send this in the AoI cascade at [mercury/aoi/create.rs:234](../../../crates/services/src/mercury/aoi/create.rs#L234), so `+0x31` should be set on AoI entry.

Why Frost works but the dead guard doesn't (open question):

- If the optimized `0x10` packet does NOT fire `Event_NetIn_onEntityMove`, then `+0x30` is **never** set on either Frost or the guard — both should fail the gate. They don't. So either:
  - (a) The AoI entry's CREATE_ENTITY property stream populates `+0x30 = 1` indirectly (the position field on creation triggers the same handler).
  - (b) Something else sets `+0x30`.
- If both work normally, the `+0x30/+0x31` bytes should be set on both Frost and the dead guard.

**Most likely actual cause** (not yet confirmed via Ghidra): the picking step `actor + 0x1b4` returns 0 for the dying guard's actor. The `ABigWorldEntity` actor stripped its entity-ID reference on death, so `FUN_00dd0de0(0)` fails to find a `GameEntity` and the resolver returns null **before** ever calling the gate.

This would also explain why the cursor still shows loot (the cursor uses a different lookup path — likely reads `mInteractionType` directly from the GameEntity via the on-hover event, which doesn't depend on the actor's `+0x1b4`).

## Recommended next moves

1. **Live debug** — set a breakpoint at `0x00e68570` (the gate predicate). Right-click corpse: does the breakpoint hit? If not, the resolver returned null **before** the gate, meaning `+0x1b4` on the corpse's actor is 0 — actor-side state, not entity-side.
2. **If gate hits**: dump `param_1+0x30` and `param_1+0x31` to confirm which is zero. Then trace what cleared it.
3. **If gate doesn't hit**: breakpoint at `0x00e84860 + offset for the +0x1b4 read` — see what value the actor's entity-ID field holds.

## Key addresses

| Address | Symbol | Role |
|---|---|---|
| `0x00e85860` | `ASGWController_Player::onMouseLook` | Click router |
| `0x00e84b20` | `ASGWController_Player::fireInteract` (wrapper) | Calls resolver, fires Event_NetOut_Interact |
| `0x00e84860` | Target resolver | Raycast → entity ID lookup → gate |
| `0x00e68570` | **Gate predicate** | `(this+0x30 != 0) && (this+0x31 != 0)` |
| `0x00e68a30` | `+0x30` setter | Called from `Event_NetIn_onEntityMove` handler |
| `0x00e6db70` | `+0x31` setter (12-byte stub) | Called from `Event_NetIn_onVisible` handler |
| `0x00e6f080` | `Event_NetIn_onEntityMove` handler | Reads locationX/Y/Z, velocity, yaw/pitch/roll |
| `0x00e688c0` | "ready" check | Reads both gate bytes; uses for pawn show/hide |
| `0x00cb7d30` | `Event_NetOut_UseAbility` ctor | For comparison |
| `0x00d97990` | `Event_NetOut_Interact` ctor | For comparison |
| `0x00c811f0` | useAbility firer #1 (lua-call path) | Caller of UseAbility ctor |
| `0x00d2ae40` | useAbility firer #2 | Caller of UseAbility ctor |
| `0x00def4b0` | interact firer | Caller of Interact ctor |
| `0x00e3cd90` | auto-attack useAbility caller (vtable) | Likely source of the `useAbility` we see in logs after death |
