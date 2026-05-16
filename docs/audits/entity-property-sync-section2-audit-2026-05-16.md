---
audit_id: entity-property-sync-section2-audit-2026-05-16
audit_date: 2026-05-16
auditor: automated (Claude Sonnet 4.6) under direction of @cadacious
spec_version: docs/drafts/spec/entity-property-sync.md @ commit 735c9d7 (branch worktree-bible+spec-entity-property-sync)
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

```text
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

---

## Appendix B — PR #305 review investigation findings (2026-05-16)

Investigator: Game Archaeology Specialist (automated, Claude Sonnet 4.6).
Scope: Clara's binary-investigable gaps from PR #305 review, organized by gap code.
All decompiles produced via Ghidra MCP bridge against `SGW.exe` (SHA-256: `109F307...`).

### B.0 — Executive summary

| Gap | Title | Disposition |
|-----|-------|-------------|
| G3  | AoI 3-phase cascade Ghidra anchors | RESOLVED |
| G4  | `leaveAoI` handler Ghidra anchor | RESOLVED |
| G5  | `CLIENT_DATA \| BASE_DATA` filter mask numeric values | RESOLVED |
| G13 | Failure mode: propID outside valid range | PARTIALLY-RESOLVED |
| G14 | Failure mode: methodID not in table | RESOLVED |
| G15 | Failure mode: MD5 schema fingerprint mismatch | UNRESOLVED |
| G16 | Failure mode: unknown typeID in delegate | PARTIALLY-RESOLVED |
| G17 | Failure mode: sub-slot decode mismatch | RESOLVED |
| G18 | Failure mode: property update for entity not in table | RESOLVED |
| G35 | `entityMessage` (msg_id 0x0D) wire format | RESOLVED |
| G36 | Property change batching | RESOLVED |
| G37 | Method argument serialization (FIXED_DICT / ARRAY / TUPLE) | PARTIALLY-RESOLVED |
| G38 | Entity reference / mailbox serialization | PARTIALLY-RESOLVED |
| G39 | Nested property updates / slice mode | RESOLVED |
| G40 | Property default values omitted from wire? | RESOLVED |

15 gaps investigated; 9 fully resolved, 4 partially resolved, 2 unresolved.

---

### B.1 — G3: AoI 3-phase cascade Ghidra anchors

**Disposition**: RESOLVED. **→ Folded into chapter §1.9** as a callout: "The 7-method `createOnClient` cascade order is server-determined."

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x00dd2270` — `EntityManager_HandleEntityCreate`
- `ghidra://SGW.exe@0x00dd2800` — `EntityManager_EnterAoI`
- `ghidra://SGW.exe@0x00dd24f0` — `GameEntityManager_FinishEntityLoad`

**Evidence retained**: `EntityManager_HandleEntityCreate @ 0x00dd2270` decompile shows the function calls `EntityManager_CreateEntity`, applies an initial world transform via `FUN_00e68a10`, then calls `GameEntityManager_FlushDeferredNotifications`. There is no client-side loop iterating a fixed seven-method list — each incoming method call is routed independently through `GameEntityManager_DispatchEntityRpc @ 0x00dd2b80`. Paths ruled out: a static ordered dispatch table (searched the `EntityManager_*` symbol space, none found); a hardcoded enumeration of method names (the decompile reads opaque indices, not symbols). The "deprecated server source" `CachedEntity::onEntityVisible` at `cached_entity.cpp:173` is embedded in SGW.exe debug strings — confirms the server-side origin of the cascade enumeration, not a client-side mechanism.

---

### B.2 — G4: `leaveAoI` handler Ghidra anchor

**Disposition**: RESOLVED. **→ Folded into chapter §1.10** as a new subsection covering `EntityManager_LeaveAoI` behavior + the deferred-leave slot at `GameEntityManager+0x3C`.

**Ghidra anchor**: `ghidra://SGW.exe@0x00dd29d0` — `EntityManager_LeaveAoI`.

**Evidence retained** (decompile path traced — both branches confirmed):

1. Debug-flag check: `if (g_bEntityRpcDebug) { log entity-id + space-id }`.
2. Primary map search at `GameEntityManager+0x18`.
3. **Path A** (entity NOT in primary map): direct `nSpaceId->vtable[2]()` invocation.
4. **Path B** (entity IS in primary map): byte-count read, `scalable_malloc(0x20)`, `ConstructMemoryOStream` copy, queue to `GameEntityManager+0x3C` via `LookupOrEmplaceSecondaryListenerSlot` + `FUN_0046eef0`.

