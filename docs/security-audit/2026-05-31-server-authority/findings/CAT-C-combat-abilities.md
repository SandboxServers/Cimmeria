# CAT-C — Combat / Abilities — Findings

**Overall trust posture (CAT-C):** The combat dispatch surface enforces the
session→player binding at the framing layer (`base/connect_loop/cell_arms.rs`
substitutes the session's `player_eid` for any client-supplied `entity_id`
prefix), and the QR/damage formula is fully server-side with no
`damage_override` field on the wire. Cooldown timing uses server-tracked
`Instant` (not client timestamps), and ammo is decremented server-side from
the bandolier slot — there is no client-supplied quantity. `trainAbility` is
validated end-to-end (archetype tree, level gate, prerequisites,
already-known no-op) before the base-side debit. AoE secondary damage is
correctly gated on primary-commit and clamped to the attacker's space.

That said, **CAT-C has multiple high-severity server-authority gaps** in
the player-controlled paths that drive ability resolution:

1. **No "is dead" guard on `respawn` or `callForAid`**. A live player can
   send these messages and be teleported + healed to full.
2. **`callForAid` accepts any `respawnerID` from the global table**, with
   no membership check against the player's current world or any
   access-list — turning the Defeat Window into an arbitrary teleporter
   to any respawn point in the database.
3. **No friendly-fire / faction / hostility check on single-target
   `useAbility`**. Players can damage other players, friendly NPCs
   (vendors, quest givers), and any in-range entity by id — PvP is
   undesigned today but the wire path is fully open.
4. **No LOS / navmesh / aggro-list validation on `useAbility` target
   selection**. The only target gate is "exists, alive, within range" —
   a modified client can fire through walls, at entities not in its AoI,
   at AI mobs leashed behind doors, etc.
5. **No "caller is stunned / movement-locked" guard on `useAbility`**.
   `BSF_MOVEMENT_LOCK` and future stun flags are ignored on the fire
   path — a stunned player can still cast.
6. **`setTargetID` accepts any i32 with no validation** (target exists,
   in AoI, same space, alive). The stored id then drives the auto-cycle
   tick's re-fire target.
7. **`useAbilityOnGroundTarget`'s ground point is unchecked for
   distance from the attacker** — a player can ground-click anywhere
   on the map; secondary targets are filtered by the click radius, but
   the click itself is never validated against the attacker's location.
   Also no NaN/Infinity guard on the floats.
8. **Min-range is never enforced** — abilities with a `min_range` (e.g.,
   shotgun-style or grenade-arc weapons) can be fired point-blank with
   damage applying.
9. **`requestHolsterWeapon` and `setCrouched` have no in-combat / dead-state
   guard** — a dead player can toggle their visual state, breaking
   appearance invariants.
10. **AI-debug client messages (`CombatDebug`, `HealDebug`, `AbilityDebug`,
    `DebugAbilityOnMob`, `EnterErrorAIState`, etc.) are entirely
    unimplemented** server-side. They are dispatchable wire surfaces
    today (the client emits them, the server's router accepts the
    method index range, the per-interface handlers either no-op them
    or return false). When implementation lands, every one needs an
    explicit `is_gm` session check — these reveal mob internals and
    debug AI state that must be GM-only.
11. **`confirmationResponse` is parsed but the `effect_id` and
    `accepted` flag are only logged.** Future use must validate the
    effect_id against an outstanding server-issued prompt for this
    player; today it's harmless but the wire shape is in place.
12. **Pet methods (`PetInvokeAbility`, `PetAbilityToggle`,
    `PetChangeStance`) are stubbed.** When implemented, pet ownership
    must be validated against a server-side pet→owner table before any
    pet ability fires; today the stubs accept arbitrary `pet_entity_id`
    from the client and log them.

The auto-cycle loop driver (`service/ticks/auto_cycle.rs`) **does**
properly read `current_target_id` live, re-validates range and target
aliveness per tick, and routes through the kill-credit wrapper. That's
the well-built end of CAT-C and the canonical pattern other handlers
should follow.

---

### CAT-C-01 — `respawn` / `callForAid` heal a non-dead player to full

**Severity**: High
**Class**: Missing state precondition (server fails to gate on "actor is dead")
**Wire surface**: `Event_NetOut_Respawn` (cell method 70), `Event_NetOut_callForAid` (cell method 67)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
A client at any HP can send `respawn` (no args) or `callForAid(respawner_id)`.
The cell-method dispatcher routes both to `handle_respawn`, which
unconditionally resets `HEALTH` and `FOCUS` to `max`, clears every state
flag (including `BSF_IN_COMBAT` and `BSF_MOVEMENT_LOCK`), drops all
ability cooldowns, snaps the player to a respawner position, and
re-broadcasts an `onStateFieldUpdate(0)` to lift the dead-cursor.
**Nothing in the handler checks that `BSF_DEAD` is set on the calling
entity.** A live player who is on a 60-second ability cooldown, taking
DoT damage, low on HP, or in combat can use this as a panic button that
resets every combat state and teleports them out.

