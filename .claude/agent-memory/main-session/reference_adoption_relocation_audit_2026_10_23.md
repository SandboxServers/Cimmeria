---
name: Adoption relocation audit, 2026-10-23
summary: Owner-approved direct-root adoption, versioned layout identity, and legacy interrupted-mover escape.
type: reference
---

At origin/main 4ed823288, U4 #1987, U5 #1988 and close-out #1989 are merged.
The shipped in_place mover created selected/game and renamed reviewed files into
it. macOS copied the install under D-LX12. Both contradict the owner's new rule.

Owner approval (2026-10-23): Windows and macOS adoption both stay at the selected
root. Explicit per-file setup/patch changes, cooked normalization, and replacement
backups there are allowed. Bulk copying, moving, or re-homing is prohibited.

New Plan.schema_version2 and InstallIntent.schema_version2 select direct-root
layout. No Plan field is added: its serialized contents are digested. The pinned
plan enforces the matching schema. InstallIntent::content_root() is a resolver,
not authentication; callers first validate owner, provenance, and journal.
Schema1 managed and successful legacy installs retain root/game. Never move them
back. Interrupted schema1 movers must refuse recovery; metadata-only abandon
retains existing bytes, staged originals, and backups for manual inspection.
Schema2 abandon undoes approved per-file changes. Direct-root Repair/Update
commit per file; selective Uninstall retains root, extras, and original backups.
The copy primitive is test-only for legacy-journal fixtures; production copy
admission is refused and shell modes offer Light on both operating systems.

Do not use AdoptionStatus.owned as content validity: it treats unreadable ownership
as present to prevent adoption over uncertain state. Do not use
LaunchStatus.installation_id: runtime availability, launcher minimum and active
operations suppress it. Authenticated installed_content plus
install_worker::content_valid supplies the content predicate (ledger seed/patch
keys, !seed_adopted, contained nonempty executable, SGWGame directory).

Contract: crates/launcher/desktop/docs/migration.md.
Design: docs/analysis/launcher-uat-fixes/design-adopt-in-place.md.
UAT: AIP-U1..U5 in the campaign README and docs/guides/unified-uat.md.
This note records source/design facts, not native macOS or real-client UAT.
No release is cut by this task.
