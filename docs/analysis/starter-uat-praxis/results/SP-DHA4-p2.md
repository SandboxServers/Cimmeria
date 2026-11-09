# SP-DHA4-p2 result (FZ-U2h part h1)

PACKET SP-DHA4-p2 VERIFIED 2026-11-09 by Sonnet; driver attempts: 1; server evidence window 06:39:11Z-06:41:17Z (bookmarks `SPU SP-DHA4-p2 FZ-U2h1` start 06:39:16Z, end 06:40:57Z). Evidence source: the dr-lab `server.log` (Source 2 of verify-server-evidence.md). Looker skipped by instruction; screenshots are file names only.

| Row | Result | Tier | Server evidence (time, event or query, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| h1 before (Marsh click on 5226) | PASS (look-and-feel NEEDS_HUMAN `H1-BEFORE`) | N1 attempted (talk result inconclusive); server saw a real chain_tag interaction | 06:39:43Z `Content: npc bark` screen 200180, chain 6018; `complete objective` 1664/6061 by chain 6018. No chain 6512, no `displaying dialog` 4457, no chain 6511 before 06:40:12Z. Player 106. | Chat line "Report received. Good work out there." | skipped | Server clauses PASS. "?" cue not gradable. |
| h1 after (1664 completed, Marsh click) | PASS (look-and-feel NEEDS_HUMAN `H1-AFTER`, `H1-DIALOG`) | N1 (`real_input`, `DialogWin` opened) | 06:40:12Z chain 6019 completes objective 6062, `Mission completed!` 1664 (path auto); `fire_mission_completed: matched`; chain 6511 `adding dialog set` 5254 / dialog 4456 on Marsh in the same ms window. 06:40:42Z and 06:40:49Z chain 6512 `displaying dialog` 4457 (npc Marsh 100199). DB: 1664 status 2, completed objectives 6059-6063. | DialogWin opened; closed with Done | skipped | The first click reported a driver-side timeout but the server did show 4457 at 06:40:42Z (a second display at 06:40:49Z from the retry). |
| 1361 never accepted | PASS | n/a | `sgw_mission` for player 106, mission 1361: no row. No `mission.accept` or 1361 mission lines in the window. | Driver: no 1361 in gmMissionList | skipped | Accept never pressed (dialog closed with Done). |

FINDINGS:
1. No WARN or ERROR lines in the window (excluding transport noise).
2. Minor, not filed: after completion the 1664 row still lists active_objective_ids [6061,6062] (status 2, current_step_id null). Likely a stale list on completed missions; no row clause depends on it.
3. Driver's first Marsh click on the 4457 step returned an error (no Window in 2500 ms) although the server had already displayed 4457; a lab timing issue, not a product bug.

NOT RUN / BLOCKED: none.

RESUME HINT: none for this part. -p3 and -p4 are graded separately; row FZ-U2h needs all three.

SIGN-OFF SHEET:
- H1-BEFORE | H1-BEFORE | lab_screenshot-20261109-003934.978.png | a "?" cue over Marsh while 1664 is on 5226 (the offer must not show yet) | skipped
- H1-AFTER | H1-AFTER | lab_screenshot-20261109-004037.305.png | a "?" marker over Marsh after 1664 completed | skipped
- H1-DIALOG | H1-DIALOG | lab_screenshot-20261109-004052.517.png | Praxis briefing dialog 4457 open, Accept button present | skipped
