---
audit_id: entity-property-sync-section2-audit-2026-05-16
audit_date: 2026-05-16
auditor: automated (Claude Sonnet 4.6) under direction of @cadacious
spec_version: docs/drafts/spec/entity-property-sync.md @ branch bible/spec-mercury-section-2
status: complete
binary_sha256: 109F307763A5C6C59FF484840739860BDC7163092F0644343D0B2C03E4925783
scope_files:
  - docs/drafts/spec/entity-property-sync.md §2.3, §2.5, §2.7, §1.15 OQ-2, §1.15 OQ-3
related:
  - docs/drafts/spec/entity-property-sync.md
  - docs/reverse-engineering/findings/entity-property-sync.md
  - docs/reverse-engineering/findings/entity-creation-wire-formats.md
revision_history:
  - 2026-05-16 v1 — initial Ghidra + client-tree pass, all 5 targets resolved
  - 2026-05-16 v2 — Appendix A added: OQ-A resolved; corrected flag table to 16 entries; §1.2 filter mask reconciled
---

# Entity Property Sync §2 — Ghidra Audit Report

## Reading conventions

- **CONFIRMED** — binary evidence supports the chapter claim exactly.
- **CORRECTED** — binary evidence contradicts the chapter; the correct claim is stated below.
- **UNRESOLVED** — evidence insufficient for a conclusive disposition.

Citations: binary anchors use `ghidra://SGW.exe@0x...`; client-tree paths are relative to
`game/sgw/Common/res/entities/`.

---

## Executive summary

Five audit targets. Three are corrections (the flag-keyword mapping, the typeID validation
gate claim, and the method-at-index-61 location), one is confirmed (createCellPlayer byte
count), and one is partially resolved but requires a doc-writer prose decision (OQ-2, dual
name fields). The 157-method table spot-check passes clean across all 19 sampled indices.

**Counts:**

| Disposition  | Count |
|---|---|
| CONFIRMED    | 1     |
| CORRECTED    | 3     |
| RESOLVED (OQ)| 1     |

---

## Target 1 — §2.3: SGW flag-keyword → bit-OR pattern

**Disposition: CORRECTED**

**Chapter claim (line ~592):** `CELL_PUBLIC` maps to `DATA_OTHER_CLIENT (0x02)` and probably
`DATA_OWN_CLIENT (0x04)` combined; `BASE` maps to `DATA_BASE (0x08)` plus `DATA_OWN_CLIENT
(0x04)`.

**Binary evidence:**

`DataDescription_ParseFlagStr @ ghidra://SGW.exe@0x015959c0` iterates a static table at
`PTR_s_CELL_PRIVATE_01e920e0 (0x01e920e0)`. Each 12-byte entry is `[ptr_to_name, flag_value,
ptr_to_warning_fn]`. Reading the 9 entries by resolving the name pointers (strings at
`0x01b1ae38..0x01b1aeb3`):

| Entry# | Keyword           | Flag value (hex) | Flag value (dec) |
|--------|-------------------|-----------------|-----------------|
| 0      | `CELL_PRIVATE`    | `0x00`          | 0               |
| 1      | `CELL_PUBLIC`     | `0x01`          | 1               |
| 2      | `OTHER_CLIENTS`   | `0x03`          | 3               |
| 3      | `OWN_CLIENT`      | `0x04`          | 4               |
| 4      | `BASE`            | `0x08`          | 8               |
| 5      | `BASE_AND_CLIENT` | `0x0c`          | 12              |
| 6      | `CELL_PUBLIC_AND_OWN` | `0x05`      | 5               |
| 7      | `ALL_CLIENTS`     | `0x07`          | 7               |
| 8      | `EDITOR_ONLY`     | `0x40`          | 64              |

The loop bound at `0x1e9219f` limits the walk to exactly 9 entries (9 × 12 = 108 bytes past
start; the 10th entry would be at `0x01e9214c`, which exceeds the bound). No warning function
pointers are non-null for any of the 9 entries.

**What the table tells us:**

1. `CELL_PUBLIC` = `0x01` which is `DATA_GHOSTED`. Not `DATA_OTHER_CLIENT (0x02)`. The
   chapter's hypothesis is wrong.
