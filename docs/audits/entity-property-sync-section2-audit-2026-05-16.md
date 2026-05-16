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

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00dd2270` — `EntityManager_HandleEntityCreate`
- `ghidra://SGW.exe@0x00dd2800` — `EntityManager_EnterAoI`
- `ghidra://SGW.exe@0x00dd24f0` — `GameEntityManager_FinishEntityLoad`

**Finding**: The chapter's 7-method cascade is not explicitly enumerated in SGW.exe. The client receives a BW `onEntityCreate` message, which `EntityManager_HandleEntityCreate @ 0x00dd2270` dispatches. That function calls `EntityManager_CreateEntity`, applies an initial world transform via `FUN_00e68a10`, then calls `GameEntityManager_FlushDeferredNotifications`. There is no client-side loop that iterates a fixed seven-method list. The method order is entirely determined by the server: whatever property delta sequence the Python `createOnClient()` chain produces (`SGWSpawnableEntity.py` → `SGWBeing.py` → `SGWMob.py`) is what arrives on the wire; the client decodes them in arrival order. The decompile shows no ordered dispatch table — it uses `GameEntityManager_DispatchEntityRpc @ 0x00dd2b80` for each incoming method call independently.

The "deprecated server source" `CachedEntity::onEntityVisible` at `cached_entity.cpp:173` is embedded in debug strings of SGW.exe. The actual dispatch is in `EntityManager_HandleEntityCreate`, confirmed. No ordered 7-method enumeration exists client-side.

**Recommendation for chapter**: Replace the "agent memory" citation in §1.9 / Figure 6 with:

> The 7-method cascade order is **server-determined**. `EntityManager_HandleEntityCreate @ ghidra://SGW.exe@0x00dd2270` receives and dispatches each method as it arrives on the wire with no client-side ordering constraint. The canonical sequence shown in Figure 6 reflects the order produced by the Python `createOnClient()` chain in `SGWMob.py` → `SGWBeing.py` → `SGWSpawnableEntity.py` on the SGW server. The client decodes whatever the server sends in whatever order it sends it.

---

### B.2 — G4: `leaveAoI` handler Ghidra anchor

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00dd29d0` — `EntityManager_LeaveAoI`

**Finding**: Decompiled in full. The function does **not** decrement a reference count (the prior plate comment "decrements reference count" was wrong). It dispatches or defers a **method call** on leave. Sequence:

1. If debug flag `g_bEntityRpcDebug (DAT_01ef2224)` set, logs entity-ID and space-ID.
2. Searches primary entity map (`this+0x18`) for the leaving entity ID.
3. **Path A — entity NOT in primary map**: executes stream callback directly via `nSpaceId->vtable[2]()`.
4. **Path B — entity IS in primary map**: reads stream byte-count, allocates a 0x20-byte `MemoryOStream` via `scalable_malloc`, constructs it with `ConstructMemoryOStream`, copies stream data into it, then queues it to the deferred-leave slot at `this+0x3C` via `LookupOrEmplaceSecondaryListenerSlot` + `FUN_0046eef0`.

There is no entity-table removal or CME `Event_EntityLeftAoI` emission visible at this function level — deferred leave delivery happens when the slot is flushed. The entity reference is not explicitly freed here.

**Recommendation for chapter**: Add to §1.10:

> `EntityManager_LeaveAoI @ ghidra://SGW.exe@0x00dd29d0` — BW `onEntityLeaveAoI` virtual. Does not free the entity immediately. If the entity is in the primary map, the leave method call is buffered to a deferred slot at `GameEntityManager+0x3C` (distinct from the entry deferred slot at `+0x30`). Entity destruction occurs when the deferred slot is flushed, not at this call site.

---

### B.3 — G5: `CLIENT_DATA | BASE_DATA` filter mask numeric values

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x01590fc0` — `EntityDescription_WriteClientData`
- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream`
- `ghidra://SGW.exe@0x015924a0` — `EntityDescription_ParseProperties` (from Appendix A)

**Finding**: Numeric filter values from decompiles:

