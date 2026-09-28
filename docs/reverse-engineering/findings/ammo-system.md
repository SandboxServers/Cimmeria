---
type: reference
audience: Contributors implementing the ammo campaign (issue #1026); domain advisors items-systems-advisor and combat-systems-advisor
last_updated: 2026-09-28
companion_docs:
  - weapon-ammo-pipeline.md
  - cooked-data-pipeline.md
  - client-wire-emit-suppression.md
  - ../../gameplay/weapon-ammo-reload.md
  - ../../gameplay/inventory-system.md
---

# Ammo System — RE Findings (AM-01)

**Campaign**: Ammo campaign, issue #1026
**Date**: 2026-09-28
**Analyst**: Game Archaeology Specialist (packet AM-01)
**Confidence**: Mixed — see per-question tables; HIGH where a live decompile or a seeded DB row backs the
claim, MEDIUM where only naming-convention or schema-consistency evidence exists, and one confirmed
absence (no reserve field anywhere in the client schema).

---

## Summary

This packet answers the five RE questions from #1026 before the ammo campaign designs the reserve
system. The headline finding: **the 2009 client schema has no working notion of a finite ammo
reserve** — no per-type count, no ammo-item category, nothing a corpse or a bag could hold. The only
per-player ammo-adjacent property is `knownAmmoTypes`, an `ARRAY<INT32>` discovery/unlock flag list
(already documented as such in `docs/gameplay/inventory-system.md:65`), not a quantity. `/gmgiveammo`
and the `bInfiniteAmmo` debug toggle are GM/debug scaffolding for a mechanic whose consumption side
left no trace in the client. **D-AM01 (items in bags) is therefore a reconstruction, not a recovered
original mechanic** — it fits cleanly into the existing Item/InvItem pipeline (no new wire structures
needed), but nothing in the binary confirms that items-in-bags is what SGW originally shipped or
intended.

On the encouraging side: `getAmmoTypes`, `getCurrentAmmoType`, and `requestAmmoChange` all resolve
through the *same live, server-populated bandolier/container cache* already identified in
`client-wire-emit-suppression.md`, not a static `CookedDataItems.pak` table. Widening a weapon's
`ammo_types` list is confirmed to be a pure DB/server-side change — no cooked-data patch required.
This directly de-risks Acceptance Criterion 1 and the TODO at `crates/entity/src/inventory.rs:81`.

---

## Q1 — The original reserve model

### What `/gmgiveammo <ammoId> <quantity>` sends

| Evidence | Address / source | Confidence |
|---|---|---|
| `/gmgiveammo` is slash-command id `0x69` (105) in the console command registry | `docs/reverse-engineering/decompiled/14_standalone_named.c:46169` (`BW__unknown_00438c40(auStack_c14,"Event_SlashCmd_GiveAmmo")`, `uStack_4 = 0x69`) | HIGH |
| `Event_SlashCmd_GiveAmmo` has its own vtable and a dedicated `vfunc_2` body at `FUN_00593490` | `docs/reverse-engineering/decompiled/06_game_events.c:4657-4665`; live decompile of `FUN_00593490` (headless Ghidra, this session) | HIGH |
| `FUN_00593490` is generic CME-emit boilerplate (allocates a 24-byte NetworkEvent shell, branches on a `NoSubject` vs typed `TypeDescriptor_01dab4cc` RTTI descriptor, calls the shared `FUN_00593380` constructor, then splices into a `MemberCallback`-style linked list at `this+0x14`) — it does **not** itself read `ammoId`/`quantity`; those are bound onto the event object earlier (the console-command tokenizer binds args to fields by name, per the same pattern `weapon-ammo-pipeline.md` §5 documented for `requestReload`'s `aReloadType`) | Live decompile (this session) | MEDIUM — the field-binding site itself was not traced this session (budget) |
| `Event_NetOut_GiveAmmo` is a registered CME NetOut event (`register_NetOut_GiveAmmo` returns the literal string) | `docs/reverse-engineering/decompiled/14_standalone_named.c:284152-284155`, `:294459` (registration id `0x10d`) | HIGH |
| `Event_NetOut_GiveAmmo` string address `019b3794` | `docs/analysis/event-net-mapping.md:724` | HIGH |
| **No `.def` entry exists for a `GiveAmmo` cell/base method anywhere under `entities/defs/`** | `grep -rn -i giveammo entities/defs/` — zero matches (this session) | HIGH (confirmed absence) |
| `GiveAmmo` appears only in the Mercury protocol category catalog, not in a per-method dispatch table | `docs/protocol/mercury-wire-format.md:673` ("Items" category list) | HIGH |

**Reading**: `GiveAmmo` is a real, registered NetOut CME event — the client can emit it — but it was
never wired to an entity method definition the way `requestReload` or `requestAmmoChange` were. This
matches `docs/commands.md`'s ❌ "Not yet" status for `/gmgiveammo` and `/gmsetinfiniteammo`: the debug
command and its wire event exist, but the developer-side receiver (an entity method with args) is
either missing from the recovered `.def` tree or was cut before the args were finalized. **The exact
byte layout of `Event_NetOut_GiveAmmo` (ammoId/quantity argument types and order) could not be
recovered this session** — the field-binding call site needs a further live trace (see Open Questions).

### Is `ammoId` an item/design id or an `EAmmoType`?

| Evidence | Source | Confidence |
|---|---|---|
| `EAmmoType` is a `UINT8` enum with 24 tokens (`AMMO_NONE`=0 .. `Dart_Adrenaline`=23) | `entities/defs/enumerations.xml:161-189` | HIGH |
| Every other ammo-type-carrying field in the schema (`SGWCombatant.currentAmmoType`, `InvItem.curAmmoType`, `InvItem.ammoTypes[]`) is an `EAmmoType`-keyed `INT32`, never an item design id | `entities/defs/interfaces/SGWCombatant.def:156-160`; `entities/defs/alias.xml` (`InvItem`); `weapon-ammo-pipeline.md` §2 | HIGH |
| `/gmgiveitem <designId> <quantity>` uses "designId" naming; `/gmgiveammo <ammoId> <quantity>` deliberately uses a different name | `docs/commands.md:255,257` | MEDIUM (naming-convention inference) |

**Conclusion (MEDIUM-HIGH)**: `ammoId` is almost certainly an `EAmmoType` enum value (0-23), not an
item/design id. The naming divergence from `GiveItem`'s `designId` is deliberate, and every other
ammo-type field in the schema uses the `EAmmoType` enum, never an item id. No decompiled evidence
directly reads the argument name off the wire this session, so this is schema-consistency inference,
not a confirmed byte read.

### Does the client have any notion of a reserve?

| Candidate | Found? | Evidence |
|---|---|---|
| Per-type ammo count/pool field | **No** — confirmed absence | Full-text search of `entities/defs/` for `ammoreserve\|ammocount\|ammostock\|reserveammo` found only `CONDITION_FEEDBACK_AmmoCount{Not}Equal\|GreaterThan\|LessThan` (`entities/defs/enumerations.xml:1266-1271`) — ability *effect-condition* comparators against the **clip** count, not a bag/reserve count (comment cites ability ids 1020, 1022) |
| Ammo items in inventory | **No dedicated category found** | No "ammo item" item-type/category distinct from ordinary `Item` rows was found in `entities/defs/` or the items seed structure; of 595+19 weapon rows only item 1874 ("tester") lists a second `EAmmoType` (per #1026's own recon) |
| A stat exposing the reserve | **No** | `AMMO_SLOT_{1,2,3}` stats (`weapon-ammo-pipeline.md` §4) are the **clip**, `min=0 cur=current_ammo max=clip_size` — no analogous reserve stat exists |
| A UI element showing the reserve | **No** | `Bandolier.lua`'s picker (per #1026) shows available *types*, not a per-type *count*; no ammo-reserve counter widget was found |
| `knownAmmoTypes` — the one per-player ammo-adjacent property | **Exists, but is not a reserve** | `entities/defs/interfaces/SGWInventoryManager.def:67-72`: `ARRAY<INT32>`, default `[]`, `CELL_PRIVATE`. Already characterized in `docs/gameplay/inventory-system.md:65` as "Discovered ammo types" — a per-type *unlock/discovery flag* list, not a counted pool. `docs/drafts/spec/entity-property-sync.md:1312` and `docs/engine/entity-type-catalog.md:877` independently confirm the same field/type. |
| `bInfiniteAmmo` debug toggle | Exists | `entities/defs/interfaces/SGWAbilityManager.def:74-79`, `CELL_PUBLIC`, `INT8` — a GM/debug toggle, not evidence of the consumption mechanic itself |

**Conclusion (HIGH confidence in the absence)**: nothing in the recovered client schema implements a
finite ammo reserve, as either a hidden per-type pool or a dedicated ammo-item category. `knownAmmoTypes`
is the closest analog and it is a discovery flag, not a count. The legacy Python tree and
`deprecated/python/` were also searched for a `GiveAmmo`/`give_ammo` handler — zero matches.

**What this means for D-AM01**: the decision to model special ammo as stackable bag items is a
**restoration-team design choice**, not a recovery of a documented original mechanic. It is a sound
choice — it needs zero new wire structures, reusing the existing `Item`/`InvItem`/loot pipeline — but
the campaign should record it as a deviation from (or rather, a completion of) an incomplete original
design, not as "restoring what SGW had."

---

## Q2 — Where the client's allowed-ammo-type list comes from

**Answer: a live, server-populated per-player cache — not `CookedDataItems.pak`.**

Three native functions behind the Lua bindings all resolve through the identical pointer chain
`<accessor @ 0x00c66ad0>() → [+0x8c] → +0x24`:

| Lua binding | Native function | Address | What it does |
|---|---|---|---|
| `getAmmoTypes(Container, slot)` | `FUN_00add4a0` | `0x00add4a0` | Looks up an entry in the map at `+0x24` (guarded `std::map`-style range checks against `+0x28`/`+0x2c`), returning an `HVSystemOptionPolicyEnum`-wrapped option list | Live decompile, this session |
| `requestAmmoChange(ItemId, AmmoType, …)` | `FUN_00ad8ee0` | `0x00ad8ee0` | Calls `FUN_00e1f4e0` against the same base pointer to send the wire request | Live decompile, this session |
| `getCurrentAmmoType(Container, slot)` | `FUN_00ad8f10` | `0x00ad8f10` | Same map lookup via `FUN_00e1c530`, then reads offset `+0x34` of the found entry | Live decompile, this session |

`docs/reverse-engineering/findings/client-wire-emit-suppression.md:92` independently documents a
fourth function, `FUN_00ad8ad0`, walking "the bandolier container map at `SGWPlayer+0x8c → *+0x24`"
to fetch the cached active slot index (offset `+0xc`) — the **same base offset chain**. Four
independently-decompiled native functions converging on the identical `+0x8c → +0x24` map is strong,
cross-corroborated evidence that this is one shared, live, server-driven per-player container/item
cache, not a static cooked-data table lookup.

**Confidence: HIGH** that the source is live/server-driven, not cooked data. **MEDIUM** on the exact
key (container id vs. item id) — the Lua wrappers for `getAmmoTypes`/`getCurrentAmmoType` each convert
two Lua arguments but the decompiler shows only the first converted value reaching the native call;
this may be a real 1-argument native signature or a decompiler artifact from MSVC's calling-convention
handling of chained conversions (see Open Questions).

**Practical conclusion for the campaign**: the per-item `ammoTypes[]` array the picker reads is the
wire property `InvItem.ammoTypes` (`entities/defs/alias.xml`), which on Cimmeria's server already
comes from the `Item.ammo_types` DB column (`db/resources/Items/Seed/items.sql`). **Widening a
weapon's allowed ammo types is a pure DB/server-side change.** No `CookedDataItems.pak` patch is
needed, which directly de-risks Acceptance Criterion 1 and the TODO at
`crates/entity/src/inventory.rs:81`.

---

## Q3 — How ammo type and toggle abilities connect

| Evidence | Source | Confidence |
|---|---|---|
| Ability 715 ("Hollow Point Ammunition"): `type_id='ABILITY_TYPE_Buff'`, description "Buff: Toggle — Damage Type: Physical, Penetration: Decreased, Damage: Increased", `passive_yn=true`, `cooldown=0`, `effect_ids='{}'` (empty) | `db/resources/Abilities/Seed/abilities.sql:4730` | HIGH (DB row, matches #1026's own citation) |
| Ability 719 ("Armor Piercing Ammunition"): same shape, `passive_yn=false`, `cooldown=30`, `effect_ids='{747}'` | `db/resources/Abilities/Seed/abilities.sql:4745` | HIGH |
| `GENERICPROPERTY_AmmoTypeId` (propId 3) drives only the client's ammo-indicator UI; no combat/damage code path reads it | `weapon-ammo-pipeline.md` §1, §6 | HIGH (pre-existing finding) |
| `requestAmmoChange` and `UseAbility`/toggle-ability activation are separate wire paths; nothing decompiled this session shows the client or a server handler auto-invoking `UseAbility(715)`/`UseAbility(719)` when `requestAmmoChange` selects that type | Live decompile (this session, `FUN_00ad8ee0`) + `weapon-ammo-pipeline.md` §5-6 | MEDIUM (confirmed independence of the two wire paths; did not find and rule out every possible server-side auto-trigger, since the original server is not recoverable) |

**Conclusion**: `requestAmmoChange` (ammo-type selection / clip loading) and toggle abilities 715/719
(damage-modifier buffs) are architecturally **independent** systems in the client. Selecting Hollow
Point ammo does not appear to automatically toggle ability 715 in the traced native path, and
`GENERICPROPERTY_AmmoTypeId` only drives the UI icon. **Acceptance Criterion 4 (damage change through
the toggle ability's effect) is therefore a design decision for the campaign to make explicitly** —
e.g., the server engaging/disengaging the matching toggle ability in lockstep with `requestAmmoChange`
— not a recovery of hidden original behavior, since no such automatic linkage was found. Ability 715's
empty `effect_ids` confirms #1026's framing that its damage/penetration effect needs reconstruction
from the description text, matching 719's effect 747 as the template.

---

## Q4 — New item ids feasibility

| Evidence | Source | Confidence |
|---|---|---|
| Cooked-data elements are fetched by numeric key on demand (`versionInfoRequest` → `onVersionInfo` → `InvalidKeys` → `elementDataRequest` → `resourceFragment`), with a client-writable local PAK cache keyed by the same numeric key | `docs/reverse-engineering/findings/cooked-data-pipeline.md` Findings 4, 6, 7 | HIGH (pre-existing, live-decompile-sourced finding) |
| The element key is a `long` end-to-end with no narrowing; the ZIP cache entry name is built by plain decimal-digit streaming, not a bounds-checked static table index | `docs/reverse-engineering/findings/cooked-dialog-override-crash.md` (same mechanism, category 5; corroborates category 4's `CookedDataItems`) | HIGH (pre-existing finding, different category but same `ServerSource<N>` template) |
| Production precedent: PR #405 pushed **modified** item element XML for existing ids 2893/4735 (icon + `max_stack_size`) via this exact handshake, with zero client patch, and it worked in-game | `docs/reverse-engineering/findings/cooked-data-pipeline.md` (PR #405 cited); `crates/services/src/base/item_overrides.rs` | HIGH (proven in production) |

**Conclusion (MEDIUM-HIGH)**: architecturally, the client does not validate a requested element key
against any compiled/static id range — it just asks the server for whatever key it's told is stale or
missing. This strongly suggests **brand-new item ids** (not just modified existing ones) would work the
same way. However, the only *empirically proven* case is PR #405's override of **existing** ids;
nobody has verified a wholly novel id end-to-end. Recommend the campaign validate a genuinely new item
id (e.g., a new "Hollow Point Rounds" ammo item) with the same low-risk PAK-override pattern before
depending on it structurally — but no blocker was found.

---

## Q5 — Client-side gates on reload or ammo picking

| Path | Gate found? | Evidence |
|---|---|---|
| Reload (`requestReload`) | **No** — unconditional emit once triggered | `weapon-ammo-pipeline.md` §5: both emitter functions fire unconditionally; no Lua-side cached-state guard documented |
| Ammo-type picking (`requestAmmoChange`) | **No** — unconditional emit once Lua arg-count validation passes | Live decompile this session (`CEGUI__unknown_00aa7e40` / `FUN_00ad8ee0`): the Lua wrapper checks argument count/type only, no local-state short-circuit comparable to the bandolier-slot-swap case |
| Bandolier **active-slot swap** (F1–F4) | **Yes** — a documented, different gate | `client-wire-emit-suppression.md` §"Failure 2": `BandolierMod.ActivateBandolierSlotN` guards on `getActiveSlotForContainer(containerId) ~= N` and no-ops `requestActiveSlotChange` if the Lua-cached active slot already matches |

**Conclusion**: no client-side suppression was found gating reload presses or ammo-type picker
selections specifically — both wire calls fire unconditionally once basic Lua argument validation
passes, matching `weapon-ammo-pipeline.md`'s existing characterization of `requestReload`. The one
confirmed suppression risk in this neighborhood is the **bandolier active-slot swap** no-op
(`client-wire-emit-suppression.md`), a related but distinct action. Flagging this as a caution for the
campaign's UI testing (AC1-3): if a tester swaps weapon slots via F-keys immediately before or after
picking an ammo type, a stale client-cached active-slot value could make the visible effect of the pick
appear to not apply, even though `requestAmmoChange` itself has no such gate.

---

## Open Questions

1. **`Event_NetOut_GiveAmmo`'s exact wire byte layout** (arg types/order for ammoId, quantity) was not
   recovered — the console-command-to-event field-binding call site (analogous to the `aReloadType`
   `SetField` pattern documented for `requestReload`) needs a further live trace from
   `FUN_00593490`'s caller context or the `Event_SlashCmd_GiveAmmo` constructor.
2. **`getAmmoTypes`/`getCurrentAmmoType`'s exact key** — container id, item id, or both (container+slot)
   — is ambiguous in the decompiled pseudocode (only one of two converted Lua arguments visibly
   reaches the native call). Needs disassembly-level (not decompiled-C-level) verification of the
   calling convention, or an x64dbg breakpoint trace of a live `getAmmoTypes` call.
3. **Whether a server-side `GiveAmmo` entity-method receiver ever existed** in a `.def` file that
   didn't survive into the recovered `entities/defs/` tree, or whether the wire event was registered
   speculatively and never finished. The absence is confirmed; the *reason* for the absence is not.
4. **Whether toggle abilities 715/719 were meant to auto-engage on `requestAmmoChange`** server-side in
   the original (now-unrecoverable) BigWorld base-app logic. The client shows no such automatic link,
   but client-side absence doesn't rule out original server-side behavior we have no binary for.

---

## Cross-reference targets

- `docs/gameplay/weapon-ammo-reload.md` — update with the Q1/Q3 findings (no reserve in the original;
  toggle abilities are independent of `requestAmmoChange`) once the campaign's reload/reserve-draw code
  lands.
- `docs/gameplay/inventory-system.md` — `knownAmmoTypes` row (line 65) is already correctly described;
  no change needed, but cross-link this finding from it.
- `docs/architecture/abilities-and-effects-system.md` — note ability 715's empty `effect_ids` and the
  need to reconstruct its Hollow Point damage/penetration modifier from the description text, mirroring
  719/effect 747.
- `docs/gap-analysis.md` / `docs/commands.md` — update `/gmgiveammo` and `/gmsetinfiniteammo` rows only
  at the campaign's close-out, once implemented (per CLAUDE.md status-doc cadence).

## Related documents

- [weapon-ammo-pipeline.md](weapon-ammo-pipeline.md) — the existing clip/reload/bandolier wire-format
  findings this doc builds on.
- [cooked-data-pipeline.md](cooked-data-pipeline.md) — the per-category element-fetch mechanism behind
  Q4.
- [client-wire-emit-suppression.md](client-wire-emit-suppression.md) — the bandolier active-slot-swap
  suppression and the `SGWPlayer+0x8c → *+0x24` container map this doc's Q2/Q5 findings corroborate.