2. `OTHER_CLIENTS` = `0x03` = `DATA_GHOSTED | DATA_OTHER_CLIENT`. It is present in the binary
   table — the chapter claim that stock-BW keywords `OWN_CLIENT`/`OTHER_CLIENTS` are absent
   from the *binary* is wrong. They are absent from the SGW *`.def` files*, but the binary
   still has them in its parse table (entries 2 and 3).
3. `BASE` = `0x08` only — no combined OWN_CLIENT. The chapter's "plus `DATA_OWN_CLIENT (0x04)`"
   is wrong.
4. Compound keywords exist in the binary: `BASE_AND_CLIENT` (0x0c = BASE|OWN_CLIENT),
   `CELL_PUBLIC_AND_OWN` (0x05 = GHOSTED|OWN_CLIENT), `ALL_CLIENTS` (0x07 = GHOSTED|OTHER_CLIENT|OWN_CLIENT).
   These are binary-only — none appear in the SGW `.def` tree.

**Correct statement of the keyword→bit mapping:**

The parser's flag table maps keywords to *pre-combined* bit values. The SGW `.def` tree uses
only `CELL_PRIVATE (0x00)`, `CELL_PUBLIC (0x01 = DATA_GHOSTED)`, and `BASE (0x08 =
DATA_BASE)`. The `OWN_CLIENT` and `OTHER_CLIENT` bits are never set for any SGW property via
a `.def` keyword. The §1.2 filter `flags & 0x06 != 0` would therefore match zero SGW
properties if applied to the `flags` byte as written.

**This raises a follow-on question (new OQ — see section 8):** If no SGW property has bits
1 or 2 set, what does the client-property pointer array at EntityDescription+0x70/+0x74
actually contain? The chapter claims this array holds references to properties where `flags &
0x06 != 0`. If the flag mapping is `CELL_PUBLIC=0x01` only, then no SGW property clears the
`0x06` filter, and the pointer array would always be empty. That contradicts the observable
wire format. Resolution: either the §1.2 filter description is wrong (the correct filter is
`flags & 0x01 != 0`, i.e., DATA_GHOSTED), or there is a post-parse step that translates
`0x01` into `0x03` (GHOSTED+OTHER_CLIENT). The ParseProperties decompile at `0x015924a0`
shows the filter `(local_7c & 6) != 0` but also a separate `(local_7c & 1) != 0` branch —
suggesting the filter is applied against the raw flag byte as read from the keyword table,
not after any translation. This needs a second-pass audit on `EntityDescription_ParseProperties`
to check if `CELL_PUBLIC (0x01)` triggers the `(flags & 6)` or the `(flags & 1)` branch.

**Ghidra anchors for this finding:**
- `ghidra://SGW.exe@0x015959c0` — `DataDescription_ParseFlagStr`, contains the loop
- `ghidra://SGW.exe@0x01e920e0` — static flag table (9 entries)
- `ghidra://SGW.exe@0x01b1ae38` — string data (`CELL_PRIVATE` through `PRIVATE` block)

**Recommendation for doc-writer:** Replace the entire §2.3 bit-OR hypothesis with the table
above. Strike the claim that `OWN_CLIENT`/`OTHER_CLIENTS` are absent from the binary. Revise
the sentence "If the SGW build never sets bits 1 or 2 through `.def` keywords" — this is
correct for the SGW `.def` files but the binary *does* have those keywords. Separately flag
the follow-on OQ about the `0x06` filter vs `0x01` mapping as an open question.

---

## Target 2 — §2.5: typeID validation gate in `CreateBasePlayer`

**Disposition: CORRECTED**

**Chapter claim (line ~666):** "a `createBasePlayer` carrying a server-only typeID would be
malformed — the client has no instantiation path for those types." [citation needed]

**Binary evidence:**

`ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` decompile:

```c
// Read entityId (4 bytes) from stream
puVar2 = (**(code **)(*(int *)pThis + 4))(4);
uVar1 = *puVar2;   // entityId

// Read typeId (2 bytes) from stream
puVar3 = (**(code **)(*(int *)pThis + 4))(2);

// Invoke delegate unconditionally — no typeID validation
if (*(undefined4 **)((int)this + 0x168) != (undefined4 *)0x0) {
    (**(code **)**(undefined4 **)((int)this + 0x168))
              (*(undefined4 *)((int)this + 0x16c), *puVar3, pThis);
}
```

