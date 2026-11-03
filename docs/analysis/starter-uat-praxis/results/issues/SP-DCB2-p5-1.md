# gmKillTarget kills a quest-tagged NPC without raising entity_dead_tag, so kill-count missions never advance (SP-DCB2-p5 CB-U06)

## Problem
`/gmkilltarget` (gmKillTarget, tier G) kills an NPC through `kill_npc_out_of_band` -> `resolve_death`, which does not call the content engine's `entity_death`. Only the combat kill-credit paths do (`cell-combat/.../use_ability/kill_credit.rs`, `effects/pulsing/pulse.rs`). So killing both Hallway05 guards (tags `Hallway05_Guard1/2`) with the GM command fires no `entity_dead_tag`, chains 1092/1093/1094 never run, and mission 686 stays active. This is the product question the CB-U06 brief asked to check once the kill is done correctly. It is also a gap in the lab and QA tooling: the tier-G kill cannot stand in for a real kill on any kill-count mission, and CB-U06 cannot be graded this way. The player cannot use the pistol in the lab (results/issues/SP-DCB2-p2-1.md).

## Evidence
Mission 686 was active (`sgw_mission` player 100: status=1, step 2353, active objectives [2730, 2731], completed [2731]; flank objective 2731 completed at 09:16:04Z by chain 1142).
- 09:16:32.09Z `gmKillTarget: killing NPC` 102671 (Hallway05_Guard2); 09:17:08.89Z the same for 102670 (Hallway05_Guard1). Both followed by `NPC death: respawn not scheduled`.
- No `fire_entity_death` row (matched or no-chains), no `Content: complete mission` 686, no `accepting mission` 687, no Matinee 1751, no dialog 2516, no blurb 5859 anywhere 09:16-09:19Z. Marsh was not despawned (his chain-1178 bark replayed after the relog).
- `crates/cell-console/src/cell/console/gm/world.rs` gmKillTarget passes `grant_xp: false`, `attacker_is_player: false`, and has no engine handle. `death/npc_only_kill.rs` states a kill by an NPC or GM carries no mission `EntityDeath`.
- Chain 1188 (mission_accepted with both guards `entity_tag_state` dead) did not help because 686 was already accepted.

## Acceptance criteria
- Either gmKillTarget credits the invoking GM player through the same `entity_death` hook as a combat kill (no XP), so tagged-NPC chains run; or the decision to keep GM kills credit-free is documented in the gmKillTarget help text and docs, and a separate lab-only way to raise `entity_dead_tag` for a tag is provided.
- A test fails when the fix is reverted: kill a tagged NPC through gmKillTarget with a kill-count chain loaded and assert the counter increments / mission completes.
- CB-U06 can be re-driven with `/gmkilltarget` on both guards and shows complete 686, Matinee 1751, dialog 2516 (+10.1 s), blurb 5859 (+26.1 s), 687 once.

## Test type
Cell-level unit/integration test (cell-console + content engine), plus a lab re-drive of CB-U06.

## Docs to update
`docs/guides/unified-uat.md` (CB-U06 step), the GM command reference for gmKillTarget, `docs/analysis/starter-uat-praxis/` ledger.

## Client impact
None. Server-authoritative; no client patch.

## Domain advisor
mission-systems-advisor, combat-systems-advisor.

## Needs a human for
The policy choice: should a GM kill count as a quest kill for the GM (the user wants lab tier-G setup to work), or stay credit-free (an admin command must not mint progress)? @Cadacious

## Implementation plan
1. In gmKillTarget, after `kill_npc_out_of_band` returns true, resolve the tag of the target and call the same `entity_death(credited=caller, ...)` the combat path uses, with no XP. Needs the content engine handle threaded into the GM handler.
2. Or add `gmFireDeadTag <tag>`.
3. Add the test and update the docs.
