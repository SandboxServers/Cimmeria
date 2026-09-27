# ORG-06 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-02.md](org-02.md), [org-e1.md](org-e1.md).

## Contract

- **Packet:** ORG-06, login restore, leave, disband and presence for Teams and Commands.
- **Decisions in force:** D-ORG04 (ORG-LOCK), D-ORG05 (id routing), D-ORG12 (a Team or Command leader cannot leave while others remain; the last member disbands), D-ORG13 (the base re-reads privilege), D-ORG20 (every voluntary disband checks the vault under ORG-LOCK), plus the ORG-E1 Q1 login order.
- **Base:** `origin/main` @ `cdbd5ce88` (ORG-02 #881), rebased before push onto `origin/main` @ `b3c384408` (no conflicts). Branch `org/06-presence`, worktree `.claude/worktrees/org-06`, test DB `sgw_org_06`.
- **Owned paths:**
  - New: `crates/base-session/src/base/organization/handlers/` (`mod.rs`, `push.rs`, `presence.rs`, `leave.rs`, `disband.rs`, `fanout.rs`, `telemetry.rs`, `tests/{mod,push,presence,leave,disband}.rs`), `crates/base-session/src/base/session_presence.rs`, `crates/cell-console/src/cell/console/org.rs`, `crates/cell-console/src/cell/console/registry/commands/org.rs`, `crates/cell-console/src/cell/console/tests/org06_disband.rs`, `crates/base/src/base/dispatch/tests/org_logoff_presence.rs`.
  - Edited: `crates/base-session/src/base/helpers/mod.rs` (`destroy_client_entities`), `crates/base-session/src/base/tick_sync.rs`, `crates/base-session/src/base/organization/{mod.rs,api.rs}`, `crates/base-session/src/base/mod.rs`, `crates/base/src/base/login/mod.rs`, `crates/base/src/base/connect_loop/{mod.rs,encrypted/mod.rs}`, `crates/base/src/base/dispatch/session.rs` (`logOff`), `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs` (one call after `push_contact_lists_on_login`), `crates/base-world-entry/src/base/world_entry/cell_dispatch/org_dispatch.rs`, `crates/wire/src/cell/messages/{org_cell_to_base,org_base_to_cell}.rs`, `crates/cell/src/cell/service/base_messages/org.rs`, `crates/cell-methods/src/cell/cell_methods/organization/{mod.rs,forward.rs}`, `crates/cell-console/src/cell/console/{mod.rs,dispatch.rs}`, `crates/cell-console/src/cell/console/registry/commands/mod.rs`, and the test call sites of `destroy_client_entities` and `handle_login`.
  - Docs: `docs/gameplay/organization-system.md` (status rows, new "Login restore, presence, leave and disband" section), `docs/commands.md` (`.org_disband`), `docs/architecture/observability.md` (`org` row), this worknote.
- **Read set:** the ledger's ORG-06 packet, contract and telemetry sections (and the wave-2 update #936, which added the `OrgMembershipEnded` Bank hook); D-ORG04/05/12/13/18/20; the ORG-E1 and ORG-02 worknotes; `docs/reverse-engineering/findings/organization-restoration.md` Q1 and `organization-wire-formats.md`; `base/contact_list/` (push and presence fanout); `player_index` and the SS-00 `log_unlisted` teardown; `crafting/sync` (bundle push); `bank_dump` and `console/bank.rs` (cell-to-base GM command shape); `crafting/allcraft.rs` (base-side GM re-check).

## Evidence

- **The id-0 offline update is safe (settled from the docs, no Ghidra).** `organization-restoration.md` Q1: the `onMemberJoinedOrganization` handler at `0x00e4e4c0` looks the record up by name and, for an existing name, compares the stored id with the wire `aMember`; when they differ it unregisters the old entity-id lookup, registers the new one only `if aMember != 0`, and overwrites the id in place, regardless of `aNewMember`. So [37] with id 0 turns a row Offline and keeps it; [39] (`0x00e4f400`) deletes the row. `onOrganizationRosterInfo` (`0x00e4ea50`) seeds new rows with id 0 and updates only the rank of existing rows, which is why the roster goes before the [37]s.
- **A-35 was still open on `main`.** The contact-list offline fanout existed only in `logOff` (`dispatch/session.rs`); `grep fanout_login_status` found no call on any teardown path, and SS-00 had added only `log_unlisted`. ORG-06 adds it (no duplicate).
- **Every session-removing path except one goes through `destroy_client_entities`:** client DISCONNECT (`connect_loop/encrypted`), inactivity timeout and send error (`tick_sync`), duplicate login and account `logOff` (`login/mod.rs`). The exception is `gate_travel::abandon_unspaced_session` (see Known gaps).
- **`logOff(1)` keeps the session until the disconnect reaps it,** so without a guard the full exit would announce offline twice (once in `logOff`, once in the teardown). `listed_online` is the guard: `logOff` announces only a listed session and clears the flag; the teardown announces only a still-listed session.
- **`onClientReady` runs on every world entry, gate travel included** (`client_ready/mod.rs` comments; crafting and the Ignore resync rely on it), so the login push and the `member_online` fanout re-run after gate travel and refresh the entity id the other rosters hold.

## Design decisions

1. **One handler module, `organization/handlers/`,** started as a directory (six files). `OrgCtx { db_pool, transport, connected, entity_to_addr, cell_tx }` and `OrgPlayer { account_id, player_id, entity_id }` are the shared inputs. ORG-05's post-creation call is `push_org_state(ctx, org_id, player, new_member)` (messaged to org-05 while writing).
2. **The push is one reliable `ChannelBundle` per organization** on the player's own entity (safe to combine; fragments a large roster). [50] carries only renamed ranks, so an unnamed rank keeps the client's default label. The player is always counted Online in their own push, even before the session is listed.
3. **"Online" is a view over the connected map** (`listed_online`, `active_player_id`, `player_entity_id`), never a second index. The ORG-E1 recommendation to send [37] only for members already streamed to the client was not followed: the packet says every online member, and an entity id the client has not streamed only registers a reverse lookup.
4. **The offline hook plumbs `transport` and `db_pool` into `destroy_client_entities`** (and so into `handle_login` and `run_tick_loop`) instead of a process-global sink, so tests pass their own transport and pool. The fanout runs on its own task (`session_presence::spawn_offline`); without a pool or a runtime (the synchronous unit tests) it logs DEBUG and sends nothing. The account-side `logOff` passes no pool: no character is in the world there.
5. **`logOff` now announces through the same `spawn_offline`,** gated on `listed_online`. Side effect on the contact list: a `logOff` during world entry (before `onClientReady` listed the character) no longer sends an offline event, which is correct because no online event was ever sent.
6. **Leave runs under ORG-LOCK:** `member_access_locked`, then `load_roster` inside the same transaction, then the leader rule, then `persistence::disband` (which checks `api::org_vault_is_empty`) for the last member or `remove_member` for anyone else. The fanout and the audit export (`export_committed`, `MemberRemoval`) run after the commit.
7. **A refused leave re-sends the true state** (`push_org_state`) for `leader_cannot_leave` and `vault_not_empty`, then a feedback line; `not_member` gets the line only. `onErrorCode` is not sent (ORG-E1 Q4: no org text).
8. **The last member leaving** gets `onOrganizationLeft` with `Requested` (they asked) and a line saying the organization was disbanded; any other online member (none, by construction) would get `Disbanded`.
9. **`.org_disband` is forwarded to the base** as a new `OrgCellToBase::GmDisband { player_id, entity_id, org_id }`. The base re-reads `access_level` from its own session (`GM_ACCESS_LEVEL = 2`), locks through `OrgAccess::system` (which writes ORG-02's `org.gm_action` audit row), and calls `persistence::disband`, so the vault check holds for GMs.
10. **The Bank hook** (`OrgBaseToCell::OrgMembershipEnded`, added to the ledger in #936) carries `player_id` as well as the ledger's `{ entity_id, org_id, reason }`, per the contract rule that every variant carries the player's `player_id` and `entity_id`; the cell arm uses it to log whether the entity is still live (`live_entity`). **Contract deviation, one extra field:** record it in the ledger or tell me to drop it.
11. **Vault test seam:** `api::org_vault_is_empty` checks a `#[cfg(test)]` task-local `VAULT_EMPTY_OVERRIDE` first. Task-local, so parallel tests never see each other's value; compiled out of production. When the Bank replaces the stub it should keep or replace the seam (its own tests will use real vault rows). This also closes ORG-02's known gap "`disband`'s `VaultNotEmpty` refusal is untested".
12. **CM 9 forwarding on the cell** is one arm (`forward::leave_to_base`) for a base-routed id; the rest of CM 9-19 stays on `forward::answer` for ORG-07. An entity with no character gets the ORG-01 refusal pair (`route = rejected`, `reason = not_a_player`).
13. **Console refusals are not counted:** `cimmeria-cell-console` has no metrics dependency, so `.org_disband`'s cell-side refusals (`org_id_invalid`, `caller_not_player`, `cell_to_base_closed`) log the `org.disband` row but do not count on `org_actions_total`. Adding the dependency would need a hakari regeneration for three typo rows.

## Telemetry added (SigNoz Logs, `service.name = 'cimmeria-server' AND scope_name = 'org' AND ...`)

| Event | Level | Filter |
|---|---|---|
| `org.login_restore` (span and outcome row: `org_count`, `pushed`; `no_db`, `db_error`) | INFO | `event = 'org.login_restore' AND player_id = <id>` |
| `org.state_push` (`source`, `roster_size`, `online_members`, `messages`, `new_member`) / `org.state_push_failed` | INFO / WARN | `event LIKE 'org.state_push%' AND org_id = <id>` |
| `member_online` / `member_offline` (`member_id`, `recipients`, `online_members`, `disconnect_reason`) | INFO | `event IN ('member_online','member_offline') AND player_id = <id>` |
| `org.presence_failed`, `org.presence_skipped` | WARN | `event LIKE 'org.presence_%'` |
| `session.presence_skipped` (`no_db`, `no_runtime`) | DEBUG | `event = 'session.presence_skipped'` |
| `org.leave` (`after` = `left` / `disbanded`; `not_member`, `leader_cannot_leave`, `vault_not_empty`, `no_db`, `db_error`) | INFO | `event = 'org.leave' AND player_id = <id>` |
| `org.disband` (GM identity; `not_gm`, `no_such_org`, `vault_not_empty`, `no_db`, `db_error`; console: `org_id_invalid`, `caller_not_player`, `cell_to_base_closed`) | INFO | `event IN ('org.disband','org.gm_action') AND org_id = <id>` |
| `member_left` (`reason = requested`, `from_rank`), `disbanded` (`reason` = `last_member_left` / `gm`, `members`, `recipients`) | DEBUG | `event IN ('member_left','disbanded') AND org_id = <id>` |
| `org.send_failed` (`what`, `reason`, `target_player_id`), `org.leave_failed`, `org.disband_failed` | WARN | `severity_text = 'WARN' AND event IN ('org.send_failed','org.leave_failed','org.disband_failed')` |
| `org.actor_mismatch` | WARN | `event = 'org.actor_mismatch'` |
| `org.forward` (`route`), `org.forward_failed` (cell) | DEBUG / WARN | `event LIKE 'org.forward%' AND method_index = 9` |
| `org.membership_ended` (cell), `org.membership_ended_skipped` (base) | DEBUG | `event LIKE 'org.membership_ended%'` |

Counter: `org_actions_total{action = login_restore | leave | disband, outcome, reason}`.

## Commands run

All from the worktree root through the lane (`target=B:\targets/org-06`). Every live-DB run reloads `sgw_org_06`; no live-DB test was skipped (the tier sets `DATABASE_URL`).

| Command | Exit | Result |
|---|---|---|
| `lane.sh cargo check -p cimmeria-wire -p cimmeria-cell -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry -p cimmeria-cell-methods -p cimmeria-cell-console --all-targets` | 0 | |
| `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry --no-fail-fast` (after rebase) | 0 | 2238 passed, 0 skipped |
| `live-db-test.sh organization org_logoff_presence org_arms contact_list base::login player_index membership_ended --no-fail-fast` (after rebase) | 0 | 253 passed |
| `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base-session -p cimmeria-base -p cimmeria-base-world-entry --all-targets -- -D warnings` | 0 | |
| `lane.sh cargo fmt --all -- --check` | 0 | |

## Regression proof

Each mutation applied alone on the committed tree, the named tests run with `live-db-test.sh <filters> --no-fail-fast` (the cell router one with `lane.sh cargo nextest run -p cimmeria-cell-methods organization::tests::router`), then `git checkout HEAD -- crates` and the restored files touched. Every run exited 100:

| Mutation | Failed |
|---|---|
| M1: the roster [38] moved after the [37]s in `org_state_messages` | `org_state_messages_follow_the_org_e1_order`, `login_push_for_a_two_org_player_is_byte_exact` |
| M2: `spawn_offline` never called from `destroy_client_entities` | `offline_fanout_on_every_disconnect_path`, `contact_list_watchers_hear_a_teardown` |
| M3: the `listed_online` gate dropped (teardown and `logOff`) | `unlisted_session_is_not_announced_twice`, `full_exit_logoff_announces_offline_once` |
| M4: the offline [37] carries the live entity id | `offline_fanout_on_every_disconnect_path` |
| M5: no D-ORG12 leader rule | `leader_leave_is_rejected_while_members_remain` |
| M6: leave authorized as a system actor (no membership read) | `org_leave_rejects_non_member` |
| M7: the vault check removed from `persistence::disband` | `last_member_leave_refused_while_vault_not_empty`, `gm_disband_refused_while_vault_not_empty` |
| M8: the base GM re-check removed | `gm_disband_rejects_non_gm`, `gm_disband_without_a_gm_session_is_refused` |
| M9: the forwarded actor not re-checked against the session | `forwarded_leave_from_a_stale_actor_is_dropped` |
| M10: `org.send_failed` downgraded to DEBUG | `presence_send_failure_warns_with_reason` |
| M11: the contact-list fanout dropped from the teardown | `contact_list_watchers_hear_a_teardown` |
| M12: the cell answers a Team or Command CM 9 instead of forwarding it | `leave_routes_on_the_squad_id_boundary` |
| M13: `OrgMembershipEnded` not sent on leave and disband | `member_leave_removes_and_fans_out`, `gm_disband_cascades_and_tells_online_members` |

## Test catalogue

- `base-session`, `organization/handlers/tests/` (live-DB, sentinels `0x7000_4C00..=0x7000_4DFF`, blocks 0-12 used; organizations named "Org06 ..." and cleaned by exact name key):
  - `push.rs`: `org_state_messages_follow_the_org_e1_order` (pure, literal bytes), `login_push_for_a_two_org_player_is_byte_exact`.
  - `presence.rs`: `offline_fanout_on_every_disconnect_path` (all five `destroy_client_entities` reasons), `unlisted_session_is_not_announced_twice`, `presence_send_failure_warns_with_reason` (LogCapture), `contact_list_watchers_hear_a_teardown` (A-35).
  - `leave.rs`: `org_leave_rejects_non_member` (CAT-M-04 name), `leader_leave_is_rejected_while_members_remain`, `member_leave_removes_and_fans_out`, `last_member_leave_disbands`, `last_member_leave_refused_while_vault_not_empty`.
  - `disband.rs`: `gm_disband_cascades_and_tells_online_members`, `gm_disband_refused_while_vault_not_empty`, `gm_disband_rejects_non_gm`.
- `base`, `dispatch/tests/org_logoff_presence.rs` (live-DB, sentinels `0x7000_4DF0..=0x7000_4DF2`): `full_exit_logoff_announces_offline_once`.
- `base-world-entry`, `org_arms.rs`: `forwarded_leave_from_a_stale_actor_is_dropped`, `gm_disband_without_a_gm_session_is_refused`.
- `cell`, `base_messages/tests/org.rs`: `membership_ended_reaches_the_org_arm`.
- `cell-methods`, `organization/tests/router.rs`: `leave_routes_on_the_squad_id_boundary` (updated: the Team side now forwards).
- `cell-console`, `tests/org06_disband.rs`: `org_disband_forwards_to_the_base`, `org_disband_with_a_bad_id_stays_on_the_cell`, `org_disband_from_a_non_gm_is_not_forwarded`.

## Known gaps

- **`gate_travel::abandon_unspaced_session`** removes a session without `destroy_client_entities`, so neither the org nor the contact-list offline fanout runs on that (already catastrophic) path; the other members see the character Online until its next login or logout. Its callers have `db_pool` and `transport`; a one-line `spawn_offline` there would close it, but the file is not in this packet's owned list.
- **A roster larger than a few thousand bytes** is fragmented by the bundle; not exercised by a test.
- **`crates/base-session/src/base/helpers/mod.rs` is over the 700-line hard cap** (866 on `main`, 898 now). Pre-existing; the ORG-06 logic itself lives in `session_presence.rs`. Worth its own split.
- **A leave racing a login restore** can send a roster that includes the leaver; the next world entry corrects it. Display reads take no lock by design.
- **The `.org_disband` cell-side refusals** are not counted on `org_actions_total` (decision 13).
- No wireclient (type 11) two-client test: this packet adds no new client flow a real client drives that the live-DB tests do not already byte-check; ORG-07's wave test covers two clients.

## Integration edits for the coordinator

- **Ledger:** mark `OrgMembershipEnded` as carrying `player_id` too (decision 10), and record `OrgCellToBase::GmDisband { player_id, entity_id, org_id }` in the message contract. `OrgCtx` now has a `cell_tx` field; ORG-05 and ORG-07 build it from `DispatchCtx`.
- **ORG-05:** call `organization::handlers::push_org_state(&ctx, org_id, &player, true)` after a committed creation.
- **ORG-07:** reuse `handlers::fanout::{online_members, send_to_members, membership_ended}` for kick and invite fanout; `destroy_client_entities` now takes `transport` and `db_pool` (clear base pending invites in the same hook). The CM 9 base route is already forwarded; generalise `forward::leave_to_base` for CM 10-17.
- **Bank (cimmeria-79):** `org_vault_is_empty` has a test-only override (decision 11); BV-07 extends the cell arm in `crates/cell/src/cell/service/base_messages/org.rs`.
- **Contended files touched:** `client_ready/mod.rs` (one call) and `helpers/mod.rs` (`destroy_client_entities`), both ORG-06's per the ledger. Also touched outside the list: `crates/base/src/base/login/mod.rs`, `connect_loop/`, `tick_sync.rs` (the `db_pool` plumbing) and `dispatch/session.rs` (`logOff`).