- `EntityDescription_WriteClientData @ 0x01590fc0` loops DataDescription array at `EntityDesc+0x60/+0x64` (element stride 0x40). Gate: `*(byte*)(pvDataDesc + 0x20) & 6 != 0`. Flag `0x06 = DATA_OTHER_CLIENT (0x02) | DATA_OWN_CLIENT (0x04)`. This is "CLIENT_DATA".
- `DataDescription_WriteToStream @ 0x015958b0` masks flags with `0x5f` before wire: strips `DATA_PERSISTENT (0x20)` and `DATA_ID (0x80)`.
- Combined masks: `CLIENT_DATA | BASE_DATA = 0x06 | 0x08 = 0x0E`; `CLIENT_DATA | CELL_DATA = 0x06 | 0x01 = 0x07`.

Per Appendix A, in SGW no property sets bits 1 or 2 (all use `CELL_PUBLIC=0x01`, `BASE=0x08`, or `CELL_PRIVATE=0x00`), so `WriteClientData` produces zero DataDescription entries for any SGW entity.

**Recommendation for chapter**: Add to §1.11:

> Numeric filter values: `CLIENT_DATA = flags & 0x06` (`DATA_OTHER_CLIENT=0x02 | DATA_OWN_CLIENT=0x04`); `BASE_DATA = DATA_BASE = 0x08`; `CELL_DATA = DATA_GHOSTED = 0x01`. Combined: `CLIENT_DATA|BASE_DATA = 0x0E`, `CLIENT_DATA|CELL_DATA = 0x07`. Flags stripped before wire: `DATA_PERSISTENT (0x20)` and `DATA_ID (0x80)` cleared via `& 0x5f` mask in `DataDescription_WriteToStream @ ghidra://SGW.exe@0x015958b0`. **SGW note**: no SGW `.def` property satisfies `flags & 0x06 != 0`, so this filter matches zero properties for all SGW entities (see Appendix A §1.2 reconciliation).

---

### B.4 — G13: Failure mode — propID outside valid range

**Disposition**: PARTIALLY-RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`

**Finding**: `FNetworkPropertyChange__vfunc_0 @ 0x015652d0` is an **outgoing** property-change serializer (calls Mercury bundle write helpers), not the inbound handler. The inbound propID decoder and bounds checker are in an upstream Mercury handler not yet identified by name in this pass. Without a confirmed inbound handler address, out-of-bounds behavior cannot be characterized.

**Recommendation for chapter**: Mark in §1.16 Failure Modes:

> **S-G13 (UNRESOLVED)**: If a property-change message arrives with a propID exceeding the entity's DataDescription array bounds, the behavior is not confirmed. `FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0` is the outgoing serializer; the inbound decoder was not located. Likely in the Mercury incoming message handlers near `ServerConnection_*`. A live x64dbg session with a crafted oversized propID is needed to determine crash vs. silent drop.

---

### B.5 — G14: Failure mode — methodID not in table

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00c6f8f0` — `ProcessEntityMethodEmission`
- `ghidra://SGW.exe@0x01590f30` — `EntityDescription_GetExposedClientMethodByIndex`

**Finding**: `ProcessEntityMethodEmission @ 0x00c6f8f0` handles unknown methodID via an explicit 0xFFFF guard:

```c
uVar3 = EntityDescription_FindMethodIdByName(pvVar5, *(ushort*)(pEntityDesc + 0x14));
if (uVar3 == 0xffff) {
    FUN_00482ff0(L"No client->server entity description mapping found for entity type %d; message id: %d.", ...);
    // falls through / returns
}
```

For a method index with no registered listener, the red-black tree traversal exits the miss branch and calls `EntityDescription_GetExposedClientMethodByIndex` which returns 0 on out-of-bounds. The function returns without dispatch. **Both paths: silent drop with an optional debug log. No crash, no disconnect.**

**Recommendation for chapter**: Add to §1.16 Failure Modes:

> **S-G14**: An incoming method byte that decodes to a methodID with no registered listener results in a silent drop after logging `"No client->server entity description mapping found for entity type %d; message id: %d."` (wide string). Confirmed in `ProcessEntityMethodEmission @ ghidra://SGW.exe@0x00c6f8f0`. The log is only active when `g_bEntityRpcDebug (DAT_01ef2224)` is set. No crash, no disconnect.

---

### B.6 — G15: Failure mode — MD5 schema fingerprint mismatch

**Disposition**: UNRESOLVED

**Ghidra anchor(s)**: None confirmed.

**Finding**: Searches for `MD5_Finalize`, `MD5_DigestToHexString`, and related names returned only CryptoPP cipher-layer wrappers — not entity schema fingerprint logic. The schema-digest comparison site was not located. Per `datatype-registry-system.md`, MD5 hashing occurs during `DataType_Register @ 0x01597ce0`; the comparison against a wire-provided value requires a separate investigation from that entry point.

