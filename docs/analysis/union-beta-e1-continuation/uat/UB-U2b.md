# UB-U2b: the speaker-0 dialog header

| Field | Value |
|---|---|
| Row | UB-U2b: read the header of a dialog whose speaker is 0 "Not Set". A blank header is cosmetic: record it, it is not a FAIL |
| Runnable | Yes, graded from other runs (no run of its own) |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` (4663), `Fxbetafjdefa` (5042, 5043) |
| Fixture | none of its own |
| Budget | 0 calls (rides on the runs below) |
| Piggyback | rides on [UB-U4](UB-U4.md) run B step 6 and [UB-U6](UB-U6.md) steps 7 and 8 |

**Why these dialogs.** The row text names 5040 and 5047 on an "Asgard fixture". Neither can be shown: both are Free Jaffa dialogs, 5040 is bind-only (no chain displays it) and 5047 belongs to mission 1503, which is not built (D-UB5 A). The speaker-0 dialogs the server does show on Beta are 4663 (the ruin note, 1427), 5042 and 5043 (the Jaffa box, 1502), so this file grades those. 5040 and 5047 stay `not-run` for good, and the row text needs a fix at the close-out ([README § Notes for the close-out](README.md#notes-for-the-close-out)).

## Where the evidence comes from

| Dialog | Run and step | Chain | Screenshot |
|---|---|---|---|
| 4663 | UB-U4 run B step 6 (the RuinNote at S4248) | 23193 | `u2b-4663` |
| 5042 | UB-U6 step 7 (the Jaffa box at S4377) | 23244 or 23245 | `u2b-5042` |
| 5043 | UB-U6 step 8 (the second screen after button 575) | 23246 | `u2b-5043` |

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Shown | Each dialog opens and its text shows | the run's DLGREAD, the screenshot, `Content: displaying dialog` with that id |
| B. Header recorded | The header line is recorded exactly for each dialog: blank, "Not Set" or a name | the DLGREAD header, the screenshot |

The row passes on A for all three. B is the finding: a blank header is the expected cosmetic case. A header that shows a name is recorded as is (also not a FAIL). If a dialog never opened, the row is `not-run` for that dialog, not FAIL, and the arc run's FAIL carries it.

## Orchestrator only: verify list

- `Content: displaying dialog` 4663, 5042 and 5043 for the right character, at the UTC of the run's step.
- Look at each screenshot yourself: the header area of the dialog window, blank or not.
- Write 5040 and 5047 as `not-run (never displayed: bind-only / 1503 not built)` in the results.
