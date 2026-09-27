# ORG-02 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [org-01.md](org-01.md).

## Contract

- **Packet:** ORG-02, schema, persistence and the bank API.
- **Decisions in force:** D-ORG04 (ORG-LOCK and the org → `sgw_player` → items lock order), D-ORG05 (id range), D-ORG07 (ranks per type), D-ORG08 and D-ORG21 (default masks), D-ORG10 and D-ORG23 (text rules), D-ORG12 (losing a leader), D-ORG18 (one Team and one Command per player), D-ORG20 (vault predicate, memberless organizations), D-ORG22 (edit semantics, caller-side).
- **Audit rows read:** A-26, A-27, A-30.
- **Base:** first built on ORG-01 (`origin/org/01-foundation` @ `36468911`); #871 squash-merged during the packet, so the branch was rebased with `git rebase --onto origin/main 36468911` onto `origin/main` @ `59c73019`. Branch `org/02-schema`, worktree `.claude/worktrees/org-02`, test DB `sgw_org_02`.
- **Owned paths:**
  - `db/sgw/Organizations/Sequences/sgw_organizations_org_id_seq.sql`, `db/sgw/Organizations/Tables/{sgw_organizations,sgw_organization_ranks,sgw_organization_members}.sql` (new)
  - `db/sgw/_functions.sql`, `db/sgw/_triggers.sql` (new aggregates), `db/sgw/_primary_keys.sql`, `_foreign_keys.sql`, `_indexes.sql`, `_sequence_ownership.sql`, `db/database.sql`
  - `crates/base-session/src/base/organization/` (new: `mod.rs`, `api.rs`, `character_delete.rs`, `persistence/{mod,error,members,texts,loads}.rs`, `persistence/tests/{mod,constraints,mutations,trigger}.rs`), `crates/base-session/src/base/mod.rs` (one `pub mod` line)
  - `crates/base-world-entry/src/base/mod.rs` (re-export `organization`), `crates/base-world-entry/src/base/character/mod.rs` (`handle_delete_character` calls `delete_character`)
  - Docs: `docs/gameplay/organization-system.md` (new Persistence section), `docs/gap-analysis.md` §23, this worknote.
- **Read set:** the ledger sections named in the task; `git show origin/feat/org-system-568:db/sgw/Organizations/**` and `…/base/organization/persistence/mod.rs`; `crates/base-session/src/base/contact_list/` (module and live-DB test shape); `crates/entity/src/organization/` (ORG-01 models); `crates/base-world-entry/src/base/character/mod.rs`; `crates/test-support/src/live_db_gate.rs`; `tools/test-live-db.sh`; `db/database.sql`, `db/resources/_triggers.sql`, `db/split_schemas.py`.

## Evidence

