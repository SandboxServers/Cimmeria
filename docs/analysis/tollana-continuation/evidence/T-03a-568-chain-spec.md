# T-03a: mission 568 "Finding Tollan Technology" chain spec

Status: SEALED for the packet-coder, with the three BLOCKING gates in section 9 (T-02 claims and one owner call). Author: mission-systems-advisor, 2026-10-17. Worktree: `zone-tollana`. Output file the coder creates: `db/resources/Content/Seed/tollana_568_chains.sql`.

Labels: ORIGINAL_DATA = a row that already ships in the seeds and must not change. CIMMERIA_RECONSTRUCTION = a row the coder adds; the original Atrea script for 568 is absent (`missions.script_name` is NULL) and no chain existed in the 2009 data we hold, so every chain, bind choice and prop below is reconstruction.

## 1. Verdicts

| Id | Verdict | One-line reason |
|---|---|---|
| D-TW1 | CONFIRMED | Item-less pilot. No `add_item`, `remove_item`, `grant_xp`, `grant_cash` or `set_interaction_type` anywhere in the file. `remove_item` is async and `HasItem` is not DB-authorable. The item-backed variant waits for HA-1360a. |
| D-TW2 | CONFIRMED, with 2153 | Accept on Tollana from Copplemann via dialog 2153 (zero buttons), accept on ANY close. 2150 draws More Info plus Accept (ambiguous). 2151 has a clean Accept but its speaker 959 is off-world. 2153 speaker 1100 is Captain Copplemann. Wart: Decline and the X also accept (section 8, W1). |
| D-TW3 | MODIFIED | (a) Core 4211 advances 1964 to 1965 to 1966 in one click, so the player never needs a second Core click. (b) Optional 2312 cannot stay uncredited: `advance_step` force-completes every non-completed objective of the OLD step, optional included. So 2312 is credited by the Nest (4213) when clicked in step 1964 or 1965, or force-credited when the Core advances past step 1965. (c) Step 1964 completes on the first click of ANY of the three props. (d) `step_enabled` / `is_enabled` flags are untouched; no Rust code filters on them (grep of `crates/**/*.rs` finds none), UAT confirms. |

## 2. Fixed data (ORIGINAL_DATA, do not edit)

- `missions.sql:1459`: mission 568, level 6, `can_abandon`/`can_fail`/`can_repeat_on_fail` true, `num_repeats` 1, `reward_naq` 0, `reward_xp` 0, `is_story` true, `script_name` NULL.
- Steps: 1963 (index 0, objective 2309, "Report to Captain Copplemann"), 1964 (index 1, objective 2310, "Search the nearby Repair Facility"), 1965 (index 2, objectives 2311 required and 2312 OPTIONAL), 1966 (index 3, objective 4491, "Return the collected pieces ... to Copplemann"). All `step_enabled`/`is_enabled` false.
- `dialog_set_maps` (set 545, topic "Finding Tollan Technology"): 2656 to dialog 2150 (flags 8388608); 2657 to 2151 (0); 4959 to 2153 (16777216); 6393 to 2154 (33554432); 6392 to 4211 (1073741824); 4958 to 4213 (1073741824); 4960 to 4214 (1073741824). All dialogs `DUIST_DefaultDialog` except 2150 (Blurb).
- Dialogs 2153 (event_set 548) and 2154 (event_set 550) are zero-button, speaker 1100 on the Copplemann screens (2153: 89683-89687; 2154: 89693-89695). 4211/4213/4214 are zero-button, speaker 0 (screens 89697, 89691, 89690). `accepts_mission_id` is NULL on all.
- `mission_status` reads FAILED as `not_active`. After one completion `repeats` is 1, not above `num_repeats` 1, so the offer guard WOULD allow a re-accept. 568 is one-shot only because chain 9001 and 9004 gate on `not_active` and a completed mission reads `completed`.

## 3. Ids this packet uses