The delegate at `*(this+0x168)` is called with the typeId directly. **There is no typeID
range check, no server-only flag test, and no rejection path before or after the delegate
call.** The function then proceeds to check for a buffered `createCellPlayer` message and
replays it if present — also with no typeID guard.

The delegate itself (`*(this+0x168)` vtable[0]) was not decompiled in this pass, so it is
possible the type validation lives one level deeper. However, the delegate is invoked
unconditionally as long as the pointer is non-null. If the delegate throws or returns an
error for an unknown typeID, there is no error-checking code in `CreateBasePlayer` to handle
it.

**Correct statement:** The client does not perform explicit typeID validation in
`CreateBasePlayer` before invoking the entity-creation delegate. Whether the delegate itself
rejects server-only typeIDs is not confirmed by this audit pass. The §2.5 claim that "the
client has no instantiation path for those types" may be correct in effect (no `.def` file
means no entity description loaded, so the delegate cannot instantiate the type) but is not
confirmed via a server-only-rejection code path.

**Ghidra anchors:**
- `ghidra://SGW.exe@0x00dddca0` — `ServerConnection_CreateBasePlayer`, full decompile
  confirms no typeID gate.

**Recommendation for doc-writer:** Change the [citation needed] to a qualified statement:
"The client passes the typeID directly to its entity-creation delegate
(`*(this+0x168)@0x00dddca0`) without an explicit validation gate. If the delegate cannot
resolve a typeID to a loaded entity description, the call would silently fail or crash —
there is no recovery path visible in `CreateBasePlayer`. Confirmed: no explicit server-only
typeID rejection in the outer handler; whether the delegate performs type-existence checking
is a secondary investigation item."

---

## Target 3 — §2.7: exact method name at index 61

**Disposition: CORRECTED**

**Chapter claim (line ~758):** Index 61 is "the second entry in GateTravel (since GateTravel
occupies 65–68, the boundary is inside the upstream `SGWInventoryManager` slot, or wherever
interface alignment lands)."

**Client-tree evidence (full cascade parse):**

Walking the cascade with a depth-tracking XML parser against the `.def` files produces the
following exact ranges (all counts confirmed, total = 157):

| Source                 | Indices     | Count |
|------------------------|-------------|-------|
| SGWSpawnableEntity     | 0–11        | 12    |
| SGWBeing (interface)   | 12–19       | 8     |
| SGWAbilityManager      | —           | 0     |
| SGWCombatant           | 20–25       | 6     |
| SGWBeing (entity own)  | 26–26       | 1     |
| Communicator           | 27–33       | 7     |
| OrganizationMember     | 34–51       | 18    |
| MinigamePlayer         | 52–64       | 13    |
| GateTravel             | 65–68       | 4     |
| SGWInventoryManager    | 69–75       | 7     |
| SGWMailManager         | 76–79       | 4     |
| Missionary             | 80–84       | 5     |
| ContactListManager     | 85–89       | 5     |
| SGWBlackMarketManager  | 90–95       | 6     |
| ClientCache            | 96–97       | 2     |
| SGWPoller              | —           | 0     |
| SGWPlayer (own)        | 98–156      | 59    |

**Index 61 = `minigameCallDisplay`**, the 10th method (offset 9, zero-based) in the
`MinigamePlayer` interface (`defs/interfaces/MinigamePlayer.def`). Index 61 is inside
`MinigamePlayer` (52–64), not inside `GateTravel` (65–68) or `SGWInventoryManager` (69–75).
The chapter's parenthetical is wrong on both counts.

**Correct statement:** "The first method at two-byte wire-encoding territory (index 61) is
`minigameCallDisplay`, the 10th entry in `interfaces/MinigamePlayer.def`, which occupies
indices 52–64 in the full SGWPlayer client-method table."

