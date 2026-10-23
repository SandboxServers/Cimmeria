---
name: Adoption relocation audit, 2026-10-23
summary: Shipped in-place adoption rehomes content into root/game; validity differs from ownership and Play readiness.
type: reference
---

At origin/main 4ed823288, U4 #1987, U5 #1988 and close-out #1989 are merged.
`engine/src/storage/adoption/in_place.rs` creates `<selected>/game` and
`move_files` renames reviewed release files into it. `light.rs` and
`in_place_replace.rs` also normalize the cooked-data directory by rename.
The macOS adoption path copies into another installation (D-LX12).
These behaviours require changes under the owner's 2026-10-23 no-relocation rule.

Do not use `AdoptionStatus.owned` as content validity: it treats unreadable ownership
as present to prevent adoption over uncertain state. Do not use
`LaunchStatus.installation_id`: runtime availability, launcher minimum and active
operations suppress it. Ownership is authenticated by `installed_content`; content
readiness is checked by `install_worker::content_valid` (ledger seed/patch keys,
contained nonempty executable, SGWGame directory). No new field may be added to
adoption `Plan`, whose serialized contents are digested.

Design: `docs/analysis/launcher-uat-fixes/design-adopt-in-place.md`.
This note records an audit, not implementation or native UAT acceptance.
