# SP-DCB2-p2-1: Guard body (template 21) cannot be targeted or interacted with by the native client

Rows: CB-U08b.15 to .19. Attempt 4 takeover, local lab, character Dcbch (server entity 52, player_id 100).

## Repro
1. Dcbch at (-324.80, 73.47, -212.25), Castle_Cellblock; mission 622 current_step_id 80623 (Frost already searched; item 3730 in Mission inventory).
2. Guard body entity 102607 (spawn 15, tag ArmYourself_GuardBody, template 21), 3.33 m away, witnessed by Dcbch (server_witnesses: witnessed_by 52).
3. client_world_click left/right, client_target allow_fallback, and the camera + /interact recipe: no setTargetID/interact reaches the server (packet tap: only avatarUpdateExplicit/perfStats). target_after stays 0.
4. Same calls on Cpl. Frost (102610) work at once (target_after 102610, tap shows setTargetID 102610).

## Evidence
- Server state is fine: mission 622 step 80623 active; chain 1005 gate (step_status 80623 active, archetype Commando=2) is met; chain 1007 (player_loaded) re-binds dialog set 5230 -> dialog 3996 (dialog_set_maps id 5230, set 803, flag 1073741824 = 0x40000000).
- resources.entity_templates: template 14 (Frost) class "being", faction 1, level 5, name_id 7031. Template 21 (guard corpse) class "spawnable", faction/level/alignment/name_id all NULL.
- client_entity_find for 102607: no name, level -1, hostility "3", mob_id 0. Frost: name "Corporal Frost", level 5.
- Native hover: client_hover_probe entity 102607 gets mouse_over = 102607 at dz 0 (pixel 610,515), but a left click yields target_after 0 (a "spawnable" has no Unit to target). Right click: mouse_over_after 0 or the guard, but no interact message is sent. `/interact` acts on the current target only (here Frost), so the guard cannot be reached.
- server_entity_get/query on 102607: interaction_type_flags 0, witness_count 0 (the guard has no witnesses of its own; it is witnessed by the player).
- Inventory now: Main = "Accessory" 10579 only; Mission = Frost's Letter 10580. No pistol 55. sgw_player_ability_grants: 0 rows for player 100.

## Classification
Client-side targeting limit of a non-Unit (spawnable) entity, with the product choice that the Guard body is a spawnable prop: left click and targetUnit cannot select it, and the right-click prop path does not fire in the lab input route. Not a server data problem (chain/dialog/step conditions are met). The proven recipe works only for Unit-class entities (Frost, Prisoner 329, door panel via targetUnit when the prop is a Unit), not for spawnable props (same as the Ambernol vial in verify-click-recipe.md).

## Suggested next step (not done here)
Either make template 21 a targetable class like template 14 ("being", with name_id) so the recipe works, or give the lab a server-side route (server_stage_fixture / grant) for the pistol and abilities, recorded as a deviation. Do not grant by GM command from the driver.
