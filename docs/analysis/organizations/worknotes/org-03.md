# ORG-03 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-01.md](org-01.md), [org-e1.md](org-e1.md).

## Contract

- **Packet:** ORG-03, squad core.
- **Decisions in force:** D-ORG03 (squads on the cell, service-wide, never persisted), D-ORG05 (squad ids from `0x4000_0000`, never reused; routing is not authorization), D-ORG06 (invites keyed by invitee `player_id` + request id, consumed on first response, 60 s, re-validated; cell request ids with bit 29 clear), D-ORG07 (squad ranks Member 2 / Leader 8), D-ORG12 (the longest-standing member inherits the lead), D-ORG16 (APPROVED: leader-only loot, 0 or 1 only; a non-leader gets an error, a line and the current [51]), D-ORG18.
- **Audit rows read:** A-10, A-16, A-18, A-19, A-23, A-24, A-30, A-41.
- **Base:** stacked on `origin/org/01-foundation` @ `36468911`; #871 squash-merged, so rebased with `git rebase --onto origin/main 36468911` onto `origin/main` @ `59c73019`, later onto `76613bf3`, and on 2026-09-27 onto `origin/main` @ `88d7da73` (SS-00, #880; see "Rebase onto 88d7da73"). Branch `org/03-squad`, worktree `.claude/worktrees/org-03`, test database `sgw_org_03`.
- **Owned paths:**
  - `crates/cell-world/src/cell/squad/` (new: `mod.rs`, `registry.rs`, `invites.rs`, `tests.rs`), `cell/mod.rs`, `space_manager/mod.rs` (the `squads` field), `space_manager/queries.rs` (`player_entity_by_player_id`)
  - `crates/entity/src/cell_entity/entity_struct.rs`, `construction.rs` (`squad_id`)
  - `crates/cell-methods/src/cell/cell_methods/organization/` (was `organization.rs`: `mod.rs` router, `forward.rs`, `squad/` with `fanout.rs`, `feedback.rs`, `invite.rs`, `loot.rs`, `membership.rs`, `world_entry.rs`, and `tests/`), `cell/mod.rs` (re-exports `squad`)
  - `crates/cell/src/cell/service/base_messages/org.rs`, `mod.rs` (the `Org` arm and the `InitPlayerState` call), `lifecycle.rs` (the `DisconnectEntity` call), `tests/org.rs`
  - `crates/base/src/base/dispatch/organization.rs`, `dispatch/mod.rs` (one argument), `dispatch/tests/organization.rs`
  - `crates/wireclient/tests/it/two_client_squad.rs`, `tests/it/main.rs`
  - `crates/server/src/logging/target_scan_tests.rs` (two pins)
  - Docs: `docs/gameplay/group-system.md` (new Squads section), `organization-system.md`, `docs/gap-analysis.md` §23, `docs/project-status.md`, `docs/architecture/wireclient.md`, `TESTING.md` (wireclient test count), this worknote.
- **Read set:** the ledger sections named in the task; `worknotes/org-01.md`, `org-e1.md`; `crates/cell-world/src/cell/space_manager/{mod,queries,entities}.rs`; `crates/cell/src/cell/service/base_messages/{mod,lifecycle,org}.rs`, `player_init/mod.rs`; `crates/base/src/base/dispatch/{mod,organization}.rs`; `crates/base-session/src/base/helpers/mod.rs:430-460` (`destroy_client_entities`); `crates/base-world-entry/src/base/world_entry/gate_travel/mod.rs`; `crates/wire/src/cell/client_methods/organization/builders.rs`; `crates/wireclient/tests/it/{two_client_castle_visibility.rs,support/mod.rs}`; `docs/architecture/negative-logging-convention.md`.

## Evidence

