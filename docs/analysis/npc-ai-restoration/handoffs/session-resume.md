# NPC AI Restoration — Session Resume

> Type: how-to. Audience: the owner (UAT) and the next coordinator session.
> Updated: 2026-09-26 (NA44). Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [telemetry plan](../telemetry.md), [external handoff validation](../evidence/handoff-2026-09-26-validation.md), [operator runbook](../../../operations/npc-ai-telemetry-runbook.md).

## State

The owner authorized an autonomous run on 2026-09-24: work the packets, merge when ready, then post `/release` on the last PR. The owner then approved the UAT-1 follow-ups (NA23, NA24) and "work on all still open items" (NA26-NA38) on 2026-09-25.

- **Merged:** every packet from NA00 to NA38 is on `main`, from PR #774 (2026-09-25) to PR #817 (2026-09-26).
- **In flight (2026-09-26):** NA39 and NA40, and NA41-NA44 from the [external handoff validation](../evidence/handoff-2026-09-26-validation.md). See [work packets, phase 6](../work-packets.md#phase-6-external-handoff-follow-ups-2026-09-26).
- **Seen in a client:** UAT-1 (2026-09-25, build `059d6038`) covered NA10-NA22; its findings became NA23 and NA24. Everything merged after that build is **UATPending**.

| Packet | PR | What changed in play |
|---|---|---|
| NA00 | #776 | Nothing. The OTLP filter now exports the NPC debug seams, each log row carries `host.name`, `cimmeria.deploy_env` and `service.version`, and every AI state change is logged. |
| NA01 | #774 | Nothing. Logged ground heights are now correct on stacked floors. |
| NA02 | #781 | Nothing. Adds detectors for stale velocity, ground deviation, stuck, off-mesh, leash loop, path status, line of sight and cover. |
| NA03 | #782 | Nothing. Adds the SigNoz dashboard "Cimmeria — NPC AI health" and the `NPC AI —` saved views. |
| NA10 | #779 | **Running in place is fixed.** Stopped NPCs now broadcast zero velocity. The malformed `onSequence` that stood in for a "movement type" is no longer sent; no movement-type message to the client exists. |
| NA11 | #783 | **NPCs stay on the floor**, including on ramps and stairs, and when backing up. |
| NA12 | #785 | **Leash is measured on the NPC's own distance from spawn.** NPCs walk home, evade while returning and heal on arrival. A lost target also sends them home. The player's combat state clears. The 6 s aggro/leash loop is gone. |
| NA13 | #787 | **Faction-10 NPCs aggro on sight** within 18 u, on the same floor, with line of sight. Chain-armed spawns 20 and 10 wait for their chains. GMs can use `.aggro off`. |
| NA14 | #789 | **Same-room assist.** The MessHall guards join each other. |
| NA15 | #788 | NPCs hold at mesh-island edges and then go home. They snap back onto the mesh, stop at least 1 u from their target, and can reach GMs standing off the mesh. |
| NA16 | #786 | **The drone fires across the vial desk.** Stationary NPCs ignore a navmesh "blocked" within their own floor band (D-NA11). |
| NA20 | #777 | Nothing. Evidence only: world-space cover lives in the map chunks. |
| NA21 | #780 | Nothing on its own. Cover seeds are now real world-space nodes per world: 236 for Cellblock, 3,788 for Castle. |
| NA22 | #790 | **NPCs use cover.** They spawn holding it, seek a covered firing position in range and fire from it, and get Cover Stance on arrival. |
| NA30 | #775 | Nothing. Doc corrections: OnGround sentinel -13000.0, Leash = 5. |
| NA24 | #791 | UAT-1 findings 4-8. A dead player drops out of every threat list and cannot use items. `.bug` bookmarks list real witnesses. Col Marsh's follow ticks. `Castle_BravoOfficer3` stands on the mesh. An empty server writes far fewer tick rows. |
| NA25 | #792 | Nothing. Every log row that reaches the log files also reaches SigNoz; TRACE goes to the `cimmeria-trace` service. |
| NA23 | #793 | UAT-1 findings 1-3 (D-NA12). **Guards in cover see and shoot from a peek point past their prop,** so they aggro from cover and no longer shoot through walls. A mess-hall strafe no longer flips them out of cover, and a flanked guard does not re-take the same slot for 6 s. |
| NA26 | #794 | **Every world has a navmesh.** New meshes ship `advisory` (information for NPCs, no player containment). |
| NA29 | #795 | **Harset gate arrival** lands on the gate row again; five world-57 spawns are mobile. |
| NA28 | #796 | Tiled navmeshes: the seven large exteriors get whole-map coverage. |
| NA27 | #797 | **NPCs no longer see or shoot through same-floor walls.** Line of sight comes from per-world collision occluders (D-NA13); this closes #784. |
| NA33 | #806 | NPC aggression level is sent to clients (`onAggressionOverrideUpdate` / `Cleared`, D-NA16). Nameplate or hostility display is expected to follow. |
| NA34 | #808 | Nothing. Two-player visibility guard and `aoi.introduce` telemetry. |
| NA32 | #810 | **Cover reduces damage** (10-60% by material, D-NA15/D-NA15a), and the hit roll's direction is fixed. **Ranged NPCs step back** when a player hugs them. |
| NA31 | #811 | **A player's targeted ability needs line of sight** where the world has an occluder (error 39, D-NA14). Eyes sit at each body set's height. |
| NA35 | #814 | **Gate travel:** the dial opens almost at once, and a 1.5 s crossing hold lets the gate sequence play before the load screen. |
| NA36 | #815 | Harset's navmesh gains its raised platforms (`KActor` and friends; `InterpActor` opt-in). |
| NA37 | #816 | Nothing. Two real wire clients see each other in Castle, including under packet loss. |
| NA38 | #817 | The server now processes a client's reliable packets in order and drops retransmitted duplicates. A stalled gap is logged. |

## Owner UAT checklist (colo, after the next `/release`)

Play as GM with `.aggro on` (the default). Use `.bug <note>` at every oddity: the bookmark rows now also carry each NPC's `ability_ids`, `weapon_visual` and your `current_target_id` (NA44). The coordinator then reads SigNoz with the [runbook](../../../operations/npc-ai-telemetry-runbook.md).

1. **Aggro (NA13, NA14, NA23).**
   - Walk the Cellblock topside. Each room's guards should engage when you enter the room, and never through walls or floors.
   - A guard spawned in cover still sees you and engages from its peek point (NA23).
   - Shooting one MessHall guard pulls the other.
   - The PRU does nothing until you pick up the vial. The first guard waits for Region8.
   - Try `.aggro off`: guards should ignore you.
2. **Movement (NA10, NA11, NA15).**
   - Guards that stop to shoot don't run in place.
   - A guard coming up the ramp toward you stays on the ramp surface.
   - Guards stop about 1 u short of you instead of inside you.
   - An SMG guard with a clear line stops to fire rather than closing to 1 u. (Today it closes until the 2 s tick sees it in range; a range-aware stop is an owner decision in the validation ledger.)
3. **Attack presentation (handoff §27).**
   - Each shot plays a visible SMG or pistol fire animation, in time with the damage number.
   - Shots come at the ability's cooldown cadence, with no damage that has no animation.
   - Castle SMG guards currently fire the Pistol Shot fallback; NA43 gives them the SMG set. Note which animation you see.
4. **Facing (handoff §27).**
   - Strafe around a fighting guard: it keeps facing you and never fires backward.
   - Note any lag in the turn; the yaw updates on the 2 s AI tick.
5. **Leash and re-engage (NA12).**
   - Run past a guard's leash range or die. The guard walks home, animating a walk rather than sliding, and heals; it doesn't teleport or freeze partway.
   - Your in-combat flag clears and regen starts.
   - After it resets, walk back in: the guard engages again once the 5 s suppression ends.
6. **Death.**
   - A guard's fire stops the moment it dies, and the death animation plays once.
   - The corpse stays, loot works and mission progress counts.
7. **Drone (NA16).** The drone fires at you across the med-station desk at 12–16 m.
8. **Cover (NA22, NA32).**
   - Guards hold their authored cover and fire from it. Guards in the open walk to nearby cover.
   - Shots at a guard in cover from the front do less damage than from the flank (NA32).
   - A ranged guard you stand on top of steps back before firing (NA32).
   - **Pose experiment:** does the model crouch at the slot with no extra message? See `findings/cover-world-placement.md` Q4. Confirm on the server side with `cover.stance event=granted`.
9. **Line of sight (NA27, NA31).**
   - No guard shoots you through a same-floor wall (NA27).
   - Your own targeted ability at a guard behind a wall is refused with "You do not have Line of Sight to your target" (NA31).
10. **Aggression broadcast (NA33).** A chain-armed NPC (the first guard after Region8, the PRU after the vial) shows as hostile on its nameplate or target frame once armed.
11. **Col. Marsh escort (handoff §28).**
    - Marsh follows you visibly, with no idle-pose slide.
    - He survives one failed path query and keeps following. Expected to improve with NA41.
    - He re-follows after combat. Expected to **fail** until NA42.
    - The ring hop keeps him with you (chain 1173); relogging mid-escort leaves him behind (owner decision).
    - Mission-controlled removal still works (chain 1161).
12. **Castle (handoff §29).** Castle guards face, fire and deal damage only with a clear line, and never chase through walls. Castle has a navmesh and an occluder; the handoff's "no navmesh" premise is wrong.
13. **Other worlds (NA26, NA28, NA29, NA36).** Harset: the gate arrival lands on the gate row, the five mobile world-57 NPCs stay on the ground, and the raised platforms hold players and NPCs. Any other world: nothing floats or falls through.
14. **Gate travel (NA35).** The dial opens almost immediately, and the gate sequence plays before the load screen.
15. **Telemetry.**
    - The dashboard fills in.
    - `cimmeria.deploy_env = 'colo'` appears on rows. At UAT-1 it still read `dev`, because Watchtower does not re-apply the compose file; see the [runbook](../../../operations/npc-ai-telemetry-runbook.md#before-you-start).
    - `stale_velocity`, `ground_deviation`, `leash loop`, `idle_parked` and `cleared_without_exit` all read about 0.

UAT-0, the planned before-picture session (D-NA06), was not run: the behaviour fixes shipped in the same release as the telemetry. The pre-fix baseline is the 2026-09-18 to 09-21 colo SigNoz data in [evidence/signoz-npc-mining.md](../evidence/signoz-npc-mining.md).

## Open items for the owner

The items the external handoff raised that need a decision (ranged chase goal, AI tick stagger, Marsh relog chains, Jaffa ability sets, effect hit sequences, warmup deferral, D-NA08 attack LoS, per-NPC trace, `.aiinfo`, threat rows, a wireclient AI encounter) are listed with their evidence in the [validation ledger](../evidence/handoff-2026-09-26-validation.md#owner-decisions).

| Item | Why it's open | Where |
|---|---|---|
| SGC_W1 Ba'al Jaffa now aggro on sight | D-NA01 is a global rule. Add NEUTRAL `aggression_override` rows if that zone should stay passive. | PR #787 |
| Cover pose | No server wire exists (D-NA10). Whether the client crouches from position alone is decided by the owner experiment above. | `findings/cover-world-placement.md` |
| Stationary relaxation where no occluder applies | D-NA11 still covers endpoints outside an occluder's trimmed area. All 23 client worlds ship an occluder, so this is rare. | README D-NA11, D-NA13 |
| Radii tuning | The D-NA09 defaults (aggro 18, assist 10, leash 50, band 4) are starting values; tune them from UAT. | template columns `aggro_radius`, `assist_radius`, `leash_distance` |
| One-way player visibility | NA37 and NA38 found no server cause; the owner's next two-client session needs capturing. | work packets NA38 |

Closed since the 2026-09-25 resume: collision-geometry line of sight (#784, NA27), Cover Stance in combat (NA32), the step-back (NA32), and the `onAggressionOverrideUpdate` broadcast (NA33).

## If something is wrong after deploy

- Start from the dashboard and the saved views (the runbook maps each question to a view).
- Each packet's behaviour lives in a small number of modules:
  - `npc_ai/transition.rs`, `idle_aggro.rs`, `aggro_gates.rs`, `assist.rs`, `chase/`, `leash/`, `fight*.rs`, `step_back.rs`;
  - `ticks/npc_ground.rs`;
  - `cover/`, `abilities/damage_apply/cover_roll.rs`;
  - `space_manager/spatial.rs` (`attack_line_of_sight`), `space_manager/occlusion.rs` and the `cimmeria-occluder` crate.
- Each PR description lists its revert-proven guards.
