# CB-N02s relog-restore audit

> **Type:** Reference
> **Audience:** Cellblock coordinator and the CB-N02 implementers
> **Last updated:** 2026-10-16

Static audit of `db/resources/Content/Seed/castle_cellblock_chains.sql` for bindings that a one-shot edge sets and a relog loses. No lab, no code, no build. Nothing here was reproduced live; every repro is on paper from the seed and the engine source.

## Mechanism

- `available_interactions` (per-player dialog-set topics) and an entity's `interaction_type_flags` (set by `set_interaction_type`) live in cell memory. A fresh Castle_CellBlock instance is built on each login, so both reset to the template defaults.
- The restore mechanism is a `player_loaded` chain keyed on the world (`Castle_CellBlock`). Precedents in the seed: 1006/1007 (622), 1045/1046 (640), 1062-1065 (641), 1074 (680), 1104 (687), 1110/1111 (688), 1162.
- `once` is per (cell entity, chain), in memory (`crates/content-engine/src/chain/mod.rs` lines 75-93). It re-arms on relog and world change, so a `once` restore chain fires again after every login.
- `fire_player_loaded` populates the archetype and the mission/step context. `fire_dialog_open` and `fire_dialog_choice` do not populate the archetype.
- `add_dialog_set` has no dedupe and `.first()` wins on a template slot. `remove_dialog_set` removes every copy.
- Nested `mission_accepted` / `mission_completed` dispatch runs mid action list, so removes belong before complete/accept actions.

## Binding table

Core-route list: the branch ledger's `route-baseline.md` is not in this worktree, so the core route is taken as missions 622, 638-641 and 680-688 (the Session A evidence). Anything outside that is `unchecked`.

| Target | Set by | Window the player needs it | Cleared by | Login restore | Verdict |
|---|---|---|---|---|---|
| 638 prisoner dialog topic (dialog set) | 1011/1012 (`enter_region` `Castle_Cellblock.Region2`, gated `not_active`) | 638 steps 2114-2116 | 1018/1019 (`remove_dialog_set` on completion) | 1026/1027 (seed lines ~789-819) | restored |
| `329_CellDoorButton` mask 256 (spawn 22, template 22, base `interaction_type` 0) | 1014/1015 (step 2115 reached) | 638 step 2115 | 1017 | none | **gap A** |
| `ArmYourself_AmbernolVial` mask 1073741824 (spawn 12, template 18, base 0) | 1031 (`enter_region` Region11, gated on step 2117) | 639 step 2145 | not verified | none | **gap B** |
| `ArmYourself_PrisonerRetrievalUnit` drone aggression | 1032 (armed at 2144) | 639 step 2144 | not verified | none | **gap C** (degraded, not stranding) |
| 622 bindings | 622 chains | 622 | per chain | 1006/1007 | restored |
| 640 bindings | 640 chains | 640 | per chain | 1045/1046 | restored |
| 641 bindings | 641 chains | 641 | per chain | 1062-1065 | restored |
| 680 bindings | 680 chains | 680 | per chain | 1074 | restored |
| 687 bindings | 687 chains | 687 | per chain | 1104 | restored |
| 688 ring bindings | 688 chains | 688 | per chain | 1110/1111 | restored |
| Escort Col Marsh follow/reposition (680/686) | 1173/1174 | 686 | space teardown | none (seed comment at ~2952 flags it) | known gap, out of scope (follow state, not a binding; needs an OR across two conditions) |
| `Preparation_ColMarsh` interact (1171, `interact_tag`) | template flags | 680 step 2344 | never | n/a, template-driven | re-armed |
| `HackTheRings_Switch` (spawn 23, template 3 flags 32) | template flags | 688 | never | n/a, template-driven | restored (template default) |
| GuardBody (spawn 15, template 21), FrostBody (spawn 19, template 14) | template flags 0 | loot tables | n/a | n/a | transient |
| Cellblock_WoodenCrate, TerminalX, ArmoryRingSwitch | not traced | unknown | unknown | unknown | unchecked |

The 622/640/641/680/687/688 rows were matched by chain id and trigger type only (each restore chain exists on `player_loaded`); I did not re-derive that each restore covers every bit its setter sets. Treat them as `restored` at the chain-existence level.

## Mission 638 restore design (chains 1026/1027)