**Note — cascade index discrepancy vs chapter §2.7 table:** The chapter table gives
`MinigamePlayer` range as 52–64, which the script confirms. However the chapter's description
of the SGWBeing layer (section 3 in the cascade) claims "indices 12–26 (8+6+1=15 methods)".
The correct range is:
- 12–19: SGWBeing interface (8)
- 20–25: SGWCombatant (6)
- 26: SGWBeing.def own (1)

Index 26 is the last index of SGWBeing's own section, and Communicator starts at 27. The
arithmetic in the chapter is consistent; only the wording in the parenthetical for index 61
is wrong.

**Recommendation for doc-writer:** Replace "[citation needed — pin the exact method name at
index 61...]" with "Index 61 = `minigameCallDisplay` (`defs/interfaces/MinigamePlayer.def`,
the 10th `<ClientMethods>` entry, zero-based), confirmed by a depth-tracking parse of the
full 17-file cascade." Also correct the parenthetical in the same paragraph: replace "that is
the **second** entry in GateTravel (since GateTravel occupies 65–68, the boundary is inside
the upstream SGWInventoryManager slot..." with "that falls at offset 9 within the
`MinigamePlayer` interface (indices 52–64)."

---

## Target 4 — §1.15 OQ-2: DataDescription dual name fields at +0x24 and +0x40

**Disposition: RESOLVED (partial)**

**Background:** `EntityDescription_FindAndWritePropertyByName @ ghidra://SGW.exe@0x0158e780`
walks 0x110-byte DataDescription elements (vector at EntityDescription+0x10/+0x14), comparing
two string fields within each element: `element+0x24` (length at +0x34, capacity at +0x38)
vs `element+0x40` (length at +0x50, capacity at +0x54). The open question was which field is
the internal name and which is the client-visible/alias name.

**Binary evidence:**

`DataDescription_Constructor @ ghidra://SGW.exe@0x01591fb0` initializes the 0x110-byte form:
- `+0x04`: first StdStringMSVC (capacity=0xf, empty) — the property name (XML tag name)
- `+0x24`: second StdStringMSVC (capacity=0xf, empty) — second name field
- `+0x40`: third StdStringMSVC (capacity=0xf, empty) — third name field

The `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` decompile clarifies the
runtime (0x40-byte compact) DataDescription layout: `this+0x24` in the **0x40-byte** form is
the entity name string written to the stream. The 0x110-byte parse-time form is distinct.

`FindAndWritePropertyByName` compares `element+0x24` against `element+0x40` within the same
0x110-byte element. The function only calls `EntityDescription_WriteClientData` (which writes
to a stream) when these two fields are equal. This is a "write only if internal name ==
external name" gate — i.e., the function skips aliased properties (where the two name fields
differ) and writes non-aliased properties.

**What populates +0x24 vs +0x40:** The parse path for the 0x110-byte form was not fully
traced in this audit pass. The small parse-time form (0x40 bytes, used in
`EntityDescription_ParseProperties`) writes only offsets +0x00 through +0x3c (confirmed by
`FUN_0158f260`). The 0x110-byte form is a distinct, larger structure. The write sites for
`+0x24` and `+0x40` in the 0x110-byte form remain untraced — this is the remaining open
sub-question.

**Resolved part:** One field is the internal/XML property name, the other is the
client-visible name. `FindAndWritePropertyByName` seeks properties where internal==external
(non-aliased). Which field is which requires finding the write site in the 0x110-byte form's
initialization path — likely in a function that copies from the small parse-time form into
the 0x110-byte runtime form.

**Ghidra anchors:**
- `ghidra://SGW.exe@0x01591fb0` — Constructor, initializes both fields as empty
- `ghidra://SGW.exe@0x0158e780` — FindAndWritePropertyByName, the comparison site
- `ghidra://SGW.exe@0x0158f260` — small-form copy (only writes +0x00..+0x3c)

**Recommendation for doc-writer:** Update OQ-2 to "PARTIALLY RESOLVED: +0x24 and +0x40 are
two name fields in the 0x110-byte parse-time DataDescription. `FindAndWritePropertyByName`
compares them to skip aliased properties. Which field carries the internal XML name and which
carries the client-visible alias is not yet pinned — requires tracing the write site that
populates the 0x110-byte form from the 0x40-byte parsed form."

