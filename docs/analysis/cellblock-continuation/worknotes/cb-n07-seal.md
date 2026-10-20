# CB-N07 seal: the barracks escape response

Status: **sealed** by CB-N07s (npc-ai-spawn-advisor, 2026-10-17, static evidence
only, no lab). CB-N07m signs off next; CB-N07a builds from `## Values`,
`## Copy` and `## SQL` only. Packet:
[w4-packets.md](../w4-packets.md#cb-n07s--barracks-response-seal).

## Verdict

**seal.** One gated, fire-once NID Guard chat line when the player enters the
barracks while 687 step 2355 is active. It changes no spawn, aggression,
threat, faction, objective or timer (see Combat impact), A2 holds in code, and
the barracks has three guards and no ambient life, so the beat is not redundant.
A1 and the client half of the hint stay `owner lab` (CB-U11).

## Values

The architect's table is correct in every row. Re-measured from the seeds:

| Fact | Value |
|---|---|
| Region12 | point set 2043, `AreaSet`, `BoundingBox`, `flags` 1 (client-hinted), world 12, four points 2264-2267: x -148.81 to -115.18, z -146.72 to -93.80, y 24.64 to 31.90 |
| Barracks guards (template 24) | spawn 25 `Barracks_Guard1` (-118.41, 24.67, -118.35); 26 `Barracks_Guard2` (-131.48, 24.67, -116.97); 36 `Barracks_Guard3` (-136.38, 24.67, -135.67). All three are inside Region12. |
| Pack spacing | 1-2: 13.14 m; 2-3: 19.3 m; 1-3: 24.96 m. All under the assist radius 26; aggro radius 18. |
| Crate (687 step 2354) | spawn 8 `Cellblock_WoodenCrate` (-130.45, 24.67, -92.07), 1.73 u north of Region12's north edge (z -93.80) |
| 687 flow | 2354 to 2355 by chains 1098, 1099, 1192-1195 (`advance_step 2355`); 2355 objectives 1100-1103; login restore 1104 |
| Existing Region12 chains | none (`git grep Region12 -- db/resources/Content` is empty; the only code hits are comments in `spawner/regions.rs` and `playtest_friction_watch`) |
| Other room barks | 1176 (`teleport_in`, mission 680), 1177 (Region3, mission 681), 1178 (Region5, mission 686), 11010 (Region2, mission 639) |

## Assumptions

| # | Result | Evidence |
|---|---|---|
| A1 | `holds` server side; the client hint is `owner lab` | `load_regions_from_db` (`cell-catalog/.../spawner/regions.rs:212`) loads every `AreaSet` row with its `flags`; Region12 has four points and flags 1, the same shape as Region2 (set 2033), which chain 11010 uses. `resolve_hinted_region` (`cell-methods/.../player/world/mod.rs:73`) fires `fire_enter_region` from the server-known position, and the loader test for 11010 covers the same path. That the 2009 client sends the hint for Region12 is only provable in CB-U11 step 2. |
| A2 | `holds` | `executor/mission.rs:306-316` calls `fire_step_activation_regions` after every real `advance_step` activation (`if activated`). `containing_regions` (`event_dispatch/step_activation/mod.rs:217`) lists client-hinted regions that contain the player's server position, with the 1.5 u tolerance of `is_point_in_region` on every axis. `replay_one` resolves `RegionEnter` through `Chain::is_mission_gated`; chain 11020 qualifies because `step_status` is a mission-state condition. Step 2355 is already active at that point (the replay doc says conditions read post-activation state), so the gate passes. Chains 1098/1099/1192-1195 reach 2355 by `advance_step`, so every crate route triggers the replay. |
| A3 | `holds` | The barracks has no flank objective; the bark touches no AI state (Combat impact). CB-U05 is not needed. |
| A4 | `holds` | The only action is one `npc_bark` at `delay_ms` 0: a chat line, no objective, movement lock or dialog window (`executor/bark.rs` sends one `EntityMethodCall` 28 to the player). |

A2 detail the owner should know. The crate is 1.73 u outside the room, which is
just beyond the 1.5 u tolerance. A player searching from the room side is
inside the replay volume, so the line plays at once when 2355 starts. A player
searching from the north side is outside it, so the line plays on the first step
into the room instead. Both play the line once and both are acceptable. If the
real hint fires first, the once gate keeps the replay from speaking twice (and
the reverse), because `fired_once_chains` is per entity and records the chain
only when its conditions pass (`executor/once_gate.rs`, `chain/mod.rs:75-92`).

Other checks the packet asked for:

- **Rollback.** `ChainEngine` skips a chain with `enabled = false` in
  `resolve_event` (`content-engine/src/chain/mod.rs:456`, `if !chain.enabled ||
  ...`), and `chain/tests.rs:136` and `:534` pin it. Rollback is the one-column
  edit stated in the SQL header.
- **No co-firing bark.** 1176 needs `teleport_in` and mission 680, 1177 needs
  Region3 and mission 681 active, 1178 needs Region5, 11010 needs Region2 and
  639/2117. Chain 11020 needs Region12 and 687/2355. Triggers and gates are
  disjoint, so none can fire on the same entry.
- **Screen cache.** `load_dialog_screen_text` (`spawner/dialogs.rs:170`) reads
  `SELECT screen_id, text FROM resources.dialog_screens` with no dialog filter, so
  a second screen under holder 60160 is cached. The table
  (`Dialogs/Tables/dialog_screens.sql`) has no unique constraint on
  `(dialog_id, index)`; `index` 1 is therefore legal, and 60160 is "never
  displayed" (`dialogs.sql:10864`). No `screen_not_cached` expected.
- **Sequence.** The highest authored screen is 200160 and `setval` is 200160
  (`dialog_screens.sql:28658`). Adding 200170 requires `setval` 200170.
- **Order note for CB-N07a.** At this base `dialog_screens.sql` has the 60160
  row at line 28650 and the `setval` at 28658. W3's 60150 / 96110 row is not here
  yet; the coder works on the W3-INT commit and must still insert directly after
  the line containing `VALUES (60160, 200160,`.

## Combat impact

None. The whole path from trigger to effect:

- `Action::NpcBark` dispatches only to `bark::npc_bark` (`executor/dispatch.rs:318-327`).
- `npc_bark` (`executor/bark.rs:62-160`) reads `dialog_screen_text`, checks
  `is_player`, logs, writes one `player_journal` note, and sends one
  `CellToBaseMsg::EntityMethodCall` with `ON_PLAYER_COMMUNICATION`. It reads no
  NPC and writes no `CellEntity` field except the once-gate set.
- The loader (`content-engine/src/loader/action_bark.rs`) accepts only
  `CHAN_say`, a non-blank speaker and an integer `screen_id`; there is no
  parameter that can name a spawn, faction or target.
- The speaker is a string label. No NPC entity is addressed, so no aggro, assist
  (`generate_threat`), threat entry or `AiState` transition can result.
- The replay path runs only mission-gated chains and never touches rings or gates
  (`step_activation/mod.rs:41-47`).

Pack behaviour is unchanged: guard spacing is under the 26 u assist radius, so
the pack still pulls as one on the first shot, and nothing in this packet
moves a guard.

## Copy

Speaker `NID Guard` (a faction label, not a named character). One line, 74
characters, no new lore:

`Intruder in the barracks! One of the prisoners got loose. Weapons free!`

It agrees with the 687 text (guards in the barracks) and with 11010's tone
("somebody is loose"). It names no character and promises no mechanic.

## SQL

Unchanged from the CB-N07a section of `w4-packets.md` (Steps 1 and 2). Chain
11020, scope `mission` 687, trigger `enter_region` `Castle_Cellblock.Region12`
(`scope` `player`, `once` true), one condition `step_status 687 '2355' eq
active`, one `npc_bark` action `{"screen_id": 200170, "speaker": "NID Guard",
"channel": "say"}` at delay 0. Screen row: `INSERT INTO dialog_screens
(dialog_id, screen_id, text, speaker_id, index) VALUES (60160, 200170, '<Copy
above>', 0, 1);` and `setval('dialog_screens_screen_id_seq', 200170, true)`.
The copy contains no apostrophe, so no quote escaping is needed. The seal found
no correction, so CB-N07a copies the packet's blocks byte for byte.

## IDs

Fresh recheck on 2026-10-17 at base 82ed4cebf, across all 736 local branches
(`git for-each-ref refs/heads`), `git grep` of the chain tables for
`VALUES (11020,` or `(11020,` and of `Content` and `Dialogs` for `200170`:

- **0 hits** for chain 11020 and for screen 200170 anywhere.
- **Control:** the same search for chain 11010 and screen 200160 hits at HEAD
  (`castle_cellblock_chains.sql:3481-3491`, `dialog_screens.sql:28650`) and on
  7 branches, so the search method works.
- The first, broader pattern `VALUES (11020,` also matches
  `texts.sql:2287` (`texts` moniker 11020, `DN_Ms_F03_Agnos_SGU_17FoodSupply`).
  That is a different table and namespace (monikers), not a chain or screen
  clash. CB-N07a must not reuse that pattern as its collision check; use the
  chain-table pattern above.
- Chain id 11020 sits inside the 11000-11499 Cellblock authored block; screen
  200170 is the next free step after 200160 (the highest authored screen).

## CB-U11

Copied from [owner-lab-checklist.md § CB-U11](../owner-lab-checklist.md#cb-u11--barracks-alarm-bark-chain-11020),
with one clarification (step 2) from the A2 detail above. Run after W4-INT, on
any fresh character that reaches mission 687.

1. During 2354 (before the crate search), walk into the barracks
   (`Castle_Cellblock.Region12`, about x -149 to -115, z -147 to -94). No alarm
   line appears.
2. Search the crate at (-130.5, 24.7, -92.1), which sits 1.7 u north of the room.
   If you are in or within 1.5 u of the room when 2355 starts, the line appears
   at once. Otherwise it appears on your first step into the barracks:
   `NID Guard: Intruder in the barracks! One of the prisoners got loose. Weapons free!`
   (chat, say channel), once. Either timing passes.
3. Leave and re-enter during 2355: no second line. Relog during 2355: at most one
   more line.
4. The three guards fight as before: no extra guard, no guard pulled from another
   room, objective counts unchanged.
5. Clear the barracks (687 completes): re-entering gives no line.
6. SigNoz: span `content.npc_bark`, INFO `Content: npc bark` with `chain_id` 11020
   and `screen_id` 200170; no WARN `screen_not_cached`.

Pass: steps 1-5. This is also the check for A1's client half: if step 2 shows no
line on entry and the server log shows no `triggerClientHintedGenericRegion` for
Region12, the client never sends that hint and the beat is dropped by setting
`enabled = false` on chain 11020 (CB-N07 closes as `Done / omitted`).

## Mission sign-off

Verdict: **approve** (CB-N07m, mission-systems-advisor, 2026-10-17, static, no lab).
No edit to Values, Copy or SQL. CB-N07b (Tier 1) is omitted by owner decision;
only the Tier 0 bark (CB-N07a) proceeds.

1. **Gate.** `step_status(687, '2355') = 'active'` is false during 2354 (the
   crate-search step; the key `mission_687_step_2355_status` reads `not_active`),
   true from the `advance_step 2355` of chains 1098/1099 (and the 1192-1195
   variants) until `complete_mission 687` from chain 1103, and `completed`
   afterwards. A walk into the barracks during 2354 fails the condition, so the
   once gate is not consumed and the line still plays on the first entry in 2355.
2. **Suppression.** `once` stops a repeat within one visit and the step gate stops
   it after the fight. A relog during 2355 can give at most one more line, the same
   accepted behaviour as 11010. An abandon and restart of 687 replays the beat once;
   that is intended.
3. **No progress change.** The chain has one `npc_bark` action and no
   `advance_step`, counter, grant or completion. Objectives 1100-1103 gate on
   `entity_dead_tag` and do not read the region or the bark; completion still runs
   through chain 1103 (`counter barracks_kills gte 2` is read before the third
   increment lands, unchanged by this packet). Chain 11020 adds no mission-state write.
4. **Text.** Mission 687 "Aftermath": step 2354 "Search the crate for any useful
   items", step 2355 "Eliminate the guards in the barracks." The line announces
   guards reacting to an escaped prisoner, which agrees with both. Marsh's barks
   (1176 teleport_in/680, 1177 Region3/681, 1178 Region5 with 686 active and 687
   `not_active`) cannot co-fire: 1178 is switched off by 687 being active. It does
   not contradict 11010 ("Somebody is loose", mission 639).
5. **A2.** Acceptable for the beat. Either the replay on step activation (player
   inside the room or within 1.5 u of it) or the first step into Region12 plays the
   line once; neither needs the crate to sit inside the volume. The beat is
   cosmetic, so dropping it via `enabled = false` (CB-U11 step 2 fallback) loses no
   progression.

ID recheck, chain-table pattern. Pattern `VALUES \(11020, '` (and `\(11020,`) over
`db/resources/Content/Seed`, plus `200170` over `Content` and `Dialogs`, at HEAD
and on all 736 local branches: 0 hits. Control `VALUES \(11010, '` hits at HEAD
(`castle_cellblock_chains.sql:3481`). The seal's chain-table pattern is correct
and avoids the `texts.sql` moniker false positive. CB-N07a should use it for its
collision check.