- **Salvage (#584).** Its schema had `leader_player_id` (with `ON DELETE RESTRICT`, which blocks deleting a leader's character outright), `name varchar(128)` with no key column, rank rows for all nine ranks including 0, no `UNIQUE (player_id, org_type)`, no id range and an unbounded sequence. Its persistence took a pool and authorized nothing under a lock. Only the table split and the cascade directions were kept; everything else was rewritten to the ledger.
- `base-session` is already in `tools/test-live-db.sh` (`LIVE_DB_CRATES`), so no infra change was needed and `docs/architecture/integration-test-infra.md` is untouched.
- The only production `DELETE FROM sgw_player` is `handle_delete_character` (`crates/base-world-entry/src/base/character/mod.rs`); every other hit is test cleanup.
- Postgres semantics confirmed by test rather than assumed: a disband cascades ranks and members through two RI cascades, and the rank foreign key's default NO ACTION check does not fire before the members cascade has run (`disband_cascades_ranks_and_members` passes with NO ACTION).

## Design decisions

1. **No leader column; one Leader per org enforced by a unique partial index** (`sgw_organization_members_one_leader_idx`, `(org_id) WHERE rank = 8`). Added on the `database-persistence` review: the whole leadership model rests on "the leader is the member at rank 8".
2. **Rank rows only for the type's ranks** (Team 2, 3, 8; Command 1-8), from `default_rank_permissions`. The member → rank composite FK therefore refuses rank 0 and Team rank 5 in the database. Rank CHECK 1-8; mask CHECK 0-`0x3FF_FFFF`; the Leader row is pinned to `0x3FF_FFFF` by a CHECK (D-ORG08: "no editor, including a GM command").
3. **The member → rank FK is NO ACTION**, so deleting a rank row that members hold is refused instead of cascading the members away. First drafted as CASCADE; changed on review and proven by the disband test.
4. **`name_key` is `org_text::name_key(validate(Name, name))`** (ASCII lowercase of the normalised name). `create_org` and `name_available` both validate, so the database never sees an unvalidated name. `motd`, `note`, `officer_note` are `NOT NULL DEFAULT ''` (the wire has no null); rank `name` is nullable (NULL = client default).
5. **The trigger acts on any member delete, not only the leader's.** It locks the org row (`FOR UPDATE`; NOT FOUND means the org itself is being deleted, so it stands down), returns if a Leader remains, otherwise promotes `ORDER BY rank DESC, joined_at, player_id`, and when nobody remains disbands if `org_vault_is_empty_sql(org_id)` or leaves a memberless org. Acting on any delete means a leaderless org, however it arose, is healed on the next delete.
6. **Character delete pre-locks.** A bare `DELETE FROM sgw_player` locks the member row first and the org second (reverse of D-ORG04), and deadlocks against a kick of the same member, or against a trigger promoting that member, in a transaction holding the org (40P01, reproduced in the regression proof). `organization::character_delete::delete_character` reads the character's orgs, locks them in `org_id` order, then deletes. `handle_delete_character` now calls it. This is outside the packet's listed files, but the deadlock is created by this packet's trigger, so it is fixed here.
7. **Every Rust mutation locks the org itself**, even though the caller will already hold the lock after `member_access_locked`: re-locking in one transaction is free, and it makes "every mutation takes lock_org first" true by construction. `member_access_locked` also takes the lock itself, so it cannot be called outside it. Both take `&mut Transaction`, so neither can run on a bare pool connection.
8. **Typed refusals leave the transaction usable.** Collidable inserts use `ON CONFLICT DO NOTHING` then work out which key matched (`AlreadyMember` vs `AlreadyInType`); the player is checked with `FOR KEY SHARE` (after the org lock) instead of letting the FK fail; `create_org` runs its writes under a savepoint and rolls it back on refusal, so a caller that commits anyway cannot persist a leaderless org (review finding).
9. **Data invariants live in persistence, authority in the handlers.** Persistence refuses: a rank the type does not use; assigning Leader, moving the leader off Leader, editing the Leader mask (`LeaderPinned`); a non-Leader joining a memberless org (`NeedsLeader`); a disband while `api::org_vault_is_empty` is false. It does not check who may act (D-ORG09 (1)-(3), (6)) or D-ORG12's "a leader cannot leave while others remain": if a caller removes the leader anyway the trigger promotes.
10. **`remove_member` reports what the trigger did** (`Unchanged`, `LeaderPromoted`, `Disbanded`, `Memberless`) by comparing the leader before and after, so ORG-06 can fan out without re-deriving it. `disband` returns the member ids for the `Disbanded` fanout.
11. **`set_text` is one function over `OrgTextTarget { Motd, Note, OfficerNote, RankName }`**; `set_rank_permissions` returns the mask it replaced (via an `UPDATE … FROM` self-join, since `RETURNING` sees only new values).
12. **Functions in `db/sgw/_functions.sql`, loaded after `_primary_keys.sql`; the trigger in `db/sgw/_triggers.sql`, loaded last**, mirroring `db/resources/` and what `db/split_schemas.py` emits. The Bank campaign must load its replacement of `org_vault_is_empty_sql` after its vault tables (a SQL-language body is checked at creation, unlike plpgsql under `check_function_bodies = false`).
13. **Isolation.** The trigger and the ORG-API rely on READ COMMITTED (each statement after the lock wait takes a fresh snapshot); documented in both.

## Commands run

All from the worktree root through the lane (`target=B:\targets/org-02`). Every live-DB run reloads `sgw_org_02` from `db/database.sql`; no live-DB test was skipped (the tier sets `DATABASE_URL`, and a set-but-unreachable URL panics rather than skips).

| Command | Exit | Result |
|---|---|---|
| `tools/build-lane/reload-db.sh`, then a psql smoke script (rolled back) | 0 | schema loads; promotion order and disband cascade behave |
| `lane.sh cargo check -p cimmeria-base-session --all-targets` | 0 | |
| `lane.sh cargo check -p cimmeria-base-session -p cimmeria-base-world-entry --all-targets` | 0 | after the review fixes |
| `live-db-test.sh base::organization` | 0 | 26 passed (20 org persistence + 6 wire decoder) |
| `live-db-test.sh "::"` (whole live-DB tier, pre-rebase) | 0 | 3684 passed, 0 skipped |
| `lane.sh cargo fmt --all -- --check` (post-rebase) | 0 | |
| `lane.sh cargo clippy -p cimmeria-base-session -p cimmeria-base-world-entry --all-targets -- -D warnings` (post-rebase) | 0 | |
| `live-db-test.sh base::organization base::character` (post-rebase) | 0 | 43 passed, including the 20 org tests and the `handle_delete_character` live-DB tests |

## Regression proof

Each revert applied on a clean tree, run, then restored with `git checkout -- .`.

- **Run A** (`live-db-test.sh base::organization::persistence --no-fail-fast`, exit 100, 4 FAILED): trigger `FOR UPDATE` removed → `trigger_waits_for_the_org_lock` ("the last member's delete must disband, not strand an empty org"); `UNIQUE (player_id, org_type)` removed → `second_team_for_same_player_is_refused` (raw insert succeeded); the `set_rank` Leader guard removed → `leader_rank_is_pinned` (got `Db(… one_leader_idx)` instead of `LeaderPinned`); the character-delete pre-lock removed → `kick_during_character_delete_does_not_deadlock` (masked here by the missing trigger lock, hence run C).
- **Run C** (only the pre-lock removed; `live-db-test.sh kick_during_character_delete`, exit 100): FAILED with `40P01 deadlock detected … while deleting tuple … in relation "sgw_organization_members"`.
- **Run B** (`… --no-fail-fast`, exit 100, 5 FAILED): `CREATE TRIGGER` commented out → `leader_delete_leaves_no_leaderless_org`, `multi_row_member_delete_promotes_or_disbands_once`, `remove_member_reports_what_the_trigger_did`, `trigger_waits_for_the_org_lock`; `create_org` committing its savepoint on refusal → `second_team_for_same_player_is_refused` ("a refused create_org must write nothing").
- **Run D** (the trigger's vault test replaced by `IF true`; `live-db-test.sh last_member_delete_with_vault`, exit 100): `last_member_delete_with_vault_leaves_memberless_org` FAILED (disbanded instead of memberless).

Restored after each; `git status` clean.

## Test catalogue

All in `crates/base-session/src/base/organization/persistence/tests/`, sentinels `0x7000_4800..=0x7000_49FF` (blocks of 16, 20 of 32 used), organizations cleaned by exact id or exact name key.

- `constraints.rs`: `create_org_writes_every_rank_row`, `second_team_for_same_player_is_refused`, `duplicate_name_key_is_refused`, `member_org_type_cannot_drift`, `org_id_outside_base_range_is_refused`, `cash_and_rank_masks_are_range_checked`, `member_rank_must_have_a_rank_row`, `disband_cascades_ranks_and_members`, `one_leader_and_held_ranks_are_enforced`.
- `trigger.rs`: `leader_delete_leaves_no_leaderless_org` (promotion order, rank beats standing, Team + Command leader deleted at once, solo disband), `last_member_delete_with_vault_leaves_memberless_org` (replaces `org_vault_is_empty_sql` with `SELECT false` inside a rolled-back transaction, then checks the stub is back), `remove_member_reports_what_the_trigger_did`, `multi_row_member_delete_promotes_or_disbands_once`, and two type-5 concurrency tests, `trigger_waits_for_the_org_lock` and `kick_during_character_delete_does_not_deadlock`, which poll `pg_stat_activity` for a lock wait before releasing the holder so they cannot pass without the race.
- `mutations.rs`: `misses_are_typed_not_ok`, `leader_rank_is_pinned`, `rank_and_permission_writes_round_trip`, `texts_are_validated_and_stored`, `loads_return_what_the_login_push_needs`.

## Round 2: telemetry (owner rule, coordinator 2026-09-27)

Ledger: `origin/docs/org-telemetry` (PR #878), "Telemetry (owner rule, 2026-09-27)" and the ORG-02 "Telemetry:" line; worker rules § "Telemetry — mandatory".

- **Persistence events.** `persistence/observe.rs::observed` wraps every public function. On success the body logs a DEBUG `event` named after the function (target `org`) with `org_id`, `player_id` where there is one, `rows_affected`, and before/after values: `set_rank` `from_rank`/`to_rank`, `set_rank_permissions` `from_mask`/`to_mask`, `set_text` `field`/`from_units`/`to_units` (the old text comes back through an `UPDATE … FROM` self-join; only lengths are logged), `remove_member` `from_rank`/`after`. A refusal logs one WARN with `event` = the function and `reason` = `OrgStoreError::reason()`. `lock_org` and `member_access_locked` warn on their own misses (`no_such_org`, `not_a_member`); the persistence layer locks through the unlogged `lock_org_quiet`, so a miss is one WARN, not two.
- **Trigger events: the audit table, not RAISE LOG.** `sgw_organization_events (event_id, org_id, event, reason, from_player_id, from_account_id, to_player_id, to_account_id, tx_id, at, exported_at)`, `event` ∈ `leader_changed` / `disbanded` / `left_memberless`, `reason` ∈ `character_deleted` (the `sgw_player` row was gone when the trigger ran) / `member_removed` (any other member delete, so a psql or GM delete is recorded too; the ledger named only `character_deleted`). No FKs: a disbanded org and a deleted character are what the rows describe.
- **Account ids at delete time.** When the trigger runs on a character delete the `sgw_player` row is already gone, so it cannot look the account up. The member row now keeps an `account_id` copy (a character never changes account), read by `insert_member` with the same `FOR KEY SHARE` select that checks the player exists.
- **Export, exactly once.** `organization/audit.rs`: every exporter is `UPDATE … SET exported_at = now() WHERE exported_at IS NULL AND … RETURNING`, so racing exporters log a row once between them. `character_delete::delete_character` reads `txid_current()` before its commit and, after it, exports that transaction's rows at INFO (`source = character_delete`), returning them in `CharacterDeletion.org_events`; `handle_delete_character` adds `org_events` to its "Character deleted" line. `remove_member` exports its transaction's rows at DEBUG inside the transaction (`source = in_transaction`; a rollback removes row and stamp together). `BaseService::start` spawns `spawn_startup_sweep` (INFO, `source = startup_sweep`). A failed export or sweep logs WARN `org_events_export` / `org_events_swept`, `reason = db_error`, and leaves the rows for the next sweep.
- **Handler edit size.** Small: the handler already called `delete_character` (round 1); the export lives in base-session and the handler only logs the count. No need to move it to ORG-06.
- **No counters.** `org_actions_total` counts handler actions (outcome rows); ORG-02 has no handler, so it adds none.

Round 2 tests (live-DB + `LogCapture`): `audit::character_delete_exports_trigger_events_once`, `audit::startup_sweep_exports_rows_a_bare_delete_left`, `audit::remove_member_exports_its_rows_in_transaction`, `telemetry::typed_misses_log_one_warn_with_reason`, `telemetry::changes_log_debug_with_before_and_after`. Sentinel blocks 20-24.

Round 2 regression proof (`live-db-test.sh base::organization::persistence::tests::telemetry base::organization::persistence::tests::audit --no-fail-fast`, restored with `git checkout -- .` after each):

- WARN removed from `observed` and the post-commit export removed from `delete_character`: exit 100, `typed_misses_log_one_warn_with_reason` (no `set_rank`/`not_a_member` WARN) and `character_delete_exports_trigger_events_once` (`org_events` empty) FAILED.
- Persistence lock switched back to the logging `lock_org` and the trigger's `leader_changed` INSERT removed: exit 100, `typed_misses_log_one_warn_with_reason` (two `lock_org` WARNs), `character_delete_exports_trigger_events_once`, `startup_sweep_exports_rows_a_bare_delete_left` and `remove_member_exports_its_rows_in_transaction` FAILED.

Round 2 commands: `lane.sh cargo clippy -p cimmeria-base-session -p cimmeria-base-world-entry -p cimmeria-base --all-targets -- -D warnings` (0); `live-db-test.sh base::organization base::character` (0, 48 passed); the whole live-DB tier `live-db-test.sh "::"` (0, 3700 passed, 0 skipped); `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` (0, 52 passed: the target scan still sees `org` reach SigNoz); `cargo fmt --all -- --check` (0).

SigNoz filters (Logs, `service.name = 'cimmeria-server'`):

| What | Filter |
|---|---|
| Trigger results from character deletes | `scope_name = 'org' AND reason = 'character_deleted'` (then `event` = `leader_changed` / `disbanded` / `left_memberless`) |
| Everything a given deleted character did to its orgs | `scope_name = 'org' AND from_player_id = <id>` |
| Rows the startup sweep caught | `scope_name = 'org' AND source = 'startup_sweep'` |
| Refused persistence writes | `scope_name = 'org' AND severity_text = 'WARN' AND reason IS NOT NULL` (narrow on `event`) |
| Rank / mask / text changes | `scope_name = 'org' AND event IN ('set_rank', 'set_rank_permissions', 'set_text')` |
| Export failures | `scope_name = 'org' AND event IN ('org_events_export', 'org_events_swept')` |

## Known gaps

- `disband`'s `VaultNotEmpty` refusal is untested: the Rust stub always returns true. The Bank campaign's replacement should add the test.
- `delete_character` reads memberships before locking; an org the character joins in that window is not pre-locked (its trigger then runs with the old lock order). Narrow, and no worse than before.
- A character delete from any other path (a future GM tool, account deletion via the `account` cascade) should go through `delete_character` or take the same locks; a bare delete stays correct but can deadlock.
- `load_memberships` / `load_roster` are display reads with no lock, by design; nothing may authorize from them.
- No `updated_at` column on any table.
- The startup-sweep call in `BaseService::start` has no test of its own; `sweep_unexported` is tested directly.
- Persistence DEBUG events are written inside the caller's transaction, before the caller decides to commit; the handler's INFO outcome row (later packets) is the committed record.
- `sgw_organization_events` is never pruned. It grows only with trigger results (leader changes, disbands), which are rare.

## Integration edits for the coordinator

- **Ledger:** record `sgw_organization_events` and the member `account_id` column in "Schema". Also record in work-packets.md "Schema" that ranks rows exist only for the type's ranks, the Leader-row CHECK, the one-Leader partial index, `motd`/notes `NOT NULL DEFAULT ''`, and the NO ACTION rank FK. Record that `lock_org` and `member_access_locked` take `&mut Transaction<'_, Postgres>` and return `Result<_, sqlx::Error>`, and that `member_access_locked` takes the lock itself.
- **New contract surface for ORG-05/06/07/08:** `persistence::{create_org, add_member, remove_member (MemberRemoval/AfterRemoval), set_rank, set_text (OrgTextTarget), set_rank_permissions, disband, load_memberships, load_roster, load_ranks, name_available, OrgStoreError}` and `character_delete::delete_character`. ORG-05 debits the D-ORG15 cost inside the `create_org` transaction after it returns (org row locked first, then `sgw_player`).
- **Bank campaign (cimmeria-97):** the ORG-API is at `crates/base-session/src/base/organization/api.rs`; the two vault stubs to replace are `api::org_vault_is_empty` and `org_vault_is_empty_sql` in `db/sgw/_functions.sql` (load the replacement after the vault tables). Message them when this merges.
- `crates/base-world-entry/src/base/character/mod.rs` is touched (one call site); flag it if another packet owns that file.
