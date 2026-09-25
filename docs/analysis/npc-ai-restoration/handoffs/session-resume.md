# NPC AI Restoration — Session Resume

> Type: how-to. Audience: the owner (UAT) and the next coordinator session.
> Updated: 2026-09-25. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md), [telemetry plan](../telemetry.md), [operator runbook](../../../operations/npc-ai-telemetry-runbook.md).

## State

The owner authorized an autonomous run on 2026-09-24: work the packets, merge when ready, then post `/release` on the last PR.

- **Merged:** every implementation packet in the ledger landed on `main` between 2026-09-24 and 2026-09-25.
- **Seen in a client:** nothing. Every behaviour packet is **UATPending**.
- **Deploy:** the last PR carries this close-out and gets the `/release` comment that ships the build to the colo.

| Packet | PR | What changed in play |
|---|---|---|
| NA00 | #776 | Nothing. The OTLP filter now exports the NPC debug seams, each log row carries `host.name`, `cimmeria.deploy_env` and `service.version`, and every AI state change is logged. |
| NA01 | #774 | Nothing. Logged ground heights are now correct on stacked floors. |
| NA02 | #781 | Nothing. Adds detectors for stale velocity, ground deviation, stuck, off-mesh, leash loop, path status, line of sight and cover. |
| NA03 | #782 | Nothing. Adds the SigNoz dashboard "Cimmeria — NPC AI health" and the `NPC AI —` saved views. |
| NA10 | #779 | **Running in place is fixed.** Stopped NPCs now broadcast zero velocity. The malformed `onSequence` that stood in for a "movement type" is no longer sent. |
| NA11 | #783 | **NPCs stay on the floor**, including on ramps and stairs, and when backing up. |
| NA12 | #785 | **Leash is measured on the NPC's own distance from spawn.** NPCs walk home, evade while returning and heal on arrival. A lost target also sends them home. The player's combat state clears. The 6 s aggro/leash loop is gone. |
| NA13 | #787 | **Faction-10 NPCs aggro on sight** within 18 u, on the same floor, with line of sight. Chain-armed spawns 20 and 10 wait for their chains. GMs can use `.aggro off`. UAT-1 found this held only for guards in the open: a guard spawned in cover looked from behind its prop and never saw anyone ([worknotes/uat-1.md](../worknotes/uat-1.md) finding 1). With NA23 it looks from the slot's peek point, and the row holds for guards in cover too. |
| NA14 | #789 | **Same-room assist.** The MessHall guards join each other. |
| NA15 | #788 | NPCs hold at mesh-island edges and then go home. They snap back onto the mesh, stop at least 1 u from their target, and can reach GMs standing off the mesh. |
| NA16 | #786 | **The drone fires across the vial desk.** Stationary NPCs ignore a navmesh "blocked" within their own floor band (D-NA11). |
| NA20 | #777 | Nothing. Evidence only: world-space cover lives in the map chunks. |
| NA21 | #780 | Nothing on its own. Cover seeds are now real world-space nodes per world: 236 for Cellblock, 3,788 for Castle. |
| NA22 | the close-out PR | **NPCs use cover.** They spawn holding it, seek a covered firing position in range and fire from it, and get Cover Stance on arrival. |
| NA30 | #775 | Nothing. Doc corrections: OnGround sentinel -13000.0, Leash = 5. |
| NA24 | #791 | UAT-1 findings 4-8. A dead player drops out of every threat list and cannot use items. `.bug` bookmarks list real witnesses. Col Marsh's follow ticks. `Castle_BravoOfficer3` stands on the mesh. An empty server writes far fewer tick rows. |
| NA23 | branch `npcai/na23-cover-los` | UAT-1 findings 1-3 (D-NA12). **Guards in cover see and shoot from a peek point past their prop,** so they aggro from cover and no longer shoot through walls. A mess-hall strafe no longer flips them out of cover, and a flanked guard does not re-take the same slot for 6 s. |

## Owner UAT checklist (colo, after `/release` deploys)