- **Only `DisconnectEntity` ends a session on the cell.** `SpaceManager::disconnect_entity` is called only from the `DisconnectEntity` arm (`crates/cell/src/cell/service/base_messages/lifecycle.rs`). The base sends it from `destroy_client_entities` (`crates/base-session/src/base/helpers/mod.rs:452`, which covers log off, crash, timeout and duplicate login), `logOff` (`crates/base/src/base/dispatch/session.rs:61`) and an abandoned gate transfer (`gate_travel/mod.rs:82`). `DestroyEntity` is also the gate-travel teardown, so the squad hook is on `DisconnectEntity` only; `destroy_entity_keeps_the_squad` pins that.
- **Gate travel re-creates the cell entity** with no `character_name` until `InitPlayerState` (`space_manager/queries.rs:330-340`), and `InitPlayerState` is sent on every world entry, gate arrivals included (the `known_stargates` comment in `base_messages/mod.rs`). So membership is keyed by `player_id` and replayed from `InitPlayerState`.
- **No ignore list exists on the cell.** The contact list (friends and ignores) is base-side database state (`crates/base-session/src/base/contact_list/`); no cell structure holds it. The ORG-03 "target is not ignoring the inviter" check is not made (see Known gaps).
- **ORG-E1 Q1 and Q6** set the fanout order ([38] before the [37]s that set member ids) and why nothing is sent to the other members on a world entry (squad frames follow entity presence).

## Design decisions

1. **The registry is a field of `SpaceManager`** (`SpaceManager::squads`), not a separate object threaded beside it. The packet says "beside `SpaceManager`, owned by the cell service (not per space)". `SpaceManager` is the single service-wide container every cell handler already receives (it holds `gm_aggro_off`, keyed by `player_id`, the same way), so this satisfies D-ORG03 without changing the signature of every cell-method dispatcher. The type lives in its own module, `cimmeria_cell_world::cell::squad`, with no I/O.
2. **Time is injected** into the registry (`now: Instant`); the handlers pass `Instant::now()`. The expiry and rate-window edges are tested exactly in the registry (59.999 s accepted, 60 s refused).
3. **Member entities are resolved live per event** (`player_entity_by_player_id`, O(online players)), never cached: entity ids are recycled and gate travel re-creates the entity. A member who resolves to nothing is in transit. Bystander messages to them are skipped (their world entry replays the squad), and a terminal [36] owed to them is queued in the registry (`owe_left`) and delivered on their next `InitPlayerState`. This closes the hole the `social-systems-engineer` review found: a kick during transit would otherwise be lost silently.
4. **Creation.** An invite from a squadless player records `squad_id: None`. The first accept founds the squad with the inviter as leader; a later accept of another invite from the same inviter joins the squad the first one created (the inviter now leads it). Both founders get the full join sequence.
5. **Accept re-validation** (D-ORG06): the invitee is still squadless; for an invite into a squad, the squad exists, the inviter is still in it (`InviterLeft`) and still leads it (`InviterNotLeader`); there is room below 6; for creation, the inviter is online (`InviterOffline`).
6. **Leave ordering.** Disband is evaluated before promotion, so a leader leaving a pair promotes nobody (review finding). The last member gets [39] for the leaver, then [36] `Disbanded`.
7. **Kick** finds the target by name among the squad's own members, so a member in transit can be kicked. A kick naming a squad the actor is not in is WARN (a forged id); not-leader, not-a-member and self-kick are DEBUG.
8. **The new leader** is announced with `onMemberRankChangedOrganization` [40] (rank 8) to the remaining members; no other message exists for it.
9. **World entry replay** sends [35] (`aNewMember` 0), [38], a [37] per other member and [51] to the arriving member only, and re-stamps `CellEntity::squad_id`. On a first login the player is in no squad and nothing is sent.
10. **Feedback.** Every refusal is `onErrorCode(0, squad id or 0, 0)` plus a feedback-channel line; `forward::send_error_and_line` is shared with ORG-01's "not available yet" answer. A sent invite confirms to the inviter ("You invited X to your squad."), and a decline tells the inviter. The wording is policy (audit A-14).
11. **Log levels** (target `squad`): membership changes at INFO (`squad.joined`, `squad.left`, `squad.kicked`, `squad.leader_promoted`, `squad.disbanded`, `squad.loot_changed`); forged shapes at WARN (`squad.response_rejected` `unknown_request`, `squad.leave_rejected` `foreign_squad_id`, `squad.kick_rejected` `not_in_that_squad`, `squad.loot_rejected` `out_of_range`, `squad.invite_rejected` `target_ambiguous`, `squad.actor_mismatch`); ordinary refusals at DEBUG. The base logs `org.invite_by_type_rejected` (`type_out_of_range`, WARN), `org.squad_forwarded` (DEBUG) and `org.squad_forward_failed` (WARN). `("squad", INFO)` and `("squad", WARN)` are pinned in `scan_finds_known_targets`.
12. **Router.** CM 8 goes to the squad handler unless the request id routes to the base (bit 29); CM 9 unless the org id routes to the base; CM 18 always. An id that routes nowhere (<= 0) reaches the squad check and is refused there as foreign. Everything else keeps ORG-01's answer in `forward.rs`.
13. **Base.** A forwarded call carries the session's `player_id` and `entity_id`; the cell re-checks that the entity is still that character (`squad.actor_mismatch`, dropped), because entity ids are recycled. If the cell channel is gone, the base answers with ORG-01's pair.
14. **Wireclient test sentinels** are `0x7000_03xx` (accounts `0x7000_0311` and `0x7000_0312`, characters `0x7000_0301` and `0x7000_0302`), cleaned up by exact id before and after.

