# Starter UAT (Praxis): a reliable route for in-world interaction

Tested live on 2026-11-03 against the local lab server and the default lab client. The character was a Cellblock Praxis starter (player entity 46), carrying missions 638 and 639. The time box was about 40 minutes.

## Result

| Target | Native click (`client_world_click`) | Recipe below | Server evidence |
|---|---|---|---|
| p7 Door Control panel `329_CellDoorButton` (102318) | fails at `observe`, `mouse_over_after 0` | **works** | tap: `setTargetID 102318`, then `interact` (msg 189). Livewire opened, `client_minigame_play` won it, mission 638 advanced (2114, 2115 done; 2116 current) |
| p7 Prisoner 329 (102307) | never hovers (camera sits in wall geometry) | **works** | tap: `setTargetID 102307`, `interact`, `onDialogDisplay 2300`. `lab_finish_dialog {accept:true}` closed out 638 (status 2) and accepted 639 (step 2117) |
| p8 drone (102310) | `client_target` failed at `on_screen` (pitch stuck at -78.75°) | targets after a camera reset | `setTargetID 102310`. `/gmkilltarget` did **not** kill it (still 250 hp); not pursued |
| p8 Ambernol vial `ArmYourself_AmbernolVial` (102312) | fails at `observe`, `mouse_over_after 0` | **does not work** | the `targetUnit` fallback sent `setTargetID 0`. `/interact 102312` sent `interact` with target 0. A point-aimed right click at (512,508) sent nothing. Step 2145 is still open |

Step 2117 → 2145 of mission 639 ("go to the drone") completed by itself. It is a region trigger (`triggerClientHintedGenericRegion`) and fired on the teleport to the drone spot; no click was needed.

## Why native clicks fail

1. **The avatar is in the way.** "Face the target" centres it on screen, which puts it straight behind the avatar. At the door, the dz=0 hover hit `own_avatar`. At the vial, `client_hover_probe` reported `avatar_px 32`.
2. **Level geometry hides the target.** At the cell the camera ended up inside dark wall geometry. Neither the Prisoner nor the door panel answered a hover at any offset (dz 100/140/170/60/0).
3. **Camera pitch gets stuck.** A failed `client_target` can leave pitch at the -78.75° clamp, and every later projection is then off screen ("could not turn the camera onto the target"). Reset it with `client_camera {pitch_deg:-10, face_entity_id:<id>}`.
4. **`client_hover_probe` blames windows that cannot block a click** (a tool bug). Its window-rect check ignores mouse-pass-through and disabled windows. Two such windows are always visible in Cellblock: `SplashHackWin` [154,77,871,307] (pass-through, disabled) and `AttackDirectionQuartersWin` [307,230,717,537] (pass-through). The probe reports `{"ui":"SplashHackWin"}` or `{"ui":"AttackDirectionQuartersWin"}` even though the game sends the click through them. `client_world_click` does not use this check; only the hover probe does. Do not read these verdicts as covering windows (#1676/#1679/#1680 are a different case).
5. **Pop-up dialogs.** A mission-advance dialog ("Secure some Ambernol…", "Retrieve the vial…") can appear over the screen centre right after a step completes. Close it first (the DIALOG op below).

## Recipe for units: NPCs and clickable panels (proven)

`<L>` is the lease id. The client must be in the world.

1. Stand 3–4.5 m from the target (interact range is 5 m). Use `server_console_exec` as the player entity:
   `.gotolocation Castle_Cellblock <x> <y> <z>` (door panel: `-285.3 65.5 -112.2`).
2. Close any open dialog with `client_batch` op `lua`:
   `if DialogWin ~= nil and DialogWin:isVisible() then DialogMod.onDialogDoneClicked(DialogWin, DialogWin) return 'closed' end return 'none'`
3. If an earlier aim failed, or the target projects off screen, reset the camera:
   `client_camera {"lease_id":"<L>","pitch_deg":-10,"face_entity_id":<id>}`
4. Set the target without hovering (the stock `targetUnit`, tier N3):
   `client_target {"lease_id":"<L>","entity_id":<id>,"allow_fallback":true,"rotate_camera":false}`
   Check that the tap shows `setTargetID <id>`, or `client_player_state` shows the target.
5. Send `/interact`. It goes to the current target; cell method 74, `overrideTarget` 0:
   ```
   client_batch {"lease_id":"<L>","stop_on_error":false,"steps":[
     {"op":"lua","id":"c1","chunk":"ChatMod.stickyCommand='/say '; Inst1Chat_Input:setText('/interact'); ChatMod.onTextAccepted(Inst1Chat_Input, Inst1Chat_Input); return 'sent'"},
     {"op":"wait","ms":2500},
     {"op":"lua","id":"w","chunk":"return tostring(DialogWin and DialogWin:isVisible())"}]}
   ```
6. Continue on what step 5 opens: `lab_finish_dialog {"lease_id":"<L>","accept":true}` for a dialog, or `client_minigame_play {"lease_id":"<L>","entity_id":<player id>}` for the Livewire. `client_minigame_play` may report `inconclusive server_unreachable` even when the game was won. Check `server_minigame_state` or the tap rows (`smartfox_victory`), and the mission row in `public.sgw_mission`.

To check the result, run `server_packet_tap_start {entity_id:<player>, capacity:2000}` before step 4 and `server_packet_tap_read` afterwards. `server_log_tail` is useless here: its 500-line buffer is all DEBUG noise.

## Small props such as the Ambernol vial (not solved)

The vial (template 18, `interaction_type_flags` 0x40000000 after step 2145 opened) has `targetable:true` in `client_entity_find`, but:

- the `targetUnit` fallback clears the target (`setTargetID 0`);
- `/interact <id>` ignores its argument: the client still sends `overrideTarget` 0;
- native hover at dz 0–170 reads `mouse_over 0`; it projects about 32 px from the avatar.

Not yet tried:

- **Move the avatar sideways** so the vial projects at least 150 px off the avatar and outside screen centre. For example, stand at (-234, 66.5, -121) and face north. Then `client_hover_probe {point:<vial>, expect_entity:102312}`. Ignore a `ui` verdict that names one of the pass-through windows above; only `mouse_over == 102312` counts. Then `client_world_click {entity_id:102312, button:"right", rotate_camera:false}`.
- **Use the first-session.toml pose** (pitch -28.8°, yaw +25° after facing, zoom 250) with a point aim.
- **Use a server-side GM route** for the pickup, if one exists, to unblock the UAT lane. Record that as a deviation, not a pass.

## Drone (p8)

- `client_target` with `allow_fallback` works after the step-3 camera reset.
- `/gmkilltarget` did not kill drone 102310 (still 250 hp). The p8 brief's TCLICK, then `/gmkilltarget`, needs a separate look.
