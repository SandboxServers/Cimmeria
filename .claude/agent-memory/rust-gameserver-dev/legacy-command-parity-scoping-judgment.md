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

- **The `Option<u32>` "fall back to caller when no target"-dead-code pattern
  recurs across every `Target::Being`/`Target::Mob`/`Target::Spawnable`
  handler, not just the ones P02 happened to touch.** `dispatch::resolve_target`
  (`crates/services/src/cell/console/dispatch.rs`) returns `Err` before `exec`
  is ever called for any typed (non-`Target::None`) spec with no current
  selection — it only ever returns `Ok(None)` for `Target::None` commands. So
  any handler behind a typed spec that still takes `target_id: Option<u32>`
  with a `.unwrap_or(caller_id)`/similar fallback has dead code; check for
  this pattern proactively in every packet that touches an existing typed
  handler, not just when a reviewer flags it. Fix: change the signature to
  take the id directly (`target: u32`) and `.expect("Target::X guarantees a
  resolved target")`-unwrap at the `dispatch.rs` call site — P02 set the
  precedent in `query.rs`, P03 repeated it in `stats.rs`.

- **`StatList` (`crates/entity/src/stats/stat_list.rs`) has no removal API,
  and `CellEntity::new` unconditionally calls `StatList::new()`, which
  populates every stat id currently used anywhere in the `.primarystats`
  family.** This means any "missing stat" / "stat absent from the block"
  acceptance criterion in this campaign cannot be satisfied by a real entity
  fixture built from `crates/services` alone — there is no public way to
  remove an entry once inserted. Don't spend time hunting for a fixture trick;
  either test the absent-stat formatting logic as a unit test on the
  extracted formatting function directly (P03's approach — see
  `crates/services/src/cell/console/stats.rs`'s `format_stat_line`), or flag
  a proposed small `StatList::remove` addition in the handoff without adding
  it unasked (it's outside `crates/services`-scoped packets' owned paths).

See also [test-file-split-without-touching-mod-rs](test-file-split-without-touching-mod-rs.md).

- **`Target::None` is not always "no target" — check whether the command's
  own legacy semantics are actually optional-target-with-caller-fallback
  before assuming the "typed spec + `.unwrap_or(caller_id)` = dead code"
  rule from above applies.** P26's `.gotoxyz` (legacy `entity = target or
  player`) is genuinely `Target::None`, so `dispatch.rs`'s `resolve_target`
  really can return `Ok(None)`, and `target.unwrap_or(caller_id)` in the
  handler is live code — the dead-code trap only applies to *typed*
  `Target::Being`/`Mob`/`Player`/`Spawnable` specs, where `dispatch.rs`
  guarantees `Some` before the handler ever runs. `.info` (P01/P02 era) is
  the other example of this same live pattern.

- **A `pub(super)`-scoped helper in a sibling module tree
  (`cell_methods::gm::forward_to_base`, `pub(super)` = visible only within
  `cell_methods`) is NOT reachable from `cell::console`, even though both
  are children of `cell::`.** Don't widen a shared helper's visibility to
  reuse it from an unrelated owned-path — that's a shared-file edit outside
  a worker's normal ownership. Reimplement the same few-line check locally
  instead (P26 did this for the "closed channel → warn + return, don't
  claim a snap that never sent" pattern) — cheaper than a coordinator
  round-trip for a 10-line helper.

- **Splitting a file into `name/mod.rs` + `name/sibling.rs` along a
  data-vs-types (not code-vs-tests) seam needs zero visibility widening
  when the moved data is already `pub(crate)` (or private) at the parent
  module** — Rust's privacy model makes a private/`pub(crate)` item visible
  to every *descendant* module of its defining module, and the new
  submodule is a descendant. P26 split `console/registry.rs` (698/700 hard
  cap) into `registry/mod.rs` (the `Target`/`Spec` types + `spec()` builder)
  and `registry/commands.rs` (just the `COMMANDS` array) this way — `commands.rs`
  reaches `spec`/`Spec`/`Target` via a plain `use super::{...}`, no `pub`
  changes anywhere. The parent's own `mod registry;` declaration needs zero
  edits either (`x.rs` → `x/mod.rs` resolves identically).

- **The campaign's mandatory "controlled negative run" has a silent trap:
  restoring the real implementation with `Copy-Item <backup> <file>` also
  restores the *backup's* mtime**, which is older than the artifact Cargo
  just built from the stub. Cargo then decides the crate is unchanged and
  re-runs the **stale stub test binary**, so the restored (correct) code
  still shows the negative run's failures and it looks like the restore
  failed. `git diff` and file hashes both say the file is fine, which makes
  it maximally confusing. Fix: `(Get-Item $f).LastWriteTime = Get-Date`
  after any backup-restore, before re-running tests. (For a *pure addition*
  packet, the "revert" to run is a deliberately naive body — e.g.
  case-insensitive / first-match-wins / failure-shapes-collapsed — not a
  deletion; expect only the contract-specific tests to fail and say in the
  handoff why the happy-path ones correctly still pass.)

- **When a lookup/query returns "which one of several," return a variant,
  never "the first one."** `SpaceManager.spaces` and `SpaceInstance.entities`
  are both `HashMap`s, so first-match-wins is genuinely nondeterministic per
  process, not merely arbitrary-looking. P44's `PlayerNameLookup::Ambiguous
  { entity_ids }` (sorted ids + `tracing::error!`) is the shape to copy for
  any "this invariant should hold, but prove we don't silently paper over it"
  acceptance criterion.

- **`CellEntity::character_name` is the player-only display name and is
  written by exactly one site** — the `BaseToCellMsg::InitPlayerState` arm in
  `cell/service/base_messages/mod.rs`. NPCs use the separate `npc_name`.
  Message ordering is `CreateEntity` → `ConnectEntity` → `InitPlayerState`,
  so a cell entity has **no name at all** between create and init — which is
  why a player mid-gate-travel is invisible to any name-keyed cell lookup
  (`create_entity` builds a fresh `CellEntity` with `character_name: None`).
  Any "find player by name" feature inherits that window; legacy's
  `PlayersByName` dict did not, because its key survived until
  `disconnected()`.

- **Check the legacy Python CLASS HIERARCHY before porting a command's
  `targetType` string to a Rust `Target` variant.** The legacy registration
  table's target class is an `isinstance` check against a base class, so
  subclasses satisfy it. `SGWPlayer(SGWBeing(SGWSpawnableEntity))` means legacy
  `Command('despawn', ..., 'SGWSpawnableEntity')` accepted a *player* — and the
  Rust `Target::Spawnable`'s `matches()` returns `true` unconditionally, so a
  literal port reproduces the hole. `grep '^class SGW' deprecated/python/cell/*.py`
  gives the whole tree in one call; do it for any command whose legacy target
  class is a base class (`SGWSpawnableEntity`, `SGWBeing`). D02 says correct
  these rather than reproduce them. For a destructive command, put the refusal
  in the `SpaceManager` primitive too, not only the registry spec — a registry
  `Target` is one table edit away from regressing, and the revert experiment
  then *demonstrates* the layering (loosening the spec alone left the player
  alive).

- **`SpaceManager::destroy_entity` does NOT do witness cleanup;
  `disconnect_entity` does.** `destroy_entity` drops the entity from
  `space.entities` + the spatial grid but leaves it in every observer's
  `witnesses` set. The next `compute_aoi_changes()` tick *does* emit `LeftAoI`
  via its "in previous but not in current" arm, so nothing is silently lost —
  but it is deferred a tick, skipped for any space whose `players` set the tick
  guard passes over, missed entirely for an observer who leaves in between, and
  never an assertable *count*. For any new "destroy an entity" command, reuse
  `disconnect_entity`'s shape (collect observers from `space.players` where
  `other.witnesses.contains(&target)` → send `LeftAoI` per observer → scrub the
  target from every witness set → `destroy_entity`). P08 packaged this as
  `SpaceManager::despawn_npc` in `space_manager/entities.rs`; `.delspawn` still
  uses bare `destroy_entity` and should be switched over by P10.

- **The `gmSpawnByCmd` cell↔base round-trip is already truthful about creation
  results — don't re-derive it, and don't undermine it.** `cell_methods/gm/spawn.rs`
  enqueues `CellToBaseMsg::GmSpawnNpc` and sends NO feedback; `base/gm_spawn.rs`
  sends the "template not found" line; `cell/service/base_messages/gm_spawn.rs`
  sends "spawned npc `<id>`" only after `spawn_npc_from_record_in_space` returns
  `Ok`. Any new spawn command should ride that and stay silent at enqueue. Its
  outcome strings are now command-neutral (`spawned npc …`, `spawn failed: …`)
  because three GM commands share it. `GmSpawnNpc` carries `heading` as of P08;
  the native `gmSpawnByCmd` sends `0.0` because its wire signature
  (`WSTRING DesignId, FLOAT XOffset, FLOAT ZOffset`) has no rotation argument.

- **CORRECTED (P18, 2026-09-17): `CellEntity.direction` is `[pitch, yaw,
  roll]` in radians for players AND NPCs alike — it is NOT a direction
  vector for players.** An earlier version of this bullet claimed the player
  form was a Cartesian vector; that was wrong, and believing it is what
  produced the whole `atan2`-vs-`direction.y` bug family (P48). Full
  evidence chain, the affected call sites, and the
  `update_entity_position`-zeroes-facing trap are in
  [cell-entity-direction-semantics](cell-entity-direction-semantics.md) —
  read that before touching any orientation code.

- **When a command reuses an existing native handler's core mechanism
  (`update_entity_position` + `note_authorized_teleport`, gated
  `TeleportPlayer` for players only), grep the native handler's own test
  file for an existing "does this actually reach witnesses" proof before
  writing a new one from scratch** — `cell_methods/gm/tests/travel.rs`'s
  `summoned_npc_is_broadcast_to_caller_witness` (a two-tick
  `compute_aoi_changes()` sequence) is the reusable proof shape for "grid
  update alone is sufficient, no separate AoI refresh needed." Still write
  your own copy against your own dispatch path rather than just citing the
  native test — P26 wrote two (NPC case + player case, since the native
  test only covers the NPC case), because the acceptance criterion is about
  *this* command's witness behavior, not the native one's.

- **Read the shared helper's SIGNATURE before copying a sibling command's
  call verbatim — the sibling may be carrying a latent bug.** P18's
  `.location` looked like a straight copy of P26's `.gotoxyz` call shape
  (`update_entity_position(id, pos, [0, 0, 0], [0.0; 3])`), but that third
  parameter is `direction: [i8; 3]` and the helper writes it into
  `cell_entity.direction` unconditionally — so every caller passing
  `[0, 0, 0]` silently zeroes the moved entity's facing. Copying the call
  would have shipped the same bug under a new command name. See
  [cell-entity-direction-semantics](cell-entity-direction-semantics.md) for
  the workaround pattern and the six affected callers.

- **A "stop and escalate for design sign-off" instruction is worth 20
  minutes of evidence-chasing first.** P18's brief said to stop if full
  Euler orientation couldn't be represented. Chasing the component order
  through three Rust sites and five legacy sites showed it maps 1:1 and no
  escalation was needed. Grep the legacy tree for every read AND write of
  the field (`\.rotation`, `rot\.`), not just the one the packet names — the
  persistence site (`SGWPlayer.save`'s `heading = rot.y`) was the single
  most decisive piece of evidence and was in neither the brief's read set
  nor the command's own file.

- **A legacy "optional trailing args" signature is usually a partial-tuple
  bug, not a feature.** `def location(player, target, x=None, y=None,
  z=None)` with a body gated on `if z is not None` means 1 or 2 args
  silently no-op'd and then printed a readout as though the GM had asked for
  one. D02 says correct it: accept only the complete shapes (here 0 or 3),
  reject everything else with no mutation and *no readout* — and assert
  `feedback.len() == 1` in the test, since a readout alongside the rejection
  is exactly the legacy bug leaking back in.
