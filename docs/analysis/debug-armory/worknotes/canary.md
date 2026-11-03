# Canary C1: armory-abilities-soldier-0, all rows BLOCKED

Run: `cgbqezffqmcnos` (batch, 2026-11-02 17:12-17:13 UTC), local server `sgw_dr_lab` (service.version 6b50d9131), instance default, account `lab` (GM), fresh character `Cgbqezffqmcnosdj` (SGU Soldier L1). Plan-only twin: `cgbqezdruzmdkm`. Bundles under `%LOCALAPPDATA%\cimmeria-lab\uat-runs\2026-11-02-unknown-<run>`.

## Verdict

One cause, two symptoms. The fresh-character flow is not broken and not slow. The runner waits for an intro dialog that an SGU character never gets on arrival. Soldier-00 BLOCKED before its own setup ran, so the teleport, `/gmgiveallabilities` and the dummy never happened. Every ability row then failed at `place`, because the character did not know the ability.

## Evidence

### Q1: the intro dialog

- `session.rs` `ensure_state`: with `[section.fresh] finish_intro` at its default (`true`, `spec.rs:92`), after `lab_play_character` it waits 15 s for `DialogWin:isVisible()` and returns `fresh intro dialog did not appear within 15 seconds` if it never opens.
- Bundle Soldier-00: play confirmed `world_hud` at 17:12:36.65; `client_wait_for` ran 15 009 ms, 56 polls, `met:false`. The six setup actions before it all passed.
- SigNoz (`service.name=cimmeria-server`, `cimmeria.lab_run=dr-lab`):
  - 17:12:36.664 `SGWPlayer.onClientReady received`, world SGC_W1
  - 17:12:36.683 `aoi.cinematic_hold` `hold_started` `hold_ms=16000`, witness 3
  - 17:12:52.684 `hold_released` `reason=timeout`, `held_ms=16020`, 18 introductions flushed
  - No dialog log line anywhere in 17:12:25-17:13:20.
- 11 s after the release (Soldier-598 `place`, 17:13:03), `visible_screens` was only `SelfStatusWin`, with no DialogWin. The dialog was not just late; it never came.
- Content: SGC_W1's intro dialog 5354 is shown only by chain 3002, on a right-click on `SGCW1_GenHammond` (`db/resources/Content/Seed/sgc_w1_chains.sql`). `first-session.toml` FS-S2/FS-S3 model the same thing: SGU play waits for the hold release, and 5354 needs a Hammond right-click. The auto-shown intro dialog (2982) belongs to the Praxis Castle_CellBlock start.
- So `finish_intro = true` (the default) can never pass for an SGU fresh section, on this server or the colo.

### Q2: hotbar placement

- Soldier-598 `resolve` step: `ability_window: null`, `hotbar_buttons: []`. The client knows the name "Quick Burst" from data, but the character has no ability-window entry and no button for it, so placement cannot bind it ("button 1 does not hold ability 598 after placement"). 714-717 are the same.
- That is the expected state of a fresh L1 Soldier that never received the Soldier-00 grants (`/gmgiveallabilities`, which never ran) and stayed in SGC_W1. The equip step worked (SMG to Bandolier slot 1, confirmed), so the inventory and UI path is fine. The UI state was not the problem: only SelfStatusWin was open, with no prompt.
- Not proven live: that placement succeeds once the grants run. It should, if `/gmgiveallabilities` grants the tree in the ability window. Only a rerun shows it.

## Scope: all 22 generated files

- `armory-abilities-*` (17 files) come from `debug_armory_specs.py` `section_header`. They take their alignment from the archetype. The SGU files get the same BLOCKED. The Praxis ones (`goauld-*` and the other Praxis archetypes) start in Castle_CellBlock, where dialog 2982 opens on its own, so the default `finish_intro = true` is right for them.
- `armory-gear-*` (5 files) come from `debug_armory_gear.py` `SECTION_TAIL`, hard-coded `alignment = "sgu"`. They hit the same BLOCKED.
- Outside this campaign, the hand-written SGU fresh sections `bank.toml`, `vendors.toml`, `crafting.toml` and `debug-area.toml` have no `finish_intro = false` either, so they have the same latent block. They are reported here, not changed.

## Proposed fix (generator only; not applied)

