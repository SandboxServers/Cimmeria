> Evidence pass, 2026-09-24. Read-only research against `main` @ 192d4216 and 7 days of colo SigNoz data (2026-09-17 to 2026-09-24). Kept verbatim as the evidence record; the ledger in [../audit.md](../audit.md) supersedes it where they differ.

# NPC Cover System — Archaeology Findings

Owner symptom: NPCs in Castle_CellBlock (world 12) and Castle (world 8) don't
seek cover, and NPCs spawned already positioned at a cover spot don't stay
there. This is a READ-ONLY research pass; no repo files were edited.

## TL;DR — root cause found

The Rust cover system (scoring, reservation, flank detection, AI integration)
is fully implemented and unit-tested, but it is fed **prefab-local
coordinates that were never transformed into world space**. The client
computes real cover-node positions at runtime by rotating+translating each
prefab's local `CoverNodePrefabData` offsets by the *placed instance's* world
transform (Ghidra-confirmed, see Part B). Cimmeria's extractor
(`tools/ue3_extract_cover_nodes.py`) captured the untransformed prefab
library only, and the loader/spatial index treat those numbers as if they
were already absolute Castle/Castle_CellBlock coordinates. For all but one
hand-authored exception, `pick_best`/`sets_near` are querying against
essentially unrelated coordinates, so cover is almost never found — and when
an NPC is manually spawned at a real in-level cover spot, it never holds a
reservation there in the first place, so nothing anchors it when combat
starts.

This was already flagged as a suspected-but-unconfirmed gap by another
session: `docs/analysis/castle-cellblock-rebuild/work-packets.md:84` reads
*"cover_nodes.sql stores prefab-local coordinates network-wide with no
per-space placement transform anywhere in the schema — meaning the
cover-proximity detection system as currently seeded is likely non-functional
for every cover set in every space, not just this one. Not investigated
further; worth its own future packet if confirmed."* This session confirms
it, with the exact transform math from the client binary.

---

## Part A — What exists in the Rust server today

Fully implemented, well-tested, and wired into the NPC AI tick:

- `crates/services/src/cell/cover/types.rs` — `CoverNode`, `CoverHeight`
  (Low 0.71m / Mid 1.07m / High 1.52m / Los 2.52m — binary-confirmed via
  direct memory read of `DAT_018f41d4/d0/cc/c8`, per code comment),
  `CoverQuality` (Good/Better/Best/None), `CoverSlotKey`, the `Cover`
  service handle.
- `crates/services/src/cell/cover/loader.rs` — loads
  `resources.cover_sets` + `resources.cover_nodes` from Postgres once at
  cell-service startup (`crates/services/src/cell/service/startup.rs:224-243`).
  Falls back to `Cover::empty()` on any load error so the cell still starts.
- `crates/services/src/cell/cover/spatial.rs` — a single **global,
  per-process** uniform grid (`GRID_CELL = 16.0`) over every loaded node
  (comment says "9,346 nodes across the corpus"). **Not scoped per space —
  every space's NPCs query the same flat index.**
- `crates/services/src/cell/cover/scoring.rs` — six-weight scorer
  (`aDistanceWeight`, `aDefCoverWeight`, `aOffCoverWeight`, `aMoveWeight`,
  `aCrossPathWeight`, `aCoverWeight`) matching the client's tunable weight
  names, plus two Cimmeria additions: a flanking half-plane test with a 5°
  hysteresis band, and a squad-affinity penalty so multiple NPCs don't pile
  into one chunk. Heavily unit-tested.
- `crates/services/src/cell/cover/reservation.rs` — slot reservation table,
  auto-releases a prior slot on re-reserve (mirrors the `.def` comment on
  `SGWCoverSet.reserveCoverSlot`).
- `crates/services/src/cell/cover/ai_integration.rs` — `maintain_cover_for_npc`,
  the decision function: `StayInCover` / `Released` (flanked) / `MoveToCover`
  / `NoCover`. Race-safe (single mutex guard for the whole pick+reserve
  sequence), unit-tested including a negative-logging regression guard.
