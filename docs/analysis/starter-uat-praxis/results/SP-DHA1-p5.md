PACKET SP-DHA1-p5 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 15:18:24-15:21:00 UTC (dr-lab logs\server.log; SigNoz not queried)
| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| H40a | PARTIAL | arrival: none needed; abandon: G (typed, never took effect) | Arrival accept PASS: 15:18:29Z "Content: accepting mission" 1200 chain 6121 for player 111, then "Mission accepted" step 3584; sgw_mission row (player 111, 1200) status 1, step 3584 at end. Abandon BLOCKED: 15:19:44Z gmMissionClear authorized, then WARN "DesignID is not a positive numeric mission id" with mission_arg empty; no "Mission abandoned" row. Re-entry: world 57 at 15:19:51Z "Restored missions from the saved journal" active 1200:3584; no chain 6121 fire and no second accept after 15:18:29Z. Mission was never abandoned, so re-entry did not test the re-accept policy. | H40-arrival = lab_screenshot-20261103-091854.442.png; H40-after-reenter = lab_screenshot-20261103-092038.096.png (client-read only, lookers skipped) | known: results\issues\SP-DHA1-p1-1.md (empty mission_arg) |

## Not run
- Chain 6124 (optional Ba'al objective, dialog 5436): dropped from this packet, NOT-RUN.
- Looker H40-arrival: skipped by instruction.
- Leg 2 abandon: BLOCKED by known SP-DHA1-p1-1 (empty mission_arg on /gmmissionclear). Re-accept-on-re-entry outcome unobserved (mission stayed active; the saved journal restored it).

## Resume hint
START LEG 2, fixture: world 57 with mission 1200 active at step 3584 (current state; accept already done). Abandon needs a working path (SP-DHA1-p1-1 fix or another G route such as the console abandon), then re-enter world 57 and read whether chain 6121 fires again. Leg 1 need not be redone. Note the accept happened at 15:18:29Z, before the h40 start bookmark (15:18:50Z), during ensure_in_world; the accept notice is a chat line and the driver did not see it (harness gap, graded on server rows).

## Recipe
- lab_ensure_in_world as Fxdhagoauld (Harset, world 57) triggers chain 6121 accept.
- .bug "SPU SP-DHA1-p5 h40 start"; CROSS to Harset_CmdCenter (world 68); .bug h40 abandon; /gmmissionclear 1200 (arrives empty); CROSS back to world 57; .bug h40 a end.

## SIGN-OFF SHEET
None (no NEEDS_HUMAN items; lookers skipped).

## Findings
1. Row H40a: the only WARN in the window is the expected-by-known-issue gmMissionClear rejection (mission_arg ""), existing issue SP-DHA1-p1-1. No new issue.
2. Harness gaps (not defects): placeholder lease id on first ensure_in_world; accept notice never visible in channel 9.
