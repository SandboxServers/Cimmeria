---
name: reference-lab-client-controls
description: "Driving the SGW client through the lab: keys, camera limits, GM teleport gotchas (2026-10-03)"
metadata:
  type: reference
---

Learned in the 2026-10-03 auto-attack UAT on colo:

- Keys: Q/E rotate the character, A/D strafe, W/S move, B opens the bag
  (not I), Tab targets the next enemy without firing, R reloads.
- Lab mouse-look does not move the camera: `client_camera` yaw and
  `client_world_click` auto-turn report `samples: 0`. Button presses go as
  window messages while motion goes through DirectInput, so the game never
  sees a held button. Camera follows the player after a GM teleport, so
  `.gotoxyz` the player until the entity projects on screen.
- `.gotoxyz` moves the SELECTED target if one is selected, not you. Use
  `.movehere` to bring a selected NPC to you.
- `client_inventory` / `client_item_action` read no containers on this
  build. Fallback: `InventoryMod.SlotWindows[i]:getName()` (Inventory_SlotN,
  1-based) plus `client_ui_click {button: 1}` with the bag open.
- Unnamed corpses (Cellblock guard) fail the hover check; click them with
  `client_world_click {point}` instead of `entity_id`.
- Packet tap decoder labels state_field 0x2 as `auto_cycling: false`;
  BSF_AUTO_CYCLING is 1 << 1, so the label is wrong.
- `server_log_tail` keeps only 500 lines (mostly DEBUG); use the packet tap
  for evidence when SigNoz is down.