**Evidence**
- Ghidra: `0x0195f9c0` `Event_NetOut_callForAid`, `0x019b33ac` `Event_NetOut_Respawn` — client RTTI strings registered via `register_NetOut_callForAid` / `register_NetOut_Respawn`. Wire format per `docs/protocol/cell-method-dispatch-table.md:272`: `callForAid` carries `INT32 respawnerID`; `respawn` carries no args. The client emits these from the Defeat Window UI on death OR from the auto-respawn timer, but the wire path has no "is-dead" predicate — any in-game key remap (or modified client) can fire them at will.
- Client behavioral log: n/a (client always emits these from the Defeat Window, so live log won't show abuse; abuse requires a tweaked client).
- Cross-ref to Rust handler (for the fix author, NOT as truth): `crates/services/src/cell/cell_methods/player/combat/mod.rs:30-37` (CALL_FOR_AID dispatch), `:108-112` (RESPAWN dispatch), `crates/services/src/cell/cell_methods/player/combat/respawn.rs:73-264` (`handle_respawn` body — no `BSF_DEAD` precondition).

**Attack scenario**
1. Player is in combat at 5 HP, ability on 30s cooldown, BSF_IN_COMBAT set, threatened by 3 NPCs.
2. Client sends `respawn()` (0xBA + 0xBD encoding for cell method 70 takes no args beyond the entity prefix).
3. Server: heals to full HP/Focus, clears all state flags (including IN_COMBAT and MOVEMENT_LOCK), clears all cooldowns, snaps to nearest respawner.
4. Observable effect: every cooldown timer resets, player is at full HP at a safe respawn point, all aggro is broken via the `threatened_mobs.clear()` + state-flag reset.

**Suggested remediation (one line)**
Reject the call when `!is_dead_state(entity.state_field)`; log at `warn!`. Consult `combat-systems-advisor` if there's a design intent to allow non-dead respawn (suicide button) — if so, the path must at minimum incur a death first, not a free heal.

**Would benefit from x64dbg trace?**
No — the wire format is documented, the handler code is the trust violation, no debugger needed.

---

### CAT-C-02 — `callForAid` accepts any `respawner_id` from the global table

**Severity**: High
**Class**: Missing access-list validation (arbitrary teleport)
**Wire surface**: `Event_NetOut_callForAid` (cell method 67)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`callForAid(respawnerID)` is supposed to be the player's chosen
respawn point from the list the server pushed in `onBeginAidWait`
(damage_apply/mod.rs:411-416 — server sends the per-world filtered
respawner list). The cell-method handler passes the client-supplied
`respawner_id` straight to `resolve_respawn_target`, which does
`space_mgr.respawners.iter().find(|r| r.respawner_id == respawner_id)`
across the **entire global respawner table** (loaded via
`load_respawners` from `resources.respawners`). There is no
membership check against the list the server actually offered, no
world-name match against the dying player's current world, no
per-faction or per-mission gating. A client that sends any positive
`respawnerID` known to exist in the DB gets teleported there.

**Evidence**
- Ghidra: `0x0195f9c0` `Event_NetOut_callForAid`. Wire shape `docs/protocol/cell-method-dispatch-table.md:272` — `INT32 respawnerID`.
- Client behavioral log: n/a (live client only sends UI-chosen ids).
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/combat/respawn.rs:281-329` (`resolve_respawn_target` — does not constrain the respawner_id to the player's current `onBeginAidWait` list or to the player's world; only falls back through priority levels if the id is not found).

**Attack scenario**
1. Player dies in Castle_CellBlock (the tutorial world). Server sends `onBeginAidWait` listing only Castle_CellBlock respawners.
2. Client ignores the listed ids and sends `callForAid(respawnerID = 12345)` where 12345 is a respawner in, say, the endgame world Othala.
3. Server resolves 12345 against the global table → returns `(Othala, [x,y,z])`.
4. Observable effect: the cross-world branch fires `GateTravel { target_world_name: "Othala", position: ... }`, teleporting the dead-character into endgame content. Combined with CAT-C-01 (live-player respawn), this is a free teleport to anywhere in the game world a respawner exists.

**Suggested remediation (one line)**
Validate `respawner_id` is in the per-world list returned by `respawners_for_world(dying_player_world)` and matches an outstanding `onBeginAidWait` offer recorded on the player session — drop anything else with a `warn!`. Same authoritative list both `onBeginAidWait` and the validator should consume.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-03 — `useAbility` allows player→player damage (no friendly-fire / faction check)

**Status**: ✅ RESOLVED (#444) — the single-target validation block in
`handle_use_ability` (`cell/abilities/use_ability/mod.rs`) now rejects,
**for player attackers**, any target that is a player or a non-hostile NPC
(`entity.is_player && (target.is_player || target.faction !=
HOSTILE_FACTION)`) before the damage pipeline, mirroring the AoE
(`abilities/dispatch.rs`) and cone (`abilities/cone_aoe.rs`) faction
filters, with a `warn!` for the attempted friendly-fire. Closes the
vendor/quest-NPC/party-member and forged-player-target vectors. The gate
is scoped to player attackers because NPC AI fight calls the same entry
point to attack a *player* (legitimate combat).
Single-target abilities resolve as damage unconditionally today (no
offensive/supportive field exists on `AbilityDef`), so supportive
single-target abilities (heal/buff an ally) will need the inverse gate
when that field is added — documented as a TODO at the guard. The flat
`HOSTILE_FACTION` sentinel is the same PvP seam the cone module already
flags for a future per-pair hostility model.

**Severity**: High
**Class**: Missing faction/hostility gate
**Wire surface**: `Event_NetOut_UseAbility` (cell method 68)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_use_ability` validates: ability exists, ability is known, ability
not on cooldown, target exists, target alive, target in range. It does
**not** check target faction or `target.is_player`. The downstream
`apply_damage_to_target` will gladly subtract HEALTH from any entity id —
another player, a vendor NPC (faction 0), a quest-giver, a friendly
escort mob — using the full QR + armor pipeline. SGW is PvE-only by
design today (`cone_aoe.rs:24` documents the assumption), but the wire
path doesn't enforce that. The cone-AoE path correctly scans only NPCs
with `HOSTILE_FACTION`; the **single-target path has no such filter**.

**Evidence**
- Ghidra: `0x019b37c4` `Event_NetOut_UseAbility`. Wire `INT32 abilityId, INT32 targetId` (`docs/protocol/cell-method-dispatch-table.md:274`).
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/use_ability/mod.rs:139-171` (validation block — no `is_player` or faction check); `crates/services/src/cell/abilities/damage_apply/mod.rs:50-545` (no hostility filter, damage applies regardless of target type).

**Attack scenario**
1. Player A enables auto-target of player B (or just looks up B's entity_id via the AoI fan-out / wire-log).
2. Client sends `useAbility(ability_id = <ranged weapon>, target_id = B_entity_id)`.
3. Server: range check passes (they're in the same instance), alive check passes, fires the full damage pipeline against player B.
4. Observable effect: B's HEALTH stat decrements, `onEffectResults` shows up on B's client (and on A's, plus AoI witnesses). At lethal damage, B is sent to the Defeat Window. Same attack works against any neutral or friendly NPC — vendors, mission contacts.

**Suggested remediation (one line)**
Add a `target_is_attackable(attacker, target, ability)` gate in `handle_use_ability` before `apply_damage_to_target`: reject when `target.is_player` (until a real PvP/duel state machine lands) or when target faction is not in the attacker's hostile-to-me set. Route through the same `HOSTILE_FACTION` sentinel as cone AoE for now; consult `combat-systems-advisor` for the long-term faction model.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-04 — `useAbility` target has no LOS / navmesh / AoI membership check

**Severity**: High
**Class**: Missing visibility/reachability gate
**Wire surface**: `Event_NetOut_UseAbility` (cell method 68)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The only spatial check between attacker and target is `position.distance_to`
against the ability's `max_range`. There is no:
- **Line-of-sight check** — the attacker can shoot through walls.
- **Navmesh-reachability check** — the attacker can hit a target standing
  in an inaccessible volume (under-the-map collision, locked rooms).
- **AoI membership check** — the attacker can hit an entity that is NOT
  in their AoI witness list (so the client wouldn't normally even know
  about it), as long as it's in the same coordinate space and within
  `max_range`. The wire-log can leak entity ids of mobs outside AoI
  during transient frames (entity creation/destruction); a modified
  client can stash those ids and fire on them later.

**Evidence**
- Ghidra: `0x019b37c4` `Event_NetOut_UseAbility` — payload is `INT32 abilityId, INT32 targetId`, no LOS bit.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/use_ability/mod.rs:139-171` — only `entity.position.distance_to(&target.position)` is consulted; `crates/services/src/cell/abilities/damage_apply/mod.rs` proceeds without re-validating LOS.

**Attack scenario**
1. Player observes the AoI fan-out of a mob behind a door (e.g., the mob spawned, then the player left AoI but kept the id).
2. Walks within `max_range` of the door's coordinates (but the mob is unreachable from the player's pathing graph).
3. Client sends `useAbility(weaponAbility, mobBehindDoor)`.
4. Server: range check passes, mob isn't dead, fires.
5. Observable effect: mob takes damage and dies through a wall. Variant: shooting at a target standing in a sniper-only nest, an unreachable balcony, or inside a quest objective area whose door isn't opened yet.

