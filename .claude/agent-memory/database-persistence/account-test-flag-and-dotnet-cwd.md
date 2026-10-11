---
name: account-test-flag-and-dotnet-cwd
description: account.test_account (TG-NET-16) flow from auth to session_kind; and the [IO.File] relative-path trap that writes into the main checkout
metadata:
  type: reference
---

- `account.test_account` (TG-NET-16, 2026-10-11) flows `validate_credentials` -> `ValidatedAccount` -> `SessionRecord` -> `PendingLogin` -> Phase 3 `ConnectedClientState.session_kind` via `cimmeria_auth::auth::session_kind()`. Seed flags ids 10-14 only; `crates/auth/src/auth/session_kind.rs` guards that set against the live DB.
- Only seven `ConnectedClientState { .. }` literals exist (grep `cinematic_spam_cancel:`); other tests go through `test_fixtures.rs`.
- **Trap:** in the PowerShell tool, `[IO.File]::ReadAllText('relative')` resolves against `[Environment]::CurrentDirectory` (the main checkout), not `Set-Location`. It silently edited main once; always pass absolute paths to .NET file APIs.
