---
name: reference_lab_harness_findings_2026_10_12
description: Live-client facts the lab-harness campaign (#1660) proved on 2026-10-12 - Stat names, zero camera pose, turn keys vs mouse-look, click-ring host, bag over vault, feet aim, 5 m 3D interact range, Lua rebuilt on relog
metadata:
  type: reference
---

Lab-harness campaign (#1660, ledger `docs/analysis/lab-harness/`, close-out `closeout.md`), probed and verified live on the colo, 2026-10-12. Tools already handle each of these; the facts matter when you write a Lua probe or a brief by hand.

- **Stat names:** `Stat` is a tolua class, so `pairs(Stat)` yields 13 metamethods, not stats. The 34 names are the keys of `Stat['.get']`. `Stat.Intelligence` exists (5); `Stat.Intellect` is nil. `getUnitStat(Unit.Player, Stat.Health)` returns a table (`current`, `max`, ...).
- **Camera actor pose reads all zero** on this client build (default and p3 alike). Use the view readout (`view_before` / `view_after`: `pitch_offset_deg`, `yaw_offset_deg`, gains); `client_camera` omits `camera_before`/`camera_after` when they are zero.
- **Mouse-look turns only the camera.** The pawn's heading answers only to the bound TurnLeft / TurnRight keys (Q / E; slot 2 Delete / PageDown; `getBindingKey(action, 1|2)`), about 110 deg/s; `W` walks along the pawn heading, not the camera's. `client_move_to` steers with the keys since LH-19.
- **Click ring host:** `subscribeGlobal` on `Root` raises "Window 'Root' is not a child of a ScriptedWindow"; subscribe on a window under a ScriptedWindow (`BackgroundWin` works). The handler is bound by name at subscribe time.
- **The bag window opens exactly over the vault** (`InventoryWin` [445,210,834,510] over `VaultWin` [445,210,834,498]), so a press on a vault slot lands on the bag's slot. Raise the source window first (`moveToFront`; `client_drag_drop` does it since LH-20).
- **Actor location is the entity's feet:** projecting it hovers nothing. Aim about 100 UE units (1 m) up.
- **Server interact range is 5.0 m in 3D** (`MAX_INTERACT_DISTANCE`), logged only as INFO `interact: too far away`; `client_entity_find`'s `distance_m` is horizontal. Stand 3.5 to 4.5 m off.
- **Lua state is rebuilt on relog:** every Lua wrapper (chat, click ring) is gone after `lab_logout` + `lab_ensure_in_world`; the event keeper re-arms them.

Related: [[reference_lab_client_controls]], [[reference_lab_parallel_clients_2026_10_10]].
