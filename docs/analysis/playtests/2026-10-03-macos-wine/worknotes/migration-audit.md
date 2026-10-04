# Legacy migration and updater audit

> **Type:** Reference
> **Audience:** Launcher contributors and reviewers
> **Companions:** [Requirements](../launcher-implementation-plan.md)
> **Last updated:** 2026-10-04
> **Evidence:** Read-only source audit at `0b10d869c869793ab506dbf9215ddb91714a244b`

## Findings

The desktop does not yet adopt legacy Windows installations. The legacy
`crates/launcher/src/config.rs` reads executable-adjacent `launcher-config.json`,
including login servers, manifest URL, game telemetry and patch settings.
Desktop preferences store only install directory and separate launcher-summary
consent. Calling the legacy config loader is unsuitable for a read-only preview:
it can write migrations back to the source.

`crates/launcher/src/identity.rs` preserves `install.json` and fails on corrupt or
future schemas instead of replacing identity. No corresponding desktop import
exists. Explicit legacy `opted_in` is consent; historical `enabled` alone is not.
Legacy game consent must never enable launcher summaries implicitly.

`crates/launcher/src/state.rs` retains seed hash, ordered patch keys and
`seed_adopted`. Desktop `storage/installed_content/` requires saved intent,
signed release evidence and ownership receipts. Its older-install migration
means an earlier desktop journal, not legacy egui content. A legacy ledger alone
does not justify desktop repair/uninstall or verified-readiness claims.

Legacy and desktop locks occupy different directories. Imported-tree mutation
needs coexistence protection, not just the desktop app-data lock.

The legacy `self_update/` code verifies Windows executable releases and supports
swap/rollback. Desktop has no updater, signed per-platform feed or release-version
mapping, and does not enforce the manifest's `min_launcher`. These remain packet
8 requirements; importing configuration does not close them.

## Next assignment and acceptance

`launcher/legacy-migration` owns native preview/confirmed import and recoverable
persistence in new `storage/migration/` modules. Shared registration is integrated
separately. Repair owns frontend and shell registration until its handoff.

Required guards cover schema variants, source preservation, exact identity and
consent, corruption/conflicts, repeated confirmation/reopen, commit failure,
adopted-content status and real process lock contention. Native Windows results
remain a separate gate. The next visible acceptance test is selecting a legacy
launcher, reviewing its import preview, confirming once and reopening with saved
settings intact; native-persistence Effect UAT must accompany that wiring.

Keep the old updater confined to its own executable. A later desktop updater
packet must establish one owner, signed package assets, version mapping,
minimum-version gating and failure recovery before replacement parity is claimed.

No builds, runtime tests, UI interaction or live-service probes were performed
by the audit. This report records implementation gaps, not validated migration.