**Suggested remediation (one line)**
Add an LOS check (raycast against the cell's static-collision data) and an AoI membership check (`target_eid` must be in `attacker.witnesses` or a 1-frame transition set) before any damage applies. Consult `movement-physics-advisor` for the LOS primitive — navmesh-reachability is overkill for a per-fire check but LOS is mandatory.

**Would benefit from x64dbg trace?**
Yes — confirming the client-side ability button isn't itself gating these (i.e., whether it's a "client wouldn't send this" vs. "client would happily send this") would tighten the severity case. The fact that the auto-cycle tick re-uses `current_target_id` LIVE strongly suggests the client never lets go of the stash even when LOS breaks, but a debugger trace would pin it.

---

### CAT-C-05 — `useAbility` has no caller-stunned / `BSF_MOVEMENT_LOCK` guard

**Severity**: Medium
**Class**: Missing caller-state precondition
**Wire surface**: `Event_NetOut_UseAbility` (cell method 68), `Event_NetOut_useAbilityOnGroundTarget` (cell method 69)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The caller-side validation in `handle_use_ability` only checks
`is_dead_state(entity.state_field)` (i.e., `BSF_DEAD` bit). It does
not check `BSF_MOVEMENT_LOCK` (set on death, future stuns, fear, root)
or any other "you can't act" condition. When stun/fear/root effects
are wired up (the `cell/effects/` framework already exists),
`useAbility` will need a corresponding gate. Today the gap is dormant
because no effect script sets `BSF_MOVEMENT_LOCK` on a live entity
outside the death path — but stun is in the design.

