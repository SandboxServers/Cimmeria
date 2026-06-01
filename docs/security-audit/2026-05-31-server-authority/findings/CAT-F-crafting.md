# CAT-F — Crafting / R&D / Training

**Overall trust posture: MIXED — TrainAbility is well-implemented; everything
else is a stub waiting to ship as an exploit chain.** The TrainAbility flow
(cell method 77) is the one path in this category that actually mutates state,
and its server-side validation chain is strong: cell-side checks ability_def
existence, player_id presence, already-known, archetype-tree membership,
level requirement, and per-prereq known-ability; the base then re-validates
under a Postgres `WHERE training_points > 0 AND NOT abilities @> [ability_id]`
guard, making double-debit on concurrent or replayed packets impossible. The
five other handlers in this category (`spendAppliedSciencePoints` 95,
`craft` 96, `research` 97, `reverseEngineer` 98, `alloying` 99,
`respecCrafting` 100) are all Phase-1 stubs that log "UNIMPLEMENTED" and
return `true` — they are NOT exploitable today because no state mutates,
but the dispatch path is wired, the wire shapes are accepted, and the
implementer who fills in Phase 2 inherits a hostile attack surface:
every one of these RPCs trusts the client to name a blueprint id, a
discipline id, or — worst of all — to name **the inventory item_ids
that will be consumed**. The set of mandatory server-authority checks
when Phase 2 lands is large and easy to skip a few of; the findings
below enumerate the must-haves derived from the Ghidra-confirmed wire
shapes and the deprecated-Python reference implementation.

A separate authority gap is also flagged on TrainAbility itself: the
**trainer-NPC interaction requirement was dropped** in the Rust port.
The original Python required `self.trainerEntity is not None` AND
`distanceTo(trainerEntity) <= MAX_INTERACT_DISTANCE` before honoring
`trainAbility`; the Rust handler does not. A client that has any
in-archetype-tree ability they qualify for can train it from anywhere
on the map without ever interacting with a trainer NPC — there is no
proximity, no "trainer-open" session state, no NPC↔ability list match.
This is the same authority gap class as the vendor `vendor_entity`
non-clearing finding in CAT-E (CAT-E-02), and the fix shape is the same:
gate the action on a server-tracked `active_trainer_entity` field set
by the interact path and validated on the train RPC. Filed as CAT-F-01
because the train-anywhere shape is reachable today.

---

### CAT-F-01 — TrainAbility skips trainer-NPC interaction and distance check

**Severity**: Medium
**Class**: Missing context check — client can invoke trainer-RPC without standing at a trainer
**Wire surface**: `Event_NetOut_TrainAbility` (cell method 77, payload `INT32 abilityId`)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (needs live debugger to confirm at triage time)

**Trust violation**
The Python reference at `deprecated/python/cell/SGWPlayer.py:1347` gates
`trainAbility` on two server-tracked conditions:
- `self.trainerEntity is not None` — i.e. the player previously invoked
  `interact()` on a trainer NPC and the cell stored a back-reference.
- `self.trainerEntity().entity().distanceTo(self.position) <=
  Constants.MAX_INTERACT_DISTANCE` — proximity at the moment of the train
  RPC, not just at the moment of the original interact.

The Rust port at `crates/services/src/cell/cell_methods/player/vendor.rs:491`
drops both checks. It validates archetype-tree membership, level, and
prerequisites — all of which it can do off the cached entity state — but
does not consult any `active_trainer_entity` field, and there is no such
field on the entity struct (parallel to `vendor_entity` for the vendor
flow). A client that knows any ability id that exists in their archetype
tree and that they meet level + prereq for can send `trainAbility` from
anywhere in the world (different map cell, mid-combat, while falling, or
from a malicious replay of an old packet) and the server will debit a
training point and grant the ability.

This is not a "free training points" exploit — the player still needs to
have earned the TPs and meet the archetype/level/prereq gates — but it
fully removes the trainer NPC from the gameplay loop, which is also the
content gate for "this trainer hasn't been unlocked for your faction yet"
and "you have to physically visit the trainer in the world." Both of
those are content-team design constraints the auth layer is supposed to
preserve, and right now it doesn't.

**Evidence**
- Ghidra: `019cf940`+ (`Event_NetOut_TrainAbility` registration) — payload
  is a single INT32 abilityId per `entities/defs/SGWPlayer.def` (method
  77 exposed declaration). Client emits only the abilityId; the trainer
  identity is server-side state.
