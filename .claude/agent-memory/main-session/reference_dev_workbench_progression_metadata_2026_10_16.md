# DW-21 dot progression metadata (2026-10-16)

Source: `docs/analysis/dev-workbench/evidence/DW-21-E.md` and the
`crates/cell-console/src/cell/console/registry/workbench_progression.rs`
catalog implementation. The 27 commands in progression, abilities, stats,
ammo and pet now have reviewed typed descriptors. The catalog's availability
is command-wide; safe subforms of `.dummy`, `.infiniteammo` and `.pet` must
stay disabled with the unsafe forms until per-form gating exists. Queued
Base grants need immutable GM and subject owner checks before DB writes and
after awaits before client/feedback sends; a DB delta alone does not prove
original-session delivery. `.givexp` can credit a reused entity ID's new
character and can increase Applied Science Points. `.giveammo` readback must
use carried bags only, full untruncated rows and the actual committed item
type. The five family `SPECS` sets and catalog projection are compared in
unit tests. No live client was used.