**Evidence**
- Ghidra: `0x019b37c4` `Event_NetOut_UseAbility`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/use_ability/mod.rs:122-124` (`is_dead_state` check, no `BSF_MOVEMENT_LOCK` check); `crates/services/src/cell/combat/state.rs:46-51` (`BSF_MOVEMENT_LOCK` is documented as multi-source for "future stun/fear effects").

**Attack scenario**
1. NPC casts a stun on player → server sets `BSF_MOVEMENT_LOCK` (with the planned effect framework).
2. Client sends `useAbility(weaponAbility, target)` during the stun window.
3. Server: only checks `BSF_DEAD`; stun bit is set but not consulted; ability fires normally.
4. Observable effect: stunned player still attacks. Variant: rooted player still uses abilities, fearful player still casts.

**Suggested remediation (one line)**
Add `if entity.has_state_flag(BSF_MOVEMENT_LOCK) || entity.has_state_flag(<stun-bit>) { return false; }` to the caller-state validation block; consult `combat-systems-advisor` for the canonical "cast-blocking" state-flag set.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-06 — `setTargetID` accepts any i32 with no existence/space/AoI validation

**Severity**: Medium
**Class**: Server stores unvalidated client input that drives later authoritative behavior
**Wire surface**: `Event_NetOut_SetTargetID` (cell method 0 — SGWBeing interface)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`cell_methods/being.rs:20-63` accepts `target_id: INT32` from the
client and stores it as `entity.current_target_id` with the only
filter being `target_id > 0 ? Some(target_id) : None`. No check that
the entity exists, is in the player's AoI, is in the same space, or
is alive. The stored value drives the auto-cycle loop driver
(`service/ticks/auto_cycle.rs`), which does re-validate per tick
(target exists, alive, in range) — so the immediate damage is gated
elsewhere. But the server also **broadcasts `onTargetUpdate(target_id)`
to every AoI witness** (being.rs:46-61), so an attacker can publish
that they "target" an arbitrary entity id, including ids the
witnesses can't see — useful for griefing UI ("Player X is targeting
you" when X has no actual line on the witness), for social-engineering
duels, or for fingerprinting the entity ID space (poke a range of
ids, see which broadcasts the server lets through).

**Evidence**
- Ghidra: `0x019bf55c` `Event_NetOut_SetTargetID`. Wire `INT32 targetId` per `docs/protocol/cell-method-dispatch-table.md:48`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/being.rs:19-63` — stores `target_id` and fans out to witnesses, no existence check.

**Attack scenario**
1. Client sends `setTargetID(arbitrary_i32)` where `arbitrary_i32` is any positive id (does not have to exist on the server).
2. Server: stores `entity.current_target_id = Some(arbitrary_i32)`, broadcasts `onTargetUpdate(arbitrary_i32)` to AoI witnesses.
3. Observable effect: witnesses' clients receive a "this player is now targeting entity X" packet for an entity they cannot see. Combined with the auto-cycle tick's re-validation, the player's auto-fire never actually triggers (the tick filters invalid targets), but the broadcast still went out — wire-spam, UI confusion, light info disclosure.

**Suggested remediation (one line)**
Reject (or store `None`) when `target_id > 0` and the entity is not present in `space_mgr` OR not in the player's AoI; only broadcast `onTargetUpdate` for ids that survive that filter.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-07 — `useAbilityOnGroundTarget` doesn't validate ground point or guard against NaN/Inf

**Severity**: Medium
**Class**: Missing input validation on client-supplied floats
**Wire surface**: `Event_NetOut_useAbilityOnGroundTarget` (cell method 69)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
The handler reads `[x, y, z]` directly from the wire as three f32s
(`cell_methods/player/combat/mod.rs:60-63`) and passes them straight to
`handle_use_ability_on_ground`, which uses them as the AoE center. There
is no check that the ground point is:
- Within `max_range` of the attacker (the **target** is range-checked
  against the attacker, but the **click point** is not).
- Within the world's MinX/MaxX/MinY/MaxY/MinZ/MaxZ bounds.
- Finite (no NaN, no Infinity). NaN comparisons fall through to false
  in the in-radius check, so the practical effect is "no targets in
  radius" → primary cast still commits cooldown/ammo. Infinity drives
  the cone-AoE math to produce NaN target lists. Both are denial-of-
  service-flavor inputs more than damage cheats.

