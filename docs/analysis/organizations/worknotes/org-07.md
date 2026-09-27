# ORG-07 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-02.md](org-02.md), [org-03.md](org-03.md), [org-05.md](org-05.md), [org-06.md](org-06.md).

## Contract

- **Packet:** ORG-07, invite, kick and rank change for Teams and Commands; `broadcast_to_org` and the non-squad CM 8-19 forward (ORG-API).
- **Decisions in force:** D-ORG04 (ORG-LOCK), D-ORG06 (pending invites, `BASE_INVITE_REQUEST_FLAG`), D-ORG07 (entry ranks), D-ORG09 (rank authority), D-ORG13 (GM commands re-read access on the base), D-ORG18 (one Team and one Command), D-ORG22 and D-ORG23 (read; no text or mask edits in this packet).
- **Coordinator notes applied:** the base-side Ignore check for squad invites and for Team/Command invites (SS-C1's session `IgnoreCache`, no second lookup); `OrgMembershipEnded { reason: Kicked }` on a kick; `broadcast_to_org` after commit with `recipients` and a WARN per failed send, built on ORG-06's fanout helpers; CM 19 forwarded, the base arm stays the reject stub; the creation race fixed (below); `.org_join` / `.org_rank` in the ORG-06 `.org_disband` pattern; feedback lines through the feedback helpers only; no edits to `gap-analysis.md`, `project-status.md` or test counts.
- **Base:** `origin/main` @ `d62132606` (ORG-05 #942). Branch `org/07-invite-kick-rank`, worktree `.claude/worktrees/org-07`, test DB `sgw_org_07`. Rebased before push (see "Commands run").
- **Sentinels:** `0x7000_4F00..=0x7000_4FFF` and `0x7000_5200..=0x7000_53EF` for the base-session handler tests (`Fixture::org07`, blocks 0-46 of 16, 33 used), `0x7000_53F0..=0x7000_53FF` for the wireclient test. ORG-06 keeps `0x7000_4C00..=0x7000_4DFF`, ORG-05 `0x7000_4E00..=0x7000_4EFF`; `0x7000_5000..=0x7000_51FF` belongs to other suites.

## Owned paths

- Base session: `crates/base-session/src/base/organization/invites/` (new: `mod.rs`, `tests.rs`), `organization/handlers/` (new: `answer.rs`, `broadcast.rs`, `gm.rs`, `invite.rs`, `invite_response.rs`, `kick.rs`, `rank.rs`, `targets.rs`; edited: `mod.rs`, `telemetry.rs`, `disband.rs`), `handlers/tests/` (new: `invite.rs`, `invite_response.rs`, `kick.rs`, `rank.rs`, `broadcast.rs`, `gm.rs`, `lock_race.rs`, `org07_support.rs`; `mod.rs` gains `Fixture::org07`), `organization/{mod.rs,api.rs}`, `persistence/members.rs` (the advisory lock), `base/mod.rs` (`ConnectedClientState::org_invites`), `test_fixtures.rs`.
- Base: `crates/base/src/base/dispatch/organization.rs` (rewritten), `organization_squad.rs` (new: the squad forward moved out, plus the Ignore check), `dispatch/mod.rs` (`db_pool` passed), `dispatch/session.rs` (`logOff` clears held invites), `dispatch/tests/organization.rs`, `dispatch/tests/org_invite_logoff.rs` (new), `login/mod.rs`.
- Base world entry: `cell_dispatch/org_dispatch.rs` (CM 8, CM 10/13-17 and CM 19 answers, `GmJoin`, `GmRank`), `tests_dispatch_arms/org_arms.rs`.
- Wire: `crates/wire/src/cell/messages/org_cell_to_base.rs` (`GmJoin`, `GmRank`).
- Cell: `crates/cell-methods/src/cell/cell_methods/organization/{mod.rs,forward.rs}`, `tests/{router.rs,squad_ping_gm.rs}`; `crates/cell-console/src/cell/console/{org.rs,dispatch.rs,mod.rs}`, `registry/commands/org.rs`, `tests/org07_join_rank.rs` (new), `tests/mod.rs`.
- Wireclient: `crates/wireclient/tests/it/two_client_command_invite.rs` (new), `main.rs`.
- `ConnectedClientState` literal sites (one new field): `base-methods` `inventory/appearance.rs`, `progression/tests.rs`; `base-world-entry` `play_character.rs`, `gate_travel/tests/mod.rs`; `services` `gate_round_trip_tests/dial_to_gate_travel.rs`.
- Docs: `docs/gameplay/organization-system.md` (status, new "Invite, kick and rank change (ORG-07)" section), `docs/gameplay/group-system.md` (Ignore row), `docs/architecture/observability.md` (`org` / `squad` row), `docs/architecture/wireclient.md`, `docs/commands.md`, `docs/protocol/sgwplayer-base-method-dispatch-table.md`, `docs/protocol/client-method-dispatch-table.md`, this worknote.

## Read set

The ORG-07 packet, contract, ORG-API, CAT-M table, telemetry section and carried gaps of `work-packets.md`; D-ORG04/06/07/09/13/18/22/23; worknotes org-02, org-03, org-05, org-06; `base/organization/{api.rs,persistence/*,handlers/*}`; `base/dispatch/{organization.rs,duel.rs,tell.rs}`; `base/contact_list/ignore/`; `base/player_index/`; `cell_methods/organization/{mod.rs,forward.rs}`; `cell-world/src/cell/squad/invites.rs` (the limits reused); `base-world-entry` `org_dispatch.rs`; `cell-console/console/org.rs`; `wireclient/tests/it/two_client_squad.rs`.

## Design decisions

1. **Pending invites live on the invitee's session** (`ConnectedClientState::org_invites`, `organization::invites::OrgInviteState`), not in a global map. Removing the session (every `destroy_client_entities` path) removes them, which is D-ORG06's "cleared on every disconnect path"; `logOff` (both variants) clears them explicitly because a return to character select keeps the session (authority review). Entries carry the invitee's `player_id`, and only entries for the current character count toward the caps. The inviter's rate-limit history is on the inviter's session and survives a character switch.
2. **Request ids** are `BASE_INVITE_REQUEST_FLAG | n` from one process-wide `AtomicI32`, `n` in `1..2^29`; exhausted is refused (`ids_exhausted`), so bit 30 and the sign bit are never set. The limits are ORG-03's: 60 s, one per pair, five per invitee, five sent per 30 s.
3. **The take is one lock hold** (`OrgInviteState::take` under the connected-map lock, then the lock is dropped before any database work), so a doubled CM 8 cannot accept twice. Unknown, expired and foreign ids are told apart in the log only; the player reads the same line for all three, so the answer is no oracle for live ids in other inboxes.
4. **Accept re-validation** under ORG-LOCK: the organization exists (`org_gone`), the inviter is still a member (`inviter_left`) whose rank holds `Invite` (`inviter_missing_permission`), and the responder is in no organization of that type (`already_in_org_type`, `already_member`). There is no member cap for Teams or Commands in the ledger, so no room check. The join rank comes from the locked organization row's type, not from the invite.
5. **Target resolution.** An invitee must be online, client-ready (the duel's rule: listed and no world-entry step pending, else `target_in_transition`) and not ignoring the inviter (`ignores_player(inviter)` or the inviter's session name; never a client-supplied name). A kicked or re-ranked member is resolved among that organization's member rows inside the locked transaction (exact name, then a unique case-insensitive match), so offline members can be kicked and re-ranked.
6. **D-ORG09 order for a rank change:** membership, target, self, rank in type, not `Leader`, `rank_unchanged` (before a direction bit is chosen), `Promote`/`Demote` by direction, then the actor strictly above both the current and the new rank. Every check runs under one lock, so the order only chooses the reported reason. The persistence layer pins `Leader` and the rank-in-type rule again.
7. **Post-commit targets are re-resolved by `player_id`** (`targets::online_now`) for the join push, the kicked player's [36] and `OrgMembershipEnded`, and the entity id in [39] / [40] (authority review): an entity id read before the transaction may have been recycled.
8. **`broadcast_to_org`** is in `handlers/broadcast.rs`, re-exported as `api::broadcast_to_org` (the ORG-API path). It reads the roster (and, with `required`, the rank table) after the commit, filters by `permissions.contains(required)`, and sends through ORG-06's `send_to_members`. `broadcast_except` (crate-private) skips one member, used for the join [37]. ORG-06's own fanouts are unchanged: their recipients come from rosters read before the commit (the leaver and the disbanded are not in the post-commit roster), so moving them onto a post-commit read would change who hears them.
9. **Creation race (ORG-05's carried gap), fixed.** `insert_member` now takes the joining character's creation advisory lock (`CREATE_LOCK_PLAYER`), after its membership check and before `FOR KEY SHARE`. An accept and a founding by the same character serialise, and the founding's pre-check sees the committed row before the sequence moves. Creation already holds that key (xact advisory locks are re-entrant). Lock-order argument, reviewed by `server-authority-enforcer`: creation never waits on an existing organization row, and taking the key only after the membership check keeps "a transaction holding an organization only waits on a non-member's key or row". The one exception, an account delete (`account_before_delete_lock_orgs`) holding a sibling character, is written into `api.rs` § "Lock order": detected as 40P01, reachable only from credential cleanup and tests.
10. **Ignore check for squads** happens on the base before `SquadInvite` is forwarded (`organization_squad.rs`), writing the `squad.invite` row (`reason = ignored`) and counting on `squad_actions_total`, since the cell never sees it. Everything else about a squad invitee stays the cell's check.
11. **Cell router:** CM 8 (base id), CM 9, 10 and 13-17 (base id) are forwarded as `ForwardCellCall`, CM 19 (base id) as `TransferCash`; CM 11 and 12 are refused as unsolicited on the cell for any id (one INFO row, counted, the refusal pair). Squad-range ids on methods squads lack keep ORG-01's answer (`route = rejected`). Every decision logs DEBUG `org.forward` with `route`.
12. **Base answers for calls no packet serves yet:** a forwarded CM 10 or 13-17 and `TransferCash` now get ORG-01's "not available yet" pair on the base (`answer::not_available`), for a live actor only; before, the base arm for `TransferCash` and unserved forwards were silent DEBUG no-ops, which the cell's own answer used to cover.
13. **GM commands** re-read the caller with `resolve_actor` (character and entity id) and the access level from that session. The same fix applied to ORG-06's `gm_disband` (`session_of`, which matched on the entity id alone; review finding, same class as the code in this packet). `.org_rank` takes an optional org id (a character can be in a Team and a Command); without it the member must be in exactly one (`org_ambiguous`). `.org_join` targets online players only, so the joiner gets the state push, and joins as `Leader` when the organization is memberless (D-ORG20 recovery).
14. **Refusal feedback** is one line on the feedback channel through `fanout::feedback`; `onErrorCode` is not sent for these (ORG-E1 Q4). Accept refusals word the one-per-type case from the responder's side.

## Advisor review

`server-authority-enforcer` (design, read-only): CONDITIONAL, five must-fix items, all applied: clear held invites on character switch (decision 1), atomic take without holding the lock across a database call (3), re-resolve post-commit targets by `player_id` (7), GM caller by character and entity (13), and the advisory lock after the membership check with the account-delete exception documented (9). Also applied: identical client line for unknown/expired/foreign (3), join rank from the locked row (4), request-id range bound (2), the `NO CYCLE` note on stored org ids (`invites/mod.rs`). Item 4 of the review ("refresh cached membership and permissions on kick and rank change") needs nothing today: no server-side cache of a member's rank or permissions exists (every authorization reads under the lock); ORG-09's officer chat and the Bank's vault session must keep reading under the lock or add a refresh on [36]/[40].

## Telemetry added

| Event | Level | SigNoz filter (`service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`) |
|---|---|---|
| `org.invite` (span and outcome row; `target_*`, `request_id`, `actor_rank`) | INFO | `event = 'org.invite' AND player_id = <id>` |
| `org.invite_response` (`after` = `joined` / `declined`; the inviter as target) | INFO | `event = 'org.invite_response' AND request_id = <id>` |
| `org.kick`, `org.rank_change` (`actor_rank`, `target_rank`, `to_rank`) | INFO | `event IN ('org.kick','org.rank_change') AND org_id = <id>` |
| `org.gm_join`, `org.gm_rank` (plus `org.gm_action` from `OrgAccess::system`) | INFO | `event IN ('org.gm_join','org.gm_rank','org.gm_action')` |
| `org.strike_team_response`, `org.pvp_leave_response` (`reason = unsolicited`, cell) | INFO | `reason = 'unsolicited'` |
| `invite_created`, `invite_consumed`, `invite_expired`, `invite_cleared` | DEBUG | `request_id = <id>` / `event = 'invite_cleared' AND player_id = <id>` |
| `member_joined` (`via`, `rank`), `member_left` (`reason = kicked`), `rank_changed` (`from_rank`, `to_rank`) | DEBUG | `event IN ('member_joined','member_left','rank_changed') AND org_id = <id>` |
| `org.forward` (`route` = `squad` / `base` / `rejected`) | DEBUG | `event = 'org.forward' AND method_index = <n>` |
| `org.broadcast` (`recipients`, `online_members`, `required`) | DEBUG | `event = 'org.broadcast' AND org_id = <id>` |
| `org.send_failed` (`what` = `organization_invite`, `organization_left_kicked`, `not_available`, the broadcast `what`), `org.broadcast_failed`, `org.action_failed` | WARN | `severity_text = 'WARN' AND event IN ('org.send_failed','org.broadcast_failed','org.action_failed')` |
| `squad.invite` with `reason = ignored` (base) | INFO | `scope_name = 'squad' AND reason = 'ignored'` |

Counter: `org_actions_total{action = invite | invite_response | kick | rank_change | gm_org_join | gm_org_rank | strike_team_response | pvp_leave_response, outcome, reason}`; the base's `org_type_invalid` refusal now counts under `action = invite`.

## Commands run

All from the worktree root through the lane (`target=B:\targets/org-07`). Every live-DB run reloads `sgw_org_07`; no live-DB test self-skipped (the tier sets `DATABASE_URL`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base-methods -p cimmeria-services --all-targets` | 0 | |
| `lane.sh cargo nextest run -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-wire --lib --no-fail-fast` (first run) | 100 | 8 failed: the ORG-01/03/06 routing tests that pinned the old "not available yet" answers; updated to the new routing |
| same, after the updates | 0 | 1806 passed |
| `live-db-test.sh organization::handlers organization::invites --no-fail-fast` | 0 | 52 passed |
| `DATABASE_URL=.../sgw_org_07 lane.sh cargo test -p cimmeria-wireclient --test it two_client_command_invite -- --test-threads=1` | 0 | 1 passed in 3.2 s, not skipped |
| `python mutations.py run` (the 22 proofs below) | 0 | each guard failed with exit 100; `git status` clean after |
| `git rebase origin/main` (onto `3ad5e571b`, #939, after #943 and #944; no conflicts) | 0 | |
| `lane.sh cargo fmt --all -- --check` | 0 | |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base-methods -p cimmeria-services -p cimmeria-wireclient -p cimmeria-cell --all-targets -- -D warnings` | 101, then 0 | `doc_lazy_continuation` on a test module header ("1)" at a line start); reworded |
| `lane.sh cargo nextest run` (the ten crates above) `--lib --no-fail-fast` | 0 | 2785 passed |
| `live-db-test.sh "::"` (the whole live-DB tier, after the rebase) | 0 | 5144 passed, 0 skipped |
| `reload-db.sh`, then `lane.sh cargo test -p cimmeria-wireclient --test it two_client -- --test-threads=1` | 101 | 8 passed (`two_client_command_invite`, `two_client_squad`, `two_client_tell`, both visibility suites); `two_client_mail_cod::cod_item_round_trip_between_two_clients` failed ("never received B's header without the item (method 76)"). **Pre-existing:** it fails identically on a clean `origin/main` @ `3ad5e571b` worktree (checked, then retired). Not caused by this packet |
| `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` | 0 | 55 passed (no new log target) |

## Regression proof

`python mutations.py run` (scratchpad script): each mutation applied alone to the committed tree, its guard run (`live-db-test.sh <tests> --no-fail-fast` or `lane.sh cargo nextest run -p <crate> --lib <tests>`), then `git checkout HEAD -- <files>` and `touch`. `git status` clean afterwards.

| Id | Mutation | Run | Exit | Failed |
|---|---|---|---|---|
| M1 | invite: the `Invite` bit check disabled | live | 100 | `org_invite_rejects_without_invite_perm` |
| M2 | invite: inviter authorized as a system actor (no membership read) | live | 100 | `org_invite_rejects_non_member_inviter` |
| M3 | base dispatch: 0xD0 types 1 and 2 routed to the squad forward | unit (`cimmeria-base`) | 100 | `team_and_command_invite_by_type_are_not_forwarded` |
| M4 | response: the entry looked up by request id alone, in any session | live | 100 | `invite_response_rejects_foreign_request_id`, `a_foreign_request_id_misses_and_leaves_the_invite` |
| M5 | `take` clones instead of removing | live | 100 | `invite_response_rejects_replay`, `an_invite_is_single_use` |
| M6 | accept: the inviter not re-read under the lock (system actor) | live | 100 | `org_accept_rejects_after_inviter_kicked` |
| M7 | kick: the rank comparison disabled | live | 100 | `org_kick_rejects_equal_or_higher_rank` |
| M8 | rank: `actor > to_rank` dropped | live | 100 | `rank_change_rejects_promote_above_self` |
| M9 | rank: the `Leader` refusal removed | live | 100 | `rank_change_rejects_assign_leader` |
| M10 | rank: the rank-in-type filter removed from the handler and from `set_rank` | live | 100 | `rank_change_rejects_rank_not_in_type` |
| M11 | kick: authorization read without the lock (plain SELECT, then a system access) | live | 100 | `kick_racing_a_held_lock_never_lets_the_kicked_member_act` |
| M12 | `insert_member`'s creation advisory lock removed | live | 100 | `an_accept_racing_a_creation_burns_no_org_id` |
| M13 | `logOff` no longer clears held invites | unit (`cimmeria-base`) | 100 | `logoff_drops_held_org_invites` |
| M14a | the base's squad Ignore check disabled | unit (`cimmeria-base`) | 100 | `squad_invite_to_a_player_who_ignores_the_inviter_is_refused` |
| M14b | the Team/Command Ignore check disabled | live | 100 | `invite_rejects_an_invitee_who_ignores_the_inviter` |
| M15 | kick: `OrgMembershipEnded` not sent | live | 100 | `org_kick_removes_and_fans_out` |
| M16 | cell: CM 11/12 forwarded to the base instead of refused | unit (`cimmeria-cell-methods`) | 100 | `strike_team_response_rejected_unsolicited`, `pvp_leave_response_rejected_unsolicited` |
| M17 | the [34] `org.send_failed` WARN downgraded to DEBUG | live | 100 | `an_unsendable_invite_warns_with_reason` |
| M18 | `.org_join`'s base access-level check removed | live | 100 | `gm_join_adds_at_entry_rank_and_keeps_the_type_rule` |
| M19 | `gm_disband`'s `session_of` matches the entity id alone (the ORG-06 code before the fix) | live | 100 | `gm_commands_from_a_recycled_entity_id_are_refused` |
| M20 | `broadcast_to_org` ignores `required` | live | 100 | `broadcast_reaches_online_members_filtered_by_permission` |
| M21 | cell: a base request id on CM 8 routed to the squad handler | unit (`cimmeria-cell-methods`) | 100 | `invite_response_routes_on_the_base_flag` |

Every mutation failed exactly its named guards and nothing else in its filter.

## Test catalogue

- `base-session` `organization::invites::tests` (unit): `request_ids_carry_the_base_flag_and_count_up`, `a_foreign_request_id_misses_and_leaves_the_invite`, `an_invite_is_single_use`, `an_invite_expires_at_sixty_seconds`, `one_pending_invite_per_pair_and_five_per_invitee`, `the_send_limit_slides`, `logoff_clears_held_invites_but_not_the_send_history`.
- `base-session` `organization::handlers::tests` (live-DB, types 3, 8, 12):
  - `invite.rs`: `invite_records_pending_and_sends_on_organization_invite`, `org_invite_rejects_non_member_inviter` (CAT-M-01), `org_invite_rejects_without_invite_perm` (CAT-M-01), `invite_by_type_never_creates_team_or_command` (CAT-M-02), `invite_by_type_finds_the_inviters_organization`, `invite_rejects_a_target_already_in_the_type`, `invite_rejects_an_invitee_who_ignores_the_inviter`, `invite_is_rate_limited_per_inviter`, `an_unsendable_invite_warns_with_reason`.
  - `invite_response.rs`: `org_accept_joins_at_entry_rank_and_fans_out`, `invite_response_rejects_foreign_request_id` (CAT-M-18), `invite_response_rejects_replay` (CAT-M-18), `org_accept_rejects_after_inviter_kicked` (CAT-M-18), `org_accept_rejects_after_inviter_loses_invite`, `org_accept_rejects_after_disband`, `org_accept_rejects_a_second_team`, `decline_tells_the_inviter`.
  - `kick.rs`: `org_kick_rejects_equal_or_higher_rank` (CAT-M-05), `org_kick_needs_eject_a_member_target_and_not_self`, `org_kick_removes_and_fans_out`, `org_kick_of_an_offline_member_uses_id_zero`.
  - `rank.rs`: `rank_change_rejects_promote_above_self`, `rank_change_rejects_assign_leader`, `rank_change_rejects_rank_not_in_type` (CAT-M-06), `rank_change_needs_the_bit_for_its_direction`, `rank_change_updates_and_fans_out`.
  - `broadcast.rs`: `broadcast_reaches_online_members_filtered_by_permission`, `broadcast_failures_warn_with_reason`.
  - `gm.rs`: `gm_join_adds_at_entry_rank_and_keeps_the_type_rule`, `gm_rank_skips_authority_but_not_the_rank_rules`, `gm_commands_from_a_recycled_entity_id_are_refused`.
  - `lock_race.rs` (type 5): `kick_racing_a_held_lock_never_lets_the_kicked_member_act` (ORG-LOCK), `an_accept_racing_a_creation_burns_no_org_id` (the ORG-05 gap). Both wait on `pg_stat_activity` lock waiters before releasing, so neither can pass without its race.
- `base` `dispatch::tests`: `organization.rs` (updated for the new routing: `squad_rank_change_is_answered_with_error_code_then_feedback`, `all_four_ids_reach_the_org_arm`, `kick_routes_on_the_squad_id_boundary`, `team_and_command_invite_by_type_are_not_forwarded`; new `squad_invite_to_a_player_who_ignores_the_inviter_is_refused`), `org_invite_logoff.rs`: `logoff_drops_held_org_invites`.
- `base-world-entry` `org_arms.rs`: `every_org_variant_reaches_the_org_arm` (updated), `unserved_calls_from_a_live_session_are_answered`, `gm_join_and_rank_without_a_gm_session_are_refused`, `forwarded_invite_response_from_a_stale_actor_is_dropped`.
- `cell-methods` `organization::tests`: `router.rs` `motd_is_decoded_and_routed` (was `motd_is_decoded_and_answered`), `invite_response_routes_on_the_base_flag` (updated), `strike_team_response_rejected_unsolicited` (CAT-M-16), `pvp_leave_response_rejected_unsolicited` (CAT-M-17), `transfer_cash_routes_to_the_base_as_transfer_cash`; `squad_ping_gm.rs` `ping_with_a_base_org_id_is_not_a_squad_ping` (updated).
- `cell-console` `tests/org07_join_rank.rs`: `org_join_forwards_to_the_base`, `org_rank_forwards_to_the_base`, `malformed_org_join_and_rank_stay_on_the_cell`, `org_join_and_rank_from_a_non_gm_are_not_forwarded`.
- `wireclient` `tests/it/two_client_command_invite.rs` (type 11): `two_clients_invite_into_a_command`.

The CAT-M names from the ledger exist as written, except `invite_by_type_rejects_type_above_command`, which is ORG-03's and unchanged.

## Known gaps

- **No real-client run.** Whether the client's `/commandinvite` and `/teaminvite` use 0xD0 with type 2 and 1 (as `/squadinvite` uses type 0, ORG-E1 Q2) or 0xCF with the org id is not confirmed; both are handled. What the client shows for [34] with type 1 or 2, and whether its Command window reacts to [39] `Kicked` and [40], is ORG-UAT's to watch.
- **Rank names in feedback lines** use the number ("rank 6"): custom names are ORG-08's, and the client's default names are client-side strings.
- **`/squadpromote`** still has no handler (0xD2 with a squad id answers "not available yet"; ORG-E1 follow-up 1).
- **Invite to an offline character** is refused (`target_not_found`): the invite needs a session to hold it and to show [34].
- **An inviter in gate transit** can still invite (their session is listed); an invitee in gate transit is refused `target_in_transition`.
- **Held invites** are not re-sent after the invitee's gate travel; the client keeps its own prompt, and the entry answers until it expires.
- **ORG-09 / Bank:** any future session- or cell-side cache of a member's rank or permissions must be refreshed on [36] and [40] (or read under the lock), or a kicked or demoted member keeps officer chat or vault access until relog.
- **A memberless organization** cannot be tested through `.org_join`'s Leader path without the Bank's vault (the SQL stub disbands instead of leaving the organization memberless), so that branch has no live-DB test.
- `crates/base-session/src/base/helpers/mod.rs` (898 lines) and `crates/entity/src/cell_entity/entity_struct.rs` remain over the hard cap (pre-existing; untouched here).

## Integration edits for the coordinator

- **Ledger (work-packets.md):** record `OrgCellToBase::{GmJoin, GmRank}` in § Messages; `broadcast_to_org` delivered (`api::broadcast_to_org`, signature `(ctx: &OrgCtx, org_id: i32, method_idx: u16, args: &[u8], required: Option<OrgPermission>) -> usize`, the number reached); pending invites held on the session (`ConnectedClientState::org_invites`), not a base-wide map; the creation race closed (ORG-05 carried gap); `.org_rank` takes an optional org id; the Ignore carried gap closed for squads and Teams/Commands; sentinel ranges above.
- **ORG-API note for the Bank (cimmeria-79):** `broadcast_to_org` is available; `OrgMembershipEnded` now also fires on a kick (`reason = Kicked`); the CM 19 cell route forwards `TransferCash` for Team and Command ids and the base arm answers "not available yet" with feedback (BV-08 replaces that arm in `base-world-entry` `org_dispatch.rs`); `add_member` now takes the creation advisory key, so the Bank's acting-character `sgw_player` update must stay a plain `UPDATE` (NO KEY UPDATE), not `SELECT … FOR UPDATE`, or it blocks `insert_member`'s `FOR KEY SHARE`. Per the coordinator's decision with the Bank (2026-09-27), `api.rs` § "Lock order" now says a path that waits on a member's `sgw_player` row while holding the organization (vault withdraw, cash `UPDATE`) takes those rows `FOR KEY SHARE` before `lock_org`, in `player_id` order; no ORG-07 path does that (accept and GM join only `FOR KEY SHARE` a joining non-member inside `add_member`; kick, rank change and GM rank touch member rows only).
- **ORG-08:** CM 13-17 already reach the base (`org_dispatch.rs::forward`, the `call =>` arm answering "not available yet"); replace that arm per method. `handlers::targets::member_by_name` resolves the officer-note target among the organization's members.
- **TESTING.md** wireclient row lists the integration tests with counts; `two_client_command_invite.rs` (1, live-DB only) should be added at the next count sweep (not edited here, per the rules).
- **Contended files touched:** `cell_methods/organization/` (`mod.rs`, `forward.rs`, tests) and `base/dispatch/organization.rs`, both ORG-07's per the ledger; also `base/dispatch/mod.rs` (the `db_pool` argument to the org arm), `base/dispatch/session.rs` (`logOff`), and ORG-06's `handlers/disband.rs` (the `session_of` fix).
