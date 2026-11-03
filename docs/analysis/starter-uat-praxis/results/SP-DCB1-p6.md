# SP-DCB1-p6 CB-U03a (Dcbbh) verified 2026-11-03 by Sonnet; driver attempt 1; server window 07:10:43Z-07:13:04Z (SigNoz + dr-lab server.log)

| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U03a Region2 entry | PASS | N1 (walk; .gotolocation was G setup) | 07:10:57.061 triggerClientHintedGenericRegion Region2 entering (entity 43); 07:10:57.062 fire_enter_region matched, 3 actions; 07:10:57.062 Content: accepting mission 638 chain 1011 (non-Jaffa); one accept only, none at the 07:11:41 re-entry after relog | none | none |
| CB-U03a restore chain after relog | PASS | G (relog) | 07:11:40.687 Content: adding dialog set chain 1026 (Human), dialog 2300, entity 44; one set; first server interact on the Prisoner 07:12:55.882 (target 102111) -> "per-player dialog set -> onDialogDisplay" dialog 2300 | none | none |
| CB-U03a 638 accepted once | PASS | N1 | exactly 1 "accepting mission" 638 in 07:10-07:13; end bookmark shows 638 status 1 | none | none |
| CB-U03a prisoner click (first press after relog) | PASS | N1 | one interact row at 07:12:55.882 (the first client_world_click call failed at read_mouse_over before reaching the server; the unchanged repeat is gotcha 21, not a finding). Pre-relog click never reached the server (camera turn failed; no interact row): driver/harness, not a finding | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-011115.870.png (failed pre-relog click, not looked at) | none |
| CB-U03a step 2114->2115 | PASS | N1 click + dialog close | 07:13:04.570 dialogButtonChoice 2300 button -1; fire_dialog_choice matched; 07:13:04.571 Content: advancing step 638 -> 2115 chain 1015; end bookmark 638 step_id 2115 | none | none |
| CB-U03a door panel Livewire start (chain 1028), 2116, 2144, 2145 | NOT-RUN | - | no rows; belongs to sibling sub-rows | none | none |

No WARN or ERROR rows in the window.

## Not run
Steps 2115 onward (door panel Livewire, dialogs 2299, 639, drone, vial). H-CB-U03-1 (med-station look): driver listed NEEDS_HUMAN none and took no screenshot; unjudged. No looker run (the only screenshot is the failed pre-relog click).

## Resume hint
Next part starts with 638 at step 2115 on character Dcbbh (world Castle_CellBlock); click the Door control panel (entity near the Prisoner) for the Livewire row. Walk, never .gotolocation, to retrigger regions.

## Recipe
Verify via SigNoz filter resource.cimmeria.lab_run='dr-lab' AND attribute.note CONTAINS 'SPU SP-DCB1-p6', then Select-String on dr-lab logs\server.log for accepting mission, adding dialog set, interact, dialogButtonChoice, advancing step.