The biggest behavioral consequence: the player can ground-click at
any coordinate, and as long as a real hostile NPC sits near the
attacker's position (within the ability's `max_range` of the attacker),
the cast commits and damages NPCs within the **click point's radius**
even when the click point is far from the attacker. This is mild
because the actual damaged targets must be within the click's
`Radius` NVP (which is small — typical 5-10m) and within the click
point itself, so the "wrong click point" attack is bounded. But
combined with a faulty radius lookup or a 1-shot AoE ability with
no falloff, it's a cheap aimbot helper.

**Evidence**
- Ghidra: `0x019bb70c` `Event_NetOut_useAbilityOnGroundTarget`. Wire `INT32 abilityId, FLOAT x, FLOAT y, FLOAT z` per `docs/protocol/cell-method-dispatch-table.md:275`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/dispatch.rs:81-273` (`handle_use_ability_on_ground` — only target-vs-attacker range is checked, click-vs-attacker is not; no NaN/Inf or world-bounds guard).

**Attack scenario**
1. Client sends `useAbilityOnGroundTarget(grenadeAbility, x=1000000.0, y=0.0, z=0.0)`.
2. Server: collects in-radius hostiles centered at (1000000, 0, 0) → empty.
3. `primary_in_range` is false → call falls through to `handle_use_ability(target_id=0)` → cooldown/ammo consumed, no damage. Minor — just spammed-cast cost.
4. NaN variant: client sends `x = NaN`. `dx*dx + dy*dy + dz*dz` is NaN, `<= radius_sq` is false everywhere → no targets caught → benign. **But:** `if primary_in_range { ... }` evaluates target_in_range via `attacker.position.distance_to(&target.position)` (no NaN involvement) — so this path is fine. The danger is future code that consumes `ground[]` for navmesh queries or world-edge checks downstream — NaN propagation will silently break invariants there.

**Suggested remediation (one line)**
Reject the call when any component of `ground[]` is non-finite (`!x.is_finite()`), and reject when `attacker.position.distance_to_xyz(ground) > max_range + ability_radius` (the click must be within reach + AoE radius of the attacker).

**Would benefit from x64dbg trace?**
Yes — confirming the client-side aim system would never produce a click outside reach would let us deprioritize, but the wire format permits it.

---

### CAT-C-08 — `useAbility` does not enforce `min_range`

**Severity**: Low
**Class**: Missing range-band enforcement
**Wire surface**: `Event_NetOut_UseAbility` (cell method 68), `Event_NetOut_useAbilityOnGroundTarget` (cell method 69)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`AbilityDef` carries a `min_range: i32` field but the validation block in
`handle_use_ability` only consults `max_range`. Abilities authored with
a positive `min_range` (shotgun-style spread weapons or grenade arcs
where point-blank fire is meant to be unavailable) can be fired with
the muzzle touching the target. The damage formula has no compensating
falloff inside the min-range band, so this typically produces
maximum-damage point-blank shots from weapons designed to require
spacing.

**Evidence**
- Ghidra: `0x019b37c4` `Event_NetOut_UseAbility`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/use_ability/mod.rs:152-169` (only `max_range` consulted); `crates/entity/src/abilities/...` `AbilityDef::min_range` exists in the schema. Same gap in `dispatch.rs:143-149` for the ground-target path.

**Attack scenario**
1. Client equips a shotgun-style ability with `min_range = 5` (designed to require 5m spacing).
2. Walks into the target at point-blank range (0.5m).
3. Fires.
4. Observable effect: full damage applies; UI may even allow the firing because the client-side animation cones don't gate.

**Suggested remediation (one line)**
Add `if d.min_range > 0 && dist < d.min_range as f32 { send onErrorCode(InsideMinRange); return false; }` to the range-check block alongside the existing `max_range` check.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-09 — `setCrouched` / `requestHolsterWeapon` have no caller-state guard

**Severity**: Low
**Class**: Missing dead-state precondition
**Wire surface**: `Event_NetOut_SetCrouched` (cell method 5 — SGWCombatant interface), `Event_NetOut_RequestHolsterWeapon` (cell method 7)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
A dead player can send `setCrouched(true|false)` and the server flips
the `BSF_CROUCHING` bit on the entity and broadcasts
`onStateFieldUpdate`. Same for `requestHolsterWeapon` — a dead player
can toggle the holster visual mid-Defeat-Window, firing a
`RefreshAppearance` to all AoI witnesses. Neither matches Python ref
behavior (`SGWBeing.py:746-770` — both are gated on alive).
Observable surface is cosmetic (state-field bit flips on a corpse,
weapon visual flips on a ragdoll), but the state-flag refcount
machinery is not re-evaluated post-respawn cleanly when these
mid-death toggles happened — the same-world respawn does
`clear_all_state_flags` which would catch this, but cross-world
respawn destroys the entity entirely so any cross-bit invariants
across that window are out of scope.