## Advisor reviews

- `social-systems-engineer` (design) found the transit hole (decision 3) and the promote-then-disband ordering (decision 6), and confirmed [40] for the new leader and the [39]-then-[36] order. It also flagged the missing ignore-list check against the ledger text; the coordinator's task scopes that to "if one exists in the cell; say so if not".
- `testing-validation-engineer` (test plan) made the replay and foreign-id guards non-vacuous (the replay via decline-then-accept and accept-leave-replay; the foreign id with the answering player holding their own invite, and the victim's invite still usable afterwards). It also added the invite-issuing negatives, the disconnect and world-entry handler tests, the router boundaries, exact levels, and the counter-near-limit tests.

## Commands run

All from the worktree root, through the lane (`target=B:\targets/org-03`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo test -p cimmeria-cell-world --lib squad` (first run) | 101 | 2 test-setup bugs (the fixture hit the rate limit; an invitee was already squadded); fixed the tests |
| `lane.sh cargo check -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base --all-targets` | 0 | |
| `lane.sh cargo clippy -p cimmeria-entity -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-wireclient -p cimmeria-server --all-targets -- -D warnings` | 101, 101, 0 | `absurd_extreme_comparisons` (registry), `cloned_ref_to_slice_refs` (a test); fixed |
| `lane.sh cargo clippy -p cimmeria-services -p cimmeria-cell-console -p cimmeria-cell-interactions -p cimmeria-cell-content -p cimmeria-cell-combat -p cimmeria-lab-mcp -p cimmeria-admin-api -p cimmeria-base-world-entry -p cimmeria-base-session --all-targets -- -D warnings` | 0 | dependents unaffected |
| After the rebase: `lane.sh cargo fmt --all -- --check` | 0 | |
| After the rebase: `lane.sh cargo test -p cimmeria-entity -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base --lib --no-fail-fast` | 0 | base 77, cell 451, cell-methods 261, cell-world 339, entity 341 |
| After the rebase: `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` | 0 | 52 passed |
| After the rebase: the seven-crate clippy above | 0 | |
| `reload-db.sh`, then `DATABASE_URL=postgres://w-testing:w-testing@localhost:5433/sgw_org_03 lane.sh cargo test -p cimmeria-wireclient --test it two_client_squad -- --test-threads=1` | 0 | 1 passed in 5.9 s, not skipped (run before and after the rebase) |
| `live-db-test.sh base_messages organization squad` | 0 | 218 run, 218 passed (the rest filtered out) |

The two-client test is not in CI's live-DB job (`tools/test-live-db.sh` runs lib tests only, audit A-41).

## Regression proof

Run A: eight reverts applied together on the committed tree, then `lane.sh cargo test -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base --lib --no-fail-fast -- squad organization org::` exited 101.

| Revert | Failing guards |
|---|---|
| The leader check in `SquadRegistry::set_loot` | `loot_mode_rejects_non_leader`, `loot_mode_is_leader_only_and_range_checked` |
| `take_invite` reads instead of removing (no consume) | `invite_response_rejects_replay`, `decline_tells_the_inviter`, `invites_are_single_use_and_keyed_by_invitee`, `squad_ids_are_never_reused` |
| The room re-check in `accept` | `squad_accept_rejects_when_full`, `accept_rejects_the_seventh_member` |
| The base's type > 2 gate | `invite_by_type_rejects_type_above_command` |
| The own-squad check in `leave` (`!= Some(org_id)` to `is_none()`) | `squad_leave_rejects_foreign_squad_id` |
| The per-inviter rate limit | `invite_rate_limit_slides` |
| The `on_disconnect` call in the `DisconnectEntity` arm | `disconnect_entity_removes_the_squad_member` |
| The `on_world_entry` call after `InitPlayerState` | `init_player_state_replays_the_squad` |

Run B: two reverts, then `lane.sh cargo test -p cimmeria-cell-world -p cimmeria-cell-methods --lib --no-fail-fast -- squad organization` exited 101.

| Revert | Failing guards |
|---|---|
| `take_invite` matches the request id alone (no composite key) | `invite_response_rejects_foreign_request_id`, `invites_are_single_use_and_keyed_by_invitee` |
| An out-of-range loot mode keeps the current mode instead of refusing | `loot_mode_rejects_out_of_range`, `loot_mode_is_leader_only_and_range_checked` |

Run C: CM 8 routed to `forward::answer` instead of the squad handler; the wireclient test exited 101 ("Alpha never received [37]").

Each run was restored with `git checkout -- crates`, and `git status` was clean afterwards.

## Telemetry (owner rule, 2026-09-27; round 2)

Implemented per `work-packets.md` § "Telemetry (owner rule, 2026-09-27)" (PR #878) and the ORG-03 "Telemetry:" line, with the Copilot-review refinements relayed by the coordinator: the actor's identity on every row, a target only where a second player exists (invite, response, kick, leader change), and every outcome row at INFO, `ok` included. Code: [`squad/telemetry.rs`](../../../../crates/cell-methods/src/cell/cell_methods/organization/squad/telemetry.rs); counter helper `cimmeria_cell_world::cell::squad::count_action` (cell-world already depends on `cimmeria-observability`, so no new dependency edge and no hakari change).

| Kind | Event | Level | Fields | SigNoz filter |
|---|---|---|---|---|
| Span | `squad.invite`, `squad.invite_response`, `squad.leave`, `squad.kick`, `squad.loot_mode` | INFO | `skip_all`; correlators only (`player_id`, `entity_id`, `request_id`, `accept`, `squad_id`, `loot_mode`) | traces: span name = the event |
| Outcome | `squad.invite` | INFO | `outcome`, `reason`, actor and target identity, `squad_id`, `request_id` | `event = 'squad.invite'` (refusals: `AND outcome = 'rejected'`) |
| Outcome | `squad.invite_response` | INFO | as above; the target is the inviter | `event = 'squad.invite_response'` |
| Outcome | `squad.leave` | INFO | actor only, `squad_id` | `event = 'squad.leave'` |
| Outcome | `squad.kick` | INFO | actor, the kicked member as target, `squad_id` | `event = 'squad.kick'` |
| Outcome | `squad.loot_mode` | INFO | actor only, `squad_id` | `event = 'squad.loot_mode'` |
| Transition | `squad_created`, `member_joined` (`rank`), `member_left` (`reason`), `leader_changed` (`from_player_id`, `to_player_id`, target = new leader), `loot_mode_changed` (`from`, `to`), `disbanded` (`reason`), `invite_created`, `invite_consumed` (`accepted`), `invite_expired` | DEBUG | actor identity, `squad_id`, `request_id` where relevant | `event = 'member_left' AND squad_id = <id>` (any transition by its name) |
| Seam | `squad.actor_mismatch` | WARN | claimed `player_id`, `entity_id` | `event = 'squad.actor_mismatch'` |
| Seam | `squad.send_failed` | WARN | `entity_id`, `method_index`, `reason = cell_to_base_closed` | `event = 'squad.send_failed'` |
| Seam (base) | `org.squad_forward_failed` + the squad outcome row with `reason = cell_unreachable` | WARN + INFO | `account_id`, `player_id`, `entity_id`, `kind` | `event = 'org.squad_forward_failed'` |
| Outcome (base) | `org.invite_by_type` (type above 2) | INFO | `outcome = rejected`, `reason = org_type_invalid`, `org_type`, identity | `event = 'org.invite_by_type'` |
| Metric | `squad_actions_total{action, outcome, reason}` | counter | `reason = none` on `ok` | metrics: `squad_actions_total` grouped by `reason` |

Reasons (closed): `target_ambiguous`, `target_in_transition`, `target_not_found`, `self_target`, `not_a_player`, `squad_full`, `already_in_squad`, `not_leader`, `rate_limited`, `invite_limit` (one per pair, or five pending), `invite_unknown`, `invite_expired`, `invite_foreign`, `loot_mode_invalid`, plus the ones this packet needed that the ledger line did not list: `not_in_squad`, `wrong_squad`, `target_not_in_squad`, `inviter_left`, `inviter_not_leader`, `inviter_offline`, `squad_gone`, `ids_exhausted`, `not_ready`, `actor_mismatch`, `cell_unreachable`. `ignored` is reserved: the cell has no ignore list. To support `invite_expired` and `invite_foreign`, `SquadRegistry::take_invite` now returns `Result<_, TakeMiss>` (checking the entry before the purge) and records expiries for `drain_expired`.

Other changes in the round: the old per-refusal WARN/DEBUG rows (`squad.invite_rejected`, `squad.response_rejected`, ...) and the INFO membership rows (`squad.joined`, `squad.leader_promoted`, ...) were replaced by the outcome rows and transitions; the squad refusal pair is now sent through `fanout::send`, so a dropped send warns on `squad` rather than `org`; a disconnect is not a player action and logs only its transitions (`member_left`, `reason = logout`). The CAT-M negative-log tests now assert the INFO outcome row and its reason.

Telemetry tests (`organization/tests/squad_telemetry.rs`): `every_action_emits_exactly_one_outcome_row`, `a_refusal_is_one_info_row_with_a_reason`, `join_transitions_are_logged`, `departure_transitions_carry_before_and_after`, `loot_mode_changed_logs_from_and_to`, `expired_invite_is_logged_as_expired`, `actor_mismatch_warns_and_is_rejected`, `dropped_send_warns`; base `squad_forward_failure_warns_and_logs_the_outcome`; registry `invite_expires_at_sixty_seconds` (now pins `TakeMiss::Expired` and `drain_expired`).

Telemetry regression proof, on the committed tree then restored with `git checkout HEAD -- crates`:

| Revert | Failing guards |
|---|---|
| `leave` fabricates a target, and a successful loot change emits no outcome row | `every_action_emits_exactly_one_outcome_row` |
| The `squad.send_failed` WARN renamed away | `dropped_send_warns` |
| An expired entry reported as unknown | `expired_invite_is_logged_as_expired`, `invite_expires_at_sixty_seconds` |
| The base's `org_type_invalid` reason changed, and its squad outcome row moved off the `squad` target | `invite_by_type_rejects_type_above_command`, `squad_forward_failure_warns_and_logs_the_outcome` |

Round-2 commands (all exit 0 unless noted): `lane.sh cargo test -p cimmeria-entity -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base --lib --no-fail-fast` (base 78, cell 451, cell-methods 269, cell-world 339, entity 341); `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` (52); the seven-crate clippy with `-D warnings`; `cargo fmt --all -- --check`; the wireclient two-client test against `sgw_org_03`; the two revert runs above (exit 101 each).

Process note: during the first telemetry revert run I restored with `git checkout -- crates` while the telemetry edits were still uncommitted, which discarded them; they were re-applied from the same scripts and re-tested before the revert run was repeated on a commit. Nothing from the lost state reached a commit.

## Rebase onto 88d7da73 (round 3)

SS-00 (#880) landed `OnlinePlayerIndex`, the rate limiter and `send_feedback_line` in `crates/base-session`, and two fields on `ConnectedClientState`. ORG-03 builds none of those fixtures, so only two text conflicts came up, both resolved by keeping both sides:

- `crates/server/src/logging/target_scan_tests.rs`: ORG-03's `("squad", INFO)` / `("squad", WARN)` pins and SS-00's `chat`, `rate_limit` and `online_index` pins.
- `docs/architecture/observability.md`: ORG-03's extended `org` / `squad` row, then SS-00's new `rate_limit`, `online_index` and `chat` rows.

All from the worktree root, through `tools/build-lane/lane.sh` (`target=B:	argets/org-03`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo fmt --all -- --check` | 0 | |
| `lane.sh cargo clippy -p cimmeria-entity -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base -p cimmeria-wireclient -p cimmeria-server --all-targets -- -D warnings` | 0 | |
| `lane.sh cargo test -p cimmeria-entity -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base --lib --no-fail-fast` | 0 | base 87 (78 plus SS-00's), cell 451, cell-methods 269, cell-world 339, entity 341 |
| `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` | 0 | 52 passed |
| `reload-db.sh` (sgw_org_03), then `DATABASE_URL=.../sgw_org_03 lane.sh cargo test -p cimmeria-wireclient --test it two_client_squad -- --test-threads=1` | 0 | 1 passed in 7.0 s, not skipped |
| `live-db-test.sh base_messages organization squad` | 0 | 227 run, 227 passed, 3543 filtered out |

Spot-check of the regression proof after the rebase, on the committed tree: the base's type > 2 gate disabled (`if false && ...`) and the `on_disconnect` call removed from the `DisconnectEntity` arm, then `lane.sh cargo test -p cimmeria-cell -p cimmeria-base --lib --no-fail-fast -- organization org:: disconnect` exited 101 with `invite_by_type_rejects_type_above_command` and `disconnect_entity_removes_the_squad_member` failing (`identity_propagation::disconnect_carries_identity_resolved_before_teardown` also failed in that threaded run; it passes with the same revert under `--test-threads=1`, so it is the known threaded LogCapture flake, not a squad dependency). Restored with `git checkout HEAD -- <the two files>`; `git status` clean.

## Known gaps

- **No ignore-list check on invite.** Ignore lists are base-side database rows; the cell has no copy. A later packet can either have the base check the target's ignore list by name before forwarding `SquadInvite` (it has the inviter's `player_id` and the database), or have `InitPlayerState` carry the ignore set to the cell. The base-side check is the better fit, because ORG-07 needs the same check for Team and Command invites.
- **No manual promote.** `/squadpromote` is inferred to use `organizationRankChange` (0xD2), unconfirmed (ORG-E1 follow-up 1). 0xD2 with a squad id keeps ORG-01's answer. It would need an `OrgBaseToCell::SquadPromote` variant.
- **0xD1 with an id <= 0** keeps ORG-01's "not available yet" answer rather than a squad refusal; ORG-07 owns the base route.
- **Roster level and archetype are a snapshot** taken at join; a level-up is not re-sent until a rejoin or a world-entry replay.
- **An expired invite answer is logged at WARN** as `unknown_request`, the same as a forged or replayed id: the registry drops expired entries, so it cannot tell them apart.
- **An inviter in gate transit** cannot found a squad (`InviterOffline`, "X is no longer online."), and misses the [37] for a newcomer joining their existing squad until their world-entry replay.
- `crates/entity/src/cell_entity/entity_struct.rs` was already 747 lines (over the 700-line hard cap) before this packet; it is 754 now. Not split here.
- Not tried with a real client. The solo fallback of ORG-UAT steps 1-4 needs ORG-04's `.squad_join`.

## Integration edits for the coordinator

- Ledger: record decision 1 (the registry is `SpaceManager::squads`) and the ignore-list gap against ORG-03's "not ignoring the inviter" line; ORG-07 inherits it.
- `crates/cell-methods/src/cell/cell_methods/organization/` is split. ORG-07 replaces `forward::answer` with the base forward and adds files beside `squad/`; ORG-04 adds squad chat and the `.squad_*` commands on top of `SpaceManager::squads` and `CellEntity::squad_id`.
- ORG-04's squad chat can read `SpaceManager::squads.squad_for(player_id)` and resolve recipients with `player_entity_by_player_id`, the same way `squad/fanout.rs` does.