| Namespace | Ids | Status |
|---|---|---|
| content_chain | 9001-9018 (inside the Tollana reserve 9000-9799; 9001 already CLAIMED in `tollana-id-register.csv`) | CLAIMED 9001; 9002-9018 coder must add register rows (CIMMERIA_RECONSTRUCTION) |
| spawn tag | `Tollana_Copplemann`, `Tollana_FoodUnit_Core`, `Tollana_FoodUnit_Nest`, `Tollana_FoodUnit_Fungus` | Copplemann CLAIMED by T-02 (template 420, spawn 19001); the three props PROPOSED |
| entity_template | 420 Copplemann (T-02, CLAIMED), 421 shared food-unit prop (PROPOSED) | 421 must be claimed by T-02 |
| spawnlist | 19001 Copplemann (CLAIMED), 19002 Core, 19003 Nest, 19004 Fungus (PROPOSED) | positions UNPLACED (T-02) |
| dialog_set slot | 420 (the template id; one slot, one live bind at a time) | |

Template 420 requirements (T-02 writes the row): `interaction_type` 0, no `event_set_id` inherited from template 48 (bindings 6503/6504 belong to Harset mission 742 and must not apply), `has_dynamic_properties` true, `class 'mob'`. It MUST NOT reuse template 48. Template 421 requirements: model on Dakara template 445 (`entity_templates_dakara_e1.sql:129`): `flags 4`, `interaction_type 1073741824`, `class 'being'`, `has_dynamic_properties` true, a client-present static mesh T-02 chooses, `name` one of the three unit names. One template serves all three spawns; the tag is what separates them.

## 4. Chain set

Common column values, unless a table says otherwise: `content_chains` = `(id, description, 'mission', 568, true, 0)`; every trigger `scope 'player'`, `once false`, `sort_order 0`; every action `delay_ms 0`, `sort_order` as listed. Conditions are `(chain, type, target_id, target_key, operator, value, sort_order)`. World id is 19. All 18 chains are CIMMERIA_RECONSTRUCTION. Descriptions are free text but must start with `568 - `.

State vocabulary used below: S0 = `mission_status not_active`; S1963/S1964/S1965/S1966 = the step is active; SDone = `completed`. Reachable states are S0, S1964, S1965, S1966, SDone (1963 is entered and left inside the accept chain).

### 4.1 Copplemann click chains (trigger `interact_tag` `Tollana_Copplemann`)

Mutually disjoint and exhaustive over S0 / S1964 / S1965 / S1966 / SDone, where SDone matches NO chain (section 8, W2).

Chain 9001 offer
| Table | Row values |
|---|---|
| trigger | `interact_tag`, `Tollana_Copplemann` |
| condition 0 | `mission_status`, 568, NULL, `eq`, `not_active` |
| action 0 | `display_dialog`, 2153, NULL, `{}` |

Chain 9002 in progress (steps 1964 and 1965)
| Table | Row values |
|---|---|
| trigger | `interact_tag`, `Tollana_Copplemann` |
| condition 0 | `mission_status`, 568, NULL, `eq`, `active` |
| condition 1 | `step_status`, 568, `1966`, `neq`, `active` |
| action 0 | `display_dialog`, 2153, NULL, `{}` |

Chain 9003 turn-in (step 1966)
| Table | Row values |
|---|---|
| trigger | `interact_tag`, `Tollana_Copplemann` |
| condition 0 | `step_status`, 568, `1966`, `eq`, `active` |
| action 0 | `display_dialog`, 2154, NULL, `{}` |

Dialog 2153 is shown for both offer and in progress; the in-progress reading is the same Copplemann briefing text, which is the least wrong zero-button reuse (section 8, W3).

### 4.2 Accept, progress and turn-in dialog_choice chains