**Evidence**
- Ghidra: `Event_NetOut_SetCrouched`, `Event_NetOut_RequestHolsterWeapon` (RTTI strings present, not enumerated above but listed in surface inventory).
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/combatant.rs:31-94`. The `SET_CROUCHED` arm has no `is_dead_state` check. The `REQUEST_HOLSTER_WEAPON` arm doesn't either.

**Attack scenario**
1. Player dies, is in Defeat Window.
2. Client sends `requestHolsterWeapon(0)` (draw weapon) while ragdolled.
3. Server: clears `weapon_holstered`, fires `RefreshAppearance` to all AoI witnesses.
4. Observable effect: weapon mesh attaches to the ragdolled corpse. Cosmetic but indicates the dead-state guard is missing.

**Suggested remediation (one line)**
Reject `SET_CROUCHED` and `REQUEST_HOLSTER_WEAPON` when `is_dead_state(entity.state_field)` is true; mirror the Python reference's alive-only gate.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-10 — AI-debug client messages are unimplemented; future implementation must be GM-gated

**Severity**: Medium (forward-looking — gap exists when handlers land)
**Class**: GM-gated wire surface with no handlers (today) and no skeleton ACL gate
**Wire surface**: `Event_NetOut_CombatDebug`, `Event_NetOut_CombatDebugVerbose`, `Event_NetOut_HealDebug`, `Event_NetOut_AbilityDebug`, `Event_NetOut_DebugAbilityOnMob`, `Event_NetOut_DebugBehaviorsOnMob`, `Event_NetOut_DebugPathsOnMob`, `Event_NetOut_EnterErrorAIState`, `Event_NetOut_ExitErrorAIState`
**Demonstrable / Likely-theoretical**: Likely-theoretical (no implementation today, but the wire surface is live)

**Trust violation**
The client emits all of these (Ghidra-confirmed RTTI strings at
`0x019b2fe8`, `0x019b3020`, `0x019b305c`, `0x019b3d1c`, `0x019b44a8`,
etc.). Server-side: `cell_methods/ability_manager.rs` has stubbed
`TOGGLE_COMBAT_DEBUG` and `TOGGLE_COMBAT_VERBOSE_DEBUG` as no-ops;
`combatant.rs` has `TOGGLE_HEAL_DEBUG` as a stub; the rest don't have
handlers wired at all (the cell-method dispatch returns `false` and
the router warn-logs as "Unhandled cell method call"). When these are
implemented (and they will be — they're useful for live debugging
mob AI), every one of them needs an explicit `is_gm` session check.
They reveal mob internal state (current behavior, threat list,
nav-path), and EnterErrorAIState / ExitErrorAIState would let a
client force mobs into broken AI states (deadlock, lockout) — a
denial-of-service against world content.

The forward-looking risk: the existing handler patterns in
CAT-N's GM commands DO check the GM flag, so the template is in
place — but the AI-debug commands sit in the CAT-C interface space
(SGWAbilityManager and SGWCombatant), which today has NO
GM-check infrastructure at all. The team needs to drop the GM check
in alongside the implementation, not after.

**Evidence**
- Ghidra: RTTI strings for every listed event confirmed present (queries above).
- Client behavioral log: n/a (debug commands aren't fired by normal play).
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/ability_manager.rs:22-25` (no-op stubs for combat-debug toggle), `combatant.rs:59-62` (no-op stub for heal-debug toggle), rest unimplemented.

**Attack scenario** (when handlers land without GM gating)
1. Client sends `Event_NetOut_DebugBehaviorsOnMob(mob_id)`.
2. Server (hypothetical future handler): responds with the mob's full behavior tree state — current node, transition history, scheduled actions.
3. Observable effect: information disclosure of mob AI internals. Variant: `EnterErrorAIState(mob_id)` forces a mob into the error state, allowing the player to disable any encounter mob at will.

**Suggested remediation (one line)**
File a "GM-gate the AI-debug interfaces" tracker now and add a regression test that any handler dispatched from method indices in the CombatDebug / HealDebug / AbilityDebug / Debug*Mob / *ErrorAIState ranges must read `session.is_gm` server-side before doing work.

**Would benefit from x64dbg trace?**
No — the surface is fully Ghidra-derivable.

---

### CAT-C-11 — Pet methods are stubbed; pet ownership must be validated at implementation time

**Severity**: Medium (forward-looking)
**Class**: Stubbed handlers that will accept arbitrary entity ids
**Wire surface**: `Event_NetOut_PetInvokeAbility`, `Event_NetOut_PetAbilityToggle`, `Event_NetOut_PetChangeStance` (cell methods 88-90)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`cell_methods/player/social.rs:15-59` parses `pet_entity_id` from the
wire and logs it as `UNIMPLEMENTED`. When the handlers land, they must
enforce that the client-supplied `pet_entity_id` is actually owned by
the calling player — server-side, via a `player→active_pet` mapping —
not by trusting the wire id. Otherwise a player can `petInvokeAbility`
on **any pet in the world** (which is shared across multiple players in
group encounters), forcing other people's pets to attack arbitrary
targets, change stance, etc.