Play as GM with `.aggro on` (the default). Use `.bug <note>` at every oddity. The coordinator then reads SigNoz with the [runbook](../../../operations/npc-ai-telemetry-runbook.md).

1. **Aggro (NA13, NA14).**
   - Walk the Cellblock topside. Each room's guards should engage when you enter the room, and never through walls or floors.
   - Shooting one MessHall guard pulls the other.
   - The PRU does nothing until you pick up the vial. The first guard waits for Region8.
   - Try `.aggro off`: guards should ignore you.
2. **Movement (NA10, NA11, NA15).**
   - Guards that stop to shoot don't run in place.
   - A guard coming up the ramp toward you stays on the ramp surface.
   - Guards stop about 1 u short of you instead of inside you.
3. **Leash (NA12).**
   - Run past a guard's leash range or die. The guard walks home and heals; it doesn't teleport or freeze partway.
   - Your in-combat flag clears and regen starts.
4. **Drone (NA16).** The drone fires at you across the med-station desk at 12–16 m.
5. **Cover (NA22).**
   - Guards hold their authored cover and fire from it. Guards in the open walk to nearby cover.
   - **Pose experiment:** does the model crouch at the slot with no extra message? See `findings/cover-world-placement.md` Q4. Confirm on the server side with `cover.stance event=granted`.
6. **Telemetry.**
   - The dashboard fills in.
   - `cimmeria.deploy_env = 'colo'` appears on rows. At UAT-1 it still read `dev`, because Watchtower does not re-apply the compose file; see the [runbook](../../../operations/npc-ai-telemetry-runbook.md#before-you-start).
   - `stale_velocity`, `ground_deviation`, `leash loop`, `idle_parked` and `cleared_without_exit` all read about 0.

UAT-0, the planned before-picture session (D-NA06), was not run: the behaviour fixes shipped in the same release as the telemetry. The pre-fix baseline is the 2026-09-18 to 09-21 colo SigNoz data in [evidence/signoz-npc-mining.md](../evidence/signoz-npc-mining.md).

## Open items for the owner

| Item | Why it's open | Where |
|---|---|---|
| Collision-geometry line of sight | The navmesh ray is wrong on 45% of its same-floor "blocked" answers. D-NA11 is only a stopgap for stationary NPCs, and the proper fix needs a per-world data-size decision. | #784 |
| Stationary NPCs shooting through same-floor walls | This is the known cost of D-NA11. About 40 stationary spawns are affected, including Harset. | README D-NA11 |
| SGC_W1 Ba'al Jaffa now aggro on sight | D-NA01 is a global rule. Add NEUTRAL `aggression_override` rows if that zone should stay passive. | PR #787 |
| `onAggressionOverrideUpdate` client broadcast | The client handler is `0x00d31bd0`. The SGWMob method index (27 by the flattening rule) is not binary-verified, so nothing is sent. | `docs/gameplay/npc-ai.md` |
| Cover Stance has no combat effect | `COVER_DEFENSE` isn't read by hit resolution yet. The magnitude is also undecided: +100 from the effect row or +200 from the ability text. | PR for NA22 |
| Cover pose | No server wire exists (D-NA10). Whether the client crouches from position alone is decided by the owner experiment above. | `findings/cover-world-placement.md` |
| Step back when a player hugs a stationary attacker | Not done, because it would make melee NPCs retreat. | PR #788 |
| Radii tuning | The D-NA09 defaults (aggro 18, assist 10, leash 50, band 4) are starting values; tune them from UAT. | template columns `aggro_radius`, `assist_radius`, `leash_distance` |

## If something is wrong after deploy

- Start from the dashboard and the saved views (the runbook maps each question to a view).
- Each packet's behaviour lives in a small number of modules:
  - `npc_ai/transition.rs`, `idle_aggro.rs`, `aggro_gates.rs`, `assist.rs`, `chase/`, `leash/`, `fight*.rs`;
  - `ticks/npc_ground.rs`;
  - `cover/`.
- Each PR description lists its revert-proven guards.
