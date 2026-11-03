PACKET SP-DCB3-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1 (19 of 28 tool calls, no harness block); old Dcbdh deleted and recreated so the part started fresh
Re-drive with repaired briefs (runner-DCB3-b, local lab, character Dcbdh). Grades are the verifier grades per part; the runner report does not itemise server timestamps, so rows without a time say so.

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T01/T02 dialog 2982 | PASS (+ NEEDS_HUMAN 1) | N1 boot click, G .bug | `displaying dialog` 2982 once at world entry | Driver: dialog window read empty | see sign-off sheet | The batch DIALOG op had already closed the dialog (brief defect, fixed): the render is a human check, H-T01-1 |
| T01/T02 mission 622 | PASS | G | `accepting mission` 622; mission table step 2113 | Tracker "Arm Yourself!" | n/a | |
| T01/T02 mission 638 | PASS | G | `accepting mission` 638 once, step 2114, on the Region2 entry (GOTO -265 65.45 -135) | n/a | n/a | |
| T01/T02 Frost searchable, Guard not | PARTIAL (UNVERIFIED) | G | Neither Frost nor Guard found by `client_entity_find` within 8 m (they are elsewhere); entity flags read 0 on both entities | n/a | n/a | A verifier read gap, not a contradiction. Brief defect: read the searchable state from the seeds, not from live flags (verify file updated) |
| Side: boot mission 689 | PASS | N1 | boot Livewire won, mission 689 complete | Chat "The prison boot unlocks" | n/a | Not part of the row grade |

Row grade: T01/T02 PARTIAL (+ NEEDS_HUMAN H-T01-1, H-T02-1). Server clauses held; the Frost and Guard searchable clause is unproven.

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-T01-1 | T01 dialog 2982 render | %LOCALAPPDATA%\cimmeria-lab\screenshots\lab_screenshot-20261103-050135.277.png
- H-T02-1 | journal shows mission 622 | %LOCALAPPDATA%\cimmeria-lab\screenshots\lab_screenshot-20261103-050141.389.png
- Minigame images (not graded) from p1, p2 and p3 are in the same folder.

FINDINGS:
(none filed)

NOT RUN / BLOCKED:
- Frost and Guard searchable clause: not proven (see the row). Read it from the seeds: spawns 19 and 15, templates 14 and 21.

BRIEF DEFECTS (fixed in the brief and verify files in this change):
- .1: dialog 2982 was already closed by the batch DIALOG op (46 s after display), so the DLG read is always empty. The brief now reads and screenshots it right after WAIT, before S2.
- .8: Frost and Guard are not found by `client_entity_find` within 8 m. The step is dropped and the verify file reads the searchable state from the seeds.

RESUME HINT: Redo only the Frost and Guard seed read. Mission 622, 638 and dialog 2982 are server-confirmed.
