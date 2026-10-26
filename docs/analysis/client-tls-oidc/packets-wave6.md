# client-tls-oidc wave 6: upgrade fix and the revocation model (PR #2031 review)

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract C16](contract.md#c16-revocation-model-pr-2031-review-waves-6-and-7-ruling-2026-10-26), [wave 7](packets-wave7.md), [ledger](README.md), [review ruling](worknotes/pr2031-ruling.md)

Waves 6 to 9 fix the PR #2031 review (head `8225135ef`, findings F01-F19). Wave 6 fixes the first-upgrade rebuild (F01) and builds the database and in-memory halves of the revocation model (contract C16.1-C16.3, C16.5, C16.6; findings F02, F03, F04). Wave 7 carries the model into the game-login handoffs (F05).

**Ships as:** every packet merges into the integration branch `client-tls-oidc/campaign` (worktree `cto-camp`) with `merge-noff.ps1`, in the order below. There is still **one PR, #2031**; no wave PR and no packet PR. Packet worktrees branch at the campaign head current at dispatch (`8225135ef` for the first ones), or at the named packet's reviewed head. Waves 8 and 9 touch other crates and run **in parallel with wave 6**.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-70 Rebuild counts a table the old schema lacks as 0 (F01) | packet-coder | none | - |
| CT-71 One advisory lock per refresh family (F02) | packet-coder | none | - |
| CT-72 Access-token fences (F02, C16.3) | packet-coder | none | - |
| CT-74 Reuse revokes the game ticket before it audits (F03) | packet-coder | none | - |
| CT-73 A new family is bound to its identity (C16.2) | packet-coder | none | CT-71 and CT-72 merged into the campaign |
| CT-75 Add method carries its family (F04, C16.5) | packet-coder | none | CT-73 merged |
| CT-76 Unlink revokes inside its transaction (C16.6) | packet-coder | none | CT-75 merged |

**Merge order:** CT-70, CT-71, CT-72, CT-74, CT-73, CT-75, CT-76. Dispatch CT-70, CT-71, CT-72 and CT-74 at once (CT-72 and CT-74 both edit `routes/token.rs`, in different functions: CT-72 `token`, `answer`, `code_grant`, `signed_in`, `session`; CT-74 only the `Rotation::Reused` arm of `refresh_grant`). If `merge-noff.ps1` stops on a conflict, `ops` aborts the merge and the orchestrator dispatches a fresh packet-coder in the later packet's worktree to merge the campaign head in and rerun its checks. CT-73, CT-75 and CT-76 each branch at the campaign head after their dependency merged.

**After the wave:** the wave review loop on `cto-camp`. The `network-security-auth` pass runs once, after wave 7 merges, over C16 as a whole.

**Sentinel ids** (each `const NAME: i32 = 0x7000_E...;`, unique across files; `crates/test-support/src/sentinel_lint.rs` fails on a duplicate): CT-71 `0x7000_E000..0x7000_E0FF`, CT-73 `0x7000_E100..`, CT-74 `0x7000_E200..`, CT-75 `0x7000_E300..`, CT-76 identity `0x7000_E400..`, CT-76 base-session `0x7000_E480..`.

## CT-70 Rebuild counts a table the old schema lacks as 0 (F01)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-70-rebuild-new-tables` · **Worktree:** `cto-70`
- **Commit:** `fix(docker): count a carried table the old database lacks as empty (CT-70)`
- **Rules (full text in [work-packets.md](work-packets.md#rules-every-packet-carries)):**
  - R1: PowerShell only, never the Bash tool; start every command with `Set-Location <repo>\.claude\worktrees\cto-70;`; absolute paths for every file write.
  - R3: edit only the files below; `git add` explicit paths; commit with the subject above and the Haiku attribution lines from work-packets.md; no `git stash`, rebase or push.
  - R4: files stay under 500 lines (both scripts are under 300).
  - R5: `docs/**/*.md` is CRLF (check `([IO.File]::ReadAllText('<abs>') -match "[^\r]\n")` is `False`). `docker/*.sh` and `tools/*.sh` are LF; keep them LF.
  - R7: no IPs, secrets or account names.
  - No cargo: this packet compiles nothing. The shell scripts run in CI only (`pr-container.yml`, job step "persistence smoke"); you cannot run Docker here.
- **The defect.** `docker/db-rebuild.sh` step 8 (l.173-194) runs `SELECT count(*) FROM ONLY "<sch>"."<tbl>"` against the **old** database (`$SRC`) for every line of `$WORK/carry.list`. This campaign adds `account_identity`, `account_identity_event` and `auth_refresh_token` to `db/persistent/persistent-public-tables.txt`; the first rebuild over a database made before this campaign fails with `relation "public.auth_refresh_token" does not exist`, `db-sync.sh` exits 1 and the server never starts. Steps 4 and 6 already cope: an absent table has no TOC line in the source dump, and step 6 reads `$NEXT`.

Files:

1. `docker/db-rebuild.sh`, step 8 only. Before the `$SRC` count, ask whether the table exists there:

   ```sh
   have=$($PSQL -d "$SRC" -At -c "SELECT to_regclass('\"$sch\".\"$tbl\"') IS NOT NULL" 2>"$WORK/err") \
       || fail "existence check of $qt in $SRC failed: $(lasterr "$WORK/err")"
   if [ "$have" = t ]; then
       cs=$($PSQL ... as today ...) || fail "... as today ..."
   else
       echo "[db-rebuild] step 8 $qt absent in $SRC (new table), counted as 0"
       cs=0
   fi
   ```

   Every other failure still fails (a psql error, a `$NEXT` count error, a mismatch). Do not touch the `$NEXT` count: the fresh dump always has the table.
2. `tools/container-persistence-smoke.sh`: a new scenario **3b "first upgrade from a schema without the identity tables"**, placed right after scenario 3's `echo "PASS scenario 3"`, in the style of the others (`psql_in`, `force`, `assert_log`, `assert_eq`):
   - Record `ACCOUNTS="$(psql_in "SELECT count(*) FROM public.account")"`.
   - Make the live database look like the base schema: drop the two guard triggers and their functions, then the three tables. Read the exact names from `db/sgw/_triggers.sql` (`account_password_guard` on `account`, `account_identity_password_guard` on `account_identity`) and `db/sgw/_functions.sql`; then `DROP TABLE account_identity_event, account_identity, auth_refresh_token CASCADE`. Grep `db/sgw/` for any other object that names these tables (a view, a function) and drop it first; list what you dropped in the report.
   - `force`, then `assert_log "rebuild"` and `assert_log "absent in"`.
   - Assert each of the three tables exists again (`SELECT to_regclass('public.<t>') IS NOT NULL` is `t`) and is empty (`count(*)` is `0`), the account count equals `$ACCOUNTS`, and the scenario-2 sentinel character `$S1` is kept.
   - `echo "PASS scenario 3b"`.
   - Scenario 3 (a same-schema rebuild) stays as the subsequent-upgrade case. Update the scenario list in the script's header comment.
3. `docs/operations/persistent-player-data.md`: one sentence where the rebuild steps are described (grep for "count"): a carried table that the old database lacks is counted as 0, so a release that adds a persistent table upgrades in place. Keep CRLF.

Also check, and say in the report: the local refresh `bootstrap/CimmeriaBootstrap/Private/Invoke-CimmeriaResourcesRefresh.ps1` has no count check and its `pg_dump -a -t` list is not `--strict-names`, so an absent table is skipped there. Change nothing in it unless that is false; if it is false, stop and report `blocked - local refresh also fails`.

**Test.** The new smoke scenario is the regression guard (an integration test of the container scripts): without the step-8 change the forced rebuild exits 1 at "count of public.account_identity... failed", so `assert_log "rebuild"` and the table assertions fail. It runs in CI only; the PR's container job proves it.

**Checks:** `git diff --check`; `([IO.File]::ReadAllText('<repo>\.claude\worktrees\cto-70\docs\operations\persistent-player-data.md') -match "[^\r]\n")` is `False`; the two `.sh` files have no CR (`([IO.File]::ReadAllText('<abs>') -match "\r")` is `False`). **Docs rows:** `docs/operations/persistent-player-data.md` (item 3).

## CT-71 One advisory lock per refresh family (F02)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-71-family-lock` · **Worktree:** `cto-71`
- **Commit:** `fix(identity): serialize every refresh-family operation on one lock (CT-71)`
- **Rules:**
  - R1: PowerShell only; start every command with `Set-Location <repo>\.claude\worktrees\cto-71;`; absolute paths for file writes.
  - R2: every cargo call through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground; never cargo directly; never `--target-dir`; read the lane log under `logs/cto-71/` before rerunning; at most 5 fix rounds per check.
  - R3: edit only the files below; `git add` explicit paths; commit at once with the subject above and the Haiku attribution lines; no stash, rebase or push.
  - R4: `refresh.rs` is 334 lines and stays under 500; new tests go in a new file.
  - R6: live-DB tests have `live_db` in the test name, start with `require_db_or_skip!`, connect with `test_support::database_url()`, use `i32` sentinel ids from `0x7000_E000..0x7000_E0FF` (a `const` each), and delete only the exact ids and family ids they inserted.
  - R7: test values `example.org`, `Player_1234`; no real names, tokens or IPs.
- **Contract excerpt (C16.1, verbatim from [contract.md](contract.md#c161-database-one-lock-per-refresh-family-ct-71)):**

  ```rust
  async fn family_lock(conn: &mut PgConnection, family_id: &str) -> Result<(), RepoError>;
  //   SELECT pg_advisory_xact_lock(hashtextextended('cimmeria.refresh_family:' || $1, 0))
  // rotate: BEGIN; SELECT family_id FROM auth_refresh_token WHERE token_hash = $1 (no row: Unknown);
  //   family_lock(family_id); then today's SELECT .. FOR UPDATE row read and today's branches
  async fn revoke_family_on(conn, family_id, now_ts) -> Result<(), RepoError>;   // family_lock, then today's UPDATE
  pub async fn revoke_account_on(conn: &mut PgConnection, account_id: u32, keep_family: Option<&str>,
      now: DateTime<Utc>) -> Result<Vec<String>, RepoError>;
  //   1. SELECT DISTINCT family_id ... WHERE account_id = $1 AND ($2::text IS NULL OR family_id <> $2)
  //        AND revoked_at IS NULL ORDER BY family_id
  //   2. family_lock(f) for each, in that order
  //   3. UPDATE .. SET revoked_at = $3::timestamptz WHERE <same predicate> RETURNING family_id (dedup, sorted)
  ```

  `revoke_family` becomes `begin; revoke_family_on; commit`; `revoke_account` and `revoke_account_except` become `begin; revoke_account_on(.., None | Some(keep), ..); commit`, signatures unchanged.

Files:

1. `crates/identity/src/repo/refresh.rs`: add `family_lock` (private); change `rotate` (l.126) to the shape above (the existing `SELECT .. FOR UPDATE` row read stays, after the lock); `revoke_family_on` (l.316) takes the lock first; `revoke_family` (l.200) drops its own lock statement and calls `revoke_family_on`; add `pub async fn revoke_account_on`; `revoke_account` (l.221) and `revoke_account_except` (l.249) become thin wrappers. Update the doc comments: say why the lock is per family (a successor inserted between two statements). `revoke_token` and `purge_expired` do not change.
2. `crates/identity/src/repo/refresh_lock_tests_live_db.rs` (new), wired from `refresh.rs` with `#[cfg(test)] #[path = "refresh_lock_tests_live_db.rs"] mod lock_tests_live_db;`.

Tests (live-DB integration tests in the module):

| Test | Shape | Fails on revert because |
|---|---|---|
| `live_db_reuse_during_a_successor_rotation_revokes_the_successor` | Insert account `TEST_ACCOUNT` and family F: spent A (`revoked_at` and `replaced_by` = B), live B. Connection 1: `BEGIN`; `family_lock(F)` (call the private fn: the test module is a child module); lock B `FOR UPDATE`; mark B revoked/replaced by C; insert live C. Spawn `rotate(pool, A_plain, now)`. Poll `pg_locks` up to 10 s for a not-granted lock in `current_database()`; commit connection 1 whether or not one appeared. Await the task. | It answers `Reused` and then no row of F has `revoked_at IS NULL`. Today's single-snapshot UPDATE waits on B, skips it and never sees C, so C stays live (reproduced in database-evidence.md). |
| `live_db_account_revoke_waits_for_an_inflight_rotation` | As above, but the second party is `revoke_account(pool, TEST_ACCOUNT, now)`. | No live row is left for the account; without the family lock the UPDATE misses C the same way. |
| `live_db_revoke_account_on_keeps_the_named_family` | Two families F1, F2 live; `revoke_account_on(tx, id, Some(F1), now)` in a transaction, commit. | Returns `[F2]`; F1's row stays live, F2's is revoked. |

Generate tokens with the module's own `issue` so hashes are real; to make A spent, `rotate` once (A to B) before connection 1 starts. Clean up `DELETE FROM auth_refresh_token WHERE account_id = TEST_ACCOUNT` then the account row, both by exact id.

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `refresh_lock_tests_live_db`; S2 `cimmeria-test-support` (the sentinel lint). **Docs rows:** none (contract C16.1 already holds the design; CT-92 folds it into the tracked docs).

## CT-72 Access-token fences (F02, C16.3)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-72-access-fences` · **Worktree:** `cto-72`
- **Commit:** `fix(identity): an access token minted after its revocation is refused (CT-72)`
- **Rules:** R1 (worktree `cto-72`), R2 (logs under `logs/cto-72/`), R3, R7 as in CT-71. R4: `stores.rs` shrinks (the access store moves out); `access_tokens.rs` stays under 400 lines including its tests file split (`access_tokens_tests.rs` via `#[path]`); `routes/token.rs` (447 lines) must stay under 500. R6: unit tests in a sibling `_tests.rs` file.
- **Contract excerpt (C16.3):**

  ```rust
  // crates/identity/src/access_tokens.rs (new), re-exported from stores.rs:
  //   pub use crate::access_tokens::{AccessGrant, AccessTokens};
  struct Inner {
      seq: u64,                                        // bumped by every revoke_*
      by_hash: HashMap<[u8; 32], AccessGrant>,
      family_revoked: HashMap<String, (u64, Instant)>, // family -> (seq of the revoke, when)
      account_revoked: HashMap<u32, AccountTomb>,
  }
  struct AccountTomb { seq_all: u64, seq_except: u64, keep: Option<String>, at: Instant }
  pub struct AccessGrant { /* today's fields */ pub minted_seq: u64 }
  pub fn fence(&self) -> u64;
  pub fn mint(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant) -> String;   // unfenced: tests and tools only
  pub fn mint_fenced(&self, account_id: u32, family_id: &str, via_code: bool, now: Instant, fence: u64) -> Option<String>;
  pub fn grant_live(&self, grant: &AccessGrant) -> bool;
  ```

  `revoked_since(account, family, fence)` is true when `family_revoked[family].0 > fence`, or `account_revoked[account].seq_all > fence`, or `seq_except > fence` and `keep != Some(family)`. `mint_fenced` answers `None` when it is true. Both mints record `minted_seq = seq`. `revoke_family` and `revoke_account` bump `seq` and set `family_revoked` / `seq_all`; `revoke_account_except(account, keep)` bumps `seq`, sets `seq_except` and `keep`, and when an older tombstone had a different `keep`, first raises `seq_all` to the old `seq_except`. Tombstones live `ACCESS_LIFE`, are pruned by `reap`, and count against `STORE_CAP` in `make_room` (oldest evicted first). `revoke_*` signatures do not change. `grant_live(g)` is `!revoked_since(g.account_id, &g.family_id, g.minted_seq)` and the grant still being in `by_hash`.

  Routes capture `let fence = state.ctx.stores.access.fence();` at entry, before any await: `token()` (passed down through `answer` to `code_grant` and `refresh_grant`), `link::create` and `link::link_existing`. `session()` calls `mint_fenced`; `None` answers `ApiError::InvalidGrant` and best-effort revokes the just-issued family with `refresh::revoke_family` (a failure is a WARN with `reason = "fence_cleanup_failed"`, never the token). No route calls the unfenced `mint`.

Files:

1. `crates/identity/src/access_tokens.rs` (new): move `AccessTokens` and `AccessGrant` out of `crates/identity/src/stores.rs` (the `AccessGrant {` literal is at `stores.rs:248`) and add the fence state above. Keep the existing methods and their behaviour (lookup by hash, `revoke_*`, `reap`, `make_room`); add `fence`, `mint_fenced`, `grant_live`. Register the module in `crates/identity/src/lib.rs`.
2. `crates/identity/src/stores.rs`: `pub use crate::access_tokens::{AccessGrant, AccessTokens};` so every `crate::stores::...` path keeps compiling.
3. `crates/identity/src/routes/token.rs`: `token` captures the fence; `answer`, `code_grant`, `refresh_grant`, `attach`, `signed_in` (l.282) and `session` (l.299) take `fence: u64`; `session` calls `mint_fenced` with the shape above. Keep `signed_in`'s `family` parameter in this packet (CT-73 replaces it).
4. `crates/identity/src/routes/link.rs`: `create` and `link_existing` capture the fence at entry and pass it to `signed_in` (calls at about l.145 and l.233).
5. `AccessGrant { .. }` struct literals outside the store gain `minted_seq: 0`: `crates/identity/src/routes/account_disabled_tests_live_db.rs:59`, `crates/identity/src/routes/ticket.rs:215` (a test literal; re-grep `AccessGrant {` across `crates/` and fix each). The unfenced `mint` callers in tests and tools stay as they are (`stores_tests.rs`, `wiring.rs:160`, `revoke.rs:60-61` tests, `account_admin/tests_live_db.rs:247`, `account_tests_live_db.rs:176`, `account_unlink_tests_live_db.rs:140/144`).
6. `crates/identity/src/access_tokens_tests.rs` (new), the unit tests below.

Tests (unit, in `access_tokens_tests.rs`):

| Test | Fails on revert because |
|---|---|
| `a_mint_fenced_before_a_family_revoke_is_refused` (fence, `revoke_family(F)`, `mint_fenced(.., F, .., fence)` is `None`) | Without the tombstone the mint succeeds. |
| `a_mint_fenced_after_the_revoke_succeeds` (revoke, then fence, then mint) | Guards against a fence that refuses forever. |
| `account_revoke_refuses_every_family_and_except_keeps_one` | `seq_all` / `seq_except` semantics. |
| `a_second_except_with_another_keep_revokes_the_first_keep` | The `seq_all` raise when `keep` changes. |
| `grant_live_is_false_for_a_grant_minted_before_its_family_revoke` | `grant_live` is CT-77's check. |
| `tombstones_expire_after_access_life_and_count_against_the_cap` | `reap` and `make_room`. |

Route test (unit or live-DB, whichever the existing `token_tests_live_db.rs` harness allows without touching its cleanup, which CT-82 owns): add `live_db_refresh_after_family_revoke_mints_no_access_token` in a **new** file `crates/identity/src/routes/token_fence_tests_live_db.rs` (sentinels `0x7000_E0C0..0x7000_E0FF`, exact-id cleanup): capture a fence, revoke the family through `state.ctx.stores.access.revoke_family`, then call `session` through the route path with the old fence; it answers `invalid_grant` and the family's refresh rows are revoked. If wiring a pre-captured fence into the route is not possible without a test hook, test `session` directly (it is `pub(super)`-reachable from a sibling test module) and say so in the report.

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `token_fence_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** none (CT-92).

## CT-74 Reuse revokes the game ticket before it audits (F03)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-74-reuse-revokes-ticket` · **Worktree:** `cto-74`
- **Commit:** `fix(identity): refresh reuse ends the launch ticket, even when the audit fails (CT-74)`
- **Rules:** R1 (worktree `cto-74`), R2 (`logs/cto-74/`), R3, R6 (sentinels `0x7000_E200..0x7000_E2FF`), R7 as in CT-71. R4: `token.rs` stays under 500 lines. Edit only the `Rotation::Reused` arm in `token.rs`; CT-72 edits the rest of that file at the same time.
- **Contract excerpt (C16.6, last sentence):** `Rotation::Reused` runs `access.revoke_family(&family_id)` and `launch_tickets.revoke_account(account_id)` **before** `event::record`, whose failure is then a WARN, not a skipped revoke.

Files:

1. `crates/identity/src/routes/token.rs`, `refresh_grant` `Rotation::Reused` arm (l.263-274): first `state.ctx.stores.access.revoke_family(&family_id);` and `state.deps.launch_tickets.revoke_account(account_id);`, then `event::record(..)`; on `Err(e)` log `tracing::warn!(account_id, reason = "reuse_audit_failed", error = %e, "refresh reuse audit event not recorded")` and still answer `invalid_grant`. The account-wide ticket revoke matches logout and the disabled branch: a ticket carries no family (accepted; W3-1).
2. `crates/identity/src/routes/token_reuse_tests_live_db.rs` (new), wired from `token.rs` the way its existing `token_tests_live_db.rs` is.

Tests (live-DB):

| Test | Shape | Fails on revert because |
|---|---|---|
| `live_db_reuse_revokes_a_minted_launch_ticket` | Account + family; mint a ticket with `state.deps.launch_tickets.mint(account, "Player_1234", ..)`; rotate once (A to B); present A again through the route (`token()` with `grant_type = refresh_token`). Then `launch_tickets.verify("Player_1234", <sha1hex of the ticket>, now)`. | Answers `TicketCheck::NoMatch`. Today the reuse arm never touches the ticket store, so it is still `Valid`. |
| `live_db_reuse_revokes_when_the_audit_insert_fails` | As above, with a test-only trigger that makes `event::record` fail for the sentinel account only (below). Assert the response is `invalid_grant`, the ticket is `NoMatch` and the family's access token no longer resolves. | Today the `?` on `event::record` returns before either revoke runs. |

  The audit-failure trigger is scoped to the sentinel account so parallel tests are unaffected: `CREATE OR REPLACE FUNCTION ct74_fail_event() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'ct74'; END $$;` and `CREATE TRIGGER ct74_fail_event BEFORE INSERT ON account_identity_event FOR EACH ROW WHEN (NEW.account_id = <sentinel>) EXECUTE FUNCTION ct74_fail_event();`. Run `DROP TRIGGER IF EXISTS ct74_fail_event ON account_identity_event` at the start and in cleanup, then drop the function. Never rename or alter the table itself.

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `token_reuse_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** none.

## CT-73 A new family is bound to the identity that earned it (C16.2)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-73-issue-bound` · **Worktree:** `cto-73` (branch at the campaign head after CT-71 and CT-72 merged)
- **Commit:** `fix(identity): issue a refresh family only while the identity is still linked (CT-73)`
- **Rules:** R1 (worktree `cto-73`), R2 (`logs/cto-73/`), R3, R6 (sentinels `0x7000_E100..0x7000_E1FF`), R7 as in CT-71. R4: `refresh.rs` and `token.rs` stay under 500 lines.
- **Why:** a sign-in resolves its account (`find_bound`), then awaits more work before `refresh::issue`. A GM or player unlink that commits in between revokes every family it can see; the family issued afterwards was never seen. Binding issuance to the account row closes the gap, because unlink holds that row `FOR UPDATE` across its delete.
- **Contract excerpt (C16.2):**

  ```rust
  pub async fn issue_on(conn: &mut PgConnection, account_id: u32, family: Option<&str>, now: DateTime<Utc>)
      -> Result<(String, String), RepoError>;          // today's `issue` body, on a connection
  pub async fn issue(pool: &PgPool, account_id: u32, family: Option<&str>, now: DateTime<Utc>)
      -> Result<(String, String), RepoError>;          // unchanged; acquires a connection, calls issue_on
  pub enum Issued { Pair(String, String), Disabled, Unbound }   // Pair = (refresh_token, family_id)
  pub async fn issue_bound(pool: &PgPool, account_id: u32, identity_id: i64, now: DateTime<Utc>)
      -> Result<Issued, RepoError>;
  //   BEGIN
  //   SELECT enabled FROM account WHERE account_id = $1 FOR SHARE      -- no row: Unbound
  //   enabled = false: Disabled (rollback)
  //   SELECT EXISTS (SELECT 1 FROM account_identity WHERE identity_id = $2 AND account_id = $1)  -- false: Unbound
  //   issue_on(tx, account_id, None, now); COMMIT; Pair
  ```

  `routes::token::signed_in` becomes `signed_in(state, account_id: u32, identity_id: i64, fence: u64, via_code: bool, now: Instant)`: it calls `issue_bound`; `Disabled` is `ApiError::AccountDisabled`, `Unbound` is `ApiError::InvalidGrant`. Callers pass the identity that resolved the sign-in: `Resolution::Existing` the `find_bound` id, `AutoLink` and the two link routes `linked.identity_id`. `attach` keeps calling `signed_in` in this packet with the identity it found or inserted; CT-75 gives it its own transaction.

Files:

1. `crates/identity/src/repo/refresh.rs`: `issue_on`, `issue` as a wrapper, `Issued`, `issue_bound`.
2. `crates/identity/src/routes/token.rs`: `signed_in` as above; callers in `code_grant` (`Existing`: `bound_id` is always `Some` there; if it is `None`, answer `InvalidGrant`), `AutoLink` (`linked.identity_id`), `attach` (the found id, or the inserted `Linked.identity_id`: `identity::insert` returns it). `identity::insert` must return the id; if it does not, report `blocked - contract gap`.
3. `crates/identity/src/routes/link.rs`: both `signed_in` calls pass `linked.identity_id`.
4. `crates/identity/src/repo/refresh_bound_tests_live_db.rs` (new, `#[path]` from `refresh.rs`).

Tests (live-DB):

| Test | Fails on revert because |
|---|---|
| `live_db_issue_bound_refuses_an_unlinked_identity` (insert account + identity, delete the identity, `issue_bound` is `Unbound`, no row in `auth_refresh_token` for the account) | Today `issue` inserts unconditionally. |
| `live_db_issue_bound_waits_for_an_unlink_in_flight` (connection 1: `BEGIN; SELECT .. FROM account WHERE account_id = $1 FOR UPDATE; DELETE FROM account_identity WHERE identity_id = $2`; spawn `issue_bound`; poll `pg_locks` up to 10 s for a waiting lock; commit; the task answers `Unbound`) | Without `FOR SHARE` the issue does not wait and inserts a live family. |
| `live_db_issue_bound_refuses_a_disabled_account` | `Disabled` arm. |
| `live_db_issue_bound_issues_for_a_linked_identity` | Guards against refusing everything. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `refresh_bound_tests_live_db`; S3 `routes::` (the existing route live-DB tests still pass with the new signature); S2 `cimmeria-test-support`. **Docs rows:** none.

## CT-75 Add method carries its family (F04, C16.5)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-75-link-intent-family` · **Worktree:** `cto-75` (branch at the campaign head after CT-73 merged)
- **Commit:** `fix(identity): Add method needs the session that started it to be live (CT-75)`
- **Rules:** R1 (worktree `cto-75`), R2 (`logs/cto-75/`), R3, R6 (sentinels `0x7000_E300..0x7000_E3FF`), R7 as in CT-71. R4: move `attach` out of `token.rs` into a new `crates/identity/src/routes/attach.rs` (`pub(super) async fn attach`), so `token.rs` shrinks.
- **The defect.** `/link/start` turns a code-based Bearer token into `LinkIntent { account_id }`. An unlink or a reuse revoke that ends that session leaves the intent, the pending provider authorization and the handoff code alive; finishing the browser step afterwards reaches `attach`, which attaches a new identity and issues a new family.
- **Contract excerpt (C16.5):** `LinkIntent { account_id: u32, family_id: String }`: `/link/start` copies `grant.family_id`. `attach` runs one transaction: `SELECT enabled FROM account WHERE account_id = $1 FOR UPDATE` (no row: `InvalidGrant`; disabled: `AccountDisabled`); `SELECT EXISTS (SELECT 1 FROM auth_refresh_token WHERE family_id = $1 AND account_id = $2 AND revoked_at IS NULL AND family_expires_at > now())` (false: `InvalidGrant`); `find_bound` on the transaction (bound elsewhere: `InvalidGrant`); `identity::insert_on(&mut tx, ..)` when unbound; `issue_on(&mut tx, account_id, None, ..)`; commit; then `session()` with the route's fence. `identity::insert` becomes a wrapper over `insert_on(conn: &mut PgConnection, account_id, &VerifiedIdentity, store_email) -> Result<Linked, RepoError>`.
- **Amendment (consolidated ruling, C12 / review B-L4):** the first statement of the transaction becomes `SELECT enabled, accesslevel, test_account FROM account WHERE account_id = $1 FOR UPDATE`. No row: `InvalidGrant`. Disabled: `AccountDisabled`. Not linkable: `NotLinkable`, using `link::linkable` on the facts it builds. Without this, an account made staff after it linked could still add a provider through `/link/start`. Sentinels unchanged (E300 range).

Files:

1. `crates/identity/src/stores.rs`: `LinkIntent` gains `family_id: String` (l.48-51).
2. `crates/identity/src/routes/account.rs`: `/link/start` (about l.125-130) fills `family_id: grant.family_id.clone()`.
3. `crates/identity/src/repo/identity.rs`: `insert_on` (today's body: lock the account `FOR UPDATE`, clear both password columns, insert) and `insert` as a wrapper (`begin; insert_on; commit`). `find_bound` gains an `_on(conn, ..)` twin if the transaction needs it; keep the pool version.
4. `crates/identity/src/routes/attach.rs` (new): `attach(state, pool, verified, intent: LinkIntent, fence, now)` with the transaction above. `session` must be reachable (`pub(super)`). `code_grant` destructures `LinkIntent { account_id, family_id }` (l.116) and calls it.
5. `crates/identity/src/routes/token.rs`: remove `attach`, add `mod`/`use` as needed (the module tree is in `routes/mod.rs`).
6. `crates/identity/src/routes/attach_tests_live_db.rs` (new).

Tests (live-DB; the existing link-route harness in `link_tests_live_db.rs` shows how to drive `/link/start`, the mock provider and `/token`; copy the setup, not its cleanup, which CT-82 replaces):

| Test | Stage revoked at | Fails on revert because |
|---|---|---|
| `live_db_add_method_after_unlink_at_intent_is_refused` | after `/link/start`, before the provider callback: revoke the family (`refresh::revoke_family`) | `/token` answers `invalid_grant` and no new identity row exists. Today `attach` checks only `enabled`. |
| `live_db_add_method_after_revoke_at_pending_is_refused` | after the provider authorize redirect, before the callback | same |
| `live_db_add_method_after_revoke_at_handoff_is_refused` | after the callback issued the handoff code, before `/token` | same |
| `live_db_add_method_with_a_live_family_attaches` | none | guards the happy path: one new identity, `signed_in` |
| `live_db_add_method_to_a_staff_account_is_refused` (amendment) | after `/link/start`, `accesslevel = 1` set by exact id, then finish | `/token` answers `not_linkable` and no identity row is added. Today `attach` checks only `enabled`. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `attach_tests_live_db`; S3 `link_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** none.

## CT-76 Unlink revokes inside its transaction (C16.6)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-76-unlink-in-tx` · **Worktree:** `cto-76` (branch at the campaign head after CT-75 merged)
- **Commit:** `fix(identity): unlink commits only with its sessions revoked (CT-76)`
- **Rules:** R1 (worktree `cto-76`), R2 (`logs/cto-76/`), R3, R7 as in CT-71. R4: `base-session/src/base/account_admin/mod.rs` stays under 500; if it would pass, move the unlink function into `account_admin/unlink.rs`. R6: sentinels `0x7000_E400..0x7000_E47F` in `cimmeria-identity`, `0x7000_E480..0x7000_E4FF` in `cimmeria-base-session`.
- **Contract excerpt (C16.6):** Both `remove_identity` functions (player `routes/account.rs`, GM `base-session/.../account_admin/mod.rs`) call `refresh::revoke_account_on(&mut tx, account_id, keep, now)` after the delete and before `commit` (player: `keep = Some(&grant.family_id)`; GM: `None`). After commit they end the in-memory sessions (player: `access.revoke_account_except` then `launch_tickets.revoke_account`; GM: `revoke::end_live_sessions`). A failed revoke rolls the unlink back: the GM reply becomes `account: could not remove identity #<id> from <target>; nothing changed, see the server log`.

Files:

1. `crates/identity/src/routes/account.rs`, `remove_identity` (l.232): call `refresh::revoke_account_on(&mut tx, account_id, Some(&grant.family_id), Utc::now())` before the commit (l.282); after commit keep `access.revoke_account_except` (l.287) and `launch_tickets.revoke_account` (l.288); remove the post-commit `refresh::revoke_account_except` (l.289).
2. `crates/base-session/src/base/account_admin/mod.rs`, the GM `remove_identity` (about l.136): the same, with `None`; remove the post-commit `refresh::revoke_account` and the partial "FAILED" reply (about l.320); a revoke error rolls back (drop the transaction) and answers the C16.6 text. Keep `end_live_sessions` after commit.
3. Tests, below, in new files `crates/identity/src/routes/account_unlink_tx_tests_live_db.rs` and `crates/base-session/src/base/account_admin/unlink_tx_tests_live_db.rs`.

Tests (live-DB):

| Test | Fails on revert because |
|---|---|
| `live_db_player_unlink_revokes_other_families_in_its_transaction` (two identities, two families F1 (caller) and F2; unlink identity 2 with F1's grant; F2 revoked, F1 live, both in the same commit: assert with a second connection that F2 is revoked as soon as the route returns) | Guards the move; on revert it still passes, so add the next test. |
| `live_db_unlink_and_attach_race_leaves_no_new_family` (connection 1 holds `SELECT .. FROM account .. FOR UPDATE` as an unlink would; spawn an Add method `/token` for the same account (CT-75 path) with a live intent family; poll `pg_locks`; then in connection 1 delete the identity, call `revoke_account_on(.., None, ..)`, commit; the `/token` answers `invalid_grant`) | Without the in-transaction revoke the intent's family is still live when attach takes the lock, and it attaches. |
| `live_db_gm_unlink_rolls_back_when_the_revoke_fails` (base-session; make `revoke_account_on` fail with a sentinel-scoped trigger on `auth_refresh_token` UPDATE, as CT-74 does on its table; the GM reply is the C16.6 text and the identity row still exists) | Today the identity is deleted and the reply says FAILED after the fact. |
| `live_db_gm_unlink_revokes_every_family` | Guards the `None` keep. |

**Checks, in order:** S1 `cimmeria-identity`; S1 `cimmeria-base-session`; S2 `cimmeria-identity`; S2 `cimmeria-base-session`; S3 `unlink_tx_tests_live_db`; S3 `account_unlink`; S2 `cimmeria-test-support`. **Docs rows:** the GM reply text is in `docs/gameplay/gm-commands.md` (or wherever `.account unlink` is documented; grep `could not remove identity` and `.account unlink` under `docs/`); update that line, CRLF.
