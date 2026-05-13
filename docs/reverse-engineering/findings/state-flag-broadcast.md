# State-Flag (BSF_*) Broadcast — Reverse Engineering Findings

**Date**: 2026-05-13
**Session**: Session 4 — W-state
**Issues covered**: #219 (BSF_InCombat lifecycle), #232 (death/respawn witness broadcast), #249 (BSF_Holster never seen by witnesses)
**Confidence**: HIGH (canonical enum from entity defs; client-side XOR-delta dispatch decompiled from binary)

---

## 1. BSF_* Flag Master Table

The `EStateField` enumeration is defined in `entities/defs/enumerations.xml` (lines 293–306). Values are **bit indices** — the wire mask is `1 << value`. The INT32 `bStateField` property on `SGWBeing` is the wire container (`CELL_PUBLIC`, propagated to all nearby cells).

| Bit index | Mask (hex) | Constant name       | Client-side side effects on transition (from `FUN_00e01c90` XOR-delta dispatch) |
|-----------|------------|---------------------|----------------------------------------------------------------------------------|
| 0         | `0x001`    | `BSF_Dead`          | Toggles entity interaction type (alive ↔ dead-interaction/looting) |
| 1         | `0x002`    | `BSF_AutoCycling`   | Toggles auto-cycle combat behavior (secondary dispatch at `0x00e01c90`) |
| 2         | `0x004`    | `BSF_Crouching`     | Movement speed and posture updates |
| 3         | `0x008`    | `BSF_InCombat`      | Routes to `FUN_00e7b4c0` — weapon animation / combat-ready stance |
| 4         | `0x010`    | `BSF_PlayingMinigame` | Minigame UI lock-out flag |
| 5         | `0x020`    | `BSF_InStealth`     | Stealth visual effect on/off |
| 6         | `0x040`    | `BSF_MovementLock`  | Disables client movement input |
| 7         | `0x080`    | `BSF_Walking`       | Walk/run toggle |
| 8         | `0x100`    | `BSF_Holster`       | **Not handled in `FUN_00e01c90`** — bit 8 is outside the low-byte TEST range; no client-side side effect dispatched here (see section 6 for full analysis) |

**Evidence**: `entities/defs/enumerations.xml` lines 293–306 (canonical definition). Bit dispatch map confirmed from assembly of `FUN_00e01c90` (address `0x00e01c90`), specifically the `TEST BL, mask` instructions at `0x00e01d72` through `0x00e01e58`.

**Key observation**: `FUN_00e01c90` tests only the low byte of the XOR delta (BL). BSF_Holster (bit 8, mask `0x100`) lives in the second byte and is NOT checked. The handler does not call `FUN_00e7b4c0` for holster transitions. See section 6 for the full implication on issue #249.

---

## 2. Client-Side Handler: `FUN_00e01c90` (GameBeing `onStateFieldUpdate`)

**Address**: `0x00e01c90`
**Role**: CME EventSignal subscriber on `Event_NetIn_onStateFieldUpdate`; the canonical state-field ingestion point for all `SGWBeing` entities on the client.

### XOR-delta dispatch pattern

```c
// Reconstructed from assembly of FUN_00e01c90 (0x00e01d62 onward)
// EBX = delta (old_state XOR new_state); EBP = new_state; ESI = this
//
// NOTE: All bit tests use TEST BL (low byte only). BSF_Holster (bit 8, mask
// 0x100) lives in the SECOND byte and is NOT TESTED here. The client does not
// dispatch any side effect for BSF_Holster in this handler.

uint old_state = *(uint*)(this + 0x158);   // ESI+0x158 — cached prior value
uint new_state = event_arg_bStateField;     // from event "bStateField" field
uint delta     = old_state ^ new_state;     // XOR: only changed bits
*(uint*)(this + 0x158) = new_state;         // store updated value

// 0x00e01d72: TEST BL, 0x2  (bit 1 — BSF_AutoCycling)
if (delta & 0x002) { FUN_00e05fb0(...); }   // auto-cycle CME event emitter

// 0x00e01de8: TEST BL, 0x8  (bit 3 — BSF_InCombat)
if ((delta & 0x008) && *(this+0x8) != NULL) { FUN_00e7b4c0(*(this+0x8), this); }

// 0x00e01dfa: TEST BL, 0x1  (bit 0 — BSF_Dead)
if (delta & 0x001) {
    FUN_00e6e330(this, NULL);               // dead-entity setup
    if (*(this+0x8) && *(*(this+0x8)+0x398)) {
        FUN_00e791d0(...);                  // pawn/ragdoll or interaction update
    }
}

// 0x00e01e2d: TEST BL, 0xC4  (bits 2+6+7 — BSF_Crouching|BSF_MovementLock|BSF_Walking)
if (delta & 0x0c4) { FUN_00dfff70(this); } // movement/posture update

// 0x00e01e39: TEST BL, 0x10  (bit 4 — BSF_PlayingMinigame)
if (delta & 0x010) { FUN_00e31aa0(...); }  // minigame UI lock

// 0x00e01e58: TEST BL, 0x20  (bit 5 — BSF_InStealth)
if (delta & 0x020) { FUN_00e060b0(...); }  // stealth CME event emitter

// 0x00e01ec6: UNCONDITIONAL — always fires
// Allocates 16-byte event data {entity_id, old_state, new_state, delta}
FUN_00e05db0(...);  // fires Event_Entity_StateFieldChanged
```

