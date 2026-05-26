---
name: entity-property-sync-oq1
description: OQ-1 / OQ-2 dispositions for entity-property-sync chapter — OQ-1 closed (architecture mismatch; updateEntity is a typed envelope, not visibility-only; client has no 0x3C/0x3D decoder), OQ-2 closed (FindAndWritePropertyByName's +0x24/+0x40 gate is tautological per F.2.2 — both fields are always zero-length strings)
metadata:
  type: project
---

## OQ-1 CLOSED — architecture mismatch (2026-05-16)

**Verdict:** The BigWorld propID wire encoding (0x3C/0x3D thresholds, §1.8 of entity-property-sync.md) is server-side only. SGW.exe does not contain the decoder.

**Why:** `updateEntity` (msg_id 0x0A) is a typed UE FArchive envelope carrying multiple sub-message types (case 1 = property delta, case 2 = move, case 3 = create, case 4 = delete, case 5 = rename, case 6 = remote console), not a BigWorld propID-prefixed wire stream. By the time bytes reach the client, the BigWorld network layer **upstream** of the UE bridge has already consumed the wire-propID prefix; the propID arrives at `FNetworkPropertyChange__vfunc_0` as a fully-decoded `uint32_t` read from the FArchive at `this+0x2c`. The 0x3C/0x3D thresholds exist only on the server side of that bridge — the SGW client never sees the encoded form, and a byte-pattern sweep of SGW.exe code confirms zero decoder presence (Audit Appendix D.3).

**Inbound property-change chain (corrected per Audit Appendix E.1–E.3):**

```text
updateEntity_Handler @ 0x00dd62c0          (Mercury msg_id 0x0A handler)
  → FListenHelper::vtable[5] = FUN_01561140 @ 0x01561140   (one-level null-check on the bridge sub-object)
    → FUN_01560ad0 @ 0x01560ad0            (BigWorld→UE bridge: [u32 length][u32 type tag] switch)
      case 1 → FNetworkPropertyChange__vfunc_0 @ 0x015652d0
        → FUN_0047f0e0(stream, this+0x2c, 4)   (reads propID as uint32_t from UE FArchive)
```

- `updateEntity_Handler @ 0x00dd62c0` forwards the Mercury stream pointer to `FListenHelper::vtable[5]`. It does **not** read entity_id from the payload, and it does **not** dispatch on `GameEntityManager`'s vtable. Appendix E.1 corrects Appendix D on this point: `[ECX+0x168]` is an `FListenHelper*` (RTTI `.?AUFListenHelper@@` at `0x01E90F98`, class info at `0x01C08E40`), not a `GameEntityManager*` — the Ghidra decompiler's `*(param_1+0x168)` notation obscured the type, and the "Too many branches" collapse on the indirect jump hid the tail-call target.
- Server-emitted property changes that bypass Mercury (the `FRemotePropagator @ 0x015605b0` → `FUN_01565390` → `FListenHelper::vfunc_5` path) converge at the same bridge `FUN_01560ad0`, so the case-1 receiver `FNetworkPropertyChange__vfunc_0` is shared between Mercury inbound and the direct-propagator path.

**Byte-pattern search (reproducibility):** Zero hits in executable code for the 0x3C/0x3D threshold-comparison instructions characteristic of a BigWorld wire-propID decoder. Search method:

- **Tool:** `mcp__ghidra__search_byte_patterns` against `SGW.exe` loaded at image base `0x00400000`.
- **Scope:** all executable sections (`.text`); little-endian (x86), case-sensitive (binary).
- **Patterns probed:** `83 F8 3C` (`CMP EAX,0x3C`), `83 F8 3D` (`CMP EAX,0x3D`), `0F B6 .. 3C` (`MOVZX r32, byte [..]` + `CMP r32, 0x3C`), `83 E8 3C` (`SUB EAX, 60`), `3C 3D` (`CMP AL, 0x3D`), plus the matching `0x3D` variants of each pattern.
- **Result:** all matches were in XML/CSS string-table data or HTML parser tables, none in code that consumes a Mercury body or an entity stream.
- **Full sweep:** documented in Audit Appendix D.3 of `docs/audits/entity-property-sync-section2-audit-2026-05-16.md`.

**Chapter updates required (for Documentation Writer) — all APPLIED:**

- ✓ APPLIED (`a20300d`) — § 1.15 OQ-1: STILL OPEN → CLOSED (architecture-mismatch). 60/316 thresholds are BW-source-only; cannot be confirmed or falsified from SGW.exe. (See chapter line containing "OQ-1: CLOSED — thresholds are server-side only.")
- ✓ APPLIED (`a20300d`) — § 1.15 OQ-X: OPEN → RESOLVED. Inbound propID decoder = `FNetworkPropertyChange__vfunc_0 @ 0x015652d0` (uint32_t via UE3 FArchive). (See "OQ-X: CLOSED — inbound dispatch chain located.")
- ✓ APPLIED (`a20300d`) — § 1.16 F1 (out-of-bounds propID): UNVERIFIED → NOT-DETERMINABLE from client binary (server-side guard, reframed as server-encoder concern).
- ✓ APPLIED (`a04f12d`) — § B/G39 (no-slice): CONFIRMED ABSENT in client-side code (`FNetworkPropertyChange__vfunc_0` writes one complete property per call; `FixedDictDataType_ToXml @ 0x01598b80` iterates fields in a flat loop with no slice-index field).

**Retracted Appendix D claim:** Appendix D's draft conclusion that msg_id 0x0A is a visibility-only signal — and that the function at `0x00dd0bb0` was a misnamed `GameEntityManager_OnEntityEnterAoI` entry point of that chain — is REFUTED by Appendix E. The Ghidra name `GameEntityManager_RemoveEntityListener` is correct; that function is a real listener-removal routine (lower_bound lookup on the listener map + `FUN_00e68df0` refcount release) but is **not** in the `updateEntity` dispatch chain at all. The proposed `GameEntityManager_OnEntityEnterAoI` rename is retracted. The plate-comment slot-number errors on `0x00dd0bb0` and `0x00dd0c10` (separate annotation-script-shift bug surfaced by Appendix E.4) are recorded in `annotation-script-shift-bugs.md` session-5 (2026-05-16).

**Key addresses (inbound chain — Audit Appendix E):**

- `updateEntity_Handler`: `0x00dd62c0`
- `updateEntity` thunk (registers handler + msg_id + name): `0x017bb570`
- `FListenHelper` vtable: `0x01b14d48`
- `FListenHelper` singleton allocator (`DAT_01ef11fd8`): `FUN_0155f9b0 @ 0x0155f9b0`
- `FListenHelper::vtable[5]`: `FUN_01561140 @ 0x01561140`
- `FUN_01560ad0` (BigWorld→UE bridge, type-tag switch): `0x01560ad0`
- `FNetworkPropertyChange__vfunc_0` (case-1 sink, FArchive uint32_t propID): `0x015652d0`
- `FRemotePropagator` (alternate entry; converges at bridge): `0x015605b0`

**Full findings:** `docs/audits/entity-property-sync-section2-audit-2026-05-16.md` Appendix D (lines 1106–1220, original visibility-only hypothesis) + Appendix E (lines 1222–1364, supersedes D's framing with the typed-envelope chain).

**How to apply:** When anyone asks about entity property sync, propID encoding, or `updateEntity` semantics, use this to correct two specific misconceptions: (a) `updateEntity` is *not* visibility-only — it is a typed UE FArchive envelope where case 1 carries the property delta and cases 2–6 carry move/create/delete/rename/remote-console; (b) the client never decodes the BigWorld wire-propID 0x3C/0x3D scheme — those bytes are consumed upstream by the BigWorld network layer, and the client receives an already-decoded `uint32_t` at the UE3 FArchive layer.

---

## OQ-2 CLOSED — name-consistency gate is tautological (2026-05-16)

**Verdict:** The `+0x24` vs `+0x40` comparison in `EntityDescription_FindAndWritePropertyByName @ 0x0158e780` is a **vestigial BigWorld dead check** that always passes for SGW-parsed `DataDescription` records. It is not a name-variant consistency gate; the original "two distinct DataDescription layout forms" hypothesis from F.2.1 is superseded by F.2.2.

**Why the gate is tautological (F.2.2 evidence):**

- `DataDescription_Constructor @ 0x01591fb0` initializes the `StdStringMSVC` length/capacity fields at `+0x34`/`+0x38` and `+0x50`/`+0x54` with `length=0, capacity=0xf` (inline SSO state, empty string).
- `DataDescription_ParseFlags @ 0x015974a0` writes a `SmartPointer<DataSection>` (the `"Default"` child section) into the inline buffer at `this+0x24` via a direct 4-byte pointer write — it does **not** update the length field at `+0x34`. The string at `+0x24` therefore reads as zero-length when decoded as `StdStringMSVC`.
- No write site in the SGW parse chain (`EntityDescription_Parse` → `EntityDescription_ParseDef` → `EntityDescription_ParseProperties` → `DataDescription_ParseFlags`, plus `DataDescription_CopyCtor`, `DataDescription_PartialInit`, `FUN_0158f260`) populates `+0x40` with any string data. It retains the constructor's zero-length state.
- Both sides of the comparison therefore have `length=0` for every property; `std::char_traits<char>::compare(any, any, 0) = 0` and `0 == 0` is the length check, so the gate always passes.

**Root cause:** BigWorld's stock `DataDescription` carried an alias / display-name field at `+0x40` (populated by an `"Alias"` or `"DisplayName"` child DataSection parser). SGW's `DataDescription_ParseFlags` never implemented or retained that second name slot; the `+0x24` SmartPointer (for `"Default"`) overwrites what would have been the first alias inline buffer, and `+0x40` is left empty. The comparison is vestigial BW infrastructure that SGW never activated.

**Supersedes F.2.1:** F.2.1 inferred the gate was a "name-consistency check between two name variants (server-symbolic vs. client-alias) that must agree for a property to be serialized." F.2.2's write-site evidence contradicts that inference — both fields are always empty, so the gate is dead.

**`FUN_0158f260` (the partial-copy function originally hypothesized as the runtime `+0x24`/`+0x40` write site):** is a partial-struct copier for the first 0x40 bytes of a `DataDescription` record (writes through `+0x3c` only). The copy at `+0x24` is SmartPointer semantics (refcount increment/decrement, vtable dispatch), not a string copy. `+0x40` is not touched. This rules `FUN_0158f260` out as the source of any non-empty string at `+0x24` or `+0x40`.

**Full findings:** `docs/audits/entity-property-sync-section2-audit-2026-05-16.md` Appendix F.2 (initial PARTIALLY-RESOLVED state — superseded), F.2.1 (closes "is FUN_0158f260 the write site?" — no), and F.2.2 (closes OQ-2 + OQ-2-bis: the gate is tautological).