1. `tools/ability_mechanics/debug_armory_specs.py` `section_header`: after `'first = "Uat"',` emit `finish_intro = false` when `alignment == "sgu"` (an SGU start has no auto intro dialog; dialog 5354 needs Hammond).
2. `tools/ability_mechanics/debug_armory_gear.py` `SECTION_TAIL`: add `finish_intro = false` after `first = "Uat"` (always SGU).
3. In `setup_row` (and the gear generator's equivalent first row), add the first-login movie hold as the first setup step, before `.gotolocation`, so the teleport is not typed into the 16 s cinematic hold. Without the intro wait, play returns about 16 s before the release. Either:
   - `{ wait_server = { log = "Cinematic AoI hold: released", target = "aoi.cinematic_hold", fields = { witness_id = "${player_entity_id}" }, timeout_ms = 30000 } }`, the FS-S2 form (check first that `wait_server` in a setup list sees a release that happened before the row started; if it does not, use the other form), or
   - `{ wait_ms = 17000 }`. The hold is a fixed 16 s timeout (`reason=timeout`); this is the hub-pass and haiku-lab-uat rule.
4. Regenerate the 22 files (`python tools/ability_mechanics/debug_armory.py specs abilities|gear --out-dir docs/guides/uat-specs`). Update `test_debug_armory_specs.py`: it asserts `setup[3]` is `/gmgiveallabilities`, so the index shifts by 1 with step 3. Add an assert that the SGU headers carry `finish_intro = false` and the Praxis ones do not.

Optional, separate: set `sequential = true` in the generated sections. A failed `-00` setup row then stops the dependent rows instead of grading them as N BLOCKED placement failures that hide the cause. That changes semantics (one bad ability would stop the rest), so it is an owner call. An alternative is a runner rule: "a BLOCKED setup row (`-00`) blocks the section".

Longer term (runner, not this campaign): make the default alignment-aware (true only for `praxis`). Or downgrade "no dialog in 15 s" to a note when the SGU hold-release fires. Then hand-written SGU sections do not need the flag.

## Rerun

Worth it once fixes 1 and 3 are in the regenerated `armory-abilities-soldier-0.toml`. A quick check without regenerating: hand-add `finish_intro = false` plus `{ wait_ms = 17000 }` to a scratch copy (not the committed spec) and point `specs_dir` at it. A rerun now, unchanged, BLOCKs the same way.

## Retry after DR-07d (run `cgbqffluzdhtlr`, 2026-11-02 17:19-17:22 UTC)

Fresh character `Cgbqffluzdhtlrdj` (SGU Soldier L1), account `lab` (GM, access_level 2). The intro block is gone. Soldier-00 BLOCKED on `uat_dummy` ("no reply to .dummy hostile within 5000 ms"); 598/717/715/716/714 BLOCKED on `uat_cooldowns_reset` the same way.

### Verdict

Not a server, GM or timing problem. Every command ran and replied; the runner could not see the replies. `client_ui_state` reads the chat tail from the hard-coded tab `Inst1Chat_TabOutput` (`crates/lab/src/supervisor/flows/ui_state.rs:182`). After a logout and re-entry in the same client process, the client builds new chat tabs (`Inst5`..`Inst9`) inside the same `Inst1ChatWin`, and all output goes to the newest selected tab. `Inst1Chat_TabOutput` stays empty, so `chat_tail` is `[]` before and after every line, `new_lines` is empty, and every chat-reply wait times out. The fresh-section flow always does `lab_logout` then create then play, so every fresh section on a long-lived client hits this.

### Evidence

- SigNoz (`cimmeria.lab_run=dr-lab`), all `GM .-console command accepted` with `access_level=2`, each followed within 1 ms by `GM console feedback sent to client`:
  - 17:20:02.96 `gotolocation` (cross-space transfer enqueued); 17:20:13.73 `gmGiveAllAbilities: granted 72 abilities`
  - 17:20:16.18 `dummy` → `lab_dummy_spawned` id 100679 (SGC Jaffa, hostile), feedback "dummy [100679] placed: ..."
  - 17:20:44.66 / 17:21:09.16 / 17:21:30.48 / 17:21:57.04 / 17:22:18.96 `cooldowns reset 598/717/715/716/714`, each with its feedback line
  - `.bug` anchors from Soldier-598 on are accepted and answered too (bookmarks recorded).
- Bundle: `uat_dummy` `result.new_lines: []`; every row's `anchor.host_seen_ms` is null (the anchor reader uses the same chat tail, so it is blind too).
- Live probe (lease taken and released, same character, after one more relog): `client_ui_state chat_lines=150` returns `chat_tail: []` while `Inst1ChatWin` is visible. `client_chat_log since_seq=0` holds every reply above, tagged `tabs: ["Inst9Chat_TabOutput"]`. Lua: `Inst1Chat_TabOutput` 0 items; `Inst9Chat_TabOutput` 2 items (Welcome, New Mission), parent `Inst9Chat_Tab` visible; `Inst8Chat_Tab` hidden. All `InstNChat_Tab` sit under `Inst1Chat_Holder` > `Inst1ChatWin`.
- The grants did land: Soldier-598 `resolve` now shows `ability_window {tree 1, index 1}`, placement on button 1 and a `useAbility` with `cast_started`. Q2 of the first canary is answered: placement works once `/gmgiveallabilities` runs.
- Minor, separate: Soldier-00's anchor `.bug uat Soldier-00` (17:19:36, about 0.3 s after world entry) never reached the server, since the row's anchor goes out before the setup's `wait_ms 17000`. It costs only the bookmark.

### Proposed fix (runner/supervisor; not applied)

1. `crates/lab/src/supervisor/flows/ui_state.rs` `sec("chat", ...)`: stop hard-coding `Inst1Chat_TabOutput`. Pick the output of the selected tab: scan `i = 1..64` for `_G["Inst"..i.."Chat_Tab"]` that is non-nil and `isVisible()`, and take `_G["Inst"..i.."Chat_TabOutput"]`. If none is visible, take the highest-index `InstNChat_TabOutput` with items, then `Inst1Chat_TabOutput`. Emit the chosen tab name as a tagged line (for example `chat_tab`) and return it as `chat_tab` beside `chat_tail`, so a bundle shows which tab it read.
2. Test (the file is over 600 lines, so put it in a sibling `ui_state_chat_tests.rs`): an mlua fixture with `Inst1Chat_TabOutput` empty, `Inst9Chat_Tab` visible and its output holding the reply. Assert `chat_tail` holds the reply and `chat_tab = "Inst9Chat_TabOutput"`. It fails with the current hard-coded `Inst1`.
3. Optional, more robust (runner): make `read_chat_of` (`crates/lab/src/uat/runner/actions.rs:432`) use `client_chat_log` with a `since_seq` baseline instead of a text-overlap diff of the tail. The ring survives relogs (`chat_log_relog_tests.rs`), has sequence numbers, and does not lose lines when 150 identical-looking lines scroll. That is a bigger change (lease-gated tool, checks.rs `CHAT_READ_TOOL` users), so it is a follow-up; fix 1 alone unblocks the canary.
4. Optional, spec: move the 17 s hold wait ahead of the anchor, or have the runner send the anchor after the first setup wait, so `-00` gets its bookmark. A cosmetic fix only.

No spec or generator change is needed for the no-reply BLOCKs. Rerun C1 unchanged once fix 1 is in a lab build.
## C2 BLOCKED at LoginWin (run `cgbqfzznxpvmst`, 2026-11-02 17:42 UTC)

Section `armory-gear-weapons-1`, run straight after C1 on the same client (pid 62532). Every row BLOCKED in 15 ms: `lab_characters` "the client is not at character select; visible_screens [LoginWin]". `_session_end` FAILed on the `Client entities cleaned up` wait.

### Cause

The supervisor's `login_state` goes stale after `lab_disconnect`. `disconnect_flow` (`crates/lab/src/supervisor/flows/login.rs:439`) clicks Back to the login screen but never calls `set_login_state`, so the state stays `CharSelect` (set by the logout before it). `LoginState` (`crates/lab/src/supervisor/mod.rs:203`) has no "at the login screen" value at all.

- C1's `_session_end` ran `lab_logout` then `lab_disconnect`: the client was at LoginWin with `login_state = character_select`.
- C2's `ensure_state` (`crates/lab/src/uat/runner/session.rs:109`) trusts `login_state`: `character_select` skips `lab_login` (line 139) and goes straight to `lab_characters`, which reads the UI and fails. Each row repeats this, so every row BLOCKs. Bundle: `lab_client_status` says `login_state: character_select` while the same row's `lab_characters` sees `LoginWin`.
- C2's `end_run_session` (line 335) also trusts the stale state, so it built a session-end row. `lab_disconnect` returned `already: true, at: login` (nothing sent), and the 30 s wait for `Client entities cleaned up` could not succeed: FAIL.
- After the diagnosis (17:47 UTC), `lab_client_status` still said `character_select`, and `lab_login` started from `reach_login screen: Login`.

### Recurrence

Yes. It happens between every pair of `lab_uat_run` calls on a long-lived client: every run that reaches its end (and has the server lab MCP) disconnects to LoginWin with the state left at `character_select`, so the next call BLOCKs every row. Sections inside one `lab_uat_run` call are not affected (the session end runs once, after the last row). Not tried live: putting several files in one call.

### Workaround (no code)

Before each `lab_uat_run` after the first: take the lease, call `lab_login` (about 7.5 s; it starts from any startup screen and sets `character_select` correctly), release the lease. `lab_client_restart` also works (`lifecycle.rs:128` resets the state to `not_started`, so `ensure_state` logs in), but it costs a client boot. Applied once at 17:47 UTC: `lab_login` confirmed `character_select`, lease released, `login_state = character_select`, client pid 62532, at character select (4 SGU Soldier slots in use: Celltest and three fresh `Cgbq*` characters).

### Proposed fix (supervisor; not applied)

1. `crates/lab/src/supervisor/mod.rs` `LoginState`: add `LoginScreen` => `"login_screen"`, documented "at the login screen after `lab_disconnect`; the account is logged off". Check consumers of the string: `ensure_state` (needs no change: anything not `character_select`/`in_world` runs `lab_login`), `end_run_session` (`login_screen` is already a `not_logged_in` skip), `uat/runner/players.rs:195`, `uat/runner/fixture.rs:158`, `uat/calibrate/drive.rs:246`. `events/keeper.rs`: treat it like `NotStarted` (it ticks), and add it to the second list in `the_keeper_skips_only_known_busy_states`.
2. `crates/lab/src/supervisor/flows/login.rs` `disconnect_flow`: call `self.set_login_state(LoginState::LoginScreen).await` after the `login_screen` wait, and on the `already: true` early return too (the client is at LoginWin either way).
3. Tests in `crates/lab/src/supervisor/flows/login_tests.rs`: (a) a fake where `LOGIN_WIN` is visible, state preset to `CharSelect`; after `disconnect_flow`, `lab_client_status` `login_state` is `login_screen`. (b) The same for the click-Back path. Each fails with the current code (the state stays `character_select`). Runner test, optional: a fake invoker whose status says `login_screen` makes `ensure_state` call `lab_login` before `lab_characters`.
4. Defense in depth, optional (runner): in `ensure_state`, when `lab_characters` or `lab_play_character` fails with "not at character select" and the reported state shows `LoginWin`, run `lab_login` once and retry. That covers any other path that leaves the state stale (a server kick, a client-side timeout to the login screen).

Rerun C2 unchanged after the workaround; no spec change is needed for this BLOCK.

## C2 rerun: client_target on the dummy fails (run `cgbqgcoconzfan`, 2026-11-02 17:45 UTC)

Section `armory-gear-weapons-1` rows GW-3128, GW-17 and GW-5680, fresh character `Cgbqgcoconzfanij` (SGU Soldier L1), after the `lab_login` workaround. GW-3128 FAIL (`read_mouse_over` timed out at pixel [333,391]), GW-5680 FAIL (`hover_timeout`, 5 pixels all `hover: nothing`), GW-17 UNVERIFIED (its SigNoz clause waits for attestation, as expected).

### Cause

The bag is still open. `@item_action equip` opens `InventoryWin` with B (`native_steps`: `open "InventoryWin" (B)`, then a right-click on `Inventory_Slot2`) and leaves it open. The gear row never closes it, so `@target` hovers a CEGUI window instead of the dummy.

- Both final screenshots (`GW-3128/final.png`, `GW-5680/final.png`): the bag sits at about x 318-705, y 233-525 of the 1024x768 client. The Jaffa dummy stands at its left edge, mostly behind it. The projected target pixel `[333,391]` (the same in both rows: `.dummy` puts the dummy 3 m from the player, `yaw_deg 180`, and the player never moves) is inside the bag. The fallback pixels at x 511, y 390-597 are the bag and the avatar's back.
- GW-17 passed `client_target` only because the tool's own view retry (`yaw_counts 250`) swung the dummy to `[124,503]`, left of the bag. Its `world_click` `before.windows` lists `InventoryWin` as open. That retry is luck, not a fix.
- Not the camera hiding the target behind the avatar (the shops SD-1 failure). The avatar covers only the bottom centre; the dummy is off to the left, 3 m away, in view.
- Not a missing teleport: the gear spec never leaves SGC_W1 (no `.gotolocation`; the HUD says `SGC_W1`), but the dummy spawns next to the player in any world, so this does not cause the failure. The section's "Debug Area" name is cosmetic here.

### The abilities rows do not target at all

Soldier-598 (PASS in `cgbqfvxipvwymg`) does not use another targeting mechanism; it uses none. The row is gear (`@item_action equip`, which leaves the bag open too), `@use_ability place`, `@cooldowns_reset`, then `@use_ability press = "key"`. The hotbar key sends `useAbility` with `target_id 0` (`client_target_inferred: true`); `player_after.target` is null. An open bag does not matter to a key press. Nothing in it can be copied for a right-click on an entity, which needs a clear line to the dummy.

### Second, smaller finding: the clear_effects teardown

`@clear_effects` fails in every gear row ("no rendered entity matches" for the player's own name), so every row gets a teardown flag. The abilities generator emits it with `optional = true`; the gear generator does not.

### Proposed fix (generator only; not applied)

In `tools/ability_mechanics/debug_armory_gear.py` `weapon_row`:

1. Setup: after the `@item_action` line, close the bag:
   `f"  {_tool('@window_click', [('target', tstr('InventoryWin__auto_closebutton__'))], 'close-bag')},"`
   (`client_window_click` takes CEGUI auto-child names; the recorder already treats this button as the bag's close, `emit_click_tests.rs:131`).
2. Step: before `@target`, face the dummy with the SD-1 camera recipe:
   `f"  {_tool('@camera', [('face_entity_id', tstr('${dummy_id}')), ('zoom_to', '400')], 'face')},"`
   then `f"  {_tool('@camera', [('yaw_counts', '-300'), ('pitch_deg', '-15')], 'offset')},"`. This moves the dummy off the avatar's back and keeps it clear of any other open window. The `"${dummy_id}"` string becomes a number, as `@target`'s `entity_id` already does.
3. Teardown: give `@clear_effects` `optional = true`, like the abilities rows. `_tool` has no `optional`, so write the line by hand: `'  { tool = "@clear_effects", args = { name = "${character}" }, optional = true },'`.
4. Tests (`tools/ability_mechanics/test_debug_armory_gear.py`): in a weapon row, assert that `InventoryWin__auto_closebutton__` comes after `@item_action` and before `@target`, that `face_entity_id` comes before `@target`, and that the `@clear_effects` line has `optional = true`. Regenerate with `python tools/ability_mechanics/debug_armory.py specs gear --out-dir docs/guides/uat-specs`.

Fix 1 alone should clear both FAILs (GW-17 shows that the target pixel works once it is not under the bag). Fix 2 is a guard for camera states where the dummy lands behind the avatar. Neither has been tried live: this diagnosis is from the bundle and screenshots only, and the client was not driven.

### The armor file

`armory-gear-armor-1` does not have this failure: its rows never target or click in the world (`@item_action equip`, `@window_read kind = "character"`, `@screenshot`). The bag stays open there too, so the human clause's screenshot can show the bag over the character sheet. Closing it with the same `@window_click` line after the equip in `armor_row` keeps the screenshot readable. Optional. Armor rows have no `@clear_effects`, so fix 3 does not apply to them.

## C1b: `armed` fails on 598/716/717 because it compares a design id with an instance id

**Run:** `uat-runs/2026-11-02-unknown-cgbqhgxhavomlo`, section `armory-abilities-soldier-0`. Diagnosis from the row bundles and the code only; the lab was not driven.

### Cause

The DR-07h `armed` clause expects the **item design id** (`rep` in `debug_armory_specs.py` `ability_row`, `item_id` in `debug_armory_gear.py` `weapon_row`). `@player_state` `/ammo/weapon_item_id` is `getItemIDForSlot(Container.Bandolier, active_slot)` (`crates/lab/src/supervisor/ui/player_state.rs`). That native returns the **item instance id** (the inventory row), not the design. The client has no native that returns an item's design id, only its name, icon, quality and tech competency per slot (`.claude/agent-memory/main-session/reference_client_action_bar_events.md:14`).

The bundles show the right weapon was active:

| Row | Expected (design) | Observed | Equip diff (`@item_action` result) |
|---|---|---|---|
| Soldier-598 | 21 | 10347 | `SGHC 6 SMG`, item_id 10347, Main slot 2 → Bandolier slot 1 |
| Soldier-716 | 21 | 10347 | 10348 out of slot 1, 10347 in |
| Soldier-717 | 3145 | 10348 | 10347 out of slot 1, 10348 in |

So each row reset slot 1, equipped its weapon there and F1 made it active; only the expectation is wrong. It is not a number-vs-string problem: `loose_eq` (`crates/lab/src/uat/clause.rs:90`) compares numbers and numeric strings alike. Soldier-714 and 715 pass because they are ungated rows with no `armed` clause.

The DA-U39 rows in `docs/guides/uat-specs/debug-area.toml` (lines 1499-1506 and 1773-1780, `value = 55`) use the same pointer with a design id. They will fail the same way unless 55 happens to be the instance id. Flag only; not part of this fix.

### Fix (generator only; not applied)

Compare the active slot's **name** with the name the row equips. The equip already selects the weapon by that name, so the check is as strong as the setup. Keep the clause required.

1. `tools/ability_mechanics/debug_armory_specs.py` lines 181-186, in `ability_row`:
   - `text = {tstr(f"the active bandolier weapon is {items[rep]['name']} before the press")}`
   - `pointer = "/ammo/weapon"`
   - `value = {tstr(items[rep]['name'])}`
2. `tools/ability_mechanics/debug_armory_gear.py` lines 141-146, in `weapon_row`: the same, with `name` (the function's own argument): `text = {tstr(f'the active bandolier weapon is {name} before the press')}`, `pointer = "/ammo/weapon"`, `value = {tstr(name)}`. Fix the comment at line 137 if it says "item".
3. Tests:
   - `tools/ability_mechanics/test_debug_armory_specs.py` lines 50 and 52: `pointer` is `/ammo/weapon` and `value` is `"Pistol"`.
   - `tools/ability_mechanics/test_debug_armory_gear.py` lines 98-99: `pointer` is `/ammo/weapon` and `value` is `name` (`"Cobra Staff"`, `"Plain Club"`).
   - Both fail when the change is reverted.
4. Regenerate (the TOMLs stay LF):
   - `python tools/ability_mechanics/debug_armory.py specs abilities --out-dir docs/guides/uat-specs`
   - `python tools/ability_mechanics/debug_armory.py specs gear --out-dir docs/guides/uat-specs`
   - Then run the lane spec guard: `committed_specs_parse_and_validate` and `every_committed_spec_tag_is_seeded`.

**Alternatives rejected:**

| Alternative | Why not |
|---|---|
| Make `armed` informational (`required = false`) | It would hide a wrong active weapon. That is the failure DR-07h exists to name. |
| Capture the instance id from the equip result's `slot_changes` | The Bandolier entry's position in the array is not fixed, so the capture needs a lab-crate change. |
| Read the design id from the client | There is no native for it. |

**Name collisions:** two designs with the same name would both pass. The equip-by-name step has the same limit, so the clause adds no new blind spot. A follow-up `/ammo/active_slot eq 1` clause would also pin the slot. It is optional.

**Untested:** none of this has been run live. The next canary run will confirm that `/ammo/weapon` returns the catalog name exactly (`getNameForSlot`, for example `SGHC 6 SMG`).
## 2026-11-02: soldier-1 degradation, the client crashes on every runner logout

**Run:** `armory-abilities-soldier-1`, run dir `uat-runs/2026-11-02-unknown-cgbqjzujpscktw`, 26 rows in 6 `lab_uat_run` calls with `end_session:false`. Result: PASS 9, FAIL 1, BLOCKED 16.

**Cause:** SGW.exe crashes, and every crash comes from a `lab_logout`. The four crash zips from today (`CrashDumps/SGW_BETA_11.02.2026_19.38.04`, `19.46.17`, `19.49.08`, `19.51.01`) are the same crash: `ACCESS_VIOLATION` at `SGW.exe!0x00EAE5C6`, with error history `Target .\Src\SGWController_Player.cpp(534)` and `DeltaTime > 0 .\Src\UnGame.cpp(2100)`. Each one comes 2 to 5 s after the front-end GUI reloads (`Interface.log` restarts with `Login.layout`), so after `/logout` is sent and before character select shows. This is the logout crash already tracked in #1680. Four of today's five logouts crashed: soldier-0's `_session_end` at 19:38, and the batch starts at 19:46, 19:49 and 19:50:55. The one at 19:43 survived. The Windows event log has no SGW entries, because UE3's own handler catches the fault. The screensaver and sleep are not involved: the crashes come within seconds of a logout, on an active client.

**Why the runner logs out at all:** each `lab_uat_run` call is a new runner, so `in_world_as` starts as `None`. In `crates/lab/src/uat/runner/session.rs:171-188`, `ensure_state` then assumes the client is in world as `req.lab_character` (the lab-account character), compares that with `want` (the section's fresh character, `Cgbqjzujpscktwzf`), finds no match and runs `lab_logout` (line 187). After that it plays the same character again. So every batch boundary makes one logout that does nothing useful, and that logout is what crashes the client.

**How the runner handles a dead bridge:** it does not. `lab_client_status` still reports `login_state: in_world`, with `running: true`, while `heartbeat.state` is `unreachable` (rows 1889, 1485). Every later row runs `ensure_state`, which calls `lab_logout` again. That call fails after 2 to 56 s (`connect to bridge at 127.0.0.1:8770`, or `-32603 dispatch timeout` while WER writes the dump), and the row is BLOCKED. This repeats until the leased watchdog relaunches the client (pid 43032, then 68496, 48652 and 480). Rows 818, 852, 1889, 2143, 1485, 1456 and 1486 were lost this way.

**Separate issue, not the bridge:** the `client_use_ability denied: ignored` BLOCKs (2419, 808, 2103, 1882, 2102, 724, 1885, 2105, 1487) also happen on a healthy client between PASS rows (pid 43032, 19:44 to 19:45). They follow the equip steps (`client_window_click`/`client_input_key` posted_only). That points at the press landing while the weapon swap is still running, or at a cooldown or GCD. It needs its own look.

**Cheapest mitigation (no code change, untested):** on every continuation batch, pass `character: "<fresh name>"` to `lab_uat_run` (here `Cgbqjzujpscktwzf`). That sets `req.lab_character == want`, so `session.rs:180` returns without logging out. Running the file in one `lab_uat_run` call avoids the boundary logouts too, but the `_session_end` logout can still crash. That one is harmless at the end of a run.

**Proposed code fix:**

1. `crates/lab/src/uat/runner/session.rs:171-188` (`ensure_state`). When `in_world_as` is `None`, read the name of the character in world before deciding to log out. Use the same name read that `lab_ensure_in_world` uses (`client_player_state`, or `server_sessions` for the lab account). If the name matches `want`, set `in_world_as` and return `Ok(())`. Log out only on a real mismatch. Test: a FakeServer that is in world as the fresh character, with `in_world_as: None`, makes no `lab_logout` call (`fixture_session_tests.rs`). Reverting the change must make the test fail.
2. `crates/lab/src/uat/runner/session.rs:137-161`. Treat `heartbeat.state == "unreachable"`, or a setup error that contains `connect to bridge` or `dispatch timeout`, as a dead client. Call `lab_client_restart` once for the row, wait for the heartbeat, set `in_world_as = None` and `login = "not_started"`, then take the normal `lab_login` and play path. Never call `lab_logout` on an unreachable bridge. This turns the 2 to 56 s logout burns into a single restart, about 15 s, and the rows run instead of BLOCKing.
3. Supervisor status, the `login_state` reporter behind `lab_client_status`: report `unresponsive`, not the cached `in_world`, while the heartbeat is unreachable. That way 2. also covers callers outside the runner.
4. Optional, still a hypothesis: the `Target` assertion suggests the crash fires when the controller still holds a target during the front-end teardown. Clearing the target before `/logout` (Escape, or a native clear-target) in `crates/lab/src/supervisor/flows/login.rs:412-421` (`logout_flow`) might stop the crash itself. That needs a live A/B over about 10 logouts before anyone relies on it.

**Lab state after diagnosis:** the client was restarted under a short lease and the lease was released. Server pid 45256 and the database were not touched.
## 2026-11-02: soldier-1 rerun, the `denied: ignored` BLOCKs are ground-targeted abilities

**Run:** `armory-abilities-soldier-1`, run dir `uat-runs/2026-11-02-unknown-cgbqkzlpgajqrl`. Result: PASS 5 (00, 818, 2143, 1456, 1486), FAIL 2 (852, `_session_end`), BLOCKED 11 (2419, 808, 2103, 1882, 2102, 724, 1885, 1889, 1485, 2105, 1487).

**Cause:** every BLOCKED row is an ability with `target_type_id = 3` (TargetGround) and `TCM_AERadius` in `resources.abilities`. Every row that pressed fine has target type 1 or 2: 818, 2143 and 1486 are `TCM_AECone`, and 1456 is a self buff. On a ground ability the client does not send anything on the key press. `AbilitySet_EmitUseAbilityOrGroundTarget` (0x00d2ae40) branches to `AbilitySet_ActivateGroundTargetReticle` (0x00dea330) and waits for a ground click before it sends `useAbilityOnGroundTarget` (cell method 69). See `docs/reverse-engineering/findings/ability-resolution-pipeline.md`. `client_use_ability` presses the key, sees no `net.out` and no cast in 2.5 s, and returns `denied: ignored`. The generator puts that call in setup (`place`), so the row is BLOCKED. Row 852 (High Explosive Mortar, also ground) got a `cast_started` from the reticle's sequence with `sent: false` on the place press, and then its step press was denied. It is the same cause, reported as a FAIL. The two SK37 rows that BLOCKED (808 Cover Denial, 1487 Penetrating Barrage) are ground-targeted too. The weapon swap, the GCD, the cooldown (`was_on_cooldown: false` everywhere), the hotbar binding and the open windows are not the cause. Placement and the key binding resolved on every row, and the PASS rows ran the same equip steps.

**Live confirmation (lease `dr-lab-diag`, about 20:18Z):** after `lab_ensure_in_world` (the client was up but in `login_state: failed` after the logout crash), `client_use_ability {724, place, press=key}` returned `denied: ignored` again. The screenshot showed the ground decal ring around the avatar. A left `client_world_click` at client point (-87000, 24600, 663), about 8 m from the player, then made the server log `abilities warmup_complete ability_id=724 ... warmup_secs=6.0` at 20:19:04Z. That is the press plus the ground click being accepted.

**Proposed fix (not applied):**

1. Runner, `crates/lab/src/supervisor/combat/use_ability/mod.rs` `use_ability_flow`, after the press at lines 262-264. Add an optional `ground` point to `UseAbilityRequest` (server or client space, as `client_world_click`'s `point` takes it). When it is set, left-click that point through the existing world-click path before the observe loop at line 307, and record the click as a `ground_confirm` step. An alternative needs no argument: when the press shows no send and the ability's target type is 3, click a point about 8 m ahead of the avatar. Test: a fake ground ability whose `net.out useAbilityOnGroundTarget` appears only after the click. It must pass with the change and come back `denied` when the change is reverted.
2. Generator, `tools/ability_mechanics/debug_armory_specs.py`. At lines 138 and 153, when `ability["target_type"] == TARGET_GROUND` (line 46), pass `ground = { x = -87000, y = 24600, z = 663, space = "client" }` on both `@use_ability` calls. That point is the Debug Area plaza where the setup row places the player. `toml_value` (line 57) stringifies dicts, so `table()` needs a nested-table case. A generator-only fix (add a `@world_click` left after each press) does not work: the press action itself fails first, so the row would still BLOCK, or FAIL in the step, unless both presses were made optional. That would hide real failures.
3. Check `max_range`: 1485 and 1487 have `max_range = 0` in the seed, so an 8 m ground point may be out of range for them. Watch for a refusal on those two rows after the fix.

**Client state:** in world as `Cgbqkzlpgajqrlzf` in the Debug Area (1300). The lease was released. Server and database were not touched. A mortar from the probe may have hit nearby Jaffa.