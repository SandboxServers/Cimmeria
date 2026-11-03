## Problem

The lab harness cannot interact with small world props (Ambernol vial `ArmYourself_AmbernolVial` 102998 / 102312 in other runs, rings switch `HackTheRings_Switch` 103005) through `client_world_click` / `client_target`. Props in interact range (3.2 to 3.9 m) never hover (`mouse_over_after 0`), `client_target` allow_fallback sends `setTargetID 0`, and `/interact` sends `overrideTarget` 0. Starter-UAT rows CB-U10b.8, .9, .12 and .13 cannot run, and the Cellblock 639 -> 640 chain cannot be driven natively. Class: harness gap (case c), not a product bug. @Cadacious

## Evidence

- SP-DCB3-p3 attempts 1 and 2 (2026-11-03, 10:29-10:38Z): vial and switch in range, `mouse_over_after 0`, `target_after 0`; hover probe `{"ui":"AttackDirectionQuartersWin"}`, 0 of 8 jitter hits, `avatar_px 9` after moving aside and facing the vial.
- Server (SigNoz, lab_run dr-lab) saw only `interact: target entity not found` with `target_entity_id 0` at 10:36:43Z; no `interact_tag`.
- Same class: docs/analysis/starter-uat-praxis/verify-click-recipe.md ("Small props ... not solved"), SP-DCB1-p8, SP-DCB2-p2-1.
- Reachable server state: mission 639 step 2145 persisted; chain 1032 (`interact_tag ArmYourself_AmbernolVial`) would grant item 19 and advance 639; 640 ring chains 1041-1044.

## Acceptance criteria

- A harness route (tool or documented recipe) makes a right-click or an equivalent `interact` land on a named small prop, with the tap showing `setTargetID <id>` then `interact`.
- Chain 1032 fires on the live client in a lab run, with `granting item` / `advancing step` in the server log.

## Test type

Lab UAT (live client); unit test for the tool's target selection if the fix is in the lab tool.

## Docs to update

docs/analysis/starter-uat-praxis/verify-click-recipe.md; docs/guides/automated-uat.md if a new tool arg is added.

## Client impact

None expected (harness side). A server GM `interact <entity>` hook would be a maintainer decision.

## Domain advisor

mission-systems-advisor (chains 1032-1044); lab tooling owner.

## Needs a human for

Deciding between a harness fix and a GM command/fixture deviation.

## Implementation plan

1. Try point-aimed click at the projected prop pixel with the pass-through windows ignored, and the first-session.toml pose.
2. If still failing, add a lab tool op that sends `interact` with an explicit entity (or a lab-only GM `.interact <entity_id>`), flagged as a deviation tier.
3. Fix the brief: item 19 (not 2893), drop loot_all, order rings after vial/640.