Chain 9004 accept on close of 2153
| Table | Row values |
|---|---|
| trigger | `dialog_choice`, `2153` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `mission_status`, 568, NULL, `eq`, `not_active` |
| action 0 | `remove_dialog_set`, 2656, NULL, `{"slot": 420}` |
| action 1 | `accept_mission`, 568, NULL, `{}` |
| action 2 | `advance_step`, 568, `1964`, `{}` |

Order is load-bearing. Action 0 first so the offer bind never coexists with the in-progress bind (the first bind with a dialog wins on a fall-through). Action 1 accepts on step 1963 and fires `mission_accepted` NESTED, which runs 9006 mid list. Action 2 force-completes 2309 and loads 2310. Do not reorder.

Chain 9005 turn-in on close of 2154
| Table | Row values |
|---|---|
| trigger | `dialog_choice`, `2154` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `step_status`, 568, `1966`, `eq`, `active` |
| action 0 | `remove_dialog_set`, 6393, NULL, `{"slot": 420}` |
| action 1 | `complete_mission`, 568, NULL, `{}` |

Action 1 closes objective 4491 (only `complete_mission` closes a turn-in objective) and fires nested `mission_completed '568'`, which no chain in this file consumes (the T-03b seam for 569). No reward row: seed reward is 0/0, owner decision O1.

Chain 9006 progress bind after accept
| Table | Row values |
|---|---|
| trigger | `mission_accepted`, `568` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| action 0 | `add_dialog_set`, 4959, NULL, `{"slot": 420, "mission_id": 568}` |

### 4.3 Relog and world-entry restore (trigger `player_loaded`, key `Tollana`)

Binds live in the per-player `available_interactions` map, are not persisted and are rebuilt empty on every world entry; `player_loaded` fires on login, gate travel and cross-world teleport. These three chains are therefore mandatory. Each carries `world 19` plus the state gate; the three gates are disjoint and SDone has no restore (no bind remains).

Chain 9007 restore offer
| Table | Row values |
|---|---|
| trigger | `player_loaded`, `Tollana` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `mission_status`, 568, NULL, `eq`, `not_active` |
| action 0 | `add_dialog_set`, 2656, NULL, `{"slot": 420}` |

Chain 9008 restore in progress
| Table | Row values |
|---|---|
| trigger | `player_loaded`, `Tollana` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `mission_status`, 568, NULL, `eq`, `active` |
| condition 2 | `step_status`, 568, `1966`, `neq`, `active` |
| action 0 | `add_dialog_set`, 4959, NULL, `{"slot": 420, "mission_id": 568}` |

Chain 9009 restore turn-in
| Table | Row values |
|---|---|
| trigger | `player_loaded`, `Tollana` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `step_status`, 568, `1966`, `eq`, `active` |
| action 0 | `add_dialog_set`, 6393, NULL, `{"slot": 420, "mission_id": 568}` |

### 4.4 Abandon and re-accept

Chain 9010 abandon repaint (shape copied from Harset 6120, `harset_goauld_chains.sql`)
| Table | Row values |
|---|---|
| trigger | `mission_abandoned`, `568` |
| condition 0 | `world`, 19, NULL, `eq`, NULL |
| condition 1 | `mission_status`, 568, NULL, `eq`, `not_active` |
| action 0 | `remove_dialog_set`, 4959, NULL, `{"slot": 420}` |
| action 1 | `remove_dialog_set`, 6393, NULL, `{"slot": 420}` |
| action 2 | `add_dialog_set`, 2656, NULL, `{"slot": 420}` |

`remove_dialog_set` is a no-op when the bind is absent, so one chain serves an abandon from step 1964, 1965 or 1966. After it, the player is in S0: 9001 matches, 9007 would match on the next entry, and re-accept runs 9004 again from the top (the offer guard permits it: `repeats` is still 0; abandon is not a completion). An abandon performed while in another world is healed by 9007 on the next Tollana entry; the `world 19` gate keeps 9010 from pushing binds to a player standing elsewhere. Failed state is unreachable in this spec (nothing fails 568); if it ever occurred it reads `not_active` and takes the S0 path.