**Evidence**
- Ghidra: `Event_NetOut_PetInvokeAbility` at `0x019b42a4` (RTTI string confirmed).
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/social.rs:15-59` — all three pet methods parse the wire `pet_entity_id` but do nothing with it.

**Attack scenario** (when handlers land without ownership gating)
1. Client sends `petInvokeAbility(pet_entity_id = other_player_pet, ability_id, target_id)`.
2. Server (hypothetical future handler): dispatches the ability on the named pet.
3. Observable effect: another player's pet starts attacking your target — griefing, faction-tag exploits.

**Suggested remediation (one line)**
At implementation time, validate `pet_entity_id == session.player.active_pet_eid` (or membership in the player's pet roster) before any pet behavior fires; reject otherwise. Same template as the vendor-template-id validation pattern in `cell_methods/player/vendor.rs:75-123`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-12 — `confirmationResponse` is parsed but unused; future use needs prompt validation

**Severity**: Low (forward-looking)
**Class**: Wire surface in place, handler is a debug log
**Wire surface**: `Event_NetOut_ConfirmEffect` (CONFIRMATION_RESPONSE cell method 4)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The handler reads `(effect_id: i32, accepted: bool)` and logs them
(`cell_methods/ability_manager.rs:26-32`). The intended use (per the
field comment "Respond to a confirmation prompt") is the server-side
"are you sure you want to engage in this risky action?" pattern —
e.g., confirming a destructive trade, joining PvP, or accepting a
duel. When this lands, the server must verify the `effect_id` matches
an outstanding server-issued prompt for **this specific player**,
and the prompt must have a TTL — otherwise a client can replay an
old prompt-confirm to bypass guard rails on a new action with a
guessable / leaked `effect_id`.

**Evidence**
- Ghidra: `0x019b4610` `Event_NetOut_ConfirmEffect` (RTTI string).
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/ability_manager.rs:26-33`.

**Attack scenario** (when handlers land without prompt validation)
1. Server pushes "are you sure you want to drop this?" prompt with `effect_id = 12345` for player A.
2. Player A's session leaks the effect_id (wire-log, friend who saw it on screen).
3. Player B replays `confirmationResponse(12345, accepted=true)` to bypass their OWN unrelated prompt with a matching id (if the id space is global/guessable).
4. Observable effect: confirmed action goes through without the prompt's intended guard.

**Suggested remediation (one line)**
At implementation time, keep a per-session set of outstanding `(effect_id, action)` tuples; reject `confirmationResponse` whose `effect_id` is not in the calling player's outstanding set; expire entries after a short TTL.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-13 — Auto-cycle target switch broadcasts to witnesses regardless of validity

**Severity**: Low
**Class**: Information disclosure / wire spam
**Wire surface**: `Event_NetOut_SetTargetID` (cell method 0)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`setTargetID` always fans `onTargetUpdate` to every AoI witness
(being.rs:46-61), even when the target id is `0` (deselect, which the
server correctly clears to `None`) or a non-existent id (which the
server happily broadcasts before any validation). A client can spam
`setTargetID(any_id)` to flood witnesses' wire and UI with bogus
target-update packets. The auto-cycle tick later filters invalid
targets, so no actual damage results, but the broadcast cost is
unbounded per inbound packet.

**Evidence**
- Ghidra: `0x019bf55c` `Event_NetOut_SetTargetID`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/being.rs:30-62` — the broadcast loop runs on the unvalidated `target_id`.

**Attack scenario**
1. Client sends `setTargetID(random_i32)` 100 times per second.
2. Server: writes 100×N witness packets per second per player (N = AoI witness count). Per-target-update CPU is small but bandwidth and per-witness client overhead is real.
3. Observable effect: wire bandwidth spike, witness clients render a target-update flicker for every spurious id.

**Suggested remediation (one line)**
Rate-limit `setTargetID` (e.g., one per 100 ms per player) and skip the broadcast when the resolved id is `None` or fails the existence/space/AoI check from CAT-C-06.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-14 — `respawn` always replenishes Focus, not just Health

**Severity**: Low
**Class**: Over-restoration on respawn
**Wire surface**: `Event_NetOut_Respawn` (cell method 70), `Event_NetOut_callForAid` (cell method 67)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_respawn` at `respawn.rs:142-148` sets both `HEALTH` and `FOCUS`
back to their `max` values. Combined with CAT-C-01 (no dead check), a
live player who has burned through their focus pool firing channelled
abilities can `respawn()` to instantly refill the focus bar without
paying the time-to-regen cost. Even with CAT-C-01 fixed, the design
question is whether respawn should refund Focus or just Health —
the python reference's `_doDoRevive` resets Health but the Focus
treatment depends on the class. Today it's unconditional and bypasses
class-specific regen mechanics.