---

## Target 5 — §1.15 OQ-3: createCellPlayer byte count

**Disposition: CONFIRMED**

**Chapter claim:** `createCellPlayer` payload is exactly 32 bytes with no trailing reads.

**Binary evidence:**

`ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0` decompile read sequence:

```
Read 1: (*stream_vtable+4)(4)  → 4 bytes  spaceId u32
Read 2: (*stream_vtable+4)(4)  → 4 bytes  vehicleId u32
Read 3: (*stream_vtable+4)()   → 8 bytes  position XY (two f32 via 8-byte read)
         + 4 bytes posZ        → (the code reads via puVar4/uStack_24 pattern, 12 bytes total)
Read 4: BundlePrimer__read3    → 12 bytes rotation (X, Z, Y order — Y/Z swapped)
```

Total: 4 + 4 + 12 + 12 = 32 bytes. No reads follow `BundlePrimer__read3` before the function
transitions to entity table bookkeeping (GetOrAddEntityTableSlot etc). The buffered-message
path (when `*(this+0x16c) == 0`) writes to a buffer and returns early — no stream reads in
the buffer path. The 32-byte count is confirmed with no tail reads.

**Note on the rotation read:** `BundlePrimer__read3` reads rotation in X, Z, Y order (Y and
Z swapped from standard XYZ). The plate comment on the function confirms this. This is not
a bug — it is how SGW transmits Euler angles.

**Ghidra anchors:**
- `ghidra://SGW.exe@0x00dda2e0` — `ServerConnection_CreateCellPlayer`, full decompile
- Rotation reader: `BundlePrimer__read3` (called inline, address embedded in compiled code)

**Recommendation for doc-writer:** Close OQ-3 as CONFIRMED. Add note: rotation is X, Z, Y
order (Y/Z swapped) per `BundlePrimer__read3`. The 32-byte count may now be stated as binary-
confirmed and the [citation needed] tag removed.

---

## 157-method table spot-check

**Disposition: ALL CHECKED CLEAN**

19 indices from the "verified high-frequency" list in §2.7 were verified by a depth-tracking
parse of all 17 cascade files:

| Index | §2.7 listed name       | Verified name          | Match |
|-------|------------------------|------------------------|-------|
| 20    | onStatUpdate           | onStatUpdate           | OK    |
| 26    | BeingAppearance        | BeingAppearance        | OK    |
| 52    | (not listed)           | onStartMinigame        | —     |
| 61    | (citation needed)      | minigameCallDisplay    | NEW   |
| 65    | setupStargateInfo      | setupStargateInfo      | OK    |
| 69    | onBagInfo              | onBagInfo              | OK    |
| 72    | onUpdateItem           | onUpdateItem           | OK    |
| 75    | onCashChanged          | onCashChanged          | OK    |
| 98    | (not listed)           | onBeginAidWait         | —     |
| 101   | onKnownAbilitiesUpdate | onKnownAbilitiesUpdate | OK    |
| 102   | onTimeofDay            | onTimeofDay            | OK    |
| 105   | onDialogDisplay        | onDialogDisplay        | OK    |
| 115   | onPlayerDataLoaded     | onPlayerDataLoaded     | OK    |
| 117   | onClientMapLoad        | onClientMapLoad        | OK    |
| 122   | setupWorldParameters   | setupWorldParameters   | OK    |
| 125   | addClientHintedGenericRegion | addClientHintedGenericRegion | OK |
| 141   | onAbilityTreeInfo      | onAbilityTreeInfo      | OK    |
| 152   | onDuelEntitiesRemove   | onDuelEntitiesRemove   | OK    |
| 155   | onPlayMovie            | onPlayMovie            | OK    |

The chapter's §2.7 listed indices are all correct. No alarm needed. The chapter's per-interface
count table also matches the cascade parse exactly (12+8+6+1+7+18+13+4+7+4+5+5+6+2+59 = 157).

---

## New open questions surfaced by this audit