### 4.5 Food-unit prop chains (trigger `interact_tag`, key = prop tag)

Props never use binds: template 421 carries the permanent mission-world-object flag, so the click works in every state and the chain decides the effect. Every prop dialog is shown with `display_dialog` from the interact trigger, which stamps `target_entity_id` for the speaker. The three props are mutually disjoint by tag; within a tag the chains are disjoint by step. `A` = `step_status 568 '1964' eq active`; `B` = `step_status 568 '1965' eq active`; `N1964` / `N1965` = the same with `neq`.

Core (tag `Tollana_FoodUnit_Core`, dialog 4211)
| Chain | Conditions (sort_order) | Actions in order |
|---|---|---|
| 9011 | 0: A | 0 `advance_step` 568 `1965`; 1 `advance_step` 568 `1966`; 2 `remove_dialog_set` 4959 `{"slot": 420}`; 3 `add_dialog_set` 6393 `{"slot": 420, "mission_id": 568}`; 4 `display_dialog` 4211 `{}` |
| 9012 | 0: B | 0 `advance_step` 568 `1966`; 1 `remove_dialog_set` 4959 `{"slot": 420}`; 2 `add_dialog_set` 6393 `{"slot": 420, "mission_id": 568}`; 3 `display_dialog` 4211 `{}` |
| 9013 | 0: N1964; 1: N1965 | 0 `display_dialog` 4211 `{}` |

Nest (tag `Tollana_FoodUnit_Nest`, dialog 4213)
| Chain | Conditions (sort_order) | Actions in order |
|---|---|---|
| 9014 | 0: A | 0 `advance_step` 568 `1965`; 1 `complete_objective` 568 `2312`; 2 `display_dialog` 4213 `{}` |
| 9015 | 0: B | 0 `complete_objective` 568 `2312`; 1 `display_dialog` 4213 `{}` |
| 9016 | 0: N1964; 1: N1965 | 0 `display_dialog` 4213 `{}` |

Fungus (tag `Tollana_FoodUnit_Fungus`, dialog 4214)
| Chain | Conditions (sort_order) | Actions in order |
|---|---|---|
| 9017 | 0: A | 0 `advance_step` 568 `1965`; 1 `display_dialog` 4214 `{}` |
| 9018 | 0: N1964 | 0 `display_dialog` 4214 `{}` |

Trigger for each: `interact_tag`, the tag, `player`, false, 0. Action columns are `(action_type, target_id, target_key, params)`; `advance_step` and `complete_objective` carry the step/objective id as TEXT in `target_key` and `{}` params; `display_dialog` carries NULL key.

Rules baked into the shapes:
- NEVER `complete_objective 2311`. It is the last required objective of step 1965 and would auto-complete the whole mission. 2311 is closed only by `advance_step 1966` (force-complete).
- `advance_step` runs no all-required-done auto-complete, so the Core's double advance is safe.
- `complete_objective 2312` on an already-credited 2312 is a logged WARN no-op. A repeat Nest click in step 1965 produces that WARN and is expected, not a defect. `complete_objective 2312` on an optional objective does not complete the mission because 2311 is still open.
- Once the mission passes 1965 (S1966 or SDone) the Nest chains fall to 9016, display only, no credit.
- Prop chains need NO `mission_status` gate: a non-participant clicking in S0 matches the "other" chain and only sees the dialog.

## 5. Seed file, linter allowlist, registration

