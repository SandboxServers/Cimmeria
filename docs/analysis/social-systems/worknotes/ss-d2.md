# SS-D2 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [SS-D1 worknote](ss-d1.md).

## Contract

- **Packet:** SS-D2, the PvP flag and the harm gate.
- **Decisions in force:** D-SS18 (5 s countdown), D-SS20 (non-lethal end: SS-D3's, not implemented here), D-SS22 (no rewards), D-SS23 (the flag is presentation only; the gate reads the registry), D-SS24 (registry, no `SGWDuelMarker`), D-SS25 (superseded by SS-E1 D-Q5: 151 at the engage and 153 at every end are safe).
- **Contract change from the coordinator:** the ledger's `combat::player_may_harm` is replaced by widening the pets campaign's `combat::player_may_attack` (PR #896). Done; no parallel predicate exists.
- **Base:** written on `origin/main` @ `e0d5cecf7`, rebased onto `851d8796b` (crafting CR-08 and CR-09, the social ledger update #907). Branch `social/d2-pvp-harm-gate`, worktree `.claude/worktrees/ss-d2`.
- **Owned paths (new):**
  - `crates/cell-world/src/cell/duel/{engage.rs, end.rs, combat.rs}`, `crates/cell-world/src/cell/duel/tests/{engage.rs, interactable.rs}`
  - `crates/cell-combat/src/cell/abilities/use_ability/tests/duel_gate.rs`
  - `crates/wireclient/tests/it/duel_two_duelists_and_a_spectator.rs`
  - `.claude/agent-memory/rust-gameserver-dev/offline-client-event-trace-and-udp-port-trap.md`
  - this file
- **Edited:** see "Contended files touched" and "Other files touched".
- **Read set:** the ledger (Contract, Contended files, SS-D2; README D-SS18, 20, 22-25; audit A-41, A-42, § 6); SS-D1's worknote and `duel/`; SS-E1's `duel-wire-formats.md` and `duel-restoration.md`; `docs/architecture/abilities-and-effects-system.md`; `combat/aggression.rs` and the four gate sites; `threat/player_combat.rs`; `space_manager/aoi.rs`; `request_entity_update.rs`; the combat advisor's `ontimerupdate-wire-and-clock.md` and `pvp-duel-readiness.md` memories; the client Lua under `Working/SGWGame/Content/UI/Core/` and `SGW.exe`.

## Evidence

No Ghidra instance was reachable (`mcp__ghidra__list_instances` returned none), so both open client questions were traced offline. The full write-up is in [duel-wire-formats.md § SS-D2 receiver trace](../../../reverse-engineering/findings/duel-wire-formats.md).

- **D-Q4, the PvP-flag vehicle: resolved, `onEntityProperty(GENERICPROPERTY_PvPFlag = 4, v)`.** The client UI reads `Property.PVPFlag` (`UnitFrames.lua:173-208`, `Squad.lua:217`) from the same `Property.*` table and `Events.PropertyUpdated` event that carry TrainingPoints, AppliedSciencePoints and AccessLevel, which the server already delivers through `onEntityProperty` and which work in game. No Lua reads a `pvpFlag` property, and `CELL_PUBLIC` is ghost-only. High confidence; the native hop from `onEntityProperty` to `Events.PropertyUpdated` was not re-traced. Client quirk: `UnitFrames.lua:175` indexes `TrackedUnits[unitID]` with the wrong case, so the flash refreshes only when a unit frame is registered or remapped (retargeting), not live.
- **D-Q1, the countdown driver: `onTimerUpdate` with `Type = 14` raises `Event_UI_DuelTimerStart`.** Proved from bytes: `0x00dec9e0` accepts type 14 only (`cmp byte [esp+0x13], 0x0e` at `0x00deca8f`), converts `BigWorldTimeComplete` to seconds remaining, and queues an event node whose type descriptor (`0x01e0da40`) and vtable (`0x019d52d0`) RTTI-name `Event_UI_DuelTimerStart`. This also corrects the "BigWorld time-complete" label in `ability-resolution-pipeline.md` and `address-map.md`.
- **D-Q5:** applied as SS-E1 found it. 151 names only the two duelists, 153 carries nothing, AoI's 152 is untouched, and the `aoi.rs` comment now says 152 erases and works through the forced recompute.

## Design decisions

- **The harm gate is one function.** `player_may_attack(attacker, target, &DuelRegistry)`: an NPC target keeps the old rule (hostile faction, not a pet); a player target needs an engaged pair in the same space (`can_harm`). The single-target launch and the warmup re-check call it directly. The ground-AoE and cone collectors now scan `combat::area_candidates` (every NPC plus the caster's engaged partner) and filter with `combat::may_hit_in_area` (`player_may_attack` for a player caster, the old hostile-faction rule for an NPC caster), so the only player a collector can ever see is the partner. Bystanders, NPC-versus-duelist and duelist-versus-NPC behaviour are unchanged.
- **Pets.** `pet::fight_refusal` calls the no-duel form, `player_may_attack_pve` (follow-up 2 below): a pet never joins its owner's duel, and no pet file needed threading. This is the only edit in a pets file.
- **The engage checks both duelists are still in the duel's space.** If either left during the countdown, nothing is flagged, the duel is dropped and whoever is still in the world hears 878 (`duel.engage_refused`, `reason = duelist_gone`, `gone = challenger | target | both`).
- **Engage order, per duelist:** 151, the flag (self and witnesses), the combat source, the "The duel has begun." line (Cimmeria's wording; there is no moniker).
- **The combat source** is the opponent's entity id in `threatened_mobs`, the set `BSF_InCombat` derives from, so the bit clears at the end only when no mob still holds the player (`duel/combat.rs`, restating `enter/exit_player_combat` because those live in `cell-combat`, which depends on `cell-world`). The state field goes to self and witnesses, as the mob path does. `generate_threat` still ignores player targets, so duel hits add no threat.
- **The flag reaches a late witness** through `duel::pvp_flag_on_enter`, called on the AoI enter path after the `EnteredAoI` and on the `requestEntityUpdate` re-emit (a re-create resets the client's copy). An unflagged player needs nothing: 0 is the client default and world entry already sends `(4, 0)`.
- **One end clear.** `duel::end_engaged(duel_id, reason)` removes the duel first (so `can_harm` is false at once), then for each duelist still at the engaged entity: flag 0 to self and witnesses, 153, combat exit; and 878 to each duelist still in the world. `Duel.engaged_entities` records the entities the engage touched, so the clear never touches a later entity of the same player. SS-D3 reuses it and adds reasons and result texts.
- **Safety ends (the packet's "can't stay Engaged forever").** The tick's `end::sweep` ends an engaged duel at `ENGAGED_LIMIT` (10 minutes, project policy, `reason = engaged_limit`) or as soon as a duelist is no longer the connected player at the engaged entity in the duel's space (`reason = duelist_gone`: logout, gate travel, a destroyed entity). The sweep uses `connected_player` (O(1)), not the `find_player` scan.
- **Countdown display.** At the accept each duelist gets `onTimerUpdate(duel_id, 14, own entity, 0, 5.0, game_time_secs() + 5.0)`. SS-D1's "Duel accepted. The duel starts in 5 seconds." line stays as the text acknowledgement.
- **`DuelState::Engaged` carries `until`** (the safety deadline). `can_harm` is now `engaged_opponent(attacker) == Some(target)`.
- **The wire constants are pinned against `entities/defs/`**: `GENERICPROPERTY_PvPFlag` and `DuelTimer` from `enumerations.xml`, and 7, 12, 143, 151, 153 from the flattened SGWPlayer client-method table (reusing `pet_def_tests`' flattener, made `pub(super)`).

## Telemetry (owner rule: debuggable from SigNoz alone)

No new log target (`duel` exists since SS-D1). No span inside the tick (rule 3); the engage and ends are one DEBUG row each.

| Event | Level | Where | Fields beyond the ids | Test |
|---|---|---|---|---|
| `duel.engaged` | DEBUG | cell tick | `duel_id`, `target_entity_id`, `target_account_id`, `space_id`, `state = engaged`, `pvp_flag_witnesses`, `target_pvp_flag_witnesses` | `engage_fans_the_pvp_flag_to_both_duelists_and_a_witness` (asserts the ids) |
| `duel.engage_refused` | DEBUG | cell tick | `reason = duelist_gone`, `gone`, `target_entity_id`, `space_id` | `countdown_end_with_a_duelist_gone_is_refused` (type 12) |
| `duel.ended` | DEBUG | cell tick (SS-D3: every end path) | `reason = engaged_limit \| duelist_gone`, `entity_id` / `target_entity_id` (the engaged entities), `cleared`, `target_cleared`, `space_id` | `engaged_limit_ends_the_duel_and_clears_both_flags`, `duelist_leaving_the_world_ends_the_duel` |
| `duel.send_failed` | WARN | cell | now also for a witness send: `witness_id`, `method_index` | shape as SS-D1's `send_failure_logs_the_recipient_and_the_other_duelist` |
| `duel.send_failed` | WARN | cell (`requestEntityUpdate`) | the PvP-flag re-emit could not be queued: `witness_id`, `target_player_id` | not unit-tested (a closed channel on the re-emit path) |
| `duel.aborted` | removed | | SS-D1's interim `reason = engage_not_implemented` row no longer exists | |

Every row carries `account_id`, `player_id` and `entity_id` for the challenger and `target_player_id` for the target, plus `duel_id`.

### SigNoz queries

| Question | Query |
|---|---|
| Did the duel start? | `scope_name = 'duel' AND duel_id = <id>`: `duel.accepted`, then `duel.engaged` (or `duel.engage_refused` with `gone`) |
| Who saw the flag? | `duel.engaged` gives `pvp_flag_witnesses` for each side; a later arrival appears as an `EnteredAoI` then a `WitnessEntityMethod` (method 7) in the same trace |
| Why did the duel end? | `scope_name = 'duel' AND event = 'duel.ended' AND duel_id = <id>`; `reason` names the end, `cleared` / `target_cleared` say whose flag and combat state were reset |
| Is anyone stuck flagged? | a `duel.engaged` with no `duel.ended` for the same `duel_id` older than 10 minutes means the sweep is not running |

## Commands run

All from the worktree root, through the lane. Exit 0 unless stated; no test self-skipped except where said.

- `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell-world --all-targets`, then `-p cimmeria-cell --all-targets`.
- `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-cell-combat duel_gate`: 2 passed.
- `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-cell-world -p cimmeria-wire duel`: 45 passed.
- `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell`: 1631 passed, 0 skipped.
- `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell --all-targets -- -D warnings`: clean.
- `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-services -p cimmeria-cell-methods -p cimmeria-cell-content -p cimmeria-cell-console -p cimmeria-cell-interactions --all-targets -- -D warnings`: clean (every dependent of the changed signature).
- `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wireclient --all-targets -- -D warnings`: clean.
- `bash tools/build-lane/lane.sh cargo fmt --all -- --check`: clean.
- `bash tools/build-lane/reload-db.sh` (into `sgw_ss_d2`), then `bash tools/build-lane/lane.sh bash -c 'export DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw_ss_d2; cargo test -p cimmeria-wireclient --test it duel_two_duelists -- --test-threads=1'`: 1 passed (9.6 s). The same for `two_client_castle_visibility::`: 2 passed, after the harness fix below.
- No live-DB tier run: this packet adds no SQL.

## Tests

| Test | Type | Guards |
|---|---|---|
| `duel_partner_damage_allowed_at_all_four_gates` | unit (cell-combat) | each gate admits the engaged partner; one section per gate |
| `bystander_untouchable_during_duel` | unit (cell-combat) | a bystander at each gate, a bystander hitting a duelist, and a partner whose duel ended during the warmup |
| `engage_fans_the_pvp_flag_to_both_duelists_and_a_witness` | type 8 | 151 with exactly the pair, the flag to self and to the other duelist and the bystander, `BSF_InCombat` to self and witnesses, the line, `can_harm` only for the pair, the `duel.engaged` ids |
| `witness_entering_mid_duel_gets_the_current_flag` | type 8 | a late witness gets both flags, after each create, and none for a non-duelist |
| `engaged_limit_ends_the_duel_and_clears_both_flags` | type 8 + 12 | the safety end clears flag (self and witnesses), 153, combat, registry; `reason = engaged_limit` |
| `duelist_leaving_the_world_ends_the_duel` | unit + 12 | `reason = duelist_gone`, the survivor fully cleared |
| `countdown_end_with_a_duelist_gone_is_refused` | type 12 | `duel.engage_refused` fields; nothing flagged |
| `interactable_npc_stays_interactable_across_a_duel` | unit | the D-SS25 guard: AoI's 152 before and after a duel, 151 names only the pair, 153 is empty, the duel never sends 152, the NPC's flags and binds untouched |
| `accept_starts_the_countdown_for_both` (updated) | type 2 | the type-14 timer to each duelist, own entity as `SourceID`, `TotalTime` 5, absolute expiry |
| `pvp_flag_is_byte_exact`, `on_duel_entities_set_is_byte_exact`, `duel_timer_is_byte_exact`, `constants_match_the_entity_defs` | type 2 | the three builders; the constants read back from `entities/defs/` |
| `duel_engages_for_two_duelists_and_flags_both_for_a_spectator` | type 11 (not in CI) | the whole flow on the real wire with two duelists and a spectator |

SS-D1's `countdown_end_aborts_until_ss_d2` is deleted: the behaviour it pinned is gone. The test harness `drain` now records witness routings (`Sent.witness`) instead of panicking on 151/153.

## Regression proof

Each mutation applied to the working tree, the named tests run, the file restored from a byte copy (the diff returned to its pre-mutation state each time):

| Mutation | Result |
|---|---|
| gate 1 (`handle.rs`) back to `target.is_player \|\| faction != HOSTILE` | `duel_partner_damage_allowed_at_all_four_gates` FAILED "gate 1: the launch at the duel partner was refused" |
| gate 2 (`warmup/tick.rs`) back to the inline rule | FAILED "gate 2: the warmup re-check refused the duel partner" |
| gate 3 (`dispatch/mod.rs`) scanning `all_npc_entity_ids()` | FAILED "gate 3: the ground AoE did not collect the duel partner" |
| gate 4 (`cone_aoe/geometry.rs`) scanning `all_npc_entity_ids()` | FAILED "gate 4: the cone did not collect the duel partner: []" |
| `player_may_attack` admitting any other player (`a != t`) | `bystander_untouchable_during_duel` FAILED at the gate-1 bystander assertion |
| no witness fan-out in `send_to_self_and_witnesses` | `engage_fans_...` and `engaged_limit_...` FAILED; the type-11 test FAILED "the spectator saw flags only for {}" |
| no AoI replay (`pvp_flag_on_enter` → `None`) | `witness_entering_mid_duel_gets_the_current_flag` FAILED |
| no `end::sweep` in the tick | `engaged_limit_...`, `duelist_leaving_...` and `interactable_...` FAILED |
| no `send_countdown` at the accept | `accept_starts_the_countdown_for_both` FAILED |

## Follow-up: pets stay out of duels, and the SS-U2 GM end

Two coordinator notes after the first push.

**Pets (from the pets campaign).** The duel widening applies only when attacker and target are the two engaged players; a pet never joins a duel (the default until the owner decides). This already held: in `player_may_attack` a pet target takes the NPC branch, which refuses every pet, and `pet::fight_refusal` refuses player targets before it calls the rule (with an empty registry), so a pet neither fights the duel opponent nor accepts its threat. The new guard `duel_opponent_cannot_harm_partner_pet` (`use_ability/tests/duel_gate.rs`) pins it: A, dueling B, cannot target B's pet (the rule, the area rule, the single-target launch, a ground AoE centred on the pet), and B's pet refuses A as a fight target and as a threat source (`fight_refusal`, `threat_refusal`, `generate_threat` leaves A off its threat list). Regression proof: a mutation that lets `player_may_attack` admit any pet while the attacker is in an engaged duel fails it at "the rule". No pets file was edited for this.

**`warmup/tick.rs`** stays at the one-condition edit for PT-04's rebase.

**SS-U2 (#910, open, not merged).** `duel::end_engaged` is the single end every path must use, and now returns the ended `Duel` (`None` when the duel is gone or not engaged, so a second call sends nothing). `EndReason::GmAborted` (`reason = gm_aborted`) exists for `.duel_end`. Guard: `gm_end_of_an_engaged_duel_clears_through_end_engaged` (both flags to 0, 153, combat cleared, the `duel.ended` row, and a second call is a no-op). The integration edits #910 needs, whichever merges second:

- `DuelState::name`: the arm is `DuelState::Engaged { .. } => "engaged"` (the variant now carries `until`).
- `duel::gm::gm_end`: before `mgr.duels.gm_abort(subject_player_id)`, if the subject's duel is `Engaged`, call `end_engaged(tx, mgr, duel_id, EndReason::GmAborted)` and return `GmAborted::Duel(duel)` without sending its own 878 lines (`end_engaged` sends them). `gm_abort` stays for a countdown or a pending challenge, where nothing was flagged.
- A `.duel_end` test on an engaged duel that asserts the flag clear and 153, like the guard above.

**Type 11.** `duel_two_duelists_and_a_spectator.rs` is written against `main`, not the #910 branch: it uses `support::enter_castle` and three real sessions, and finishes in about 10 s, well inside the 60 s reap window, so it needs no sparbot heartbeat. If #910 moves `enter_world` into `crates/wireclient/src/world_entry.rs` and changes `support`, adapt the test at merge; it does not use `sparbot`.

Commands (exit 0): `lane.sh cargo nextest run -p cimmeria-cell-combat duel_gate` (3 passed); `lane.sh cargo nextest run -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell -p cimmeria-wire` (1633 passed, 0 skipped); `lane.sh cargo clippy -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell --all-targets -- -D warnings` (clean); `lane.sh cargo fmt --all` (clean).

## Follow-up 2: the explicit no-duel rule for pets, and the pet guard's proof

- **`combat::player_may_attack_pve(attacker, target)`** (`aggression.rs`) is the no-duel form of the rule: a hostile-faction NPC that is not a pet, never a player, never a pet. It is also the NPC half of `player_may_attack`, which calls it, so the two cannot drift. `pet::fight_refusal` now calls it instead of `player_may_attack(owner, target, &DuelRegistry::default())`: "a pet never duels" is explicit, and no registry is built per pet tick. Keep that call inside `fight_refusal` after PT-04 (#901), so one change covers pet AI, pet commands (CM 88, `player/pet/invoke.rs`) and the warmup's pet path.
- **`duel_opponent_cannot_harm_partner_pet`** now also covers the cone (A fires at a hostile mob just past B's pet; the cone must not collect the pet), the no-duel rule, and that the pet is not an area candidate. It checks: the rule, the area rule, the no-duel rule, the candidate list, A's single-target launch, a ground AoE centred on the pet, the cone, and B's pet refusing A as a fight target, as a threat source and on its threat list.
- **Revert proof** (each applied, the guard run, the file restored byte for byte):

| Mutation | Result |
|---|---|
| `may_hit_in_area` also admits any entity whose pet owner is one of the caster's engaged duelists | FAILED at "the area rule" |
| `player_may_attack` admits any pet while the attacker is in an engaged duel | FAILED at "the rule" |
| the same `may_hit_in_area` leak **and** `area_candidates` adding the opponent's pets, with the direct predicate assertion disabled | FAILED at "the cone collected the partner's pet: [100000]" |

  The ground AoE on the pet stays unhit under that last mutation because its primary target, the pet, is refused by gate 1 before any secondary is collected; pets are also not in `all_npc_entity_ids` (SGWMob class only), so a collector leak needs both the candidate and the rule to leak.
- **`player_may_attack` / `player_may_attack_pve` callers on this branch (before #901):** `use_ability/handle.rs:256` (single target), `use_ability/warmup/tick.rs:165` (warmup re-check), `aggression.rs:136` (`may_hit_in_area`, used by `dispatch/mod.rs` and `cone_aoe/geometry.rs`), `aggression.rs:109` (`player_may_attack` → `_pve` for NPC targets), `npc_ai/pet/mod.rs:123` (`fight_refusal`, `_pve`). Re-list after the #901 rebase, which adds `warmup/pet_order.rs` and `player/pet/invoke.rs` as `fight_refusal` callers.
- **Merge order:** #901 (PT-04) lands first; then rebase onto `main` (the player condition in `warmup/tick.rs` is back to main's text there, so the one-line replacement applies) and push with `--force-with-lease`.

Commands (exit 0): `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell -p cimmeria-cell-methods` (1868 passed, 0 skipped); `lane.sh cargo clippy` on those five plus `cimmeria-services`, `--all-targets -- -D warnings` (clean); `lane.sh cargo fmt --all -- --check` (clean).

## Rebase onto PT-04 (#901)

Rebased onto `origin/main` @ `91d36d02b` (#901 merged). No conflicts: the player condition in `warmup/tick.rs` applied as one line, and #901's pet re-check (`warmup/pet_order.rs::pet_fire_refusal`) sits after it. The gap-analysis rows still sum to the totals line (471 / CW 169 / NT 65 / IM 104 / KM 129 / NU 4).

Every caller of the hostility rule after the rebase:

| Caller | Calls | Covers |
|---|---|---|
| `cell-combat/.../use_ability/handle.rs:256` | `player_may_attack(entity, target, &duels)` | single-target launch (gate 1) |
| `cell-combat/.../use_ability/warmup/tick.rs:165` | `player_may_attack(caster, target, &duels)` | player warmup re-check (gate 2) |
| `cell-world/.../combat/aggression.rs:136` (`may_hit_in_area`) | `player_may_attack(attacker, candidate, duels)` | ground AoE and cone collectors (gates 3 and 4) |
| `cell-world/.../combat/aggression.rs:109` | `player_may_attack_pve` | the NPC half of `player_may_attack` |
| `cell/.../service/ticks/auto_cycle.rs:122` | `player_may_attack(e, t, &duels)` | the auto-cycle stop for a player target (review fix 2) |
| `cell-combat/.../npc_ai/pet/mod.rs:123` (`fight_refusal`) | `player_may_attack_pve(owner, target)` | every pet path, through `fight_refusal`'s callers: `warmup/pet_order.rs:56` (pet cast re-check), `cell-methods/.../player/pet/invoke.rs:261` (CM 88 owner orders), `pet/defend.rs:45`, `pet/disengage.rs:57`, `pet/engage.rs:147`, `pet/stance.rs:101` and `threat_refusal` (`pet/mod.rs:152`) |

The pet-side check stays inside `fight_refusal`, so the no-duel rule covers pet AI, pet commands and the pet warmup with one call.

Commands (exit 0): `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell -p cimmeria-cell-methods`; `lane.sh cargo clippy` on those five plus `cimmeria-services`, `--all-targets -- -D warnings`; `lane.sh cargo fmt --all -- --check`. Results in the final report.

## Security review fixes (PR #911, d2-review)

The review found the core invariant holds, conditional on five side paths. All are fixed in this PR, each with a guard proven by revert.

| # | Finding | Fix | Guard |
|---|---|---|---|
| 1 | MEDIUM: the effect pulse never re-checks hostility and `end_engaged` stripped no effects, so a partner's DoT, stun or snare kept working after the end | `end_engaged` removes every active effect on each duelist whose `invoker_id` is the other duelist's engaged entity (`duel/effects.rs::strip_from`), running the script's `on_remove`, the stat flush and the zero `onTimerUpdate`, the same cleanup as the pulse sweep. The effect scripts live in `cell-world`, so no hook was needed. `duel.ended` gains `effects_removed` / `target_effects_removed` | `partner_dot_stops_when_the_duel_ends` (cell-combat, `use_ability/tests/duel_end.rs`): both DoTs pulse while engaged; after `end_engaged` only a bystander's DoT remains and lands, the partner DoT's icon is cleared |
| 2 | MEDIUM: an auto-cycle loop on the ex-partner re-fired every 100 ms into the #444 WARN with the button lit | `auto_cycle_tick` treats a player target that fails `player_may_attack` as invalid: clears the loop, sends `onStateFieldUpdate`, sends "Auto-attack stopped: that player is not your duel opponent." and logs `duel.auto_cycle_stopped` (`reason = not_duel_opponent`) | `auto_cycle_on_the_partner_fires_during_the_duel_and_stops_after_it` (cell, `ticks/auto_cycle_duel_tests.rs`): fires while engaged; after the end the loop is cleared, un-lit, one line, one row, no further sends, no #444 WARN |
| 3 | LOW-MEDIUM: `pet::defend::sync_owner_combat` treated the duel opponent in `threatened_mobs` as a stale mob, dropping the owner out of combat mid-duel | its stale filter prunes only NPC combat sources and never a player source (one filter line and a comment in the pets campaign's file). This replaced an earlier special case for `duel::engaged_opponent_entity`, at the pets coordinator's suggestion: no player-versus-player source can be pruned by accident, whatever adds it | `pet_owner_stays_in_combat_during_a_duel` (cell-combat, `duel_end.rs`): three AI ticks with B's pet summoned leave A in B's combat sources and `BSF_InCombat` set |
| 4 | LOW: `area_candidates` added the stored engaged entity id without re-checking it | `engaged_opponent_entity` re-checks `connected_player` (the id must still be the connected player playing the opponent, in the duel's space), which covers `area_candidates` | `recycled_partner_entity_is_not_offered_as_the_partner` (cell-world): after the partner's entity is destroyed and its id given to an NPC, the id is no longer the partner and is a candidate once |
| 5 | Telemetry | `duel.ended` captures both identities before teardown and adds `target_account_id`; the `requestEntityUpdate` `duel.send_failed` row adds `duel_id`; the effect-cleanup send failure carries `account_id` / `player_id` | `engaged_limit_ends_the_duel_and_clears_both_flags` now asserts `account_id` and `target_account_id` |
| 6 | Doc | `find_player`'s doc comment is back on `find_player`; `engaged_opponent_entity` has its own | -- |

Revert proof (each applied, the guard run, the file restored byte for byte):

| Mutation | Result |
|---|---|
| `end_engaged` without `strip_from` | `partner_dot_stops_when_the_duel_ends` FAILED "only the bystander's effect remains" |
| auto-cycle without the `player_may_attack` check | `auto_cycle_on_the_partner_...` FAILED "the loop is cleared" |
| pet stale filter pruning player sources again (`.filter(\|_\| true)`) | `pet_owner_stays_in_combat_during_a_duel` FAILED "the pet sweep dropped the duel opponent as a stale mob" |
| `engaged_opponent_entity` returning the stored id unchecked | `recycled_partner_entity_is_not_offered_as_the_partner` FAILED "a recycled id is not the partner" |

Also: the fixtures in `duel_gate.rs` passed a literal space id 1 to `start_duel`; they now use the attacker's real space, which the connected re-check needs. The reviewer's memory note `.claude/agent-memory/server-authority-enforcer/reference_duel_harm_gate.md` (and its index line) is committed, with a status line saying which of its side paths this PR closed. The copy in the main checkout is untracked there and was not touched.

Rebased onto `origin/main` @ `6ce726feb` (PT-07 #908, CR-07 #905, BM-03a #882). The only conflict was the gap-analysis summary sentence; the totals were recomputed from the 45 rows: 471 / CW 169 / NT 65 / IM 105 / KM 128 / NU 4. After the rebase: `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell -p cimmeria-cell-methods` (1923 passed, 0 skipped); clippy on those plus `cimmeria-services` and `cimmeria-cell-console` clean. Before the rebase: the same nextest (1919 passed, 0 skipped); `lane.sh cargo clippy` on those five plus `cimmeria-services`, `--all-targets -- -D warnings` (clean); `lane.sh cargo fmt --all -- --check` (clean).

## Docs

- `docs/reverse-engineering/findings/duel-wire-formats.md`: the SS-D2 receiver trace (D-Q4 resolved, D-Q1 driver), both headings updated.
- `docs/reverse-engineering/findings/duel-restoration.md`: open questions 1 and 7 closed; the lifecycle's flag wording.
- `docs/reverse-engineering/findings/ability-resolution-pipeline.md`, `docs/reverse-engineering/address-map.md`, `docs/reverse-engineering/findings/npc-movement-pathfinding.md`: type 14 / `0x00dec9e0` relabelled as the duel countdown.
- `docs/gameplay/duel-system.md`: status, "The engaged duel (SS-D2)", feature rows, RE priority 2.
- `docs/architecture/abilities-and-effects-system.md`: decision 24, the single hostility rule.
- `docs/gap-analysis.md` § 27 (Duel start KM → IM; Dueling 6 = 3 IM + 3 KM; totals, after the rebase onto CR-08/CR-09, IM 104, KM 129; the summary percentages recomputed and checked against the 45 rows), `docs/project-status.md` Dueling row, `docs/game-systems.md` § Dueling, `docs/protocol/message-catalog.md` (Dueling NetIn 3 of 4).
- The dispatch tables have no status column and their 7, 12, 143, 151-153 rows were already right; unchanged. `duel-restoration.md:50` (audit A-47) was already corrected by SS-E1.

## Known gaps

- **Partner damage is still lethal.** A duelist brought to 0 HP by the partner goes down the normal player death path (BSF_DEAD, Defeat Window, respawn) and the duel stays engaged until the sweep sees them gone or the 10-minute limit. D-SS20's 1 HP clamp and the health end are SS-D3; ship SS-D2 and SS-D3 in the same release if possible.
- **No forfeit, range, disconnect-hook or teleport end** (SS-D3). The sweep covers logout and gate travel as `duelist_gone` within one tick, with 878 instead of the SS-D3 texts.
- **The unit-frame flag does not refresh live** on the observer's client, because of the client Lua case typo above; it refreshes on retarget. Nothing server-side can fix that without a client patch.
- **No `SGWDuelMarker`** (D-SS24): the client needs none for the flag, 151/153 or the timer.
- **Type 11 does not run in CI** (audit A-60). It passed locally against `sgw_ss_d2`.
- `ENGAGED_LIMIT` (10 minutes) is project policy, like every other duel number.

## Contended files touched

- `crates/cell-combat/src/cell/abilities/use_ability/handle.rs`: the gate call now passes `&space_mgr.duels`, and its comment (+3 lines; 683 lines, under the 700 cap).
- `crates/cell-combat/src/cell/abilities/use_ability/warmup/tick.rs`: the inline `target.is_player || faction != HOSTILE_FACTION` re-check replaced by `player_may_attack(caster, target, &space_mgr.duels)` (one condition and its comment). **Pets PR #901 (PT-04) edits this file too.**
- `crates/cell-combat/src/cell/abilities/dispatch/mod.rs`: `collect_ground_targets` takes the attacker id and uses `area_candidates` + `may_hit_in_area`.
- `crates/cell-combat/src/cell/abilities/cone_aoe/geometry.rs` (and the `cone_aoe/mod.rs` doc): the same for the cone.
- `crates/cell-combat/src/cell/service/npc_ai/pet/mod.rs` (pets campaign's file): one call, `player_may_attack_pve(owner, target)`, with a comment.
- `crates/wire/src/cell/client_methods/pet_def_tests.rs` (pets campaign's file): `flattened_client_methods`, `index_of` and `enum_value` made `pub(super)` so the duel pin can reuse them.

## Other files touched

`crates/cell-world/src/cell/combat/{aggression.rs, mod.rs}`, `crates/cell-combat/src/cell/{mod.rs, combat/mod.rs}` (re-exports), `crates/cell-world/src/cell/duel/{mod.rs, registry.rs, limits.rs, outbound.rs, response.rs, tick.rs}` and its tests, `crates/cell-world/src/cell/space_manager/aoi.rs` (the flag replay and the 152 comment), `crates/cell/src/cell/service/base_messages/request_entity_update.rs` (the re-emit replay), `crates/wire/src/cell/client_methods/duel.rs`, `crates/wireclient/tests/it/{main.rs, support/mod.rs}`.

`support/mod.rs` gained `ephemeral_udp_port()` for the BaseApp port. On this host the TCP-ephemeral ports Windows handed out fell in a Hyper-V UDP exclusion range, so every `Orchestrator` start failed with WSAEACCES (10013), for the existing visibility tests too. Disabling the sandbox changed nothing; 40 of 40 TCP-ephemeral ports failed a UDP bind.

## Integration edits for the coordinator

- Before merging, tell the pets coordinator (now cimmeria-e4) about the `warmup/tick.rs`, `pet/mod.rs` and `pet_def_tests.rs` edits above (PR #901 touches `warmup/tick.rs`; the conflict is one condition).
- `docs/gap-analysis.md` and `docs/project-status.md` totals: other campaigns move the same totals line; recompute from the rows on merge.
- Ledger: SS-D2 → Review; D-SS23's provisional wording can be settled (the vehicle is `onEntityProperty(4, v)`); SS-E1 D-Q4 and the D-Q1 driver are closed by this packet; SS-D3 should call `duel::end_engaged` and add its reasons to `EndReason`.
- The branch is rebased onto `851d8796b`. The only conflict was the gap-analysis totals (crafting moved three rows); resolved by recomputing from the rows.

- SS-U2 (#910): whichever of #910 and this branch merges second routes `DuelRegistry::gm_abort` on an `Engaged` duel through `duel::end_engaged(..., EndReason::GmAborted)` (details in "Follow-up: pets stay out of duels, and the SS-U2 GM end"), and adds the `DuelState::Engaged { .. }` arm to `DuelState::name`.

## Open questions

- Should SS-D3 keep the `duelist_gone` sweep once the disconnect and teleport hooks exist? It is cheap and catches any missed path; I would keep it as a backstop.
- The owner may want a result text other than 878 for the safety ends; there is no moniker for "duel timed out".
