# client-tls-oidc wave 7: game-login handoffs, account lookup, guards, test cleanup (PR #2031 review)

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract C16.4](contract.md#c164-memory-launch-ticket-guards-ct-77-ct-78-ct-79), [wave 6](packets-wave6.md), [wave 8](packets-wave8.md), [wave 9](packets-wave9.md), [ledger](README.md), [review ruling](worknotes/pr2031-ruling.md)

Wave 7 carries the revocation model into the game-login handoffs (F05, contract C16.4), fixes the two folded-name lookups (F13), closes the database-guard holes (F14) and makes the live-DB cleanup own only its rows (F19).

**Ships as:** every packet merges into `client-tls-oidc/campaign` (`cto-camp`) with `merge-noff.ps1`; still **one PR, #2031**. Wave 7 opens when wave 6's last packet (CT-76) has merged into the campaign; CT-77 needs CT-72's `grant_live`, and CT-82 rewrites cleanup that CT-75 copied. Waves 8 and 9 run in parallel.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-77 Ticket guards and verify-all (F05, F13b) | packet-coder | none | wave 6 merged |
| CT-80 One account per name, or none (F13a) | packet-coder | none | wave 6 merged |
| CT-81 Guard triggers lock the account (F14) | packet-coder | none | wave 6 merged |
| CT-82 Live-DB cleanup deletes only what it inserted (F19) | packet-coder | none | wave 6 merged |
| CT-78 SOAP handoffs carry the guard (F05) | packet-coder | none | CT-77 merged |
| CT-79 BaseApp admission checks the guard (F05) | packet-coder | none | CT-78 merged |

**Merge order:** CT-77, CT-80, CT-81, CT-82, CT-78, CT-79. Dispatch CT-77, CT-80, CT-81 and CT-82 at once (no shared file: CT-77 edits `cimmeria-auth` `launch_ticket.rs` and identity `routes/ticket.rs` / `revoke.rs`; CT-80 `repo/account.rs` and `account_admin/mod.rs`; CT-81 `db/sgw/` and a new repo test file; CT-82 three route test files). CT-78 branches at the campaign head after CT-77 merged; CT-79 after CT-78.

**After the wave:** the wave review loop on `cto-camp`, then **one** read-only `network-security-auth` pass over waves 6 and 7 together on the campaign head (brief: "review the revocation model of contract C16 against findings F02-F05 in `docs/analysis/client-tls-oidc/worknotes/pr2031-ruling.md`; numbered fix notes only; verdict ship, fix or decision").

**Sentinel ids** (`const NAME: u32 = 0x7000_E...;` or `i32`, unique across files): CT-77 `0x7000_EA00..0x7000_EAFF`, CT-80 `0x7000_E500..0x7000_E5FF`, CT-81 `0x7000_E600..0x7000_E6FF`, CT-82 token `0x7000_E700..0x7000_E7FF`, account `0x7000_E800..0x7000_E8FF`, link `0x7000_E900..0x7000_E9EF`, near-match rows `0x7000_E9F0..0x7000_E9FF`.

## CT-77 Ticket guards and verify-all (F05, F13b)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-77-ticket-guard` · **Worktree:** `cto-77`
- **Commit:** `fix(auth): a revoked account's launch ticket guards fail, and verify checks every same-name ticket (CT-77)`
- **Rules:**
  - R1: PowerShell only, never the Bash tool; start every command with `Set-Location <repo>\.claude\worktrees\cto-77;`; absolute paths for every file write.
  - R2: every cargo call through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground; never cargo directly; never `--target-dir`; read the lane log under `logs/cto-77/` before rerunning; at most 5 fix rounds per check.
  - R3: edit only the files below; `git add` explicit paths; commit at once with the subject above and the Haiku attribution lines from work-packets.md; no `git stash`, rebase or push.
  - R4: `launch_ticket.rs` is 361 lines, 150 of them inline tests: move `mod tests` (l.210-361) to a new `crates/auth/src/auth/launch_ticket_tests.rs` with `#[cfg(test)] #[path = "launch_ticket_tests.rs"] mod tests;`, so the store stays under 400. `routes/ticket.rs` (231) stays under 300.
  - R6: unit tests in the sibling tests file; the live-DB test has `live_db` in its name, starts with `require_db_or_skip!`, uses sentinels `0x7000_EA00..0x7000_EAFF` and deletes exactly its ids.
  - R7: test values `Player_1234`, `example.org`; never log a ticket.
  - R8: names come from the contract excerpt; a gap is `blocked - contract gap: <what>`.
- **Contract excerpt (C16.4):**

  ```rust
  // crates/auth/src/auth/launch_ticket.rs, re-exported from crates/auth/src/auth/mod.rs
  #[derive(Clone, Debug)]
  pub struct TicketGuard(Arc<AtomicBool>);
  impl TicketGuard { pub fn is_current(&self) -> bool; }
  pub enum TicketCheck { NoMatch, Valid { account_id: u32, guard: TicketGuard } }   // drops Copy, PartialEq, Eq
  impl LaunchTicketStore {
      pub fn guard(&self, account_id: u32) -> TicketGuard;        // a fresh guard registered under the account
      pub fn mint(&self, account_id: u32, name: &str, launch_id: &str, now: Instant) -> MintedTicket;   // unchanged; takes its own guard
      pub fn mint_guarded(&self, account_id: u32, name: &str, launch_id: &str, now: Instant, guard: &TicketGuard)
          -> Option<MintedTicket>;                                 // None when !guard.is_current()
  }
  ```

  The store keeps `guards: HashMap<u32, Vec<Weak<AtomicBool>>>` beside the entries. `revoke_account(id)` removes the entry, stores `false` into every guard of the account and drops the list. `end` and expiry do **not** flip guards. `reap` drops dead `Weak`s. `verify` compares the digest against **every** entry whose folded name matches (constant-time each) and answers the one that matches, or `NoMatch`; a matching entry whose guard is not current is `NoMatch` and is removed. Order in `revoke::end_in`: access tokens first, then launch tickets. The ticket route, after the Bearer check: `let guard = launch_tickets.guard(account_id);` then `if !access.grant_live(&grant) { InvalidToken }`, then the DB reads, then `mint_guarded(.., &guard)`, `None` answering `InvalidToken`.

Files:

1. `crates/auth/src/auth/launch_ticket.rs`:
   - One lock over both maps: `struct Inner { entries: HashMap<u32, Entry>, guards: HashMap<u32, Vec<Weak<AtomicBool>>> }`, `LaunchTicketStore { inner: Mutex<Inner> }` (keep `#[derive(Default)]`, the poison-tolerant `lock()` and the `Debug` impl).
   - `Entry` gains `guard: TicketGuard`. `TicketGuard::is_current` is `self.0.load(Ordering::SeqCst)`.
   - A private `fn register(inner: &mut Inner, account_id: u32) -> TicketGuard` (new `Arc<AtomicBool::new(true)>`, push its `Weak`). `guard()` locks and calls it. `mint` locks, registers, inserts. `mint_guarded` locks, returns `None` when `!guard.is_current()`, else inserts an entry holding `guard.clone()`. All three build the entry the same way (one private helper).
   - `verify`: collect every account id whose `name_lower` matches; remove the expired ones; among the rest, the entry whose `hash` matches by `ct_eq` (compare all of them, do not stop early). A match with a stale guard is removed and answers `NoMatch`; a live match sets `last_used` and answers `Valid { account_id, guard: entry.guard.clone() }`.
   - `revoke_account`: remove the entry; `if let Some(list) = guards.remove(&id)` store `false` into every upgradable `Weak`.
   - `reap`: as today for entries, then `guards.retain(|_, list| { list.retain(|w| w.strong_count() > 0); !list.is_empty() })`.
   - Doc comment on `revoke_account`: it is the only path that flips guards, and why (a handoff already issued must die with the account's sessions; an `end` from the launcher must not kill a running Change Server).
2. `crates/auth/src/auth/launch_ticket_tests.rs` (new): today's tests moved. Equality asserts on `TicketCheck` no longer compile: use `assert!(matches!(store.verify(..), TicketCheck::Valid { account_id: 1, .. }))` and `matches!(.., TicketCheck::NoMatch)`.
3. `crates/auth/src/auth/mod.rs` l.50: `pub use launch_ticket::{LaunchTicketStore, MintedTicket, TicketCheck, TicketGuard};`.
4. `crates/auth/src/auth/handlers.rs` l.181 and l.194: the patterns become `TicketCheck::Valid { account_id, .. }`. Nothing else in this file (CT-78 uses the guard).
5. `crates/identity/src/routes/ticket.rs` `mint_ticket` (l.83-115): after `let account_id = grant.account_id;` take `let guard = state.deps.launch_tickets.guard(account_id);`, then `if !state.ctx.stores.access.grant_live(grant) { return Err(ApiError::InvalidToken); }`, then today's DB reads, then `mint_guarded(account_id, &name, &launch_id, now, &guard).ok_or(ApiError::InvalidToken)?`. A comment states the order argument from the excerpt.
6. `crates/identity/src/revoke.rs` `end_in` (l.42-45): the order is already access then tickets; add a comment saying it is load-bearing (C16.4) so nobody swaps it.
7. Every other `TicketCheck` equality in `crates/` (re-grep `TicketCheck::` and `launch_tickets.verify`; CT-74's `token_reuse_tests_live_db.rs` compares with `NoMatch`): convert to `matches!`.
8. `crates/identity/src/routes/ticket_guard_tests_live_db.rs` (new), wired from `ticket.rs` with `#[cfg(test)] #[path = "ticket_guard_tests_live_db.rs"] mod guard_tests_live_db;` so it can call the private `mint_ticket`.

Tests:

| Test | Type | Fails on revert because |
|---|---|---|
| `verify_finds_the_ticket_among_two_same_name_accounts` (mint for 1 `Alice` and 2 `ALICE`; each one's SHA-1 verifies `Valid` with its own id; then mint in the order 2, 1 on a fresh store and repeat) | unit | `.find` returns the same first entry for both names, so one of the two verifies is `NoMatch` whatever the hash order. |
| `revoke_account_flips_every_guard_of_the_account` (two `guard(1)` plus a minted ticket's guard from `verify`; `revoke_account(1)`; all three `!is_current()`; a guard of account 2 stays current) | unit | No guards today. |
| `end_and_expiry_leave_the_guard_current` | unit | Guards against flipping on `end`. |
| `mint_guarded_with_a_revoked_guard_mints_nothing` (`guard(1)`, `revoke_account(1)`, `mint_guarded` is `None`, `live_count() == 0`) | unit | The race the route closes. |
| `verify_refuses_and_removes_a_ticket_whose_guard_was_flipped` (build the store state directly in the test module: insert an `Entry` whose guard's `AtomicBool` the test has set to `false`, then `verify` with its SHA-1 is `NoMatch` and `live_count() == 0`) | unit | Guards the stale-guard branch; without it a flipped entry still verifies. |
| `reap_drops_dead_guards` | unit | Guards the memory bound. |
| `live_db_ticket_route_refuses_a_grant_revoked_before_the_mint` (insert account `0x7000_EA00` `Player_1234`; `stores.access.mint(..)` an access token, look its grant up, then `stores.access.revoke_account(id)`; call `mint_ticket(&state, "203.0.113.5", &grant, body)` directly) | live-DB | Answers `InvalidToken` and `launch_tickets.live_count() == 0`. Today `mint_ticket` never rechecks the grant and mints. |

  Build the route state the way `routes/token.rs`'s `span_tests::test_state` does, with `deps.db = Some(pool)`; if that helper cannot take a pool, copy the `harness` setup of `token_tests_live_db.rs` (setup only; CT-82 owns its cleanup).

**Checks, in order:** S1 `cimmeria-auth`; S1 `cimmeria-identity`; S1 `cimmeria-base` (it matches on `TicketCheck` nowhere today, but compiles `cimmeria-auth`); S2 `cimmeria-auth`; S2 `cimmeria-identity`; S3 `ticket_guard_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** none (CT-92).

## CT-80 One account per name, or none (F13a)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-80-name-match` · **Worktree:** `cto-80`
- **Commit:** `fix(identity): GM account commands refuse a name two accounts share without case (CT-80)`
- **Rules:** R1 (worktree `cto-80`), R2 (`logs/cto-80/`), R3, R7, R8 as in CT-77. R4: `repo/account.rs` (111) and `account_admin/mod.rs` (408, plus CT-76's change) stay under 500. R5: docs are CRLF. R6: live-DB tests, sentinels `0x7000_E500..0x7000_E5FF`, exact-id cleanup.
- **Why:** legacy password login matches the exact name; the `account` table has no unique index on `lower(account_name)`. `facts_by_name` picks the oldest folded match, so `.account linkcode ALICE` can hand recovery authority for `Alice` while echoing `ALICE`; linking with that code clears `Alice`'s password.
- **Ruling (F13a):** an exact match wins; with no exact match a single folded match is used; any tie is refused.

Files:

1. `crates/identity/src/repo/account.rs`: replace `facts_by_name` with

   ```rust
   pub enum NameMatch { None, One(AccountFacts), Ambiguous }
   /// Exact name first (`account_name = $1`): one row is `One`, several are `Ambiguous`.
   /// No exact row: folded (`lower(account_name) = lower($1)`): none `None`, one `One`, several `Ambiguous`.
   pub async fn match_name(pool: &PgPool, name: &str) -> Result<NameMatch, RepoError>;
   ```

   One query is enough: `SELECT account_id, enabled, accesslevel, test_account, account_name = $1 AS exact FROM account WHERE lower(account_name) = lower($1) ORDER BY account_id LIMIT 3`, then decide in Rust (count the exact rows first). Update the doc comment.
2. `crates/base-session/src/base/account_admin/mod.rs` `answer` (l.243): `match account::match_name(pool, target).await?` — `None` keeps today's `unknown_account` reply; `Ambiguous` logs `log_outcome(caller, name, false, Some("ambiguous_account"), target, None, None)` and replies `account: more than one account is named {target} when case is ignored; type the name exactly as it is stored`; `One(facts)` continues as today.
3. `crates/identity/src/repo/tests_live_db.rs:404`: the one test caller; switch to `match_name` and `matches!(.., NameMatch::One(_))` or `None` as the test meant.
4. `crates/identity/src/repo/account_name_tests_live_db.rs` (new), `#[path]` from `account.rs`.
5. Docs: the GM `.account` reference (grep `.account linkcode` under `docs/`; expected `docs/gameplay/gm-commands.md` or `docs/technical/launcher-sign-in.md`): one line on the ambiguous refusal. CRLF.

Tests (live-DB, names `CT80Alice` / `ct80alice` / `CT80ALICE` on sentinel ids):

| Test | Fails on revert because |
|---|---|
| `live_db_exact_name_wins_over_an_older_case_variant` (ids E500 `ct80alice`, E501 `CT80Alice`; `match_name("CT80Alice")` is `One` with E501) | Today the oldest folded match, E500, wins. |
| `live_db_two_case_variants_without_an_exact_match_are_ambiguous` (`match_name("CT80ALICE")`) | Today it answers E500. |
| `live_db_single_folded_match_is_used` (only E502 `CT80Bob`; `match_name("ct80bob")` is `One`) | Guards the GM's usual case-insensitive typing. |
| `live_db_gm_linkcode_refuses_an_ambiguous_name` (base-session, through the `run`/`answer` path the existing `account_admin/tests_live_db.rs` drives; ids E580, E581; the reply is the ambiguous text and no link code is issued) | Today a code is issued for the older account. |

  Put the base-session test in a new `crates/base-session/src/base/account_admin/name_tests_live_db.rs` (ids `0x7000_E580..0x7000_E5FF`).

**Checks, in order:** S1 `cimmeria-identity`; S1 `cimmeria-base-session`; S2 `cimmeria-identity`; S2 `cimmeria-base-session`; S3 `account_name_tests_live_db`; S3 `name_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** the GM command reference (item 5).

## CT-81 Guard triggers lock the account (F14)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-81-guard-lock` · **Worktree:** `cto-81`
- **Commit:** `fix(db): the identity guard locks the account and covers reassignment (CT-81)`
- **Rules:** R1 (worktree `cto-81`), R2 (`logs/cto-81/`), R3, R7 as in CT-77. R5: `docs/**/*.md` CRLF; keep each `db/sgw/*.sql` file's current line endings (check with `([IO.File]::ReadAllText('<abs>') -match "\r\n")` before and after). R6: live-DB tests, sentinels `0x7000_E600..0x7000_E6FF`, exact-id cleanup. Seeds are the source of truth: edit `db/sgw/` only, never add a `db/scripts/` migration. `live-db-test.ps1` reloads this worktree's database from `db/sgw/`, so the new function is in place when the tests run.
- **The defect (reproduced, database-evidence.md § F14):** `account_identity_password_guard` locks the account only when it already has a password (`EXISTS (.. AND password IS NOT NULL ..) FOR SHARE` locks no row otherwise), so an identity insert and a password `UPDATE` racing on a passwordless account both commit. The trigger fires `BEFORE INSERT` only, so `UPDATE account_identity SET account_id = <passworded account>` bypasses it.
- **Ruling:** lock the target account row `FOR UPDATE` whatever its password, then test it. The password `UPDATE` holds the same row lock, so the two writers serialize; the plpgsql statement after the lock reads a fresh snapshot in READ COMMITTED (and fails with a serialization error under REPEATABLE READ, which also refuses). Lock order is account then identity in both paths.

Files:

1. `db/sgw/_functions.sql` `account_identity_password_guard` (l.290-300) body:

   ```sql
   DECLARE
       has_pw boolean;
   BEGIN
       SELECT (password IS NOT NULL OR password_hash_v2 IS NOT NULL) INTO has_pw
         FROM account WHERE account_id = NEW.account_id FOR UPDATE;
       IF has_pw THEN
           RAISE EXCEPTION 'account_has_password' USING ERRCODE = 'check_violation';
       END IF;
       RETURN NEW;
   END;
   ```

   (No row: `has_pw` is null and the FK refuses the insert afterwards, as today.) Update the comment above it: the lock is unconditional so a concurrent password write waits.
2. `db/sgw/_triggers.sql` l.50-51: `BEFORE INSERT OR UPDATE OF account_id ON account_identity`.
3. `docs/technical/launcher-sign-in.md:77`: keep the claim and add how: the identity guard locks the account row, so concurrent writers serialize, and it also covers moving an identity to another account. CRLF.
4. `crates/identity/src/repo/guard_tests_live_db.rs` (new), `#[path]` from `crates/identity/src/repo/mod.rs` (or wherever `tests_live_db` is wired). The existing `crates/auth/src/auth/identity_schema_live_db.rs` keeps its single-connection tests unchanged.

Tests (live-DB; each uses two pool connections and `pg_locks` polling up to 10 s for a not-granted lock, then commits the holder):

| Test | Fails on revert because |
|---|---|
| `live_db_password_set_waits_for_an_uncommitted_identity_insert` (E600 passwordless; conn 1 `BEGIN` + insert an identity for E600, left open; spawn conn 2 `UPDATE account SET password = '<40 hex>' WHERE account_id = E600`; wait for its lock; commit conn 1; conn 2 fails with SQLSTATE `23514`, message `account_has_identity`) | Today conn 2 does not wait: both commit and E600 holds both (evidence: `concurrent result: 1\|t`). |
| `live_db_identity_insert_waits_for_an_uncommitted_password_set` (reverse order; the insert fails `account_has_password`) | Today the insert's `FOR SHARE` locks no row of a passwordless account, so it does not wait and both commit. |
| `live_db_moving_an_identity_to_a_passworded_account_is_refused` (E601 passwordless with an identity, E602 with a password; `UPDATE account_identity SET account_id = E602 WHERE ..` fails `23514`) | Today the trigger is `BEFORE INSERT` only (evidence: `identity UPDATE bypass: 2\|t`). |
| `live_db_moving_an_identity_to_a_passwordless_account_succeeds` | Guards against refusing every move. |

  Cleanup: delete the identities, then the accounts, by exact sentinel id; always in a guard that also runs after a failed assertion (the pattern in `refresh_tests.rs`).

**Checks, in order:** S1 `cimmeria-identity`; S3 `guard_tests_live_db`; S3 `identity_schema_live_db`; S3 `repo::`; S2 `cimmeria-test-support`. **Docs rows:** `docs/technical/launcher-sign-in.md` (item 3).

## CT-82 Live-DB cleanup deletes only what it inserted (F19)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-82-exact-cleanup` · **Worktree:** `cto-82`
- **Commit:** `test(identity): route live-DB tests delete only their own accounts (CT-82)`
- **Rules:** R1 (worktree `cto-82`), R2 (`logs/cto-82/`), R3, R7 as in CT-77. R4: `link_tests_live_db.rs` (598) and `account_tests_live_db.rs` (592) must not grow past 650; put the new near-match test in a new file. R6: sentinels below; the type may stay `u32` (the lint accepts `u32` and `i32`).
- **The defect:** `clean` deletes id ranges (`BETWEEN FIRST_ID AND LAST_ID`), every `CT26Player%` / `CT30-%` name, and every account holding a `ct26-%` subject of any provider. On a developer database that deletes accounts the tests never made, with their characters (cascade). Ids around 2,000,000 also sit outside the `0x7000_xxxx` fixture convention.

Files:

1. `crates/identity/src/routes/token_tests_live_db.rs`: every `2_000_5xx` const becomes `0x7000_E7xx` (keep the names; re-grep the file for any literal id in the old range). `clean` deletes `WHERE account_id = ANY($1)` over a `const OWNED: &[u32]` listing every id const, bound as `Vec<i32>`. Drop `FIRST_ID` / `LAST_ID`.
2. `crates/identity/src/routes/account_tests_live_db.rs`: the same with `0x7000_E8xx`. Accounts the tests create through sign-up (named `CT30-...`): collect them by the exact identity the test signed up with, `DELETE FROM account WHERE account_id IN (SELECT account_id FROM account_identity WHERE provider = $1 AND subject = ANY($2))` with `provider = 'dev'` and a `const SIGNUP_SUBJECTS: &[&str]` that lists every subject literal the file uses for a sign-up (grep `subject`). If a sign-up subject is built at run time, make it a const.
3. `crates/identity/src/routes/link_tests_live_db.rs`: the same with `0x7000_E9xx` (`0x7000_E900..0x7000_E9EF`) and the `CT26Player` sign-ups collected by exact `(provider, subject)`; `clean` stays `pub(super) async fn clean(pool: &PgPool)` so its callers (`link_password_tests_live_db.rs`, `link_tests_guards_live_db.rs`, `account_unlink_tests_live_db.rs`, and CT-75's `attach_tests_live_db.rs`) keep compiling; give those files' own ids and subjects to `OWNED` / `SIGNUP_SUBJECTS` if they insert any (grep them). The count at l.399 (`account_name LIKE 'CT26Player%'`) becomes a count over the same exact `(provider, subject)` set.
4. `crates/identity/src/routes/cleanup_ownership_tests_live_db.rs` (new), wired from `routes/mod.rs` beside the others.

Test:

| Test | Fails on revert because |
|---|---|
| `live_db_cleanup_keeps_near_match_rows` (insert `0x7000_E9F0` named `CT26PlayerKeep`, `0x7000_E9F1` named `CT30-keep`, and `0x7000_E9F2` holding an identity with provider `other` and subject `ct26-keep`; run each file's `clean` (make them `pub(super)` where they are private); all three rows still exist; then delete them by exact id) | Today's `LIKE` and range predicates delete all three. |

**Checks, in order:** S1 `cimmeria-identity`; S3 `cleanup_ownership_tests_live_db`; S3 `token_tests_live_db`; S3 `account_tests_live_db`; S3 `link_`; S3 `account_unlink`; S2 `cimmeria-test-support`. **Docs rows:** none.

## CT-78 SOAP handoffs carry the guard (F05)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-78-soap-guard` · **Worktree:** `cto-78` (branch at the campaign head after CT-77 merged)
- **Commit:** `fix(auth): a SID or BaseApp ticket from a revoked launch ticket is refused (CT-78)`
- **Rules:** R1 (worktree `cto-78`), R2 (`logs/cto-78/`), R3, R7 as in CT-77. R4: `handlers.rs` is 609 lines: keep the growth under 40 lines; if it would pass 650, move `handle_server_selection` into a new `handlers_select.rs` unchanged apart from the guard. `crates/base/src/base/login/tests/mod.rs` is 765 lines (over the cap): it must shrink, never grow (item 6). R6: unit tests.
- **Contract excerpt (C16.4, handoffs):** `SessionRecord.ticket_guard: Option<TicketGuard>` and `PendingLogin.ticket_guard: Option<TicketGuard>` (`None` for password logins). Checked: in UserAuth after the `ticket_account` await, before the SID insert (refuse as `InvalidCredentials`, `reason = "ticket_revoked"`); in ServerSelection after the SID is removed (as `sid_unknown`, `reason = "ticket_revoked"`); at BaseApp admission after the age check (CT-79).

Files:

1. `crates/auth/src/auth/mod.rs`: `PendingLogin` (l.141) and `SessionRecord` (l.176) gain `pub ticket_guard: Option<TicketGuard>` / `ticket_guard: Option<TicketGuard>` with a doc line (`Some` only for a launch-ticket login; checked at each handoff).
2. `crates/auth/src/auth/handlers.rs`, UserAuth:
   - l.180-196: keep the guard: `TicketCheck::Valid { account_id, guard } => Some((account_id, guard))`; the TLS-refused arm ignores it (`..`). `ticket_id` becomes `Option<(u32, TicketGuard)>`; adjust its uses (l.204-263 use the id; l.363 `ticket_id.is_some()`).
   - Right after the `ticket_account(..).instrument(..).await` (l.216), call `crate::auth::test_hooks::after_ticket_lookup();` (a no-op outside tests).
   - Immediately before the SID insert (l.338), after every await of the handler: if the guard is `Some` and `!is_current()`: `tracing::info!(account_id, account_name = %req.account_name, reason = "ticket_revoked", "Launch ticket was revoked during login; refused")`, `audit!(LoginOutcome::InvalidCredentials)`, `return login_error(4, "The account name or password is incorrect.")`.
   - The `SessionRecord` literal gets `ticket_guard: ticket_id.as_ref().map(|(_, g)| g.clone())`.
3. `crates/auth/src/auth/handlers.rs`, ServerSelection: after the `session` match (l.441): if `session.ticket_guard` is `Some` and not current, `tracing::info!(reason = "ticket_revoked", account_id = session.account_id, peer_ip = %client_ip, "Phase 2 refused: the launch ticket behind this SID was revoked")` and `return select_error(15, "Your logon session has expired. Please log in again.")`. The `PendingLogin` literal (l.509) gets `ticket_guard: session.ticket_guard.clone()`.
4. `crates/auth/src/auth/test_hooks.rs` (new, registered in `auth/mod.rs` as `pub(crate) mod test_hooks;`):

   ```rust
   #[cfg(test)]
   thread_local! { static AFTER_TICKET_LOOKUP: RefCell<Option<Box<dyn FnOnce()>>> = RefCell::new(None); }
   #[cfg(test)]
   pub(crate) fn set_after_ticket_lookup(f: impl FnOnce() + 'static);
   pub(crate) fn after_ticket_lookup();   // #[cfg(test)]: take and run the hook; else an empty #[inline] fn
   ```

   The tests run on the current-thread `#[tokio::test]` runtime, so the hook runs on the test's thread.
5. Every `SessionRecord { .. }` / `PendingLogin { .. }` literal gains `ticket_guard: None`: `crates/auth/src/auth/handlers_tests.rs:291` and `:342`, `crates/auth/src/auth/service_tests.rs:24` and `:42`, `crates/base/src/base/session_wire_proof_tests/service.rs:13`, `crates/base/src/base/connect_loop/relaunch_tests.rs:69`, and `make_pending_login` (item 6). Re-grep `PendingLogin {` and `SessionRecord {` across `crates/` and `tools/`.
6. `crates/base/src/base/login/tests/mod.rs`: move `make_pending_login` (l.205-219) into a new `crates/base/src/base/login/tests/fixtures.rs` (`pub(super) fn make_pending_login`, with `ticket_guard: None`), declare `mod fixtures;` and `use fixtures::make_pending_login;` in `tests/mod.rs`. The file shrinks by about 12 lines.
7. `crates/auth/src/auth/handlers_guard_tests.rs` (new), `#[cfg(test)] #[path]` from `handlers.rs`; reuse the request builders of `handlers_tests.rs` (make the needed helpers `pub(super)`; do not copy them).

Tests (unit, developer mode with no database, so `ticket_account` answers without SQL; follow how `handlers_tests.rs` posts a ticket login over TLS):

| Test | Fails on revert because |
|---|---|
| `userauth_refuses_when_the_ticket_is_revoked_during_the_lookup` (mint a ticket; set the hook to `launch_tickets.revoke_account(id)`; post UserAuth with the ticket's SHA-1 over TLS; the answer is error 4 and `state.sessions` is empty) | Without the check the SID is inserted with a live session. |
| `server_selection_refuses_a_sid_whose_ticket_was_revoked` (a successful ticket UserAuth, then `revoke_account`, then ServerSelection with that SID: error 15, no `pending_logins` entry) | Today the SID becomes a BaseApp ticket. |
| `server_selection_with_a_live_ticket_guard_still_succeeds` | Guards against refusing every ticket login. |
| `password_login_sessions_carry_no_guard` (a password-style login in dev mode; the stored `SessionRecord.ticket_guard` is `None`) | Guards the `None` default. |

**Checks, in order:** S1 `cimmeria-auth`; S1 `cimmeria-base`; S2 `cimmeria-auth`; S2 `cimmeria-base`; S2 `cimmeria-test-support`. **Docs rows:** none (CT-92 documents the handoff checks).

## CT-79 BaseApp admission checks the guard (F05)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-79-base-guard` · **Worktree:** `cto-79` (branch at the campaign head after CT-78 merged)
- **Commit:** `fix(base): a BaseApp ticket from a revoked launch ticket is burned (CT-79)`
- **Rules:** R1 (worktree `cto-79`), R2 (`logs/cto-79/`), R3, R7 as in CT-77. R4: `crates/base/src/base/login/mod.rs` is 609 lines; add at most 15. `tests/mod.rs` gets one `mod` line only (CT-78 shrank it). R6: unit test.
- **Contract excerpt (C16.4):** at BaseApp admission in `handle_login_with_issuer`, after the age check: burn the pending login, WARN `reason = "ticket_revoked"`, the same refusal as an expired ticket.

Files:

1. `crates/base/src/base/login/mod.rs`, after the age check (l.111-123), before `relaunch::address_claim`:

   ```rust
   if peeked.ticket_guard.as_ref().is_some_and(|g| !g.is_current()) {
       burn_ticket(pending_logins, ticket)?;
       tracing::warn!(%addr, account_id = peeked.account_id, account_name = %peeked.account_name,
           reason = "ticket_revoked", "Phase 3 ticket comes from a revoked launch ticket; refusing the login");
       return Ok(());
   }
   ```

   A doc line says an established connection is not touched (the accepted exception); only new admissions are refused.
2. `crates/base/src/base/login/tests/ticket_revoked.rs` (new), modelled on `tests/ticket_age.rs` (same capture of the WARN row), plus `mod ticket_revoked;` in `tests/mod.rs`.

Tests (unit):

| Test | Fails on revert because |
|---|---|
| `a_pending_login_from_a_revoked_launch_ticket_is_burned` (`make_pending_login` with `ticket_guard: Some(store.guard(id))`; `store.revoke_account(id)`; `handle_login_with_issuer`; the pending entry is gone, no client is connected, and the WARN row has `reason = "ticket_revoked"`) | Today admission ignores revocation and connects the client. |
| `a_pending_login_with_a_live_guard_is_admitted` | Guards against refusing every ticket login. |

**Checks, in order:** S1 `cimmeria-base`; S2 `cimmeria-base`; S2 `cimmeria-test-support`. **Docs rows:** none.
