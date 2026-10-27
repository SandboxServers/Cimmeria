# Wave 2 review fix pass 2: worknote

Status: done. No next step.

Done:
- Step 0: reverted the token.rs experiment (`git checkout`), worktree clean.
- N1 to N9 committed in ded04dc49, 423149cee, ff564d2ed, 4ca19dc7e, ce4caa70b, a29b8e27e (earlier sessions).
- Live DB run found one failure: `live_db_link_code_global_cap` (429 at redeem 5). Cause: LinkExistingIp (5 per 15 min per IP) also applies on the link-code path. Fixed in 67c3f96a6 by sending each redeem from its own peer (`post_from` made `pub(super)`).
- Live DB tier: 2720 passed, 0 failed.
- Revert proofs (each restored with git checkout, none committed):
  - A: logout revoke removed (token.rs:366) -> `live_db_logout_revokes` fails.
  - B: disabled-refresh revoke removed (token.rs:252) -> `live_db_disabled_account_refresh_is_403` fails.
  - C: malformed-name guard removed (link.rs) -> `live_db_link_existing_refuses_a_malformed_name` fails (401 invalid_credentials).
  - D: burn-first grant order (link.rs create) -> `live_db_signup_refuses_young_provider_accounts` fails (400 invalid_grant on second post).
  - E: global link-code charge removed (link.rs) -> `live_db_link_code_global_cap` fails (401 at 51st).
- fmt check and clippy `-D warnings` for cimmeria-identity pass.

Left: nothing in the packet. N2/N3 (limits.rs unit tests) were not re-proved in this session; the earlier session's finding is taken as stated.

Last check status: `status=ok exit=0 ran=2.202s` (clippy, cimmeria-identity).