**BSF_Holster (bit 8, mask 0x100) is NOT handled in this function.** All `TEST BL, mask` instructions operate on the low byte of the delta register. Bit 8 requires `TEST EBX, 0x100` (a 32-bit test), which does not appear in this function. The client receives `onStateFieldUpdate` for BSF_Holster changes but executes no side effects in `FUN_00e01c90` for that bit.

### Key invariant from the XOR pattern

The handler is delta-safe: calling `onStateFieldUpdate` with an unchanged value is a no-op for all per-bit effects. The client does not re-animate or re-update if the same `bStateField` value arrives twice. This means it is safe to replay the current `state_field` during AoI entry (it will only trigger side effects for bits that differ from 0, which is the initial client-side default).

### Secondary event dispatch

After all bit checks, `FUN_00e01c90` fires `Event_Entity_StateFieldChanged` unconditionally. Confirmed subscribers:
- `GameProxyPlayer` — updates HUD/UI state
- `USGWTargetIndicator` — updates the targeting reticle (dead/alive indicator)

---

## 3. Wire Format

From `entities/defs/interfaces/SGWBeing.def` (lines 230–237) and confirmed by the universal RPC dispatcher at `0x00c6fc40`:

```
onStateFieldUpdate(INT32 bStateField)
  Payload: 4 bytes, little-endian INT32
  Direction: server → client (ClientMethod)
  Delivery: cell AoI (CELL_PUBLIC property context)
```

The `bStateField` property is marked `CELL_PUBLIC` in the `.def`, meaning the server infrastructure propagates changes to all nearby cells. However — this propagation is property-sync, not a ClientMethod call. The `onStateFieldUpdate` ClientMethod is explicitly sent by server code to specific targets. The two mechanisms are independent: property-sync keeps the value consistent, but only the ClientMethod call triggers the XOR-delta side effects (animations, interaction-type change, etc.).

---

## 4. Issue #219 — BSF_InCombat Lifecycle Broken

### Binary evidence

- Bit 3 (`BSF_InCombat`) in the XOR-delta dispatch calls `FUN_00e7b4c0` — weapon combat stance animation
- Both set (bit becomes 1) and clear (bit becomes 0) directions trigger the weapon animation update
- The animation update requires the full new `bStateField` value, not just the delta mask

### Server-side implementation (Cimmeria)

`crates/services/src/cell/combat/threat.rs`:

```rust
// Enter: NPC threatens player
pub fn enter_player_combat(player: &mut CellEntity, npc_id: u32) -> Option<u32> {
    player.threatened_mobs.insert(npc_id);
    if player.threatened_mobs.len() == 1 {
        // First threat — set BSF_InCombat
        player.state_field |= BSF_IN_COMBAT;
        Some(player.state_field)
    } else {
        None  // Already in combat — no broadcast needed
    }
}

// Clear: NPC dies or leaves
pub fn exit_player_combat(player: &mut CellEntity, npc_id: u32) -> Option<u32> {
    player.threatened_mobs.remove(&npc_id);
    if player.threatened_mobs.is_empty() {
        player.state_field &= !BSF_IN_COMBAT;
        Some(player.state_field)
    } else {
        None
    }
}
```

The callers in `threat.rs` send `onStateFieldUpdate` via `EntityMethodCall` — which reaches only the owning player's client.

### Root cause of #219

**Witnesses never receive `onStateFieldUpdate` on BSF_InCombat transitions.** Other players in the same AoI do not see the combat-stance change on the affected player. The cursor indicator change (`USGWTargetIndicator`) on the *witness* client never fires because `FUN_00e01c90` is never triggered for them.

