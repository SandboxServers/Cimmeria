# Organizations Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](README.md), [audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [ability-tree ledger](../ability-trees/work-packets.md) (same dispatch rules).

## Dispatch rules

- One worktree per worker: `bash tools/build-lane/mk-worktree.sh org/<packet>-<slug> org-<packet>`. It junctions `external/` in.
- Every cargo call goes through the lane: `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Never use `--exclusive` or `--workspace` while other campaigns are working. Live-DB tests: `bash tools/build-lane/live-db-test.sh <filter>`, which uses the worktree's own `sgw_<worktree>` database.
- Rust is pinned to 1.98.1 by `rust-toolchain.toml`. After any dependency change, run `cargo hakari generate && cargo hakari manage-deps --yes` and `python tools/crate-graph/crate_graph.py --check`.
- Remove a worktree's `external` junction non-recursively (`cmd /c rmdir <wt>\external`) before removing the worktree.
- Squash-merge on green CI **and** a clean Copilot review (owner rule, 2026-09-27). The coordinator requests it with `MSYS_NO_PATHCONV=1 gh api -X POST repos/SandboxServers/Cimmeria/pulls/<N>/requested_reviewers -f 'reviewers[]=copilot-pull-request-reviewer[bot]'`, fixes or answers every comment, and re-requests after a substantive fix. When a PR's CI predates the current `main`, rebase and re-test first.
- **Amendment (owner, relayed 2026-09-27):** Copilot reviews are suspended because the review spend is exhausted. Until the owner lifts that, squash-merge on green CI plus the coordinator's own review of the diff (D-ORG25). Everything else in the rule above stands.
- Initial state: documentation only, against `main` @ `70795027`. ORG-E1 and ORG-01 started on 2026-09-27 from `95366c59`.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

## Contract fixed by this ledger

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming locally. ORG-01 creates everything in this section except the schema (ORG-02).

**Models** (`crates/entity/src/organization/`, re-exported from `cimmeria_entity::organization`):

- `OrgType { Squad = 0, Team = 1, Command = 2 }` with `is_persistent()`.
- `OrgRank(u8)` newtype over the nine `EOrganizationRank` values (0-8), with `for_type(OrgType) -> &'static [OrgRank]` returning the D-ORG07 set.
- `OrgPermission`: 26-bit flags from `enumerations.xml:1907` (`ALL = 0x3FF_FFFF`), plus `editable_for(OrgType)`: 14 bits for Command, 12 for Team (audit A-12).
- `OrgLeaveReason { Requested = 0, Kicked = 1, Disbanded = 2, Logout = 3 }` (`EReasons`, `enumerations.xml:104`).
- `SquadLootType { RoundRobin = 0, FreeForAll = 1 }`.
- `SQUAD_ORG_ID_MIN = 0x4000_0000` (squad org ids, D-ORG05), `BASE_INVITE_REQUEST_FLAG = 1 << 29` (base-issued request ids, D-ORG06), `MAX_SQUAD_SIZE = 6`, and the D-ORG10 caps. The doc comment on each of the two id constants names the other, because they are different bits with different owners.
- `org_text::validate(field, &str) -> Result<String, TextReject>`: the one implementation of D-ORG10, including the name normaliser that produces `name_key`.
- `default_rank_permissions(OrgType) -> [(OrgRank, OrgPermission)]` (D-ORG08 as amended by D-ORG21). Both the Rust creation path and any seed read this one table.

Every enum value is pinned by a literal test against `enumerations.xml`, never against itself.

**Wire** (`crates/wire/src/cell/client_methods/organization.rs` and `…/player.rs`): one `build_*` per client method 34-51, 134 and 135, each with a byte-exact test. The four base methods (0xCF-0xD2) get typed decoders in `crates/wire`, and the cell-method decoders for CM 13, 14, 15, 17 and 94 read their trailing `WSTRING`s.

**Messages.** Org traffic between the cell and the base goes through **one nested enum per direction**: `CellToBaseMsg::Org(OrgCellToBase)` and `BaseToCellMsg::Org(OrgBaseToCell)`. ORG-01 adds the two outer variants and each nested enum with the variants listed in each packet below, so later packets add nested variants (in their own file) instead of editing `cell_to_base.rs` and `base_to_cell.rs`. Every variant carries the acting player's `player_id` and `entity_id` **from the cell's own session state**, never from the client payload, and never carries a privilege bit.

**Schema** (ORG-02, `db/sgw/Organizations/`):

| Table | Columns |
|---|---|
| `sgw_organizations` | `org_id integer CHECK (org_id BETWEEN 1 AND 1073741823)` (sequence), `org_type smallint CHECK (org_type IN (1, 2))`, `name varchar(60)`, `name_key varchar(60)` (D-ORG10), `motd varchar(255)`, `cash bigint NOT NULL DEFAULT 0 CHECK (cash >= 0)`, `experience bigint NOT NULL DEFAULT 0`, `created_at timestamptz`. `UNIQUE (org_type, name_key)`, and `UNIQUE (org_id, org_type)` as the target of the member FK. |
| `sgw_organization_ranks` | `(org_id, rank smallint)` primary key, `name varchar(32)`, `permissions integer CHECK (permissions BETWEEN 0 AND 67108863)`. One row per rank the type uses. |
| `sgw_organization_members` | `(org_id, player_id)` primary key, `rank smallint`, `note varchar(128)`, `officer_note varchar(128)`, `joined_at timestamptz`. FK `(org_id, rank)` → ranks; FK `player_id` → `sgw_player ON DELETE CASCADE`. `UNIQUE (player_id, org_type)` through a denormalised `org_type` column, which enforces D-ORG18 in the database; a composite FK `(org_id, org_type)` → `sgw_organizations (org_id, org_type)` stops it drifting. An `AFTER DELETE` trigger implements D-ORG12's leader-deletion rule. |

