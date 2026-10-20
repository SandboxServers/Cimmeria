# Minigames lab UAT recipes

**Ask the user before any lab use.** This page is a set of recipes. It runs nothing, and writing it used no lab.

| | |
| --- | --- |
| Type | How-to (lab recipes) |
| Audience | An orchestrator briefing a Haiku `lab-driver`; engineers verifying a minigame change |
| Last updated | 2026-10-20 (MG-73) |
| Companions | [README](README.md) (ledger, D-MG11, D-MG22, D-MG23), [spec-launch-flow.md](spec-launch-flow.md), [spec-shells.md](spec-shells.md), [content-map.md](content-map.md), [Debug Area minigame terminals](../../content/debug-area.md#minigame-terminals), [minigame-system.md](../../gameplay/minigame-system.md), [lab-uat skill](../../../.claude/skills/lab-uat/SKILL.md), [unified UAT guide](../../guides/unified-uat.md) |

[unified-uat.md](../../guides/unified-uat.md) is where MG-99 records the manual rows. This page links there and does not duplicate it.

> **Status: never run.** No client has played any recipe below. Every "expected" line comes from a seed, a chain-replay test or Rust source, not from a lab session. A step that disagrees with the client is a finding. Write it in the ledger; do not bend the recipe to fit.

## Before you start

1. **Ask the user and wait for a yes.** Another session may hold the lab, and lab input hooks swallow the user's own mouse clicks. If `lab_lease_status` shows a holder, wait.
2. Run a local server on the branch under test and point the lab at it ([lab-uat § 1](../../../.claude/skills/lab-uat/SKILL.md)). Seeds reload on a server start; a DB reload deletes the lab character, so recreate it.
3. `lab_lease_acquire`, then one `lab_ensure_in_world`. Release the lease at the end.
4. The server must have its minigame TCP listener enabled. With it off, a launch logs WARN `reason = "minigame_server_disabled"` and nothing opens (`cell_dispatch/minigame.rs`).

### Gotchas for the driver brief

Put these first in every `lab-driver` brief. They are the [lab-uat § 4 sheet](../../../.claude/skills/lab-uat/SKILL.md) plus the minigame-specific ones.

- Stand 4-6 m from a target, on open floor, never against a wall. Server interact range is 5.0 m. Click **by entity id**, not by point.
- Clear every dialog first. A GM jump under an open dialog orphans it.
- Teleport with `.gotolocation <World> x y z`. Never `.gotoxyz`: it moves your selected target.
- Read the chat input back (`return Inst1Chat_Input:getText()`) before pressing Enter.
- A minigame is a Flash window. Escape does not close it. Read the close button's rect and click there.
- A held player cannot walk (MG-63a): the server refuses moves beyond 0.5 m of the hold point. "I could not walk away" while a window is open is the expected result.
- `client_minigame_play` (tier N1) drives a game; if a recipe says "win" and the game's rules are not scripted, report that instead of guessing a solution.
- Report raw results: chat lines verbatim, window text, entity ids. No diagnosis.

### How to grade each recipe

The driver's summary is a claim. Verify each one against the server before you grade it:

- `server_log_tail` keeps only 500 lines. Search `minigame`, `start_minigame`, `reason=`.
- `server_minigame_state` shows the live session.
- SigNoz for anything older than the tail ([telemetry-triage](../../../.claude/skills/telemetry-triage/SKILL.md)). Filter on the server service, `body CONTAINS 'minigame'`, and narrow by the `reason` or `game` attribute named in the recipe.
- Record the result per recipe in the campaign ledger and the unified UAT row. A FAIL becomes a fix packet.

### `reason=` values you will meet

| Value | Meaning | Source |
| --- | --- | --- |
| `min_play_time` | A client `victory` under 5 s on a shell was refused (WARN) | `games/placeholder.rs` |
| `timeout` | A shell hit its 60 s countdown; sent `failure` (logged at DEBUG, so it may not show at the default level) | `games/placeholder.rs` |
| `unknown_minigame_type` | Content load dropped a `start_minigame` row with an unknown or wrong-case name (WARN, at server start) | `content-engine` loader |
| `minigame_launch_refused` (`event=`) with `reason=` | An ability launch was refused: `caster_dead`, `stale_queue`, `already_playing`, `offer_pending`, `player_mismatch` | `start_minigame.rs`, `minigame_launch/state.rs` |
| `player_cancelled`, `no_offer`, `offer_expired`, `stale_offer` | Pre-game dialog answers (D-MG11 on only) | `minigame_launch/state.rs` |
| `minigame_server_disabled` | The TCP listener is off | `cell_dispatch/minigame.rs` |

## R1. Livewire, the cellblock pair

**Goal:** the oldest seeded pair still works end to end: launcher chain 1016, victory chain 1017 (mission 638, the cell door button). The same shape backs 1041/1042 (mission 640) and 1060/1061 (mission 641); run those only if R1 passes and the user wants all three.

**Setup**

1. `/gmmissionassign 638 1`, then `/gmmissionadvance 638 2115` (a step id of mission 638).
2. `.gotolocation Castle_CellBlock` and stand 4 m from the object tagged `329_CellDoorButton`. Confirm its entity id with `client_entity_find`.

**Steps**

1. Hover the button. Expected: the Livewire cursor (interaction bit 256).
2. Click it by entity id.
3. Expected: a Livewire window opens. `BSF_PlayingMinigame` is set; try to walk 3 m and expect to stay put.
4. Win the game (`client_minigame_play`, or by hand).

**Expected client view:** the window closes on the win, mission 638 moves to step 2116, sequence 1749 plays, and the button loses its minigame cursor (`set_interaction_type` clears mask 256). You can walk again.

**Server log:** chain 1016 fires, one `StartMinigame` goes to base, the session starts, a win result comes back, and chain 1017 runs `advance_step 638 2116`.

**SigNoz:** `body CONTAINS 'minigame'` for the player's session id across the window; one start, one result.

**Fail signs:** no window and a log line `interact_no_effect` means the step was not active or the tag is wrong. The button keeping its cursor after a win means chain 1017's third action did not run.

## R2. The five ported games through the Debug Area terminals

**Goal:** each of CrystalGame, Alignment, GoauldCrystals, ConverseBasicHumanoid and Livewire starts from its terminal, plays, and speaks the right result line. Spawns 14100-14104, tags `DebugArea_Minigame_<Game>`, launcher chains 24050-24054 ([debug-area.md](../../content/debug-area.md#minigame-terminals)).

**Setup:** GM account. `.gotolocation DebugArea`, then walk to the plaza's north row (z about -903). The terminals stand at x 234, 239, 244, 257, 262 and 267. Clear dialogs. Stand about 5 m south of the terminal you are testing, for example `.gotolocation DebugArea 239 6.7 -898` for Alignment. The y value is approximate; correct it from `client_player_state` if you land under or above the floor.

**Per game, steps**

1. Find the entity id of the terminal (`client_entity_find`, tag from the table).
2. Hover it. Expected: the Livewire cursor on every terminal (bit 256 is the only confirmed minigame cursor).
3. Click it by entity id. Expected: the named game's window opens at difficulty 1.
4. Play to a **win**. Expected chat: "Minigame round trip complete: the server recorded your win." (dialog 60120, screen 200020, chain 24056, spoken as `Terminal`).
5. Click again and **lose** (let the timer run out, or fail the game's loss condition). Expected chat: "Minigame round trip complete: the server recorded your loss." (screen 200021, chain 24057).

| Game | Terminal spawn | Launcher chain | Server-validated rules |
| --- | --- | --- | --- |
| CrystalGame | 14100 | 24050 | yes |
| Alignment | 14101 | 24051 | yes |
| GoauldCrystals | 14102 | 24052 | yes |
| ConverseBasicHumanoid | 14103 | 24053 | see [spec-conversebasichumanoid.md](spec-conversebasichumanoid.md) |
| Livewire | 14104 | 24054 | yes |

**Server log:** chain 24050-24054 fires, one `StartMinigame` with the game name, the session opens, the result comes back, then chain 24056 or 24057. No WARN with `reason=` apart from the ones in "Expected" above.

**SigNoz:** `body CONTAINS 'minigame'` plus the game name. A win must show a victory result and then the barking chain; a loss, a failure result.

**Fail signs:** a window for the wrong game (a launcher chain keyed on the wrong tag); no chat line after the window closes (the `on_victory_chains` or failure link is broken); WARN `unknown_minigame_type` at server start (a wrong-case name in the seed).

## R3. The terminal placement check (MG-74)

**Goal:** close the open "Placement is unchecked in the client" warning in [debug-area.md](../../content/debug-area.md#minigame-terminals).

Read-only. Do this on the first R2 pass, before clicking anything.

1. From the plaza, `client_entity_table` filtered to the six tags. Expected: six entities.
2. For each, take one screenshot standing 5 m south. Expected: the terminal stands on the courtyard floor (not floating, not under it, not inside a wall) and x 252 is clear for the walkway.
3. Read its nameplate. Expected: `Minigame: <Game>`. Report what is shown. The `display_name` on a `being`-class object is **unverified**; "no nameplate" is a valid answer and a finding, not a failure of the lab run.
4. Hover each. Expected: the Livewire cursor.

Record the six y values the client reports; a mismatch with the seed's 6.66-6.90 beyond about 0.5 m means the placement needs a fix.

## R4. Mission 582, step 2307: Converse with Marsh

**Goal:** chain 24000 launches `ConverseBasicHumanoid` at difficulty 1 on Colonel Marsh, and chain 24001 completes objective 2675 and advances to step 3274. Mission 582 has no offer path (M10: giver Blackstock has no spawn and set 530 is unbound), so a GM sets it active.

**Setup**

1. `/gmmissionassign 582 1`, then `/gmmissionadvance 582 2307`.
2. Marsh is spawn 344, tag `CmdCenter_Marsh`, world 68 (Harset). `.gotolocation` there and stand 4 m from him. Make sure missions 1360 (step 4038) and 1361 (step 4694) are **not** active: the launcher refuses to fire while they are.

**Steps**

1. Click Marsh by entity id.
2. Expected: dialog 2433 ("Blackstock presented me with some very interesting videotapes...") appears, then the Converse game window.
3. Play to a win.

**Expected:** the window closes, objective 2675 completes and the mission shows step 3274 "Convince Marsh you are innocent." A **loss** leaves step 2307 active and Marsh clickable again; there is no failure chain by design.

**Guard check:** with mission 1360 on step 4038 (`/gmmissionadvance 1360 4038` after assigning it), a click on Marsh must run chain 6501 and never start the game.

**Server log:** chain 24000 fires, one `StartMinigame('ConverseBasicHumanoid', difficulty 1)`, victory runs chain 24001.

**Fail signs:** the game starts under mission 1360 (the `neq` guards are wrong); the dialog shows and no window opens (check `minigame_server_disabled`).

## R5. Converse abilities 1327 and 2089

**Goal:** both abilities run effect script `StartMinigame` (effects 1643 and 2813, NVPs 25501-25504) and the caster plays `ConverseBasicHumanoid` at difficulty 1. Neither ability is granted to any class (content-map §8), so a GM grants it. There is no victory or failure chain; the session result is the only outcome.

**Steps**

1. `/gmgiveability 1327` (repeat later with 2089). Open the ability window and place the ability on the hotbar if it is not there already.
2. Clear dialogs. Use the ability (`client_use_ability`).
3. Expected: the Converse window opens for the caster. No target is needed.
4. Win or lose, then close. Expected: `BSF_PlayingMinigame` clears and you can walk.
5. Use the ability **while the window is open**. Expected: refused, WARN `event=minigame_launch_refused reason=already_playing` (or `offer_pending` with D-MG11 on).
6. The dead-caster refusal (`caster_dead`) is covered by a unit test only; do not try to test it in the lab.

**Server log:** `minigame_launch_queued` from the script, then the launch drains after effects resolve and a `StartMinigame` goes to base.

**Fail signs:** the ability fires with no window and no `minigame_launch_queued` line (the script name did not load: check effects 1643 and 2813 `script_name`).

## R6. Prison Boot (items, mission 689, abilities 1597 and 1598)

**Goal:** the boot holds its wearer, and a Livewire win frees them. Chains 1022-1025, MG-63a (movement hold) and MG-63b (`MovementHold` and `MovementRelease` scripts).

**Setup**

1. Put the boots on the character: `/gmgiveitem 3438 1`, then equip item 3438 "Prison Boots".
2. Relog into `Castle_CellBlock` so chains 1022 and 1023 run on zone load: mission 689 is accepted (hidden, no quest-log entry) and ability 1597 launches.

**Steps**

1. Wait for the zone to finish loading. Try to walk 3 m. Expected: you cannot (the server holds you within 0.5 m).
2. Double-click the worn boots (`item_use` 3438). Expected: Livewire opens (chain 1024).
3. Confirm a minigame window also holds movement: you still cannot walk.
4. Win the game. Expected: chain 1025 launches ability 1598, clears the lock and completes mission 689. You can walk.
5. Relog. Expected: you are **not** held (chain 1023's guard `mission_status 689 neq completed` is false).

**Server log:** chain 1023 launch of ability 1597 on load; a movement rejection while held (grep `MovementLocked`; the exact line is unverified); chain 1024 `StartMinigame`; chain 1025 completes mission 689.

**Reset for a re-run:** a completed mission is left alone by `/gmmissionclear`. Use a fresh character.

**Fail signs:** you walk away on load (MG-63b scripts or the hold failed); a win does not free you (ability 1598 or its release effect did not run); you are held again after the relog (mission 689 did not complete).

## R7. Shell refusal and the 60 s timeout (MG-64)

**Goal:** the `Hack` shell, launched from the Debug Area Hack terminal (spawn 14105, tag `DebugArea_Minigame_Hack`, chain 24055), shows a countdown, refuses a `victory` in the first 5 s, and fails by itself at 60 s.

**Steps**

1. Stand 5 m south of the terminal at x 267, z -903. Click it by entity id.
2. Expected: the Hack shell window with a countdown at 60 s.
3. Press its win button **within 5 s**. Expected: nothing happens; the server logs WARN `reason = "min_play_time"` with `elapsed_ms` under 5000.
4. Wait the full 60 s without pressing win. Expected: the shell shows its failure state and the chat line "Minigame round trip complete: the server recorded your loss." (screen 200021).
5. Click the terminal again. Wait 6 s, then press win. Expected: accepted; the win line (screen 200020).

**Server log and SigNoz:** the refusal is WARN and visible; the `timeout` line is DEBUG, so prove the timeout by the failure result and chain 24057 instead. SigNoz: `body CONTAINS 'min_play_time'`.

**Fail signs:** the countdown is missing (spec-shells §10 item 4: the shell may ignore `fullgamestate`; report it); the window never fails on its own; the first win press is accepted.

**Unknown-name check (no client):** a seed row with an unknown or wrong-case game name logs WARN `reason = "unknown_minigame_type"` at server start and is dropped. This recipe does not need the lab: grep the server start log.

## R8. D-MG11: the pre-game dialog, off and on

**Goal:** `CIMMERIA_MINIGAME_START_DIALOG` (default off) switches the launch between "window at once" and the original pre-game dialog ([README D-MG11](README.md)). Turning it on by default is a follow-up decided from this recipe.

**Off (default).** Do R2 on one terminal. Expected: the window opens at once.

**On.** Restart the server with `CIMMERIA_MINIGAME_START_DIALOG=1` (accepted: `1`, `true`, `on`, `yes`; an unrecognised value logs WARN `event=feature_flag_invalid` and stays off). Click the same terminal.

1. Expected: the pre-game dialog (client message 53) appears first, naming the game, "Difficulty 1", with Play available.
2. Press **Play**. Expected: the game window opens (cell method 24).
3. Click the terminal again and press **Cancel**. Expected: no window, `player_cancelled` logged, and the hold clears so you can walk (cell method 30).
4. Click again and **leave the dialog unanswered for 60 s**. Expected: the offer expires; the next launch works. Expiry is checked lazily when the next answer arrives (D-MG19), so a stale `offer_expired` or `stale_offer` after the wait is expected, not a bug.
5. With the dialog open, use ability 1327. Expected: refused with `offer_pending`.
6. Repeat step 1 with ability 1327 as the launcher: D-MG20 sends ability launches through the same path, so the dialog must appear for them too.

**Fail signs:** the dialog appears with the flag off; the player stays frozen after Cancel or after an expiry (the hold must clear: D-MG19 sets it at launch, not when the dialog opens).

After the run, restore the environment to its default.

## Recording results

| Recipe | Needs | Result | Evidence | Date |
| --- | --- | --- | --- | --- |
| R1 | Castle account | not run | | |
| R2 | GM, Debug Area | not run | | |
| R3 | GM, Debug Area | not run | | |
| R4 | GM, Harset | not run | | |
| R5 | GM | not run | | |
| R6 | fresh character, Castle | not run | | |
| R7 | GM, Debug Area | not run | | |
| R8 | server restart (ask) | not run | | |

Update this table, the [README ledger](README.md), and the manual row in [unified-uat.md](../../guides/unified-uat.md) in the same change.