- `crates/services/src/cell/service/npc_ai/fight.rs:262-365` — the actual
  call site. Runs when `use_cover && !is_stationary`; overrides
  `nav_target_pos` with the cover slot's position on `StayInCover`/
  `MoveToCover`; releases cover on leash/idle/death transitions (lines
  138-140, 198-200 in that file).
- `crates/services/src/cell/cover/detection.rs` — a *separate*, player-only
  proximity sweep (`COVER_PROXIMITY_RADIUS = 5.0`) that fires
  `OnPlayerEnteredCover`/`OnPlayerLeftCover`/`OnPlayerInCoverDuration`
  content-engine triggers (used by Castle Cellblock mission chains 1032/
  1033/1035, objectives 2482/2484 — see work-packets.md C05).
- Entity fields: `entities/defs/SGWMob.def` has `useCover` (INT8, default 0),
  `CombatStance`, `bCoverFromTarget`, `lastCoverMove`, `lastCoverCheck`,
  `reservedCoverNode` (python dict), and cell method
  `onReserveCoverSlot(entityID, chunkID, nodeID, slotID, success)`.
  `entities/defs/SGWCoverSet.def` is the `ServerOnly` cover-set entity with
  `reserveCoverSlot`/`releaseCoverSlot` cell methods and
  `publicReservationData` (CELL_PUBLIC, `ARRAY<PublicCoverNodeReservationData>`).
- **Spawner gap (already flagged in-code):**
  `crates/services/src/cell/space_manager/spawn.rs:179-183` hardcodes
  `e.use_cover = true` for every spawned NPC with a comment: *"existing mob
  templates don't [carry `use_cover`] ... follow-up once the
  `entity_templates.use_cover` column lands"*. So today `use_cover` is not
  actually read from the DB template/`useCover` def field — it's a blanket
  `true`. This is a secondary gap, not the primary one (see below), but
  worth closing in the same pass since the wire semantics for stationary
  turrets/emplacements (`is_stationary` gate) already carve those out.
- Legacy Python (`deprecated/python/cell/SGWCoverSet.py`,
  `deprecated/python/base/SGWCoverSet.py`) are **empty stub classes**
  (`class SGWCoverSet(object): def __init__(self): super().__init__()`) —
  the original SGW Python cell-script snapshot in this repo never
  implemented cover AI either. `deprecated/python/cell/SGWMob.py` has
  exactly one relevant line, a TODO: `# TODO: Add other factors into target
  selection (distance, cover, LOS, ...)`. So there is no legacy reference
  implementation to port from — Cimmeria's cover AI is original design
  against the binary's data shapes, not a port.
- **Stale doc reference:** `crates/services/src/cell/cover/mod.rs:8`
  references `docs/architecture/cover-system.md` for "the design overview"
  — **this file does not exist** in the repo. Worth creating or fixing the
  pointer.

### Seed data — the bug, in detail

`db/resources/AI/Seed/cover_sets.sql` (1,381 rows) and
`db/resources/AI/Seed/cover_nodes.sql` (~9,346 rows) are generated by
`tools/ue3_extract_cover_nodes.py` from `covernodes_nikols.pak` +
`covernodes_sdeiter.pak` — binary ZIP archives of **per-prefab cover node
templates**, keyed by prefab/mesh chunk name, e.g.
`_AN-Bench00-15-15`, `_AN-Canal_Cnr00-15-15`, `_CA-CellBlock_Int00-15-15`,
`_CA-CastleEntrance_Facade00-15-15`. These names are art-set prefixes
(`_AN-` = generic environment, `_CA-` = Castle, `_HT-`/`_GA-`/`_HB-`/`_EM-`/
`_LUS-` = other art sets) for **reusable static-mesh/prefab pieces** (a
bench, a canal corner, a railing, a rockpile), not specific level
placements. Only 7 of the 1,381 chunk names even reference Castle/CellBlock,
and those are art-set prefab pieces (a facade piece, an interior wall
piece), not per-instance placements in a specific map.