**Evidence**
- Ghidra: `0x019b33ac` `Event_NetOut_Respawn`, `0x0195f9c0` `Event_NetOut_callForAid`.
- Client behavioral log: n/a.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/combat/respawn.rs:142-148`.

**Attack scenario**
1. Player burns 100% of focus pool casting a high-cost ability.
2. Client sends `respawn()` (gated by CAT-C-01 fix not yet shipped).
3. Server: refills both HEALTH and FOCUS to max.
4. Observable effect: full Focus refund mid-fight (assuming CAT-C-01 is not yet fixed). Once CAT-C-01 IS fixed, this is a moot point because only dead players hit this path, and at that point the design question is whether dead-respawn should refund focus.

**Suggested remediation (one line)**
Consult `combat-systems-advisor` on whether respawn should restore Focus by archetype-specific rules; in the interim, only restore Health and let the post-respawn regen tick refill focus naturally.

**Would benefit from x64dbg trace?**
No.

---

### CAT-C-15 — `useAbility` consumes ammo even when target is dead-but-still-in-space

**Severity**: Low
**Class**: Mild ammo-waste / quality-of-life seam (not exploit shape, but pin)
**Wire surface**: `Event_NetOut_UseAbility` (cell method 68)
**Demonstrable / Likely-theoretical**: Demonstrable

**Trust violation**
`handle_use_ability` checks `combat::is_dead_state(target.state_field)`
and returns `false` early without consuming ammo (use_ability/mod.rs
:142-150). That's correct. **But:** when target is `-1` (no-target /
self-buff) the function bypasses the target validation entirely and
fires — including consuming ammo for ranged abilities. A client that
sends `useAbility(rangedAbility, -1)` consumes one ammo round per
fire and emits all the cooldown/sequence packets but applies no
damage. This isn't really exploitable (the player just wastes their
own ammo) but it's a wire-shape that future grief-prevention should
think about — chained with the auto-cycle tick, a corrupted
`current_target_id = -1` could quietly drain ammo every cooldown
elapse.

**Evidence**
- Ghidra: `0x019b37c4` `Event_NetOut_UseAbility`.
- Cross-ref to Rust handler: `crates/services/src/cell/abilities/use_ability/mod.rs:488-501` (no-target branch consumes ammo, returns true).

**Attack scenario**
1. (Self-griefing or accidental) `current_target_id = -1` persists from a deselect glitch.
2. Auto-cycle tick re-fires `handle_use_ability(ability, target_id=-1)` every cooldown.
3. Server: consumes ammo, fires no-damage burst, restarts cooldown.
4. Observable effect: player's ammo drains over time with no damage output.

**Suggested remediation (one line)**
For weapon ranged abilities (`required_ammo > 0`), reject calls with `target_id <= 0` unless the ability is explicitly authored as a no-target self-buff. Mirrors the auto-cycle tick's `current_target_id` precondition.

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

- **"Server-trusted hit/miss roll"** — the QR roll is deterministic via a
  server-derived seed (`pseudo_random_seed(entity_id, ability_id, effect_seq)`)
  and the formula is fully server-side. The client never supplies a roll
  result. Cleanly server-authoritative.
- **"Damage formula could be exploited via stat-buff spam"** — buffs
  apply through server-side stat mutations, and the DAMAGE / PENETRATION /
  QR_MOD reads pull from `attacker.stats` (server state) not client args.
  Out of scope for CAT-C.
- **"`requestAmmoChange` validation"** — covered in CAT-D (inventory).
  The bandolier whitelist + slot-id keying are already in place per the
  reviewed handler at `cell_methods/inventory/bandolier.rs:518-712`.
- **"Cone AoE width / orientation spoofing"** — cone is anchored at
  attacker, oriented toward primary target — both server state. No
  client-supplied cone params. Solid.
- **"Auto-cycle target-switch race"** — the tick re-reads
  `current_target_id` live every 100 ms and re-validates target
  existence + range; a target switch during cooldown is handled
  correctly via the existing test `auto_cycle_tick_refires_at_live_current_target`.
- **"`useAbility` rate-limit"** — the server-side cooldown gates this;
  no separate rate-limit needed at the wire layer for the single-target
  path. Pickable to file if a per-IP burst guard becomes desirable
  later for DoS prevention, but not a server-authority gap.
- **"Channel cancellation on dead caster"** — `cancel_channels_from_attacker`
  is called inside `handle_use_ability` at the same-ability switch
  point. If the caster dies mid-channel, the death path
  (`apply_death_transition`) is responsible — out of scope for CAT-C
  (cross-system, channel lifecycle is `cell/effects/` territory).
- **"`requestHolsterWeapon` while in pending swap"** — the bandolier
  slot-swap choreography (`pending_slot_swap_at`) is the gate, already
  in place at `bandolier.rs:226-237` for ability fire. Holster toggle
  during swap is cosmetic and reverts on the tick.
- **"`setMovementType` server authority"** — that's CAT-B
  (movement). The setMovementType handler stores the cached byte and
  fans out; the cell method does not give the player movement
  authority that movement-physics-advisor doesn't already audit.
- **"AoE primary's `primary_in_range` bypass"** — looked at this
  closely. The check `attacker.position.distance_to(&target.position) <= max_range`
  is the same single-target gate, just applied per-target. No bypass
  shape under the current code; the **click point itself** not being
  range-checked is filed as CAT-C-07 because it's separate.
- **"Negative ammo via fire-during-reload race"** — the existing
  `reload_complete_at.is_some()` gate at use_ability/mod.rs:291
  is the documented fix; the comment cross-refs why this is the right
  shape. Already pinned.
