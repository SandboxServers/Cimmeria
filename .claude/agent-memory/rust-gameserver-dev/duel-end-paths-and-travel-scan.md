---
name: duel-end-paths-and-travel-scan
description: SS-D3 duel end seams — every TeleportPlayer/GateTravel site must call duel::on_travel (scan test); the 1 HP clamp ends the duel only after the whole damage resolution
metadata:
  type: project
---

Since SS-D3 (2026-09-27, branch `social/d3-duel-end-paths`) every engaged-duel end goes through `cell-world` `duel::end_engaged(tx, mgr, duel_id, EndReason)`; `EndReason::Defeated { loser, reason }` is a decided end (879 to the winner).

**Why:** a path that removes a duel from the registry itself leaves both PvP flags and the combat pair set; a lethal duel would route duel kills through loot/XP.

**How to apply:**

- A new cell path that sends `CellToBaseMsg::TeleportPlayer` or `GateTravel` must call `duel::on_travel(tx, mgr, eid)` before the send, or `every_travel_site_ends_the_duel` (cell-world) fails. It sits beside the pets PT-02 scan ([[pet-owner-lifecycle-hooks]]); a new site needs both hooks.
- A new HEALTH-writing damage seam must call `duel::clamp_partner_lethal` before its stat flush and before any death check, and `duel::finish_clamped` only after everything else in the same resolution (script bleeds, DoT registration): ending first strips effects and ends `can_harm` before the rest is clamped.
- `effect_pulse_tick` fires from a pre-await snapshot; it now skips instances no longer on the entity (`still_active`). Keep that check if the loop is refactored.
- A DoT still never kills a player (`dot_kill_credit` returns for players); the duel tick's dead-duelist check covers duelists.