The one exception:
`cover_sets.sql:1400-1405` — chunk_id 1381, `Castle_CellBlock_MedStationDesk`,
`primary_author='Cimmeria'`, `src_pak='Castle_CellBlock-fffefffd.umap'`
(**not** one of the two `.pak` files). This was hand-added during the C05
work packet (`docs/analysis/castle-cellblock-rebuild/work-packets.md:80-84`,
2026-09-18) after two independent shape-matching attempts against the
generic prefab-node corpus **both failed to find a match**, "consistent with
these actors never having been captured by the prefab-pak extraction
pipeline" — i.e., someone went and got the real world-space coordinates for
this one desk directly, rather than trusting the pak-derived data. The same
work-packet entry is where the general problem got flagged (line 84, quoted
above) but not chased further.

`resources.cover_nodes.pos_x/y/z` are stored directly as loaded
(`crates/services/src/cell/cover/loader.rs:108-110`) with no transform
applied anywhere in the load or index-build path
(`crates/services/src/cell/cover/spatial.rs:44-50`, `CoverIndex::build`
just buckets nodes by their raw `pos` into the grid).

---

## Part B — What the original client actually does (Ghidra-confirmed)

All addresses below came from live Ghidra MCP queries against `SGW.exe`
this session (`search_functions`, `search_strings`, `decompile_function`),
not from memory.

### The transform bug, confirmed at the source

`USGWCoverNodeComponent_SpawnCoverNode` — decompiled at **`0x00904d80`**
(function currently named `FUN_00904d80` in the live Ghidra project; matches
the address already on record in `docs/reverse-engineering/findings/cover-system.md`).
Per-record loop reads a `CoverNodePrefabData` record (confirmed 0x18-byte
stride: `param_2 * 0x18` indexing, matching the struct layout already
documented in the findings doc: `+0x00/0x04/0x08` position floats, `+0x10`
orient float, `+0x14` quality byte, `+0x15` height/width byte). The
critical lines:

```c
fStack_a4 = fStack_64 * fVar9 + fStack_54 * fVar1 + fStack_74 * fVar2 + fStack_44 + (float)param_1[0x37];
fStack_a0 = (float)param_1[0x38] + fStack_70 * fVar2 + fStack_60 * fVar9 + fStack_50 * fVar1 + fStack_40;
fStack_9c = (float)param_1[0x39] + fStack_6c * fVar2 + fStack_5c * fVar9 + fStack_4c * fVar1 + fStack_3c;
```