**Recommendation for chapter**: Mark in §1.16:

> **S-G15 (UNRESOLVED)**: The site where the client compares a schema MD5 fingerprint against a server-provided value has not been located. Starting point for a future pass: `DataType_Register @ ghidra://SGW.exe@0x01597ce0` and callers of the CryptoPP MD5 functions (`0x01604e80`). The Mercury `protocol_digest` (32-char MD5 hex) is confirmed per the Mercury §2 findings; whether entity schema digest is checked separately is unknown.

---

### B.7 — G16: Failure mode — unknown typeID in delegate

**Disposition**: PARTIALLY-RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00dddca0` — `ServerConnection_CreateBasePlayer`
- `ghidra://SGW.exe@0x00a35210` — callee of `CreateBasePlayer` (log/diagnostic, not the entity-creation delegate)

**Finding**: `ServerConnection_CreateBasePlayer` calls two functions: `FUN_00a35210` (a varargs logger wrapper) and `ServerConnection_CreateCellPlayer`. The entity-creation delegate at `*(this+0x168)` is a runtime function pointer; its target was not resolved statically. The outer handler has no typeID bounds check and no error-recovery after the delegate call. Failure mode inside the delegate requires a live session.

**Recommendation for chapter**: Update §1.16:

> **S-G16 (PARTIALLY-RESOLVED)**: `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` passes typeID directly to the entity-creation delegate at `*(this+0x168)` without validation. The delegate is a runtime function pointer; its behavior on unknown typeID requires a live x64dbg session. No error-recovery is visible in the outer handler.

---

### B.8 — G17: Failure mode — sub-slot decode mismatch

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x01590bb0` — `MethodDescription_ComputeIdBase`
- `ghidra://SGW.exe@0x00c6f8f0` — `ProcessEntityMethodEmission` (tree-miss path)
- `ghidra://SGW.exe@0x01590f30` — `EntityDescription_GetExposedClientMethodByIndex`

**Finding**: Sub-slot decode formula from `MethodDescription_ComputeIdBase @ 0x01590bb0`:

```c
idBase = 0x3e - (nExposedCount + 0xc0) / 0xff;
if (nCurrentId >= idBase) {
    extraByte = vtable[1](1);  // read one more byte
    nCurrentId = extraByte + (nCurrentId - idBase) * 0x100 + idBase;
}
```

If the decoded `nCurrentId` exceeds `exposedMethodCount`, `ProcessEntityMethodEmission` reaches the red-black tree miss branch and calls `EntityDescription_GetExposedClientMethodByIndex` which returns 0. The dispatch returns without calling any handler. **Silent drop, no crash, no disconnect.**

**Recommendation for chapter**: Add to §1.16 Failure Modes:

> **S-G17**: A wire method-byte sequence decoding (via `MethodDescription_ComputeIdBase @ ghidra://SGW.exe@0x01590bb0`, formula `idBase = 0x3E - (N+0xC0)/0xFF`) to a method index exceeding the entity's exposed-method count results in a red-black tree miss in `ProcessEntityMethodEmission @ ghidra://SGW.exe@0x00c6f8f0`. Silent drop. No crash, no disconnect. `EntityDescription_GetExposedClientMethodByIndex @ ghidra://SGW.exe@0x01590f30` returns 0 on out-of-bounds.

---

### B.9 — G18: Failure mode — property update for entity not in table

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00dd2b80` — `GameEntityManager_DispatchEntityRpc`

**Finding**: `GameEntityManager_DispatchEntityRpc @ 0x00dd2b80` handles the "entity not found" case by **buffering**, not dropping. When entityID is absent from the primary map and is not the controlled entity, execution reaches `LAB_00dd2c99`:

```c
iVar2 = (*pnByteStream+8)();                   // read byte count
piVar3 = scalable_malloc(0x20);                // allocate MemoryOStream
ConstructMemoryOStream(piVar3, iVar2);
(*pnByteStream+4)(iVar2, iVar2);               // read data into stream
LookupOrEmplaceSecondaryListenerSlot(ESI+0x3c, ...);
FUN_0046eef0(pvVar5, piVar6);                  // enqueue for deferred dispatch
```

The message is held indefinitely in the deferred slot at `GameEntityManager+0x3C`. If the entity never re-enters, the buffer is never flushed. **A server-side guarantee that LeaveAoI always precedes late property updates is required to prevent ghost deliveries.**

**Recommendation for chapter**: Add to §1.16 Failure Modes:

> **S-G18**: A property-update or method-call arriving for an entity absent from the client's entity table (race: entity left AoI) is **buffered, not dropped** by `GameEntityManager_DispatchEntityRpc @ ghidra://SGW.exe@0x00dd2b80`. The payload is held in a `MemoryOStream` at `GameEntityManager+0x3C` and delivered if the entity re-enters. No TTL or discard path exists. The server must ensure leaveAoI always precedes any late property updates for a given entity to prevent ghost delivery.

