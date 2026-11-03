PACKET SP-DCB3-p5 VERIFIED 2026-11-03 by Sonnet; driver attempts: 2 (attempt 1 targeted the wrong NPC, BLOCKED; attempt 2 with a coordinator target override); grade PARTIAL
Re-drive with repaired briefs (runner-DCB3-b, local lab, character Dcbdh). Grades are the verifier grades per part; the runner report does not itemise server timestamps, so rows without a time say so. Run before p4, the order that keeps 641 un-accepted.

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U12d precondition | PASS | n/a | 638 done, 641 not active | n/a | n/a | |
| CB-U12d catalog | PASS | n/a | catalog row for 62060 present | n/a | n/a | |
| CB-U12d no early offer | PASS | n/a | no `accepting mission` 62060 before 638 completes | n/a | n/a | |
| CB-U12d accept | PARTIAL (NATIVE_SHORTFALL) | N3 | chain 11033; `accepting mission` 62060; `displaying dialog` 60151 | Dialog 60151 shows the known client syntax error | H-CB-U12-1 (no screenshot) | Giver is an ambient prisoner, not Prisoner 329 |
| CB-U12d aid branch (Dcbdh) | PASS | N3 | chain 11036 granted item 2893 x2 once; 62060 complete | Item shows client-side as "Health Slappack TC1" (instance 10600) | n/a | |
| CB-U12d sabotage branch (Dcbdk) | NOT-RUN | n/a | n/a | n/a | n/a | |
| CB-U12d not both | PASS | n/a | no second reward row | n/a | n/a | |
| CB-U12d relog | PASS | n/a | no second reward, no offer after completion | n/a | n/a | |
| CB-U12d full bag | NOT-RUN | n/a | n/a | n/a | n/a | optional |
| CB-U12d skip | PARTIAL | n/a | 641 was never accepted in this part, so "stop offering after 641" is unproven here | n/a | n/a | Belongs to p4's Marsh accept |

Row grade: CB-U12d PARTIAL (aid branch PASS at N3).

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-CB-U12-1 | dialog 60151 render | no screenshot was taken.

FINDINGS: none new (the 60151 syntax error is the known text defect: issue SP-DCB3-p2-1).

NOT RUN / BLOCKED:
- Sabotage branch (Dcbdk) and full bag.
- Skip: needs 641 accepted (SP-DCB3-p4).

BRIEF DEFECTS (fixed in this change):
- S1 and .1 to .5 named Prisoner 329 as the 62060 giver. The real giver is the ambient prisoner (template 176, spawns 520/521, shown "Op-CORE Soldier"; the re-drive found entity 103285 at -271 65.45 -135). The brief now uses a GOTO plus FIND.
- 2893 shows client-side as "Health Slappack TC1" (instance 10600).
- The inventory must be reread after the relog; the brief now adds INV to .9.
- The aid click is a plain /interact on the same prisoner.
- Waves-table order: p5 now runs before p4 (the Marsh accept in p4 closes the 62060 offer).

RESUME HINT: Run the sabotage branch on a fresh character (Dcbdk); do not re-run the aid branch on Dcbdh.