**OQ-A (from Target 1 correction):** If `CELL_PUBLIC` maps to `0x01 = DATA_GHOSTED` only, and
the client-property array filter is `flags & 0x06 != 0` (bits OTHER_CLIENT | OWN_CLIENT), then
no SGW property would ever be added to that array. But the wire format clearly sends property
data to clients. Resolution path: re-examine `EntityDescription_ParseProperties @ 0x015924a0`
for the `(local_7c & 1)` vs `(local_7c & 6)` branch structure more carefully. The decompile
shows both branch conditions — the `(flags & 1)` branch also exists. The §1.2 filter
description may be using a combined mask that includes DATA_GHOSTED; the text "bits 1+2"
using 0-based bit numbering would map to bits 0x02 and 0x04, but the actual DATA_GHOSTED bit
at 0x01 may be the correct gate.

**OQ-B (from Target 4, remaining):** Which of `+0x24` / `+0x40` in the 0x110-byte
DataDescription is internal name vs client-visible name. Trace the function that initializes
the 0x110-byte form from the 0x40-byte parse-time form. Likely a copy-constructor or
assignment operator not yet decompiled.

---

## New Ghidra anchors to add to chapter frontmatter evidence_refs.re

The following addresses were confirmed in this audit pass and should be added to `evidence_refs.re`
in the chapter frontmatter:

- `ghidra://SGW.exe@0x015959c0` — `DataDescription_ParseFlagStr`, flag-name→bit-value table walker
- `ghidra://SGW.exe@0x01e920e0` — static flag table (16 entries: CELL_PRIVATE through ALL_CLIENT; corrected from earlier "9 entries" count)
- `ghidra://SGW.exe@0x01b1ae38` — string data for flag table names (primary keywords, entries 0–8)
- `ghidra://SGW.exe@0x01b1aeb4` — string data for deprecated-alias entries (entries 9–15, all with non-null warning function pointers)

---

## Appendix A — OQ-A resolution: §1.2 filter mask

**OQ-A disposition: RESOLVED**

### What was asked

If `CELL_PUBLIC` maps to `0x01 = DATA_GHOSTED` (bit 0 only), and the client-property pointer
array gate is `flags & 0x06` (bits 1+2 = `DATA_OTHER_CLIENT | DATA_OWN_CLIENT`), then no SGW
property would ever enter that array — contradicting the observable wire format. Determine
which of the four hypotheses (a)–(d) is correct.

### Decompile evidence — `DataDescription_ParseFlagStr @ 0x015959c0`

The function does a **pure table-walk with a direct value assignment**:

```c
*pOutFlags = (uint)local_4[1];   // single assignment, no post-OR
```

There is no additional bit-setting after the table lookup. The flag value written to
`*pOutFlags` is exactly the integer stored at `table_entry[1]`. No case (b) bit-expansion
happens here.

**Correction to Target 1 — table has 16 entries, not 9.**

The prior pass stated "9 entries (entries 9–15 would exceed the bound)." That was wrong. The
loop bound is `0x1e9219f < (int)local_4` (exclusive upper bound = `0x01e921a0`). Entry 15
begins at `0x01e920e0 + 15×12 = 0x01e92194`, which is `<= 0x01e9219f`. Entry 16 would start
at `0x01e921a0`, which fails the bound check. So the table has exactly 16 entries.

Complete flag table (`ghidra://SGW.exe@0x01e920e0`, 16×12-byte entries, little-endian):

