---
name: entity-property-sync-oq1
description: OQ-1 and OQ-2 dispositions for entity-property-sync chapter — OQ-1 closed (architecture mismatch), OQ-2 closed (FUN_0158f260 decompile confirms two distinct DataDescription layout forms)
metadata:
  type: project
---

## OQ-1 CLOSED — architecture mismatch (2026-05-16)

**Verdict:** The BigWorld propID wire encoding (0x3C/0x3D thresholds, §1.8 of entity-property-sync.md) is server-side only. SGW.exe does not contain the decoder.

**Why:** `updateEntity` (msg_id 0x0A) is the entity-enters-AoI visibility signal, not a property delta stream. Property changes flow through UE3's native property replication bridge instead of the BigWorld wire propID path.

**Property change path (all Ghidra-confirmed):**
- `FRemotePropagator` (`FUN_015605b0 @ 0x015605b0`) → `FUN_01565390` → `FListenHelper::vfunc_5 @ 0x01561140` → `FUN_01560ad0 @ 0x01560ad0` (type-tag switch, case 1) → `FNetworkPropertyChange__vfunc_0 @ 0x015652d0`
- `FNetworkPropertyChange__vfunc_0` reads propID as **uint32_t** (4 bytes) from UE3 FArchive via `FUN_0047f0e0(stream, this+0x2c, 4)` — NOT as the 1-2 byte BigWorld wire field.

**`updateEntity` handler chain:**
- Thunk at `0x017bb570` registers handler `0x00dd62c0`, msg_id `0x0A`, name `"updateEntity"`.
- Handler `0x00dd62c0`: null-checks `[ECX+0x168]` (EntityManager ptr), reads entity_id from first 4 payload bytes, dispatches to vtable slot 5 (`+0x14`).
- Vtable slot 5 = `0x00dd0bb0` = entity-visibility toggle (NOT listener removal — see annotation bug below).

**Byte-pattern search:** Zero hits for 0x3C/0x3D threshold comparisons (`CMP EAX,0x3C`, `MOVZX+CMP 0x3C`, `SUB EAX,60`, `CMP AL,0x3D`, etc.) in executable code. All hits were XML/CSS/string parsers.

**Chapter updates required (for Documentation Writer):**
- OQ-1 § 1.15: STILL OPEN → CLOSED (architecture-mismatch). 60/316 thresholds are BW-source-only; cannot be confirmed or falsified from SGW.exe.
- OQ-X §1.15: OPEN → RESOLVED. Inbound propID decoder = `FNetworkPropertyChange__vfunc_0 @ 0x015652d0` (uint32_t via UE3 FArchive).
- F1 (out-of-bounds propID): → NOT-DETERMINABLE from client binary (server-side guard).
- G39 (no-slice): → CONFIRMED ABSENT in client-side code.

**Annotation bug:** `GameEntityManager_RemoveEntityListener @ 0x00dd0bb0` is misnamed. Correct name: `GameEntityManager_OnEntityEnterAoI`. Recorded in `annotation-script-shift-bugs.md` session-5 section (2026-05-16).

**Key addresses:**
- `FNetworkPropertyChange__vfunc_0`: `0x015652d0`
- `FListenHelper` vtable: `0x01b14d4c`
- `FListenHelper` singleton: `DAT_01ef11fd8`
- `FUN_01560ad0` (type-tag switch): `0x01560ad0`
- `FRemotePropagator` bridge: `0x015605b0`
- `updateEntity` handler: `0x00dd62c0`
- `GameEntityManager_OnEntityEnterAoI` (misnamed): `0x00dd0bb0`
- `updateEntity` thunk: `0x017bb570`

**Full findings:** `docs/audits/entity-property-sync-section2-audit-2026-05-16.md` Appendix D (lines 1088–1192).

**How to apply:** When anyone asks about entity property sync, propID encoding, or `updateEntity` semantics, use this to correct the assumption that the client decodes the BigWorld wire propID format. The client's incoming path is UE3-native; only the server-side BigWorld layer uses the 0x3C/0x3D encoding.

---

## OQ-2 CLOSED — two distinct DataDescription layout forms (2026-05-16)

**Verdict:** `FUN_0158f260 @ 0x0158f260` is a partial-struct copier (offsets `+0x00` through `+0x3c` only). It is NOT the write site for `StdStringMSVC` fields at `+0x24` or `+0x40`.

**The write to `this+0x24` in `FUN_0158f260`:** Copies `src+0x24` via SmartPointer semantics (refcount increment/decrement, vtable dispatch). This is a `SmartPointer<DataSection>` copy (the Default child section), not a string copy.

**`this+0x40` is not touched** by `FUN_0158f260` — the function terminates at `+0x3c`.

**Resolution:** The parse-time DataDescription form (0x110-byte, initialized by `DataDescription_Constructor @ 0x01591fb0` and written by `DataDescription_ParseFlags @ 0x015974a0`) and the runtime/iterated form (read by `EntityDescription_FindAndWritePropertyByName @ 0x0158e780`) share the same 0x110-byte size but have different field interpretations at `+0x24`:
- Parse-time: `+0x24` = `SmartPointer<DataSection>` (Default child XML section)
- Runtime/iterated: `+0x24` = `StdStringMSVC` (one of two name variants compared by `FindAndWritePropertyByName`)

The comparison in `EntityDescription_FindAndWritePropertyByName` between `+0x24` and `+0x40` is a name-consistency gate between two name variants (XML element name vs. alias), not a redundant duplication. The write site for the runtime `StdStringMSVC` fields is a separate initialization path not involving `FUN_0158f260`.

**Full findings:** `docs/audits/entity-property-sync-section2-audit-2026-05-16.md` Appendix F.2 and F.2.1 (lines 1385–1447).
