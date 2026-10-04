---
name: Desktop existing-user adoption contract audit
description: Legacy claims do not establish desktop ownership; source-preserving verified-copy proposal and remaining Play/update seams
type: reference
---

Verified 2026-10-04 at ba4b20b6b: legacy `adopt_existing_install` copies an
archive digest into a ledger without checking game bytes. Desktop
`install_worker::content_valid` is layout/ledger validation, not exhaustive
integrity verification. `installed_content` requires a first-install intent and
root ownership with `destination/game`; the old game normally has `Working`
directly under its selected root. Repair reconstructs the same signed release;
there is no Update operation at this revision. Imported config is archived rather
than consumed by Play, and shell Play requires patches even though its engine
resource model allows no patches.

The bounded audit proposes a separately selected desktop-owned verified copy,
reusing actual matched source bytes and preserving the old root. It requires
reference reconstruction from authenticated seed/patch artifacts, explicit review
of modifications/config differences, provenance-aware receipts, effective imported
settings and a separate signed content Update transition. This is proposed design,
not delivered behavior. Full contract, ownership split and tests:
`docs/analysis/playtests/2026-10-03-macos-wine/worknotes/adoption-contract-audit.md`.
