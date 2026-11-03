PACKET SP-DCB3-p1 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1; server evidence window 10:12:05Z-10:13:23Z (lab_run dr-lab, server 6b50d9131417)

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| T01/T02 dialog 2982 | PASS (+ NEEDS_HUMAN 1) | N1 boot click, G .bug | Bookmark 10:12:10Z (playtest.bookmark, player 102 Dcbdh, entity 57) recent_events: world_enter seq1, action_list 1001 AcceptMission > DisplayDialog > AddDialogSet seq2, `dialog=2982 chain=1001` seq3; only one dialog event, so once | Driver: no dialog open at read time | A/B: NO dialog box with "The last thing you remember" | Dialog was shown ~93 s before the read; render is a human check |
| T01/T02 mission 622 | PASS | G | Both bookmarks (10:12:10Z, 10:13:03Z): mission 622 status 1, step_id 2113, objective 2452 visible, 3238 hidden/optional | Tracker "Arm Yourself!" | n/a | Journal open not confirmed (see sheet) |
| T01/T02 Frost searchable, Guard not | PARTIAL (UNVERIFIED) | G | Seed rows: spawn 19 ArmYourself_FrostBody tpl 14 (Cpt frost corpse), spawn 15 ArmYourself_GuardBody tpl 21; both tpl interaction_type 0, vault_scope personal. Neither is in the live cell entity query for space 65609 (29 NPCs, no FrostBody/GuardBody tag). Spawn 20 ArmYourself_NIDGuard (entity 102905) is live with interaction_type_flags 0 | Driver saw no Frost or Guard body within 8 m | n/a | Searchable flag could not be read: corpses not spawned in the cell when queried (after boot step done, 10:12:52Z mission 689 completed). Not graded FAIL, because the corpses may be personal-scope and spawn later |
| Side: boot step (mission 689) | PASS | N1 | 10:12:52Z "Content: completing mission" chain 1025, "Mission completed directly" 689 Prison Boot Lock, status 2; bookmark 2 shows 689 completed_steps [68900]; action_list 1024 StartMinigame at ~10:12:39Z | Chat "The prison boot unlocks" | n/a | Not part of the row grade; consistent with the driver |

SIGN-OFF SHEET (NEEDS_HUMAN):
- H-T01-1 | T01 dialog 2982 render | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-041216.320.png | Driver: no dialog open (not viewed). Looker: no dialog box, no journal, buff icon UNSURE.
- H-T02-1 | journal shows mission 622 | C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261103-041226.525.png | Driver: no journal in ui_state. Looker: NO journal; an Alloy/Craft crafting window is open instead (the j key may be bound to crafting or the key hit another window).
- H-T03 | Stasis Sickness icon (K7, none expected) | same two screenshots | Looker UNSURE; a small icon by the portrait, no clear buff icons.
- Minigame images (not graded): client_minigame_play-20261103-041252.497.png and .498.png in the same folder.

FINDINGS:
(none filed). The empty Frost/Guard check is a lab/recipe gap (case c), not a product bug.

NOT RUN / BLOCKED:
- Frost/Guard searchable flag: not readable via server_entity_get because the corpse spawns were not live. Next driver recipe: right after world entry, before the boot minigame, run server_entity_query with tag ArmYourself_FrostBody and ArmYourself_GuardBody (space 65609, near -325,73,-210) and read interaction_type_flags. If still absent, check whether the corpses spawn only on a later mission step.
- Journal: j key opened a crafting window per the looker. Use the journal UI button or a verified binding.

RESUME HINT: redo only the Frost/Guard entity check and the journal look. Mission 622 and dialog 2982 are already server-confirmed. The boot is already complete for Dcbdh.
