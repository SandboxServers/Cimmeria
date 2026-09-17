# legacy-command-parity campaign: scoping judgment calls (from P02)

Working a packet in the Cimmeria `legacy-command-parity` campaign
(`docs/analysis/legacy-command-parity/`) means porting a legacy Python dot
command to Rust. Recurring judgment calls worth remembering:

- **A packet's own "read-only reference" file list may not contain the real
  geometry/logic.** P02's brief pointed at `Entity.py` for `.facing`, but the
  actual `facing`/`facingType`/`distanceTo` math lives in
  `SGWSpawnableEntity.py` (a different file, not in the initial read set).
  Always grep the legacy tree for the method name before assuming a "thin
  wrapper" (W) label in the audit is correct — verify, don't trust the
  work-packet's work-kind letter.

- **When a Rust field's numeric scheme has *already diverged* from legacy's
  enum for the same concept, do NOT port the legacy name table wholesale.**
  Example: `CellEntity::faction: u8` uses a documented simplified scheme
  (0=neutral/1=Tau'ri/3=SGC/10=hostile — see `entity_struct.rs` doc comment)
  that is incompatible with legacy's 34-entry `FACTION_*` table (legacy
  `FACTION_SGC = 2`, this codebase's `3` means SGC). Porting the wrong table
  would silently mislabel data. Check whether the *existing* Rust field
  already has a doc-commented numbering convention before assuming "port the
  legacy enum names" is safe — it's only safe when the numbering matches
  (verified true for `alignment`/`archetype` in this codebase, false for
  `faction`).

- **A named "stop condition" in a work packet is worth verifying by actually
  reading the struct**, not just taking the coordinator's word that it might
  apply. For P02's `.combatinfo`, confirmed by reading
  `crates/entity/src/abilities/defs.rs`/`manager.rs` end to end that there is
  truly no ability-type field anywhere (only ability *ids* are tracked) — this
  turns a "maybe blocked" hedge into a confirmed, citable finding for the
  handoff, which is much more useful to the coordinator than "I think this
  might be missing."

- **A genuinely scoped-down implementation (some checks real, some omitted
  with a code comment + handoff writeup) is preferred over blocking the whole
  command**, per this campaign's D06 ("stubs are not parity" cuts both ways —
  an unregistered command is also not parity). Register the command, run the
  checks you can support with real data, and flag the rest as a stop
  condition for the coordinator to disposition — don't invent data to force
  full parity, don't withhold a functional partial command either.

- **Clippy's `excessive_precision`/`approx_constant`** will flag a hand-typed
  legacy float constant (e.g. Python's `0.78539816` for π/4) — swap for the
  matching `std::f32::consts::*` item; it's the same real number, just
  expressed correctly.

See also [test-file-split-without-touching-mod-rs](test-file-split-without-touching-mod-rs.md).
