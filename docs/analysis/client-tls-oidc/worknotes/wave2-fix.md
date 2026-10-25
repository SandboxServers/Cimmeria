# wave2-fix worknote

Source notes: `.claude/reports/client-tls-oidc/wave2-review-1.md` (N1..N7) and `.claude/reports/client-tls-oidc/wave2-security.md`.

## Done (committed on client-tls-oidc/campaign)
- High proxy trusted peers: 889df955b.
- N1 053aa5743, N2 dc0f5bd75, N3 cafbc2c3c, N4 fca8b4a01, N6 46ee17522, N7 3f17f05e5.
- Security #6 8265df145 (logout revokes launch tickets).
- Security #2 e3bcebd84 (IPv6 /64 rate-limit keys).

## Not fixed, by decision
- N5: contract.md is untracked in the main checkout; text change `+ Send + Sync` is for the coordinator.
- Security #5: covered by N1. Security #10: no fix needed.

## Left, in order
1. Security #3: reject over-long names before the LinkExistingAccount limiter (routes/link.rs), normalise limiter key.
2. Security #4: dummy argon2 verify for unknown names in verify_account_password (crates/auth password_check.rs); same error code for unknown account and wrong password on link/existing.
3. Security #7, #8, #9 (Low): link-code attempt limit; burn link grant after signup checks; reap or cap uncapped stores (crates/identity/src/stores.rs).
4. Check logging::unpaired_id_tests::unpaired_id_fields_only_shrink (cimmeria-server, handlers.rs) on pre-wave head 40770070c and on current head; fix only if wave 2 broke it, else report pre-existing.
5. live_db_ via tools/build-lane/live-db-test.ps1 (foreground); lane clippy -D warnings and nextest for -p cimmeria-identity and -p cimmeria-auth; fmt.
6. Report at .claude/reports/client-tls-oidc/wave2-fix-1.md.

## Next step
Security #3 (step 1 above).

## Last check status
none in this session yet (previous: status=ok live_db_ 2718 passed after N7).
