# Permanent owner and current release

> **Type:** Reference (implementation evidence)
> **Audience:** Launcher implementers and reviewers
> **Last updated:** 2026-10-04
> **Companions:** [Migration contract](../../../../../crates/launcher/desktop/docs/migration.md), [adoption contract](adoption-contract-audit.md)

## Scope

Base `ece32e585`, branch `launcher/owner-receipts`. This removes the requirement
that the current signed release use the original installation operation UUID.
The permanent owner marker, saved Install intent, original signed evidence and
adoption provenance remain intact. Installed index schema 3 binds a separate
`ReleaseIdentity`; content-ready schema 2 binds that reference to the permanent
installation UUID. Existing schemas remain readable without rewriting old plans.

Repair admission, extraction, commit, recovery and backup cleanup use the current
reference. Uninstall checks that receipt while retaining the original owner and
both releases' evidence. Play/minimum and prerequisite inspection already use the
authenticated release returned by the installed-content reader.

## Evidence and limits

- Two-release retained-evidence test reopens exact original signed bytes, refuses
  cross-binding valid evidence to the other digest, and preserves the Install
  intent without synthesizing another Install.
- Installed-content fixtures cover reopen, Repair plan/extraction binding,
  foreign-owner/old-receipt refusal, and uninstall preserving signed evidence.
- Real local HTTP seed reconstruction and filesystem replacement use a different
  signed seed containing a new file. Repair preserves the original owner bytes,
  publishes the current receipt and keeps the damaged old tree as backup.
- Replacing Repair commit's current-release lookup with the original Install
  lookup makes that regression fail with reconciliation required. The source was
  restored immediately after the negative check.

These fixtures construct a post-publication current-release state directly.
They do not implement or validate game Update admission, publication checkpoints,
Update rollback, effective imported settings, adopted Play, Windows execution or
packaged UI. The next user-visible acceptance journey is confirmed game Update,
then reopen, Repair of that updated release, and uninstall of the same owner.

Scoped commands use the standalone desktop manifest, package
`cimmeria-launcher-engine`, and the build lane. Filters:
`storage::release_evidence`, `storage::installed_content`, and
`reconstruction_and_commit_keep_current`. Strict all-target engine clippy is
required alongside the full engine suite before this branch is integrated.

## Local results

Native Mac engine suite: **368 passed, 18 ignored** after restoring the regression
probe (`20261004-124759-33363`). Strict engine all-target Clippy passed
(`20261004-124949-34090`), as did formatting and diff checks. The scoped release
evidence tests passed 5/5 and installed-content tests 9/9. The new real-seed
Repair commit test passed; substituting the original-release lookup failed it
(`20261004-124719-33071`). Native Windows and shell integration checks remain open.
