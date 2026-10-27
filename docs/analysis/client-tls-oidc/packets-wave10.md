# client-tls-oidc wave 10: review hardening (PR #2031 consolidated ruling)

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [ledger](README.md), [resume card](RESUME.md), [wave 6](packets-wave6.md), [wave 7](packets-wave7.md), [wave 8 and tail](packets-wave8.md), [wave 9](packets-wave9.md), [earlier review ruling](worknotes/pr2031-ruling.md)
> **Authority:** the consolidated architect ruling recorded in this file (inputs: the PR review F01-F19 and two external reviews, A and B). Where this file and an older wave file disagree, this file wins.

Wave 10 fixes five defects the external reviews found outside F01-F19 (C02, C04's fail-open, C06, C08, C15), adds the three packets the owner's calls on 2026-10-26 released (CT-94, CT-101, CT-102), and records CT-100 as Deferred. Line numbers are from `cto-camp` head `8225135ef`; re-grep every anchor before editing.

**Ships as:** every packet merges into `client-tls-oidc/campaign` (`cto-camp`) with `merge-noff.ps1`; still **one PR, #2031**, with the `full-ci` label before it merges. Retire each `cto-NN` worktree as soon as it is merged in; never run `git worktree prune`.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-95 Proxy trusted-peers parse fails closed (C04) | packet-coder | none | - |
| CT-97 Password check without oracles (C08) | packet-coder | none | - |
| CT-98 Limiter keeps account windows (C06) | packet-coder | none | - |
| CT-99 Argon2 off the async workers, two at a time (C15, D-CT28) | packet-coder | none | CT-97 merged |
| CT-96 Peek the link code, redeem it last (C02) | packet-coder | none | CT-73 and CT-82 merged |
| CT-94 Stored email authority (C01, D-CT23 = a) | packet-coder | owner decided 2026-10-26 | none (see the packet for the files it shares) |
| CT-101 Loopback-only default proxy peers (C04, D-CT26 = b) | packet-coder | owner decided 2026-10-26 | CT-95 merged |
| CT-102 Short unredeemed ticket life (C05, D-CT27 = b) | packet-coder | owner decided 2026-10-26 | CT-77 merged |
| CT-100 `main.rs` env-table row | packet-coder | Deferred | launcher-cache-presync's `main.rs` work merged into `main`, then `main` merged into `cto-camp` |

**Merge order:** CT-95, CT-97, CT-98, CT-99, CT-96 (the ruling's wave 10 order); then, in the tail, CT-94, CT-101, CT-102, before CT-92 and CT-100. Dispatch CT-95, CT-97, CT-98 and CT-94 at once (CT-94 is disjoint from them).

**After the wave:** packet loops; one wave loop over wave 10 on `cto-camp`; then **one `network-security-auth` pass over waves 6, 7 and 10 together**, after CT-96 merges (wave 10 edits the same trust boundary).

**Parallel safety.** Wave 10's files are disjoint from waves 6-9 except for two, each serialized:

- `routes/link.rs`: CT-96 opens after CT-73 merged.
- `docs/operations/auth-https.md`: CT-95 merges long before CT-92.

CT-97 and CT-99 share `password_check.rs` and are chained. CT-94 edits `repo/identity.rs` and `link.rs`/`routes/token.rs` call sites: dispatch it after CT-75 and CT-96 have merged if `merge-noff.ps1` conflicts, otherwise in parallel. No wave 10 packet touches `worker.rs`, `login_port.rs` or `crates/server/src/main.rs`.

**Sentinel ids** (each `const NAME: i32 = 0x7000_E...;`, unique across files; `crates/test-support/src/sentinel_lint.rs` fails on a duplicate):

| Packet | Range |
|---|---|
| CT-96 | `0x7000_EB00..0x7000_EB7F` |
| CT-97 | `0x7000_EB80..0x7000_EBBF` |
| CT-94 | `0x7000_EBC0..0x7000_EBFF` (assigned here from the free space the ruling names: "EBC0 and up") |
| CT-95, CT-98, CT-99, CT-101, CT-102 | none (unit tests only) |

Next free sentinel: `0x7000_EC00`. E300 is CT-75's range (wave 6) and stays unchanged. Next free ids: packets CT-103 and up, decisions D-CT31 and up.

## Rules every wave 10 packet carries

These restate work-packets.md R1-R10 for a packet-coder that does not read CLAUDE.md. Each packet below lists which of them apply and its own values.

- **R1 Shell.** PowerShell only, never Bash. Start every call with `Set-Location <wt>;`, where `<wt>` is `<repo>\.claude\worktrees\cto-NN`. Writes use absolute paths (`[IO.File]` resolves relative paths against the main checkout).
- **R2 Lane.** Every compiling cargo call goes through the lane, in the foreground: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly, and never pass `--target-dir`. Read `logs/cto-NN/` before you rerun anything. At most 5 fix rounds, then report `blocked`.
- **R3 Scope.** Edit only the files the packet names. `git add` explicit paths, and commit as soon as the checks pass. No stash, prune, rebase or push. Never touch `crates/server/src/main.rs`, `worker.rs` or `login_port.rs` (another campaign owns them).
- **R4 Caps.** 500 lines soft, 700 hard per file. Put tests in a sibling `_tests.rs` file wired with `#[cfg(test)] #[path = "..."] mod ...;`.
- **R5 Line endings.** `docs/**/*.md` is CRLF. After any docs edit, `([IO.File]::ReadAllText('<abs>') -match "[^\r]\n")` must print `False`.
- **R6 Tests.** Each fix needs a test that fails when the fix is reverted.
  - Live-DB tests have `live_db` in the name, start with `require_db_or_skip!`, connect with `test_support::database_url()`, use the packet's `0x7000_EBxx` sentinel ids, and delete by exact inserted id. Never use `LIKE` or range cleanup.
- **R7 Secrets.** Test values are `example.org`, `203.0.113.x` and names like `Player_1234`. Never log codes, tokens, tickets, subjects, emails, account names or passwords.
- **R8 Contract.** Use the names given here. A missing name is `blocked - contract gap`.
- **R9 Generated blocks.** Never edit a `<!-- gen:NAME -->` block.
- **Checks.**
  - S1 `<pkg>`: `pwsh tools/build-lane/lane.ps1 cargo fmt -p <pkg>`, then `pwsh tools/build-lane/lane.ps1 cargo clippy -p <pkg> --all-targets -- -D warnings`.
  - S2 `<pkg> [filter]`: `pwsh tools/build-lane/lane.ps1 cargo nextest run -p <pkg> [filter]`.
  - S3 `<filter>`: `pwsh tools/build-lane/live-db-test.ps1 <filter>`.
- **Report.** Write to `<wt>\.claude\reports\client-tls-oidc\<packet>-coder.md`. Return one line: `<packet> <done|blocked|failed> <sha|-> <report-path>`. Commit trailers come from work-packets.md § Attribution.

## CT-95 Proxy trusted-peers parse fails closed (C04)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-95-proxy-peers-fail-closed` · **Worktree:** `cto-95`, branched at the campaign head (opens now).
- **Commit:** `fix(server): a bad AUTH_PROXY_TRUSTED_PEERS trusts no peer (CT-95)`
- **Rules:** R1 (`cto-95`), R2 (`logs/cto-95/`), R3, R4, R5, R6 (unit tests only; no env mutation in new tests), R7, R9.
- **The defect.** `crates/server/src/config_env_auth.rs` l.49-66: a list with one bad entry keeps the built-in default (127/8, ::1, 172.16/12) and logs only a WARN. A typo meant to narrow trust therefore widens it.
- **Contract.**
  - A bad entry sets `cfg.auth_proxy_trusted_peers = Vec::new()`. `proxy_layer` (`auth/proxy.rs` l.45) then trusts no peer, so `/auth/v1` on the proxy listener answers `403 https_required`.
  - It logs `tracing::error!(reason = "bad_auth_proxy_trusted_peers", "AUTH_PROXY_TRUSTED_PEERS has an entry that is not a CIDR network; the proxy listener trusts no peer")`. No value is logged.
  - When `cfg.auth_proxy_addr` is `Some` and the variable is unset or blank, log `tracing::warn!(reason = "auth_proxy_default_peers", "AUTH_PROXY_ADDR is set but AUTH_PROXY_TRUSTED_PEERS is not; trusting only loopback (127.0.0.0/8, ::1), so a proxy on another network gets 403 https_required")` (amended by the wave 10 ruling after CT-101).

Files:

1. `crates/server/src/config_env_auth.rs`:
   - Add `fn apply_trusted_peers(cfg: &mut ServerConfig, raw: Option<&str>) -> bool`. It returns whether a non-blank value was given.
     - `None` or blank: no change.
     - Every entry parses (`PeerNet::parse`, skipping blank entries), and at least one does: set the list.
     - Otherwise: empty list plus the ERROR.
   - `apply` calls it with `std::env::var("AUTH_PROXY_TRUSTED_PEERS").ok().as_deref()`. After the `AUTH_PROXY_ADDR` block, it emits the WARN when the proxy address is set and the call returned `false`.
   - Update the fn doc (l.8-11).
   - Tests go in the existing `mod tests`; the file stays under 200 lines.
2. `docs/operations/auth-https.md` l.135: replace "one bad entry also keeps the default" with "a list with a bad entry trusts no peer and logs the `bad_auth_proxy_trusted_peers` ERROR, so `/auth/v1` on the proxy listener answers `403 https_required` until it is fixed". CRLF.

Do not edit `crates/server/src/main.rs`. Its l.19 env-table row goes stale; CT-100 fixes it later.

Tests (unit, pure, through `apply_trusted_peers` on `ServerConfig::default()`):

| Test | Fails on revert because |
|---|---|
| `trusted_peers_env_with_a_bad_entry_trusts_no_peer` (`Some("127.0.0.0/8,not-a-net")` → list empty) | Today the default list stays (non-empty). |
| `trusted_peers_env_parses_a_list` (`Some("10.0.0.1/32, ::1")` → contains 10.0.0.1 and ::1, not 172.17.0.1) | Guards the happy path. |
| `trusted_peers_env_blank_keeps_the_default` (`None` and `Some("  ")` → contains 172.17.0.1, returns `false`) | Guards unset. |

**Checks, in order:** S1 `cimmeria-server`; S2 `cimmeria-server config_env_auth`. **Docs rows:** `docs/operations/auth-https.md`.

## CT-96 Peek the link code, redeem it last (C02)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-96-link-code-peek` · **Worktree:** `cto-96`, branched at the campaign head **after CT-73 and CT-82 merged**. CT-72 and CT-73 edit `routes/link.rs`; CT-82 owns the cleanup in `link_tests_live_db.rs`.
- **Commit:** `fix(identity): a refused link keeps its link code (CT-96)`
- **Rules:**
  - R1 (`cto-96`), R2 (`logs/cto-96/`), R3, R4, R7, R9.
  - R6: sentinels `0x7000_EB00..0x7000_EB7F`.
  - Do not edit `link_tests_live_db.rs`. Use its `pub(super)` helpers as `link_tests_guards_live_db.rs` does (`harness`, `grant`, `verified`, `post_json`, `post_from`). Copy any private helper you need into the new file.
- **Contract.**
  - `LinkCodeStore::peek(&self, code: &str, now: Instant) -> Option<u32>`: the same case-insensitive, per-entry constant-time scan as `redeem`, without removing anything. An expired code is `None`.
  - Move the scan into one private helper that both `peek` and `redeem` use.
  - `link_existing` peeks in the code arm. It calls `redeem` after the linkable check and before the grant `take`. If `redeem` returns anything but `Some(account_id)`, it answers `ApiError::InvalidCredentials`.

Files:

1. `crates/identity/src/link_code.rs` (125 lines): `peek`, the shared scan helper, and two unit tests in the existing `mod tests`.
2. `crates/identity/src/routes/link.rs`:
   - The code arm (about l.173-182 at `8225135ef`; re-find it after CT-73) keeps the `LinkCodeGlobal` limit, then `peek`s. The match yields `(account_id, Option<String>)`, the code it was given.
   - After the `linkable` block (about l.207-218), before the grant `take`, redeem the code as in the contract.
   - Add `#[cfg(test)] #[path = "link_code_tests_live_db.rs"] mod link_code_tests_live_db;` next to the other test modules (about l.281-285).
3. `crates/identity/src/routes/link_code_tests_live_db.rs` (new).

Tests:

| Test | Fails on revert because |
|---|---|
| `peek_does_not_spend_the_code` (unit: issue, peek returns `Some`, redeem returns `Some`) | `peek` is new. Also guards that it does not remove. |
| `peek_of_an_expired_code_is_none` (unit: issue at t0, peek at t0 + 16 min) | Guards expiry. |
| `live_db_refused_link_code_keeps_the_code` (account `0x7000_EB00` disabled. Issue a code. `/link/existing` with the code answers 403 `account_disabled`. Set `enabled = true` by exact id. The same code with the kept grant links: `signed_in`, one identity row.) | Today the first request burns the code, so the second answers 401 `invalid_credentials`. |
| `live_db_staff_link_code_target_keeps_the_code` (account `0x7000_EB01` with `accesslevel = 1`, refused `not_linkable`; set `accesslevel = 0`, the same code links) | Same. |

Clean up the accounts, identities and events by exact id.

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity link_code`; S3 `link_code_tests_live_db`; S3 `link_tests` (the existing `live_db_link_code_links_once` and `live_db_link_code_to_disabled_account_is_account_disabled` still pass); S2 `cimmeria-test-support` (the sentinel registry). **Docs rows:** none.

## CT-97 Password check without oracles (C08)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-97-password-check-oracles` · **Worktree:** `cto-97`, branched at the campaign head (opens now).
- **Commit:** `fix(auth): link password check answers alike for every account (CT-97)`
- **Rules:**
  - R1 (`cto-97`), R2 (`logs/cto-97/`), R3, R4, R7, R9.
  - R6: sentinels `0x7000_EB80..0x7000_EBBF`, account names `ct97_player_1`.., cleanup by exact id.
  - Do **not** edit `credentials.rs`: the SOAP path keeps its answers.
- **The defect.** `crates/auth/src/auth/password_check.rs` calls `validate_credentials`, which answers `AccountDisabled` before it checks the password (`credentials.rs` l.151-153). That call also makes a legacy SHA-1 row fast, while unknown and argon2id rows are slow.
- **Contract.** `verify_account_password` keeps its signature and its `PasswordCheck` variants.
  - It reads `SELECT account_id, password, password_hash_v2, password_algo, enabled FROM account WHERE account_name = $1` into a private `#[derive(sqlx::FromRow)] struct AccountRow`.
  - It then calls a private sync `fn check_row(row: Option<&AccountRow>, plaintext: &str) -> PasswordCheck`. CT-99 moves this function into `spawn_blocking`, so it must stay sync and free of DB access.
  - Every verify goes through one private `fn argon2_verify(plaintext: &str, phc: &str) -> bool`. It increments `#[cfg(test)] pub(super) static ARGON2_VERIFIES: AtomicUsize`, then calls `verify_argon2id`.
  - `check_row` spends exactly one `argon2_verify` on every path:

    | Row | Verify against | Right when |
    |---|---|---|
    | none | `dummy_argon2id_hash()` | never (`Invalid`) |
    | argon2id with a hash | the stored hash | the verify passes |
    | argon2id with no hash | the dummy | never |
    | legacy | the dummy | `!stored.is_empty()` and `stored.to_ascii_uppercase().as_bytes().ct_eq(sha1_upper_hex(plaintext).as_bytes())` (`subtle`, already a dependency) |
    | any other algo | the dummy | never |

  - The answer is `Invalid` when the password is not right, `Disabled` when it is right but `!enabled`, else `Ok { account_id }`.
  - Never migrate the hash.

Files:

1. `crates/auth/src/auth/password_check.rs` (80 lines): as above. Drop the now-unused `validate_credentials`, `ClientCredential` and `AuthCredError` imports, update the module doc, and add `#[cfg(test)] #[path = "password_check_tests_live_db.rs"] mod tests_live_db;`.
2. `crates/auth/src/auth/password_check_tests_live_db.rs` (new). Copy the account-insert shape from `live_db_verify_account_password_keeps_legacy_rows` in `handlers_ticket_tests.rs`. Hash argon2id rows with `super::super::password_hash::hash_argon2id` (or the path the module tree gives).

Tests (live-DB, through `verify_account_password`):

| Test | Fails on revert because |
|---|---|
| `live_db_disabled_account_with_a_wrong_password_is_invalid` (argon2id row, `enabled = false`) | Today `Disabled`. |
| `live_db_legacy_row_spends_one_argon2_verify` (legacy row, wrong password; the `ARGON2_VERIFIES` delta is exactly 1) | Today 0. |
| `live_db_disabled_account_with_the_right_password_is_disabled` | Guards the disabled answer for the owner. |
| `live_db_unknown_name_spends_one_argon2_verify` (delta 1, `Invalid`) | Guards the dummy. |

nextest runs each test in its own process, so the static counter is not shared between tests.

**Checks, in order:** S1 `cimmeria-auth`; S2 `cimmeria-auth`; S3 `password_check_tests_live_db`; S3 `live_db_verify_account_password_keeps_legacy_rows`; S3 `link_password_tests_live_db`; S2 `cimmeria-test-support`. **Docs rows:** none.

## CT-98 Limiter keeps account windows (C06)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-98-limiter-eviction` · **Worktree:** `cto-98`, branched at the campaign head (opens now).
- **Commit:** `fix(identity): the limiter never evicts a live account window (CT-98)`
- **Rules:**
  - R1 (`cto-98`), R2 (`logs/cto-98/`), R3, R6 (unit tests), R7, R9.
  - R4: `limits.rs` (381 lines) stays under 450. New tests go in `limits_eviction_tests.rs`, wired with `#[cfg(test)] #[path = "limits_eviction_tests.rs"] mod eviction_tests;`.
- **Contract.** Replace `evict_oldest` (l.171-183) with `fn make_room(windows: &mut HashMap<(Bucket, String), Window>, now: Instant) -> bool`. `check` (l.97-99) calls it when the key is new and `windows.len() >= MAX_KEYS`. When it returns `false`, `check` returns `Err(RetryAfter { secs: 60 })` without inserting.

  `make_room` runs these steps:

  1. `retain` drops every stale window (`is_stale`).
  2. If the map is still at `MAX_KEYS`, it collects the windows of the IP-keyed buckets (`AnyRoute`, `Authorize`, `Token`, `LinkExistingIp`, `SignupIp`). It removes the `EVICT_BATCH = 1024` that expire soonest (`select_nth_unstable_by_key` on `start + window_len(bucket)`), or all of them if there are fewer.
  3. It returns whether `len < MAX_KEYS`.

  `LinkExistingAccount`, `LinkCodeGlobal` and `SignupGlobal` are never evicted while live. Update the `MAX_KEYS` doc (l.14-16).

Files:

1. `crates/identity/src/limits.rs`: `make_room`, `EVICT_BATCH`, the `check` change, the docs and the `#[path]` mod. The existing `limiter_eviction_spares_*` tests stay where they are and must still pass.
2. `crates/identity/src/limits_eviction_tests.rs` (new): `use super::*;`. Each test builds one `RateLimiter::with_limits(&Limits::default())`, or the constructor the existing tests use.

Tests (unit, one clock `t0 = Instant::now()`):

| Test | Fails on revert because |
|---|---|
| `limiter_at_cap_drops_every_stale_window_first` (fill `MAX_KEYS` `AnyRoute` keys at t0; one new key at t0 + 61 s; `window_count() == 1`) | Today one window is evicted and the count is `MAX_KEYS`. |
| `limiter_never_evicts_a_live_link_account_window_while_ip_windows_remain` (5 `LinkExistingAccount` "victim" checks at t0 are `Ok`; fill to `MAX_KEYS` with `LinkExistingIp` keys at t0 + 1 s; one more new IP key at t0 + 2 s; "victim" at t0 + 3 s is `Err`) | Today the victim's window expires soonest, is evicted, and the sixth guess is `Ok`. |
| `limiter_full_of_live_account_windows_refuses_a_new_key` (`MAX_KEYS` `LinkExistingAccount` keys at t0; a new one at t0 + 1 s is `Err`; `window_count() == MAX_KEYS`) | Today it evicts one and answers `Ok`. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity limits`; S2 `cimmeria-identity`. **Docs rows:** none.

## CT-99 Argon2 off the async workers, two at a time (C15, D-CT28)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-99-argon2-gate` · **Worktree:** `cto-99`, branched at the campaign head **after CT-97 merged** (same file).
- **Commit:** `fix(auth): link password checks run off the async workers, two at a time (CT-99)`
- **Rules:** R1 (`cto-99`), R2 (`logs/cto-99/`), R3, R4, R6 (unit tests, no DB), R7, R9. `tokio` is already a dependency with `full` features; do not edit `Cargo.toml`.
- **Contract.** In `crates/auth/src/auth/password_check.rs`:
  - `const ARGON2_PERMITS: usize = 2;` and `static ARGON2_GATE: LazyLock<tokio::sync::Semaphore>`.
  - `AccountRow` becomes `Send + 'static` (owned `String`s, as it already is).
  - New private `async fn gated_check(row: Option<AccountRow>, plaintext: String) -> PasswordCheck`. It runs `let _permit = ARGON2_GATE.acquire().await` (a closed semaphore is `DbError`, logged), then `tokio::task::spawn_blocking(move || check_row(row.as_ref(), &plaintext)).await`. A `JoinError` is `DbError`, logged `reason = "password_check_join"`.
  - `verify_account_password` calls `gated_check` after the query.
  - `#[cfg(test)]` statics `IN_FLIGHT` and `PEAK` (`AtomicUsize`): `argon2_verify` increments `IN_FLIGHT`, records the max in `PEAK`, verifies, then decrements.
  - Add `#[cfg(test)] #[path = "password_check_gate_tests.rs"] mod gate_tests;`.

Files:

1. `crates/auth/src/auth/password_check.rs`.
2. `crates/auth/src/auth/password_check_gate_tests.rs` (new).

Tests (unit; `gated_check(None, "pw".into())` does one real dummy verify):

| Test | Fails on revert because |
|---|---|
| `gated_check_runs_at_most_two_at_once` (`#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`; four `gated_check` calls in a `JoinSet`; `PEAK` is between 1 and 2) | Without the semaphore, `PEAK` reaches 3-4. |
| `gated_check_leaves_the_async_runtime_free` (`#[tokio::test(flavor = "current_thread")]`; a spawned ticker counts `tokio::time::sleep(1 ms)` loops while one `gated_check` runs; ticks > 0) | With `check_row` called inline on the one runtime thread, the ticker never runs: 0. |

"Revert" here means `gated_check`'s body calling `check_row` inline. The reviewer checks it that way.

**Checks, in order:** S1 `cimmeria-auth`; S2 `cimmeria-auth password_check`; S3 `password_check_tests_live_db`; S3 `link_password_tests_live_db`. **Docs rows:** none.

## Tail packets released or recorded by wave 10

### CT-94 Stored email authority (C01, D-CT23 = (a))

- **Decision:** the owner chose option (a) on 2026-10-26: refresh or clear `email` and `email_verified_at` on every sign-in, plus trusted-provider-only candidates. Options (b) (a `last_used_at` window), (c) (a confirm step) and (d) (accept) are not built.
- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-94-stored-email-authority` · **Worktree:** `cto-94`, branched at the campaign head. Merge it after wave 10 and before CT-92.
- **Commit:** `fix(identity): sign-in refreshes the stored email; only trusted providers auto-link (CT-94)`
- **Rules:** R1 (`cto-94`), R2 (`logs/cto-94/`), R3, R4, R5 (docs), R6 (live-DB; sentinels `0x7000_EBC0..0x7000_EBFF`, assigned here to avoid E300, which CT-75 uses, and the other wave 10 slices), R7, R9.
- **The defect (C01).** `repo/identity.rs` `email_candidates` (l.73-90) matches `lower(email)=$1 AND email_verified_at IS NOT NULL` across every provider and issuer. `touch` (l.182-192) updates only `last_used_at`, so a stored email is never refreshed. `link.rs` `resolve` (l.82-103) auto-links on exactly one linkable candidate. An identity whose provider later changed the email, or lost `trusted_email`, keeps its old auto-link authority.
- **Contract.**
  - On every successful sign-in of an existing identity, the repository updates `email` and `email_verified_at` from the verified identity in the same statement that sets `last_used_at`. A provider that is not `trusted_email` in the current config, or a sign-in that carries no verified email, stores both as `NULL`.
  - `email_candidates` accepts only identities whose provider is `trusted_email` in the **current** config: the caller passes the list of trusted provider ids, and the query filters on the identity's provider column against it. An empty list yields no candidates.
  - The signature changes are the minimum these two rules need. Re-find the real names and the provider column at your branch point; a missing name is `blocked - contract gap`.

Files (re-find each at the branch point):

1. `crates/identity/src/repo/identity.rs`: the `touch` change and the `email_candidates` filter.
2. The call sites that pass the verified identity and the trusted-provider list: `crates/identity/src/routes/ctx.rs` (l.86 already reads `trusted_email()` per provider) and `routes/token.rs` (about l.137). Keep edits to the call lines.
3. A new `crates/identity/src/repo/identity_email_tests_live_db.rs`, wired with `#[cfg(test)] #[path = "identity_email_tests_live_db.rs"] mod email_tests_live_db;`. Copy the setup shape from the existing identity repo live-DB tests; cleanup by exact id.
4. `docs/technical/launcher-sign-in.md`: one sentence in the account-linking paragraph (the stored email is refreshed on each sign-in; only a provider trusted now can auto-link). CRLF.

Tests (live-DB):

| Test | Fails on revert because |
|---|---|
| `live_db_stale_email_does_not_auto_link` (an identity with a stored email whose next sign-in carries another email; a new provider presenting the old email gets `needs_link`) | On revert the stale email still matches and it auto-links (ruling C01). |
| `live_db_signin_without_a_verified_email_clears_the_stored_one` (second sign-in carries none: `email` and `email_verified_at` are `NULL`) | Today `touch` leaves both. |
| `live_db_identity_of_an_untrusted_provider_is_not_a_candidate` (candidate query with a trusted list that omits the identity's provider returns none) | Today the query matches every provider. |
| `live_db_trusted_provider_identity_is_still_a_candidate` | Guards against refusing everything. |

**Checks, in order:** S1 `cimmeria-identity`; S2 `cimmeria-identity`; S3 `identity_email_tests_live_db`; S3 `link_tests`; S2 `cimmeria-test-support`. **Docs rows:** `docs/technical/launcher-sign-in.md`.

### CT-100 `main.rs` env-table row (Deferred)

After CT-95, `crates/server/src/main.rs` l.19 still says a bad list "keeps the default". launcher-cache-presync owns that file, so the one-line doc fix waits until its `main.rs` work has merged into `main` and `main` has merged into `cto-camp`. Status **Deferred**. It is a packet-coder packet:

- edit only that row;
- check S1 `cimmeria-server`;
- commit `docs(server): AUTH_PROXY_TRUSTED_PEERS row matches the fail-closed parse (CT-100)`.

If PR #2031 is ready first, the orchestrator ships it with CT-100 open and files an issue for the row. CT-101 also leaves a stale line in that table (the default list); CT-100 fixes both together.

### CT-101 Loopback-only default for the proxy peers (C04, D-CT26 = (b))

- **Decision:** the owner chose option (b) on 2026-10-26: the code default of `auth_proxy_trusted_peers` is loopback only (127/8, ::1). A fresh host fails closed instead of trusting every Docker bridge.
- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-101-loopback-default-peers` · **Worktree:** `cto-101`, branched at the campaign head **after CT-95 merged** (CT-95 writes the same `auth-https.md` line and the env parse). Merge it before CT-92.
- **Commit:** `fix(common): proxy trusted peers default to loopback only (CT-101)`
- **Rules:** R1 (`cto-101`), R2 (`logs/cto-101/`), R3, R4, R5, R6 (unit test), R7, R9. Do not edit `crates/server/src/main.rs`.
- **Contract.** `PeerNet::loopback()` (new fn next to `loopback_and_docker_bridge`, `crates/common/src/config.rs` l.237-252) returns 127/8 and ::1. The default at l.178 becomes `PeerNet::loopback()`. `compose.auth-proxy.yml` then needs `AUTH_PROXY_TRUSTED_PEERS` set to the bridge gateway (CT-52 computes it; `auth-https.md` l.63). Until it is set the proxy listener answers 403 `https_required`, and CT-95's `auth_proxy_default_peers` WARN names the cause.

Files:

1. `crates/common/src/config.rs`: `PeerNet::loopback()` and the default at l.178; update the field doc.
2. `docker/.env.example` (l.38-39): the `AUTH_PROXY_TRUSTED_PEERS` comment says the default is loopback and the bridge gateway must be set for the compose overlay.
3. `docker/compose.auth-proxy.yml` (comment at l.16): the same.
4. `docs/operations/auth-https.md` and `docs/technical/launcher-sign-in.md` (l.126): the default is loopback only. CRLF.

The `main.rs` env-table row joins CT-100.

Test (unit): `default_trusted_peers_exclude_the_docker_bridge` (the default list contains 127.0.0.1 and ::1 and does **not** contain 172.17.0.1). It fails on revert because the old default contains the 172.16/12 bridge range.

**Checks, in order:** S1 `cimmeria-common`; S2 `cimmeria-common config`; S1 `cimmeria-server`; S2 `cimmeria-server config_env_auth`. **Docs rows:** `docs/operations/auth-https.md`, `docs/technical/launcher-sign-in.md`.

### CT-102 Short unredeemed ticket life (C05, D-CT27 = (b))

- **Decision:** the owner chose option (b) on 2026-10-26: a short life until the first SOAP redemption (5 min); after it, the current 30 min idle and 12 h absolute. Options (a), (c) (IP binding) and (d) are not built. Server-only; no client patch.
- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-102-unredeemed-ticket-life` · **Worktree:** `cto-102`, branched at the campaign head **after CT-77 merged** (same file, `auth/launch_ticket.rs`). Merge it before CT-92.
- **Commit:** `fix(auth): an unredeemed launch ticket expires after five minutes (CT-102)`
- **Rules:** R1 (`cto-102`), R2 (`logs/cto-102/`), R3, R4, R5, R6 (unit tests, no sentinels), R7, R9. Do not edit `worker.rs` or `login_port.rs`.
- **The defect (C05).** `TICKET_ABSOLUTE` is 12 h (l.21) and `TICKET_IDLE` 30 min (l.24). `verify` (l.116-135) refreshes the idle clock on every SOAP use, so a thief who logs in at least every 30 min keeps a stolen ticket to 12 h. The launcher hands the ticket to the client within seconds of minting, so a short unredeemed life breaks no honest flow.
- **Contract.** `launch_ticket.rs` `Entry` gains `redeemed: bool` (false at mint). A new `TICKET_UNREDEEMED: Duration = 5 min`. `expired` uses `TICKET_UNREDEEMED` while `!redeemed`. The first successful SOAP `verify` sets `redeemed = true`, after which the existing idle and absolute limits apply. `heartbeat` does not extend an unredeemed entry. Re-read `verify` as CT-77 left it (TicketGuard; every candidate checked) and keep that behavior.

Files:

1. `crates/auth/src/auth/launch_ticket.rs` (and its sibling `_tests.rs` file if CT-77 split one): the field, the constant, `expired`, `verify`, `heartbeat`. Update the module doc.
2. `docs/technical/launcher-sign-in.md`: the revocation-model section's ticket lifetime sentence (5 min until redeemed). CRLF.

Tests (unit, injected clock):

| Test | Fails on revert because |
|---|---|
| `an_unredeemed_ticket_expires_after_five_minutes` (mint; at t0 + 5 min + 1 s `verify` is not `Valid`) | Today it stays `Valid` up to the 30 min idle limit. |
| `a_redeemed_ticket_keeps_the_idle_and_absolute_limits` (redeem at t0 + 1 min; still `Valid` at t0 + 20 min; expired past 30 min idle) | Guards the post-redemption life (Change Server re-sends the credential within the life). |
| `heartbeat_does_not_extend_an_unredeemed_ticket` | The unredeemed clock must not be extendable. |

**Checks, in order:** S1 `cimmeria-auth`; S2 `cimmeria-auth launch_ticket`. **Docs rows:** `docs/technical/launcher-sign-in.md`.

## Amendments to existing packets

These went into the packet files before dispatch, per the consolidated ruling § 3 "Amendments". The full text is in [wave 6](packets-wave6.md) (CT-75), [wave 8](packets-wave8.md) (CT-84, CT-92) and is summarized here so a reader of this wave sees them.

- **CT-75** (C12, B-L4): `attach` also checks `link::linkable` in its transaction, and gains `live_db_add_method_to_a_staff_account_is_refused`. Sentinels unchanged (E300 range).
- **CT-84** (C09, A-L3; D-CT24 amended): `validate_nbf = true` with the 60 s leeway, and two tests (`a_token_with_nbf_five_minutes_ahead_is_refused`, `a_token_with_nbf_thirty_seconds_ahead_is_accepted`).
- **CT-92** (documentation-writer): documents C03, C07, C10, C11, C14, C01, C05 and the C04 residual in `docs/technical/launcher-sign-in.md`; adds the forged `X-Forwarded-*` warning to `docs/operations/auth-https.md`; notes that the SOAP disabled answer (error 5) is legacy parity (C08).

## Owner calls still open

- **D-CT29** (C10, OIDC and the provider-age gate): stays open and joins the CT-53 item. Options: (a) document only; (b) when the gate is configured (`min days > 0`), refuse sign-up from a provider that reports no creation time unless it is marked exempt in config; (c) drop the gate. Recommendation: (b), with each CT-53 provider marked exempt or not when it is registered. CT-92 documents the gap until then.
- **D-CT28** is decided by the architect (CT-99), not blocked.