Three caller sites for BSF_InCombat changes:
1. `enter_player_combat()` — sends `EntityMethodCall` (owning client only)
2. `exit_player_combat()` via `clear_dead_npc_from_all_player_threat()` — sends `EntityMethodCall` per affected player (each player's own client only)
3. Player logout/disconnect — state cleared, but client is gone, so broadcast is moot

### Fix for #219

In `threat.rs`, replace `EntityMethodCall` with `send_entity_method` (or equivalent witness fanout). `send_entity_method` in `crates/services/src/cell/abilities/mod.rs` already fans out to all AoI witnesses via `WitnessEntityMethod`. No new mechanism is needed — only the call site changes.

---

## 5. Issue #232 — Death/Respawn State Not Broadcast to Witnesses

Two independent bugs with the same root cause pattern: `onStateFieldUpdate` sent only to the owning client, not to AoI witnesses.

### Bug A: AoI entry hardcodes `state_field = 0`

**Location**: `crates/services/src/mercury/aoi/create.rs` (lines 175–181)

```rust
// 12. onStateFieldUpdate(0) — alive state   ← HARDCODED ZERO — BUG #232
append_entity_method(
    &mut body,
    method_idx::ON_STATE_FIELD_UPDATE,
    entity_id,
    &0u32.to_le_bytes(),  // always 0, ignores entity's actual state_field
);
```

Function `build_create_entity_cascade` takes no `state_field` parameter. A witness entering AoI while the entity is already dead receives `onStateFieldUpdate(0)`, which means BSF_Dead is NOT set on the witness's client. The corpse appears alive and interactable (alive cursor, no loot prompt).

**Fix for Bug A**: Add `state_field: u32` parameter to `build_create_entity_cascade`. Pass in the entity's actual `state_field` value at call time. The XOR-delta handler on the client will correctly process the live state on AoI entry (since initial client-side cache is 0, any non-zero state_field correctly triggers transitions).

### Bug B: Respawn state-clear not broadcast to witnesses

**Location**: `crates/services/src/cell/cell_methods/player/combat/respawn.rs`

```rust
// Clear state flags (includes BSF_Dead)
entity.clear_all_state_flags();

// Send onStateFieldUpdate(0) ONLY to owning client:
crate::cell::abilities::send_entity_method(
    entity_id,
    crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
    0u32.to_le_bytes().to_vec(),
    tx,
    space_mgr,
).await;
```

Wait — `send_entity_method` in `crates/services/src/cell/abilities/mod.rs` fans to witnesses. Re-reading: this call IS through `send_entity_method`, not `EntityMethodCall` directly. The code above needs verification against the actual `send_entity_method` implementation.

**Confirmed**: `send_entity_method` in `crates/services/src/cell/abilities/messaging.rs` routes by entity type:
- Player entity → `EntityMethodCall` (owning client only)
- NPC/ghost entity → `WitnessEntityMethod` fan to all witnesses

Since the respawning entity is a player, Bug B is **confirmed**: the respawn `onStateFieldUpdate(0)` reaches only the player's own client. Witnesses still see the dead state (BSF_Dead bit set from the original death burst) and the player appears as a corpse until they leave and re-enter AoI.

**Confirmed bug for Bug A**: The hardcoded-zero AoI entry case is independently confirmed and unambiguously broken regardless of `send_entity_method` semantics.

### Death burst ordering (confirmed correct, for reference)

`crates/services/src/cell/abilities/death.rs` sends in this order:
1. `onTargetUpdate(0)` to attacker (if player)
2. BSF_InCombat clear for affected players (via threat tracking)
3. Loot generation + `INTERACTION_TYPE` update for NPC
4. `onStateFieldUpdate(target_state)` with BSF_Dead set — the load-bearing death signal

This ordering is correct: loot must be generated before the interaction type marks the entity as a corpse, or a race condition allows loot display with no contents.

---

## 6. Issue #249 — BSF_Holster: Client Handler Not in `FUN_00e01c90`

### Binary evidence (corrected from session-4 pre-compaction notes)

**`FUN_00e01c90` does NOT handle BSF_Holster (bit 8).** Assembly analysis of the XOR-delta dispatch confirms all bit tests use `TEST BL` (low-byte register), spanning bits 0–7 only. The full test sequence is:

```asm
00e01d72: TEST BL, 0x02   ; bit 1 — BSF_AutoCycling
00e01de8: TEST BL, 0x08   ; bit 3 — BSF_InCombat → FUN_00e7b4c0
00e01dfa: TEST BL, 0x01   ; bit 0 — BSF_Dead → FUN_00e6e330
00e01e2d: TEST BL, 0xC4   ; bits 2+6+7 — BSF_Crouching|BSF_MovementLock|BSF_Walking → FUN_00dfff70
00e01e39: TEST BL, 0x10   ; bit 4 — BSF_PlayingMinigame → FUN_00e31aa0
00e01e58: TEST BL, 0x20   ; bit 5 — BSF_InStealth → FUN_00e060b0
```

There is no `TEST EBX, 0x100` (which would be needed for bit 8). BSF_Holster changes arriving via `onStateFieldUpdate` are absorbed into `this+0x158` (the cached state), but no side effect fires within this handler.

**Implication for #249**: The holster animation is driven by a different mechanism — likely a separate ClientMethod or a direct animation call not routed through `bStateField`. The BSF_Holster bit in `bStateField` may function as a state-persistence flag (so late-joining witnesses can query it) without being the actual animation trigger. The animation trigger may be a separate, dedicated ClientMethod.

### Server-side implementation (Cimmeria)

`crates/services/src/cell/cell_methods/combatant.rs`:

```rust
REQUEST_HOLSTER_WEAPON => {
    // Sets/clears BSF_HOLSTER in entity.state_field
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index: 19,  // ON_STATE_FIELD_UPDATE — hardcoded index
            args: new_state.to_le_bytes().to_vec(),
        })
        .await;
    // TODO: also send to witnesses via AoI broadcast
}
```

Two bugs in one call site:
1. `EntityMethodCall` reaches only the owning player — witnesses never see holster/draw
2. `method_index: 19` is a hardcoded integer; should use `method_idx::ON_STATE_FIELD_UPDATE`

The same `EntityMethodCall`-only pattern applies to `SET_CROUCHED` in the same file.

### Root cause of #249

Witnesses never receive `onStateFieldUpdate` when a player holsters or draws their weapon. On the witness's client, `FUN_00e7b4c0` never fires for this entity, so the weapon remains in whatever animation state it was last known to be. Players appear to always have their weapon drawn (the default AoI-entry state with `onStateFieldUpdate(0)`, which has BSF_Holster = 0 = unholstered).

### Fix for #249

Replace `EntityMethodCall` with `send_entity_method` (witness fan variant) in the `REQUEST_HOLSTER_WEAPON` arm. Also fix `SET_CROUCHED` in the same change. Replace hardcoded `method_index: 19` with `method_idx::ON_STATE_FIELD_UPDATE`.

---

## 7. Recommended Rust Server Changes

### Issue #219 — BSF_InCombat witness broadcast

**File**: `crates/services/src/cell/combat/threat.rs`

`send_entity_method` routes to `EntityMethodCall` for players, so it does NOT reach witnesses. The fix requires iterating `space_mgr.get_witnesses_of(player_entity_id)` and sending `WitnessEntityMethod` for each witness. Extract this into a helper (e.g., `send_player_method_to_witnesses`) that mirrors the NPC arm of `send_entity_method`.

Alternatively, extend `send_entity_method` to accept a flag indicating AoI fanout regardless of entity type, but that changes the existing NPC semantics so a new helper is safer.

### Issue #232 — AoI entry hardcoded zero

**File**: `crates/services/src/mercury/aoi/create.rs`

```rust
// Before:
pub fn build_create_entity_cascade(
    key: ..., seq_id: ..., acks: ..., entity_id: u32, class_id: u32, level: u8,
    npc_data: Option<&NpcAoIData>,
) -> Vec<u8>

// After:
pub fn build_create_entity_cascade(
    key: ..., seq_id: ..., acks: ..., entity_id: u32, class_id: u32, level: u8,
    npc_data: Option<&NpcAoIData>, state_field: u32,  // ← new
) -> Vec<u8>
```

Pass `entity.state_field` at all call sites. The XOR-delta handler is idempotent with the initial-value-of-0 assumption on the client, so passing the live state_field is safe.

### Issue #249 — BSF_Holster witness broadcast

**File**: `crates/services/src/cell/cell_methods/combatant.rs`

In the `REQUEST_HOLSTER_WEAPON` arm and `SET_CROUCHED` arm:
- `send_entity_method` will NOT work because it routes players to `EntityMethodCall` (owning client only). Use the witness-fanout helper described under #219 instead.
- Remove hardcoded `method_index: 19`; use `method_idx::ON_STATE_FIELD_UPDATE`.
- The owning player also needs the update (to see their own weapon holster), so send both `EntityMethodCall` to the owning player AND `WitnessEntityMethod` to all witnesses.

---

## 8. Open Questions

1. **`send_entity_method` semantics** — RESOLVED: `messaging.rs::send_entity_method` sends `EntityMethodCall` for player entities (owning client only) and `WitnessEntityMethod` fan for NPC entities. Bug B of #232 is confirmed: both the respawn `onStateFieldUpdate(0)` and the BSF_InCombat transitions for player entities need a separate "player-to-witnesses" fanout mechanism that does not exist today. The fix requires iterating `space_mgr.get_witnesses_of(player_entity_id)` and sending `WitnessEntityMethod` for each — the same pattern used by `send_entity_method` for NPC entities.

2. **`FUN_00e7b4c0` full behavior**: Only known to be the weapon-animation handler for bit 3 and bit 8. Full decompilation would confirm whether it also handles the weapon holster visual (hiding the mesh) or only the animation state machine transition. LOW priority — the broadcast fix is needed regardless.

3. **`BSF_AutoCycling` semantics**: Bit 1 is in the enum but has no corresponding server-side constant in `crates/services/src/cell/combat/state.rs`. Not set by any server code found. May be a client-only flag set by the auto-attack cycle; if so, `onStateFieldUpdate` for it is driven by a client→server→broadcast round-trip not yet traced.

4. **`BSF_Walking` source**: No server code found that sets bit 7. Likely driven by movement input processing or avatar update messages rather than a dedicated server signal. May be client-authoritative.

5. **Witness broadcast for `BSF_Dead` on NPC spawn**: When a dead NPC is in AoI (unusual but possible in respawn-timer window), does `build_create_entity_cascade` correctly represent the NPC's dead state? Currently no — the hardcoded-zero bug affects NPCs equally.

---

## 9. Key Addresses

| Address | Name | Role |
|---------|------|------|
| `0x00e01c90` | `GameBeing_MemberCallbackRtti_onStateFieldUpdate` | CME subscriber: ingests `bStateField`, XOR-delta dispatch, fires `Event_Entity_StateFieldChanged` |
| `0x00e7b4c0` | `FUN_00e7b4c0` (weapon anim handler) | Called by `FUN_00e01c90` for bit-3 (BSF_InCombat) and bit-8 (BSF_Holster) transitions |

See the "State-flag broadcast" subsection in `docs/reverse-engineering/address-map.md` for the full address registry.

---

## 10. Revised Analysis — Issue #249 (BSF_Holster) — W-holster-finder Session 5b

**Confidence**: HIGH (write site confirmed by binary search; implication is deduction)

### Correction to section 6

Section 6 stated that the holster animation is "driven by a different mechanism — likely a separate ClientMethod or a direct animation call." The W-holster-finder session found the actual mechanism:

**The posture/weapon-category byte at `entity+0x3D2` is written exclusively by `FUN_00ec0840` (`CompositedAppearanceProxy::ApplyToPawn`, `CompositedAppearanceProxy.cpp` at `0x00ec0840`).** This function is triggered by the appearance compositing pipeline (`Event_NetIn_BeingAppearance`) — NOT by the BSF state-flag system.

The byte-pattern search `88 ?? D2 03 00 00` across the entire binary returns a single match at `0x00ec08e5` inside `FUN_00ec0840`. There is no other writer.

### Revised root cause of #249

The holster visual state on the client is determined by the **weapon category** in `entity+0x3D2`, which is set when the server sends `Event_NetIn_BeingAppearance` with the equipped (or unequipped) weapon's `ComponentList`.

- **Draw weapon**: Server sends `BeingAppearance` with weapon component in `ComponentList` → appearance job → `entity+0x3D2` = weapon category (e.g., OneHanded=1, TwoHanded=2) → `USGWAnim_BlendByPosture` selects non-relaxed stance.
- **Holster weapon**: Server sends `BeingAppearance` with weapon component removed or set to relaxed category → `entity+0x3D2` = 4 (melee/relaxed) or similar → `USGWAnim_BlendByPosture` selects relaxed stance.

**BSF_Holster (bit 8)** appears to be a state-persistence bit — witnesses can query it to know the holster state, but it does not drive the animation system directly. The prior fix proposal (broadcast `onStateFieldUpdate` for holster) may still be needed for state persistence, but the visual animation requires a `BeingAppearance` update.

### Revised fix for #249

The complete fix requires two actions:
1. **BSF_Holster broadcast** (existing plan): Broadcast `onStateFieldUpdate` with BSF_Holster toggled to all AoI witnesses (so late-joining witnesses can query holster state via the state field).
2. **BeingAppearance broadcast** (new finding): Also broadcast an updated `Event_NetIn_BeingAppearance` to all AoI witnesses when the player holsters or draws. This triggers the appearance compositing on each witness's client, which writes `entity+0x3D2` and updates the visual stance.

Without the `BeingAppearance` broadcast to witnesses, witnesses will see the player's weapon remain in whatever stance the last appearance update set — holster toggle via BSF_Holster alone will NOT change the animation.
