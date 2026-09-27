# Pet Client Contract — Wire Indices and Unit.PetN Binding

> **Date**: 2026-09-27
> **Phase**: Pets restoration campaign, packet PT-E1 (client contract, closing the static gaps)
> **Confidence**: HIGH (wire method indices; `Unit.Pet1..4` slot IDs; the ownerID-driven bind
> mechanism and its two-message gate); LOW (SpeedPet consumer — unresolved, same as prior pass)
> **Sources**: `SGW.exe` Ghidra (image base `00400000`); `entities/defs/SGWPet.def`,
> `entities/defs/enumerations.xml`; `docs/reverse-engineering/findings/pet-restoration.md`,
> `pet-wire-formats.md`, `dialog-portrait-lookup.md`; `docs/protocol/client-method-dispatch-table.md`;
> the pets-campaign research docs (`docs/analysis/pets/research/client-static-re.md`,
> `code-map.md`, on branch `docs/pets-campaign-plan`, not yet merged to `main`)
> **Tracking**: pets campaign packet PT-E1 (issue #570)

This finding closes the two open items PT-01 was blocked on: the SGWPet client method
wire indices, and the mechanism that binds an entity to `Unit.Pet1..Pet4` (and, transitively,
to the pet bar / pet info UI). Everything below is from static Ghidra analysis; no client was
run and no x64dbg was attached.

## 1. SGWPet client method wire indices — CONFIRMED

**`onPetAbilityList` = 29, `onPetStanceList` = 30, `onPetStanceUpdate` = 31.** All three
direct-encode (`msg_id = 0x80 | index`): `0x9D`, `0x9E`, `0x9F`.

`SGWPet.def` has no `<Implements>` block and declares exactly 3 `<ClientMethods>` in this
order: `onPetAbilityList`, `onPetStanceList`, `onPetStanceUpdate`. Per the BigWorld flattening
rule already validated for SGWPlayer (157/157 methods, zero mismatches) and SGWMob (NA33), a
leaf entity's own `<ClientMethods>` are appended, in document order, after the flattened
`<Implements>` + ancestor-own prefix. SGWPet's ancestor chain is `SGWEntity →
SGWSpawnableEntity → SGWBeing → SGWMob → SGWPet`, and the SGWMob table already published in
`client-method-dispatch-table.md` gives that prefix as indices 0-28 (29 methods total,
`Lootable` contributing none). SGWPet's own 3 methods therefore land at 29, 30, 31 — total 32
methods, matching `IDBASE_NPC_DEFAULT = 62` (`crates/mercury/src/channel_bundle/idbase.rs`) and
the entity-property-sync appendix B figure of 32 for SGWPet cited in `code-map.md §1.5`.

This is consistent with the handlers, though the handlers alone do not prove the numbers: the
three GamePet-side CME handlers for these methods carry the exact `.def` `<ArgName>` strings as
their internal property-list keys, which confirms *which* method each handler processes. The
numeric indices 29/30/31 still rest on the flattening rule and the SGWMob prefix; no
EntityDescription or dispatch mapping was read that would rule out a remap —

- `GamePet__OnPetAbilityListChanged` (`0x00d39eb0`, renamed this session; was `FUN_00d39eb0`)
  builds its temporary property list keyed on the literal string `"aAbilityList"`.
- `GamePet__OnPetStanceListChanged` (`0x00d3a070`; was `FUN_00d3a070`) keys on `"aStanceList"`.
- `GamePet__OnPetStanceUpdateChanged` (`0x00d3a260`; was `FUN_00d3a260`) keys on `"aStance"`.

These match `SGWPet.def`'s `<ArgName>` values (`aAbilityList`, `aStanceList`, `aStance`)
exactly, confirming these are GamePet's own 3 methods and that no additional inherited
method reordering happened between the SGWMob prefix and SGWPet's own block.

**Do not confuse client CME *registration* order with the wire index** — `pet-restoration.md`
previously listed "`onPetAbilityList` [client idx 1] / `onPetStanceList` [idx 0] /
`onPetStanceUpdate` [idx 2]" next to the registrar addresses `0x00d77720`/`0x00d779c0`/
`0x00d77c60`. Those are the order the client happens to construct/subscribe its `MemberCallback`
objects in (an implementation artifact), not the flattened wire method index. Corrected in
`pet-restoration.md` in this PR (see §4).

## 2. What fills `Unit.Pet1..Pet4` — CONFIRMED

### 2.1 The slot IDs

`Unit.Pet1`, `Pet2`, `Pet3`, `Pet4` are **GameEntityManager slots 10, 11, 12, 13** respectively.
This is confirmed two independent ways:

**(a) The Lua constant table.** The client registers `Unit.PetN` (and `Unit.Dialog`,
`Unit.DialogSpeaker`, `Unit.SquadN`, etc.) as plain Lua numeric constants via a long chain of
`RegisterLuaConstant(luaState, name, thunk, 0)` calls in the giant CEGUI/Lua-binding function
`CEGUI_ButtonBase_3` (`0x00acbb10`-`0x00ad6b65`). Each name's thunk is a tiny
`int→double` conversion stub (`CVTSI2SD`) reading one dword from a contiguous C array at
`0x01b160c0`+; the array index run for the four Pet thunks (`0x00a9fe80`, `0x00a9feb0`,
`0x00a9fee0`, `0x00a9ff10`) reads dwords at `0x01b160e8`/`ec`/`f0`/`f4`, whose **values** are
`10, 11, 12, 13` (read directly via `read_memory`). The table also carries `SquadMax=9` at
`0x01b160e4` immediately before Pet1, and `Dialog=17`/`DialogSpeaker=18` at `0x01b16108`/`10c`
several slots later (see the note in §4 — this contradicts a prior doc's slot-17 label for
`DialogSpeaker`).

**(b) Direct confirmation from the slot-bind loop itself.** `GamePet__SyncLocalOwnerPetSlots`
(`0x00d39880`; was `FUN_00d39880`) contains:

```c
iVar1 = 10;
do {
    iVar3 = GameBeing__GetPetIdAt(this, iVar1 - 10U);   // this->petIds[i], i = 0..3
    iVar2 = (iStack_24 <= (int)(iVar1 - 10U)) ? 0 : iVar3;
    HVSystemOptionPolicyEnum__unknown_00c67bd0(GameEntityManager, iVar1, iVar2);  // slot-map(slot=10..13, entityId)
    iVar1 = iVar1 + 1;
} while (iVar1 < 0xe);   // 10,11,12,13
```

`0x00c67bd0` is the same "GameEntityManager slot mapping" function already documented in
`dialog-portrait-lookup.md` (`FUN_00c67bd0(GameEntityManager, slot, entityId)` → stores
`entityId` at `slot`, emits `Event_UI_UnitMappingChanged(slot)`; the Lua/native UI layer then
reads `GameEntityManager[slot]` on that event to resolve `Unit.PetN`, `Unit.Dialog`, etc. to a
live unit handle). `this` in the loop above is the **local player's** `GameBeing*` (fetched via
the same `GameLocalPlayer` accessor pattern used elsewhere in this file), not the pet — the pet
is only the trigger.

`GameBeing__GetPetIdAt` (`0x00dffa80`; was `FUN_00dffa80`) is a bounds-checked read of a
`std::vector<int>`-shaped field at `GameBeing+0x144`..`+0x148` (begin/end pointers), returning
0 if the index is out of range. This is the client's own local, native array of the player's
current pet entity ids — **not derived from the wire at all**; it is populated by
`GameBeing__AddPetId`/`GameBeing__RemovePetId` (below), and `GamePet__SyncLocalOwnerPetSlots`
just projects up to 4 elements of it onto slots 10-13.

### 2.2 What populates that array — the ownerID property

`GamePet__OnOwnerIdChanged_ValuePushed` (`0x00d39a10`; was `FUN_00d39a10`) is a GamePet CME
handler gated on `*(uint*)(this+0x38) & 0x400` — bit `0x400` = 1024 =
`ENTITYFLAG_Pet` (`entities/defs/enumerations.xml`) — i.e. it only proceeds for entities that
carry the Pet flag. It reads an incoming value at `param_1+8` (the new ownerID) and:

- if `param_1+8 < 1` (no/invalid owner): resolves the *local player* mailbox and calls
  `GameBeing__RemovePetId(localPlayerBeing, this->entityId)`.
- if `param_1+8 >= 1` (a valid owner id): resolves that id through the entity manager,
  `dynamic_cast`s it to `GameBeing*` (RTTI-checked), and calls
  `GameBeing__AddPetId(ownerBeing, this->entityId)` — **only if the cast succeeds**, so a
  non-`GameBeing` owner id is silently ignored, not crashed on.

`GameBeing__AddPetId` (`0x00e007b0`) and `GameBeing__RemovePetId` (`0x00e00270`) are a linear
"insert-if-absent" / "find-and-erase" pair over the same `GameBeing+0x144..0x148` vector that
`GameBeing__GetPetIdAt` reads. This is the client's `GameBeing::AddPet(id)` /
`GameBeing::RemovePet(id)`.

A second handler, `LAB_00d39ae0` (an inline label inside `GamePet__SubscribeEvents`, not a
standalone function Ghidra could be given its own name), does the complementary "re-fetch"
side: gated on an event whose first field equals `4` (an int32-property-change type code) and
whose second field (`+4`) equals `this->entityId` (`this+0xc`), it unconditionally sets a
readiness byte `this+0x171 = 1`, then — if two other readiness bytes (`this+0x170`,
`this+0x172`) are also non-zero — calls `GameEntityBase__GetGenericPropertyInt32(this, 5)`
(renamed from `FUN_00e6dd30`; `5 = GENERICPROPERTY_PetOwnerId` per `enumerations.xml:1727`) and,
if that is `> 0`, calls `GamePet__SyncLocalOwnerPetSlots(this)`.

**`GameEntityBase__GetGenericPropertyInt32(entity, 5)` reading `GENERICPROPERTY_PetOwnerId`
is the confirmation this packet needed for A-05**: the client resolves pet ownership through
the generic-property channel, keyed on property id 5, exactly as `code-map.md §1.4`'s leading
candidate proposed. `SGWPet.ownerID` is the only `CELL_PUBLIC` INT32 property on `SGWPet` (every
other property besides `ownerID`/`ownerBase` is `CELL_PRIVATE`; `ownerBase` is a `MAILBOX`, a
different wire type), which is also why the "type code == 4" gate in `LAB_00d39ae0` needs no
further per-property disambiguation — there is only one INT32 generic property this handler
could ever see fire on a `SGWPet` entity.

### 2.3 The three-byte readiness gate — what the server must actually send

`GamePet__ctor` (`0x00d39cb0`) zero-initializes the object, then sets three specific bytes:
`this+0x170 = 1` (unconditional — a constant "IsPet" type marker, always true from
construction; **not** a wire-driven readiness flag. This corrects the byte-offset transcription
in the campaign's static-RE note (`docs/analysis/pets/research/client-static-re.md` §A), which
listed this field as `[0x5c] = 1`: that number is `param_1[0x5c]` in `undefined4*` pointer
arithmetic, i.e. byte offset `0x5c*4 = 0x170`, not raw byte `0x5c`). The ctor also sets
`+0x171 = 0`, `+0x172 = 0` and `+0x173 = 0xff` (the `+0x173` byte is the
pet's *cached current stance*, updated later by `GamePet__OnPetStanceUpdateChanged`; `0xff`
read as a signed `INT8` is `-1`, a "no stance yet" sentinel).

Three call sites gate `GamePet__SyncLocalOwnerPetSlots` on `+0x170 != 0 && +0x171 != 0 &&
+0x172 != 0`:

| Byte | Set by | Meaning |
|---|---|---|
| `+0x170` | `GamePet__ctor` (always `1`) | Constant type marker — always satisfied, not a real gate |
| `+0x171` | `LAB_00d39ae0` (ownerID generic-property change, unconditionally) **and** `GamePet__OnPetAbilityListChanged` (as a side effect of a successful ability-list parse) | Effectively "ownerID **or** ability list has been observed" |
| `+0x172` | `GamePet__OnPetStanceListChanged` **only** | "`onPetStanceList` has arrived" |

**`GamePet__OnPetStanceUpdateChanged` (`onPetStanceUpdate`) does *not* participate in this
gate at all** — it only writes the cached current-stance byte at `+0x173` and fires a UI
event (`Event_UI_PetStanceChange`, matching the RTTI already found in the client-static-re
research pass). This resolves `pet-wire-formats.md`'s implementation note and
`pet-restoration.md`'s open question 4 ("stance change: user-only or also auto-sync?") for the
specific question of whether `onPetStanceUpdate` gates pet recognition: **it does not**, so the
legacy Python's commented-out `onPetStanceUpdate` call (per `code-map.md §0`) would not, by
itself, have prevented `Unit.PetN` from ever populating — the two real blockers were the
missing `ownerID` property and the missing `onPetStanceList` call, both of which the legacy
server also never sent correctly (or, per `pet-restoration.md`, sent inconsistently).

**Because `+0x171` can be set by *either* the ownerID property change *or* a successful
ability-list parse, and `+0x172` is set only by the stance list, the three-way gate reduces to
a two-input AND with either input able to arrive first:**

> The server must deliver, in any order, **both**:
>
> 1. `onEntityProperty(GENERICPROPERTY_PetOwnerId = 5, ownerEntityId)` for the pet entity
>    (the same generic-property mechanism `create.rs:158-175` already uses for `DatabaseId`);
> 2. `onPetStanceList(ARRAY<INT8>)` on the pet entity's own `SGWPet` client method (index 30).
>
> `onPetAbilityList` (index 29) is *also* required for `Unit.PetN` to populate in practice,
> because it is the message that fills the pet's visible ability bar — but per the gate logic
> above it is not strictly load-bearing for the slot bind by itself (only its side effect on
> `+0x171` is, and that is redundant with the ownerID property already setting the same byte).
> `onPetStanceUpdate` (index 31) is optional for slot binding, but should still be sent whenever
> the pet's stance differs from its `EPetStance` default (`Defensive = 1`), or the pet info
> window will show the `+0x173` ctor sentinel (`-1`, "no stance") until the player changes it.

This is the exact message sequence PT-01's `pet_create_on_client_events(witness, &entity)`
helper (`code-map.md §1.4`, item 2) needs to assemble for the owner-only createOnClient replay:
`onEntityProperty(PetOwnerId=5, owner)` (cascade, **immediately after `onEntityFlags` carrying `ENTITYFLAG_Pet`**: the owner handler checks the Pet flag when the property arrives, so an owner property sent before the flags is ignored and the pet never binds),
then `onPetAbilityList`, `onPetStanceList`, and (if non-default) `onPetStanceUpdate` — matching
the legacy Python's `onPetAbilityList` → `onPetStanceList` ordering, now with the ownerID
property confirmed as the missing third ingredient.

**Confidence**: HIGH for the ownerID mechanism, the slot IDs, and the `+0x172`/stance-list
pairing (each independently traced to a specific decompiled handler with a literal `.def`
`ArgName` string match). MEDIUM for the exact semantic label "ready flag" on `+0x171`/`+0x172`
— the bytes are confirmed to gate the call, but this pass did not prove there is no *other*
consumer of those same bytes elsewhere in the ~1700-byte `GamePet` object.

## 3. `EAbilityFlags.SpeedPet` (16384) — NOT resolved this pass

No further progress beyond `client-static-re.md §E`. A targeted search for named functions or
strings referencing `SpeedGrenade`/`SpeedDeploy`/`SpeedAttack`/`SpeedPet` returned nothing (these
are pure bitfield values with no RTTI or string trace). Closing this needs a decompile of the
ability cast-time/warmup-computation function to find where it masks `EAbilityFlags` against
`0x800`/`0x1000`/`0x2000`/`0x4000` (`SpeedGrenade`/`SpeedDeploy`/`SpeedAttack`/`SpeedPet`) — that
function was not located this pass (it is not adjacent to any of the GamePet-specific code
explored above; it almost certainly lives in the generic ability-resolution code shared by all
`SGWAbilityManager`-implementing entities, which is a much larger, unrelated search surface).
Left as an open item; D-PT10's `speedPet`-scales-warmup design default stands as a greenfield
decision, not a recovered constant.

## 4. Corrections applied to existing docs (this PR)

- **`pet-wire-formats.md`**: `onPetStanceList` corrected from `ARRAY<INT32>` to `ARRAY<INT8>`
  (4B count + N×1B, not N×4B); `onPetStanceUpdate` corrected from `INT32` (5B total) to `INT8`
  (2B total, 1B header + 1B arg). Both already matched `.def` and were already flagged as wrong
  by `pet-restoration.md` — this PR fixes the actual table, which had not been updated since.
- **`pet-restoration.md`**: the "`onPetAbilityList` [idx 1] / `onPetStanceList` [idx 0] /
  `onPetStanceUpdate` [idx 2]" line is corrected to state plainly that those are the client's
  internal `MemberCallback` construction order, not the wire method index, and that the wire
  indices are 29/30/31 (§1 above; not "idx 0/1/2" as previously written, and not the campaign
  ledger's informal shorthand either — the ledger already uses 29/30/31 correctly).

## 5. Notable side finding — not in scope to fix here

The Lua slot-constant table used to derive `Pet1..4` in §2.1 also carries `Dialog = 17` (at
`0x01b16108`) and `DialogSpeaker = 18` (at `0x01b1610c`). `dialog-portrait-lookup.md` states
"`FUN_00c67bd0(GameEntityManager, 0x11, entity_id)` [slot 17 = `DialogSpeaker`]" — `0x11 = 17`,
which per this table's values is actually `Dialog`, not `DialogSpeaker` (`DialogSpeaker = 18 =
0x12`). This may be a labeling swap in that earlier finding (its own call-site trace showed
`FUN_00d22c90(dialog, 0x1bbc)` → `FUN_00c67bd0(mgr, 0x11, ...)` for the "NPC-side slot", which
the doc inferred was `DialogSpeaker` by name association rather than by cross-checking this
constant table, which had not been found yet at the time). This is **not corrected here** —
dialog UI ownership sits with other campaign work, not PT-E1 — but is flagged for whoever next
touches `dialog-portrait-lookup.md` to re-verify against `0x01b16108`/`0x1610c` directly.

## Ghidra annotations applied this session

Renames (all previously `FUN_xxxxxxxx`):

- `0x00d39880` → `GamePet__SyncLocalOwnerPetSlots`
- `0x00dffa80` → `GameBeing__GetPetIdAt`
- `0x00e007b0` → `GameBeing__AddPetId`
- `0x00e00270` → `GameBeing__RemovePetId`
- `0x00e6dd30` → `GameEntityBase__GetGenericPropertyInt32`
- `0x00d3a3d0` → `GamePet__SubscribeEvents`
- `0x00d3a5a0` → `GamePet__UnsubscribeEvents_dtor`
- `0x00d39a10` → `GamePet__OnOwnerIdChanged_ValuePushed`
- `0x00d39eb0` → `GamePet__OnPetAbilityListChanged`
- `0x00d3a070` → `GamePet__OnPetStanceListChanged`
- `0x00d3a260` → `GamePet__OnPetStanceUpdateChanged`

Plate comments added at `0x00d39880` and `0x00d3a3d0` summarizing the slot-bind mechanism and
the readiness-gate table above, for the next investigator.

Not renamed: `LAB_00d39ae0`, `LAB_00d39600`, `LAB_00d39d80` — these are inline labels inside
`GamePet__SubscribeEvents`'s body (compiler-generated local callback thunks), not functions
Ghidra will let a rename attach to without first splitting them out; left as-is with this
document as the pointer.

## Open questions

1. **`SpeedPet` (§3)** — needs the shared ability-resolution/warmup decompile, out of this
   pass's budget.
2. **Exact semantics of `LAB_00d39600`/`LAB_00d39d80`** (the other two `GamePet__SubscribeEvents`
   handlers) — briefly examined; `LAB_00d39600` re-reads ownerID and, if it matches a just-
   destroyed entity's id, calls `FUN_00e6e330(this, 0)` (plausibly an owner-gone/despawn-visual
   reaction); `LAB_00d39d80` manipulates a second small array at `this+0x17c`
   (plausibly `toggledAbilities`, `CELL_PRIVATE` in the `.def` and therefore never wire-visible
   to the client in the current schema — worth double-checking if a future packet needs the
   toggle-ability round trip). Neither affects the `Unit.PetN` bind question this packet was
   scoped to close, so left untraced further.
3. **Dialog/DialogSpeaker slot-value swap (§5)** — flagged, not fixed; out of PT-E1's scope.
