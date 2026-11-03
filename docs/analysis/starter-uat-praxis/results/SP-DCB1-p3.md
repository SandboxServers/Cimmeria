# SP-DCB1-p3 result

Runner DCB1-c, 2026-11-03. Driver attempts: 2 (both BLOCKED at S1, general-purpose driver). No verifier run: nothing reached the world.

| Row | Grade | Server evidence | Issue |
|---|---|---|---|
| CB-U01c (Jaffa boot hold, win, release) | BLOCKED | character Dcbbj was never created; the lab create-character flow cannot make a Praxis Jaffa | none filed (harness defect, see below) |

## Why BLOCKED

`lab_ensure_in_world` with archetype `Jaffa` fails at `lab_create_character` step `first_name`:
`click CharCreate_Name1Edit: CharCreate_Name1Edit is not visible; client state: {"visible_screens":["CharCreateWin"]}`.
Reproduced a third time by the runner calling `lab_create_character {alignment praxis, archetype Jaffa, gender male, first Uat, last Dcbbj}` directly on a fresh client after `lab_login`. The account had a free slot (`lab_ensure_character_slot`: free_slots 1, nothing deleted), so it is not a slot limit.
Screenshot of the Jaffa create screen (C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-001909.508.png) shows a single `Name` box plus a `Suffix` dropdown and no first/last name pair, so the tool's first/last name widgets (`CharCreate_Name1Edit`) do not exist for this archetype page. Commando (Dcbbh) used the two-name flow successfully.
After a failed create the client stays on CharCreateWin and `lab_ensure_in_world` cannot recover (77 Escape presses, no login screen); only `lab_client_restart` clears it.

Deterministic harness gap (lab `lab_create_character` flow has no Jaffa name path), not a server finding. Attempt 3 and the takeover would use the same tool, so they were not spent.