1. File header comment: cite this doc, state that all chains are CIMMERIA_RECONSTRUCTION, list the id range 9001-9018, state reward unresolved.
2. `db/database.sql`: add `\ir resources/Content/Seed/tollana_568_chains.sql` directly after the last existing `*_chains.sql` include.
3. `crates/content-engine/tests/it/interact_tag_linter.rs`, fn `allowlist`: add `("tollana_568_chains.sql", 9001..=9003) // Tollana_Copplemann: bit comes from the per-player dialog bind on slot 420 (add_dialog_set), not a zone-wide flag` and `("tollana_568_chains.sql", 9011..=9018) // Tollana_FoodUnit_*: template 421 default INT_MissionWorldObject (1073741824), as Dakara templates 445-448`. Do not allowlist any other id. Do not add `set_interaction_type` to the file.
4. `dialog_button_linter` must pass UNCHANGED: 2153, 2154, 4211, 4213, 4214 stay zero-button (R1/R2). Dialogs 2153 and 2154 are keyed by `dialog_choice` and are `DUIST_DefaultDialog` (R4 ok).
5. Add the id rows 9002-9018, tags and template/spawn status to `tollana-id-register.csv` (CRLF preserved).

## 6. Double-display and race analysis

- Bind auto-display versus `interact_tag`: a matching `interact_tag` chain marks the click handled and skips `handle_interact`, so the bind's dialog never auto-opens. In S0, S1964, S1965, S1966 exactly one Copplemann chain matches (disjoint, exhaustive), so the bind is glyph-only. In SDone no chain matches AND no bind remains (6393 is removed by 9005 before `complete_mission`), so the click falls through to nothing: silent, no double dialog.
- A bind left behind would be the only way to double-display. Invariant: slot 420 holds at most one live bind, always the one for the current state (S0 2656; S1964/S1965 4959; S1966 6393; SDone none). Every transition removes before it adds (9004, Core 9011/9012, 9005, 9010). The restore chains add into an empty map, never on top of a live bind (world entry resets it). `add_dialog_set` does not dedupe, so the coder MUST NOT add a second `add_dialog_set` for any state.
- Re-display of 2153: `dialog_choice` is gated on `open_dialog_id`, cleared on the first valid choice, so closing a re-displayed 2153 fires 9004 at most once. A SECOND close of an in-progress 2153 fires nothing because 9004 also gates on `not_active`.
- Double-click on the offer: two `display_dialog 2153` calls before a close, then one close. Only the first close finds `open_dialog_id` set; the post-state is the single-accept state. Tests assert post-state, never counts of intermediate dialogs.
- Accept racing the prop clicks: a prop clicked in S0 matches "other" and cannot advance anything. A prop clicked between `accept_mission` and `advance_step 1964` inside 9004 is impossible because the cell processes one event list at a time.
- Nested dispatch: 9006 runs mid 9004 (between actions 1 and 2). It only adds a bind and does not read step state, so the nested timing is harmless. Conditions of 9004 were frozen at resolve (S0), so the later actions are unaffected by the state change.
- Step-status timing trap: 9011 advances twice. Its conditions (A) were resolved once, so the second `advance_step` is not re-gated by the first; this is intended. Do not add a step gate to the second action.

## 7. Two-player correctness

- Mission state, `available_interactions` binds and `open_dialog_id` are per player. `add_dialog_set` / `remove_dialog_set` push glyph flags to the acting player's witness set only. No chain in the file calls `set_interaction_type`, so no template-level or entity-level flag ever changes for a bystander. Player A accepting never alters what player B sees: B keeps the offer glyph (or none, if B is SDone).
- Copplemann (spawn 19001) and the props (19002-19004) are shared entities; every effect keys off the CLICKING player's mission state, which the conditions read per context. Two players in different states clicking the same prop in the same tick each match the chain for their own state.
- Visible shared wart: the permanent prop glyph shows to players who never accepted 568 (W5).
- The Copplemann glyph for B is correct only after B's `player_loaded` restore. A player who is in Tollana when ANOTHER player's abandon runs is unaffected (witness push is per player).
- The disjointness invariant holds per player, so a test with two contexts proves isolation by construction (section 10, T-16).

## 8. Warts and owner decisions

