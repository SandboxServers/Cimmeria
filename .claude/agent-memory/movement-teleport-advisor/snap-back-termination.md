---
name: snap-back-termination
description: Block-on-sight failure mode — a snap-back whose target is itself invalid loops forever; plus the GM off-navmesh allowance and the case-sensitive world-name trap
metadata:
  type: project
---

**Add to the block-on-sight list: a correction is only terminal if its target is a position the validator would itself accept.**

**Why:** live incident (SigNoz, 2026-09). A GM ended up at `[0,0,0]` in Castle Cellblock — inside the space, off the walkable mesh. Every inbound `AVATAR_UPDATE_EXPLICIT` was rejected `OffNavmesh`; the reject path snapped the client to `last_valid`, which *was* `[0,0,0]`; the client obeyed, re-reported, got rejected again — ~12-15 `FORCED_POSITION` per second until disconnect. A second entity hit the identical loop at `[204.33, -40.26, 6.18]` two minutes later. Player-visible symptom: "stuck in the air rubber-banding ~3 inches up and down".

The root enabler is structural: `update_entity_position` is *deliberately* unchecked so ring transport, respawn, content teleport, NPC movement and GM travel can place an entity anywhere. Nothing validated that the resulting position was one the client could legally occupy, and `last_valid` is read straight off the cell entity. Same state reaches an ordinary player via a stale persisted `sgw_player` row on reconnect or authored-but-unreachable content coordinates.

**How to apply:** any new snap-back / forced-position correction path must answer "what if the target is also invalid?". The seam that now does this is `SpaceManager::reject_outcome` in `crates/services/src/cell/space_manager/client_move.rs` (split out of `entities.rs`), which resolves every hard reject into `Rejected` (target sound → ordinary correction), `Recovered` (target unusable or budget spent → relocate to nearest navmesh point / nearest world respawner / AABB clamp, write it through, `note_authorized_teleport`, snap the client *there*), or `CorrectionSuppressed` (nowhere safe → emit nothing). `MovementValidator::MAX_SNAP_BACK_CORRECTIONS = 5` is the unconditional backstop; any accepted position clears the count.

Two other things learned in the same pass:

- **World-name lookups are exact-match and the input can be typed.** `world_is_known` / `default_space_for_world` / `world_name_for_space` all key on the `spaces.xml` spelling. `.gotolocation harset ...` reported `"Unable to find world: harset"` while `Harset` sat in the table — the only thing wrong was the capital H. `SpaceManager::canonical_world_name` now canonicalises at the console/transfer boundary; everything downstream still compares exactly, because the base's `find_or_create_space` is exact and a lowercase name leaking through would fail *after* teardown (un-spaced player). Watch for this any time a world name comes off a chat line or a config string.
- **GMs get a navmesh-only warn-only allowance, keyed on `CellEntity::access_level >= GameMaster`.** Distinct from the `movement_unrestricted` toggle (`onPhysics` / `/gmsetfly`), which needs an explicit in-game command. Bounds and teleport stay hard-rejecting for GMs. `access_level` comes from `account.accesslevel` at login via `InitPlayerState`, never a client byte — same trust model as `cell::dispatch::gm_gate` and the `.`-console channel gate.

Design written up in `docs/architecture/movement-validation.md` (sections "GM off-navmesh allowance" and "Correction termination"). See [[pr1-bounds-seam]] for the surrounding seam and [[authorized-teleport-paths]] for the unchecked-write paths that can strand an entity.