`fVar9/fVar1/fVar2` are the record's local `pos.x/y/z`. `fStack_64.._44`
etc. are the 3x3 rotation-matrix rows built by `FUN_004e6d50(&fStack_74,
&iStack_80)` immediately before (a rotator/quaternion → matrix conversion —
consistent signature for UE3's `FRotationMatrix`/actor rotation lookup).
`param_1[0x37]/[0x38]/[0x39]` are the owning component/actor's cached world
position. **This is `world_pos = owner_rotation_matrix * local_pos +
owner_world_pos`** — a textbook per-instance placement transform.

The orientation gets the same treatment: the local `orient` float
(`+0x10` in the record) is converted from radians to UE3 rotation units
(`* DAT_0181998c * _DAT_018199e0`) and then composed with the owner's own
rotation (`piVar5[0x94] = piVar5[0x94] + param_1[0x3b]`, and a
quaternion/rotator combine via `FUN_004f7bc0`).

**Conclusion:** the shipped client (and, by symmetry, the original SGW
server that authored the equivalent world-space data) never uses the raw
`covernodes_*.pak` local offsets directly as gameplay-relevant positions.
It resolves them against wherever the specific prefab **instance** is
placed in a specific level, at the moment `USGWCoverNodeComponent` spawns
concrete `ACoverLink` actors (class `SGW_Cover.CoverNode`, per the
`UObject__StaticLoadObject(..., L"SGW_Cover.CoverNode", ...)` call in the
same function) into that level.

### Corroborating evidence for the prefab → instance model

- Editor commands `LoadCoverNodeFromPrefab` (str @ `0x01a91f14`) and
  `AddCoverNodeToPrefabInstance` (str @ `0x01a91f44`) — explicit editor
  operations for attaching a prefab's cover-node template onto one *placed
  instance* of that prefab.
- `"Client regenerated cover links for chunk %08.8x"` (str @ `0x019ae708`)
  — cover links are rebuilt per BigWorld/UE3 **chunk** (i.e., per
  streaming-load unit of a specific map), confirming the runtime cover
  graph is a property of the *level*, not the prefab library.
- `"Export Cover Nodes to CSV"` (str @ `0x01a3f290`/`0x01a3f420`) with
  header `CoverNodeXPosition,CoverNodeYPosition,CoverNodeZPosition,
  CoverNodeHeight,CoverNodeWidth,CoverNodeQuality,OrientationRadians` — an
  editor tool to dump the *resolved* (post-transform, presumably
  world-space) cover nodes for a level, which is exactly the dataset
  Cimmeria actually needs and does not have.
- `BUILDCOVER FROMDEFINEPATHS=0`/`=TRUE` (strs @ `0x01a3fb54` etc.) and
  `BuildCoverNodes`/`BuildCover` — a cook/build-time step that bakes the
  final cover-link graph, consistent with `ACoverLink` actors ending up
  concretely placed inside the cooked `.umap` chunk files (the same class
  of data this agent has previously extracted for terrain/BSP —
  see `docs/reverse-engineering/findings/bsp-model-polys-serialize.md` and
  `docs/reverse-engineering/findings/ue3-terrain-serialize` precedent).

### Remaining Ghidra open questions from `cover-system.md` — status

The existing `docs/reverse-engineering/findings/cover-system.md` (HIGH
confidence, V5 campaign W-cover session) already covers most of the wire
surface (movement type 0 = `MOB_MOVEMENT_Cover`, `SGWCoverSet` cell
methods, `CoverInfo` player HUD object, the three `Event_NetOut_Cover*`
GM-tuning signals) — not re-litigated here. This session's new contribution
is the transform-math confirmation above (their open question 6, "are
1,332 cover nodes the full set...examine covernodes_local.pak", is now
answered: yes for the template library, but the template library alone is
architecturally insufficient — it needs per-instance placement data this
session located the mechanism for but did not extract).

Their open question 5 (how does `USGWAnim_BlendByCover` know "in cover" vs
"moving to cover") remains unconfirmed — not chased this session; lower
priority than the coordinate bug since the finding doc's existing inference
("position-at-node is sufficient, no server message needed") is plausible
and doesn't block a fix.

---

## Part C — Minimal faithful behavior and what it needs

### The real fix: extract cover nodes from the cooked `.umap` chunks, not the prefab pak

Given Part B, the architecturally-correct data source is the **concrete,
already-transformed `ACoverLink` actor placements baked into each level's
cooked map chunks** (e.g. `Castle_CellBlock-*.umap`,
`Castle-*.umap`) — the same class of extraction this project has already
done for terrain (`UTerrain::Serialize`) and BSP/`UModel`/`UPolys` data.
`crates/upk-objects` already parses UE3 package structure; a cover-node
extractor would walk each map chunk's actor export table for `ACoverLink`
(and `ACoverSlotMarker`/`ASGWSpecCoverNode`) exports, read each actor's
`Location`/`Rotation` (already in absolute space, no further transform
needed) and the per-slot metadata off the `ACoverLink` struct layout
documented in `cover-system.md` (`+0x28c` slot array, `+0x290` slot count,
`+0x9a` flags). This would need read access to the actual cooked client
tree (`..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC` per repo
convention) which this session did not walk (out of scope: read-only
research, no new extraction tooling written).

A lighter interim alternative if a full `.umap` actor extractor is too
large a lift right now: manually author additional one-off `cover_sets`/
`cover_nodes` rows the way `Castle_CellBlock_MedStationDesk` (chunk_id
1381) was done — i.e., hand-locate real cover-worthy spots in
Castle_CellBlock/Castle (corners, doorframes, low walls) from level
geometry/screenshots and seed them as genuine world-space entries. This
scales badly (there are two full worlds' worth of guard encounters) but
unblocks specific missions/rooms immediately, same as C05/C06 already did.

### Minimal viable behavior once real coordinates exist

The Rust logic already does everything else correctly:

1. **NPC spawned in cover, stays there:** today, nothing reserves a slot at
   spawn time — an NPC only acquires a reservation on its first
   `Fighting`-state AI tick via `maintain_cover_for_npc`, and only if
   `pick_best` finds a real node near its *current* position. If the
   spawner instead reserved the nearest valid cover slot (post-fix, with
   real coordinates) at spawn time when the spawn position is within, say,
   1–2m of a known node, the NPC would enter combat already holding that
   slot and `StayInCover`/flank-check logic (already implemented) would
   keep it there until flanked. This is a small, additive change to
   spawner logic once real per-space coordinates exist — no new design
   needed.
2. **NPC seeks cover when available:** already implemented end-to-end
   (`pick_best` → reserve → `MoveToCover` → `nav_target_pos` override →
   existing movement/nav-path plumbing sends `aMovementType=0`
   `MOB_MOVEMENT_Cover` per `crate::cell::abilities::broadcast_movement_type`
   call sites). Will "just work" once the position data is real.
3. Close the `spawn.rs:179-183` gap (`use_cover` hardcoded `true`) by
   threading the entity template's `useCover` value through once that
   column exists — otherwise every NPC (including ones that shouldn't
   duck, e.g. melee-only brutes) attempts cover behavior. Independent of
   the coordinate bug but should probably land in the same pass since both
   surface as "cover behaves wrong."

---

## Part D — Telemetry for SigNoz visibility

### What's already there (good)

- `crates/services/src/cell/service/npc_ai/fight.rs:290-362` — structured
  `tracing::info!`/`debug!` at `target: "npc_ai"`, `event = "decision"`,
  `decision_outcome` field, for `stay_in_cover`, `move_to_cover`, and
  `cover_released_flanked` (with `chunk_id`/`node_id`), plus the
  `OnNpcFlanked`/`player_flanked_npc` content-engine dispatch on release.
- `crates/services/src/cell/cover/ai_integration.rs:63-71` — `warn!` at
  `target: "cover.reservation"`, `reason = "cover_slot_taken"` when a
  reservation race is lost (npc_id, holder, chunk_id, node_id fields).
  Unit-tested (`try_reserve_warns_when_slot_taken_by_other_holder`).
- `crates/services/src/cell/service/ticks/cover.rs:150-204` (player-side
  detection) — `debug!` at `target: "cover.detection"` with full geometry
  (`edge`, `entity_id`, `cover_set_id`, `crouched`, position, nearest node
  id/distance) on every enter/leave edge; also a `player_journal::note`
  entry (`COVER_EDGE` kind) for player-facing debugging.

### The gap that would have caught this bug immediately

`crates/services/src/cell/service/npc_ai/fight.rs:363` —
`CoverDecision::NoCover => {}` is **completely silent**. This is the branch
hit every single time `use_cover=true`, the target is out of range, and
`pick_best` finds nothing nearby — which, given the coordinate bug, is
effectively *every* NPC combat encounter in Castle/Castle_CellBlock today.
There is currently no way to tell from logs whether an NPC:
(a) didn't need cover (target in range — benign), vs.
(b) wanted cover and found nothing within `MAX_COVER_DISTANCE` (30m) —
the silent-failure case that is the actual bug.

**Recommendation** — split the `NoCover` telemetry in
`ai_integration.rs`/`fight.rs` per
`docs/architecture/negative-logging-convention.md` conventions:

1. In `maintain_cover_for_npc` (`ai_integration.rs`), when `pick_best`
   returns `None` at line ~208-211, emit:

   ```rust
   tracing::debug!(
       target: "cover.selection",
       npc_id = npc_id.0,
       npc_x = npc_pos.x, npc_y = npc_pos.y, npc_z = npc_pos.z,
       threat_x = threat_pos.x, threat_y = threat_pos.y, threat_z = threat_pos.z,
       search_radius = MAX_COVER_DISTANCE,
       reason = "no_candidate_in_radius",
       "NPC wanted cover but no unreserved node was found within range"
   );
   ```

   Level `debug` in steady state is fine, but consider a rate-limited
   `warn!` (or a counter metric) if this fires for a large fraction of
   fight ticks in a space that has *any* cover data loaded — that ratio is
   exactly the signal that would have surfaced this bug from a live
   SigNoz session without needing Ghidra at all.
2. In `fight.rs`'s `CoverDecision::NoCover => {}` arm, distinguish the two
   causes explicitly instead of a no-op:

   ```rust
   CoverDecision::NoCover => {
       tracing::trace!(
           target: "npc_ai",
           event = "decision",
           decision_outcome = "no_cover",
           npc_id, target_id, in_range,
           "NPC AI: cover not used this tick"
       );
   }
   ```

   `in_range=true` cases are noise (expected, most ticks); keep those at
   `trace`. The signal is in aggregating `decision_outcome="no_cover"
   AND in_range=false` over time per space — that ratio, per space, is the
   single metric the owner asked for ("does this space have working
   cover").
3. **Space-level startup metric:** at cell startup
   (`crates/services/src/cell/service/startup.rs:224-243`), after
   `Cover::from_loaded`, log the loaded node/set counts (this already
   happens generically at the DB-query level in `loader.rs:78`/`166`) but
   add a per-space cross-check: for each space, count how many loaded
   cover nodes fall within that space's `MinX/MaxX/MinY/MaxY` bounding box
   (from the spaces XML already parsed by `SpaceManager`). Log a `warn!`
   (`target: "cover.coverage"`, `reason = "no_nodes_in_space_bounds"`) for
   any space with `use_cover`-eligible NPCs but zero in-bounds nodes. This
   single check, run once at startup, would have caught today's bug in
   under a second without touching gameplay code — Castle/Castle_CellBlock
   would show 0 or near-0 in-bounds nodes despite 9,346 nodes being loaded
   globally.
4. Keep the existing `cover.detection`/`cover.reservation`/`npc_ai`
   `decision_outcome` fields — they're already well-shaped for SigNoz
   `groupBy(decision_outcome)` queries; the fix above just adds the
   missing branch.

---

## Part C (revised) — Scoped to "faithful to original, first pass"

Owner decision (relayed): first pass should match the original as closely
as client + data allow — cover-node selection, crouch/in-cover pose,
peek-and-shoot, and seeking cover under fire — with anything needing a
client patch explicitly flagged. This section supersedes the generic
Part C above with that scope.

### New evidence this pass (all server-side / data, no client patch needed)

- **`EStance` (`entities/defs/enumerations.xml:190-198`:
  `STANCE_Undefined/Defensive/Conservative/Aggressive`) is very likely what
  `SGWMob.CombatStance`/`changeStance` actually carries** — not a visual
  pose enum. The `.def` comment on `CombatStance` ("whether and how I use
  cover") reads as an AI *policy* knob (how readily this NPC falls back to
  cover), not "is my avatar crouched right now." `priorStance` ("the
  stance I was previously in, and need to return to") is consistent with a
  save/restore around a temporary override (e.g. a scripted "force
  aggressive, ignore cover" moment then revert). `changeStance` lives under
  `<CellMethods>` (`entities/defs/SGWMob.def:565-762`), i.e. it's callable
  on the cell entity (including by the cell's own AI script calling it on
  itself) — not a `ClientMethod` broadcast; `SGWMob`'s `<ClientMethods>`
  block (lines 554-563) only has `onAggressionOverrideUpdate/Cleared`, so
  there is no dedicated cover-pose broadcast method for NPCs.
  `entities/defs/SGWGmPlayer.def:307-310` has a matching GM debug command
  `gmSetMobStance(aNewStance)` — a ready-made faithful test/debug hook once
  `CombatStance` is wired.
- **Ability 1451, "Cover Stance" (`db/resources/Abilities/Seed/abilities.sql:2072`:
  `+200 Cover Defense`, cooldown 2, flags 520) already exists in the seed
  data.** This looks like the mechanical payoff for "peek-and-shoot" /
  "seek cover under fire" fidelity: grant this buff when an NPC (or player)
  enters cover (`StayInCover`/`MoveToCover` decisions) and remove it on
  `Released`/leash/death — using the ability/effect pipeline that already
  exists (`docs/architecture/abilities-and-effects-system.md`), no new
  subsystem required. This is likely closer to "faithful peek-and-shoot"
  than any lean/peek animation state machine: the NPC keeps firing its
  normal ability rotation from its current (crouched) position/pose, just
  with a combat-relevant defense bonus active — simple, server-authoritative,
  and matches the ability data already in the DB.
- **`USGWAnim_BlendByCover`'s only compiled (native) function is a trivial
  vtable stub that returns `1`** (decompiled live this session at
  `0x00e90c60` — `return 1;`, nothing else). This means the actual per-frame
  "which pose to blend to" decision is **UnrealScript bytecode**
  (`SGWAnim_BlendByCover.cpp`'s native shell wraps a `.uc` class), not
  compiled x86 — it is **not recoverable via Ghidra native decompilation**.
  Likewise, targeted string searches this session for `CoverAction`,
  `ClaimCover`, `EvaluateCover`, `LeanLeft/Right`, `PeekLoc`, `CoverStance`
  (as an animation-state string) found **no hits** — stock UE3/UDK's
  Gears-style lean-and-peek cover state machine does not appear to be
  present in this binary at all, native or string-referenced. This is a
  genuine unresolved boundary, not a gap in this session's effort — full
  resolution would require an UnrealScript-bytecode decompile of
  `SGWAnim_BlendByCover.uc` (and whatever Pawn/AIController class drives
  it), which is out of scope here.

### What can be built now with NO client patch (server-authoritative, matches the existing wire contract)

1. Real per-space cover-node world coordinates (Part B's fix — extract
   `ACoverLink` actors from the cooked `.umap` chunks, or hand-author
   verified entries the way chunk 1381 was done). This alone is
   prerequisite to everything else below actually being visible in-game.
2. NPC seeks cover under fire and moves to it: **already implemented**
   (`pick_best` → reserve → `MoveToCover` → `nav_target_pos` override →
   `broadcast_movement_type(..., MobMovementType::Cover, ...)`), and needs
   nothing more than #1 to start working.
3. NPC spawned already in cover stays there: needs the small spawner
   addition described in the original Part C (reserve the nearest valid
   slot at spawn time if the spawn position is within a tight tolerance of
   a real node) — additive, no client change.
4. "Seek cover under fire" specifically (as distinct from "seek cover
   whenever out of range," which is what's implemented today): if the
   owner wants cover-seeking gated on "currently being shot at / taking
   damage" rather than purely on range, that's a small condition change in
   `fight.rs`'s `use_cover && !is_stationary` gate (e.g., add a recent-
   damage-taken or `in_range==true but under fire` branch) — still no
   client patch, this is purely a server AI-timing decision the client
   already renders correctly via existing movement-type + position wire
   messages.
5. "Peek-and-shoot" via the Cover Stance ability buff (above) — grant/
   revoke ability 1451 on cover enter/exit. No client patch: buffs/ability
   casts already round-trip over the existing wire protocol.
6. `CombatStance`/`changeStance`/`priorStance` wiring as an AI-tuning
   layer (how eagerly a given NPC archetype uses cover) — purely
   server-side data plumbing (entity template field → AI decision
   weighting), no client visibility at all beyond its effect on movement/
   ability choices the client already renders.

### What MIGHT need a client patch — flagged, not resolved

- **The crouch/stand/prone visual pose itself.** The existing finding doc's
  working hypothesis (client infers pose purely from the NPC's position
  coinciding with height metadata on a *real, level-placed* `ACoverLink`
  actor) is plausible and, if true, needs no client patch — just correct
  world-space node data (item #1 above) so the NPC's synced position
  actually lands on a real cover actor. **But this is not confirmed.** If
  the pose instead requires an explicit claim/link reference set on the
  NPC's pawn (the native "ClaimedBy" field at `ACoverLink+0x50` documented
  in `cover-system.md`, or some other binding this session did not trace
  because it likely lives in UnrealScript, not native code), and nothing
  server-side currently sets that reference, **the pose might never
  trigger regardless of position accuracy** — in which case the fix is a
  client-side hook (comparable in scope to the ring-transport-688 map
  patch, `docs/architecture/... ring transport client patch precedent`)
  that claims the nearest `ACoverLink` for a networked NPC pawn when the
  server signals cover state.
- **If the cooked Castle/Castle_CellBlock `.umap` chunks turn out to have
  no baked `ACoverLink` actors at all for a given room** (i.e., the level
  was shipped without cover authored there, or cover authoring was
  incomplete at ship — plausible for a game this deep in development), no
  server logic can produce a real pose in that room; a client map patch
  (adding/fixing `ACoverLink` placements, the same class of change as the
  Phase 0 UPK patcher used for the ring ceremony, PR #751) would be the
  only faithful option. This can only be confirmed by actually opening the
  relevant `.umap` chunks (not done this session — read-only research,
  and reading cooked map internals needs the `crates/upk-objects` actor
  export reader, which does not yet parse `ACoverLink` specifically).
- **Recommended cheap validation before committing engineering effort:**
  spawn a test NPC at the one real, confirmed-world-space cover node this
  project already has — chunk_id 1381, `Castle_CellBlock_MedStationDesk`
  (`db/resources/AI/Seed/cover_nodes.sql:~9383-9390`, near `(-234, 66.5,
  -124.7)` per `work-packets.md:86`) — with `use_cover=true` and a nearby
  threat, and watch a live client. If a crouch pose appears purely from
  correct positioning, the "no client patch needed" branch above is
  confirmed and the `.umap` extraction effort is fully justified and
  sufficient. If it doesn't, that's the fastest possible signal that a
  client-side claim mechanism (or missing level authoring) is the blocker,
  before spending effort building a full `ACoverLink` extractor.

## Open questions / follow-ups for the owner

1. Does the owner want a full `.umap` `ACoverLink` actor extractor (the
   architecturally-correct fix, larger effort, benefits every space at
   once), or hand-authored per-room entries the way C05 did (faster for
   specific missions, doesn't scale)? This determines whether the next
   step is a new `tools/` extractor + `crates/upk-objects` actor-export
   reader, or more one-off `cover_sets`/`cover_nodes` SQL rows.
2. The `docs/architecture/cover-system.md` reference in
   `crates/services/src/cell/cover/mod.rs:8` is dead — confirm whether it
   should be created (design ADR) or the comment corrected to point at
   `docs/reverse-engineering/findings/cover-system.md` instead.
3. `spawn.rs`'s hardcoded `use_cover = true` — worth its own small
   follow-up once/if `entity_templates.use_cover` (or the `.def`'s
   `useCover` field) is threaded through; independent of the coordinate
   bug but likely to get re-discovered as "weird" once cover actually
   starts working for some NPCs and not others.
