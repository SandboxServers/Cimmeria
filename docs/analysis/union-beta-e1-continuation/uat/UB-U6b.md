# UB-U6b: type-4 dialog buttons reach the server

| Field | Value |
|---|---|
| Row | UB-U6b: at the steps that show buttons 247, 456, 457, 458, 575 and 576, press each. Each press reaches the server as a `dialog_choice` and the keyed chain fires on the first press |
| Runnable | Yes. Graded from the arc runs first; a one-button fallback run only for a button its arc run did not press |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` (247, 456, 457, 458), `Fxbetafjdefa` (575, 576) |
| Fixture | none for the graded path; F-B247 ... F-B576 for a fallback ([fixtures.md § Fallback fixtures](fixtures.md#fallback-fixtures-for-ub-u6b)) |
| Budget | 0 calls on the graded path; 15 calls per fallback run |
| Piggyback | rides on [UB-U3](UB-U3.md) run C step 8, [UB-U4](UB-U4.md) run A steps 3 and 4 and run B step 2, [UB-U6](UB-U6.md) steps 8 and 9 |

**From the seeds** (`dialog_screen_buttons.sql`, the Beta E1 chain files):

| Button | Text | On dialog | Shown by | Keyed chain | What the press does |
|---|---|---|---|---|---|
| 247 | `Shut down the worm program` | 3224 (Admin Workstation) | 23104 or 23105 (478 S1746 / S1747) | 23106 | 478 → S1749, binds Carter 3884 |
| 456 | `Search the SGU Network.` | 4655 (Hub Computer) | 23133 (1425 S4242) | 23134 | 1425 → S4243 |
| 457 | `Hack the computer's login.` | 5497 (Science Computer) | 23135 (1425 S4243) | 23136 | 1425 → S4244 |
| 458 | `Upload the photo to Daniel Jackson.` | 4659 (Hub Computer) | 23168 (1426 S4246) | 23169 | 1426 → S4247, removes 4832, binds Carter 5462 |
| 575 | `Hack the box.` | 5042 (Jaffa box) | 23244 or 23245 (1502 S4376 / S4377) | 23246 | 1502 → S4378, shows 5043 |
| 576 | `Hack the box.` | 5043 (Jaffa box) | 23246 or 23247 (1502 S4378) | 23248 | grants 5175, 1502 → S4481, binds Ras'nor 5887 |

**The -1 trap.** A `dialog_choice` chain matches the dialog id only, so pressing Done (button -1) on the same dialog also fires it. A matched chain alone is not a pass: the log line must carry the real button id.

## Graded path (no run of its own)

| Button | Arc run and step | Screenshot or DLGREAD |
|---|---|---|
| 247 | UB-U3 run C step 8 | C8 PRESS result |
| 456 | UB-U4 run A step 3 | A3 PRESS result |
| 457 | UB-U4 run A step 4 | A4 PRESS result |
| 458 | UB-U4 run B step 2 | B2 PRESS result |
| 575 | UB-U6 step 8 | `u2b-5043` |
| 576 | UB-U6 step 9 | step 9 PRESS result |

## Fallback (one button per run)

Run only for a button whose arc run never pressed it (the arc failed before that step, or the log line is missing). Stage that button's F-B fixture, then dispatch:

| Fixture | Object (template) | Button text |
|---|---|---|
| F-B247 | Admin Workstation (2323) | `Shut down the worm program` |
| F-B456 | Hub Computer (2322) | `Search the SGU Network.` |
| F-B457 | Science Computer (2324) | `Hack the computer's login.` |
| F-B458 | Hub Computer (2322) | `Upload the photo to Daniel Jackson.` |
| F-B575 | Jaffa box (2326) | `Hack the box.` |
| F-B576 | Jaffa box (2326) | `Hack the box.` |

Driver steps (fallback):

1. `lab_ensure_in_world {"lease_id":L,"character":"<the fixture's character>"}`. WAIT17. DIALOG.
2. FIND(<the object's template>). APPROACH.
3. CLICK(it). DLGREAD: record the dialog text and every button. SHOT(`u6b-<button>-before`).
4. PRESS(<the button text>). SHOT(`u6b-<button>-after`).
5. DIALOG.

Stop conditions (fallback): the click at step 3 opens no dialog after the one retry: record FAIL for the click and stop; the button is `not-run`, because it was never shown.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Shown | The button's dialog opens with the button listed under its seeded text | DLGREAD, the screenshot |
| B. Reaches the server | The press logs a dialog choice for that dialog with the real button id (not -1) | server log |
| C. First press | The keyed chain fires on that first press: its step change (table above) follows at once, with no second press | server log, the next DLGREAD |

The row passes when all six buttons pass A, B and C. A button that was never shown is `not-run`, not FAIL.

## Orchestrator only: verify list

- Per button: `fire_dialog_choice: matched` with `dialog_id` and `button_id` as in the table (247 on 3224, 456 on 4655, 457 on 5497, 458 on 4659, 575 on 5042, 576 on 5043). `button_id` -1 is a FAIL for that button even when the chain matched.
- Then the chain's action lines: `Content: advancing step` (478 → 1749; 1425 → 4243, → 4244; 1426 → 4247; 1502 → 4378, → 4481), and for 576 `Content: granting item` 5175.
- Exactly one press per button in the log before the step change (a second press means the first did not reach the server: FAIL C).