- Python ref: `deprecated/python/cell/SGWPlayer.py:1347-1361` —
  trainer-presence + distance gate documented as a contract the cell
  enforces, with `onError("Not interacting with a trainer entity")` /
  `onError("You are too far away to interact with that")` feedback.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/vendor.rs:491`
  (no trainer-presence check; no distance check).
- Companion struct that should hold the trainer back-ref: see how
  `vendor_entity` is stored on the entity in
  `crates/services/src/cell/interactions/vendor.rs:20` — the trainer flow
  needs a parallel `trainer_entity: Option<u32>` field set by
  `try_open_trainer` at `crates/services/src/cell/interactions/trainer.rs:55`.

**Attack scenario**
1. Player walks up to an in-world trainer once during a normal session,
   triggering `try_open_trainer` and receiving `onTrainerOpen` with the
   abilities they're eligible for.
2. Player walks away (or zones out, or dies) — there is no server-side
   `trainer_entity = None` on any of these transitions because the field
   doesn't exist.
3. Player or a replay tool sends `trainAbility(abilityId)` from anywhere
   for any ability that's in their archetype tree and that they meet
   level + prereq for — the server validates the static gates and
   forwards `CellToBaseMsg::TrainAbility` to the base, which debits 1 TP
   and grants the ability.
4. Observable effect: ability granted, TP debited, `onKnownAbilitiesUpdate`
   broadcast, no trainer NPC ever consulted. Content team's "you must
   visit the trainer" gameplay invariant is broken.

**Suggested remediation (one line)**
Add `trainer_entity: Option<u32>` on the cell entity, set by `try_open_trainer`,
cleared on death/zone/disconnect/distance-out, and require it set + within
`MAX_INTERACT_DISTANCE` before forwarding the cell→base TrainAbility message;
also validate that the requested ability_id is in `trainer_abilities[(list_id,
archetype_id)]` for the open trainer, not just in `archetype_ability_trees`.

**Would benefit from x64dbg trace?**
No — the wire shape is single-INT32 and the trust violation is on a
server-side state field the Rust port hasn't added yet.

---

### CAT-F-02 — `spendAppliedSciencePoints` will trust client's discipline_id when Phase 2 lands

**Severity**: High (latent — Phase 1 is no-op; first Phase 2 impl that lands without these checks is critical)
**Class**: Future-implementer trap — full ASP/discipline mutation path is wired through with stub handler
**Wire surface**: `Event_NetOut_SpendAppliedSciencePoint` (cell method 95, payload `INT32 aDisciplineSeqId`)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (Phase 2 will ship the mutation; the brief explicitly asks "what should the server validate?")

**Trust violation**
The Phase 1 handler at `crates/services/src/cell/cell_methods/player/crafting.rs:23`
accepts the message, parses the 4-byte `discipline_id`, logs
`"UNIMPLEMENTED: spendAppliedSciencePoints (Phase 2)"`, and returns `true`
(handled). When Phase 2 implements the mutation, the implementer must
faithfully port every check from `deprecated/python/cell/Crafter.py:154`
(`spendAppliedSciencePoints`):

1. **ASP balance check** — `self.appliedSciencePoints >= 1` — must be a
   server-authoritative read of `sgw_player.applied_science_points`, NOT
   a trust of any client-side cached value.
2. **Discipline exists** — `DefMgr.get('discipline', disciplineId)` —
   resources lookup against `discipline` table; rejects client-invented
   IDs.
3. **Already-known guard** — `disciplineId not in self.disciplines` —
   silent no-op on duplicates (a replayed packet must not debit twice).
4. **Racial paradigm gate** — `racialParadigms[discipline.racialParadigm.id]
   >= discipline.racialParadigmLevel` — server-authoritative read of the
   player's paradigm levels.
5. **Prerequisite disciplines** — every entry in `discipline.requiredDisciplines`
   must already be in the player's `disciplines` AND at expertise >= 50.
6. **Atomic** — decrement ASP + learn discipline + insert initial
   expertise=1 row in a single Postgres transaction, with the discipline
   uniqueness derived from a `WHERE NOT (discipline_ids @> ARRAY[$1])`
   guard so two concurrent or replayed packets debit at most once.

The wire shape carries ONLY `aDisciplineSeqId`. Every other field in
the validation chain — ASP balance, paradigm level, prerequisite
expertise — must come from server-authoritative state. A Phase 2 impl
that reads any of these from a client-supplied or client-cached value
is a critical exploit (infinite disciplines via 0-cost spend, or
bypassing the paradigm/prereq gates).

Filing this finding now because the brief explicitly asks for the
validation checklist that Phase 2 must satisfy and because the Phase 1
stub already returns `true` — a next-iteration patch that adds the DB
write without the gates would slip through review as "wire it up to
the DB."

**Evidence**
- Wire shape: `entities/defs/SGWPlayer.def:916-919` (`spendAppliedSciencePoints
  <Arg> INT32 <ArgName> aDisciplineSeqId </ArgName>`). Confirmed by Ghidra
  `Event_NetOut_SpendAppliedSciencePoint` registration at `019db48c`.
- Python ref: `deprecated/python/cell/Crafter.py:154-188` — full validation
  chain in source.
- Cross-ref to Rust stub: `crates/services/src/cell/cell_methods/player/crafting.rs:23-43`.

**Attack scenario** (post-Phase-2, assuming a partial impl)
1. Attacker sends `spendAppliedSciencePoints(disciplineId=<any>)` —
   choosing an id that they don't qualify for (high paradigm prereq,
   or prereq disciplines they don't know).
2. Phase 2 impl that omitted the paradigm or prereq check (an easy
   mistake — Python's `learnDiscipline` is permissive on its own; the
   gates live in `spendAppliedSciencePoints` not in `learnDiscipline`)
   debits ASP, learns the discipline, sends `onUpdateDiscipline` and
   `onEntityProperty(AppliedSciencePoints, ...)`.
3. Attacker now has a discipline that the content-design team intended
   to gate behind paradigm/prereq progression — they can craft items
   the design intended to be out-of-reach.

**Suggested remediation (one line)**
Phase 2 implementer must port all six checks from
`deprecated/python/cell/Crafter.py:154`, with ASP read + decrement +
discipline + expertise row insert in a single Postgres transaction
guarded by `WHERE applied_science_points >= 1 AND NOT (discipline_ids
@> ARRAY[$1])` (parallel to the TrainAbility pattern at
`crates/services/src/base/world_entry/methods/progression/mod.rs:456`).

**Would benefit from x64dbg trace?**
No — the wire shape is single-INT32; the audit is on the server-side
validation completeness, not the wire decoding.

---

### CAT-F-03 — `craft` will trust client-supplied item_ids for material consumption

**Severity**: Critical (latent — Phase 1 is no-op; this is the bug that will eat the most reviewer-cycles when Phase 2 lands)
**Class**: Future-implementer trap — client picks the consumption set; server must validate ownership, type, quantity, bag location
**Wire surface**: `Event_NetOut_Craft` (cell method 96, payload `INT32 aCraftId, ARRAY<ItemID> aItems, INT32 aQuantity`)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (the wire shape carries client-named item_ids; the brief flags this exact concern as central question 1)

**Trust violation**
The Ghidra-confirmed wire payload for `craft` (cell method 96) is three
fields: a blueprint id, an **array of item_ids** the client says it
wants to consume, and a quantity (Ghidra:
`Event_NetOut_Craft` emitter at `00e47b10`, with named args
`aCraftId`/`aItems`/`aQuantity`). The .def-level shape is confirmed at
`entities/defs/SGWPlayer.def:921-926`.

Every one of the three client fields is adversarial:

1. **`aCraftId` (blueprint id)** — must be checked against the player's
   server-authoritative `sgw_player.blueprint_ids` array. A client can
   send any int32 — the server must reject any blueprint not in the
   known list.
2. **`aItems` (array of item_ids)** — every item_id must be:
   - **Owned**: the row in `sgw_inventory` belongs to this player. A
     client that sends a friend's item_id (or one it's seen in a trade
     window, or a random scan) MUST be rejected.
   - **In main or crafting bag**: the Python at
     `deprecated/python/cell/Crafter.py:212-215` requires
     `bagId in (INV_Main, INV_Crafting)` — equipped items, bandolier
     items, mail-attached items, etc. are NOT eligible.
   - **Of a type matching the blueprint's component set**: server
     looks up the blueprint's component requirements from
     `resources.blueprints_components` keyed by `(blueprint_id,
     component_set_id)` and confirms the supplied item types cover
     the requirement. (Python: lines 218-231.)
   - **Sufficient stack quantity**: `sum(item.quantity for item in
     items if item.typeId == component.item.id) >= component.quantity *
     quantity_requested`. (Python: lines 233-239.)
3. **`aQuantity`** — must be ≥ 1. Must be sanity-bounded (no `i32::MAX`
   that overflows the multiplication on the quantity-sufficient check).
   Must produce only as many outputs as the recipe permits, server-side.

If Phase 2 ships without ALL of these, the exploits are immediate:

- **Dupe via friend's item_id**: pass an item_id you don't own; the
  server "consumes" it and produces the output. The friend still has
  the item (it was never in your row); you have a free crafted item.
- **Dupe via equipped item_id**: pass your own equipped weapon's
  item_id; if the bag-location check is missing, you "consume" the
  weapon for crafting AND still have it equipped (the equipment slot
  reference doesn't decrement). Free output + weapon retained.
- **Recipe-bypass via wrong type**: if the type check is missing,
  pass low-tier scrap item_ids against a high-tier blueprint; you
  get the high-tier output for the cheap materials.
- **Quantity-bypass**: pass `aQuantity = 999999`; if the
  type-sufficient check uses int multiplication without overflow
  guard or doesn't recompute the deduction, you produce 999999 outputs
  while consuming the items only once.
- **Forbidden-blueprint bypass**: pass any `aCraftId`; if the
  blueprint-known check is missing, you craft items you never learned
  the recipe for.

Filing this now because the brief explicitly asks "Can client provide
arbitrary material item_ids that they don't own?" and "Can client claim
a recipe they don't know?" — the answer to both today is "the handler
is a stub, so trivially no; the answer post-Phase-2 depends on whether
the implementer remembers all four item-id checks."

The Rust persistence layer is partially in place: `blueprint_ids` is
loaded into in-memory `CraftingState` (see
`crates/services/src/base/crafting/persistence.rs:39`), and
`sgw_inventory` rows include `bag_id`; the data is available. The risk
is the cell→base coordination: cell must lock the item rows it intends
to consume (`FOR UPDATE` against `sgw_inventory`) BEFORE running the
type-and-quantity-sufficient check, and the produce-output INSERT must
be in the same transaction as the consume-input UPDATE/DELETE. A
disconnect or concurrent request between read and write is a dupe
window.

**Evidence**
- Ghidra: `00e47b10` `Event_NetOut_Craft` emit site — wire field order
  is aCraftId (INT32), aItems (array of INT32 item_ids), aQuantity (INT32).
- Ghidra: `00aaa370` — Lua binding `requestCraftBlueprintProduct` that
  invokes the emit (proves the client-side path).
- Def file: `entities/defs/SGWPlayer.def:921-926`.
- Python ref (full validation chain): `deprecated/python/cell/Crafter.py:191-254`.
- Resources schema: `db/resources/Entities/Tables/blueprints.sql`,
  `db/resources/Entities/Tables/blueprints_components.sql` — server has
  the data needed for recipe validation.
- Cross-ref to Rust stub: `crates/services/src/cell/cell_methods/player/crafting.rs:45-57`.

**Attack scenario** (post-Phase-2, assuming the implementer misses item-ownership)
1. Attacker is in a party with a friend whose inventory the attacker
   has scrolled past (item_ids leak in `onUpdateItem` broadcasts to
   nearby clients in some flows — verify against CAT-D findings).
2. Attacker sends `craft(aCraftId=<known blueprint>, aItems=[friend's
   item_id, friend's item_id, ...], aQuantity=1)`.
3. Server (missing ownership check) finds the item_ids in `sgw_inventory`,
   confirms types match the blueprint, and runs the consume. The
   DELETE/UPDATE on `sgw_inventory` happens against the FRIEND's rows
   (since the cell didn't gate by player_id).
4. Attacker receives the crafted output; friend's inventory is silently
   drained. Observable effect: friend's items disappear; attacker gets
   free output.

**Suggested remediation (one line)**
Phase 2 implementer must validate (a) `aCraftId ∈
sgw_player.blueprint_ids` for `player_id`, (b) every `aItems` entry has
`sgw_inventory.player_id = $caller_player_id` AND `bag_id IN
(INV_Main, INV_Crafting)`, (c) the (type, quantity_sum) tuple satisfies
the `blueprints_components` requirement for some component_set_id, and
(d) consume + produce in a single `BEGIN…COMMIT` with `FOR UPDATE` on
all input rows. Reject (not partially-consume) on any failure.

**Would benefit from x64dbg trace?**
Yes — confirm the per-item `ItemID` wire encoding matches the
`ItemID = INT32` alias before Phase 2 lands; the .def says INT32 but
the alias.xml resolution should be confirmed end-to-end against an
actual `craft()` packet at runtime to avoid an off-by-N parse on the
array.

---

### CAT-F-04 — `research` and `reverseEngineer` will trust client-named item_ids; outcome must be server-rolled

**Severity**: Critical (latent — Phase 1 is no-op)
**Class**: Future-implementer trap — same as CAT-F-03 plus server-side RNG requirement
**Wire surface**: `Event_NetOut_Research` (method 97, `ItemID aItemId, ARRAY<ItemID> aKickers`), `Event_NetOut_ReverseEngineer` (method 98, `ItemID aItemId`)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical

**Trust violation**
Two related flows share the same client-named-item-id pattern as `craft`:

1. **`research(aItemId, aKickers[])`** — client says which item to
   research and which kicker items to add. Both consume on success.
   The expertise gain is a random roll: Python
   `deprecated/python/cell/Crafter.py:277-345` computes
   `chance = 100 - currentExpertise + 5 * len(kickers)` and rolls
   `random() * 100 < chance`. **This roll MUST happen server-side.** If
   any part of it is exposed to the client (or worse, the client tells
   the server "I rolled successfully"), the player gets free expertise
   on every research.
2. **`reverseEngineer(aItemId)`** — client says which item to RE.
   Server consumes the item and rolls a discovery quantity per
   component (Python lines 364-414). Same RNG concern.

For both: the consume-input check is the same gate set as `craft`
(ownership, bag location, item type, sufficient quantity), AND for
research the kickers must have `type.kicker == true` AND for both the
researched/RE'd item must have `type.researchable == true` /
`type.reverseEngineerable == true`. These are resources lookups, not
client-supplied.

Failure modes if Phase 2 ships a partial impl:
- Missing researchable/reverseEngineerable type flag → can RE any
  item, including ones the design intended as terminal (e.g. quest
  items, soulbound gear).
- Missing kicker `type.kicker` check → use random valuable items as
  kickers; they get consumed but they shouldn't have been eligible,
  inflating the success chance.
- Trusting any client-asserted RNG state (timestamp seed, "roll
  result" field) → guaranteed-success exploit.
- The Python `randint(0, len(disciplines))` and `randint(0,
  len(blueprints))` patterns are off-by-one bugs in the original
  (inclusive upper bound) — Rust implementers must use a half-open
  range like `rng.gen_range(0..n)`, not port the bug. Note this is
  cosmetic, not security-relevant, but it's a porting trap.

**Evidence**
- Def file: `entities/defs/SGWPlayer.def:928-937`.
- Python ref: `deprecated/python/cell/Crafter.py:277-345` (research),
  `364-414` (reverseEngineer).
- Cross-ref to Rust stub: `crates/services/src/cell/cell_methods/player/crafting.rs:59-67`.

**Attack scenario** (post-Phase-2, missing researchable check)
1. Attacker sends `research(aItemId=<quest item id>, aKickers=[])`.
2. Server (missing researchable type check) finds the item, confirms
   ownership, consumes it.
3. Observable effect: quest item destroyed (potentially blocking the
   player's own progression as collateral damage, OR — if the RE roll
   returns components for a quest item — letting them turn quest items
   into reverse-engineered "blueprints" they shouldn't get).

**Suggested remediation (one line)**
Same item-ownership/bag/type/quantity guards as CAT-F-03, plus
`item.type.researchable` / `item.type.kicker` /
`item.type.reverseEngineerable` resource-table checks; the outcome
roll must use a server-side RNG seeded from non-client state, with
no client-supplied "seed" or "result" field accepted from the wire.

**Would benefit from x64dbg trace?**
No — the wire shape is fully documented in the .def and
straightforward. The risk is server-side validation completeness, not
wire decoding.

---

### CAT-F-05 — `alloying` will trust client-supplied currentTier + lowerTier item_ids and tier metadata

**Severity**: Critical (latent — Phase 1 is no-op)
**Class**: Future-implementer trap — multi-item recipe with tier-progression rules
**Wire surface**: `Event_NetOut_Alloy` (cell method 99, payload `INT32 aCraftId, ItemID aCurrentTierItemId, ARRAY<ItemID> aLowerTierItems`)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical

**Trust violation**
Alloy is craft's cousin but with two distinct material inputs:
- `aCurrentTierItemId` (single ItemID) — the "primary" component, must
  match the alloy blueprint's expected current-tier type.
- `aLowerTierItems` (ARRAY<ItemID>) — N "elementary" components from
  the tier *below* the current item's tier, with `N` determined by the
  elementary items' quality (`Constants.ALLOYING_ELEMENTARY_COUNTS[quality]`
  in the Python at `Crafter.py:485`).

The Python validation chain (`Crafter.py:431-505`) is the longest in
this category and is the most likely to be partially-ported:

1. Blueprint known (`blueprintId in self.blueprints`) — server-side
   ownership check on blueprints.
2. Blueprint is an alloy blueprint (`blueprint.alloy is True`) —
   prevents using a crafting blueprint id in this RPC and bypassing
   the alloying-specific tier rules.
3. All items exist and are owned by the player (current-tier item +
   every elementary).
4. All items are in main or crafting bag.
5. Current-tier item type matches the blueprint's single-component
   requirement (line 469).
6. Current-tier item quantity >= requirement quantity.
7. Elementary count matches `ALLOYING_ELEMENTARY_COUNTS[quality]` for
   the elementary's quality (line 484-489) — note: quality is taken
   from the **first elementary's type**, not from a client field,
   which is good — but the implementer must take it from a
   server-side type lookup, NOT from any client-supplied "tier"
   field.
8. Every elementary has `type.tier == currentTier.type.tier - 1`
   (line 493-497) — server-side type lookup again, not client-asserted.

Failure shapes if Phase 2 lands without these:
- Skipping check #2 (`blueprint.alloy`) lets a client send a craft
  blueprint id to `alloying()` and bypass the tier-decrement chain
  entirely, "alloying" any blueprint without the elementary-tier
  cost.
- Skipping check #5/#6 lets the client substitute the wrong primary
  type as long as it's some item they own.
- Skipping check #7/#8 lets the client send arbitrary "elementary"
  items (e.g. low-quality stack components for a high-tier alloy)
  and produce the high-tier alloy for cheap.
- Same dupe-via-friend's-item_id concern as CAT-F-03 if ownership is
  missed.

The Python source itself contains a real bug at line 469: `if
requirement['item'].id != component.typeId is None` is a precedence
error (`x != y is None` parses as `(x != y) and (y is None)`) that
silently always returns False, meaning the original Python *never
actually validated* current-tier type. The Rust port must implement
the check correctly (compare type ids directly), not faithfully port
the bug. Filing this as a watch-out because a "match Python behavior"
approach would inherit the broken check.

**Evidence**
- Def file: `entities/defs/SGWPlayer.def:939-944`.
- Ghidra: `Event_NetOut_Alloy` registration at `019db404` (`.PBV..._VEvent_NetOut_Alloy_…`
  vftable at `01e2c55c` confirms the typed payload class exists).
- Python ref: `deprecated/python/cell/Crafter.py:431-513`. Note the
  precedence bug at 469 + 473 that the Rust port MUST fix, not copy.
- Cross-ref to Rust stub: `crates/services/src/cell/cell_methods/player/crafting.rs:69-81`.

**Attack scenario** (post-Phase-2 missing the `blueprint.alloy` flag check)
1. Attacker has any blueprint known — including non-alloy ones.
2. Attacker sends `alloying(aCraftId=<non-alloy blueprint>,
   aCurrentTierItemId=<cheap item they own>, aLowerTierItems=[])`.
3. Server (missing alloy-flag check) treats the request as valid,
   consumes the cheap current-tier item, produces the non-alloy
   blueprint's output — completely bypassing the elementary-tier
   material cost the non-alloy blueprint normally requires.

**Suggested remediation (one line)**
Phase 2 implementer must implement all eight Python-style guards with
the `blueprint.alloy` check as #2 (a `WHERE blueprint_id = $1 AND
is_alloy = true` against `resources.blueprints`), and tier comparison
against server-side `resources.item_list_items.tier`, NOT any
client-supplied tier field; consume + produce in one transaction.

**Would benefit from x64dbg trace?**
Yes — the elementary count is dynamic (`ALLOYING_ELEMENTARY_COUNTS[quality]`)
and an x64dbg trace would confirm the actual array length the client
emits at runtime, which is helpful for the receiver's bound check on
parse.

---

### CAT-F-06 — Phase-1 stubs return `true` (handled) for every craft RPC with no idempotency guard

**Severity**: Low (latent infrastructure risk)
**Class**: Wire-shape acceptance without state-change — silent drop today, partial-state risk on Phase 2
**Wire surface**: All five crafting RPCs: methods 95–100
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (Phase 2 risk; documented for the implementer)

**Trust violation**
Today, the dispatch arms at
`crates/services/src/cell/cell_methods/player/crafting.rs:23-86` all
return `true` (handled) after logging "UNIMPLEMENTED". The client
treats `true` as "the server accepted the call"; the absence of a
follow-up `onUpdateDiscipline` / `onUpdateItem` / `onCraftingStarted`
is the only signal the client gets that nothing happened.

That's not exploitable today (no state changes), but it does mean
the dispatch IS wired and the message IS being accepted. The Phase-2
risk is: an implementer adds a partial side-effect — e.g. sets
`craftingBlueprintId` on the entity, or starts a `craftTimer`, or
emits `onCraftingStarted` to the client — *before* completing all
the ownership/recipe/quantity validation. If the validation fails
*after* that partial mutation, the player is "stuck in busy" (no
output, but the busy state preventing further crafts), or worse,
the in-memory mutation desyncs from the DB.

The right shape is: every craft RPC's first commit-point must be the
atomic DB transaction that consumes the materials AND produces the
output AND updates the in-memory crafting state — no in-memory or
in-flight mutation before validation completes.

Additionally, the `respecCrafting` (method 100) stub at line 83
accepts the message with zero validation; when Phase 2 lands, the
implementer must validate the player has enough naquadah for the
respec cost (`onCraftingRespecPrompt CostToRespec`) BEFORE clearing
disciplines, and the deduct + clear must be atomic.

**Evidence**
- All five stub arms: `crates/services/src/cell/cell_methods/player/crafting.rs:23-86`.
- All return `true` (line 42, 56, 61, 66, 80, 85) — dispatcher treats
  this as "handled".
- Python ref for `respecCrafting`: `deprecated/python/cell/SGWPlayer.py`
  (search `respecCrafting` — confirms the naquadah deduct + clear-disciplines
  flow).

**Attack scenario** (Phase 2 partial-mutation shape, hypothetical)
1. Phase 2 implementer adds `craftingBlueprintId = aCraftId` and
   `entity.busy = true` at the top of the `craft` handler — before
   running the material-ownership check.
2. Attacker sends `craft(aCraftId=<known>, aItems=[<non-owned ids>],
   aQuantity=1)`.
3. The ownership check fails, the handler returns. But `busy = true`
   was already set; no rollback. The player is locked out of crafting
   until disconnect.
4. Observable effect: griefing self-DoS, or — if combined with a
   forced disconnect — abandoned `busy = true` rows that survive
   reconnect.

**Suggested remediation (one line)**
Phase 2 implementers: validation must complete BEFORE any
in-memory mutation, busy-flag set, or `onCraftingStarted` emit; all
state changes (consume input, produce output, set timer, update
in-memory state) live in one atomic block.

**Would benefit from x64dbg trace?**
No.

---

### CAT-F-07 — No "busy" / induction-timer state tracked in Rust today

**Severity**: Medium (latent — emerges with Phase 2)
**Class**: Missing server-tracked rate limit — concurrent-request dupe via missing busy flag
**Wire surface**: All craft / research / alloying RPCs (methods 96, 97, 98, 99)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical

**Trust violation**
The Python reference decorates `craft`/`research`/`reverseEngineer`/
`alloying` with `@mustBeIdle`, which checks
`deprecated/python/cell/SGWBeing.py:25` for `self.busy == False`
before invoking the body. The `busy` flag is set when a crafting
induction timer (3-second wait) starts and cleared on completion.

The Rust port has no `busy` field on the entity, no induction-timer
infrastructure, and no equivalent gate. When Phase 2 lands the
mutation logic, two concurrent or rapidly-replayed craft packets will
both pass the ownership + recipe checks and both consume materials.
If they target different stacks, the player consumes 2× the
materials. If they target the same stack via the same item_id, the
DB row-locking will save them, but the in-memory state will desync
(both fired `onCraftingStarted` to the client).

Worse: research and reverseEngineer roll their outcome randomly.
Without a busy gate, an attacker can fire 100 concurrent `research`
packets on the same item_id (the DB DELETE will succeed only once,
but if validation isn't ordered correctly, an attacker can get
multiple `onExpertiseGained` rolls per consumed item) — the exact
shape depends on the implementation but the absence of a per-player
"one craft at a time" gate is the root.

**Evidence**
- Python ref: `deprecated/python/cell/SGWBeing.py:25-38` (`mustBeIdle`
  decorator definition) and `deprecated/python/cell/SGWPlayer.py`
  (lines 1620-1641 — `@mustBeIdle` applied to all four craft RPCs).
- Rust audit (no `busy` field):
  `crates/services/src/cell/space_manager/mod.rs` and the entity
  struct have no `busy: bool` or `craft_timer: Option<…>`. Confirmed
  via `grep -i "pub busy" crates/services` returning no matches.

**Attack scenario**
1. Attacker has materials sufficient for ONE craft of blueprint X.
2. Attacker sends 100 `craft(aCraftId=X, ...)` packets in quick
   succession (replay tool or scripted client).
3. Phase 2 impl (missing busy gate) validates each packet against
   in-memory inventory at packet-receive time. If the DB consume
   uses `WHERE quantity >= $needed` row-locks correctly, only one
   succeeds — but a partial impl that decrements in-memory state
   before issuing the DB DELETE could see the in-memory count drop
   to 0 between the validation and the DELETE for 99 of the 100
   packets, each of which "produces" the output before the DB
   reflects the consume.
4. Observable effect: depending on the impl gap, attacker either
   gets multiple craft outputs for one material set, or the
   in-memory/DB inventories desync until next relog.

**Suggested remediation (one line)**
Add a per-entity `crafting_busy: Option<CraftTimer>` field (parallel
to Python's `busy + craftTimer`) set at the top of the validated
handler (after material lock, before output emit) and cleared by the
timer-completion callback; reject any `craft`/`research`/`alloying`
RPC where `crafting_busy.is_some()`.

**Would benefit from x64dbg trace?**
No — this is a server-side state-machine design gap.

---

### CAT-F-08 — TrainAbility cell-side accepts request even when player_level field is stale

**Severity**: Low
**Class**: TOCTOU / stale-cache risk on a non-critical validation
**Wire surface**: `Event_NetOut_TrainAbility` (method 77)
**Demonstrable / Likely-theoretical**: Likely-exploitable theoretical (narrow window; depends on grant_xp ordering)

**Trust violation**
The cell-side level check at
`crates/services/src/cell/cell_methods/player/vendor.rs:583` reads
`entity.level as i32` from in-memory cell state. The base-side
progression handler at
`crates/services/src/base/world_entry/methods/progression/mod.rs:163`
updates `state.player_level` AFTER the DB persist succeeds, and the
cell receives `BaseToCellMsg::LevelUpdate` separately.

There's a narrow window where:
- A `grant_xp` is in-flight in the base→cell→DB pipeline.
- The cell's `entity.level` is the OLD (pre-level-up) value because
  the `BaseToCellMsg::LevelUpdate` hasn't been processed yet.
- A `trainAbility` arrives for an ability that requires the NEW
  level.
- The cell rejects with "level too low" even though the player has,
  authoritatively, just gained the level.

This is *not* an attack vector — it just rejects a legit train
attempt — but the inverse is the concern: if the cell→base sequence
for level decreases (death penalty, respec, GM `SetLevel`) updates
in-memory level AFTER the cell already accepted a `trainAbility`,
the cell might forward a TrainAbility for an ability that the
player *no longer qualifies for*. The base re-validates only
`training_points > 0`, not level. So a respec that decreases level
mid-flight could let a TrainAbility for a now-too-high ability
slip through.

This is narrow enough that it's a low-severity informational, not
an exploit shape — but it should be filed because the fix is cheap
(re-validate level on the base side, parallel to the existing
`training_points > 0` gate).

**Evidence**
- Cell-side level read: `crates/services/src/cell/cell_methods/player/vendor.rs:583`.
- Base-side handler with no level re-check:
  `crates/services/src/base/world_entry/methods/progression/mod.rs:400`
  (no `WHERE level >=` clause in the UPDATE).
- Resources table `archetype_ability_tree` has `level` per entry,
  so the base could JOIN to re-validate.

**Attack scenario**
1. GM (or a respec flow) reduces the player's level from 10 to 5.
2. The cell's in-memory `entity.level` is still 10 because the
   level-update message hasn't reached the cell yet (or the cell
   has crashed and is mid-recovery).
3. Attacker (or any client) sends `trainAbility(<requires_level_8>)`.
4. Cell validates 10 >= 8, forwards `CellToBaseMsg::TrainAbility`.
5. Base validates `training_points > 0` AND `NOT abilities @>
   [ability_id]` — both pass — and grants the ability.
6. Observable effect: player now has an ability they don't qualify
   for at their current level.

**Suggested remediation (one line)**
In `handle_train_ability` at
`crates/services/src/base/world_entry/methods/progression/mod.rs:456`,
join against `resources.archetype_ability_tree` and add `AND level <=
(SELECT level FROM sgw_player WHERE player_id = $2)` (or pre-fetch
the required level and pass it through the `CellToBaseMsg::TrainAbility`
variant for the base to compare against the row).

**Would benefit from x64dbg trace?**
No.

---

## Not Filed

- **"Stub returns true" considered as Critical** — the brief asks for
  exploit-shaped findings; today's stubs don't mutate state, so the
  "client thinks it worked but server did nothing" desync is a UX
  bug, not an exploit. Documented as CAT-F-06 at Low for the Phase
  2 implementer; not duplicated as a separate Critical.
- **Python `randint(0, len(list))` off-by-one** — original Python
  research/RE has `randint(0, len(disciplines))` (inclusive upper
  bound) — a porting trap rather than an exploit. Flagged inline in
  CAT-F-04; not a standalone finding.
- **`respecCrafting` cost validation** — wire payload is empty
  (`<respecCrafting><Exposed/></respecCrafting>`); cost comes from
  `onCraftingRespecPrompt` server→client and is a server-side
  resources lookup. Today the handler is a no-op stub; future
  implementer's risk is captured in CAT-F-06 (validation-before-mutation
  discipline). Not a separate finding.
- **`onUpdateDiscipline` / `onUpdateKnownCrafts` wire shape** — these
  are server→client, not in our adversarial-input scope. The
  generated decoders exist at
  `crates/services/src/wire_log/decoders/generated.rs:1327` for
  observability only.
- **`GiveBlueprint` / `GiveAppliedSciencePoints` / `GiveTrainingPoints`
  / `GiveRacialParadigmLevels` / `GiveExpertise` GM commands** — these
  are in CAT-N's scope (GM debug commands); already enumerated in the
  surface inventory under CAT-N. None are wired in Rust today; their
  authentication will be audited under CAT-N.
- **`gainAppliedSciencePoints` / `updateCraftingFlags` BASE-side methods**
  — these are server-internal RPCs invoked by content-engine actions
  (level-up, mission rewards), not client-facing. Trust-boundary
  analysis is internal-RPC scope, not adversarial-client scope.
- **Concurrency safety of `CraftingState` load/save (existing
  persistence layer)** — the `load_crafting_state` / `save_crafting_state`
  in `crates/services/src/base/crafting/persistence.rs` are already
  transactional and use `RowNotFound` to guard silent-no-op saves.
  No exploit shape applies until a writer calls them (Phase 2).
- **Wire-format truncation handling** — the stubs check `args.len() >=
  4` before parsing. The error path is `warn!` + `return true`. A
  client that sends 0 bytes on craft (msg 96) gets a warn-log but no
  exploit shape today; future Phase 2 must keep the length check as
  the first gate, not the last.
- **Cell `_space_mgr` parameter is `&mut` but unused (`_space_mgr`)**
  in the stub. This is fine for Phase 1; Phase 2 will use it. Not a
  finding.