| Entry | Keyword              | Flag value | Bits set (DATA_* names)                    | Warn fn |
|-------|----------------------|------------|--------------------------------------------|---------|
| 0     | `CELL_PRIVATE`       | `0x00`     | (none)                                     | null    |
| 1     | `CELL_PUBLIC`        | `0x01`     | `DATA_GHOSTED`                             | null    |
| 2     | `OTHER_CLIENTS`      | `0x03`     | `DATA_GHOSTED \| DATA_OTHER_CLIENT`         | null    |
| 3     | `OWN_CLIENT`         | `0x04`     | `DATA_OWN_CLIENT`                          | null    |
| 4     | `BASE`               | `0x08`     | `DATA_BASE`                                | null    |
| 5     | `BASE_AND_CLIENT`    | `0x0c`     | `DATA_BASE \| DATA_OWN_CLIENT`              | null    |
| 6     | `CELL_PUBLIC_AND_OWN`| `0x05`     | `DATA_GHOSTED \| DATA_OWN_CLIENT`           | null    |
| 7     | `ALL_CLIENTS`        | `0x07`     | `DATA_GHOSTED \| DATA_OTHER_CLIENT \| DATA_OWN_CLIENT` | null |
| 8     | `EDITOR_ONLY`        | `0x40`     | `DATA_EDITOR_ONLY`                         | null    |
| 9     | `PRIVATE`            | `0x00`     | (none) — deprecated alias for CELL_PRIVATE | non-null|
| 10    | `CELL`               | `0x01`     | `DATA_GHOSTED` — deprecated alias          | non-null|
| 11    | `GHOSTED`            | `0x01`     | `DATA_GHOSTED` — deprecated alias          | non-null|
| 12    | `OTHER_CLIENT`       | `0x03`     | `DATA_GHOSTED \| DATA_OTHER_CLIENT` — deprecated alias | non-null|
| 13    | `GHOSTED_AND_OWN`    | `0x05`     | `DATA_GHOSTED \| DATA_OWN_CLIENT` — deprecated alias | non-null|
| 14    | `CELL_AND_OWN`       | `0x05`     | `DATA_GHOSTED \| DATA_OWN_CLIENT` — deprecated alias | non-null|
| 15    | `ALL_CLIENT`         | `0x07`     | `DATA_GHOSTED \| DATA_OTHER_CLIENT \| DATA_OWN_CLIENT` — deprecated alias | non-null|

String data for entries 0–8 at `ghidra://SGW.exe@0x01b1ae38`; entries 9–15 at
`ghidra://SGW.exe@0x01b1aeb4`. Non-null warning functions in entries 9–15 are BigWorld
"using deprecated flag name" emitters (the exact warning string "Using old Flag" is visible
at `ghidra://SGW.exe@0x01b1af14` offset +0x44: `"DataDescription::parse: Using old Fl..."`).

### Decompile evidence — `EntityDescription_ParseProperties @ 0x015924a0`

The function uses `local_7c` as the flag field for each property. Two distinct conditionals:

**Conditional 1 — type-mismatch warning gate (does NOT affect the pointer array):**

```c
if ((local_7c & 6) == 0) {
    // bits 0x02|0x04 not set — property is BASE or CELL_PRIVATE or CELL_PUBLIC
    if (((local_7c & 1) != 0) && ...) {
        // emit PYTHON/USER_TYPE/CLASS/ARRAY/TUPLE/FIXED_DICT complex-type warnings
        // "Property: %s.%s: properties should not be propagated to the client."
        // This fires for CELL_PUBLIC (0x01) properties with complex types.
    }
} else {
    // bits 0x02|0x04 set — OWN_CLIENT or OTHER_CLIENTS etc
    // emit: "Property: %s.%s: properties should not be propagated to the client."
}
```

**Conditional 2 — insertion into main array and client-property pointer array:**

```c
if ((local_7c >> 6 & 1) == 0) {   // EDITOR_ONLY (0x40) NOT set
    // always insert into main property array at EntityDesc+0x5c/+0x60
    DataDescriptionVec_PushBack((void *)((int)pvVar1 + 0x5c), local_9c, unaff_EDI);

    if ((local_7c & 6) != 0) {
        // push pointer into client-property array at EntityDesc+0x70/+0x74
        FUN_015fbd50((void *)((int)local_110 + 0x6c), &local_d4);
    }
}
```

The filter mask for the client-property pointer array at `+0x70/+0x74` is **exactly
`flags & 0x06`** — no other path inserts into that array.

### Reconciled findings

**Hypothesis confirmed: (a) — the §1.2 filter description is accurate; what is wrong is the
interpretation of which properties match it in SGW.**

The chapter's `flags & 0x06` claim for the client-property array is **binary-correct**. The
chapter's implicit assumption that `CELL_PUBLIC` properties enter this array is **wrong**.

Corrected claim set:

