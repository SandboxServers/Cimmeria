# ORG-09 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-04.md](org-04.md), [org-07.md](org-07.md).

## Contract

- **Packet:** ORG-09, team (3), command (5) and officer (6) chat to the online members of the sender's organization of that type; officer requires `OfficerChat`.
- **Decisions in force:** D-ORG14 (channel ids follow `enumerations.xml`; officer needs no registration), D-ORG26 (the channel-id alignment, `world_entry_chat.rs` and the `CHAN_*` values belong to SS-C4; not touched here), D-ORG08 / D-ORG21 (default rank masks: `OfficerChat` on Command `Officer` and `SeniorOfficer`, Team `SeniorMember`, and `Leader`), SS-00 (`RateCategory::Chat`), SS-C3 (#925, allowlist and `.mute` on every channel), SS-C4 (#937).
- **Base:** `origin/main` @ `2b30ec1ba` (ORG-07 #945) at the start, rebased onto `6607fbdc2` (BV-05 #947) before the push. Branch `org/09-org-chat`, worktree `.claude/worktrees/org-09`, test DB `sgw_org_09`.
- **Sentinels:** `0x7000_5600..=0x7000_56FF` (`Fixture::org09`, 16 blocks of 16, blocks 0-5 used, names `Org09P<n>`, organizations "Org09 ..."). Mute-table player id `0x7300_0318` in `chat_channel_mute.rs`.

## Owned paths

- Base session: `crates/base-session/src/base/organization/handlers/chat.rs` (new), `handlers/mod.rs` (module, re-exports), `handlers/tests/chat.rs` (new), `handlers/tests/mod.rs` (`Fixture::org09`, sentinel docs).
- Base: `crates/base/src/base/dispatch/chat.rs` (`ChatRoutes`, the org branch, the pre-relay `org.chat` refusal rows), `dispatch/mod.rs` (passes `entity_to_addr` and `db_pool`), `dispatch/chat_gates.rs` (module doc only), `dispatch/tests/chat_org_refusals.rs` (new), `tests/chat_channel_mute.rs` (forward test updated, mute-order test added), `tests/chat_flood_limit.rs` and `tests/tell.rs` (harness call sites), `tests/mod.rs`.
- Cell: `crates/cell-console/src/cell/console/chat/mod.rs` (comment in the fallback arm only).
- Docs: `docs/gameplay/organization-system.md` (status, "Team, command and officer chat (ORG-09)"), `docs/gameplay/chat-system.md` (organization-channel routing only), `docs/architecture/observability.md` (`org` row).

## Read set

ORG-09 packet, "Common acceptance" and telemetry sections of `work-packets.md`; D-ORG08/14/21/26; audit A-12, A-36; ORG-E1 Q5 in `docs/reverse-engineering/findings/organization-restoration.md`; worknotes org-04, org-07; `crates/base/src/base/dispatch/{chat.rs,chat_gates.rs,tell.rs,mod.rs}`; `crates/cell-console/src/cell/console/chat/{mod.rs,squad.rs}`; `base/organization/handlers/{broadcast.rs,fanout.rs,telemetry.rs,mod.rs}`, `persistence/loads.rs`; `crates/entity/src/organization/permissions.rs`; `crates/wire/src/cell/chat.rs` (the literal-channel guard); `deprecated/python/base/Chat.py`, `Atrea/enums.py`.

## Evidence and decisions

1. **Officer (6) is a Command channel.** `OfficerChat` is in the Command rank editor only (`Command.lua` `commandPermissions`, audit A-12; `OrgPermission::COMMAND_EDITABLE` adds it to Team's twelve), the officer ranks (6, 7) exist only in a Command, and the legacy enum has `MAIL_ToCommandOfficers = 32` with no Team twin (`deprecated/python/Atrea/enums.py:739`). The legacy `Chat.py:147-150` only declares the channels as `CHANNEL_FLAG_OnCell` and routes nothing. A Team's `SeniorMember` default mask carries the bit (it copies `Officer`'s), but a Team has no officer channel. Approved by the coordinator on 2026-09-27.
2. **Routed on the base, after every existing gate.** The org branch sits in `send_player_communication_at` after the bucket, `refuse_channel`, `refuse_if_muted` and `validate(TextField::ChatText)`, beside the tell branch. Lines on 3, 5 and 6 are no longer forwarded to the cell. The cell's fallback arm stays as the answer for any id that still arrives there.
3. **Membership read.** `load_memberships` (the display read, no org lock). A chat line changes no state, and locking the org row per line would serialize chat with every mutation; the stale window is one line during a committing demotion, the same window `broadcast_to_org`'s own recipient filter has. Recorded in the module docs and the gameplay doc.
4. **Fanout.** `broadcast_except(ctx, org_id, 28, args, required, Some(speaker), "chat")`, then the speaker's own copy via `send_to_player`, so `recipients` counts members reached without the speaker's copy (as ORG-04 does) and a lost echo gets its own WARN. `required = Some(OFFICER_CHAT)` on 6.
5. **Pre-relay refusals** (`rate_limited`, `text_invalid`) also write an `org.chat` row on 3/5/6, mirroring ORG-04's `squad.chat` rows, via `org_chat::log_refused_before_relay`. A mute refusal writes no `org.chat` row (as with squad): SS-C3's `chat.muted_refused` is the row.
6. **Reasons kept local** to `handlers/chat.rs` (strings on the row, counted through `telemetry::count`), so `OrgReject` in the shared `telemetry.rs` is untouched and ORG-08/ORG-10 do not conflict on it.
7. **`ChatRoutes`** bundles `cell_tx`, `entity_to_addr` and `db_pool` for `send_player_communication_at`, keeping it at seven parameters.

## Telemetry added

| Event | Level | SigNoz filter (`scope_name = 'org'`) |
|---|---|---|
| `org.chat` span | INFO | span name `org.chat` |
| `org.chat` outcome (`ok` / `rejected`; `reason` = `not_in_org` \| `missing_permission` \| `no_db` \| `db_error` \| `not_in_world` \| `rate_limited` \| `text_invalid`; `channel`, `org_id`, `org_type`, `recipients`, `text_units`, `account_id`, `player_id`, `entity_id`) | INFO | `event = 'org.chat' AND player_id = <id>` |
| `org.send_failed` `what = chat_echo` (the speaker's copy) | WARN | `event = 'org.send_failed' AND what = 'chat_echo'` |
| `org.send_failed` `what = chat` (a member), `org.broadcast` `what = chat` | WARN / DEBUG | `what = 'chat'` (existing ORG-07 seams) |
| `org.chat_lookup_failed` (`reason` a persistence reason) | WARN | `event = 'org.chat_lookup_failed'` |
| `org_actions_total{action="chat", outcome, reason}` | counter | metric |

## Commands run

All from the worktree root.

| Command | Exit | Result |
|---|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-base -p cimmeria-base-session --all-targets` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-base --lib chat` | 0 | 53 passed |
| `bash tools/build-lane/live-db-test.sh handlers::tests::chat` | 0 | 5 passed, 0 skipped |
| `bash tools/build-lane/lane.sh cargo fmt --all -- --check` (after rebase) | 0 | clean |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-base -p cimmeria-base-session -p cimmeria-cell-console --all-targets -- -D warnings` | 0 | clean |
| `bash tools/build-lane/lane.sh cargo nextest run --profile=ci -p <crate> --lib` for base / base-session / cell-console / wire | 0 | 153 / 580 / 381 / 261 passed, 0 skipped |
| `bash tools/build-lane/live-db-test.sh ::` (after rebase) | 0 | 5233 passed, 0 skipped |

## Regression proof

Each revert was applied to the committed tree, run, then restored with `git checkout HEAD -- <file>` and `touch`.

| Revert | Failing tests |
|---|---|
| Org branch in `chat.rs` disabled (3/5/6 go back to the cell) | `chat_channel_mute::chat_forwards_allowlisted_channels_to_the_cell`, `chat_org_refusals::team_flood_logs_rate_limited_once_per_notice` |
| Org branch moved ahead of `refuse_if_muted` | `chat_channel_mute::muted_player_org_line_refused_before_org_chat`, `chat_org_refusals::org_line_with_a_forbidden_character_logs_text_invalid` |
| `log_refused_before_relay` call disabled | `chat_org_refusals::org_line_with_a_forbidden_character_logs_text_invalid`, `team_flood_logs_rate_limited_once_per_notice` |
| Officer recipient filter `required` -> `None` | `handlers::tests::chat::officer_chat_reaches_only_officer_chat_ranks` |
| Sender `OfficerChat` check disabled | `officer_chat_without_officer_chat_is_refused_with_feedback` |
| Speaker echo not sent | `officer_chat_reaches_only_officer_chat_ranks`, `org_chat_seams_warn_with_reason`, `team_and_command_chat_reach_every_online_member` |
| Org-type match replaced by the first membership | `officer_chat_reaches_only_officer_chat_ranks`, `speaker_in_no_org_of_that_type_gets_feedback`, `team_and_command_chat_reach_every_online_member` |

## Test catalogue

- `base-session` `handlers::tests::chat` (live-DB, types 3, 8 and 12): `team_and_command_chat_reach_every_online_member` (byte-exact [28] per member, offline and non-member skipped, a member moved to another world still hears it, no cell forward, row fields, never the text); `officer_chat_reaches_only_officer_chat_ranks` (Leader and Officer only, Initiate and a Team member skipped, no `onChatJoined` to anyone); `officer_chat_without_officer_chat_is_refused_with_feedback`; `speaker_in_no_org_of_that_type_gets_feedback` (team, and command/officer lines); `org_chat_seams_warn_with_reason` (echo WARN, `no_db`).
- `base` `dispatch::tests` (no DB): `chat_channel_mute::chat_forwards_allowlisted_channels_to_the_cell` (updated), `chat_channel_mute::muted_player_org_line_refused_before_org_chat`, `chat_org_refusals::{org_line_with_a_forbidden_character_logs_text_invalid, squad_line_refusal_logs_no_org_row, team_flood_logs_rate_limited_once_per_notice}`.
- Existing guards still covering the packet: `cimmeria_wire::cell::chat::tests::no_player_communication_call_uses_a_literal_channel` (the new call passes the `channel` variable), SS-C4's `on_client_ready_burst_registers_no_built_in_channel`.

## Known gaps

- No two-client wireclient test for org chat; the live-DB fanout test drives real sessions through `TestTransport`. ORG-UAT should check a team, command and officer line on two clients, and whether the client echoes its own line locally (a second copy, as in the squad carried gap).
- "Members in different spaces or cells": the base keeps no space per session, so the fanout is space-agnostic by construction; the test moves a member's `world_location` and asserts the line still arrives, which is the only space state the base holds.
- `db_error` on the membership read has no forced-failure test (the `no_db` path is tested; the `db_error` arm shares its feedback line and row).
- A mute refusal on 3/5/6 writes no `org.chat` row (consistent with squad chat); the refusal is `chat.muted_refused` on `chat`.

## Integration edits for the coordinator

- Ledger: ORG-09 status, and record the officer-is-Command decision (Evidence 1) as a D-ORG row if it should be ratified.
- `docs/gap-analysis.md` and `docs/project-status.md` untouched per the rules (ORG-11).
- Sentinel `0x7000_5600..=0x7000_56FF` claimed (told org-08 and the coordinator).
