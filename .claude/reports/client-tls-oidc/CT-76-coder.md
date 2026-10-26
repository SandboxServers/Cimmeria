# CT-76 coder report

Status: done for CT-76 packet work. Part 1 0762f8fc5, part 2 7e04c130b, worknote and report 9951f7d67.

## Committed

- Part 1, 0762f8fc5: production edits.
  - crates/identity/src/routes/account.rs: player `remove_identity` runs `refresh::revoke_account_on(&mut tx, account_id, Some(&grant.family_id), Utc::now())` before commit. The post-commit `revoke_account_except` is removed. In-memory `access.revoke_account_except` and `launch_tickets.revoke_account` stay after commit.
  - crates/base-session/src/base/account_admin/mod.rs: GM `remove_identity` runs `revoke_account_on(.., None, ..)` before commit. A failure returns `Removal::RevokeFailed` and the tx drops (rollback). The reply is `account: could not remove identity #<id> from <target>; nothing changed, see the server log`. `revoke::end_live_sessions` stays after commit.
  - docs/commands.md: one sentence on the rollback reply (CRLF kept).
- Part 2, 7e04c130b: live-DB tests.
  - crates/identity/src/routes/account_unlink_tx_tests_live_db.rs (sentinels 0x7000_E400/E401), registered in account_tests_live_db.rs.
  - crates/base-session/src/base/account_admin/unlink_tx_tests_live_db.rs (sentinels E480/E481), registered in account_admin/mod.rs.
  - Clippy fix: `&mut *unlink_tx` changed to `&mut unlink_tx` (explicit_auto_deref).

## Checks (lane, Windows)

- S1 cimmeria-identity (`cargo check --all-targets`): status=ok exit=0
- S1 cimmeria-base-session (`cargo check --all-targets`): status=ok exit=0
- cargo fmt -p cimmeria-identity -p cimmeria-base-session: status=ok exit=0
- S2 cimmeria-base-session (clippy): status=ok exit=0
- S2 cimmeria-identity (clippy, `--all-targets -- -D warnings`): status=ok exit=0 (after the part 2 fix)
- S3 unlink_tx_tests_live_db (base-session filter, 2 tests): status=ok exit=0 (2 passed)
- S3 unlink_tx:: (identity, 2 tests): status=ok exit=0 (2 passed)
- S3 account_unlink: status=ok exit=0 (1 passed)
- S2 cimmeria-test-support (clippy): status=ok exit=0

## Revert-proof

- Identity: removed the in-tx revoke in routes/account.rs; live_db_player_unlink_revokes_other_families_in_its_transaction FAILED. Restored from HEAD; tests pass again (2/2).
- Base-session: replaced the in-tx revoke with a no-op in account_admin/mod.rs; live_db_gm_unlink_rolls_back_when_the_revoke_fails and live_db_gm_unlink_revokes_every_family both FAILED. Restored from HEAD; tests pass again (2/2).
- Worktree restored; no uncommitted changes.

## Deviations

- Sentinel range: the brief says "next free after 0x7000_E303"; spec R6 says E400 for identity and E480 for base-session. Followed the spec. CT-75 uses E300..E3FF, so E400 is the next free block.
- The GM `revoke::end_live_sessions` returns bool. The spec removes the partial reply, so the bool is no longer read. Reply behaviour is unchanged.
- The worknote at the spec path did not exist in any checkout. It is at docs/analysis/client-tls-oidc/worknotes/CT-76.md in the worktree.
