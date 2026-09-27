# UAT-1 (2026-09-25)

> Type: explanation. Audience: the coordinator, the owner and later NPC AI workers.
> Updated: 2026-09-25. Companions: [work packets](../work-packets.md) (NA23, NA24), [decisions](../README.md#decisions) (D-NA12), [session resume](../handoffs/session-resume.md), [operator runbook](../../../operations/npc-ai-telemetry-runbook.md), [cover system](../../../architecture/cover-system.md).

UAT-1 was the first owner session on a build with every NA packet merged:

- account 6 (Lomiada), Castle_CellBlock (world 12), on the colo;
- 2026-09-25 12:10-12:19 UTC;
- build `059d6038` (#790).

The evidence is the colo SigNoz, filtered on `service.version = '059d6038e7a6e9c34fe3cc1caa880b499de19c4d'`, and the owner's `.bug` notes. Two follow-up packets came out of it: NA24 (#791, merged as `73486cb2`) and NA23 (this worknote's companion branch).

## Confirmed working

These held in play and must not regress:

- Chain-armed spawns 20 (PRU) and 10 (first guard) waited for their chains.
- The MessHall guards assisted each other, both ways.
- No guard aggroed across floors.
- The PRU drone fired across the med-station desk at 12.3-12.8 u with `los_policy=stationary_relaxed` (D-NA11).
- Guards spawned in cover held their slot, and Cover Stance was granted and removed in balance.

## Findings

| # | Finding | Evidence | Fixed by |
|---|---|---|---|
| 1 | **Guards spawned in cover never proximity-aggro.** Their line-of-sight ray starts behind their own cover prop, which is a navmesh hole, so it stops within half a metre. | `Hallway01_Guard` (npc 100162) rejected the player `no_los` at 13.6 u (12:16:20, 12:16:32) and 8.1 u (12:16:44), on the same floor (dy 0.05). Its ray hit 0.34-0.42 u from the NPC. `Hallway02_Guard`'s ray hit at 0.0 u (its start snapped 0.59 u onto the mesh), and so did `Hallway04_Guard`'s; the MessHall pair was rejected at 11.5-15.6 u. There was one proximity aggro all session, at 4.6 u. Owner notes: "first hallway guard aggro range to short", "or he doesnt aggro at all". | NA23 |
| 2 | **Guards in cover shoot through walls.** `los_policy=in_cover_slot` skipped the line-of-sight check entirely. | All 31 `in_cover_slot` tick rows read `los=blocked` (MessHall_Guard1 15, Hallway02 8, MessHall_Guard2 6, Hallway01 2). `Hallway02_Guard` (100170), on slot 1200034 at (-113.5, 39.6, -63.0), shot and killed the player at (-130.3, 39.55, -79.5), 22.9-23.5 u away round two hallway corners, 12:17:16-12:17:20. Owner note: "they shoot thru walls". | NA23 |
| 3 | **Cover flank churn in the MessHall.** A threat standing almost side-on to a cover facing flips the 5 degree flank test as it strafes, and the NPC re-picks the slot it just left. | 100160: `stay_in_cover` at 12:14:06, `cover_released_flanked` at 12:14:08, chased to about 1 u while in range (`has_los=false`, `in_range=true`), and re-picked the same slot 1200053 at 12:14:14. 100161: flanked out of 1200046 at 12:14:06, arrived at 1200015 at 12:14:10, flanked again at 12:14:12. Cover Stance flipped on each hop. The `cover.flank_check` rows put all three flanks at a normalised dot of -0.17 to -0.20 (100-102 degrees off the node facing, 10-12 degrees past side-on), two metres of strafe from a dot of +0.17. | NA23 |
| 4 | **A dead player was re-targeted after a medkit heal while dead.** `Hallway02_Guard` killed the player; the dead player used a medkit, chain 4001 healed the corpse to 500 HP, and the next fight pass kept the target. | 12:17:20-12:17:28 | NA24 (#791): `useItem` is refused while `BSF_DEAD`, a `BSF_DEAD` target counts as dead whatever its HEALTH, and a dying player leaves every threat list |
| 5 | **`.bug` bookmarks listed no witnesses.** `witness_count` was 0 and `caller_witnesses_it` false on every bookmark row, even for a guard fighting the tester. | `playtest.bookmark.entity` rows | NA24 (#791): the snapshot reads the tester's AoI |
| 6 | **Col Marsh's follow never ticked.** Template 10 is class `being`, and both the AI tick and the movement tick listed mobs only. | No `npc_ai.tick` rows for the follow | NA24 (#791): `ai_driven_npc_entity_ids` |
| 7 | **`Castle_BravoOfficer3` was off the mesh.** An `npc_off_mesh` WARN every 30 s at 1.572 u, with nobody in Castle (402 rows). | `npc_ai.off_mesh` rows | NA24 (#791): spawn 244 moved onto the floor; one WARN for a spawn that never moved |
| 8 | **Idle tick volume.** About 14 `npc_ai.tick` rows a second with the server empty, because every hostile Idle NPC is ticked every 2 s since NA13. | `npc_ai.tick` rate | NA24 (#791): an unwitnessed Idle NPC writes one row per 60 s |
| 9 | **The colo still reports `deploy_env = dev`.** `cimmeria.deploy_env` reads `dev` and `host.name` is the container id, because Watchtower updates the image but does not re-apply the compose file. Until the owner re-applies it, a `cimmeria.deploy_env = 'colo'` filter returns nothing; filter on `service.version` or `host.name` instead. | every resource block | owner (compose re-apply) |
| 10 | **"The shoot animation doesn't play."** | owner note | not NPC AI scope: the pre-existing combat-animation issue |

## What NA23 changed

Findings 1 and 2 have one cause: the ray starts behind the cover. NA23 (decision D-NA12) gives every cover slot a **peek point**: where the NPC's shot clears the prop. `cover::find_peek` searches the navmesh from the node:

- **over the prop**, along the node's facing toward the defended side, 0.5-3.5 u in 0.25 u steps. The first sample that lands on the mesh within 0.3 u and has at least 1 u of clear floor ahead is taken;
- else **round either end** of the marker, 0.6 u past `width / 2`, if the NPC can walk there in at most 6 u.

An NPC standing within 1.5 u of the slot it holds sees a target when the ray from the peek point, or its own ray, is clear. Neither adds a shot through a wall: the peek point is past the prop, and a clear ray from the NPC crosses nothing. The same rule serves:

- the Idle aggro scan;
- the assist check;
- the attack check, logged as `los_policy=cover_peek`;
- the `npc_ai.tick` row.

The slot is reserved from spawn, so an Idle guard authored in cover gets it too.

Measured on `castle_cellblock.nav` and the world-12 cover seed:

| Guard | Slot | Peek |
|---|---|---|
| `Hallway01_Guard` | 1200037/3, a Mid counter 5 u wide (the walk round it is 9.4 u) | 1.08 u over it |
| `Hallway02_Guard` | 1200034/0 | 1.30 u over it |
| `MessHall_Guard2` | 1200053/0, a mess table | 3.36 u over it |
| `MessHall_Guard1` | 1200046/0, a mess table | 3.46 u over it |

Across all 236 world-12 markers, 150 peek over their prop (0.5-3.6 u out, median 1.6 u), 17 peek round it and 69 have no peek point. Every over-the-prop peek has a full navmesh route from behind its marker (the longest is 22 u, round a long counter), so none lands on another mesh island.

From the peek point:

- `Hallway01_Guard` sees the UAT-1 player at 8.1 u and 5.9 u. The 13.6 u spot is still blocked, from the peek point as well as from the guard.
- `Hallway02_Guard` does not see the spot where it killed the player.

**Holding fire.** An NPC at its slot with no line from there holds fire (`cover_no_shot`). After 3 s it gives the slot up (`cover_released_no_shot`) and fights as a mobile NPC out of cover. A new pick must have a shot from the slot's peek point (the pick log counts the rejects as `no_shot`).

**Flank churn (finding 3):**

- A held slot is released as flanked only once the threat is 20 degrees past side-on (normalised dot below -0.342; NA22 used 5 degrees), and a free slot is picked only with the threat in front of side-on (dot at least 0). All three UAT-1 releases fall inside that band.
- A slot given up as flanked, blind or unreachable cannot be picked again by the same NPC for 6 s.
- A flanked NPC with a clear line from where it stands fires in place; it only chases when it has no line.

**Known cost.** The mess hall's tables are navmesh holes too, so from its slot a mess-hall guard sees little of the room. In play it holds fire from cover for 3 s, then leaves the slot and closes in the way any mobile NPC does. The collision-geometry occluder (#784) is the real fix for furniture.

Tests: `crates/services/src/cell/service/tests/npc_ai_cover_peek.rs` (the UAT-1 positions on the real mesh and seed), plus unit tests in `cover/ai_integration_tests.rs` and `cover/scoring_tests.rs`. Each one fails with its fix reverted.
