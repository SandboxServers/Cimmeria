---
audit_id: entity-property-sync-section2-audit-2026-05-16
audit_date: 2026-05-16
auditor: automated (Claude Sonnet 4.6) under direction of @cadacious
spec_version: docs/drafts/spec/entity-property-sync.md @ commit d180b7c (branch worktree-bible+spec-entity-property-sync)
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
  - 2026-05-16 v3 — Appendices B and C folded into chapter; this doc thinned to evidence-ledger role
  - 2026-05-16 v4 — Appendix D added (OQ-1 inbound decoder follow-up); Appendix E added (msg_id 0x0A handler verification, refutes Appendix D's "visibility-only" framing); supersede markers on Appendix D header and §D.6

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

## Appendix D — OQ-1 inbound decoder findings (2026-05-16 follow-up)

**Session:** post-compaction Ghidra investigation, 2026-05-16.

> [!WARNING] **SUPERSEDED IN PART by Appendix E (2026-05-16).** The "msg_id `0x0A` is a visibility signal not a property-delta carrier" framing in this appendix was REFUTED — see Appendix E for the verified call chain. The `0x3C`/`0x3D` byte-pattern absence finding (§D.3) and the UE3 FArchive bridge identification (§D.2) still stand. OQ-1's closure reason is "thresholds absent from client code" not "architecture mismatch", and the chapter §1.8 framing (property delta rides on `updateEntity`) remains correct.

**Verdict (corrected, see Appendix E for refutation): OQ-1 CLOSED — the `0x3C`/`0x3D` threshold-comparison instructions are absent from SGW.exe client code (exhaustive byte-pattern search §D.3). Thresholds are server-side encoding constants and cannot be verified or falsified from any SGW binary. The "architecture-mismatch" framing in the rest of this appendix — that msg_id `0x0A` is visibility-only and not the property-delta carrier — is wrong; see Appendix E.**

### D.1 — `updateEntity` handler chain (full trace)

The `updateEntity` Mercury message (msg_id `0x0A`) handler was traced from registration to final dispatch.

**Registration** (thunk at `ghidra://SGW.exe@0x017bb570`):

- Descriptor `0x019d13e0` written to `[0x01e51d18]`
- Handler `0x00dd62c0` written to `[0x01e51d1c]`
- Message name string at `ghidra://SGW.exe@0x019d0a60` = `"updateEntity"`

**Handler `ghidra://SGW.exe@0x00dd62c0`** (disassembly-confirmed):

```asm
CMP [ECX+0x168], 0      ; EntityManager null-check
JZ ret
MOV EDX, [ESP+0x4]      ; EDX = Mercury MessageHandler struct ptr
MOV ECX, [ECX+0x168]    ; ECX = EntityManager*
MOV EAX, [ECX]          ; vtable
MOV EDX, [EDX]          ; EDX = *(msg) = entity_id (first 4 bytes of payload)
MOV EAX, [EAX+0x14]     ; vtable slot 5
MOV [ESP+0x4], EDX      ; pass entity_id as arg
JMP EAX                 ; tail-call EntityManager->vtable[5]
RET 4
```

**EntityManager vtable slot 5** (`+0x14`) = `ghidra://SGW.exe@0x00dd0bb0` (decompiler-confirmed):

- Looks up `entity_id` in the listener map at `EntityManager+0x18`
- Calls `FUN_00e68df0(found_entity, 0)` which sets `entity+0x30 = entity+0x31 = 0`
- Calls `FUN_00e688c0(entity)` → `FUN_00768970(entity+8, 1)` — toggles UE3 actor visibility bit in `actor+0x70`

**Conclusion:** `updateEntity` in SGW.exe is the **entity-enters-AoI / becomes-visible signal**. It carries only a 4-byte entity_id on the wire. It does NOT carry a propID-prefixed property delta stream. The current name `GameEntityManager_RemoveEntityListener` for `0x00dd0bb0` is wrong; proposed rename: `GameEntityManager_SetEntityVisible`.

### D.2 — Property change pipeline in SGW.exe (full trace)

Entity property updates flow through a UE3 native property replication bridge, NOT through the BigWorld wire propID encoding.

**Full path** (all addresses Ghidra-decompile-confirmed):

1. `FRemotePropagator.cpp` (`FUN_015605b0 @ ghidra://SGW.exe@0x015605b0`) — collects UE3 actor property deltas and calls `FUN_01565390`
2. `FUN_01565390 @ 0x01565390` — serializes to UE3 FArchive and calls `FListenHelper::vtable[4](FListenHelper_ptr, entity_id, flags, data_ptr, data_len)`
3. `FListenHelper::vfunc_5 @ 0x01561140` — null-checks `this+0x28` and calls `FUN_01560ad0`
4. `FUN_01560ad0 @ 0x01560ad0` — reads type tag (`uint32_t` at buf+0), switches on cases 1-6:
   - Case 1 = `FNetworkPropertyChange` → `FNetworkPropertyChange__vfunc_0(local_c4, stream)`
5. `FNetworkPropertyChange__vfunc_0 @ 0x015652d0`:

   ```c
   FUN_0047f0e0(stream, this+0x2c, 4);  // reads propID as uint32_t into this+0x2c
   ```

   — propID arrives as a full 32-bit integer in UE3 FArchive format, not as a 1-2 byte BigWorld wire field

**`FListenHelper` singleton:** allocated by `FUN_0155f9b0 @ 0x0155f9b0`, stored at `DAT_01ef11fd8`. vtable at `ghidra://SGW.exe@0x01b14d4c`.

**Conclusion:** SGW.exe's entity property update path reads propID as a 4-byte `uint32_t` from a UE3 FArchive — bypassing BigWorld's wire-level propID encoding entirely. The 0x3C/0x3D threshold scheme applies only in the server-side BigWorld layer when it serializes the property delta into the Mercury `updateEntity` payload.

### D.3 — Byte-pattern exhaustive search for 0x3C/0x3D threshold constants

Every reasonable encoding of the threshold comparison was searched in SGW.exe's code:

| Pattern | Hits | Disposition |
|---------|------|-------------|
| `3C 3C 75` (CMP AL,0x3C; JNE) | 1 | XML parser at `0x013461f9` |
| `3C 3C 0F 86` (CMP AL,0x3C; JBE) | 0 | — |
| `3C 3D 0F 86` (CMP AL,0x3D; JBE) | 0 | — |
| `83 F8 3C` (CMP EAX,0x3C) | 20+ | All unrelated: XML, CSS, string parsers |
| `83 F8 3B` (CMP EAX,0x3B — value 59) | 6 | All unrelated parsers |
| `0F B6 ?? 83 F8 3C` (MOVZX + CMP 0x3C) | 0 | — |
| `0F B6 ?? 83 F8 3B` (MOVZX + CMP 0x3B) | 0 | — |
| `3C 00 00 00 3D 00 00 00` (dword constants 60, 61) | 4 | All data segment: Huffman tables |
| `2D 3C 00 00 00` (SUB EAX, 60) | 0 | — |

**Conclusion:** The 0x3C/0x3D threshold comparison instructions are absent from SGW.exe's executable code. This is consistent with the threshold being applied server-side only.

### D.4 — Bonus: F1 (out-of-bounds propID behavior)

Cannot be determined from SGW.exe. The bounds check applies on the server at encode time (`EntityDescription::addChangeToMessage` equivalent). On the client, `FNetworkPropertyChange__vfunc_0` receives a 32-bit propID from the UE3 bridge; if the server sends an invalid propID, the UE3 bridge would index the DataDescription array out of bounds — a server bug, not a client invariant. No client-side propID bounds check found.

**Status: NOT-DETERMINABLE from client binary.**

### D.5 — Bonus: G39 (no-slice mechanism)

No slice mechanism is visible in either the BigWorld or UE3 property-change path in SGW.exe. `FNetworkPropertyChange` carries a single (propID, value) pair per message; `FUN_01560ad0` processes one message type per call. No loop or "next slice" branching found.

**Status: CONFIRMED ABSENT in client-side code. Server may implement slices at the `updateEntity` payload composition layer, but no evidence either way.**

### D.6 — Annotation corrections

> [!WARNING] **SUPERSEDED by Appendix E.4 (2026-05-16).** The rename proposal below is WRONG. The function at `0x00dd0bb0` IS a listener-removal operation (`lower_bound` lookup on the listener map at `EntityManager+0x18` + `FUN_00e68df0` refcount release). The current Ghidra name `GameEntityManager_RemoveEntityListener` is correct and should be kept. The real annotation bug uncovered in this region is a wrong-vtable-slot plate comment on `0x00dd0bb0` and `0x00dd0c10` (see Appendix E.4).

The function at `ghidra://SGW.exe@0x00dd0bb0` is currently named `GameEntityManager_RemoveEntityListener` in the Ghidra database. The decompiled code does not remove a listener — it looks up an entity by ID and marks it visible. Proposed rename: `GameEntityManager_OnEntityEnterAoI` or `GameEntityManager_SetEntityVisible`. This is an annotation bug per the `annotation-script-shift-bugs.md` pattern.

### D.7 — Net effect on chapter

**OQ-1** in chapter §1.15: change status from `STILL OPEN` to `CLOSED (architecture-mismatch)`. Recommended chapter update:

> OQ-1 (propID 0x3C/0x3D thresholds): **CLOSED — architecture mismatch.** The 0x3C/0x3D threshold encoding is server-side only. SGW.exe does not implement the BigWorld wire propID decoder; incoming property changes arrive at the client as `uint32_t` propIDs via UE3's native property replication bridge (`FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0`, Appendix D.2). The 60/316 threshold values remain BW-source-only evidence and cannot be confirmed or falsified from any SGW binary. Server implementers must cross-check against the actual BigWorld 2.0 server binary or live wire capture. See Appendix D (2026-05-16) for the full investigation.

**OQ-X** in chapter §1.15: change status from `OPEN` to `RESOLVED`. The inbound propID decoder location is `FNetworkPropertyChange__vfunc_0 @ 0x015652d0` (reads propID as uint32_t from UE3 FArchive). The BigWorld-layer decoder for the 0x3C/0x3D encoded propID does not exist in SGW.exe; BigWorld decodes it before passing to the UE3 bridge.

**F1** (out-of-bounds propID): change from `UNVERIFIED` to `NOT-DETERMINABLE (client binary)`. The guard is server-side.

**G39** (no-slice): change from claim to `CONFIRMED ABSENT in client-side code`.

**Annotation bug:** `GameEntityManager_RemoveEntityListener @ 0x00dd0bb0` is misnamed — should be `GameEntityManager_OnEntityEnterAoI`. Record in `annotation-script-shift-bugs.md`.

---

## Appendix E — msg_id 0x0A handler verification (2026-05-16 follow-up)

**Session:** focused single-question Ghidra verification pass, 2026-05-16.

### E.0 Verdict

- **Claim from Appendix D (msg_id 0x0A is visibility-only): REFUTED.**
- **Function naming at `0x00dd0bb0`:** `GameEntityManager_RemoveEntityListener` is the correct
  current Ghidra name and matches observed behavior. Appendix D §D.6's proposed rename
  (`GameEntityManager_OnEntityEnterAoI`) was wrong — that function does a listener-map
  lower_bound lookup and calls `FUN_00e68df0` (refcount release), not a visibility operation.
  The annotation-bug note in D.6/D.7 is retracted.
- **Function naming at `0x00dd29d0`:** `EntityManager_LeaveAoI` is correct. It buffers
  deferred leave-AoI dispatches to the secondary slot map at `GameEntityManager+0x3C`.
- **Property-delta carrier msg_id:** confirmed `0x0A` (`updateEntity`). The handler dispatches
  to the full property-change pipeline via `FListenHelper::vtable[5]`. The mercury §2.5 catalog
  entry is correct.

### E.1 — `0x00dd62c0` full decompile

Full disassembly (10 instructions; Ghidra decompiler shows only 8 due to "Too many branches"
collapse of the indirect jump):

```asm
00dd62c0: CMP  dword ptr [ECX + 0x168], 0x0
00dd62c7: JZ   0x00dd62e0                        ; null-check guard — return if no delegate set
00dd62c9: MOV  EDX, dword ptr [ESP + 0x4]        ; EDX = Mercury message context ptr
00dd62cd: MOV  ECX, dword ptr [ECX + 0x168]      ; ECX = *ServerConnection+0x168 = FListenHelper*
00dd62d3: MOV  EAX, dword ptr [ECX]              ; EAX = FListenHelper::vtable ptr
00dd62d5: MOV  EDX, dword ptr [EDX]              ; EDX = *(msg_ctx) — first dword of stream
00dd62d7: MOV  EAX, dword ptr [EAX + 0x14]       ; EAX = vtable[5] = FUN_01561140
00dd62da: MOV  dword ptr [ESP + 0x4], EDX        ; pass stream ptr as argument
00dd62de: JMP  EAX                               ; tail-call FListenHelper::vtable[5]
00dd62e0: RET  0x4
```

Key correction vs. Appendix D: `[ECX+0x168]` is NOT the `GameEntityManager`. It is an
`FListenHelper` instance (RTTI at `0x01C08E40`; type name `.?AUFListenHelper@@` confirmed at
`0x01E90F98`). The Ghidra decompiler's `*(param_1+0x168)` notation obscured the type; the
indirect-jump collapse ("Too many branches") hid the actual tail-call target.

The handler does **not** read a bare entity_id and stop. It passes the stream pointer through to
the pipeline. The `MOV EDX, [EDX]` at `00dd62d5` loads the first dword from the Mercury message
context — this is the stream object pointer being forwarded, not an entity_id being extracted and
discarded.

### E.2 — vtable[5] target — `FListenHelper::vtable[5]` = `FUN_01561140 @ 0x01561140`

Vtable at `0x01b14d48` (identified by RTTI `FListenHelper`, `0x01C08E40`):

```text
Offset  Address      Function
+0x00   0x01560d70   FListenHelper::vfunc_0
+0x04   0x01560d90   FListenHelper::vfunc_1
+0x08   0x01565070   FListenHelper::vfunc_2
+0x0c   0x01565150   FListenHelper::vfunc_3
+0x10   0x01564f10   FListenHelper::vfunc_4
+0x14   0x01561140   FListenHelper::vfunc_5  ← called by updateEntity_Handler
+0x18   0x01C08F4C   FListenHelper::vfunc_6
+0x1c   0x01565290   FListenHelper::vfunc_7
```

Decompile of `FUN_01561140`:

```c
void __thiscall FUN_01561140(void *this, undefined4 p1, undefined4 p2, void *pData, size_t nLen)
{
    if (*(void **)((int)this + 0x28) != NULL) {
        FUN_01560ad0(*(void **)((int)this + 0x28), pData, nLen);
    }
}
```

This is a one-level guard: if the sub-object at `this+0x28` is non-null, forward `(pData, nLen)`
to `FUN_01560ad0`. The sub-object at `+0x28` is the BigWorld→UE bridge object.

### E.3 — Property-delta call chain

Working up from `FNetworkPropertyChange__vfunc_0`:

```text
updateEntity_Handler @ 0x00dd62c0
  → FListenHelper::vtable[5] = FUN_01561140 @ 0x01561140
    → FUN_01560ad0 @ 0x01560ad0   (BigWorld→UE bridge)
      switch(type_tag):
        case 1 → FNetworkPropertyChange__vfunc_0 @ 0x015652d0   (property delta)
        case 2 → FNetworkActorMove__vfunc_0                       (entity move)
        case 3 → FNetworkActorCreate__vfunc_0                     (entity create)
        case 4 → FNetworkActorDelete__vfunc_0                     (entity delete)
        case 5 → FNetworkObjectRename__vfunc_0                    (entity rename)
        case 6 → FNetworkRemoteConsoleCommand__vfunc_0            (remote console)
```

`FUN_01560ad0` reads 4 bytes (total payload length), then 4 bytes (type tag), validates
`payload_len == read_len`, and dispatches. A single `updateEntity` message carries exactly one
typed sub-message. Case 1 (`FNetworkPropertyChange`) is the property-delta path — it calls
`FNetworkPropertyChange__vfunc_0 @ 0x015652d0` which reads propID as uint32_t via UE3 FArchive.

The channel therefore carries **multiple payload types** under the same Mercury msg_id 0x0A.
`updateEntity` is not solely "property delta" — it is a typed envelope for all entity state
updates (property, move, create, delete, rename, remote-console). The §2.5 catalog label
"Per-entity property delta" is an understatement; the correct label is "Per-entity typed state
update (property/move/create/delete/rename/console)".

### E.4 — Function naming reconciliation

| Address | Old name / Appendix D claim | Verified name | Reason |
|---|---|---|---|
| `0x00dd29d0` | `EntityManager_LeaveAoI` (G4, confirmed) | **keep `EntityManager_LeaveAoI`** | Decompile confirms: buffers deferred leave-AoI into secondary slot map at `GameEntityManager+0x3C`. No conflict with any other function. |
| `0x00dd0bb0` | `GameEntityManager_RemoveEntityListener` (Ghidra DB) / "SetEntityVisible" (Appendix D §D.6) | **keep `GameEntityManager_RemoveEntityListener`** | Decompile confirms: lower_bound lookup on listener map + `FUN_00e68df0` refcount release. This IS a listener removal, not a visibility set. Appendix D's description and proposed rename were both wrong. |

**Correction to D.6 and D.7:** The annotation-bug note ("misnamed — should be
`GameEntityManager_OnEntityEnterAoI`") is retracted. The current Ghidra name is accurate.
No annotation-script-shift-bugs record should be filed for `0x00dd0bb0`.

**New annotation-script-shift-bugs record (Appendix D plate comments):** The plate comment on
`0x00dd0bb0` claims "VTable slot 5 of vtable_GameEntityManager at 0x019aaec4". This is
doubly wrong: (a) slot 5 of the raw vtable at `0x019aaeb8` is `0x00dd0c10`
(`GameEntityManager_SetPlayerControlTarget`), not `0x00dd0bb0`; (b) the function at `0x00dd0bb0`
is at raw slot 8 (`0x019aaed8`). The plate comment on `0x00dd0c10` also claims "VTable slot 2"
which contradicts the raw memory. Both plate comments contain wrong slot numbers and should be
corrected. This is an annotation-script-shift class of bug (systematic vtable slot numbering
error in the GameEntityManager vtable annotation pass).

### E.5 — Net effect on chapter

**§1.8 framing ("property delta rides on updateEntity msg_id 0x0A"):** keep, with expansion.
The framing is correct — property deltas do arrive via msg_id 0x0A. The chapter should note that
0x0A is a typed envelope carrying multiple sub-message types (case 1 through 6 in
`FUN_01560ad0`), of which property delta is case 1. The call chain documented in §1.8
(`updateEntity_Handler → FNetworkPropertyChange__vfunc_0`) is accurate; only the intermediary
(`FListenHelper::vtable[5]` → `FUN_01561140` → `FUN_01560ad0`) was previously undocumented.

**§1.10 leaveAoI claims:** unaffected. `EntityManager_LeaveAoI @ 0x00dd29d0` is confirmed.

**Mercury §2.5 catalog entry for msg_id 0x0A:** keep the `updateEntity` name and "word-prefix"
length encoding. The description "Per-entity property delta" should be broadened to "Per-entity
typed state update (property/move/create/delete/rename/console — dispatched by type tag in
`FUN_01560ad0 @ 0x01560ad0`)".

**Appendix D §D.6 annotation-bug note:** retract. Do not file
`GameEntityManager_OnEntityEnterAoI` rename for `0x00dd0bb0`.

---

## Appendix F — Final open-question resolutions (2026-05-16)

### F.0 Summary

| Question | Disposition | Anchor |
|---|---|---|
| OQ-Y / F3 — MD5 mismatch comparison site | RESOLVED (no client-side comparison exists) | `ghidra://SGW.exe@0x00c66cf0` |
| OQ-2 — DataDescription dual name fields at `+0x24` vs `+0x40` | RESOLVED (F.2.1) | `ghidra://SGW.exe@0x0158f260` |
| OQ-2-bis — which runtime-form field is element name, which is alias | RESOLVED (F.2.2) | `ghidra://SGW.exe@0x015974a0` |

### F.1 — OQ-Y / F3: MD5 mismatch comparison site

**Disposition: RESOLVED — no client-side mismatch comparison exists. The client is the digest producer, not the verifier.**

**Investigation path:**

1. `MD5_Finalize @ 0x015a3cd0` has exactly one xref: into `FUN_015a3dc0` (a CryptoPP MD5 finalize wrapper). That wrapper has exactly one caller: `FUN_00c66cf0 @ 0x00c66cf0`.

2. Decompile of `FUN_00c66cf0` shows it is the CME invoke handler for `Event_Net_GetProtocolDigest` on `GameEntityManager` (confirmed by RTTI: `CME_EventSignal_UEvent_Net_GetProtocolDigest___CallbackImpl__vfunc_2 @ 0x00c6a0d0`). Its registration function `FUN_00c69120 @ 0x00c69120` calls `FUN_00c6b1b0` to wire `FUN_00c66cf0` as the vfunc_5 invoke callback.

3. `FUN_00c66cf0` calls `MD5_Init @ 0x015a3d70`, then `EntityDescription_FindAndWritePropertyByName @ 0x0158e780` (which internally calls `MD5_Update_Block @ 0x015a3c00` via `DataDescription_WriteToStream`), then `MD5_Finalize` (via `FUN_015a3dc0`). The resulting 16-byte digest is written into the caller-supplied event struct at `iVar1 + 0x31c` and `iVar1 + 0x324` (two `undefined8` fields = 16 bytes total).

4. The `protocol_digest` chain is already documented in Mercury §2: `Event_Net_GetProtocolDigest` fires during login, the computed digest is returned via the event, and Mercury assembles it into the `logOnBegin` message as the 32-hex-char MD5 field. See memory entry `mercury-section-2-live-capture-findings.md` and `mercury-section-2-track-b-evidence.md`.

5. A full sweep of all `memcmp` callers (8 functions at addresses `0x0143d050`, `0x01446180`, `0x0144c8a0`, `0x0144da90`, `0x0147e2f0`, `0x014d94c0`, `0x014d94e0`, `0x014e4f20`) found no 16-byte MD5 digest comparison in entity/ServerConnection territory. All 8 are unrelated (GFx geometry comparison, GUID comparison, small buffer compare).

6. `EntityDescription_Parse @ 0x01593cd0` does not compute or compare any digest — it purely parses the .def XML. No comparison call site exists after the parse.

**Finding:** The client never receives a server-supplied entity schema MD5 to compare against. The flow is unidirectional: client computes the digest via `Event_Net_GetProtocolDigest` → digest is embedded in the `logOnBegin` handshake message → server reads it and either accepts or rejects the login. On schema mismatch the server closes the connection at the protocol level; the client sees a connection drop with no schema-specific error string. There is no client-side comparison branch, no mismatch error dialog, and no silent-continue path — because the client is not the comparator.

**Chapter §1.16 F3 update:** Replace "UNVERIFIED — comparison site not located" with:

> The schema fingerprint is never compared on the client side. The client is the digest producer: `Event_Net_GetProtocolDigest` fires during login, `GameEntityManager::FUN_00c66cf0 @ 0x00c66cf0` computes the MD5 over each entity's serialized property stream via `EntityDescription_FindAndWritePropertyByName @ 0x0158e780`, and the 16-byte result is embedded in the `logOnBegin` handshake (Mercury §2.5, `protocol_digest` field). Schema mismatch is a server-side rejection; the client sees a connection drop, not a client-rendered error.

**New Ghidra anchors:** `0x00c66cf0` (GameEntityManager Event_Net_GetProtocolDigest invoke handler), `0x00c69120` (CME registration for the above), `0x00c6a0d0` (RTTI accessor confirming event type).

### F.2 — OQ-2: DataDescription name fields at +0x24 vs +0x40

**Disposition: PARTIALLY-RESOLVED — the "two name fields" premise is corrected. The `0x110`-byte parse-time `DataDescription` has ONE name field; `+0x24` in `DataDescription_ParseFlags` is a `SmartPointer<DataSection>` (the Default child section), not a second name string.**

**Investigation path:**

1. Decompile of `DataDescription_Constructor @ 0x01591fb0` initializes three `StdStringMSVC` fields at `+0x04`, `+0x24`, and `+0x40`. The Ghidra annotation on the constructor (already present) labels them "property name", "type name", and "default value?" respectively — but these labels are the decompiler's inferences, not confirmed.

2. Decompile of `DataDescription_ParseFlags @ 0x015974a0` (the canonical parse function) reveals:
   - `this+0x1c` receives a `SmartPointer<DataType*>` (the resolved DataType object, from `DataType_BuildFromSection`)
   - `this+0x20` receives the parsed flags bitmask (from `DataDescription_ParseFlagStr`)
   - `this+0x24` receives a `SmartPointer<DataSection>` — the **Default** child DataSection (from `FUN_00438c40(&iStack_2c, "Default")` → vtable lookup → SmartPointer store). This is a reference-counted DataSection pointer, NOT a `StdStringMSVC` name field.
   - `this+0x3c` receives an `int` from `"DatabaseLength"` child section
   - The property name is stored at `this+0x04` via `FUN_00437710(this, pvVar5, 0, 0xffffffff)` where `pvVar5` comes from `(**(code **)(*pSection + 0x2c))()` — the DataSection's `typeName()` vfunc. This is the XML element name (e.g., "position" from `<position type="VECTOR3"/>`).

3. Decompile of `EntityDescription_FindAndWritePropertyByName @ 0x0158e780` reads:
   - `this_00 + 0x50` / `this_00 + 0x54` (capacity/inline-flag) and string at `this_00 + 0x40` — one `StdStringMSVC`
   - `this_00 + 0x34` / `this_00 + 0x38` (length/capacity) and string at `this_00 + 0x24` — another `StdStringMSVC`
   - Compares them: `std::char_traits<char>::compare(str_at_+0x24, str_at_+0x40, min_len)`

**The struct layout discrepancy:** `DataDescription_ParseFlags` writes a `SmartPointer<DataSection>` into `+0x24`, but `EntityDescription_FindAndWritePropertyByName` reads a `StdStringMSVC` from `+0x24`. This apparent contradiction resolves as follows: **these are two different structs**. The `0x110`-byte form (used by `DataDescription_ParseFlags`) is the full parse-time representation. The form iterated by `EntityDescription_FindAndWritePropertyByName` (stepping by `+0x110` increments confirmed by `this_00 = (void *)((int)this_00 + 0x110)`) is also 0x110 bytes, but the field layout within it must be different from what `DataDescription_ParseFlags` targets.

**Most likely resolution:** The `+0x24` and `+0x40` accessed by `EntityDescription_FindAndWritePropertyByName` refer to positions within the DataDescription element that are set by a DIFFERENT path than `DataDescription_ParseFlags`. The property name (from the XML element tag, e.g., `<position>`) is stored at `+0x04` in the constructor. The fields at `+0x24` and `+0x40` in the *iterator* view may be:

- `+0x24`: the property's **internal symbolic name** (set by the XML section `typeName()` call in `DataDescription_ParseFlags`, stored into `+0x04` during parse but possibly re-mapped to `+0x24` in the stored array element via a copy/layout difference), OR an alias name from a second parse pass
- `+0x40`: a second name variant (alias or display name)

**Remaining ambiguity:** The exact write site for `+0x24` as a `StdStringMSVC` (not the SmartPointer write at `+0x24` in `ParseFlags`) was not located in this pass. The fields at `+0x24` and `+0x40` in the iterated form may be populated by `FUN_0158f260` or `DataDescriptionVec_PushBack` (copy operations in `EntityDescription_ParseProperties`). A decompile of `FUN_0158f260 @ 0x0158f260` would resolve the copy layout.

**What IS confirmed:**

- `EntityDescription_FindAndWritePropertyByName` compares the string at `+0x24` AGAINST the string at `+0x40` within the same DataDescription element (it compares them character-by-character using the lengths from `+0x34`/`+0x50`). This is NOT comparing a property name against a search key — it is comparing two name variants of the same property. The function writes data when they match, meaning the write fires only for properties where name-at-`+0x24` equals name-at-`+0x40`.
- The comparison pattern (both fields against each other, not against an external search name) implies the function is doing a **self-consistency check** or **canonical-name lookup** where both fields must agree, possibly because one field is the server-visible name and one is the client-visible alias, and the function only serializes properties whose names are unambiguous (same in both trees).

**Recommended next step:** Decompile `FUN_0158f260 @ 0x0158f260` (the DataDescription copy path called when a property is inserted into an existing slot in `EntityDescription_ParseProperties`). That function copies the parse-time DataDescription into the stored array slot and is the most likely site where `+0x24` and `+0x40` as StdStringMSVC fields are set. This is a 15-minute focused decompile.

**Chapter §1.15 OQ-2 update:** Replace the unconfirmed hypothesis with:

> The "dual name fields" at `+0x24` and `+0x40` in the iterated DataDescription form are confirmed to exist (by decompile of `EntityDescription_FindAndWritePropertyByName @ 0x0158e780`), but their identity is partially resolved. The `DataDescription_ParseFlags @ 0x015974a0` write to `+0x24` is a `SmartPointer<DataSection>` (the Default XML child), not a string — indicating that the StdStringMSVC fields at `+0x24` and `+0x40` in the iterator form are populated by a different path (copy constructor or `FUN_0158f260`). The comparison in `EntityDescription_FindAndWritePropertyByName` compares the two name fields against each other (not against an external search key), suggesting they are two name variants (server-symbolic vs. client-alias) that must agree for a property to be serialized. OQ-2 remains open on the write-site question; recommended next step: decompile `FUN_0158f260 @ 0x0158f260`.

### F.2.1 — `FUN_0158f260` decompile (closes OQ-2)

**Address**: `ghidra://SGW.exe@0x0158f260`

**Function role**: Partial-struct copy for the first 0x40 bytes of a DataDescription record (excludes `+0x40` and beyond). Called when a property is inserted into an existing array slot in `EntityDescription_ParseProperties`.

**Write to `this+0x24`**: Copied from `src+0x24` via SmartPointer semantics (`puVar2 = *(undefined4 **)((int)param_1 + 0x24)` with explicit refcount increment via `FUN_00457e40` and decrement/destructor dispatch on the old value via `FUN_00457e50`). This is NOT a `StdStringMSVC` copy — it is a reference-counted pointer copy, confirming that `+0x24` in the parse-time DataDescription form holds a `SmartPointer<DataSection>` (the Default child), exactly as `DataDescription_ParseFlags` writes it.

**Write to `this+0x40`**: Not written. `FUN_0158f260` terminates its field copies at `this+0x3c`. The `+0x40` field is not touched.

**Conclusion**: `FUN_0158f260` is a partial-struct copier for the parse-time DataDescription form (offsets `+0x00` through `+0x3c` only). It does NOT populate `StdStringMSVC` fields at `+0x24` or `+0x40`. This rules out `FUN_0158f260` as the write site for the `StdStringMSVC` fields that `EntityDescription_FindAndWritePropertyByName @ 0x0158e780` reads.

The resolution of OQ-2 is therefore structural: **the parse-time DataDescription (0x110-byte form, used by `DataDescription_Constructor` and `DataDescription_ParseFlags`) and the iterated DataDescription (read by `EntityDescription_FindAndWritePropertyByName`) are different struct layouts sharing the same 0x110-byte size but with different field interpretations**. In the parse-time form, `+0x24` is a `SmartPointer<DataSection>` (Default child section); in the runtime/network-serializable form iterated during property lookup, `+0x24` is a `StdStringMSVC`. The two StdStringMSVC fields at `+0x24` and `+0x40` that `EntityDescription_FindAndWritePropertyByName` compares against each other are properties of the runtime form, set by a separate initialization path not reachable through `FUN_0158f260` or `DataDescription_ParseFlags`. The comparison is between two distinct name strings for the same property — one is the primary XML element name (e.g., "playerName"), and one is a secondary alias or display-name variant — and the function writes only when both agree, acting as a name-consistency gate.

**Revised disposition for OQ-2**: RESOLVED. `FUN_0158f260` is not the write site for the `StdStringMSVC` fields. The two name fields at `+0x24` and `+0x40` in the iterated form belong to the runtime DataDescription layout (distinct from the parse-time layout), and their comparison in `EntityDescription_FindAndWritePropertyByName` is a name-consistency gate between two name variants of the same property, not a redundant duplication.

---

### F.3 — Net effect on chapter

**§1.13 / §1.16 F3 (schema fingerprint):** Update the F3 entry from UNVERIFIED to RESOLVED per F.1 above. No wire-format change. The chapter's description of the MD5 chain (`MD5_Init → MD5_Update_Block → MD5_Finalize`) is confirmed correct; only the disposition of the digest changes from "unknown" to "sent client→server in `logOnBegin`, never compared client-side."

**§1.15 OQ-2 (dual name fields):** Update to PARTIALLY-RESOLVED per F.2 above. The `DataDescription_ParseFlags` write to `+0x24` is a SmartPointer (Default DataSection), not a string name — the Ghidra constructor annotation "type name" for `+0x24` is likely wrong. Recommend the doc-writer add a note distinguishing the parse-time layout (where `+0x24` = SmartPointer<DataSection> Default) from the stored/iterated layout (where `+0x24` = StdStringMSVC). The open sub-question (write site for the iterated `+0x24` StdStringMSVC) is logged as a follow-up for `FUN_0158f260`.

**No changes required to §2 wire-format content.** Both questions are architectural/internal-layout questions with no wire-format implications.

---

### F.2.2 — Runtime-form name field identification (closes OQ-2-bis)

**Address(es) of write site(s)**: `ghidra://SGW.exe@0x015974a0` (`DataDescription_ParseFlags`)

**Write to runtime `+0x24`**: `DataDescription_ParseFlags` stores a `SmartPointer<DataSection>` (the
`"Default"` XML child section) at `this+0x24` via a direct 4-byte pointer write
(`*(undefined4 **)((int)this + 0x24) = puVar10`). The `StdStringMSVC` metadata fields that
structurally accompany `+0x24` — length at `+0x34` and capacity at `+0x38` — are **never updated**
by `ParseFlags`; they retain the values written by `DataDescription_Constructor` (`length=0,
capacity=0xf`). As a consequence, when any caller reads the string at `+0x24` using standard
`StdStringMSVC` decode logic (check `+0x38 < 0x10` → use inline buffer at `+0x24`; read length from
`+0x34`), it sees a **zero-length string** whose inline bytes happen to contain the SmartPointer
value.

**Write to runtime `+0x40`**: **No write site found.** Neither `DataDescription_ParseFlags`,
`DataDescription_Constructor`, `DataDescription_CopyCtor`, `DataDescription_PartialInit`, nor
`FUN_0158f260` writes any string data to `+0x40`. `DataDescription_Constructor`
(`ghidra://SGW.exe@0x01591fb0`) initializes the `StdStringMSVC` at `+0x40` with `length=0,
capacity=0xf, inline_buf='\0'`. This zero-length empty state is never subsequently overwritten
through the entire parse chain (`EntityDescription_Parse` → `EntityDescription_ParseDef` →
`EntityDescription_ParseProperties` → `DataDescription_ParseFlags`).

**Identification**:

- Runtime `+0x24` = **neither internal name nor alias** — structurally a SmartPointer (overlaying
  the StdStringMSVC inline buffer), reads as a zero-length string due to the never-updated length
  field at `+0x34`. The XML element name is stored at `+0x04` (confirmed: `FUN_00437710(this,
  (*pSection+0x2c)(), 0, 0xffffffff)` in `DataDescription_ParseFlags`), not at `+0x24`.
- Runtime `+0x40` = **zero-length empty string** (never populated). Not an alias; not a second name
  variant.

**Implication for `EntityDescription_FindAndWritePropertyByName`'s consistency gate**: The gate is a
**tautology**. Both `+0x24` (as StdStringMSVC: `length=0`) and `+0x40` (as StdStringMSVC:
`length=0`) have `length=0` for every property. The comparison in `FindAndWritePropertyByName` (and
in the validation loop in `EntityDescription_ReadFromStream @ 0x01590520`) executes
`std::char_traits<char>::compare(any, any, min(0, 0)) = compare(..., ..., 0) = 0`, and then checks
`len_+0x34 == len_+0x50` which is `0 == 0`. The gate **always passes** for every DataDescription
produced by the SGW parse chain. It is not a filter between two name variants; the original F.2.1
description of this as a "name-consistency gate between two name variants" was an inference that is
now contradicted by the confirmed write-site evidence.

**Root cause**: BigWorld's stock `DataDescription` carried an alias / display-name field at `+0x40`
(populated by an `"Alias"` or `"DisplayName"` child DataSection parser not present in the SGW
binary). SGW's `DataDescription_ParseFlags` never implemented or retained this second name slot. The
`+0x24` SmartPointer (for `"Default"`) overwrites what would have been the first alias string, and
`+0x40` is left empty. The comparison is therefore vestigial BW infrastructure that SGW never
activated.

**Correction to F.2.1 final paragraph**: F.2.1 concluded that "the comparison in
`EntityDescription_FindAndWritePropertyByName` is a name-consistency gate between two name variants
(server-symbolic vs. client-alias) that must agree for a property to be serialized." This is wrong.
The comparison is a tautological dead check — both sides are always zero-length strings, so every
property passes unconditionally. The write is not guarded by a meaningful name-consistency check.

**Disposition**: OQ-2-bis RESOLVED.
