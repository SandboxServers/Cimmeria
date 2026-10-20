# Union SGC continuation ledger

> **Type:** Reference (campaign ledger)
> **Audience:** The SGC_W1 Union coordinator and the packet coders, reviewers and specialists named in the packet table
> **Last updated:** 2026-10-19 (US-99 close-out)
> **Index:** [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md) (packet text, findings and contract; this ledger does not copy it)
> **Companions:** [RESUME.md](RESUME.md), [crossings.md](crossings.md), [placement.md](placement.md), UAT steps [U1](uat-U1.md), [U2](uat-U2.md), [U3](uat-U3.md), [unified-uat.md § SGC_W1](../../guides/unified-uat.md#sgc_w1-human-union-start), [execution contract](../union-zone-planning/execution-contract.md)

## Purpose

Track the SGC_W1 (world 58) Union campaign packet by packet: which packets are ready, which are blocked and why, which IDs they own, and what US-00 verified before any seed or code packet starts. US-00 changes no seed and no code. It records the ID re-check, the crossings handshake with Dakara and Omega, and the engine facts every SGC packet relies on.

## Release readiness (2026-10-19)

The SGC_W1 Union continuation is **code-complete and unplayed**. Every chain is covered by database and executor tests; no step has run in a client. All three UAT files are owner-run, and agents did no lab use.

### What is on `main`

Packets US-00, US-01, US-02, US-10, US-11, US-12, US-13, US-14, US-15, US-16, US-20, US-21, US-22, US-23, US-24, US-30, US-31 and US-32 merged to `main` before the owner's "no PRs until the restoration is done" directive. The PR numbers are in the packet table below.

### What is on `union-sgc/sgc-final` (local, no PR yet)

| Item | Note |
|---|---|
| US-33 | 1655 Free Jaffa chains 3166-3169 and the disjointness suite `disjoint.rs` |
| US-34 | Return state and relog chains 3171-3175 |
| S4 fix pass | `disjoint.rs` selector widened to every chain touching missions 1563-1571 (F2), exact documented exceptions for chains 3172 (Free Jaffa only) and 3103 (space-scoped death trigger, no `archetype` parameter on that dispatcher), Omega's chain block 63800-63999 covered (F3); the five tutorial chains that had no Human pair got one (D-run6) |
| `rig.rs` | The test rig `ARRIVAL` is the sealed P13 (167.00, -10.80, 18.00) |
| US-25 | Reverted (D-run7): Dakara owns the gate 27 pin |
| This close-out | UAT files U1, U2, U3, status docs, indices |

Chain tests: the `union_sgc` live-DB module passed 105 tests at the S4 fix pass and `cimmeria-content-engine` passed 316. That is the whole of the testing.

### What remains owner-side

| Item | State |
|---|---|
| One PR for `union-sgc/sgc-final` | Held by the owner's directive (no push, PR or merge-queue activity) |
| UAT U1, U2, U3 | UATPending: [uat-U1.md](uat-U1.md), [uat-U2.md](uat-U2.md), [uat-U3.md](uat-U3.md) |
| Placement lab checks owed | Listed in [uat-U1.md](uat-U1.md#placement-lab-checks-owed) |
| US-16 escort walk | Judge the navmesh stall in [uat-U1.md](uat-U1.md#the-1564-escort-walk-judge-it) |
| D-US2, D-US3 | Label policy and authored scale; US-40 stays HELD until the label policy is set |
| D-US4 to D-US8, D-US12 | Shipped on their defaults; the owner can overrule later |
| `dakara/dk-sgc` merge | Needed for the gate 27 pin (see below) |
| HELD-A, H-REPRESS | Hidden objectives left uncredited; a second Harriman press inside the 3 s window can teleport twice (U2-07 characterises it) |

### Known limitations and release notes

- **Arrival heading is not carried.** `Action::CrossWorldTeleport` has no heading field, so a crossing delivers a position only. The Omega pin's heading 3.1409 (and any SGC heading) is dropped, and the player faces whatever the engine gives. [U3](uat-U3.md) checks position and records facing.
- **The US-16 escort stalls.** The SGC_W1 navmesh has no route between Carter's spawn, the Failsafe room and the Gate Room, so her follow stops at the first gap. The chains still end correctly (she is teleported to the device point). The owner accepted this (D-US13 = a); [U1](uat-U1.md) judges it.
- **Gate 27 depends on Dakara.** The gate 27 arrival pin (DK-SGC-01) and the `debug_area` test fix live on `dakara/dk-sgc`. Until that branch merges, gate 27 arrival (a GM or dial arrival, not our chain crossings, which carry their own coordinates) stays off the navmesh by 3.4 m. Merge `dakara/dk-sgc` first, or accept that.
- **US-40 is held.** Authored ambience and Carter's line 5394 wait for the label policy (D-US2, D-US3). Nothing authored ships.
- **Dakara and Omega receivers.** The returns from Dakara (1651, 1654) and the Omega arrival need Dakara DK-30b, DK-33 and Omega's C4 receiver on the same build. Without them those UAT steps are NOT RUN, not FAIL.
## Status of this ledger

- US-00: **Done** (merged #1880). The ID re-check found no collision and the crossings table has no unresolved mismatch (the C4 pin was published and is in row C4).
- The D-US1 amendment request below is history; the coordinator owns merging it into `docs/analysis/four-zone-planning/id-allocation.md`.
- The campaign is closed except for the owner-side items in § Release readiness.

## Findings (F1-F13)

Copied as one-line summaries from the index § Findings (verified there at `d1bba0400`). The evidence stays in the index.

| # | Finding | Packets |
|---|---|---|
| F1 | No "talk" interaction bit. Each press needs a marker set by an earlier chain or a `player_loaded` restore. | all |
| F2 | Carter's flavor line 5394 has no press; dropped from the story path and parked in US-40. | US-24, US-40 |
| F3 | `mission_completed` fires after the state change, so one chain can carry `mission_completed X` and `player_loaded` together. | all |
| F4 | `delay_ms` runs from fire time; a deferred `advance_step` leaves the old gate open. Departures advance the first step at 0 ms. | US-24, US-30, US-32 |
| F5 | Point sets with `flags & 1` are hinted to the client, so cylinder regions need no client change. | US-11, US-15, US-22 |
| F6 | Dialog 5371 is display only (never keyed); 5855 fails linter R1 (never keyed); 5370 passes R1 (keyable). | US-12, US-33 |
| F7 | `entity_dead_tag` fires once per killer with the killer's context; a DoT kill fires nothing. | US-20, US-21, US-22 |
| F8 | Chain 3005 moves Hammond to the armory during 1559; US-22 walks him home on the 1568 completion. | US-22, US-24 |
| F9 | The interact-tag linter is per file: each `interact_tag` key needs a `set_interaction_type` row in the same file. | all |
| F10 | `dialog_button_linter/seed_model.rs:14` lists only five `CHAIN_FILES`; US-10 adds the 14 new files. | US-10 |
| F11 | Static rows 57-63 are untagged Jaffa on the 1564 route; 65-67 tagged Gate Room Jaffa; 71 and 77 are 1562 attackers. | US-11, US-20, US-21, US-34 |
| F12 | Carter hands over her notes herself (5370 screen 9, 5371). Template 5801, spawn 15802 and P03 stay reserved and unused. | US-11, US-12 |
| F13 | `complete_objective` completes the whole mission once every non-optional active objective of the current step is complete. Verified in § Engine facts below. | US-12, US-15, US-20-US-23, US-30, US-31 |

## Decisions

Status vocabulary: Decided, BlockedDecision (owner or partner must answer), HELD. Non-blocking rows name the packets that carry their default.

| ID | Question | Owner | Default used | Status |
|---|---|---|---|---|
| D-US1 | ID blocks (see § ID re-check and the amendment request below) | Architect | blocks as listed | Decided; this ledger records the re-check. Coordinator merges the request |
| D-US2 | Label policy for reconstructed and authored content shown to players | Owner | none | Small labelled authored content allowed (2026-10-19); the label policy itself is still unset, so US-40 stays HELD |
| D-US3 | Scale of authored content (U-D07) | Owner | none authored | Small authored content allowed (2026-10-19); US-40 HELD on the label policy |
| D-US4 | Visible dial before each crossing (sequences 10047/10060) | Owner (maintainer) | play both sequences; drop the rows if no | BlockedDecision, non-blocking: US-24, US-30, US-32 |
| D-US5 | Return state seen by a player returning through C2/C3 | Owner | no Gate Room Jaffa once 1568 is done; no story Jaffa for a Free Jaffa; Carter in the Gate Room during 4931 | BlockedDecision, non-blocking: US-34 |
| D-US6 | Walk into gate 27 instead of crossing on Harriman's interact | Owner | cross on the Harriman interact after the dial | BlockedDecision, non-blocking |
| D-US7 | Virus hack minigame or plain interact (O5369) | Owner | plain interact | BlockedDecision, non-blocking: US-12 |
| D-US8 | 1565 flank objective O5375: untracked or credited | Owner | untracked (optional) | BlockedDecision, non-blocking: US-20 |
| D-US9 | Edit gate 27 row or carry coordinates in chains | Architect | chains carry H3; row untouched | Decided, stays as decided. The gate 27 row pin is Dakara's DK-SGC-01 (D-run7) |
| D-US10 | Static Jaffa rows 57-63 versus the 1565/1567 story Jaffa (options A, B, C) | Owner | B (tag the static rows as story Jaffa) | Decided 2026-10-19: B |
| D-US11 | 1655 letter item 6786 at departure | Owner + Omega | 6786 to Humans on Hammond's 5856; nothing to Free Jaffa | Decided 2026-10-19: Humans only |
| D-UD2 | Who owns the 1651/4930 departure | Dakara (D-UD2) | DK-30 | Accepted by SGC; confirmed in Dakara plan (C2 row) |
| D-US13 | US-16 escort: accept the navmesh stall or re-slice | Owner | (a) accept | Decided 2026-10-19: (a); U1 judges it |
| D-run6 | The disjointness suite found five tutorial chains (3086, 3087, 3089, 3096, 3097) with no Human pair | Autonomous run | add the Human pair | Decided: added |
| D-run7 | US-25 duplicated Dakara's DK-SGC-01 gate 27 pin | Autonomous run | revert US-25 | Decided: reverted; release depends on `dakara/dk-sgc` |
| D-run8 | Chain 3172 is Free Jaffa only but `disjoint.rs` demanded Human only | Autonomous run | exact exception for 3172, selector widened | Decided |

## ID re-check (US-00 step 2)

Searched every `.sql` under `db/resources/` (394 files), the Dakara, Omega, Beta and four-zone plans, and the seed rows in COPY-free `INSERT` form (first value of each tuple, table-aware for `content_*`, `spawnlist`, `point_sets`, `point_set_points`, `entity_templates` and the dialog tables).

| Kind | Block | Search | Hits |
|---|---|---|---|
| chain ids | 3051-3199 | first value of every `content_*` tuple, plus any tuple starting with the number | 0 |
| spawnlist ids | 15800-15849 | first value of `spawnlist` tuples | 0 |
| point_sets | 15800-15819 | first value of `point_sets` tuples | 0 |
| points | 15800-15899 | first value of `point_set_points` tuples | 0 |
| entity_templates | 5800-5809 | first value of `entity_templates` tuples | 0 |
| dialogs | 58000-58049 | first value of `dialogs`, `dialog_sets`, `dialog_screens`, `dialog_set_maps`, `dialog_screen_buttons` tuples | 0 |

Result: **no collision.** Zero hits in each block, so no `blocked` return from this step.

Non-hits, recorded so the coordinator does not mistake them for collisions:

- `id-allocation.md:21` lists the existing SGC chains as 3001-3037 and 3041-3044. The block 3051-3199 starts after them.
- The Beta plan uses `show 3057`, `show 3161` (dialog ids in the `show` action) and item 3165 (Asgard Medical Scanner). These are dialog and item ids, not chain ids, and sit outside the 58000-58049 dialog block. They do not collide with a chain id.
- The Dakara and Omega plans cite SGC chain 3123-3131 (UD-04a) and nothing in the SGC blocks.
- `abilities.sql` rows in the 3915-3950 range are ability ids, not chain ids.

## Amendment request for the coordinator (D-US1)

Request; the coordinator merges this into `docs/analysis/four-zone-planning/id-allocation.md`. US-00 does not edit that file.

| Kind | Block | Use |
|---|---|---|
| chains | 3051-3199 | SGC_W1 Union packets (US-12 to US-40, per index § ID allocation); 3180-3199 is US-40 (HELD) |
| spawnlist | 15800-15849 | 15800 Harriman, 15801-15802 reserved (F12), 15803 Carter terminal, 15804 Failsafe Device, 15805-15810 crates |
| point_sets | 15800-15819 | 15800 `SGC_W1.FailsafeRoom`, 15801 `SGC_W1.GateControl` |
| points | 15800-15899 | point ids for the same sets |
| entity_templates | 5800-5809 | 5800 Failsafe Device, 5801 reserved (F12), 5802 pressable crate, 5803 Carter terminal |
| dialogs | 58000-58049 | US-40 only, HELD |

## Crossings handshake (US-00 step 3)

The frozen C1-C4 rows and their partner checks are in [crossings.md](crossings.md). Summary:

- C1, C2, C2b, C3: **match** the Dakara plan (DK-10b, DK-30, DK-33 and D-UD2).
- C4: **was a mismatch on the arrival pin; resolved 2026-10-19** (Omega published x 34.70, y -51.28, z -142.00, heading 3.1409 and row C4 carries it; the heading is not carried by the engine). The original finding follows as history. SGC K5 used a candidate gate 5 pin (34.705, -50.634, -149.092). Omega's plan gives a first choice (34.70, -51.86, -142.0) that is a candidate, not a pin: `omega-work-packets.md:121` says "first choice", and `omega-packets/UO-00.md:29` says the pin is copied from UO-01's worknote only if UO-01 confirms it. Expected 1655 state on 4940 matches. The D-US11 letter decision matches D-UO11.
  - Resolution (done): Omega published the pin from UO-01 and SGC changed K5 to that value.

## Engine facts (US-00 step 4)

Each row is confirmed with file:line, or the difference is stated.

| Fact | Evidence | Result |
|---|---|---|
| F13 `complete_objective` completes the mission when every non-optional active objective of the current step is complete | `crates/cell-content/src/cell/missions/progression.rs:325` (`complete_objective`), `:469` (`all_required_complete`), `:484-505` (vacuous branch and `mission.complete()`) | Confirmed. Difference: a step whose objectives are **all optional** completes the mission on the first completion (`:469`, comment at `:477-483`). This is the F21 trap for 1655 S4940 |
| 1651 step 4931 has two required objectives, O6037 and O5956 | `db/resources/Missions/Seed/mission_objectives.sql:3102` (6037, step 4931, `is_optional` false), `:3104` (5956, step 4931, `is_optional` false) | Confirmed |
| 1654 step 4939 has O5961 and O5962, both required | `mission_objectives.sql:3126` (5961), `:3128` (5962), both `is_optional` false | Confirmed |
| Chains matched by one event share one pre-action snapshot and run in chain-id order | `crates/content-engine/src/chain/mod.rs:284` (`fire_event`) evaluates each chain's conditions and then runs its actions inside the loop. `:423` (`resolve_event`) collects matches before running. Order: `register_chain` (`:152-168`) sorts each bucket by priority descending, stable, so ties keep load order. The `chain_rows` query order was not traced | **Confirmed, with one condition.** Order is priority descending, then load order. The loader reads `content_chains` (and triggers) `ORDER BY chain_id` (`crates/cell-content/src/cell/content/engine_loader.rs:53, 70`) and `register_chain` sorts stably, so chains of **equal priority** run in chain-id order. `content_chains.priority` defaults to 0 (`db/resources/Content/Tables/content_chains.sql:9`); an SGC chain must leave it at 0 for the chain-id rule to hold. Production never calls `fire_event` (no non-test caller); every dispatch (for example `event_dispatch/mission.rs:68, 133, 206`) uses `resolve_event`, which evaluates all conditions before any action runs, so the shared pre-action snapshot holds. Nested lifecycle dispatch can still see mid-list changes (agent memory `multi-chain-dispatch-semantics.md`) |
| Trigger rows OR together | `crates/content-engine/src/loader/mod.rs:236-251`: one in-memory `Chain` per trigger row, the same chain id | Confirmed |
| Condition rows AND together | `crates/content-engine/src/chain/mod.rs:322` (`conditions.iter().all`) | Confirmed |
| Zero-button dialog sends `dialog_choice (id, -1)` on close; closing a dialog with buttons sends nothing | `crates/cell-content/src/cell/content/event_dispatch/dialog.rs:164-167` names button -1 as "closed" for logging only | **Confirmed.** `dialog.rs:164-167` is the server read side only. The send side is client RE, recorded in agent memory `dialog-chain-authoring-rules.md:75-101` (Ghidra `FUN_00d249c0` sends `ButtonId = -1` only when the dialog's total button count is 0; a button-bearing dialog sends nothing on close). No lab check is needed to use the rule |
| `spawn_entity` skips a tag already alive | `crates/cell-content/src/cell/content/executor/spawn/mod.rs:170` (`tag_already_live`, logged and skipped) | Confirmed |
| `despawn_entity` on an absent tag logs DEBUG and returns | `executor/spawn/mod.rs:300-318` (`despawn_by_tag`, `tracing::debug!` "entity tag not found for despawn", then `return`) | Confirmed |
| SGC_W1 is a per-player instance | `entities/spaces.xml:39` (`Instanced="true"`); `crates/cell-world/src/cell/arrival.rs:111-113, 201` (instanced destinations have no space until a traveller arrives); `crates/cell-world/src/cell/space_manager/lifecycle.rs:34` (`create_space_instance`) | Confirmed: instanced, and created per arrival |

F13 is already recorded in agent memory (`.claude/agent-memory/mission-systems-advisor/advance-step-vs-complete-objective.md`). No new memory note was needed for it.

## Seal checklist per packet (US-00 step 5)

> History: the Sealed or open column records the state when US-00 wrote it (2026-10-18). The current status of every packet is in the packet table below.

"Sealed" means the packet's own facts are fixed. "Open" names the item that keeps it BlockedDependency. The allowlist file is the packet's chain or test file from the index § ID allocation.

| Packet | IDs it owns | Allowlist file | Test module | Sealed or open |
|---|---|---|---|---|
| US-00 | none | this ledger and crossings.md | none | Sealed by this commit (docs) |
| US-01 | P01-P16, H3 (no ids) | `union-sgc-continuation/placement.md` | none (docs seal) | Open: not run. Its seal feeds US-30 and US-32 |
| US-02 | meshes, name_ids, sequences 10047/10060, crate room | per packet file | none (docs seal) | Open: not run |
| US-10 | 14 empty chain files, CHAIN_FILES registration | `db/resources/Content/Seed/sgc_w1_union_*` (14), `dialog_button_linter` list, `database.sql` `\ir` lines | `chain_replay_tests/union_sgc/` | Open: needs US-00 merged; no IDs of its own |
| US-11 | spawnlist 15800-15810, templates 5800/5802/5803, point sets 15800/15801, points 15800-15899, Jaffa tags | `Worlds/Seed/spawnlist.sql`, `Events/Seed/point_sets.sql`, `Entities/Seed/entity_templates.sql` | per packet | Open: needs US-10, US-01, US-02 and D-US10 default |
| US-12 | chains 3051-3057 | `sgc_w1_union_1563_chains.sql` | `virus_1563.rs` | Open: needs US-10, US-11; D-US7 non-blocking |
| US-13 | chains 3080-3081 | `sgc_w1_union_1571_chains.sql` | `aux_1571.rs` | Open: needs US-10 |
| US-14 | none (log tests) | per packet | `executor/tests/teleport.rs` | Open: needs US-00 |
| US-15 | chains 3060-3071 | `sgc_w1_union_1564_chains.sql` | `failsafe_1564.rs` | Open: needs US-10, US-11 |
| US-16 | chains 3073-3078 | `sgc_w1_union_escort_chains.sql` | `escort_1564.rs` | Open: needs US-15; movement sign-off |
| US-20 | chains 3085-3093 | `sgc_w1_union_tactics_chains.sql` | `tactics_1565_1566.rs` | Open: needs US-10, US-11; D-US8, D-US10 |
| US-21 | chains 3095-3099 | `sgc_w1_union_1567_chains.sql` | `clear_way_1567.rs` | Open: needs US-10, US-11; D-US10 |
| US-22 | chains 3100-3109 | `sgc_w1_union_1568_chains.sql` | `gate_room_1568.rs` | Open: needs US-10, US-11; movement sign-off |
| US-23 | chains 3110-3122 | `sgc_w1_union_1569_chains.sql` | `ordnance_1569.rs` | Open: needs US-10, US-11; item sign-off |
| US-24 | chains 3123-3131 | `sgc_w1_union_1570_chains.sql` | `depart_1570.rs` | Open: needs US-10, US-14, US-02; D-US4 default |
| US-30 | chains 3133-3142 | `sgc_w1_union_1651_chains.sql` | `generator_1651.rs` | Open: needs US-10, US-11, US-01, UD-00 |
| US-31 | chains 3148-3152 | `sgc_w1_union_1654_chains.sql` | `turnin_1654.rs` | Open: needs US-10, UD-00 |
| US-32 | chains 3156-3165 | `sgc_w1_union_1655_human_chains.sql` | `letter_1655_human.rs` | Open: C4 pin (crossings.md), UO-00, D-US11 default |
| US-33 | chains 3166-3169 | `sgc_w1_union_1655_jaffa_chains.sql` | `letter_1655_jaffa.rs`, `disjoint.rs` | Done (local, integrated on union-sgc/sgc-final) |
| US-34 | chains 3171-3175 | `sgc_w1_union_return_chains.sql` | `return_state.rs` | Done (local, integrated on union-sgc/sgc-final) |
| US-25 | none | none | none | Dropped: gate 27 pin owned by Dakara DK-SGC-01 on dakara/dk-sgc (D-run7) |
| US-40 | chains 3180-3199, dialogs 58000-58049 | `sgc_w1_union_authored_chains.sql` | `authored.rs` | **BlockedDecision (HELD)**: D-US2, D-US3 |
| US-99 | none | docs | none | Open: all others |
| US-U1 | none (lab) | none | lab | UATPending: asks the user |
| US-U2 | none (lab) | none | lab | UATPending: asks the user |

## Packet table

Status vocabulary: Done / Dropped / HELD / UATPending. The packet text stays in the index and its packet files.

| Wave | Packet | Status | Note |
|---|---|---|---|
| S0 | US-00 | Done (merged #1880) | ledger, ID re-check, crossings |
| S0 | US-01 | Done (merged #1886) | placement seal |
| S0 | US-02 | Done (merged #1884) | meshes, sequences, crate room |
| S1 | US-10 | Done (merged #1891) | scaffold |
| S1 | US-11 | Done (merged #1917) | templates, spawns, point sets, Jaffa tags (D-US10 = B) |
| S1 | US-14 | Done (merged #1893) | crossing log tests |
| S2 | US-12 | Done (merged #1919) | 1563 |
| S2 | US-13 | Done (merged #1920) | 1571 |
| S2 | US-15 | Done (merged #1921) | 1564 |
| S2 | US-20 | Done (merged #1922) | 1565, 1566 |
| S2 | US-21 | Done (merged #1923) | 1567 |
| S2 | US-22 | Done (merged #1924) | 1568 |
| S3 | US-16 | Done (merged #1933) | escort; navmesh stall accepted (D-US13 = a) |
| S3 | US-23 | Done (merged #1934) | 1569 |
| S3 | US-24 | Done (merged #1936) | 1570 and C1 |
| S3 | US-30 | Done (merged #1937) | 1651, C2 arrival, C2b |
| S3 | US-31 | Done (merged #1935) | 1654 |
| S3 | US-32 | Done (merged #1938) | 1655 Human and C4 |
| S4 | US-33 | Done (local, on `union-sgc/sgc-final`) | 1655 Free Jaffa and the disjointness suite |
| S4 | US-34 | Done (local, on `union-sgc/sgc-final`) | return state |
| S4 | US-25 | Dropped | gate 27 pin owned by Dakara DK-SGC-01 on `dakara/dk-sgc` (D-run7) |
| S4 | US-40 | HELD | D-US2, D-US3: label policy unset |
| S4 | US-U1 | UATPending | [uat-U1.md](uat-U1.md), owner-run |
| S4 | US-U2 | UATPending | [uat-U2.md](uat-U2.md), owner-run |
| S4 | US-U3 | UATPending | [uat-U3.md](uat-U3.md), owner-run; added by the owner: the Omega arrival |
| S4 | US-99 | Done (local, on `union-sgc/sgc-final`) | this close-out |
UD-00 and UO-00 are the Dakara and Omega ID packets. Whether they are dispatched or merged is the coordinator's gate and is not verified here.
