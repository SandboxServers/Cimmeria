## Status

Confirmed product defect (seed text), owner-confirmed 2026-11-03. The owner reports the parse error appears for a human player, not only in the lab client. The dialogs belong to the custom quests added in Cellblock (missions 62050 and 62060). Affected dialogs: 60150, 60110 and 60151.

## Defect

Dialog 60150 ("Cellblock - The Remaining Pods: Prisoner 329 report"), the first screen shown after accepting mission 62050, renders in the client as `Prisoner` followed by `ERROR: syntax error, unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON`. The report text is unreadable. The same error appears in dialog 60151 (known issue) and in dialog 60110 ("Vault expansion. I can add 10 slots ...", recorded by LP-06b). 60150 is the CB-U10a row's own expected dialog, so it is not covered by the 60151 known-issue exemption.

## Research detail

Seed rows (CIMMERIA_AUTHORING, CB-N05):

- `db/resources/Dialogs/Seed/dialogs.sql:10875`: `(60150, 0, NULL, 'DUIST_DefaultDialog', NULL, NULL, 'Cellblock - The Remaining Pods: Prisoner 329 report')`
- `db/resources/Dialogs/Seed/dialog_screens.sql:28665`: `(60150, 96110, 'Prisoner 329 says there are still people in the other pods. Tell Col. Marsh to check them before you leave.', 0, 0)`
- Fired by chain 11004 (`castle_cellblock_chains.sql:3564-3580`), a `display_dialog` from a `mission_accepted` trigger (a monologue path, not an interact path).

Where the error lands: the client prints "Prisoner" then the error, so the parse dies at the space before the digit `3` of "329". Dialog 60110 dies at the space before "10" ("I can add 10 slots", `dialog_screens.sql:28646`). Dialog 60151 (screen 96111) carries "Prisoner 329" too (`dialog_screens.sql:28668`). The shape is the same in all three: whitespace followed by a digit run. The text is plain ASCII with no hidden characters, and ordinary punctuation (". ") is accepted. Dialogs 2299 and 2982, which have no digits, render.

Cause (inferred, not proven): the client's markup lexer has a CHARACTER class that excludes digits. After a word, the grammar expects CHARACTER or COLON, so a whitespace token in a state that expects a `key:value` tag (as in `set:ItemIcon001 image:Spray_Injector`, `items.sql:139`) is a syntax error. The grammar is client-side; confirming it needs the `re-lookup` skill (Ghidra, string "SGW_WHITESPACE").

Open discriminator: stock seed texts also have word-space-digit shapes, such as "Level 16. Understood." (5357), "Classification 212 ..." (2339), "Bring 20 M67 Grenades ..." (572) and "Container 37A1 ..." (1168). Nothing in docs shows whether those render. If they render, the cause is something else about authored dialogs (the three failing rows are authored screen ids 96110, 96111 and 200010, not client-stock ids). This does not change the defect or the fix; it changes the rule the lint test encodes.

## Evidence

- SigNoz `cimmeria-server`, lab_run `dr-lab`, 2026-11-03 10:23:06.644Z: `Content: accepting mission` 62050 (chain 11003); 10:23:06.647Z `Content: displaying dialog` 60150 (chain 11004). Each fired once.
- `resources.dialog_screens` 60150 index 0 holds the full text above, so the server sent real text.
- Driver `client_window_read` on the open DialogWin: name `Dcbdh`, screen text `PrisonerERROR: syntax error, unexpected SGW_WHITESPACE, expecting SGW_CHARACTER or SGW_COLON`. Screenshot: C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-042311.658.png.
- docs/analysis/launcher-cache-presync/worknotes/LP-06b.md line 43 records the same error for dialog 60110.
- Owner confirmation (2026-11-03): the error appears for a human player.

## Fix

Safe fix: a seed text edit in `db/resources/Dialogs/Seed/dialog_screens.sql`. Spell the number out ("Prisoner three twenty-nine says ...", "add ten slots"), or use "Prisoner #329" only if the lab proves `#` passes. No client patch is needed. A change to the client parser would need a maintainer decision.

Cheapest discriminating check before choosing the wording: show a stock dialog with a digit after a space (572 or 5357) in the lab and read the DialogWin text.

## Acceptance criteria

- Dialog 60150 shows its full text in the client with no parser error.
- Dialogs 60110 and 60151 show their full text with no parser error.
- A test over the dialog seed text fails on any screen text the client markup parser rejects (the rule is found from the client parser; test the digit-after-space hypothesis first).

## Test type

Seed lint test (`live_db`) over `resources.dialog_screens.text`, plus a lab UAT re-run of CB-U10a.

## Docs to update

docs/guides/unified-uat.md (CB-U10a, known-issue list), docs/analysis/starter-uat-praxis/README.md.

## Client impact

None with a seed text edit.

## Domain advisor

mission-systems-advisor (dialogs); game-archaeology-specialist for the client parser rule.

## Needs a human for

Looking at each dialog (60150, 60110, 60151) after the fix to confirm the text and layout.

## Implementation plan

1. Find the client's dialog-text markup grammar (tokens SGW_WHITESPACE, SGW_CHARACTER, SGW_COLON) with the re-lookup skill.
2. Confirm which character sequences it rejects.
3. Edit the affected seed text in `db/resources/` (60150, 60110, 60151 and any others the lint finds).
4. Add the seed lint test and re-run CB-U10a in the lab.

@Cadacious
