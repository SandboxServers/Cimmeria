---
name: atrea-node-mapping
description: Atrea Script Editor node (Event_*/Act_*) to Cimmeria content-engine trigger/action mapping, including the nodes with no port today.
metadata:
  type: project
---

# Atrea node → Cimmeria chain mapping

Read the `.script` XML under `deprecated/data-scripts/scripts/` alongside the
compiled `.py`: the XML keeps the designer's section `<Comment>` blocks and the
node `Property` names, which the python loses. Behaviour authority is still the
python (per the Castle Cellblock precedence rule).

| Atrea node | Compiled python | Cimmeria equivalent | Status |
|---|---|---|---|
| `Event_MissionUpdate` (Mission Id) | `subscribe("mission.accepted::N")` | trigger `mission_accepted` | works, but only fires from a chain's own `accept_mission` |
| `Event_DialogChoice` | `subscribe("dialog.choice::N")` | trigger `dialog_choice` | ✅ |
| `Event_Item` (item id) | `subscribe("item.use::N")` | trigger `item_use` | ✅ |
| `Event_DialogSetMap` | `subscribe("dialog_set.open::N")` | trigger `dialog_set_open` | ❌ **never dispatched** |
| `Event_EntityInteract` (Tag) | `subscribe("entity.interact.tag::T")` | trigger `interact_tag` | ✅ |
| `Event_EntityInteract` (Template Name) | `subscribe("entity.interact.template::T")` | trigger `interact_template` | ✅ |
| `Event_GenericRegion` (Tag) | `subscribe("client_hinted_region::K")` | trigger `enter_region` / `exit_region` (key = `point_sets.name`) | ✅ |
| `Event_Dead` | — | trigger `entity_dead_tag` | ✅ (tag form only) |
| `Act_AdvanceMission` (Mission, Step) | `missions.advance(m, s)` | action `advance_step` | ✅ |
| `Act_UpdateMission` | `missions.complete(m)` | action `complete_mission` | ✅ |
| `Act_UpdateMissionObjective` | `missions.completeObjective(m,o)` | action `complete_objective` | ✅ |
| `Act_GetMissionObj` | `missions.getObjectiveStatus(m,o)` | condition `objective_status` | ✅ |
| `Act_GetMissionStep` | — | condition `step_status` | ✅ |
| `Act_GiveItems` (Design, qty) | `inventory.pickedUpItem(d,q)` | action `add_item` | ✅ |
| `Act_RemoveItems` (Design) | `inventory.removeItemByDesign(d,1,False)` | action `remove_item` | ✅ |
| `Act_AddDialog` (tpl, set, mission) | `player.addDialog(tpl, set, mission)` | action `add_dialog` (`target_id`=set, `params.entity_template`) | ✅ — note `mission_id` param is **discarded** by the executor |
| `Act_RemoveDialog` | `player.removeDialog(tpl, set)` | action `remove_dialog_set` | ✅ |
| `Act_Dialog` | `player.displayDialog(None, id)` | action `display_dialog` | ✅ |
| `Act_Teleport` (World Name, Destination) | `target.moveTo(x,y,z, worldName=W)` | action `cross_world_teleport` | ✅ |
| `Act_RingTransportDlg` (Region Id) | `space.transporters.get(N).interact()` | action `trigger_transporter` (`params.regionId`) | ✅ |
| `Counter_Int` (+ `Value == A`) | inline int counter | `increment_counter` + condition `counter` | ✅ — **counter condition reads the PRE-increment value; author `gte target-1`** |
| `Cmp_Str` / `Var_String` | `if a == b` | no equivalent — there is no string-compare condition | ❌ must be split into one chain per literal |
| `Act_GetProperty` (entity → tag) | `entity.tag` | no equivalent — triggers carry the tag directly | n/a, refactor to `interact_tag` |
| `Act_LaunchAbility` | — | action `launch_ability` | ❌ no executor arm on main (lands with `feat/content-effect-apply-entry-point`) |

Common shape: an Atrea graph that fans one event out to N `Cmp_Str` branches
becomes **N separate chains**, each with its own `interact_tag` trigger and its
own `objective_status eq active` gate. Conditions AND together; for OR you
author another chain.
