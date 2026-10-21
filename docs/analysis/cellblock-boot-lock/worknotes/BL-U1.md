# BL-U1 - cellblock boot lock lab UAT

Date: 2026-10-20 local (log times are UTC 2026-10-21). Server: branch cellblock-boot-lock/integration, DB sgw_cbl_integration (local, login :8081, lab endpoint :8451), logs in `.claude/worktrees/cbl-integration/logs/`.
Lease: lease-dd91bbe6d2574e2b9663f195b8f6e929 (BL-U1 coordinator, default instance, account lab); released at the end. Client restarted with owner approval (pid 76820 -> 237744).
run_dir: none. This was a manual run (no `lab_uat_run`); evidence is the server log, content.log, protocol.log, the DB and screenshots.

**Overall: PASS.** Every step for A and B passed. No `hold_skipped_*` line anywhere in server.log (count 0), so the fail-open gate is clear. Two caveats: the 10 m walk stopped at ~9.5 m from the prop (geometry), and on cancel the server logs a WARN `result_deposit_refused` (below).

Screenshots are in `C:\Users\Steve\AppData\Local\cimmeria-lab\screenshots\`.

## (a) Read-only seed checks: PASS (from the first attempt; DB not reloaded)

| Check | Result |
| --- | --- |
| resources.entity_templates 1250 | PASS. static_mesh `Ga-Props.GA-PuzzleStation00`, display_name `Prison Boot Control` (name_id 7550), template_name `Cellblock - Prison Boot Control`, flags 4 |
| resources.spawnlist 284 | PASS. world_id 12, (-333.03, 73.472, -227.32), tag `Cellblock_PrisonBootControl`, template 1250, is_stationary true |
| content_chains 1022-1025, 1047, 1048 | PASS. all enabled, scope mission 689 |
| content_triggers | PASS. 1024 interact_tag; 1047 player_loaded `Castle_CellBlock`; 1048 mission_completed `689` |
| content_actions 1023 | PASS. movement_lock hold at [-334.231, 73.472, -228.026], anchor_tag `Cellblock_PrisonBootControl`, max_distance 3.0, client_flag false; no launch_ability 1597 |
| content_actions 1025 | PASS. launch_ability 1598, movement_lock release, set_interaction_type ~256 and ~1073741824, complete_mission 689 |
| content_actions 1047 / 1048 | PASS. \|256 \|1073741824 / release, ~256, ~1073741824 |

## (b) Characters (kept, not deleted)

| Char | Name | player_id | Notes |
| --- | --- | --- | --- |
| A | Boot Alphalock (list name `Alphalock`) | 71 | Praxis Soldier, male, via `lab_create_character` |
| B | Betalock | 72 | Praxis Goa'uld; single-name race, created by hand with `client_ui_sequence` (CharCreate_NameEdit, CharCreate_CreateButton). Server: char_def 19 BS_GoauldFemale, start_state NON_CANONICAL_BLOCKED_LEGACY, world Castle_CellBlock |

Lab setup notes: the login combo was preselected to `CastleLocal`; set to `Local` with Lua (`Login_ServerTypeCombo:findItemWithText("Local",nil)`, runtime only) before `lab_login`. A keep-awake job held the display and killed Ribbons.scr for the run.

## (c) Char A (Alphalock)

| # | Step | Result | Evidence |
| --- | --- | --- | --- |
| A1 | Held at login | PASS | Entity 2, space 65555. Mission 689 accepted 02:33:42.277Z (chain 1022). `movement_lock.hold` decision_outcome=held 02:33:42.284627Z, chain 1023, client_flag false, anchor_tag Cellblock_PrisonBootControl. Position (-334.23, 73.47, -228.03). Hold text on the wire 02:33:42.304736Z, chat seq 24291 (Feedback/SYSTEM), exact: "Your prison boot is locked to the floor. Right-click the boot control beside you and win the Livewire game to work it loose." |
| A2 | Prop found, glow | PASS | Prop 100592 at (-333.03, 73.47, -227.32), 1.39 m. interaction_type_flags 1073742080 (256 + 1073741824), set 02:33:42.287Z (content.log `\|256`, `\|1073741824`). `lab_screenshot-20261020-213630.680.png`: bright yellow glow, green nameplate |
| A3 | Walk 6 m for 3 s | PASS | Move to (-339.4, 73.47, -231.0): timed out at (-335.06, 73.47, -227.41); after 2 s at (-334.39, 73.47, -227.99). Move to (-328.0, 73.47, -232.0): timed out at (-334.37, 73.47, -227.42). hold_snap 02:36:32.698, 33.826, 34.647, 35.540, 44.773 (client ~1.2-1.3 m off the anchor; all within 1.5 m). Hold text on the wire only at 02:36:32.700845Z and 02:36:44.775408Z (5 s throttle holds); chat seqs 25079/25080. Also WARN `movement.speed_warning` x2 (warn-only) |
| A4 | Relog mid-lock | PASS | Entity 3, space 65556. Held 02:37:28.158867Z (chain 1023); flags set 02:37:28.1589Z; prop 100641 flags 1073742080. Hold text on the wire 02:37:28.172270Z, chat seq 27584 |
| A5 | Right-click by entity_id, first press | PASS | Held spot is 1.34-1.52 m from the prop, so the 3-4 m stand was impossible; clicked from the held spot, pitch -15.03 (`lab_screenshot-20261020-213824.601.png`). MinigameWin opened on the first press (real_input N1). interact 02:38:30.799340Z target 100641; fire_interact_tag matched, Livewire chain 1024, on_victory [1025]; minigame started 02:38:31.096Z |
| A6 | Cancel once; still held; retry | PASS | Closed with MinigameWin__auto_closebutton__ (950,118). Cancel 02:38:35.645Z reason player_closed_window, outcome canceled. Position unchanged (-334.37, -227.42). Retry interact 02:38:48.932785Z reopened MinigameWin. See WARN note below |
| A7 | Win | PASS | `client_minigame_play` (entity 3): smartfox_victory true; client outcome "unverified" (window hidden), server confirms. `client_minigame_play-20261020-214334.209.png` / `.210.png`. Victory 02:43:33.985Z chains [1025]; ability 1598 launched 02:43:33.989796Z; `movement_lock.release` released 02:43:33.990427Z; flags `~256`, `~1073741824` -> 0 at 02:43:33.9905Z; mission 689 completed 02:43:33.991355Z; chain 1048 release nothing_held 02:43:33.994Z. Release text on the wire 02:43:33.996205Z, chat seq 32496, exact: "The prison boot unlocks. You can move again." |
| A8 | Walk 10 m, no snap-back | PASS (caveat) | Reached (-342.24, -231.89) before geometry stopped it; final (-341.52, 73.47, -231.5), 9.47 m from the prop. No hold_snap or hold line after the win |
| A9 | Glow gone | PASS | Flags 0 on the server. `lab_screenshot-20261020-214433.928.png` shows only the faintly lit top, the same as the flags-0 baseline after relog `lab_screenshot-20261020-215011.855.png` |
| A10 | Relog: no hold | PASS | Entity 4, space 65557. fire_player_loaded matched 2 actions 02:49:30.998892Z; no movement_lock line, no hold text. Stayed at (-341.52, -231.5). Prop 100690 flags 0 |
| A11 | Mission 689 | PASS | sgw_mission player 71: before the win status 1, step 68900; after status 2, repeats 1, current_step null |

## (d) Char B (Betalock, Goa'uld)

| # | Step | Result | Evidence |
| --- | --- | --- | --- |
| B1 | Held at login | PASS | Entity 5. Missions 622 and 689 accepted 02:50:32.58Z. Held 02:50:32.582339Z (chain 1023, client_flag false). Flags `\|256`, `\|1073741824` (1073742080) 02:50:32.5824Z. Prop 100739 at (-333.03, 73.47, -227.32), 1.39 m. Player (-334.23, 73.47, -228.03). Hold text on the wire 02:50:32.586544Z (the chat capture missed it: hook installed late). `lab_screenshot-20261020-215135.124.png` |
| B2 | Walk 6 m for 3 s | PASS | `client_move_to` timed out at (-335.05, -227.64); after the wait (-334.35, 73.47, -228.07). hold_snap 02:51:38.427, 39.523, 40.299, 41.147. Hold text on the wire once, 02:51:38.428281Z (throttle holds) |
| B3 | Right-click, first press | PASS | interact 02:52:00.266326Z, MinigameWin opened on the first press (real_input, 1.52 m); minigame started 02:52:00.575Z |
| B4 | Cancel once; still held; retry | PASS | Cancel 02:52:03.944736Z; same WARN `result_deposit_refused` 02:52:03.945328Z. Position unchanged (-334.35, -228.07). Retry interact 02:52:17.875540Z reopened the window |
| B5 | Win | PASS | `client_minigame_play` (entity 5); `client_minigame_play-20261020-215632.833.png` / `.834.png`. Victory 02:56:32.459081Z chain 1025; ability 1598 02:56:32.460187Z; release (released) 02:56:32.460354Z; flags -> 0 02:56:32.4604Z; mission 689 completed 02:56:32.460679Z; chain 1048 nothing_held. Release text on the wire 02:56:32.461267Z, and visible in chat: "The prison boot unlocks. You can move again." |
| B6 | Walk 10 m, no snap-back | PASS (caveat) | Move to (-344.0, 73.47, -233.7) stuck on geometry at (-342.29, 73.47, -231.9); after 3 s at (-341.65, 73.47, -231.48), 9.57 m from the prop. No hold_snap or movement_lock line after 02:56:32 |
| B7 | Glow gone | PASS | Prop 100739 flags 0. `lab_screenshot-20261020-215729.873.png` matches the flags-0 baseline |
| B8 | Relog: no hold | PASS | Entity 6, space 65559. Entered world 02:58:22.533048Z; fire_player_loaded matched 2 actions 02:58:22.533514Z; no movement_lock line, no hold or release text on the wire. Stayed at (-341.65, 73.47, -231.48). Prop 100788 flags 0 |
| B9 | Mission 689 | PASS | sgw_mission player 72: status 2, repeats 1, current_step null |

## Findings for follow-up

- **WARN on cancel:** `Minigame terminal result refused reason=result_deposit_refused` at 02:38:35.646689Z (A) and 02:52:03.945328Z (B), right after the canceled result. It is not a hold_skipped line, and the hold and retry behave correctly. It is probably the cancel trying to deposit a terminal result the session no longer accepts. Worth a look so a normal cancel doesn't log WARN.
- **Stand distance:** the hold anchor is ~1.4 m from the prop, so the driver gotcha "stand 3-4 m away" can't apply here; first-press clicks worked from the held spot.
- **10 m walk:** both characters stopped at ~9.5 m from the prop (~8-9 m from the start) on cell geometry. No snap-back at any point, which is the property under test.
- **Lab:** `lab_login` doesn't select the login combo row from lab-account.json (`CastleLocal` was preselected); `lab_create_character` can't make Goa'uld (single-name). Chat capture missed B's login hold line; protocol.log is the evidence.