---

### B.10 — G35: `entityMessage` (msg_id 0x0D) wire format

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x00dd66e0` — `RouteEntityMessageToHandler`
- `ghidra://SGW.exe@0x00dd6a60` — `ServerConnection_StartEntityMessage`
- `ghidra://SGW.exe@0x00dd6690` — `InstallEntityMessageHandlerVtable`

**Finding**: Entity messages use a flags-byte protocol. `RouteEntityMessageToHandler @ 0x00dd66e0` reads `flags = *pMsg` and routes:

- `flags & 0x40` (bit 6 set): calls `vtable+0x20(flags & 0x3f)` — volatile/unreliable path.
- else: calls `vtable+0x24(flags & 0x7f, pHandler)` — reliable entity message.

`ServerConnection_StartEntityMessage @ 0x00dd6a60` writes the outgoing cell-method byte as `(methodIndex & 0x7F) | 0x80`. The `0x80` high bit is the cell-message marker; bits 0–6 carry the method index. This matches BigWorld 1.9.1 `servconn.cpp::startEntityMessage` convention.

**Client→server cell entity message wire layout** (confirmed):
1. Byte 0: `(methodIndex & 0x7F) | 0x80`
2. Bytes 1–4: entity ID (4 bytes, little-endian)
3. Remaining: method arguments (variable)

**Server→client `entityMessage` (msg_id 0x0D)**: dispatched via `RouteEntityMessageToHandler`. If `*(this+0x168) == 0` (no handler), message is silently dropped.

**Recommendation for chapter**: Add a §2.X `entityMessage` subsection:

> `entityMessage` (msg_id `0x0D`) client→server cell layout: `flags[1] | entityId[4] | args[variable]`. Flags: high bit `0x80` = cell-message marker; bit 6 `0x40` = volatile (unreliable); bits 0–5 = method index (volatile) or bits 0–6 (reliable). Server→client direction: `RouteEntityMessageToHandler @ ghidra://SGW.exe@0x00dd66e0` dispatches based on flags byte. Silent drop if no handler registered at `GameEntityManager+0x168`. Confirmed at `ServerConnection_StartEntityMessage @ ghidra://SGW.exe@0x00dd6a60`.

---

### B.11 — G36: Property change batching

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`

**Finding**: `FNetworkPropertyChange__vfunc_0` writes one property change per invocation — three helper calls (4-byte index write, two string/value writes), no loop. There is no multi-property batch message type. Mercury's bundle mechanism auto-aggregates multiple sequential InterfaceElement messages into one UDP payload via cwndsize/MTU, giving the appearance of batching at the network layer. Each property change remains an independent InterfaceElement.

**Recommendation for chapter**: Add to §1.X (property streaming):

> Each property change is its own InterfaceElement; there is no batch-property-change message type (`FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0` writes one property per call). Multiple simultaneous property changes arrive as consecutive InterfaceElements in the same Mercury bundle via the bundle aggregation layer.

---

### B.12 — G37: Method argument serialization (FIXED_DICT / ARRAY / TUPLE)

**Disposition**: PARTIALLY-RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream` (vtable+0x24 dispatch)
- `ghidra://SGW.exe@0x01598b80` — `FixedDictDataType_ToXml` (field layout)

**Finding**: `DataDescription_WriteToStream @ 0x015958b0` confirms the DataType schema-stream virtual is at **vtable offset +0x24** (index 9):

```c
(**(code**)(**(int**)(this+0x1c) + 0x24))(stream);
```