The "decrements reference count" plate comment is **wrong** — annotation-script bug similar to the `MethodDescription_Destructor` rename issue tracked in chapter §1.15 OQ-5. Worth recording in the annotation-script-shift-bugs log.

---

### B.3 — G5: `CLIENT_DATA | BASE_DATA` filter mask numeric values

**Disposition**: RESOLVED. **→ Folded into chapter §1.11** with the verified numeric values + 0x5f strip mask + reconciliation with the §1.2 SGW divergence.

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x01590fc0` — `EntityDescription_WriteClientData`
- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream`
- `ghidra://SGW.exe@0x015924a0` — `EntityDescription_ParseProperties` (from Appendix A)

**Evidence retained**: decompile of `WriteClientData` shows the gate `*(byte*)(pvDataDesc + 0x20) & 6 != 0` — `DATA_OTHER_CLIENT (0x02) | DATA_OWN_CLIENT (0x04)` = `CLIENT_DATA = 0x06`. `WriteToStream` masks with `0x5f` before wire — strips `DATA_PERSISTENT (0x20)` and `DATA_ID (0x80)`. Combined: `CLIENT_DATA | BASE_DATA = 0x06 | 0x08 = 0x0E`; `CLIENT_DATA | CELL_DATA = 0x06 | 0x01 = 0x07`. Paths ruled out: separate `BASE_DATA` constant table (none in `.data`); independent `CELL_DATA` mask (it's the same bit as `DATA_GHOSTED`).

---

### B.4 — G13: Failure mode — propID outside valid range

**Disposition**: PARTIALLY-RESOLVED. **→ Folded into chapter §1.16 as F1** ("decoder behavior UNVERIFIED") + tracked as **OQ-X** in §1.15.

**Ghidra anchor**: `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`.

**Evidence retained**: `FNetworkPropertyChange__vfunc_0 @ 0x015652d0` is the **outgoing** property-change serializer — calls Mercury bundle write helpers, not an inbound handler. RTTI confirms this is `FNetworkPropertyChange` from Unreal's replication system, on the emit path. The **inbound** propID decoder + bounds checker were searched for and not located in this pass. Search surface ruled out: `ServerConnection_*` immediate handlers (none of the candidates dispatched on propID bounds); the message-catalog row for `updateEntity` msg_id `0x0A` points at `FUN_01560ad0` which delegates opaquely. Path to closure: a live x64dbg session with a crafted oversized propID, OR locate the inbound dispatcher via callers of `EntityDescription_GetClientPropertyByIndex @ 0x01590d80`.

---

### B.5 — G14: Failure mode — methodID not in table

**Disposition**: RESOLVED. **→ Folded into chapter §1.16 as F2** (silent drop + wide-string log).

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x00c6f8f0` — `ProcessEntityMethodEmission`
- `ghidra://SGW.exe@0x01590f30` — `EntityDescription_GetExposedClientMethodByIndex`

**Evidence retained** — decompile excerpt that pins the behavior:

```c
uVar3 = EntityDescription_FindMethodIdByName(pvVar5, *(ushort*)(pEntityDesc + 0x14));
if (uVar3 == 0xffff) {
    FUN_00482ff0(L"No client->server entity description mapping found for entity type %d; message id: %d.", ...);
    // falls through / returns
}
```

The `0xFFFF` sentinel is the red-black tree's "not found" return; the wide-string log only fires when `g_bEntityRpcDebug (DAT_01ef2224)` is set. For a method index with no registered listener, `EntityDescription_GetExposedClientMethodByIndex` returns `0` on out-of-bounds and the dispatch returns without calling any handler. Both paths: silent drop. No crash, no disconnect.

---

### B.6 — G15: Failure mode — MD5 schema fingerprint mismatch

**Disposition**: UNRESOLVED. **→ Folded into chapter §1.16 as F3** (UNVERIFIED) + tracked as **OQ-Y** in §1.15.

**Ghidra anchor(s)**: None confirmed for the comparison site.

**Evidence retained / paths searched**: Searches for `MD5_Finalize`, `MD5_DigestToHexString`, and the CryptoPP cipher-layer wrappers (`0x01604e80`) returned only Mercury `protocol_digest` machinery, not entity-schema fingerprint logic. Per `datatype-registry-system.md`, MD5 hashing occurs during `DataType_Register @ 0x01597ce0`; the comparison against a wire-provided value remains untraced. Search surface ruled out: `ServerConnection_*` immediate inbound paths (none dispatched on a digest value); the message-catalog inbound rows (none mention "schema" or "digest" by name in plate comments). Path to closure: callers of CryptoPP MD5 functions at `0x01604e80`, and the `EntityDescription_WriteClientData` MD5-feed loop callers.

---

### B.7 — G16: Failure mode — unknown typeID in delegate

**Disposition**: PARTIALLY-RESOLVED. **→ Folded into chapter §1.16 as F4** + §2.5 already records the client-tree side.

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x00dddca0` — `ServerConnection_CreateBasePlayer`
- `ghidra://SGW.exe@0x00a35210` — callee (varargs logger wrapper, not the entity-creation delegate)

**Evidence retained**: `ServerConnection_CreateBasePlayer` calls `FUN_00a35210` (logger) and `ServerConnection_CreateCellPlayer`. The entity-creation delegate at `*(this+0x168)` is a runtime function pointer — static target unresolvable in this pass. The outer handler has **no typeID bounds check** and **no error-recovery** path after the delegate call. Paths ruled out: a pre-call validation gate (none in the prologue); a post-call error branch (none after the delegate return). Failure-mode characterization inside the delegate requires a live session.

---

### B.8 — G17: Failure mode — sub-slot decode mismatch

**Disposition**: RESOLVED. **→ Folded into chapter §1.16 as F5** (silent drop via red-black tree miss).

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x01590bb0` — `MethodDescription_ComputeIdBase`
- `ghidra://SGW.exe@0x00c6f8f0` — `ProcessEntityMethodEmission` (tree-miss path)
- `ghidra://SGW.exe@0x01590f30` — `EntityDescription_GetExposedClientMethodByIndex`

**Evidence retained** — the decode formula pinned at `MethodDescription_ComputeIdBase @ 0x01590bb0`:

```c
idBase = 0x3e - (nExposedCount + 0xc0) / 0xff;
if (nCurrentId >= idBase) {
    extraByte = vtable[1](1);  // read one more byte
    nCurrentId = extraByte + (nCurrentId - idBase) * 0x100 + idBase;
}
```

Out-of-bounds path: `ProcessEntityMethodEmission` reaches the red-black tree miss branch → `EntityDescription_GetExposedClientMethodByIndex` returns `0` → dispatch returns without invoking any handler. Same silent-drop end state as F2, reached via the sub-slot decode rather than the direct table lookup.

---

### B.9 — G18: Failure mode — property update for entity not in table (the load-bearing one)

**Disposition**: RESOLVED. **→ Folded into chapter §1.16 as F6** — the buffered-indefinitely invariant, the one server implementers must respect.

**Ghidra anchor**: `ghidra://SGW.exe@0x00dd2b80` — `GameEntityManager_DispatchEntityRpc`.

**Evidence retained** — the buffering decompile excerpt at `LAB_00dd2c99`:

```c
iVar2 = (*pnByteStream+8)();                   // read byte count
piVar3 = scalable_malloc(0x20);                // allocate MemoryOStream
ConstructMemoryOStream(piVar3, iVar2);
(*pnByteStream+4)(iVar2, iVar2);               // read data into stream
LookupOrEmplaceSecondaryListenerSlot(ESI+0x3c, ...);
FUN_0046eef0(pvVar5, piVar6);                  // enqueue for deferred dispatch
```

Buffer slot is `GameEntityManager+0x3C` (same slot the `LeaveAoI` deferred-leave path writes to). **No TTL, no discard path** — the buffer is held indefinitely. The chapter calls out the implication: server must guarantee `leaveAoI` precedes any late property update for a given entityID to avoid ghost-delivery races.

---

### B.10 — G35: `entityMessage` (msg_id 0x0D) wire format

**Disposition**: RESOLVED. **→ Folded into chapter §1.5** as the volatile cell-method variant + the `RouteEntityMessageToHandler` bit-6 dispatch note.

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x00dd66e0` — `RouteEntityMessageToHandler`
- `ghidra://SGW.exe@0x00dd6a60` — `ServerConnection_StartEntityMessage`
- `ghidra://SGW.exe@0x00dd6690` — `InstallEntityMessageHandlerVtable`

**Evidence retained**: `RouteEntityMessageToHandler @ 0x00dd66e0` reads `flags = *pMsg` and routes on bit 6: `(flags & 0x40)` → `vtable+0x20(flags & 0x3f)` (volatile / unreliable); else → `vtable+0x24(flags & 0x7f, pHandler)` (reliable). `ServerConnection_StartEntityMessage @ 0x00dd6a60` writes outgoing cell-method byte as `(methodIndex & 0x7F) | 0x80` — matches BigWorld 1.9.1 `servconn.cpp::startEntityMessage`. Server→client `entityMessage` (msg_id `0x0D`) dispatches via `RouteEntityMessageToHandler`; if `*(this+0x168) == 0` (no handler installed), the message is silently dropped — same shape as F2 / F5.

Wire layout (client→server cell entity message, confirmed):

1. Byte 0: `(methodIndex & 0x7F) | 0x80` (reliable) or `(methodIndex & 0x3F) | 0xC0` (volatile, when bit 6 set on a cell byte; per the route table not yet observed on the wire).
2. Bytes 1–4: entity ID, u32 LE.
3. Bytes 5+: method arguments (variable).

---

### B.11 — G36: Property change batching

**Disposition**: RESOLVED. **→ Folded into chapter §1.8** as the "no batch-property-change message type" protocol invariant.

**Ghidra anchor**: `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`.

**Evidence retained**: `FNetworkPropertyChange__vfunc_0` writes one property change per invocation — three helper calls (4-byte index write, two string/value writes), no loop. No multi-property batch message type exists. Mercury bundle aggregation (`spec.protocol.mercury-wire-format` §1) makes the appearance of batching at the network layer — each property change remains an independent InterfaceElement, the bundle layer is what packs them into one UDP payload.

---

### B.12 — G37: Method argument serialization (FIXED_DICT / ARRAY / TUPLE)

**Disposition**: PARTIALLY-RESOLVED. **→ Folded into chapter §1.13** (schema virtual at `vtable+0x24` confirmed) + §1.15 OQ-4 (runtime-value virtual still unconfirmed).

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream` (vtable+0x24 dispatch)
- `ghidra://SGW.exe@0x01598b80` — `FixedDictDataType_ToXml` (field layout)

**Evidence retained**: indirect-call pattern `(**(code**)(**(int**)(this+0x1c) + 0x24))(stream)` in `DataDescription_WriteToStream` pins the schema virtual at vtable offset `+0x24` (slot index 9). `FixedDictDataType` in-memory layout from `FixedDictDataType_ToXml`: `+0x10` = `allowNone` flag byte; `+0x18/+0x1c` = field-array begin/end (element stride `0x28`); per field: `+0x04..+0x18` SSO name string, `+0x14` name length, `+0x1c` nested DataType pointer (recursive via `vtable+0x24`). The schema wire layout per FIXED_DICT field is therefore `[name_bytes][nested_type_descriptor_via_vtable+0x24]`. The **runtime value** virtual is a different slot — `+0x28` or `+0x2c` — and was not decompiled in this pass. `ArrayDataType` and `TupleDataType` runtime-value virtuals likewise untraced.

---

### B.13 — G38: Entity reference / mailbox serialization

**Disposition**: PARTIALLY-RESOLVED. **→ Folded into chapter §1.13** (MailBoxDataType vtable identity confirmed) + §1.15 OQ-4 sub-bullet (wire layout untraced).

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x0159b850` — `VMailBoxDataType___SimpleMetaDataType__vfunc_0` (destructor — confirms vtable identity)
- `ghidra://SGW.exe@0x0159b480` — `MailBoxDataType DtorBody`

**Evidence retained**: `FUN_0159b480 @ 0x0159b480` is the MailBoxDataType DtorBody — its instruction sequence (MSVC scalar-destructor + vtable-load to `0x0159b850`) confirms the vtable identity as `SimpleMetaDataType<class_MailBoxDataType>::vftable`. The stream-writer virtual at `vtable+0x24` (the slot pinned by B.12) was not decompiled in this pass. BW 1.9.1 reference for a mailbox wire value: `channelId u16 + indexInComponent u16 + spaceId u32` = 8 bytes; unverified for SGW.

---

### B.14 — G39: Nested property updates / slice mode

**Disposition**: RESOLVED. **→ Folded into chapter §1.8** as the "no slice / no sub-field updates" protocol invariant.

**Ghidra anchor(s)**:

- `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`
- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream`

**Evidence retained**: `FNetworkPropertyChange__vfunc_0` writes one complete property change per call with **no inner-field selector**. `FixedDictDataType_ToXml` iterates all fields in a flat loop with no slice-index field. Paths ruled out: a `PROPERTY_CHANGE_TYPE_SLICE` constant anywhere in `.data` (none found); a sub-field-index parameter on the property-change emitter (not present). Any change to a field inside a `FIXED_DICT` property re-serializes the **full** property value.

---

### B.15 — G40: Property default values omitted from wire?

**Disposition**: RESOLVED. **→ Folded into chapter §1.8** as the "no client-side default-value filtering" protocol invariant.

**Ghidra anchor**: `ghidra://SGW.exe@0x01590fc0` — `EntityDescription_WriteClientData`.

**Evidence retained**: `EntityDescription_WriteClientData @ 0x01590fc0` emits all matching DataDescriptions unconditionally — the loop body has no default-value comparison and no skip-on-default branch. Per Appendix A, the SGW client-property filter matches zero properties anyway, so the schema-write loop produces zero entries for SGW entities. For actual property *values* (sent in the `createBasePlayer` data stream), default-omission behavior is server-side; the client reads whatever the server sends with no default-filtering logic visible.

---

### B.16 — New anchors to add to chapter `evidence_refs.re`

The following addresses were confirmed in this pass and are not yet in Appendix A's anchor list:

| Address | Symbol | Notes |
|---------|--------|-------|
| `ghidra://SGW.exe@0x00dd29d0` | `EntityManager_LeaveAoI` | G4 |
| `ghidra://SGW.exe@0x00dd2270` | `EntityManager_HandleEntityCreate` | G3 |
| `ghidra://SGW.exe@0x00dd2800` | `EntityManager_EnterAoI` | G3 cascade context |
| `ghidra://SGW.exe@0x00dd2b80` | `GameEntityManager_DispatchEntityRpc` | G14, G18 |
| `ghidra://SGW.exe@0x00c6f8f0` | `ProcessEntityMethodEmission` | G14, G17 |
| `ghidra://SGW.exe@0x01590bb0` | `MethodDescription_ComputeIdBase` | G17 |
| `ghidra://SGW.exe@0x01590f30` | `EntityDescription_GetExposedClientMethodByIndex` | G14, G17 |
| `ghidra://SGW.exe@0x01590fc0` | `EntityDescription_WriteClientData` | G5, G36, G40 |
| `ghidra://SGW.exe@0x015958b0` | `DataDescription_WriteToStream` | G5, G37, G39 |
| `ghidra://SGW.exe@0x01598b80` | `FixedDictDataType_ToXml` | G37 |
| `ghidra://SGW.exe@0x0159b480` | MailBoxDataType DtorBody | G38 |
| `ghidra://SGW.exe@0x0159b850` | `VMailBoxDataType___SimpleMetaDataType__vfunc_0` | G38 |
| `ghidra://SGW.exe@0x00dd66e0` | `RouteEntityMessageToHandler` | G35 |
| `ghidra://SGW.exe@0x00dd6a60` | `ServerConnection_StartEntityMessage` | G35 |
| `ghidra://SGW.exe@0x00dd6690` | `InstallEntityMessageHandlerVtable` | G35 |
| `ghidra://SGW.exe@0x015652d0` | `FNetworkPropertyChange__vfunc_0` | G13, G36, G39 |

---

## Appendix C -- Wire-capture validation (2026-05-16)

Investigator: Game Archaeology Specialist (automated, Claude Sonnet 4.6).
Tool: `tools/entity_property_sync_resolver.py` (new, ~350 LOC; forks
`tools/mercury_dispute_resolver.py` infrastructure, extends with
entity-property-sync-specific message parsing).

### C.0 -- Summary

| Validation | Disposition | Evidence |
|------------|-------------|----------|
| V1: sub-slot threshold (idBase=61 vs 62) | CONFIRMS | 29 sub-slot packets; 5 Rust-unique, 0 spec-unique |
| V2: createBasePlayer layout | CONFIRMS (layout); PARTIAL (no SGWPlayer instance) | 3 msgs; u32+u16+stream at offsets 0/4/6 confirmed |
| V3: createCellPlayer 32-byte fixed | CONFIRMS | 1 msg, exactly 32 bytes, vehicleId=0 |
| V4: propID 0x3C/0x3D thresholds | NOT-CONFIRMED-IN-CAPTURE | No updateEntity (0x0A) msgs in session |
| V5: cell-method 0x80..0xBF | CONFIRMS | 112 cell-method bytes; all in range |
| V6: base-method 0xC0..0xFF | CONFIRMS | 39 base-method bytes; all in range |
| V7: createCellPlayer Y/Z rotation swap | NOT-DETERMINABLE | Single msg; all rotation values 0.0 |
| V8: enableEntities 8-byte body | CONFIRMS | 9 msgs, all 8 bytes, content undefined |

### C.1 -- Setup

- **Pcap**: `game/sgw/Working/binaries/sessions/2026-05-16_08-21.pcap`
- **Keys**: `game/sgw/Working/binaries/sessions/2026-05-16_08-07-keys.txt`
  - First key (64-char hex): `E94459B43FAE75ED...C345CFE2` (AES-256 session key)
  - Note: file contains 5 keys; the first (longest) was used; pcap decryption succeeded

    at 99.95% (10315/10320 packets)

- **Packet stats**: 10320 UDP total; 10315 decrypted; 4366 server-to-client; 2377

  client-to-server; 1577 undirected (direction detection by 2-sided port heuristic,
  server port detected as 32832; confirmed by session log `2026-05-16_08-07.log` line 39:
  `Nub::registerChannel: registering channel from address 127.0.0.1:32832`)

- **Tool**: `tools/entity_property_sync_resolver.py` -- ~350 LOC, reuses

  `pcap_dissect.py` decryption and footer-parse primitives; adds entity-method
  iteration, createBasePlayer/createCellPlayer payload decode, updateEntity first-byte
  scan, cell/base method byte histograms, enableEntities extraction

### C.2 -- V1: Sub-slot threshold (idBase=61 vs 62)

**Disposition**: CONFIRMS. **→ Already wire-confirmed in chapter §1.4** prior to this pass; this capture re-confirms from a second pcap. No chapter change required.

**Wire evidence retained** (29 0xBD-prefixed sub-slot messages, sub_idx histogram):

| sub_idx | count | spec interp (sub+62) | Rust interp (sub+61) |
|---------|-------|----------------------|----------------------|
| 4 | 1 | method 66 (unnamed) | method 65 = SETUP_STARGATE_INFO |
| 19 | 1 | method 81 = ON_STORE_UPDATE | method 80 = ON_STORE_OPEN |
| 20 | 1 | method 82 (unnamed) | method 81 = ON_STORE_UPDATE |
| 21 | 2 | method 83 (unnamed) | method 82 (unnamed) |
| 41 | 1 | method 103 (unnamed) | method 102 = ON_TIME_OF_DAY |
| 44 | 1 | method 106 (unnamed) | method 105 = ON_DIALOG_DISPLAY |
| 61 | 1 | method 123 (unnamed) | method 122 = SETUP_WORLD_PARAMETERS |
| 63 | 1 | method 125 = ADD_CLIENT_HINTED_GENERIC_REGION | method 124 = CLEAR_HINTED_REGIONS |
| 64 | 19 | method 126 = ON_RESET_MAP_INFO | method 125 = ADD_CLIENT_HINTED_GENERIC_REGION |
| 91 | 1 | method 153 (unnamed) | method 152 (unnamed) |

5 Rust-unique observations (only make sense under idBase=61): sub_idx=4 → SETUP_STARGATE_INFO, sub_idx=41 → ON_TIME_OF_DAY, sub_idx=44 → ON_DIALOG_DISPLAY, sub_idx=61 → SETUP_WORLD_PARAMETERS (the landmark method whose name matches its method-index exactly under idBase=61). Zero spec-unique observations. Consistent with the prior `sessions/2026-05-15_14-05.pcap` validation (18 Rust-unique, 0 spec-unique).

### C.3 -- V2: createBasePlayer (msg_id 0x05) wire layout

**Disposition**: CONFIRMS layout. **→ Folded into chapter §1.6** "Wire-confirmed" callout.

**Wire evidence retained** — 3 createBasePlayer messages, raw payloads:

```text
Msg 1: 01 00 00 00 07 00   entityId=1  typeId=7  (0x0007)  prop_stream=0 bytes
Msg 2: 02 00 00 00 02 00   entityId=2  typeId=2  (0x0002)  prop_stream=0 bytes
Msg 3: 01 00 00 00 07 00   entityId=1  typeId=7  (0x0007)  prop_stream=0 bytes
```

typeId=2 = SGWBeing, typeId=7 = SGWDuelMarker — both pre-session entities. The session ended before a SGWPlayer (typeId=3) emission with non-empty property stream; **layout is confirmed, non-empty-stream case is not yet wire-witnessed**. The 0-byte property streams are consistent with both entity types having no OWN_CLIENT / OTHER_CLIENT props (per the §1.2 / §2.3 keyword-surface findings).

### C.4 -- V3: createCellPlayer (msg_id 0x06) -- fixed 32-byte payload

**Disposition**: CONFIRMS. **→ Folded into chapter §1.7** as a worked-example "Wire-confirmed" block with the concrete Atrea spawn coords.

**Wire evidence retained** — 1 message, 32 bytes exact:

```text
Raw (32 bytes):
10 00 01 00  00 00 00 00  91 1D A7 C3  AA F1 92 42
A8 06 64 C3  00 00 00 00  00 00 00 00  00 00 00 00

Decoded:
  Offset  0-3:  spaceId   = 0x00010010 = 65552
  Offset  4-7:  vehicleId = 0x00000000 = 0
  Offset  8-11: posX      = -334.231  (0xC3A71D91)
  Offset 12-15: posY      =   73.472  (0x4292F1AA)
  Offset 16-19: posZ      = -228.026  (0xC36406A8)
  Offset 20-23: rotX      = 0.0000
  Offset 24-27: rotZ      = 0.0000  (chapter slot: yaw)
  Offset 28-31: rotY      = 0.0000  (chapter slot: roll)
```

WORD_LENGTH framing gave 32 as the payload length; iterator consumed the full payload with no remainder.

**V7 note retained**: all rotation values 0.0 → swap claim not wire-distinguishable from this capture (folded into chapter §1.7 as an inline NOTE callout). The Ghidra evidence (`FUN_015846a0` applying the swap internally) remains the primary citation.

### C.5 -- V4: Property-change propID encoding (OQ-1 / G7)

**Disposition**: NOT-CONFIRMED-IN-CAPTURE. **→ Folded into chapter §1.15 OQ-1** with capture-attempt status + path to closure.

**Wire evidence retained**: zero `updateEntity` (msg_id `0x0A`) messages in this pcap. No `0x3C` or `0x3D` first-bytes in server→client payloads. Capture window ended at `EntityManager::disconnected` shortly after world entry — before any sustained in-world property changes (stat updates, inventory churn, ability use). Path to closure documented in chapter §1.15.

### C.6 -- V5: Cell-method wire byte mask

**Disposition**: CONFIRMS. **→ Folded into chapter §1.5** "Wire-confirmed" callout.

**Wire evidence retained** — 112 cell-range bytes (0x80..0xBF), top 10 histogram:

```text
0xBD  n=35   methodId=61 (0x3D)  -- sub-slot sentinel (extended encoding)
0x80  n=26   methodId=0           -- ON_SEQUENCE
0x84  n=7    methodId=4           -- ON_ENTITY_FLAGS
0x9A  n=6    methodId=26          -- BEING_APPEARANCE
0x8A  n=6    methodId=10          -- ON_ENTITY_TINT
0x83  n=4    methodId=3           -- INTERACTION_TYPE
0x8F  n=3    methodId=15          -- ON_LEVEL_UPDATE
0x93  n=3    methodId=19          -- ON_STATE_FIELD_UPDATE
0x88  n=3    methodId=8           -- ON_VISIBLE
0x82  n=3    methodId=2           -- (unnamed in METHOD_IDX)
```

All 112 in `0x80..0xBF`. The 35× `0xBD` is the sub-slot sentinel, consistent with V1.

### C.7 -- V6: Base-method wire byte mask + 0xFF boundary

**Disposition**: CONFIRMS. **→ Folded into chapter §1.5** "Wire-confirmed" callout; the **0xFF boundary case is the new contribution** (one observation, methodId=63 → `(63 & 0x3F) | 0xC0 = 0xFF`, explicit witness to the 6-bit mask at its boundary).

**Wire evidence retained** — 39 base-range bytes (0xC0..0xFF), top 10 histogram:

```text
0xC0  n=24   methodId=0           -- (unnamed in METHOD_IDX; likely base method 0)
0xC6  n=4    methodId=6
0xC7  n=2    methodId=7           -- ON_ENTITY_PROPERTY
0xD5  n=2    methodId=21          -- ON_STAT_BASE_UPDATE
0xFF  n=1    methodId=63          -- (at mask boundary: (63 & 0x3F) | 0xC0 = 0xFF)
0xC3  n=1    methodId=3           -- INTERACTION_TYPE
0xC4  n=1    methodId=4           -- ON_ENTITY_FLAGS
0xD8  n=1    methodId=24          -- ON_ALIGNMENT_UPDATE
0xDD  n=1    methodId=29
0xD6  n=1    methodId=22
```

All 39 in `0xC0..0xFF`.

### C.8 -- V7: Y/Z rotation swap in createCellPlayer

**Disposition**: NOT-DETERMINABLE. See C.4 — all rotation values 0.0. **→ Folded into chapter §1.7** as a NOTE callout pinning the swap claim as static-decompile-only.

### C.9 -- V8: enableEntities (msg_id 0x08, client-to-server) 8-byte body

**Disposition**: CONFIRMS. **→ Folded into chapter §1.17 as new gotcha S6** — the "scri" stack-frame artifact is the worked example.

**Wire evidence retained** — 9 messages, all 8 bytes, client→server. Representative payloads:

```text
Msg 1: 00 00 00 C0 40 44 00 80   (float-looking bytes; not a u32 pair)
Msg 2: 00 00 00 80 F8 43 00 C0
Msg 3: 73 00 63 00 72 00 69 00   (ASCII 's','c','r','i' -- fragment of wide-string)
Msg 4: 86 80 4C 4B 59 99 F8 B2   (random-looking bytes)
Msg 5: 8F 5A C7 82 94 50 77 AA
```

Content varies per message; no pattern consistent with `[i32 entityId][i32 flag]`. The W-enable-entities mercury finding (`ghidra://SGW.exe@0x00dd928f`) showed 8 bytes written from an uninitialized / reused stack region — Msg 3's `"scri"` ASCII pattern is the smoking gun, consistent with a prior wide-string stack frame contaminating the buffer.

### C.10 -- Net effect on chapter confidence

All wire-capture promotions and the OQ-status updates have been folded into the chapter directly — see the chapter's §1.5, §1.6, §1.7, §1.8, §1.15 OQ-1, and §1.16 S6 entries for the new wire-confirmed language. This subsection is preserved as the audit ledger only.

**Promotions reflected in the chapter** (was → now, all in `docs/drafts/spec/entity-property-sync.md`):

| Section | Claim | Was | Now |
|---------|-------|-----|-----|
| §1.4 | SGWPlayer idBase=61 | HIGH (prior wire evidence) | RE-CONFIRMED (second pcap) |
| §1.5 | Cell-method wire byte 0x80..0xBF | HIGH (Ghidra) | WIRE-CONFIRMED |
| §1.5 | Base-method wire byte 0xC0..0xFF + 0xFF boundary | HIGH (Ghidra) | WIRE-CONFIRMED |
| §1.6 | createBasePlayer layout: u32+u16+stream | MEDIUM (BW source) | WIRE-CONFIRMED (layout only — property stream non-zero case still pending) |
| §1.7 | createCellPlayer 32-byte fixed | MEDIUM (Ghidra only) | WIRE-CONFIRMED (32-byte payload + concrete spawn coords worked-example) |
| §1.7 | vehicleId=0 at world entry | Assertion | WIRE-CONFIRMED |
| §1.17 S6 (new) | enableEntities 8-byte undefined body | new chapter gotcha | WIRE-CONFIRMED via "scri" stack-frame artifact |

**Open questions that remain open after this pass** (now tracked in chapter §1.15):

| OQ | Status | Path to closure |
|----|--------|-----------------|
| OQ-1: propID 0x3C/0x3D thresholds | STILL OPEN | 60+ s in-world capture with property changes (updateEntity msg_id 0x0A); cheapest probe is an `ON_STAT_UPDATE` from a health change |
| V7 → §1.7 footnote: createCellPlayer Y/Z rotation swap | NOT-DETERMINABLE | Capture with non-zero spawn orientation |
| OQ-X (new): inbound propID decoder location | OPEN | Callers of `EntityDescription_GetClientPropertyByIndex @ 0x01590d80` + `ServerConnection_*` inbound dispatchers (linked to chapter F1) |

**spaceId format observation** (not folded — minor footnote candidate). The capture's `spaceId = 65552 = 0x00010010` is a compound field — high word `0x0001` is space type/category, low word `0x0010` is instance — matching the BigWorld convention. Captured here in case a future §1.7 footnote wants it; not currently surfaced in the chapter.
