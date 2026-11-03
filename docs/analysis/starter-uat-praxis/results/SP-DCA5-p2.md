PACKET SP-DCA5-p2 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2 (attempt 1 blocked at L2 on a stale Livewire window left by the previous part; coordinator logged out, attempt 2 ran); server evidence window 13:24:26Z-13:27:45Z (local lab server, run dr-lab, server.log UTC)

Substitutions (instance p2, not p3): Jaffa = Fxdcafour (player 107, 1521 at 4456) for Uat FxDcafjc; fresh character = Fxdcathree (player 106, Soldier, 701/703 completed, 567 not_active, staged at Copplemann) for Uat FxDcaffb. Looker skipped; screenshots by file name only.

| Row | Result | Tier | Server evidence | Client evidence | Notes |
|---|---|---|---|---|---|
| S8 | PASS | G (lab_logout + lab_ensure_in_world) | one logoff 13:24:35.98, one login 13:24:53.85 (world Castle, missions_restored). sgw_mission 1521 player 107: current_step 4456, status 1. No advance_step, no 4458 or 4991 in the window. | back in world 8 | pre-relog 4456 rests on the fixture plus no step-change event |
| FZ-U1a | PASS (server); dialog layout NEEDS_HUMAN | N1 click on Accept (driver reported posted_only, server shows it landed) | login 13:25:55.98: dialog set bound (chain 1395, Copplemann). 13:27:07.22 interact, dialog 2043 displayed. 13:27:21.44 dialogButtonChoice 2043, chain 1396 accepted 567. sgw_mission 106/567: status 1, step 2000, objective 2338. | DialogWin opened, Accept enabled (client-read) | SP-DCA5-p3/p4 not blocked |

FINDINGS:
1. 13:24:47.92 WARN archetype_has_no_tree for Jaffa (archetype 8) at relog world entry; same as the DCA5-p1 note; not filed (owner call).
2. 13:25:46.97 WARN REQUEST_ENTITY_UPDATE before player entity connected (no_player_entity), right after the character switch logoff; benign, outside the row windows.
No product bugs; no issues filed.

NOT RUN / BLOCKED: none. Harness gap: a stale MinigameWin left open makes lab_ensure_in_world fail ("player's name did not read for 15 s"); lab_logout clears it.
Brief defects: brief says INSTANCE p3 but the lane runs on p2; fixture names (Uat FxDcafjc, Uat FxDcaffb) replaced by existing Fx characters; S1/S8.4 positions read in client units, not server metres.

SIGN-OFF SHEET
FZ-offer | lab_screenshot-20261103-072711.838.png | dialog about bringing Romney's Files with Accept and close buttons | looker skipped | cannot tell
FZ-accepted | lab_screenshot-20261103-072721.993.png | taken after Accept click | looker skipped | cannot tell
S8-after-relog | lab_screenshot-20261103-072541.087.png | Fxdcafour back in world 8 | looker skipped | cannot tell
