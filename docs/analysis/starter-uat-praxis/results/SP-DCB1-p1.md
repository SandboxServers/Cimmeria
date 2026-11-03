# SP-DCB1-p1 result (CB-U01a)

Verified 2026-11-03 by Sonnet. Driver attempts: 2. Server evidence: re-graded (see Re-grade below); the original UNVERIFIED came from the empty `server_log_tail` ring.

| Row | Grade | Tier | Server evidence | Screenshot | Issue |
|---|---|---|---|---|---|
| CB-U01a hold on load | PASS (+ NEEDS_HUMAN 1) | N1 | movement.hold_snap reason=content_hold, player Dcbbh (98), entity 36, from 05:57:46Z (movement.validation). Driver saw no snap-back on screen: a client-display question, not proof of no hold | H-CB-U01-1 lab_screenshot-20261102-235750.892.png (driver did not view; looker not run) | none |
| CB-U01a Livewire win | NOT-RUN | - | driver never played the Livewire; no win | none | none |
| CB-U01a relog holds again | PASS | N1 | after the relog, new entity 37 (space 65583): movement.hold_snap reason=content_hold through 05:58:50Z, same anchor | none | none |
| CB-U01a release / mission 689 | NOT-RUN | - | no win, so not reached | none | none |
| Jaffa variant (Dcbbj) | NOT-RUN | - | no Dcbbj row in sgw_player; only Dcbbh (player_id 98) exists | none | none |

NEEDS_HUMAN: H-CB-U01-1 (glow on Prison Boot Control prop) at C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\lab_screenshot-20261102-235750.892.png; driver did not view it.

## Not run
Livewire win, post-win release, mission 689 check, Jaffa variant. The driver stopped at BOOT without playing the Livewire (not in its step list).

## Resume hint
Next driver: after login, confirm the hold with a walk and note whether the position snaps back (none seen this attempt, so confirm the hold actually applied); right-click the Prison Boot Control prop and use client_minigame_play; win; relog; walk again; then repeat for Dcbbj. Keep `.bug SPU SP-DCB1-p1 CB-U01a start/end` and verify promptly, since the server log buffer was empty at verify time.

## Recipe
Fresh Human Commando Dcbbh into world 12, bookmark, walk 2500 ms, relog, walk, read chat channel 9 for "boot" (notice seen: "Your prison boot is locked to the floor..."), bookmark end.

## Re-grade
Re-graded 2026-11-03 from `dr-lab` logs\server.log (recipe: verify-server-evidence.md). Bookmarks `CB-U01a start` 05:57:42Z and `CB-U01a end` 05:58:53Z (player 98 Dcbbh). 11 `movement.hold_snap` (reason `content_hold`) rows between 05:57:46Z and 05:58:50Z, entity 36 before the relog and entity 37 after it. Only the two hold clauses changed. No `prison boot unlocks` or mission-completion line in the window, consistent with the Livewire win, release and Jaffa rows staying NOT-RUN. No FAIL, no issue file.
