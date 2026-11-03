## Problem

Dialog 60150 ("Cellblock - The Remaining Pods: Prisoner 329 report"), the first screen shown after accepting mission 62050, renders in the client as `Prisoner` followed by `ERROR: syntax error, unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON`. The seed text is "Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave." The player cannot read the report. This is the same error class as the known dialog 60151 report, and the same message that LP-06b recorded for dialog 60110 ("Vault expansion. I can add 10 slots ..."). 60150 is the row's own expected dialog, so it is not covered by the 60151 known-issue exemption.

Hypothesis (unverified): the client's text-markup parser fails on a space followed by a digit. Both 60110 ("add 10") and 60150 ("Prisoner 329") have that pattern. Dialogs 2299 and 2982, which have no digits, render.

## Evidence

- SigNoz `cimmeria-server`, lab_run `dr-lab`, 2026-11-03 10:23:06.644Z: `Content: accepting mission` 62050 (chain 11003); 10:23:06.647Z `Content: displaying dialog` 60150 (chain 11004). Each fired once.
- `resources.dialog_screens` 60150 index 0 holds the full text above, so the server sent real text.
- Driver `client_window_read` on the open DialogWin: name `Dcbdh`, screen text `PrisonerERROR: syntax error, unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON`. Screenshot: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-042311.658.png.
- docs/analysis/launcher-cache-presync/worknotes/LP-06b.md line 43 records the same error for dialog 60110.

## Acceptance criteria

- Dialog 60150 shows its full text in the client with no parser error.
- A test over the dialog seed text fails on any screen text the client markup parser rejects (the rule is found from the client parser; test the digit-after-space hypothesis first).
- Dialogs 60110 and 60151 are checked against the same rule.

## Test type

Seed lint test (`live_db`) over `resources.dialog_screens.text`, plus a lab UAT re-run of CB-U10a.

## Docs to update

docs/guides/unified-uat.md (CB-U10a, known-issue list), docs/analysis/starter-uat-praxis/README.md.

## Client impact

None if the fix is a seed text edit. A change to the client parser would need a maintainer decision.

## Domain advisor

mission-systems-advisor (dialogs); game-archaeology-specialist for the client parser rule.

## Needs a human for

Looking at the dialog after the fix, to confirm that the text and layout are right.

## Implementation plan

1. Find the client's dialog-text markup grammar (tokens SGW_WHITESPACE, SGW_CHARACTER, SGW_COLON) with the re-lookup skill.
2. Confirm which character sequences it rejects.
3. Edit the affected seed text in `db/resources/` (60150, 60110, 60151 and any others the lint finds).
4. Add the seed lint test and re-run CB-U10a in the lab.

@Cadacious
