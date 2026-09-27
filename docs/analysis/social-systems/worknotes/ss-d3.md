# SS-D3 Worknotes

> Type: reference. Audience: social-systems coordinator and the server-authority reviewer.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [SS-D1 worknote](ss-d1.md), [SS-D2 worknote](ss-d2.md).

## Contract

- **Packet:** SS-D3, the duel end paths.
- **Decisions in force:** D-SS19 (40-unit arena, 5 s outside loses with `EDUEL_DEFEAT_Range`), D-SS20 (non-lethal: partner damage clamps at 1 HP and ends the duel on health; third-party damage stays lethal), D-SS21 (unchanged), D-SS22 (no rewards; 879 to the winner, a feedback line to the loser), D-SS23 (the flag is presentation only), D-SS24 (no `SGWDuelMarker`).
- **Base:** `origin/main` @ `f2af64cf4` (SS-C1 #893, on top of SS-D2 #911). Branch `social/d3-duel-end-paths`, worktree `.claude/worktrees/ss-d3`.
- **Owned paths (new):**
  - `crates/cell-world/src/cell/duel/{forfeit.rs, paths.rs}`
  - `crates/cell-world/src/cell/duel/tests/{end_paths.rs, end_table.rs}`
  - `crates/cell-combat/src/cell/abilities/use_ability/tests/duel_nonlethal.rs`
  - this file
- **Read set:** the ledger (Contract, Contended files, SS-D3; README D-SS19 to D-SS22; audit A-45 and § 6 CAT-M-14, CAT-M-15, the non-lethal rows); the SS-D1 and SS-D2 worknotes (including SS-D2's review follow-up); `duel-wire-formats.md`; `abilities-and-effects-system.md` decision 24; `entities/defs/enumerations.xml` (`EDuelDefeatReason`); `texts.sql` 872-880; `damage_apply/mod.rs`, `effects/pulsing/tick.rs`, `death/mod.rs`, `space_manager/entities.rs`, `base_messages/lifecycle.rs`; every `TeleportPlayer` / `GateTravel` site; the pets PT-02 scan test.

## Evidence

- **Reason numbering** is the client's `EDuelDefeatReason` (`enumerations.xml:1760-1771`): Health 1, LeftSquad 2, Connection 3, Range 4, Teleport 5, InDuel 6, Forfeit 7. No wire method carries it (neither 151-153 nor any duel method has a reason argument), so the cell logs it as `defeat_reason` on `duel.ended`. The five values used are pinned against the XML in `constants_match_the_entity_defs`. LeftSquad and InDuel have no path while squad duels are refused.
- **879 and 880** are `texts.sql` rows ("You won the duel", "You cannot forfeit a duel until you are engaged in one"), pinned verbatim in `moniker_texts_match_the_seed`. There is no loss moniker, so the loser lines are Cimmeria's wording (D-SS22 allows this).
- **The trade cancel** the packet refers to is not in `SpaceManager::disconnect_entity`: it is in `handle_disconnect_entity` (`crates/cell/src/cell/service/base_messages/lifecycle.rs:241`; also 175 on `DestroyEntity`). The ledger assigns `disconnect_entity` (`entities.rs`) to SS-D3, and that is the function every disconnect goes through, so the hook is there, beside the pets and ring-transport cleanups, before the entity is removed.
- **A DoT kills no player today.** `dot_kill_credit` returns early for a player target (`tick.rs`), so a player an NPC DoT takes to 0 HP never reaches `resolve_death`. The duel tick's dead-duelist check catches that case for a duelist. The general bug is outside this packet (see Known gaps).

## Design decisions

- **One end, extended, not a parallel `end_duel`.** `EndReason` gains `Defeated { loser: i32, reason: DefeatReason }`. `end_engaged`'s signature is unchanged, so SS-U2's `end_engaged(tx, mgr, duel_id, EndReason::GmAborted)` still compiles and still aborts (878 to both). A `Defeated` whose `loser` is not one of the pair is treated as an abort. The clear is SS-D2's (flag 0 to self and witnesses, 153, partner effects stripped, combat pair dropped); after it, a decided end sends 879 to the winner and the reason's line to the loser, and an abort sends 878 to both. A disconnect loser gets no line.
- **The paths.**

  | Path | Seam | Reason |
  |---|---|---|
  | Forfeit | `duel::forfeit::handle` from CM 103 (`cell-methods/.../player/social.rs`) | `Forfeit` (7); not engaged → 880, nothing changes |
  | Partner lethal damage | `duel::clamp_partner_lethal` in `apply_damage_to_target` (after the direct damage and after the effect scripts) and in `fire_pulse`; `duel::finish_clamped` ends it | `Health` (1) |
  | Death from anyone else | `duel::on_death` in `resolve_death`, for a player target | `Health` (1) |
  | Disconnect | `duel::on_disconnect` in `SpaceManager::disconnect_entity` | `Connection` (3) |
  | Teleport, gate travel | `duel::on_travel` before every `TeleportPlayer` / `GateTravel` send (13 sites in 8 files) | `Teleport` (5) |
  | Range | the duel tick (`end::sweep`) | `Range` (4) |
  | Backstops | the tick: a dead duelist (`Health`), one duelist gone from the space (`Teleport` if in the world elsewhere, else `Connection`), both gone (`duelist_gone`, abort), `ENGAGED_LIMIT` (abort) | |

- **Clamp ordering.** The clamp only writes HEALTH = 1 and returns a `ClampedHit`; the duel ends after the rest of the resolution. Ending at once would make `can_harm` false and strip the partner's effects before the same hit's script bleed or newly registered DoT had been clamped, and those would then kill after the duel. In `apply_damage_to_target` the end is the last statement, after the pulsing effects are registered, so `strip_from` removes them. The clamp runs before the stat flush in both seams, so the client is told 1, never 0.
- **Pulse tick fix.** `effect_pulse_tick` fired every due instance from a snapshot taken before any await. A clamped pulse ends the duel and strips the partner's other DoTs, but a second partner DoT due in the same tick would still fire from the snapshot, now unclamped, and kill. The loop now skips an instance that is no longer on the entity (`still_active`). This also closes the existing channel-cancel window the file's own comment describes.
- **Travel sites.** Every cell path that sends `TeleportPlayer` or `GateTravel` calls `duel::on_travel` immediately before the send, after the site's own validation. `every_travel_site_ends_the_duel` scans production code the way the pets PT-02 scan does and fails when a file has more travel sends than duel hooks; the movement validator's snap-back is exempt. The call is a no-op for anyone not in a duel, challenge or countdown. The sweep catches a space change by a path with no hook, within one tick.
- **Range.** Checked on the duel tick (no span, rule 3). The arena is `ARENA_RADIUS` (40) around the centre SS-D1 stores at the accept, 3D distance. Leaving warns once ("You are outside the duel area. Return within 5 seconds or you lose the duel.") and starts `Duel.out_of_range_since[side]`; coming back clears it; `RANGE_GRACE` (5 s) outside loses. If both are outside past the grace, the challenger is checked first.
- **The response window and countdown.** `DuelRegistry::withdraw(player_id)` removes a challenge to or from the player and a duel in its countdown, with no pair cooldown (nobody declined). `on_disconnect`, `on_travel` and `on_death` call it when the player has no engaged duel, and the other side hears 878 at once (the leaver too, except on a disconnect). Before this, the engage or the 30 s expiry discovered the departure.
- **Forfeit** acts only on the caller's own engaged duel, and only at the entity that was engaged. Both are read from the caller's cell entity; CM 103 has no arguments. A countdown, a pending challenge, no duel, or a bystander calling it all get 880 and change nothing (`stage` on the refusal row says which).

## Security reasoning (for the server-authority review)

- **Client input.** Only CM 103 is new, and it has no payload. The caller's identity is the cell entity the message arrived on (`entity_id` from the session), resolved with `connected_player`, which requires the entity to be connected and to play that `player_id`. A client cannot name another duel, another player or a reason. Forfeiting is never a way to lose someone else's duel: the loser is always the caller.
- **The clamp cannot be used to make a player unkillable.** It applies only when the attacker and the target are exactly the engaged pair's engaged entities (both entity ids and both `player_id`s must match, and the duel must be `Engaged`). Damage from an NPC, a bystander, a pet (a pet's damage is invoked by the pet entity, not the owner) or an old DoT from a previous duel partner is never clamped. After a clamped end, `can_harm` is false and the partner's effects are gone, so the "held at 1 HP" state is not reusable.
- **No death-path side effects from a duel.** A clamped duelist never reaches `resolve_death`: no `BSF_Dead`, no loot, no kill XP (which is NPC-only anyway), no contact-list Death fanout, no Defeat Window, no respawn. Guarded by `no_loot_xp_or_corpse_after_a_clamped_end` and the pulse test.
- **No reward path.** Nothing is persisted or granted on any end (D-SS22). The only outputs are the clear, feedback lines and the log row.
- **Leaving cannot strand a flag.** Every exit (disconnect, travel, death, range, forfeit, GM, the limit) goes through `end_engaged`; the table test fails if any one path skips it. A path someone adds later without a hook is still caught by the sweep within one tick (gone or dead) or by the scan test (a travel send).
- **Order on disconnect.** The duel ends before the entity is removed, so B's flag is cleared for the witnesses who still see B, and the partner leaves combat and hears 879 at once.
- **Races.** Everything runs on the cell's single task with `&mut SpaceManager`; no await sits between the registry check and the clear inside `end_engaged` (the registry entry is removed first, so a second end of the same duel is a no-op). The clamp and its end happen within one `apply_damage_to_target` or one `fire_pulse` call.

## Telemetry

No new log target (`duel` since SS-D1). No span inside a tick.

| Event | Level | Where | Fields beyond the ids | Test |
|---|---|---|---|---|
| `duel.ended` (extended) | DEBUG | `end_engaged` | `reason` (`forfeit`, `health`, `connection`, `range`, `teleport`, `engaged_limit`, `duelist_gone`, `gm_aborted`), `outcome` (`decided` or `aborted`), `loser_player_id`, `winner_player_id`, `defeat_reason` (`EDuelDefeatReason` value), and SS-D2's `cleared`, `effects_removed`, `space_id` | `forfeit_ends_the_duel_with_the_caller_as_loser` and every end-path test assert the ids and outcome |
| `duel.forfeit` | INFO span | CM 103 | `account_id`, `player_id`, `entity_id` | — |
| `duel.forfeit_refused` | DEBUG | forfeit | `reason = not_engaged`, `stage` (`none`, `challenge`, `countdown`, `engaged_elsewhere`), `duel_id`, `target_player_id` | `forfeit_rejected_when_not_engaged` (type 12) |
| `duel.forfeit_refused` | WARN | forfeit | `reason = not_a_player` | — (an entity that is not a connected player) |
| `duel.lethal_clamped` | DEBUG | the clamp | `source` (`ability`, `ability_script`, `effect_pulse`), `health_before`, `health_after = 1`, `target_entity_id`, `target_account_id` | `lethal_partner_hit_clamps_to_one_hp` |
| `duel.out_of_range` / `duel.back_in_range` | DEBUG | the tick | `distance`, `arena_radius`, `grace_ms` / `outside_ms` | `range_ends_duel` counts both |
| `duel.withdrawn` | DEBUG | leave paths | `stage` (`challenge`, `countdown`), `reason` (`connection`, `teleport`, `health`) | `leaving_withdraws_a_challenge_and_a_countdown` |

Every row carries `account_id`, `player_id`, `entity_id` for the actor (the attacker on a clamp, the leaver on a withdrawal, the challenger on `duel.ended`) and `target_player_id` for the other duelist, plus `duel_id`. `duel.ended` resolves both account ids before the teardown and, when an engaged entity is already gone, from the entity the player now has.

| Question | SigNoz query |
|---|---|
| How did player X's duel end? | `scope_name = 'duel' AND (player_id = X OR target_player_id = X) AND event = 'duel.ended'`: `reason`, `outcome`, `loser_player_id`, `winner_player_id` |
| Was a lethal hit held? | `event = 'duel.lethal_clamped' AND target_player_id = X`: `source`, `health_before` |
| Why could X not forfeit? | `event = 'duel.forfeit_refused' AND player_id = X`: `stage` |
| Did X leave the arena? | `event IN ('duel.out_of_range', 'duel.back_in_range') AND player_id = X` |

## Commands run

All from the worktree root through the lane, exit 0 unless stated; nothing self-skipped.

- `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell-world --all-targets`, then `-p cimmeria-cell-combat -p cimmeria-cell-methods --all-targets`, then `-p cimmeria-cell-console -p cimmeria-cell-content -p cimmeria-cell-interactions -p cimmeria-cell --all-targets`.
- `lane.sh cargo nextest run -p cimmeria-cell-world -p cimmeria-wire duel`: first run 56 passed, 1 failed (`space_change_without_the_hook_ends_as_teleport`: `duel.ended` lost the account id of a duelist whose engaged entity was gone; fixed in `end_engaged`), then all passed.
- `lane.sh cargo nextest run -p cimmeria-cell-combat duel`: 9 passed. `lane.sh cargo nextest run -p cimmeria-cell-methods duel`: 2 passed.
- `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-cell-content -p cimmeria-cell-interactions -p cimmeria-cell`: **3222 passed, 0 skipped**.
- `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-cell-content -p cimmeria-cell-interactions -p cimmeria-cell -p cimmeria-wireclient -p cimmeria-services --all-targets -- -D warnings`: clean.
- `lane.sh cargo fmt --all -- --check`: clean.
- `bash tools/build-lane/reload-db.sh` (into `sgw_ss_d3`), then `lane.sh bash -c 'export DATABASE_URL=…/sgw_ss_d3; cargo test -p cimmeria-wireclient --test it duel_two_duelists -- --test-threads=1'`: 1 passed (9.9 s, a real run: three sessions, the engage, then B's forfeit).
- No live-DB tier: this packet adds no SQL.

## Tests

| Test | Type | Guards |
|---|---|---|
| `forfeit_rejected_when_not_engaged` | 12 | CAT-M-14: no duel, a waiting challenge, the countdown, a bystander: 880 each, nothing changes, `stage` on each row |
| `forfeit_ends_the_duel_with_the_caller_as_loser` | 8 | 879 / forfeit line, both flags 0, 153, registry empty, `defeat_reason = 7` |
| `disconnect_ends_duel_and_clears_both_flags` | 8 | CAT-M-15: the leaver's flag cleared for its witnesses before it goes, the partner's flag, 153, combat, 879, `defeat_reason = 3` |
| `teleport_ends_duel` | 8 | CAT-M-15: `on_travel` → `Teleport`; a bystander's travel touches nothing |
| `space_change_without_the_hook_ends_as_teleport` | unit | the sweep backstop names the loser and the reason |
| `range_ends_duel` | 8 | CAT-M-15, D-SS19: one warning, no end at 4.999 s, the clock restarts after coming back, the end at 5 s |
| `third_party_death_loses_the_duel` | unit | `on_death` and the sweep's dead-duelist check; no clamp on a third party |
| `leaving_withdraws_a_challenge_and_a_countdown` | 12 | the response window and the countdown end on the leave paths, no cooldown |
| `every_end_path_clears_pvp_flag` | 8, table | nine rows (forfeit, clamp, death, disconnect, travel, range, limit, GM, hookless removal): own flag 0, every witness 0, 153, registry empty, one `duel.ended` with the row's reason |
| `every_travel_site_ends_the_duel` | source scan | every `TeleportPlayer` / `GateTravel` send has a `duel::on_travel` |
| `lethal_partner_hit_clamps_to_one_hp` | unit | D-SS20 ability path: 1 HP, every `onStatUpdate` says 1, the same hit's DoT stripped, `duel.lethal_clamped` fields |
| `no_loot_xp_or_corpse_after_a_clamped_end` | unit | no `BSF_Dead`, `ContactListPresenceEvent`, `GrantXP`, `onBeginAidWait` or "Target killed!"; `can_harm` false; combat dropped |
| `lethal_partner_bleed_clamps_to_one_hp` | unit | D-SS20 effect path (a DoT from the partner cannot kill): two partner DoTs due in one tick, one pulse fires, 1 HP, both stripped, no death; a bystander's DoT afterwards is not clamped |
| `third_party_kill_is_normal_death` | unit | a third party's kill: corpse, death fanout, not clamped, the duel lost on health |
| `duel_forfeit_routes_to_the_duel_handler` | unit | CM 103 reaches the handler (880), not the old stub |
| `constants_match_the_entity_defs`, `moniker_texts_match_the_seed` (extended) | 2 | the five `EDuelDefeatReason` values and 879/880 read back from the defs and the seed |
| `duel_engages_for_two_duelists_and_flags_both_for_a_spectator` (extended) | 11 (not in CI) | B's forfeit on the real wire: 153 to both, both flags 0 for the spectator, 879 to A |

SS-D2's `duelist_leaving_the_world_ends_the_duel` now expects the sweep's decided end (`reason = connection`, 879 to the survivor) instead of 878.

Audit § 6 names kept: `forfeit_rejected_when_not_engaged`, `disconnect_ends_duel_and_clears_both_flags`, `teleport_ends_duel`, `range_ends_duel`, `every_end_path_clears_pvp_flag`, `lethal_partner_hit_clamps_to_one_hp`, `lethal_partner_bleed_clamps_to_one_hp`, `third_party_kill_is_normal_death`. The packet's "no loot, XP or corpse" test is `no_loot_xp_or_corpse_after_a_clamped_end`.

## Regression proof

Each mutation applied by a script, the named tests run with `--no-fail-fast`, the file restored with `git checkout HEAD --` and touched (the tree was clean afterwards). Every one failed its guard:

| Mutation | Failed |
|---|---|
| M1 forfeit accepts a duel in any state | `forfeit_rejected_when_not_engaged` (no 880 during the countdown) |
| M2 no `on_disconnect` in `disconnect_entity` | `disconnect_ends_duel_and_clears_both_flags`, `every_end_path_clears_pvp_flag` |
| M3 `on_travel` a no-op | `teleport_ends_duel`, `every_end_path_clears_pvp_flag` |
| M4 the gate-travel site's hook removed | `every_travel_site_ends_the_duel` ("gate_travel/mod.rs: 1 travel sends, 0 duel hooks") |
| M5 no range loser | `range_ends_duel`, `every_end_path_clears_pvp_flag` |
| M6 forfeit removes the duel from the registry without `end_engaged` | `every_end_path_clears_pvp_flag` ("Forfeit: 10's own flag not cleared") |
| M7 no clamp after the direct damage | `lethal_partner_hit_clamps_to_one_hp` (HEALTH 0), `no_loot_xp_or_corpse_after_a_clamped_end` |
| M8 no clamp in `fire_pulse` | `lethal_partner_bleed_clamps_to_one_hp` (HEALTH 0) |
| M9 no `still_active` skip in the pulse tick | `lethal_partner_bleed_clamps_to_one_hp` (the second DoT killed) |
| M10 the duel ended right after the first clamp (before the DoT registration) | `lethal_partner_hit_clamps_to_one_hp` ("the DoT this hit registered is stripped by the end") |
| M11 no `on_death` in `resolve_death` | `third_party_kill_is_normal_death` |
| M12 the sweep ignores a dead duelist | `third_party_death_loses_the_duel` |
| M13 the leave paths do not withdraw | `leaving_withdraws_a_challenge_and_a_countdown` |
| M14 CM 103 back to the `UNIMPLEMENTED` stub | `duel_forfeit_routes_to_the_duel_handler` |

## Known gaps

- **A DoT still kills no player** outside a duel: `dot_kill_credit` returns for a player, so a player an NPC DoT takes to 0 HP stands at 0 HP without dying (the existing "DoT death" blocker in `project_enemy_combat_runtime_blockers`). For a duelist the tick ends the duel on health; the death itself is not this packet's.
- **Both duelists past the range grace** in the same tick: the challenger loses. No real path produces a simultaneous exit to the tick.
- **An in-arena GM teleport** of a duelist ends the duel as `Teleport`, by design: every teleport ends it.
- **The countdown splash** keeps counting on the client after a withdrawn countdown (878 is the only notice); no cancel form of the type-14 timer is known.
- Every duel number (40 units, 5 s, 10 minutes) is project policy, not recovered data.
- Type 11 does not run in CI (audit A-60).

## Contended files touched

- `crates/cell-world/src/cell/space_manager/entities.rs`: one call, `duel::on_disconnect`, in `disconnect_entity` (SS-D3 only).
- `crates/cell-combat/src/cell/abilities/damage_apply/mod.rs` and `crates/cell-combat/src/cell/effects/pulsing/tick.rs`: the clamp (SS-D3 only).

## Other files touched

- `crates/cell-world/src/cell/duel/{end.rs, mod.rs, registry.rs, limits.rs, tick.rs}`, `tests/{mod.rs, engage.rs}`.
- `crates/cell-combat/src/cell/abilities/death/mod.rs` (`on_death`), `use_ability/tests/{mod.rs, duel_gate.rs, duel_end.rs}` (fixtures made `pub(super)`).
- `crates/cell-methods/src/cell/cell_methods/player/social.rs` (the CM 103 arm and its test).
- One `duel::on_travel` call before each travel send: `cell-console/.../gm/travel.rs` (4), `placement.rs`, `travel/mod.rs`, `cell-content/.../executor/transport.rs` (2), `ring_transport/dispatch.rs` (2), `cell-interactions/.../gate_travel/mod.rs`, `respawn/mod.rs`, `space_transfer/mod.rs`. These are other campaigns' files; each edit is the one call and a two-line comment.
- `crates/wire/src/cell/client_methods/duel.rs`, `crates/wireclient/tests/it/duel_two_duelists_and_a_spectator.rs`.

## Docs

- `docs/gameplay/duel-system.md`: status, "The end of a duel (SS-D3)" with the path table, the feature rows, RE priorities 4 and 5.
- `docs/architecture/abilities-and-effects-system.md`: decision 26 (the clamp in both seams, the ordering, the pulse skip); decision 24's consequence line.
- `docs/game-systems.md` § Dueling, `docs/protocol/message-catalog.md` (DuelForfeit YES; Dueling NetOut 3 of 3, 86%).
- `docs/gap-analysis.md` and `docs/project-status.md` are **not** edited (owner rule, close-out only); the deltas are under "Close-out edits for SS-99".

## Integration edits for the coordinator

- **SS-U2 (#910):** `end_engaged`'s signature is unchanged; `EndReason::GmAborted` still aborts with 878 to both. SS-D2's integration note still applies (route an `Engaged` duel through `end_engaged(..., EndReason::GmAborted)`, and `DuelState::Engaged { .. }` in `DuelState::name`). If `.duel_end` should name a loser, it can pass `EndReason::Defeated { loser, reason }`; no GM reason exists in `EDuelDefeatReason`, so abort is the right default. Any new GM teleport command must call `duel::on_travel` before its `TeleportPlayer` or `every_travel_site_ends_the_duel` fails.
- **Pets:** no pet behaviour changed. The pets campaign's travel scan and this one sit side by side; a new travel site needs both hooks.
- Ledger: SS-D3 → Review; audit § 6 CAT-M-14, CAT-M-15 and the non-lethal rows are covered by the tests above.

## Rebase onto `origin/main` @ `23e97ac58` (SS-U2 #910, SS-M2 #912)

- Conflicts only in `duel/{mod.rs, registry.rs, tests/mod.rs}` (SS-U2's `gm` module and `GmAborted` beside this packet's `forfeit`, `paths` and `Withdrawn`); both sides kept.
- SS-U2's `duel::gm::gm_end` calls `end_engaged(tx, mgr, duel_id, EndReason::GmAborted)`, which still aborts with 878 to both; its tests pass. SS-U2 added no `TeleportPlayer` / `GateTravel` site (`every_travel_site_ends_the_duel` passes).
- `gap-analysis.md` and `project-status.md` restored to main's version (owner rule); the deltas are below.
- After the rebase: `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-combat -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-cell-content -p cimmeria-cell-interactions -p cimmeria-cell -p cimmeria-wireclient`: 3298 passed, 1 skipped (SS-U2's `#[ignore]` sparbot keep-alive test; the wireclient DB tests self-skip without `DATABASE_URL` in that run). Clippy on those crates plus `cimmeria-services`, `--all-targets -D warnings`: clean. fmt: clean. `reload-db.sh`, then `cargo test -p cimmeria-wireclient --test it duel -- --test-threads=1` with `sgw_ss_d3`: 3 passed (this packet's forfeit extension and SS-U2's two sparbot tests), 1 ignored.

## Close-out edits for SS-99

For `docs/gap-analysis.md` and `docs/project-status.md`, which packets no longer edit:

- **gap-analysis § 27 rows:**
  - `Duel forfeit`: KM → IM. Code: `cell/duel/forfeit.rs`. Notes: "SS-D3: engaged only (880 otherwise), the caller loses, 879 to the partner. Unit, type 12 and type 11 tested; not yet client-tested".
  - `Defeat conditions`: KM → IM, Blocks `--`. Code: `cell/duel/end.rs, paths.rs; damage_apply; effects/pulsing/tick.rs`. Notes: "SS-D3: Health (the 1 HP clamp, or a third-party death), Connection, Range, Teleport, Forfeit. LeftSquad and InDuel have no path while squad duels are refused. Unit and type 8 tested; not yet client-tested".
  - Duel marker entity: stays KM (D-SS24).
- **§ 27 "Rust code" bullet:** replace "`duelForfeit` (CM 103) still logs `UNIMPLEMENTED`; health, range, disconnect and teleport ends are SS-D3" with: SS-D3 added every end path through `end_engaged` (879 to the winner, a line to the loser): forfeit (CM 103, engaged only, else 880), the non-lethal 1 HP clamp in the ability and effect-pulse paths (D-SS20), death from anyone else, disconnect, every teleport and gate travel, and range (40 units for 5 s, D-SS19), each logged with the client's `EDuelDefeatReason`.
- **Matrix:** Dueling 6 = 5 IM + 1 KM (was 3 IM + 3 KM): IM +2, KM −2 on the totals line, then recompute the summary percentages from the rows.
- **System-docs row:** Dueling: "Challenge, response, countdown, engaged duel, PvP flag, harm gate (SS-D1, SS-D2) and every end path, non-lethal (SS-D3); no `SGWDuelMarker` (D-SS24)".
- **project-status Dueling row:** status IM, "6 (5 IM, 1 KM)", "Challenge, response, the countdown, the engaged duel and every end implemented (SS-D1 to SS-D3): duelists are PvP-flagged, can damage each other and only each other, and are in combat together. A duel ends on forfeit, on partner damage that would kill (held at 1 HP instead, D-SS20), on death from anyone else, disconnect, teleport or range; the winner hears 879, nothing is awarded (D-SS22). The duel marker is not ported (D-SS24)".
- Already in this PR (not close-out files): message-catalog DuelForfeit YES and Dueling coverage 86%; game-systems § Dueling.

## Open questions

- Settled by the coordinator: a GM `.duel_end` stays an abort with no loser (878).
- Should a DoT kill a player outside a duel? That is the combat campaign's call; the duel is safe either way.