1. **`CELL_PUBLIC` sets only `DATA_GHOSTED` (bit 0 = `0x01`).** It does NOT set bits 1 or 2.
   `CELL_PUBLIC & 0x06 == 0`. A `CELL_PUBLIC` property is inserted into the **main** property
   array at `EntityDesc+0x5c/+0x60` but is NOT inserted into the client-property pointer array
   at `EntityDesc+0x70/+0x74`.

2. **`BASE` sets only `DATA_BASE` (bit 3 = `0x08`).** `BASE & 0x06 == 0`. BASE properties also
   go to the main array only, not the client-property pointer array.

3. **The client-property pointer array at `+0x70/+0x74` is only populated by keywords that
   set `OWN_CLIENT` (bit 2 = `0x04`) or `DATA_OTHER_CLIENT` (bit 1 = `0x02`):** specifically
   `OTHER_CLIENTS (0x03)`, `OWN_CLIENT (0x04)`, `BASE_AND_CLIENT (0x0c)`,
   `CELL_PUBLIC_AND_OWN (0x05)`, `ALL_CLIENTS (0x07)`, and their deprecated aliases. None of
   these keywords appear in any SGW `.def` file. Therefore the client-property pointer array
   is effectively always empty in SGW.

4. **The `flags & 0x01` (`DATA_GHOSTED`) branch controls complex-type warnings only** — it is
   not a gate for the client-property pointer array, contrary to what the OQ-A resolution
   path suggested.

5. **The observable wire format for property updates is driven by the main property array at
   `+0x5c/+0x60`, not the client-property pointer array at `+0x70/+0x74`.** The chapter's
   §1.2 claim that the client-property array is the routing table for client property updates
   is the actual error; the filter mask expression itself is binary-correct.

### Impact on §1.2 — what the doc-writer must fix

The chapter §1.2 states (paraphrasing): "the client-property pointer array holds references to
all properties where `flags & 0x06 != 0`; these are the properties that reach the client."

Binary evidence shows this is architecturally correct for stock BigWorld but practically always
empty in SGW because no SGW `.def` property uses a keyword that sets bits 1 or 2. The
conclusion to draw is:

- The §1.2 sentence "properties where `flags & 0x06 != 0` are pushed to the client-property
  pointer array" is a correct description of the binary mechanism.
- The §1.2 implication that `CELL_PUBLIC` properties are included in this set is wrong.
- The doc-writer should add a "SGW-specific note": in SGW, every property in the `.def` tree
  uses `CELL_PUBLIC (0x01)`, `BASE (0x08)`, or `CELL_PRIVATE (0x00)`. None of these sets bits
  1 or 2, so the `+0x70/+0x74` client-property pointer array is empty for all SGW entities.
  Client property updates are routed via the main DataDescription array at `+0x5c/+0x60`
  instead.

### Impact on Target 1 — update needed

Target 1 requires one correction beyond what was already noted:

- "9 entries" → "16 entries (9 primary + 7 deprecated-alias entries with non-null warning
  functions; the deprecated aliases set the same flag values as their primary equivalents)."
- The conclusion "No warning function pointers are non-null for any of the 9 entries" was
  incorrect — entries 9–15 all have non-null warning function pointers.
- The rest of Target 1 (keyword→bit mapping for primary keywords, the follow-on OQ about the
  filter mask) is confirmed correct by this pass.

### Ghidra anchors for Appendix A

- `ghidra://SGW.exe@0x015959c0` — `DataDescription_ParseFlagStr`, pure table-walk, single
  `*pOutFlags = table[1]` assignment (no post-OR)
- `ghidra://SGW.exe@0x015924a0` — `EntityDescription_ParseProperties`, `(local_7c & 6)` gate
  for client-property pointer array at `+0x70/+0x74`; `(local_7c & 1)` gate for complex-type
  warnings only
- `ghidra://SGW.exe@0x01e920e0` — complete 16-entry flag table
- `ghidra://SGW.exe@0x01b1ae38` — string block for entries 0–8 (primary keywords)
- `ghidra://SGW.exe@0x01b1aeb4` — string block for entries 9–15 (deprecated aliases)
- `ghidra://SGW.exe@0x01b1af14` — deprecation warning string prefix
  `"DataDescription::parse: Using old Fl..."` confirming entries 9–15 are deprecated aliases