`FixedDictDataType_ToXml @ 0x01598b80` reveals `FixedDictDataType` in-memory layout:
- `+0x10`: `allowNone` flag byte
- `+0x18/+0x1c`: field array begin/end (element stride `0x28` = 40 bytes)
- Per field: `+0x04..+0x18` name string (SSO); `+0x14` name length; `+0x1c` = nested DataType pointer (dispatched via `vtable+0x24` recursively)

Wire schema layout per FIXED_DICT field: `[name_bytes][nested_type_descriptor_via_vtable+0x24]`.

The `vtable+0x24` slot is the **schema-descriptor writer** (used when sending entity schema). Runtime value serialization (property update values) uses a different virtual not yet identified — likely `+0x28` or `+0x2c`. `ArrayDataType` and `TupleDataType` stream virtuals not decompiled in this pass.

**Recommendation for chapter**: Add to §2.X:

> DataType schema serialization dispatches via `vtable+0x24` (index 9), confirmed at `DataDescription_WriteToStream @ ghidra://SGW.exe@0x015958b0`. `FIXED_DICT` schema wire: tag `"FixedDict"[10]`, `allowNone[1]`, then per-field: `[name_bytes][recursive_type_via_vtable+0x24]` (`FixedDictDataType_ToXml @ ghidra://SGW.exe@0x01598b80`). Runtime value serialization uses a different virtual (vtable+0x28 or +0x2c — unconfirmed); a follow-up pass is recommended.

---

### B.13 — G38: Entity reference / mailbox serialization

**Disposition**: PARTIALLY-RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x0159b850` — `VMailBoxDataType___SimpleMetaDataType__vfunc_0` (destructor)
- `ghidra://SGW.exe@0x0159b480` — MailBoxDataType DtorBody (confirms vtable identity)

**Finding**: `FUN_0159b480 @ 0x0159b480` is the MailBoxDataType DtorBody. It confirms the vtable is `SimpleMetaDataType<class_MailBoxDataType>::vftable`. The stream-writer virtual at `vtable+0x24` was not decompiled in this pass. BW 1.9.1 reference format for a mailbox wire value: `channelId[2] + indexInComponent[2] + spaceId[4]` = 8 bytes total; this is the expected SGW format but is not binary-confirmed.

**Recommendation for chapter**:

> **G38 (PARTIALLY-RESOLVED)**: `MailBoxDataType` vtable confirmed at `ghidra://SGW.exe@0x0159b850`. Wire layout not yet confirmed from binary; BW 1.9.1 reference is `channelId[2] + indexInComponent[2] + spaceId[4]` (8 bytes). Verify by decompiling `SimpleMetaDataType<class_MailBoxDataType>::vftable` slot `+0x24`.

---

### B.14 — G39: Nested property updates / slice mode

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x015652d0` — `FNetworkPropertyChange__vfunc_0`
- `ghidra://SGW.exe@0x015958b0` — `DataDescription_WriteToStream`

**Finding**: No evidence of `PROPERTY_CHANGE_TYPE_SLICE` or sub-field selection exists in the SGW client binary. `FNetworkPropertyChange__vfunc_0` writes one complete property change per call with no inner-field selector. `FixedDictDataType_ToXml` iterates all fields in a flat loop with no slice-index field. A change to any field within a `FIXED_DICT` property causes the full dict value to be re-serialized.

**Recommendation for chapter**: Add to §1.X:

> Nested `FIXED_DICT` properties do not support slice updates. A change to any inner field causes the entire property value to be re-sent (`FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0` writes one property at a time with no sub-field selector).

---

### B.15 — G40: Property default values omitted from wire?

**Disposition**: RESOLVED

**Ghidra anchor(s)**:
- `ghidra://SGW.exe@0x01590fc0` — `EntityDescription_WriteClientData`

**Finding**: `EntityDescription_WriteClientData @ 0x01590fc0` emits all matching DataDescriptions unconditionally — no default-value comparison exists in the loop. However, as established in Appendix A, no SGW property satisfies `flags & 0x06 != 0`, so this loop produces zero entries for SGW entities regardless. For actual property *values* (sent in the `createBasePlayer` data stream), default-omission behavior is server-side; the client reads whatever the server sends with no default-filtering logic visible.

**Recommendation for chapter**: Add to §1.X:

> The client performs no default-value filtering: `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` emits matching properties unconditionally. Default-value omission, if implemented, is a server-side concern. The client will always process any property value the server sends, regardless of whether it matches the `.def` `<Default>`.

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