1. Is `archetype neq 8` / `eq 8` the same split as 1011/1012? Yes. 1026/1027 use the same archetype-8 split as 1011/1012, and `fire_player_loaded` populates the archetype, so the condition evaluates.
2. Is `once = true` safe? Yes. It is per (cell entity, chain) in memory and re-arms on relog, so the restore fires on every login and only once per instance.
3. Does `mission_status eq active` cover 2114, 2115 and 2116? Yes. The mission is active across all three steps. Chain 1021 needs the topic at 2116, and the restore keeps it for the whole active window.
4. Does 1018/1019's `remove_dialog_set` on completion leave nothing to restore after relog? Yes. After completion `mission_status` is no longer `active`, so 1026/1027 do not re-add the topic, and a fresh instance has no stale set to remove.
5. Is the `329_CellDoorButton` mask a separate gap? Yes. 1026/1027 restore only the dialog topic. The door-button mask (256, set at 2115 by 1014/1015) is not touched, so it is gap A below.

## Sealed children

### CB-N02b — Cell door button mask lost on relog at 638 step 2115

- Fault: `329_CellDoorButton` gets mask 256 only on the 1014/1015 edge at step 2115, so a relog at 2115 loads template flags 0 and the button is inert.
- Paper repro: reach 638 step 2115 (1014/1015 fire, the button is interactable), log out, log in. A fresh instance spawns spawn 22 with template 22 `interaction_type` 0 and nothing re-sets 256. Click the cell door button: no interaction, the player is stranded in the cell.
- Missing chain: trigger `player_loaded` on `Castle_CellBlock`; condition `step_status` 638 `'2115'` `eq` `'active'` (target_id 638, target_key `'2115'`, value `'active'`); action `set_interaction_type` with tag `329_CellDoorButton`, params `{"op":"|","mask":256}`. Mirror the archetype split of 1014/1015 if those are archetype-gated (read them before writing).
- IDs: coordinator allocates.
- Test: extend `crates/cell-content/src/cell/content/chain_replay_tests/mission_638.rs` (currently 346 lines). Mirror the existing 1026/1027 test; assert the mask is set after `player_loaded` at 2115 and not set at 2114 or after completion.

### CB-N02c — Ambernol vial mask lost on relog at 639 step 2145

- Fault: `ArmYourself_AmbernolVial` gets mask 1073741824 only from chain 1031 on `enter_region` Region11 gated on step 2117, so a relog at 2145 loses the bit and the vial cannot be used.
- Paper repro: reach 639 step 2145 (the vial is interactable), log out, log in. Spawn 12 (template 18, `interaction_type` 0) loads with flags 0, and 1031 will not fire again because the player is already past the 2117 region gate. Click the vial: no interaction, mission 639 stalls.
- Missing chain: trigger `player_loaded` on `Castle_CellBlock`; condition `step_status` 639 `'2145'` `eq` `'active'`; action `set_interaction_type` tag `ArmYourself_AmbernolVial`, params `{"op":"|","mask":1073741824}`.
- IDs: coordinator allocates.
- Test: `crates/cell-content/src/cell/content/chain_replay_tests/mission_639.rs` (currently 289 lines). Do not use `mission_639_cover.rs` (635 lines, near the 700 cap).
- Verify before shipping: where the vial bit is cleared (use, or step advance) and that the restore does not outlive that window.

### CB-N02d — Drone aggression not re-armed after relog at 639 step 2144 (degraded, not stranding)

- Fault: chain 1032 arms `ArmYourself_PrisonerRetrievalUnit` aggression at 2144; a fresh instance resets it to NEUTRAL, so the drone is passive after a relog.
- Paper repro: reach 2144 (1032 fires), log out, log in. The drone spawns NEUTRAL and nothing re-arms it. The fight is skipped or the player must reach it unprovoked; the mission can still complete, so this is degraded play and not a strand.
- Missing chain: `player_loaded` on `Castle_CellBlock`, condition `step_status` 639 `'2144'` `eq` `'active'`, and the same aggression action 1032 uses (copy its action row exactly; the action type and tag were not re-read in this audit). Precedent: 1009.
- IDs: coordinator allocates.
- Test: `mission_639.rs` (289 lines), same file as CB-N02c.

## Seal check for CB-N02a

sealed. 1026/1027 are present in the seed and correct: the archetype split matches 1011/1012, `once = true` is safe, and `mission_status eq active` covers 2114-2116. The door-button mask is a separate gap (CB-N02b) and does not block CB-N02a.

## Pilot measurement

- Start: 2026-10-16T14:43:37Z. End: 2026-10-16T14:47:37Z.
- Elapsed: 4 minutes wall time.
- Tokens: not exposed by the harness.
- Lane runs: 0.