| Id | Item | Disposition |
|---|---|---|
| W1 | 2153 has zero buttons, so Decline and the X also accept (any close fires button -1). | Accepted for the pilot. A button on 2153 would strand the accept (R2 rule). Needs a design call only if the owner wants a real Decline. |
| W2 | SDone: Copplemann click is silent, no glyph. | Intended: 568 is a one-shot, the follow-up lives in T-03b (569, `mission_completed '568'`). |
| W3 | In-progress click re-shows briefing 2153, not a status line. | Least-wrong zero-button reuse; no in-progress-specific dialog exists. |
| W4 | Core text says "take it back to Copplemann" though no item exists. | Text is ORIGINAL_DATA; narrative only. The item-backed variant is HA-1360a. |
| W5 | Props are clickable and glyph-visible to everyone, in every state. | Dakara 445-448 precedent; template default flag. |
| W6 | Speaker 1100 per screen versus template speaker 968 unverified. | UAT row U3 below. |
| W7 | Positions UNPLACED; mesh for 421 unchosen. | Blocking gate G1, T-02. |
| O1 | Reward: seed is 0 naq / 0 xp, `complete_mission` grants nothing. | OWNER DECISION. The coder must NOT add `grant_xp` / `grant_cash`. Any reward is a later one-row chain edit once decided. |

## 9. Blocking gates (coder returns `blocked` if unmet)

- G1: T-02 has claimed template 421 and spawns 19002-19004 with a client-present static mesh and positions. Without them the three prop chains have nothing to click.
- G2: the id scan for 9001-9018 and the tags finds no collision on current main and the four active zone branches.
- G3: owner decision O1 is recorded as "none for now" (this spec ships it that way; a different answer means one extra action row on chain 9005).

## 10. Test list

Home: new directory `crates/cell-content/src/cell/content/chain_replay_tests/mission_568/` (`mod.rs`, `disjointness.rs`, `progression.rs`, `relog.rs`, `abandon.rs`, `two_player.rs`), registered in `chain_replay_tests/mod.rs`. Mirror `mission_1361/` and `mission_abandoned.rs`. Helpers: `build_engine(Some(&pool))`, `require_db_or_skip!`, `ExecutionContext::new()`, `ctx.world_id = 19`, `ctx.set_param("mission_568_status", ...)`, `"mission_568_step_1964_status"` and the 1965/1966 equivalents, `"entity_tag"`, `"dialog_id"`, `"open_dialog_id"`, `TriggerEvent { trigger_type: TriggerType::InteractTag, ... }` through `engine.resolve_event`. Live-DB tests are prefixed `live_db_` and run the real executor against a test database.