The leader is the member whose rank is 8; there is no `leader_player_id` column to keep in step.

ORG-02 (#881) also ships two `BEFORE DELETE` triggers that make every delete path take the ORG-LOCK order before the cascade reaches the member rows (`db/sgw/_functions.sql`, `db/sgw/_triggers.sql`):

- `sgw_player_before_delete_lock_orgs` (function `org_player_before_delete`) locks the deleted character's organization rows, in `org_id` order.
- `account_before_delete_lock_orgs` on `account` (function `org_account_before_delete`) locks all of the account's `sgw_player` rows in `player_id` order, then all of their organizations in `org_id` order. Without it, an account delete cascading over two characters would lock organizations character by character and could deadlock against a single-character delete.

Organization live-DB tests use the sentinel range `0x7000_4800..=0x7000_4BFF` (`crates/base-session/src/base/organization/persistence/tests/mod.rs`).

### Bank campaign API (ORG-API)

The Bank / Vault campaign (cimmeria-79) builds the Team and Command vaults and the treasury on top of this. ORG-02 and ORG-07 deliver it, in `crates/base-session/src/base/organization/api.rs`:

- `lock_org(tx: &mut Transaction, org_id) -> Result<Option<OrgHeader>>`: `SELECT … FOR UPDATE` on the org row. **It is always the first lock a transaction takes** (ORG-LOCK, D-ORG04); the bank then locks `sgw_player`, then item rows.
- `member_access_locked(tx, org_id, player_id) -> Result<Option<OrgAccess>>`, where `OrgAccess { org_id, org_type, rank, permissions }`, read inside that same transaction. `None` means not a member, so the caller rejects. There is deliberately **no** pool-level variant: never authorize from a read outside the lock.
- `broadcast_to_org(ctx, org_id, method_idx, args, required: Option<OrgPermission>)`: sends a client method to every online member, optionally only those whose rank holds a permission (for example `ViewBankLogs`). Call it after the commit.
- `build_on_organization_cash_update` in `crates/wire`, and the `cash` value in the login header (ORG-06).
- The CM 19 route: the cell forwards `OrgCellToBase::TransferCash { player_id, entity_id, org_id, amount }`, with `player_id` and `entity_id` taken from the session. Until the bank lands, the base arm rejects it with feedback (`onErrorCode`) and logs `org.transfer_cash_unimplemented`. The bank replaces that one arm and owns its acceptance list: `amount != 0` on the signed `i32`, with the sign choosing deposit or withdraw; `DepositCash` or `WithdrawCash` by direction (distinct bits); no overflow of the wallet or of `cash`; the D-ORG19 cap; and one transaction under ORG-LOCK.

**Delivered so far.** ORG-02 (#881) shipped `lock_org`, `member_access_locked` (the `OrgAccess` it returns is tied to its transaction; using it in another one fails with `OrgStoreError::StaleAccess` or `ActorMismatch`), `org_vault_is_empty` (stub, returns true), the SQL stub `org_vault_is_empty_sql`, and the `cash` column. `broadcast_to_org` and the CM 9-19 non-squad forward come with ORG-07. The CM 19 base arm stays the reject stub (`org.transfer_cash_unimplemented`), and replacing it is the bank's **BV-08**.

**Known gap for the bank.** Neither ORG-06 nor ORG-07 closes an org vault window that is already open when its holder leaves, is kicked, or the organization disbands. BV-07 closes it on `onOrganizationLeft` [36].

## CAT-M coverage

Every in-scope finding from [CAT-M](../../security-audit/2026-05-31-server-authority/findings/CAT-M-org-squad-duel.md) has an owner, an invariant and a named regression test. Workers keep these test names (or record the rename in their worknote).

| Finding | Packet | Invariant | Test |
|---|---|---|---|
| M-01 invite without membership | ORG-07 | The inviter is a member holding `Invite`, checked under ORG-LOCK | `org_invite_rejects_non_member_inviter`, `org_invite_rejects_without_invite_perm` |
| M-02 invite by type | ORG-03, ORG-07 | Type 0-2 only; types 1 and 2 never create implicitly | `invite_by_type_rejects_type_above_command`, `invite_by_type_never_creates_team_or_command` |
| M-03 creation | ORG-05 | Pending creation required; D-ORG10 name rules; cost in the base transaction | `create_rejects_without_pending_creation`, `create_rejects_invalid_names`, `create_duplicate_name_costs_nothing` |
| M-04 leave with a foreign id | ORG-03, ORG-06 | The caller is a member of that org | `squad_leave_rejects_foreign_squad_id`, `org_leave_rejects_non_member` |
| M-05 kick | ORG-07 | D-ORG09 (1)-(2) | `org_kick_rejects_equal_or_higher_rank` |
| M-06 rank change | ORG-07 | D-ORG09 (1), (2), (4), (5) | `rank_change_rejects_promote_above_self`, `rank_change_rejects_assign_leader`, `rank_change_rejects_rank_not_in_type` |
| M-07 permissions | ORG-08 | D-ORG09 (3), (6) | `set_perms_rejects_grant_of_unheld_bit`, `set_perms_rejects_own_rank`, `set_perms_rejects_leader_row` |
| M-08 rank name | ORG-08 | `RankNames`, D-ORG09 (3), D-ORG10 | `set_rank_name_rejects_without_perm`, `set_rank_name_rejects_over_cap` |
| M-10 MOTD and notes | ORG-08 | Bits per method; officer note resolves among that org's members only, with a rank check; D-ORG10 | `motd_rejects_without_perm`, `officer_note_rejects_target_outside_org`, `officer_note_rejects_higher_rank_target`, `text_rejects_bidi_and_zero_width` |
| M-11 loot mode | ORG-03 | D-ORG16 leader only; value 0 or 1 | `loot_mode_rejects_non_leader`, `loot_mode_rejects_out_of_range` |
| M-16, M-17 strike team and PvP leave | ORG-07 | No request is ever issued, so every response is unsolicited | `strike_team_response_rejected_unsolicited`, `pvp_leave_response_rejected_unsolicited` |
| M-18 invite response | ORG-03, ORG-07 | D-ORG06 composite key, single use, re-validated on accept | `invite_response_rejects_foreign_request_id`, `invite_response_rejects_replay`, `squad_accept_rejects_when_full`, `org_accept_rejects_after_inviter_kicked` |
| M-09 cash | Bank campaign | ORG-API acceptance list | (Bank campaign) |

## Dependency graph and waves

```text
Wave 0            Wave 1                 Wave 2                           Wave 3                 Wave 4                         Wave 5
ORG-E1 evidence ─────────────────────────────────────────────────────────────────┐
ORG-01 contract ┬─► ORG-02 schema+API ─┬─► ORG-05 creation + registrar ─┐                                                   
                │                      └─► ORG-06 login, leave, presence┴─► ORG-07 invite/kick/rank ┬─► ORG-08 MOTD/notes/editor ─┐
                └─► ORG-03 squad core ───► ORG-04 squad chat + ping ─────────────────────────────────├─► ORG-09 org chat channels ─┼─► ORG-11 close-out ─► ORG-UAT
                                                                                                     └─► ORG-10 GM suite + UAT doc ┘
```

The critical path is ORG-01 → ORG-02 → ORG-06 → ORG-07. ORG-01 is kept small: models, wire, decoders and message plumbing, with no behaviour change. The Squad line (ORG-03, ORG-04) needs no database and runs beside the persistent line, so the first playable slice is a squad.

**Contended files.** The coordinator merges these one packet at a time, in the order shown:

- `crates/cell-methods/src/cell/cell_methods/organization/`: ORG-03 split the old `organization.rs` into `mod.rs` (the id-range router), `forward.rs` and a `squad/` directory, with tests in `organization/tests/`. ORG-07 (forwarding to the base) → ORG-08 add files there.
- `crates/base/src/base/dispatch/mod.rs`: ORG-01 (constants and one `organization` arm that calls `dispatch/organization.rs`), then ORG-03 and ORG-07 edit only `organization.rs`.
- `crates/cell-console/src/cell/console/chat/` (`mod.rs`, `spatial.rs`, `feedback.rs`, `squad.rs`, and `chat/tests/`; #885 split the old `chat.rs`, ORG-04 added `squad.rs`) → ORG-09 (team, command and officer). Generic chat logs in these submodules use `target: CHAT_LOG_TARGET`; squad and org chat rows stay on the `squad` and `org` targets (D-ORG24).
- `crates/base-session/src/base/world_entry_chat.rs` (`DEFAULT_CHAT_CHANNELS`) and the `CHAN_*` constants: owned by the social campaign's SS-C4 (D-ORG26), no longer by ORG-09.
- `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs`: ORG-06 only (one call after `push_contact_lists_on_login`).
- `crates/base-session/src/base/helpers/mod.rs` (`destroy_client_entities`): ORG-06 only. It also adds the missing contact-list offline fanout there (audit A-35).
- `crates/cell-interactions/src/cell/interactions/dispatch/interact.rs`: ORG-05 only, one call beside `try_open_dhd`.
- `db/resources/**/entity_templates.sql`, `spawnlist.sql`: ORG-05 only, inside templates 330-349 and spawns 430-449. The stasis hub PR (#846) has merged.

**Carried gaps.** ORG-03 and ORG-04 merged with these open. The packet that inherits each one lists it too.

- No ignore-list check on squad invites. ORG-07 adds the base-side ignore check for `SquadInvite` as well as for Team and Command invites.
- No `/squadpromote`: the 0xD2 path is unconfirmed (ORG-E1 follow-up 1, and [README § Where confidence is low](README.md#where-confidence-is-low)).
- An inviter in gate transit cannot found a squad, and a member in gate transit misses squad chat lines. ORG-UAT step 3 watches for the second.
- The client may display a squad line twice if it also echoes it locally. ORG-UAT step 2 checks this on a real client.
- `crates/entity/src/cell_entity/entity_struct.rs` is over the 700-line hard cap (775 lines on `main`; 751 before ORG-03) and needs a split. No packet owns it yet.
- ORG-04 added the crate edge `cimmeria-cell-console -> cimmeria-cell-methods` (the `.squad_*` commands call the squad handlers). It is the only edge that way.

## Common acceptance

### Telemetry (owner rule, 2026-09-27)

A support question such as "player X tried Y at time T and it failed" must be answerable from SigNoz alone: which path ran, why it was refused, the before and after values of anything transactional, and the correlating ids. Every packet follows [instrumentation-discipline.md](../../architecture/instrumentation-discipline.md), [negative-logging-convention.md](../../architecture/negative-logging-convention.md) and the [observability.md](../../architecture/observability.md) target catalog:

- **Targets.** Squads log on `squad`, Teams and Commands on `org`. Those are the only two targets, both already in `OTEL_FILTER` (ORG-01). A new target needs an `OTEL_FILTER` row and its pin.
- **Spans.** One `level = "info"` span per dispatch entrypoint, named `squad.<action>` or `org.<action>` (`squad.invite`, `squad.invite_response`, `squad.leave`, `squad.kick`, `squad.loot_mode`, `org.create`, `org.invite`, `org.invite_response`, `org.leave`, `org.kick`, `org.rank_change`, `org.set_text`, `org.set_rank_permissions`, `org.login_restore`, and so on), with `skip_all` and correlator fields only.
- **One outcome row per action.** Every handled action ends in exactly one INFO event with `event = "<action>"`, `outcome` (`ok` or `rejected`), and on a refusal a `reason` from a closed, snake_case set that the packet documents in the catalog row (for example `not_member`, `rank_too_low`, `missing_permission`, `target_ambiguous`, `target_in_transition`, `squad_full`, `already_in_org_type`, `rate_limited`, `invite_expired`, `invite_unknown`, `name_taken`, `text_invalid`, `leader_cannot_leave`, `vault_not_empty`). No path returns without one.
- **Identity.** Every event about a player carries the acting player's `account_id` and `player_id`, as `Option`s from the side's identity resolver, never `unwrap_or(0)`. Only an action or transition that involves a second player (an invite, a kick, a rank change, an officer note, a leader change) also carries `target_account_id` and `target_player_id`; nothing is fabricated for a one-player event. `entity_id` stays alongside. Correlators: `org_id` (or `squad_id`), `org_type`, `request_id`, and the method index.
- **State transitions.** One DEBUG event per change, with `event` = `squad_created`, `member_joined`, `member_left` (with `reason` = the `OrgLeaveReason` label), `leader_changed` (`from_player_id`, `to_player_id`, `reason`), `rank_changed` (`from_rank`, `to_rank`), `permissions_changed` (`rank`, `from_mask`, `to_mask`), `text_changed` (`field`, `from_units`, `to_units`, never the text), `loot_mode_changed` (`from`, `to`), `disbanded` (`reason`), `invite_created`, `invite_consumed`, `invite_expired`.
- **Negative seams.** A `rows_affected == 0`, a lookup miss, an identity miss or a dropped `send` logs WARN with `reason`. Each gets a `LogCapture` test (TESTING.md type 12), which is in addition to the CAT-M rejection tests.
- **Metrics.** Counters `squad_actions_total` and `org_actions_total`, labelled only by `action`, `outcome` and `reason` (all enumerated). Never an id as a label.
- **Catalog.** Each packet extends the `org` / `squad` row of the observability.md target catalog with the events and reasons it adds.

- Every behaviour change ships a regression guard that **fails when the change is reverted**. Prove it once (revert, run, restore) and record the proof in the worknote ([TESTING.md](../../../TESTING.md)).
- Client-method output: byte-exact wire tests (type 2). Fanout: type 8, against `TestTransport`. Persistence and every `rows_affected` invariant: live-DB (type 3, `require_db_or_skip!`, sentinels in `0x7000_xxxx`, cleanup by exact id). Rejections: negative-log tests with `LogCapture` (type 12). Two-player flows: one wireclient test (type 11) per wave.
- ORG-LOCK (D-ORG04): every Team/Command mutation takes `lock_org` first and authorizes inside the transaction. ORG-07 adds a concurrency test (TESTING.md type 5): a kick or demotion racing a transaction that holds the org lock never lets the kicked member act.
- Every rejected action gives the player visible feedback on the first press: a re-send of the true state, a line on the feedback channel, or both. `onErrorCode` alone is not enough, because its text needs a cooked-data category Cimmeria does not serve (ORG-E1 Q4). A silent drop is a bug.
- Log targets `org` and `squad` are added to `OTEL_FILTER` with its pinning assertion.
- Each packet updates the docs it owes: `docs/gameplay/organization-system.md` and `group-system.md`, and `docs/protocol/` for any wire message. `docs/gap-analysis.md` §23 and `docs/project-status.md` are close-out only (ORG-11), per the 2026-09-27 rule.
- Workers write a worknote at `docs/analysis/organizations/worknotes/<packet>.md` (CRLF). They do not edit this file or the README; the coordinator owns the ledger.

## Wave 0

### ORG-E1: Client evidence

Status: **Integrated** (#861). Writer: `game-archaeology-specialist`. Static Ghidra and Lua only; no debugger on the live client. Answers: [worknote](worknotes/org-e1.md).

Answer, with addresses and a verdict each, into `docs/reverse-engineering/findings/organization-restoration.md` (close its open questions) and `organization-wire-formats.md`:

1. How the client's per-member record gets its member id (offset 0 of the roster record, audit A-11). Does `onOrganizationRosterInfo` rebuild the list with id 0, and does `onMemberJoinedOrganization` for an existing name update the id in place? This decides the ORG-06 login sequence.
2. The `squadKick(name)` native: which method does it send? Also `squadPromote`, `squadLeave`, and the `/squadinvite` family.
3. Which client method shows another member's minimap ping (`receivedMinimapPing` is server-internal).
4. Does anything turn `onOrganizationCreationResult`'s `Result` and `RetCode` into text? Which `onErrorCode` ids render readable org text (search `ERRORCODE_*` near organization strings)?
5. Does the client hardcode any `EChannel` id (for example `/tell` sending 10, or the officer tab using 6), or does it take every id from `onChatJoined`? Is officer (6) accepted without registration? This settles D-ORG14.
6. Which Lua path shows the squad member frames (`Unit.Squad1..6`): the squad roster, or entity presence? Does a squad member in another space show at all?

Deliverable: a docs-only PR. It blocks nothing; ORG-03, ORG-06 and ORG-09 read it when they start and follow the recommendation if it has not landed.

### ORG-01: Contract foundation

Status: **Integrated** (#871). Writer: `rust-gameserver-dev`. Advisor: `social-systems-engineer`.

Build the whole [contract](#contract-fixed-by-this-ledger) except the schema:

- The models, with literal pins.
- Every client-method builder (34-51, 134, 135), byte-exact against the `.def` field order. Port and check #584's `base/organization/wire.rs`; it had the `onOrganizationLeft` field order wrong once (audit A-30).
- Fix the decoders for CM 13, 14, 15, 17 and 94 to read their strings, with a test per method that feeds a real payload.
- The four base-method constants and decoders (0xCF-0xD2) and one `organization` arm in the base dispatcher. The arm decodes, logs at `debug`, and replies with `onErrorCode` until ORG-03 and ORG-07 fill it in. Fix `docs/protocol/sgwplayer-base-method-dispatch-table.md` if the plan PR has not.
- The `method_idx` entries in `crates/wire/src/mercury/mod.rs`.
- The empty `OrgCellToBase` and `OrgBaseToCell` enums and their outer variants, routed to no-op arms on both sides.
- Public `cell_method(idx, entity_id, args)` and `base_method(msg_id, args)` builders on the wireclient `GameSession`, with a unit test on the bytes.

No gameplay behaviour changes. Acceptance: byte tests for every builder and decoder, the enum pins, and one wireclient builder test.

## Wave 1

### ORG-02: Schema, persistence and the bank API

Status: **Integrated** (#881). Writer: `rust-gameserver-dev`. Reviewer: `database-persistence`.

- The three tables, the sequence and their constraints, wired into `db/database.sql` and the `_primary_keys`, `_foreign_keys`, `_indexes` and `_sequence_ownership` aggregates. No seed rows. No `db/scripts/` migration.
- `crates/base-session/src/base/organization/persistence/`: `create_org` (the org row, one rank row per `OrgRank::for_type`, and the leader, in one transaction), `load_memberships(player_id)`, `load_roster(org_id)`, `add_member`, `remove_member`, `set_rank`, `set_text` (MOTD, note, officer note, rank name), `set_rank_permissions`, `disband`, and the name-uniqueness check.
- The ORG-API read and lock functions (`member_access`, `lock_org`).
- The D-ORG12 leader-deletion trigger, and the `org_vault_is_empty_sql(org_id)` SQL stub it calls (D-ORG20: a last-member delete with a non-empty vault leaves a memberless org). A memberless org is skipped by login restore and fanout, and listed by `.org_list`.
- Live-DB tests: creation writes every rank row; a second Team for the same player fails on `UNIQUE (player_id, org_type)`; a duplicate `name_key` fails; the member `org_type` cannot drift from its org; an `org_id` at or above `0x4000_0000` is refused; disband cascades; `cash` cannot go negative; deleting the leader's character promotes the next member, or disbands a one-member org, and no leaderless org remains (`leader_delete_leaves_no_leaderless_org`); `rows_affected == 0` paths return a typed miss, not `Ok`.

- Telemetry: every persistence function logs DEBUG `event` = its name with `org_id`, `player_id` where there is one, `rows_affected`, and before/after values for rank and permission changes; a typed miss logs WARN with `reason`. The leader-deletion trigger runs outside Rust, so it writes each promotion, disband or memberless result to an `sgw_organization_events` table with immutable identities captured at delete time (`org_id`, `event`, `reason = character_deleted`, `from_player_id`, `from_account_id`, `to_player_id`, `to_account_id`, `at`, `exported_at`). The Rust character-delete handler (`crates/base-world-entry/src/base/character/`) exports the rows its own transaction produced immediately after commit, as `org` `event = leader_changed` / `disbanded` / `left_memberless`, and stamps `exported_at`. A startup sweep exports any row still unstamped (from a GM or test delete that bypassed the handler). Export is at-least-once: a crash between the send and the stamp re-sends the row at the next startup, so every exported event carries the table's `org_event_id`, and a query deduplicates on it. A live-DB test proves both paths.

Message cimmeria-79 when this merges.

### ORG-03: Squad core

Status: **Integrated** (#886). Writer: `rust-gameserver-dev`. Reviewers: `social-systems-engineer`, `server-authority-enforcer`, `testing-validation-engineer` (test plan).

- A service-wide `SquadRegistry` owned by the cell service beside `SpaceManager` (D-ORG03): squads keyed by id from `SQUAD_ORG_ID_MIN` (monotonic, never reused), members (by `player_id`), members in join order, the leader, the loot type, and pending invites (D-ORG06). A `squad_id: Option<i32>` field on `CellEntity` stands in for the `squad` CELL_PUBLIC property, which is never sent to clients (audit A-19).
- `organizationInviteByType(0, name)` at the base resolves nothing itself: it forwards `OrgBaseToCell::SquadInvite { inviter_entity_id, target_name }`. The cell resolves the name with `find_online_player_by_name`. `Ambiguous`, `InTransition` ("that player is travelling, try again"), `NotFound`, self and non-player targets are rejected with feedback; the first match is never guessed. It then checks both sides: the target is not in a squad, the inviter is the leader or has no squad, the squad has room for 6, no duplicate pending invite for the pair, at most 5 pending invites per invitee, at most 5 invites per inviter per 30 seconds, and the target is not ignoring the inviter. It creates the squad on the first accept and sends `onOrganizationInvite` [34].
- CM 8 accept or decline, matched and consumed per D-ORG06, and re-validated on accept (the squad or inviter still exists, there is still room). On accept: `onOrganizationJoined` [35] and `onOrganizationRosterInfo` [38] to the new member, and `onMemberJoinedOrganization` [37] to the others. On decline: feedback to the inviter.
- CM 9 leave (the id must be the caller's own squad), kick (the leader only; ORG-E1 Q2 confirmed it arrives as base method 0xD1 `organizationKick` carrying the squad's id, which the base forwards to the cell; `/squadinvite` is 0xD0 with type 0, and there is no separate squad wire path), promote-to-leader, and disband on the last member. Leader auto-promotion follows D-ORG12. `onOrganizationLeft` [36] with the right `OrgLeaveReason`, and `onMemberLeftOrganization` [39] to the rest.
- CM 18 loot mode per D-ORG16 (leader only, range-checked), and `onSquadLootType` [51] to every member.
- Disconnect: the `DisconnectEntity` arm removes the member with reason `Logout`, and expires their pending invites.
- `organizationInviteByType` for types 1 and 2 is left to ORG-07.
- Telemetry: spans `squad.invite`, `squad.invite_response`, `squad.leave`, `squad.kick`, `squad.loot_mode`; one outcome row per action with the ORG-03 reasons (`target_ambiguous`, `target_in_transition`, `target_not_found`, `self_target`, `not_a_player`, `squad_full`, `already_in_squad`, `not_leader`, `rate_limited`, `invite_limit`, `invite_unknown`, `invite_expired`, `invite_foreign`, `ignored`, `loot_mode_invalid`); transitions `squad_created`, `member_joined`, `member_left`, `leader_changed`, `loot_mode_changed`, `disbanded`, `invite_created`, `invite_consumed`, `invite_expired`; every row carries `squad_id` and the actor's identity, plus the target's where the action has one (invite, response, kick, leader change).
- Tests: unit tests for the registry state machine; fanout byte tests; a negative-log test for each rejection, including the CAT-M names above (two accepts into a five-member squad leave six, never seven); and a two-client wireclient test (invite, accept, both rosters, leave).

## Wave 2

### ORG-04: Squad chat and minimap ping

Status: **Integrated** (#922). Writer: `rust-gameserver-dev`.

- A `CHAN_SQUAD` (4) arm in the cell's chat match: the sender's squad members get `onPlayerCommunication`, wherever they are. The flood limit is the social campaign's `base::rate_limit` `RateCategory::Chat` (SS-00, applied before the cell forward) and the text filter is `org_text::validate(TextField::ChatText)`; neither is re-implemented here. A sender with no squad gets feedback.
- CM 10 `BroadcastMinimapPing` for squads: validate (the id must be the caller's squad; at most one ping per second per member) and log. ORG-E1 Q3 found no receive-side message in the client, so there is nothing to fan out.
- `.squad_invite <name>`, `.squad_join <player>` (a GM joins a target's squad without a handshake) and `.squad_info` in the `.` console, so one tester can build a squad with a sentinel character.
- Telemetry: `squad.chat` and `squad.ping` outcome rows at INFO like every other action (the SS-00 flood limit bounds their rate), `rejected` rows carry `reason` (`not_in_squad`, `rate_limited`, `text_invalid`, `wrong_squad`), and every row carries `recipients` (count) and `text_units`, never the text. GM commands log `org.gm_action` with the actor and the target.
- Tests: fanout reaches members in two different spaces; a non-member sends nothing.

### ORG-05: Creation and the registrar NPCs

Status: **Writing** (dispatched 2026-09-27, branch `org/05-creation`). Writer: `rust-gameserver-dev`. Reviewer: `server-authority-enforcer`. Owner decision D-ORG15: free (the constants are 0).

- An `OrganizationCreation` interaction: `try_open_org_registrar` in `crates/cell-interactions/src/cell/interactions/org_registrar.rs`, called beside `try_open_dhd`. It is keyed on seed data (a template column naming Team or Command, or the `INT_ORGANIZATION` flag plus the type), never on an entity id. It checks distance and eligibility (not already in an organization of that type), records a pending creation keyed by `player_id` with the type, a 5-minute expiry and 3 attempts, and sends `launchOrganizationCreation(type)` [135]. An ineligible player gets feedback instead of a dialog.
- CM 94 `onOrganizationCreation(name)`: requires the pending creation (the type comes from it, never from the wire), validates the name (D-ORG10), and forwards `OrgCellToBase::Create`. The base re-checks eligibility and runs `create_org`, debiting the D-ORG15 cost in the same transaction, and replies. A rejection uses one attempt; a success, a disconnect or a change of space clears the pending creation. The client gets `onOrganizationCreationResult` [134], then `onOrganizationJoined` [35], the header and the roster (reusing ORG-06's login push).
- Seed a Team registrar and a Command registrar in the stasis-room debug hub (templates 330 and 331, spawns 430 and 431), at the slots `docs/content/debug-hub.md` gives, and document them there.
- `.org_create <team|command> <name>` creates directly, skipping the NPC.
- Telemetry: `org.registrar_open` (outcome, `org_type`, `npc_entity_id`, reason `not_eligible` / `too_far`), `org.create` (outcome, `org_type`, `name_units`, the new `org_id`, `cost_before` / `cost_after` of the player's naquadah when the cost is non-zero, reasons `no_pending_creation`, `pending_expired`, `text_invalid`, `name_taken`, `already_in_org_type`, `insufficient_funds`), and `pending_creation_created` / `_consumed` / `_expired` transitions.
- Tests: live-DB creation; a rejection test per invalid name (empty, 61 units, duplicate, already a member); CM 94 without a pending creation is rejected; a seed guard that the two registrar templates exist and carry the interaction.

### ORG-06: Login restore, leave, disband and presence

Status: **Writing** (dispatched 2026-09-27, branch `org/06-presence`). Writer: `rust-gameserver-dev`. Advisor: `social-systems-engineer`.

- On `onClientReady`, after `push_contact_lists_on_login`: for each Team and Command the player belongs to, send `onOrganizationJoined` [35] (`aNewMember = 0`), the name [43], MOTD [45], cash [48], experience [44], rank permissions [49], rank names [50] and the roster [38]. ORG-E1 Q1 fixes the order: the roster first, which the client stores with every member id 0 ("Offline"), then `onMemberJoinedOrganization` [37] with `aNewMember = 0` and the member's entity id for each online member, which is the only message that sets an id.
- Presence fanout: login and every disconnect path (hook `destroy_client_entities`, which covers crash, timeout and quit) tell the online members: `onMemberJoinedOrganization` with the entity id on login, and on logout the same message with id 0 (never `onMemberLeftOrganization`, which removes the row). Confirm the id-0 update against the handler at `0x00e4e4c0` before relying on it. The same hook adds the contact-list offline fanout that is missing today (audit A-35).
- CM 9 leave for Teams and Commands, routed to the base (D-ORG05): D-ORG12's leader rule, D-ORG20's vault check before any disband (ship the `org_vault_is_empty` stub), `onOrganizationLeft` [36] with `Requested`, and `onMemberLeftOrganization` [39] to the online members. The last member leaving disbands the organization, and any online members get `onOrganizationLeft` with `Disbanded`.
- `.org_disband <orgId>` for GMs.
- Telemetry: `org.login_restore` per player (`org_count`, and per org `org_id`, `roster_size`, `online_members`), `org.presence` (`event` = `member_online` / `member_offline`, `recipients`, and the `disconnect_reason` passed from `destroy_client_entities`), `org.leave` and `org.disband` outcome rows (`leader_cannot_leave`, `vault_not_empty`, `not_member`), and the character-delete events from ORG-02's audit table. The contact-list offline fanout it adds logs on its existing target.
- Tests: a live-DB login push for a two-organization player (byte-exact sequence); fanout on each disconnect path; a leader-leave rejection; a disband cascade.

## Wave 3

### ORG-07: Invite, kick and rank change

Status: **BlockedDependency** (ORG-05, ORG-06). Writer: `rust-gameserver-dev`. Reviewers: `server-authority-enforcer`, `testing-validation-engineer`.

- Base methods `organizationInvite` (0xCF) and `organizationInviteByType` (0xD0, types 1 and 2; above 2 is rejected, and neither type is ever created implicitly): the inviter must be a member holding `Invite` in that org, checked under ORG-LOCK; the target must be online (not ambiguous, not self), not already in an org of that type (D-ORG18) and not ignoring the inviter. The ORG-03 invite rate limits apply. The base records the pending invite (D-ORG06, base range) and sends `onOrganizationInvite` [34]. CM 8 with `BASE_INVITE_REQUEST_FLAG` set is forwarded to the base, which consumes and re-validates the entry under ORG-LOCK (D-ORG06), adds the member at the D-ORG07 entry rank, and fans out as ORG-03 does. Base pending invites are cleared on every disconnect path (`destroy_client_entities`).
- Carried from ORG-03: the base also checks the ignore list for `SquadInvite` before forwarding it to the cell, so squad invites get the same ignore rule as Team and Command invites.
- `organizationKick` (0xD1) and `organizationRankChange` (0xD2) under D-ORG09, then `onOrganizationLeft` [36] with `Kicked` and `onMemberLeftOrganization` [39], or `onMemberRankChangedOrganization` [40], to the online members.
- The cell's org-id router forwards CM 9-19 for non-squad ids to the base. CM 11 and 12 (strike team and PvP leave) are rejected and logged as unsolicited, since no strike-team request is ever issued (CAT-M-16, M-17). CM 19 gets the ORG-API stub.
- Deliver `broadcast_to_org` and message cimmeria-79 that the ORG-API is complete. This packet does not close an open org vault window on kick; that is BV-07's (see the ORG-API known gap).
- `.org_join <orgId> [player]` and `.org_rank <player> <rank>` for GMs; they bypass the permission checks but not the type and uniqueness rules.
- Telemetry: `org.invite`, `org.invite_response`, `org.kick`, `org.rank_change` outcome rows with the actor's and target's identity and ranks (`actor_rank`, `target_rank`, `to_rank`) and every D-ORG09 reason (`missing_permission`, `rank_too_low`, `rank_not_in_type`, `leader_not_assignable`, `self_target`); the forward router logs `org.forward` with `route` (`squad` / `base` / `rejected`); `org.strike_team_response` and `org.pvp_leave_response` log `rejected` with `reason = unsolicited`. `broadcast_to_org` logs `recipients` and, at WARN, each failed send.
- Tests: the CAT-M names for M-01, M-02, M-05, M-06, M-16, M-17 and M-18; the ORG-LOCK concurrency test; a two-client wireclient invite into a Command.

## Wave 4

### ORG-08: MOTD, notes and the rank editor

Status: **BlockedDependency** (ORG-07). Writer: `rust-gameserver-dev`. Reviewer: `server-authority-enforcer`.

- CM 13 MOTD (`MOTD`), CM 14 own note (`RosterNotes`), CM 15 officer note (`OfficerNotes`; the name resolves among that org's member rows only, and D-ORG09 (2) applies to the target), CM 16 rank permissions (`AlterPerms`, D-ORG09 (3) and (6); the `Leader` row is refused), CM 17 rank name (`RankNames`, D-ORG09 (3)). Text per D-ORG10. All under ORG-LOCK.
- Fanout: [45], [46], [47] (officer notes only to members holding `OfficerNotes`), [49] and [50].
- Telemetry: `org.set_text` (`field`, `from_units`, `to_units`, `target_player_id` for an officer note) and `org.set_rank_permissions` (`rank`, `from_mask`, `to_mask`, `wire_mask`, and the `PermEditReject` reason), plus `org.set_rank_name`; each with its CAT-M reasons.
- Tests: the CAT-M names for M-07, M-08 and M-10; officer-note fanout filtered by permission.

### ORG-09: Team, Command and officer chat

Status: **BlockedDependency** (ORG-07, and the social campaign's SS-C4). Writer: `rust-gameserver-dev`.

- Chat on team (3), command (5) and officer (6) reaches the online members of the sender's organization of that type. Officer requires `OfficerChat`. SS-00's `RateCategory::Chat` limit and `org_text::validate(TextField::ChatText)` apply; neither is re-implemented. Membership comes from the base, so these channels are routed base-side before the message is forwarded to the cell.
- Register officer (6) with the client when the player holds `OfficerChat` in a Command.
- **Scope change (D-ORG26, agreed with the social coordinator 2026-09-27).** The D-ORG14 channel-id alignment moved to the social campaign's SS-C4: registering server 8 and tell 10 in `DEFAULT_CHAT_CHANNELS`, moving server-to-client sends off 7 and 9 onto the right channels (GM feedback on 9), realigning `CHAN_*` to `enumerations.xml`, swapping the social locals (the `chat_gates.rs` allowlist, `dispatch::tell::TELL_CHANNEL`, SS-C2's `CHAN_SERVER`), `docs/gameplay/chat-system.md`, the DEBUG `channels_registered` event and the pinned registered-set test. This packet builds on SS-C4 and does not repeat any of it.
- Do not re-implement mute. SS-C3 (#925) already routes channels 3-6 unchanged through the base allowlist and applies `.mute` to every channel, the org ones included. `ChatCellToBase` has `Mute` and `Unmute`; this packet may add variants.
- Telemetry: `org.chat` per message at INFO (`ok` or `rejected`; the SS-00 flood limit bounds the rate), with `channel`, `org_id`, `recipients`, `text_units` and the reason.
- Tests: fanout per channel; officer chat without the permission is rejected with feedback; officer (6) is registered only for a player holding `OfficerChat`.

### ORG-10: GM suite and UAT checklist

Status: **BlockedDependency** (ORG-07). Writer: `rust-gameserver-dev`.

- Fill any gaps in the `.` console set: `.org_info [player]` (every membership, rank and permission mask), `.org_list`, `.org_set_perms <orgId> <rank> <mask>`, and `gmReloadOrganizations` (164) as a re-push of the caller's login sequence.
- Write `docs/guides/organizations-uat.md`: the two-client script and the one-client GM fallback for every step of [ORG-UAT](#org-uat-owner-two-client-uat-colo), with what to watch for and which log line each step emits.
- Every GM org command keeps the D-ORG10 caps and the D-ORG09 mask clamp, refuses to edit the `Leader` row, and logs `org.gm_action` with the actor (D-ORG13).
- Telemetry: every GM org command logs `org.gm_action` (INFO) with the GM's identity, the command, the target's identity and the result. The UAT guide gives, for each step, the SigNoz query that shows it (see ORG-UAT).
- Tests: a registry pin for every new command, a dispatch test per command, and a pin that index 164 is inside the gated `SGWGmPlayer` tail.

## Wave 5

### ORG-11: Close-out

Status: **BlockedDependency** (every packet). Writer: coordinator, with `documentation-writer`.

- Update `docs/gameplay/organization-system.md` and `group-system.md` (status tables), `docs/gap-analysis.md` §23, `docs/project-status.md`, `docs/game-systems.md`, the protocol dispatch tables, and `docs/reverse-engineering/findings/organization-restoration.md`.
- Close #568 and #584 with pointers here.
- Write the final `handoffs/session-resume.md`, then comment `/release` on this PR (from PowerShell, or with `MSYS_NO_PATHCONV=1`).

### ORG-UAT: Owner two-client UAT (colo)

Status: **BlockedDependency** (ORG-11). Run by the owner, as GM, with two accounts (A and B). Use `.bug <note>` at each oddity.

1. **Squad invite.** A types `/squadinvite B`. B sees the invite and accepts. Both squad frames show the other. *Solo fallback:* `.squad_join` a sentinel character.
2. **Squad chat and loot.** Both chat on `/squad`; each line shows once, not twice (a client-side echo would double it). B (not leader) changes the loot mode: the menu snaps back with an error. A changes it: both see the change.
3. **Squad across worlds.** B gates to another world. The squad survives, and squad chat still arrives.
4. **Squad leave.** A leaves. B becomes leader, and the squad disbands when B leaves.
5. **Create a Command.** A talks to the Command registrar in the stasis-room debug hub, names the Command, and sees the Command window with A as leader. A second try with the same name is rejected visibly.
6. **Invite into the Command.** A invites B from the Command window. B accepts; both rosters update.
7. **Rank.** A promotes B to Officer. B tries to promote A and is rejected. B, as Officer, invites and kicks a third character (or a GM-joined sentinel).
8. **Texts.** A sets the MOTD, a note and an officer note, and renames a rank. B sees each change; B's view hides officer notes until B holds `OfficerNotes`.
9. **Command and officer chat.** Both reach the other. B loses `OfficerChat` and is refused.
10. **Relog.** Both relog. The Command window, MOTD, ranks and roster come back, and each sees the other come online and go offline.
11. **Team.** Repeat 5 and 6 with the Team registrar. A holds a Team and a Command at the same time (D-ORG18).
12. **Leave and disband.** A cannot leave while B is in the Command. B leaves, then A leaves, and the Command is gone (`.org_list`).

Vault and treasury UAT belongs to the Bank campaign.

**Reading a step back from SigNoz.** Every step leaves rows the owner or the coordinator can query without a repro. In the Logs explorer, filter `service.name = cimmeria-server` and:

| To see | Filter |
|---|---|
| Everything one player did to squads or organizations | `scope_name IN ('squad','org') AND player_id = <id>`, ordered by time |
| Why an action was refused | `scope_name IN ('squad','org') AND outcome = 'rejected'`, grouped by `event`, `reason` |
| One squad's or organization's history | `squad_id = <id>` or `org_id = <id>` |
| Who was told about a change | `event IN ('member_joined','member_left','leader_changed','rank_changed')` with `recipients` |
| Login restore for a player | `event = 'login_restore' AND player_id = <id>` |
| GM test actions | `event = 'gm_action'` |
| Malformed client payloads | `severity_text = 'WARN' AND event LIKE '%malformed'` |

`.bug <note>` at any oddity ties the moment to a `playtest.bookmark` row. ORG-10's guide lists the expected rows for each step.
