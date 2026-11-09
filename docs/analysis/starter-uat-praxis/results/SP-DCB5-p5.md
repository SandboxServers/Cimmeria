# SP-DCB5-p5 (T15.1) - runner-DCA-y, 2026-11-09, local lab p2

| Row | Result | Tier | Server evidence | Notes |
|---|---|---|---|---|
| T15.1 | BLOCKED-known | G (kills), N1/N3 (targets) | `gmKillTarget: killing NPC` for 101471 (Hallway01_Guard, 10:07:11Z) and 101479 (Hallway02_Guard, 10:08:06Z); entity 101479 ai_state Dead, health 0. No `Content:` rows in the window; sgw_mission for Fxdlone (player 108) still shows 682 status 1 step 2349; chain 1088 (trigger `entity_dead_tag` Hallway01_Guard) never fired | Same cause as results/issues/SP-DCB2-p5-1.md: GM kill raises no entity_dead_tag. The 681..686 chain clauses cannot be graded this way; no issue filed (duplicate). |

Fixture: Fxdlone staged world Castle_CellBlock (not Castle_Cellblock; stage refuses the latter), position [-128,39.5,-70], missions 681 completed, 682 step 2349. Driver A (Haiku) did bookmark and entity find; coordinator did client_target (Hallway01: N3 targetUnit fallback after native hover hit own_avatar; Hallway02: N1 after moving 4 m off and camera reset) and sent the kill lines (tier G).
Brief defect: `/gmkilltarget` without an id returns "Not enough parameters." (client slash command needs `<entityId>`; server uses the selected target). Briefs for p4-p7 use the bare form; add the id.
NEEDS_HUMAN H-T15-1 (no toast/popup after kills): lab_screenshot-20261109-040811.641.png
