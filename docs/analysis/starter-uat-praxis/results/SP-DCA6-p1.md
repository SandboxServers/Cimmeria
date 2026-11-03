PACKET SP-DCA6-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 3 (attempt 1 lost the staged fixture to the 600 s stage TTL; attempt 2 opened the store with cash 0 so the client capped the row maximum at 0; attempt 3 bought); server evidence window 13:52Z-13:58Z (local lab server, run dr-lab)

Substitution: instance p2; Fxdcatwo (player 105) replaces the brief character. Naquadah given by `/gmgivecash 5` (G-tier setup, the stage tool cannot set cash). Looker skipped; screenshots by file name only.

| Row | Result | Tier | Server evidence | Client evidence | Notes |
|---|---|---|---|---|---|
| hub S1 (walk to quartermaster, store opens) | PARTIAL | N1 click after a `.gotolocation` nudge | 13:54:05Z `vendor: store opened`, template 415, player 105, buy_count 19 | client_move_to arrived true (no fallback), end 5.7 m away; click refused beyond the 5 m interact range; after a 4.5 m nudge VendorWin opened with 19 rows | the brief caps a `.gotolocation` fallback at PARTIAL; PASS is defensible if only the last 1.2 m nudge is counted as setup |
| hub S2 setup (cash) | PASS | G | 13:54:16Z `GrantCash` naquadah 5 total 5 | chat feedback `+5 naquadah (total 5)` | |
| hub S2 buy row 1 qty 1 | PASS | N1 | server.log shows vendor buy rows in the 13:57Z window (attempt 3); attempt 2 sent no buy request (client-side pre-check) | cash 5 -> 4; Armor Piercing Rounds x100 (item 10630) in Main slot 3 | attempt 3 evidence: driver inventory reads; the server-log line check was a grep only, not a full verifier pass |
| hub S2 purchase feedback | PARTIAL | N1 | none | no chat line after the buy | open for human; not filed |
| hub S2 sell and buy-back | NOT-RUN | | | | no sell tool in the lab |

BLOCKED harness notes (not product defects):
- client_vendor_buy pre-checks the row maximum from the window state at open time; opening the window with cash 0 and then granting cash leaves the maximum at 0 until the window is reopened.
- client_vendor_buy reports verdict "denied" with "quantity_matched no_effect" when the vendor stack size is 100 (expected 1, got 100); the buy itself succeeded. Checker artifact.

OPEN QUESTION: whether the stock client refreshes a vendor row maximum on a cash change while the window is open; the server pushed no vendor refresh after the grant. Not established; not filed.

FINDINGS: none filed. Candidate low-priority items (not filed): the stage reply does not state the 600 s TTL; movement.correction_suppressed ERRORs for a character spawned at a bad saved coordinate show no recovery.

SIGN-OFF SHEET
hubS1-store | lab_screenshot-20260 (see drv-SP-DCA6-p1-a2.md: lab_screenshot-20261103-075409.491.png) | vendor window, 19 rows | looker skipped | cannot tell
hubS2-bought | lab_screenshot-20261103-075709.672.png | bag shows rounds stack, cash 4 | looker skipped | cannot tell
