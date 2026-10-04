---
name: Permanent installation owner and current release
description: Signed release evidence is separate from immutable Install ownership; consumer coverage and remaining Update transition
type: reference
---

The desktop owner's InstallIntent and root marker remain immutable. ReleaseIdentity
selects exact retained signed bytes by evidence UUID plus manifest digest. Installed
index schema 3 and content-ready schema 2 bind a separate current release while
schema 1/2 indexes and original receipts remain readable. Repair plans/extraction
and uninstall carry the current reference without changing the permanent UUID.

Two-release fixture states verify consumers only; no Update writer or publication
transaction is implemented by this change. See the permanent-owner section in
`crates/launcher/desktop/docs/migration.md`. Do not claim Update, rollback, effective
settings or adopted Play completion from receipt-reader tests.