| # | Test name | Event and context | Assertion | Fails on revert because |
|---|---|---|---|---|
| T1 | `live_db_tollana_568_seed_loads_18_chains` | load seed, no event | chains 9001-9018 enabled, scope mission 568, none refused by the loader | deleting or malforming the seed drops chains (a malformed condition row leaves the chain UNGATED, so also assert each chain's condition count) |
| T2 | `copplemann_chains_pairwise_disjoint` | InteractTag `Tollana_Copplemann` over status {not_active, active, completed} x 1966 {not_active, active, completed} | fired set is exactly: not_active -> {9001}; active and 1966 not active -> {9002}; 1966 active -> {9003}; completed -> {} | dropping `step_status neq` on 9002 makes 9002 and 9003 co-fire |
| T3 | `prop_chains_pairwise_disjoint` | InteractTag each prop tag x step {none, 1964, 1965, 1966, completed} | exactly one chain fires, per the tables in 4.5 | loosening an "other" chain to one condition makes it co-fire with the step chain |
| T4 | `offer_click_displays_2153` | InteractTag Copplemann, S0 | action list is `[display_dialog 2153]`, no other action | wrong dialog id or extra action |
| T5 | `live_db_accept_on_2153_close` | DialogChoice `2153`, `open_dialog_id` 2153, S0 | order remove(2656), accept, advance(1964) in the action log; after: mission 568 active, step 1964 active, 2309 completed, 2310 active; binds on slot 420 == `[4959]` exactly | reordering actions leaves 2656 and 4959 both bound, or accept sees the wrong step |
| T6 | `reclose_2153_is_noop_when_active` | DialogChoice `2153`, mission active | zero chains fire | removing the `not_active` gate re-runs accept on every close |
| T7 | `live_db_core_first_click_reaches_1966` | InteractTag Core at S1964 | after: step 1966 active, 2310/2311/2312 completed, mission still ACTIVE (not completed), slot 420 == `[6393]` | using `complete_objective 2311` would auto-complete the mission; dropping the double advance strands the step |
| T8 | `live_db_core_from_1965_reaches_1966` | InteractTag Core at S1965 | step 1966 active, binds `[6393]` | 9012 missing |
| T9 | `live_db_nest_credits_2312_once` | Nest at S1964, again at S1965 | after first: step 1965 active, 2312 completed, 2311 active; second click logs the complete_objective no-op WARN and state is unchanged; mission not completed | removing 9014's advance leaves 2312 unrostered; using required-only logic would complete early |
| T10 | `live_db_fungus_advances_1964_only` | Fungus at S1964, again | step 1965 active; no objective 2311/2312 credit; second click leaves state unchanged | 9017 missing |
| T11 | `live_db_turnin_completes_and_unbinds` | Copplemann click at S1966 then DialogChoice `2154` | click displays 2154; after close: mission completed, `repeats` 1, slot 420 empty, nested `mission_completed` fired once with no chain consuming it | gate removal or a missing remove leaves a stale 6393 bind |
| T12 | `completed_state_is_silent` | Copplemann and each prop at SDone | Copplemann fires {}; props fire only the display-only chain; no action mutates state | adding an SDone Copplemann chain would re-offer |
| T13 | `bind_hygiene_slot_420_one_live_bind` | scripted walk: S0, accept, Nest, Core, turn-in; and the abandon branch | after EVERY event the slot holds the expected single bind (len <= 1) | dropping any `remove_dialog_set` yields two live binds and a fall-through double display |
| T14 | `restore_chains_paint_each_state` | PlayerLoaded `Tollana`, world 19, per state {not_active, 1964, 1965, 1966, completed}, plus world 1 | not_active -> `[2656]`; 1964/1965 -> `[4959]`; 1966 -> `[6393]`; completed and wrong world -> `[]` | missing restore means a relog shows no glyph |
| T15 | `live_db_abandon_then_reaccept` | accept, advance to 1965, `abandon_mission`, then offer + 2153 close again | after abandon: not_active, slot 420 == `[2656]`; re-accept reaches step 1964 with `repeats` 0; abandon from step 1966 also ends at `[2656]` | 9010 missing leaves a dead progress glyph or no re-offer |
| T16 | `two_player_isolation` | two `ExecutionContext`s: A at S1964, B at S0; A clicks Core | A reaches 1966; B's state and B's fired chains are unchanged (B still matches 9001 and prop "other") | any shared (non-per-player) state would leak |
| T17 | `seed_has_no_forbidden_actions` | text scan of the seed file | no `grant_xp`, `grant_cash`, `add_item`, `remove_item`, `set_interaction_type`, `complete_objective` with target_key `2311`, no `delay_ms` other than 0 | a "helpful" reward or item row slips in |
| T18 | `double_click_offer_post_state` | two offer clicks, then one 2153 close | post-state is the single accept state; binds `[4959]` | missing `open_dialog_id` gate |
| T19 | existing linters | run `interact_tag_linter` and `dialog_button_linter` suites | green with only the allowlist entries of section 5 | an unlisted tag chain or a button on 2153 fails them |

Run order: T1 first (a failing load makes the rest meaningless). Revert proof: for each of T2, T5, T7, T11, T13 delete the guarded line from the seed or swap the action order and show the test red, then restore. Run through the build lane only (`pwsh tools/build-lane/lane.ps1 cargo test -p cell-content ...`), never cargo directly.

## 11. Telemetry the executor already emits (use in UAT and SigNoz)

| Message | Emitted when | Key fields |
|---|---|---|
| `Content: accepting mission` | 9004 action 1 | mission 568 |
| `Mission accepted` | accept lands | `event=mission_accepted` |
| `Content: accept_mission refused by offer guard` / `accept_mission: offer refused - mission state preserved` | accept refused (e.g. already active, completed with repeats exhausted) | `event=mission_accept_refused`, `reason` |
| `Content: advancing step` | each `advance_step` | mission 568, new step id |
| `Content: complete objective` | Nest credit | objective 2312 |
| `Content: completing mission` | 9005 | mission 568 |
| `Content: complete called on non-active mission - skipping mission_completed event` | `complete_mission` on a non-active mission | indicates a state-gate bug |
| `Content: abandoning mission` | abandon | |
| `Content: adding dialog set` | every bind add | `interaction_only`, `interactions_count`, `dialog_set_map_id`, `template_id` |
| `Content: removing dialog set` | every bind removal | |
| `dialog_set_maps cache miss for add_dialog_set` (WARN) | dsm id not cached | must never appear for 2656/4959/6393 |
| `advance_step: mission not found` (WARN) | wrong mission id | must never appear |
| `objective_never_completed` (stuck-player signal) | a step left with open objectives | expected silence at turn-in |

Expected sequence for a clean run (UAT checklist for the owner or lab): offer click shows 2153 -> close -> `accepting mission`, `Mission accepted`, `adding dialog set` (template_id 420, dsm 4959), `advancing step` (1964) -> prop click -> `advancing step` (1965) (+ `complete objective` 2312 on the Nest) -> Core click -> `advancing step` (1966), `removing dialog set` 4959, `adding dialog set` 6393 -> Copplemann click -> 2154 -> close -> `removing dialog set` 6393, `completing mission`. A repeat Nest click logs one complete-objective WARN: not a fault. The per-chain "fired" log line name was not re-verified here; do not assert on it.

UAT rows the owner runs (no agent drives the lab): U1 offer glyph and 2153 text; U2 each prop dialog and the 1964->1965 journal change; U3 speaker label on 2153/2154; U4 relog at each state keeps the right glyph; U5 abandon then re-accept; U6 turn-in and the silent completed state; U7 journal shows objectives despite `is_enabled` false.

## 12. Stop rules (return `blocked`, do not improvise)

1. Gate G1, G2 or G3 unmet, or template 421 / spawns 19002-19004 not claimed by T-02.
2. The loader refuses any literal in section 4 (unknown condition or action type, JSON params rejected). Do not reshape; report the exact error.
3. T2 or T3 shows two chains co-firing in any state, or a state with zero chains where one is promised.
4. T9 shows `complete_objective 2312` completing the mission, or T7 shows the mission completing before turn-in.
5. After `advance_step`, the new step's objective roster is empty (an `is_enabled` / `step_enabled` filter exists that grep missed). Do not flip the flags in the seed.
6. `dialog_choice` does not fire on closing 2153 or 2154 in the replay/live test. Do not add a button to "fix" it.
7. `dialog_button_linter` or `interact_tag_linter` fails on any id outside the section 5 allowlist. Do not widen the allowlist.
8. Any need to change Rust executor code, add an action type, add a `db/scripts` migration, or edit a mission/dialog/dialog_set_maps row.
9. The `dialog_set_maps cache miss` WARN appears, or `add_dialog_set` leaves slot 420 with two live binds.
10. The urge to add a reward, an item row, a `set_interaction_type`, or a Decline/Accept button. All are owner decisions.
11. Anything that needs the lab, the client, a server, or a position decision. Positions belong to T-02.
