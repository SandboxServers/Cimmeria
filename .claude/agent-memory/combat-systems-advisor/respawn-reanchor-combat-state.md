---
name: respawn-reanchor-combat-state
description: Same-world respawn reanchor (CREATE_BASE_PLAYER) wipes client combat caches; stats were pushed BEFORE the reanchor and never after (FIXED, PR __PR__); auto-attack arrives as interact (idx 74), not useAbility (68)
metadata:
  type: project
---

Diagnosed 2026-09-28 (colo build 5f9730c6, "abilities die after respawn", player 71).
**Fixed in PR __PR__** (same PR also fixed NPCs respawning in the death pose).

- Auto-attack (559 SMG, 579 pistol) reaches the server as cell method 74 `interact`
  (9 bytes, `00000000 0d <target u32>`), and the interact handler turns it into
  `handle_use_ability`. It never passes through the client hotbar `useAbility` (68) path, so
  "auto-attack still works" says nothing about the hotbar.
- Hotbar `useAbility` (idx 68, 13 bytes) is rare on the wire: 16 events colo-wide in 7 days.
  A client-side gate drops the send without leaving a server log, so SigNoz can only show that
  nothing arrived. `docs/reverse-engineering/findings/client-wire-emit-suppression.md` lists the
  known gates.
- Was: `cell-interactions/src/cell/respawn/mod.rs` sent `onStatUpdate` (HEALTH/FOCUS dirty only)
  and `onStateFieldUpdate` BEFORE `ReanchorPlayer`, so they reached the entity the client was
  about to destroy, and `resync_after_pawn_recreate` never replayed 20/21/23/141 or level.
  Now: nothing stat/state goes before the reanchor; the resync replays level (15), state field
  (19, always), full stats (20), base stats (21), archetype (23), ability tree (141), then
  101/70/missions. Its log lists them in `replayed`.
- Was: the reanchor log field `not_resent="mission_log,abilities,stats"` was hard-coded and stale.
  Now `resent_by_cell` lists what the cell replays.
- Was: the reanchor (and gate travel) always sent class 0x02, demoting a GM (who logs in as 0x03)
  on every respawn. Now both reuse `ConnectedClientState::player_class_id`, cached in
  `play_character`. The "0x03 shifts indices" claim was disproved in play_character.rs.
- If a future report says abilities die after a respawn again, check that the resync still runs
  after `ReanchorPlayer` on the same channel, and that `replayed` shows up in SigNoz.
