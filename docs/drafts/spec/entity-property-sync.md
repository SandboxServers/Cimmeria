---
title: Entity Property Sync
chapter_id: spec.protocol.entity-property-sync
status: draft
last_verified: 2026-05-16
verified_by: automated-agent
confidence:
  re: medium
  client: high
  deprecated: n/a
  rust_expected: n/a
  rust_actual: n/a
evidence_refs:
  re:
    - docs/reverse-engineering/findings/entity-property-sync.md
    - docs/reverse-engineering/findings/entity-creation-wire-formats.md
    - docs/reverse-engineering/findings/entity-types-wire-formats.md
    - docs/reverse-engineering/findings/world-entry-pipeline.md
    - ghidra://SGW.exe@0x01593600
    - ghidra://SGW.exe@0x01593cd0
    - ghidra://SGW.exe@0x015924a0
    - ghidra://SGW.exe@0x015974a0
    - ghidra://SGW.exe@0x01594f60
    - ghidra://SGW.exe@0x01590df0
    - ghidra://SGW.exe@0x01590ee0
    - ghidra://SGW.exe@0x00c6fc40
    - ghidra://SGW.exe@0x00c6f8f0
    - ghidra://SGW.exe@0x00dddca0
    - ghidra://SGW.exe@0x00dda2e0
    - ghidra://SGW.exe@0x015652d0
    - ghidra://SGW.exe@0x01596c40
    - ghidra://SGW.exe@0x01597150
    - ghidra://SGW.exe@0x01597ce0
    - ghidra://SGW.exe@0x01595f00
    - ghidra://SGW.exe@0x015a3d70
    - ghidra://SGW.exe@0x015a3c00
    - ghidra://SGW.exe@0x015a3cd0
    - ghidra://SGW.exe@0x0158e710
    - ghidra://SGW.exe@0x0158e780
    - ghidra://SGW.exe@0x00dd27f0
    - ghidra://SGW.exe@0x00dd2800
    - ghidra://SGW.exe@0x00dd24f0
    - ghidra://SGW.exe@0x00dd2270
    - ghidra://SGW.exe@0x00dd20b0
    - ghidra://SGW.exe@0x00dd09e0
    - ghidra://SGW.exe@0x00dd1d00
    - ghidra://SGW.exe@0x00dd2b80
    - ghidra://SGW.exe@0x00dd1e40
    - ghidra://SGW.exe@0x01590fc0
    - ghidra://SGW.exe@0x01590d80
    - ghidra://SGW.exe@0x01591fb0
    - ghidra://SGW.exe@0x01560ad0
    - ghidra://SGW.exe@0x015652d0
    - ghidra://SGW.exe@0x015942f0
    - ghidra://SGW.exe@0x01593930
    - ghidra://SGW.exe@0x015959c0
    - ghidra://SGW.exe@0x01e920e0
    - ghidra://SGW.exe@0x01b1ae38
    - ghidra://SGW.exe@0x01b1aeb4
    - ghidra://SGW.exe@0x01b1af14
    - ghidra://SGW.exe@0x00dd29d0
    - ghidra://SGW.exe@0x01590bb0
    - ghidra://SGW.exe@0x01590f30
    - ghidra://SGW.exe@0x015958b0
    - ghidra://SGW.exe@0x01598b80
    - ghidra://SGW.exe@0x0159b480
    - ghidra://SGW.exe@0x0159b850
    - ghidra://SGW.exe@0x00dd66e0
    - ghidra://SGW.exe@0x00dd6a60
    - ghidra://SGW.exe@0x00dd6980
    - ghidra://SGW.exe@0x00dd6690
    - ghidra://SGW.exe@0x01604e80
  client:
    - game/sgw/Common/res/entities/entities.xml:1-32
    - game/sgw/Common/res/entities/defs/alias.xml
    - game/sgw/Common/res/entities/defs/enumerations.xml
    - game/sgw/Common/res/entities/defs/SGWEntity.def:1-146
    - game/sgw/Common/res/entities/defs/SGWSpawnableEntity.def:1-226
    - game/sgw/Common/res/entities/defs/SGWBeing.def:1-33
    - game/sgw/Common/res/entities/defs/SGWPlayer.def:1-1448
    - game/sgw/Common/res/entities/defs/Account.def:1-107
    - game/sgw/Common/res/entities/defs/interfaces/SGWBeing.def:1-303
    - game/sgw/Common/res/entities/defs/interfaces/Communicator.def:1-247
    - game/sgw/Common/res/entities/defs/interfaces/OrganizationMember.def:1-454
    - game/sgw/Common/res/entities/defs/interfaces/MinigamePlayer.def:1-542
    - game/sgw/Common/res/entities/defs/interfaces/GateTravel.def:1-95
    - game/sgw/Common/res/entities/defs/interfaces/SGWInventoryManager.def:1-222
    - game/sgw/Common/res/entities/defs/interfaces/SGWMailManager.def:1-107
    - game/sgw/Common/res/entities/defs/interfaces/Missionary.def:1-191
    - game/sgw/Common/res/entities/defs/interfaces/ContactListManager.def:1-106
    - game/sgw/Common/res/entities/defs/interfaces/SGWBlackMarketManager.def:1-114
    - game/sgw/Common/res/entities/defs/interfaces/ClientCache.def:1-43
    - game/sgw/Common/res/entities/defs/interfaces/SGWPoller.def:1-28
    - game/sgw/Common/res/entities/defs/interfaces/SGWCombatant.def:1-286
    - game/sgw/Common/res/entities/defs/interfaces/SGWAbilityManager.def:1-310
    - game/sgw/Common/res/entities/defs/interfaces/DistributionGroupMember.def:1-93
    - game/sgw/Common/res/entities/defs/interfaces/EventParticipant.def:1-35
    - game/sgw/Common/res/entities/defs/interfaces/GroupAuthority.def:1-68
    - game/sgw/Common/res/entities/defs/interfaces/Lootable.def:1-88
    - game/sgw/Working/SGWGame/Config/DefaultEngine.ini
    - game/sgw/Working/SGWGame/Config/DefaultGame.ini
  deprecated: []
  rust: []
related_chapters:
  - spec.protocol.mercury-wire-format
  - spec.protocol.position-updates
  - spec.protocol.message-catalog
  - spec.engine.entity-description-parse-chain
  - spec.engine.universal-rpc-dispatcher
  - spec.world.world-entry
binary_scope:
  file: SGW.exe
  sha256: 109F307763A5C6C59FF484840739860BDC7163092F0644343D0B2C03E4925783
  image_base: 0x00400000
disputed_by: []
supersedes: []
---

# Entity Property Sync

An entity in BigWorld is a distributed object: every server-authoritative property and method has a numeric identifier the wire uses instead of a string name, and every property mutation rides one of three serialization pipes — entity creation, AoI introduction, or runtime property-change — each with its own byte layout. This chapter pins each of those pipes to the binary that emits them, plus the bit-by-bit rules the client uses to decode them.

The property/method ID space is constructed at parse time by walking each entity's `.def` file plus its parent and implements chain; the resulting `(entity_type, property_id)` and `(entity_type, method_category, method_id)` tables are the contract between server and client. Get the table wrong by one slot and property updates write the wrong fields on the client — silently, with no error. The mercury chapter (`spec.protocol.mercury-wire-format`) carries the envelope; this chapter carries the payload.

Schema construction itself — parse order, ID assignment mechanics, DataType class hierarchy, MD5 signature digest — is owned by the future `spec.engine.entity-description-parse-chain` chapter (planned but not yet written; the chapter ID is reserved in [`docs/spec/README.md`](../../spec/README.md) under `spec.engine`). Section 1 summarizes those mechanics where the wire format depends on them and cross-references forward for full detail. Later cross-references to `spec.engine.entity-description-parse-chain` in this chapter implicitly inherit the same "planned but not yet written" caveat.

---

## Section 1 — RE findings

This section distills the Ghidra decompilation evidence for every layer of the entity property/method sync system: schema construction (§1.1–§1.4), wire dispatch (§1.5), creation-time wire formats (§1.6–§1.7), runtime property-change wire format (§1.8), AoI introduction cascade (§1.9–§1.10), data-domain filters (§1.11), DataType registries and MD5 schema fingerprint (§1.12–§1.13). §1.14 is the source-of-truth crosswalk; §1.15 records open questions. Every factual claim resolves to a `ghidra://SGW.exe@0x<addr>` anchor (image base `0x00400000`).

The client binary (`SGW.exe`) is the reference for all ID-table construction and wire-decode logic. The server-side encoders (BaseApp/CellApp) are not in the SGW binary; where a claim about the *server's* emit logic cannot be confirmed in SGW.exe, this is stated explicitly and sourced either to BigWorld 2.0.1 source or flagged as hypothesis.

---

### 1.1 propID / methodID wire contract — summary

Entities in BigWorld carry three disjoint ID spaces, all assigned sequentially at parse time:

- **propID** — a client-property ordinal, zero-based. In stock BigWorld it indexes the filtered subset of properties that have `DATA_OWN_CLIENT (0x04)` or `DATA_OTHER_CLIENT (0x02)` flags (the `+0x70/+0x74` client-property pointer array). **In SGW the routing is different — lead with the SGW behavior**: the wire propID indexes the main DataDescription array at `EntityDescription+0x5c/+0x60` directly. See §1.2 for the source-doc-override callout (and §2.3 for the keyword-surface audit that confirms it). The stock-BigWorld description is preserved here for readers tracing the divergence; the SGW routing is what server implementers must encode against. Either way, this ordinal is what rides the wire in property-change messages and in the creation-time property streams.
- **cellMethodID** — a zero-based ordinal within the entity's CellMethods list, restricted to methods marked `<Exposed/>`.
- **baseMethodID** — a zero-based ordinal within the entity's BaseMethods list, restricted to methods marked `<Exposed/>`.

These three tables are constructed by the entity-description parse chain, which walks `<Parent>` → `<Implements>` → own sections in that order. The full parse-chain mechanics — parse order, ID assignment algorithm, DataType class hierarchy, schema MD5 — are owned by the future `spec.engine.entity-description-parse-chain` chapter (planned but not yet written). §1.1 here is a working summary sufficient for the wire format; forward-reference that chapter for the complete picture.

> [!NOTE] **Version note.** All Ghidra addresses in this chapter are scoped to the SGW.exe build identified in the frontmatter (`binary_scope.sha256: 109F307763A5C6C59FF484840739860BDC7163092F0644343D0B2C03E4925783`, image base `0x00400000`). Different client builds may have different addresses. Verify your binary matches before using these anchors.

**Entry point**: `EntityDescription_Parse @ ghidra://SGW.exe@0x01593cd0` opens `entities/defs/<name>.def`, resolves the `<Parent>` chain recursively (parent first), reads `<ClientName>` and `<ServerOnly>`, then calls `EntityDescription_ParseDef @ ghidra://SGW.exe@0x01593600` to parse `<Implements>` → `<Properties>` → `<ClientMethods>` → `<CellMethods>` → `<BaseMethods>` in that fixed order.

![EntityDescription_Parse recursion tree showing parent chain resolved first, then Implements interfaces in XML order, then own Properties/ClientMethods/CellMethods/BaseMethods sections, with propID and methodID counters accumulating across the tree.](figures/entity-property-sync-01-parse-order-tree.svg)

*Figure 1: parse order — `EntityDescription_Parse` walks `<Parent>` recursively (parent first), then `<Implements>` in XML order, then own sections in the fixed order Properties → ClientMethods → CellMethods → BaseMethods. propID and methodID counters accumulate left-to-right across the tree, so a parent's last ID + 1 is the next contributor's first ID. SGWPlayer's full cascade is detailed in Figure 8.*

---

### 1.2 Property table and flag bits

**Confirmed** by decompilation of `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` and `DataDescription_ParseFlags @ ghidra://SGW.exe@0x015974a0` (named `DataDescription_parse_2` in the old RE doc).

Each property in the `<Properties>` XML section is parsed into a `DataDescription` struct. The flags byte at `DataDescription+0x20` is an 8-bit bitmask:

| Bit | Hex | Flag | `.def` keyword |
|-----|-----|------|----------------|
| 0 | `0x01` | `DATA_GHOSTED` | `CELL_PUBLIC` |
| 1 | `0x02` | `DATA_OTHER_CLIENT` | `OTHER_CLIENTS` |
| 2 | `0x04` | `DATA_OWN_CLIENT` | `OWN_CLIENT` |
| 3 | `0x08` | `DATA_BASE` | `BASE` |
| 4 | `0x10` | `DATA_CLIENT_ONLY` | (no SGW `.def` keyword — bit exists only in the binary flag-bit space) |
| 5 | `0x20` | `DATA_PERSISTENT` | `<Persistent>true</Persistent>` |
| 6 | `0x40` | `DATA_EDITOR_ONLY` | `EDITOR_ONLY` |
| 7 | `0x80` | `DATA_ID` | `<Identifier>true</Identifier>` |

![Eight-bit register layout for the property flags byte at DataDescription+0x20, with bit positions, hex values, and .def keywords for each of the eight flag bits; bits 1 and 2 marked unused in SGW.](figures/entity-property-sync-02-property-flag-bits.svg)

*Figure 2: bitmask layout of the 8-bit `DataDescription+0x20` flag byte. Bits 1 (`DATA_OTHER_CLIENT`) and 2 (`DATA_OWN_CLIENT`) exist in the parser's table but never set in SGW — see §2.3 and Figure 7 for the keyword-surface divergence.*

Persistence is injected at parse time (`*pOutFlags |= 0x20` when `"Persistent"` child is true); identifier likewise (`|= 0x80` when `"Identifier"` is true). Both are confirmed in the decompile of `DataDescription_ParseFlags`.

**Four filtered lists** the parser builds (confirmed by `EntityDescription_ParseProperties` decompile):

1. **All-properties array** at `EntityDescription+0x5c/+0x60` (element size `0x40`): every non-`EDITOR_ONLY` property, in parse order.
2. **Client-property pointer array** at `EntityDescription+0x70/+0x74`: indices of properties where `flags & 0x06 != 0` (`OWN_CLIENT` or `OTHER_CLIENT`). These get sequential client-propID ordinals.
3. **Property name→index map** at `EntityDescription+0x7c` (`std::map<string, uint>`): red-black tree for name lookup.
4. **Reserved-name set** (local `auStack_c4` in the parse loop): five SGW-specific names registered in a separate lookup structure.

*Source-doc override (CME divergence — see §2.3 for the SGW `.def` keyword surface):* The `flags & 0x06` filter at `+0x70/+0x74` is binary-correct as described above. But in SGW the array is effectively **always empty**: §2.3's audit of `DataDescription_ParseFlagStr @ ghidra://SGW.exe@0x015959c0` confirms `CELL_PUBLIC` maps to bit 0 only (`DATA_GHOSTED = 0x01`) and `BASE` maps to bit 3 only (`DATA_BASE = 0x08`). Neither sets bit 1 or bit 2, and no SGW `.def` keyword does. Client property updates in SGW are therefore routed via the **main DataDescription array at `+0x5c/+0x60`**, not via the client-property pointer array at `+0x70/+0x74`. This diverges from stock BigWorld, which assumes `OWN_CLIENT` / `OTHER_CLIENTS` keywords are in use and the `+0x70/+0x74` array is the routing table. Audit-confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Appendix A (decompile of `EntityDescription_ParseProperties` Conditional 2 at `ghidra://SGW.exe@0x015924a0`).

**The five excluded names** registered in the reserved-name set (confirmed verbatim in the `EntityDescription_ParseProperties` decompile):

- `publicReservationData`
- `publicMissionData`
- `completedMissions`
- `aggressionOverrides`
- `effectMonikers`

These names appear in the complex-type validation set and the reservation lookup; they are **not inserted into the client-property pointer array** even if they carry client-visible flags.

**Type restrictions on propagated properties**: properties with `flags & 0x06 != 0` (client-visible) that resolve to type names `"PYTHON"`, `"USER_TYPE"`, `"CLASS"`, `"ARRAY"`, `"TUPLE"`, or `"FIXED_DICT"` with complex-subtype members trigger warning logs during parse but are not excluded from the arrays. The specific log format strings confirmed in decompile: `"Property: %s.%s: properties should not be propagated to the client.\n"`, `"Property: %s.%s: USER_TYPE properties should not be propagated.\n"`, etc.

---

### 1.3 Method table and three categories

**Confirmed** by decompilation of `EntityDescription_ParseDef @ ghidra://SGW.exe@0x01593600` and `MethodDescription_parse @ ghidra://SGW.exe@0x01594f60`.

`EntityDescription_ParseDef` dispatches to five section parsers in strict sequential order (confirmed by the full decompile call chain):

| Parser | Address | Section | Call position |
|--------|---------|---------|---------------|
| `EntityDescription_ParseImplements` | `ghidra://SGW.exe@0x01593930` | `<Implements>` — walks interface .def files | 1st |
| `EntityDescription_ParseProperties` | `ghidra://SGW.exe@0x015924a0` | `<Properties>` | 2nd |
| `EntityDescription_ParseClientMethods` | `ghidra://SGW.exe@0x01593420` | `<ClientMethods>` — server→client only | 3rd |
| `EntityDescription_ParseCellMethods` | `ghidra://SGW.exe@0x015934c0` | `<CellMethods>` — client→CellApp RPCs | 4th |
| `EntityDescription_ParseBaseMethods` | `ghidra://SGW.exe@0x01593560` | `<BaseMethods>` — client→BaseApp RPCs | 5th |

`EntityDescription_ParseImplements @ ghidra://SGW.exe@0x01593930` is the first call inside `ParseDef`. It walks each `<Interface>` child of the `<Implements>` XML section and recursively loads the interface `.def` file via the same parse chain. Interface properties and methods are contributed to the same sequential ID counters as the entity's own sections, in the order the `<Implements>` list declares them. This confirms the full parse ordering: `<Implements>` interfaces before `<Properties>` before any method category.

IDs are assigned sequentially within each category, starting from `0`, in parse order (parent first, then `<Implements>` interfaces in declaration order, then own methods). The `<Exposed/>` tag is the flag that makes a CellMethod or BaseMethod callable from the client side; ClientMethods do not need it.

`MethodDescription_parse` stores the `<Exposed/>` tag as a flag at `MethodDescription+0x1c`, bit 2 (`0x04`). Confirmed in `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` which filters with `(*(byte *)((int)pvVar6 + 0x1c) & 4) != 0` when iterating CellMethods and ClientMethods.

---

### 1.4 Sub-slot client method encoding

**Confirmed** by decompilation of `EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0` and `EntityDescription_DecodeClientMethodId @ ghidra://SGW.exe@0x01590ee0`.

BigWorld supports more than 256 exposed client methods by using a two-byte encoding for methods past a threshold. The threshold is **not a global constant** — it is computed dynamically per entity type:

```text
nExposedCount = (ptr24 - ptr20) / 4   // count of exposed client methods
iVar2         = (nExposedCount + 0xC0) / 0xFF
idBase        = 0x3E - iVar2           // single-byte region = [0, idBase)
```

For an entity with few exposed methods (`nExposedCount <= 62`), `iVar2 = 0` (integer division) and `idBase = 0x3E = 62`. Methods at index 0..61 get single-byte encoding; methods at index 62+ get two-byte encoding.

**SGWPlayer has 157 exposed client methods** (confirmed by old RE doc §13, which sourced this from the SGWPlayer parse chain). Plugging in: `iVar2 = (157 + 0xC0) / 0xFF = 0x15D / 0xFF = 1`. `idBase = 0x3E - 1 = 61`.

For SGWPlayer specifically: methods 0–60 → single-byte wire encoding; methods 61–156 → two-byte encoding.

**Encode/decode mechanics** (from the decompile):

- *Single-byte* (serial `i < idBase`): `MethodDescription+0x44 = i`, `MethodDescription+0x48 = 0xFFFFFFFF`.
- *Two-byte* (serial `i >= idBase`): `MethodDescription+0x44 = high_byte + idBase`, `MethodDescription+0x48 = low_byte`, where `high_byte = (i - idBase) >> 8`, `low_byte = (i - idBase) & 0xFF`.
- Decode: `result = iVar1` if `iVar1 - idBase < 0`; else `result = (iVar1 - idBase) * 0x100 + sub_byte + idBase`.

![Decision tree for the sub-slot client-method encoder showing the idBase computation, the i < idBase single-byte branch, the i >= idBase two-byte branch, and the SGWPlayer worked example with idBase=61 and the indices 60/61/156 cases.](figures/entity-property-sync-03-subslot-encoding.svg)

*Figure 3: sub-slot encoding decision tree. The threshold `idBase = 0x3E - (nExposed + 0xC0) / 0xFF` is computed per-entity. For SGWPlayer (`nExposed = 157`), `idBase = 61`: indices 0–60 take the single-byte path; indices 61–156 take the two-byte path. The first two-byte method is index 61 (`minigameCallDisplay`); see §2.7 and Figure 8 for the parse-cascade context.*

*Source-doc override (old `entity-property-sync.md` finding doc §13):* The old doc stated that the threshold `0x3e = 62` matches the BigWorld 2.0.1 `checkExposedForSubSlots()` boundary exactly and that sub-slot encoding applies to all methods at index 62 and above. This is only correct for entities with `nExposedCount <= 62`. For SGWPlayer with 157 methods, the threshold is **61**, not 62. The Ghidra decompile at `0x01590df0` makes the formula unambiguous.

---

### 1.5 Universal RPC dispatcher — on-wire method byte

**Confirmed** by decompilation of `RouteOutgoingEntityRpc @ ghidra://SGW.exe@0x00c6fc40`, `ServerConnection_StartEntityMessage @ ghidra://SGW.exe@0x00dd6a60`, and `ServerConnection_StartProxyMessage @ ghidra://SGW.exe@0x00dd6980`.

`RouteOutgoingEntityRpc` is the single exit point for all outgoing entity method calls. Its dispatch logic:

1. Reads dispatch-type from `pArgData+0x1c` bits 0–1: `0` = CellApp (entity) method; `2` = BaseApp (proxy) method.
2. For CellApp: calls `ServerConnection_StartEntityMessage(pvVar5, methodId, entityId_byte)`. Wire encoding: `(methodId & 0x7F) | 0x80`. Then writes a 4-byte entity ID slot via vtable+0x10.
3. For BaseApp: calls `ServerConnection_StartProxyMessage(pvVar5, methodId, 0)`. Wire encoding: `(methodId & 0x3F) | 0xC0`. No entity-ID write (the channel's bound entity is implicit).

Confirmed verbatim in the `StartEntityMessage` decompile: `_DAT_01ef2630 = CONCAT71(DAT_01ef2630_1, pThis._0_1_) | 0x80`. Confirmed in `StartProxyMessage`: `_DAT_01ef2610 = CONCAT71(DAT_01ef2610_1, pThis._0_1_) | 0xc0`.

*Source-doc override (old `entity-property-sync.md` finding doc §3):* The old doc showed `methodID | 0x80` / `| 0xC0` without noting the AND-mask. **The base path uses `methodID & 0x3F` first** (6-bit primary ID space, not 7). The cell path uses `methodID & 0x7F` (7-bit). These are 6-bit vs 7-bit ID spaces respectively.

**Cell-method wire shape**: `[0x80..0xBF: (methodId & 0x7F) | 0x80]` `[u16 WORD_LENGTH]` `[u32 entity_id]` `[serialized args]`

**Base-method wire shape**: `[0xC0..0xFF: (methodId & 0x3F) | 0xC0]` `[u16 WORD_LENGTH]` `[serialized args]`  (no entity_id — proxy is bound per-channel)

**Volatile cell-method variant.** `RouteEntityMessageToHandler @ ghidra://SGW.exe@0x00dd66e0` reads the byte at message offset 0 and routes on bit 6: when `flags & 0x40` is set the call goes through `vtable+0x20(flags & 0x3F)` (volatile / unreliable path, 6-bit method index space inside the cell range); otherwise it goes through `vtable+0x24(flags & 0x7F, …)` (reliable cell-method path). Volatile cell entity messages therefore mask with `0x3F` after the cell `0x80` marker — the high two bits encode reliability + cell-marker. `InstallEntityMessageHandlerVtable @ ghidra://SGW.exe@0x00dd6690` is the install site if you need to chase the vtable.

**Wire-confirmed.** A fresh decryption of `game/sgw/Working/binaries/sessions/2026-05-16_08-21.pcap` showed 112 cell-method bytes in `0x80..0xBF` and 39 base-method bytes in `0xC0..0xFF` (no out-of-range values). The cell range included 35 `0xBD` sub-slot sentinels (the §1.4 two-byte trigger) and named methods `ON_ENTITY_FLAGS = 0x84`, `BEING_APPEARANCE = 0x9A`, `ON_ENTITY_TINT = 0x8A`, `ON_LEVEL_UPDATE = 0x8F`, `ON_STATE_FIELD_UPDATE = 0x93`, `ON_VISIBLE = 0x88`, `INTERACTION_TYPE = 0x83` — every observed byte matches `(methodId & 0x7F) | 0x80`. The base range hit the **`0xFF` boundary** at base-method index 63: `(63 & 0x3F) | 0xC0 = 0xC0 | 0x3F = 0xFF`, the highest valid base wire byte and a useful explicit witness to the 6-bit mask. See audit Appendix C.6/C.7.

![RouteOutgoingEntityRpc dispatch fork showing the bits-0..1 read at pArgData+0x1c, the cell branch through StartEntityMessage with the 0x80 OR mask and a 4-byte entityId write, and the base branch through StartProxyMessage with the 0xC0 OR mask and no entityId.](figures/entity-property-sync-04-cell-vs-base-dispatch.svg)

*Figure 4: cell-method vs base-method dispatch fork inside `RouteOutgoingEntityRpc`. The two bits at `pArgData+0x1c` select the path; the cell path uses a 7-bit primary ID space (`0x80..0xBF`) and emits a 4-byte entity ID slot, while the base path uses a 6-bit primary ID space (`0xC0..0xFF`) and omits the entity ID (the proxy is implicit on the channel).*

The method-ID lookup feeding the dispatcher is `EntityDescription_FindMethodIdByName @ ghidra://SGW.exe@0x0158e710`, which returns a `uint16` from an MSVC red-black tree; sentinel `0xFFFF` means "not found". Its companion `EntityDescription_FindAndWritePropertyByName @ ghidra://SGW.exe@0x0158e780` is the property-lookup equivalent used by the emit path.

The companion upstream handler that processes *inbound* entity RPC events is `ProcessEntityMethodEmission @ ghidra://SGW.exe@0x00c6f8f0`, which traverses the same red-black tree and dispatches to the CME `Event_NetIn_EntityProperty` signal.

---

### 1.6 `createBasePlayer` wire format

**Confirmed** by decompilation of `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0`.

`createBasePlayer` carries msg_id `0x05` in the BigWorld InterfaceElement table, with WORD_LENGTH framing. The handler reads:

```text
createBasePlayer wire layout:
  Offset 0: entityId  u32 LE  — player entity ID; stored at ServerConnection+0x16c
  Offset 4: typeId    u16 LE  — entity class index in entities.xml (0x03 = SGWPlayer)
  Offset 6: [property stream — variable, handled by entity-creation delegate]
```

The decompile shows `*(undefined4 *)((int)this + 0x16c) = uVar1` after the 4-byte read, confirming `+0x16c` is the `playerEntityId` slot.

Property stream format for `createBasePlayer`: properties filtered by `CLIENT_DATA | BASE_DATA` (properties with `DATA_OWN_CLIENT (0x04)` or `DATA_BASE (0x08)` flags), serialized in sequential propID order with no propID prefix bytes — the client knows the ordering from the schema fingerprint.

**Wire-confirmed.** Three `createBasePlayer` messages decrypted from `sessions/2026-05-16_08-21.pcap` show the `[u32 entityId][u16 typeId][property stream]` shape unambiguously — payloads `01 00 00 00 07 00`, `02 00 00 00 02 00`, `01 00 00 00 07 00` decode to entityIDs (1, 2, 1) and typeIDs (7, 2, 7) matching `SGWDuelMarker` and `SGWBeing` from §2.5's table. The captured property streams were zero bytes — consistent with both entity types being all-`CELL_PRIVATE`/no-OWN_CLIENT props (the SGW divergence in §1.2). The session ended before any SGWPlayer (typeID `0x03`) instance emitted, so a non-empty property stream is not yet wire-witnessed; the byte-offset layout, however, is locked. See audit Appendix C.3.

**Buffering rule**: after reading entityId and typeId, if the `createCellPlayer` message buffer at `ServerConnection+0xfe0` has pending content (`remainingLength() > 0`), the handler immediately calls `ServerConnection_CreateCellPlayer` on the buffered data. Confirmed by the decompile: `if (0 < iVar4) { ... ServerConnection_CreateCellPlayer(..., pStream_00); ... }`.

---

### 1.7 `createCellPlayer` wire format

**Confirmed** by decompilation of `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0`.

`createCellPlayer` carries msg_id `0x06`, WORD_LENGTH-framed at **32 bytes** (fixed-size payload). The handler reads:

```text
createCellPlayer wire layout (32 bytes):
  Offset  0: spaceId   u32 LE  — BigWorld space identifier
  Offset  4: vehicleId u32 LE  — vehicle entity ID (0 at world entry)
  Offset  8: posX      f32 LE  — X position
  Offset 12: posY      f32 LE  — Y position (vertical)
  Offset 16: posZ      f32 LE  — Z position
  Offset 20: rotX      f32 LE  — rotation X (pitch)
  Offset 24: rotZ      f32 LE  — rotation Z (yaw)   *** Y/Z ORDER SWAP ***
  Offset 28: rotY      f32 LE  — rotation Y (roll)   *** Y/Z ORDER SWAP ***
```

Byte-offset breakdown (the same payload viewed as a structured table):

| Offset | Size | Type | Field |
|--------|------|------|-------|
| 0 | 4 | `u32` LE | `spaceId` |
| 4 | 4 | `u32` LE | `vehicleId` (always 0 at world entry) |
| 8 | 4 | `f32` LE | `posX` |
| 12 | 4 | `f32` LE | `posY` |
| 16 | 4 | `f32` LE | `posZ` |
| 20 | 4 | `f32` LE | `rotX` (pitch) |
| 24 | 4 | `f32` LE | `rotZ` (yaw) — **Y/Z ORDER SWAP** |
| 28 | 4 | `f32` LE | `rotY` (roll) — **Y/Z ORDER SWAP** |

Rotation is emitted in X, Z, Y order (not X, Y, Z). The swap is applied by `FUN_015846a0` internally. This is confirmed in the Ghidra plate comment: `"Rotation read via FUN_015846a0 which applies the swap internally"`.

> [!NOTE] **Y/Z swap is Ghidra-only.** A wire capture (audit Appendix C.4 / C.8) observed exactly one `createCellPlayer` payload during world entry, and all three rotation floats were `0.0` (default spawn orientation). The swap claim therefore remains static-decompile-only — the bit-pattern at offsets 20/24/28 is whatever `FUN_015846a0` writes there, but distinguishing yaw from roll on the wire requires a capture with non-zero spawn orientation. Server implementers should encode in X, Z, Y order per the static evidence and revisit when a non-zero-rotation capture lands.

**No property stream in the 32-byte `createCellPlayer` payload.** Cell-entity properties are delivered via the `createBasePlayer` property stream using the BASE+CLIENT domain filters. The old RE doc's reference to a `PropertyStream` after position in `createCellPlayer` is incorrect.

**Wire-confirmed (worked example).** The one `createCellPlayer` decrypted from `sessions/2026-05-16_08-21.pcap` was exactly 32 bytes: `10 00 01 00 | 00 00 00 00 | 91 1D A7 C3 | AA F1 92 42 | A8 06 64 C3 | 00 00 00 00 | 00 00 00 00 | 00 00 00 00`. Decoded: `spaceId = 0x00010010` (compound: high word `0x0001` is space type/category, low word `0x0010` is instance), `vehicleId = 0`, `(posX, posY, posZ) = (-334.231, 73.472, -228.026)` — plausible Atrea spawn coordinates, not test defaults — `(rotX, rotZ, rotY) = (0.0, 0.0, 0.0)`. The WORD_LENGTH framing in the bundle gave exactly 32 as the payload length; the iterator consumed the full payload with no remainder. Audit Appendix C.4.

*Source-doc override (old `entity-property-sync.md` finding doc §4, createCellPlayer table):* The old doc showed `[Skip 4B][SpaceID 4B][Position 12B][PropertyStream var]`. This was wrong in two ways: (1) the first 4 bytes are `spaceId`, not a skip — the old doc's "skip 4" was `spaceId` misread; (2) there is an explicit `vehicleId` field at offset 4 that the old doc didn't surface; (3) there is a 12-byte rotation field (X/Z/Y order); (4) no property stream in this message.

**Buffering rule**: if `playerEntityId` (at `ServerConnection+0x16c`) is zero when `createCellPlayer` arrives, the handler buffers the full message into `ServerConnection+0xfe0` (an `FMemoryReader`/buffer object) and asserts `createCellPlayerMsg_.remainingLength() == 0` before storing. Replay happens inside `createBasePlayer` as described in §1.6.

![Side-by-side byte layouts for createBasePlayer (variable WORD-framed, entityId u32, typeId u16, property stream) and createCellPlayer (fixed 32 bytes: spaceId, vehicleId, posX/Y/Z, rotX/Z/Y with the Y/Z swap highlighted), plus the buffering-rule callout.](figures/entity-property-sync-05-createplayer-byte-layouts.svg)

*Figure 5: wire-byte layouts for `createBasePlayer` (msg_id `0x05`) and `createCellPlayer` (msg_id `0x06`). `createBasePlayer` is WORD-length framed with a variable property stream filtered by `CLIENT_DATA | BASE_DATA`; `createCellPlayer` is a fixed 32 bytes with no property stream, and the rotation triplet is emitted in X, Z, Y order (the SGW Y/Z swap). If `createCellPlayer` arrives before `createBasePlayer`, the handler buffers it into `ServerConnection+0xfe0` and replays it once `playerEntityId` is set.*

For the position update wire format that follows (after world entry), see `spec.protocol.position-updates`.

---

### 1.8 Runtime property-change wire format

**Status: MEDIUM confidence.** The runtime property delta rides on top of the `updateEntity` system message (msg_id `0x0A`, `WORD_LENGTH`-framed) — its envelope is the Mercury bundle and its msg_id slot lives in the client descriptor table catalogued in [`spec.protocol.mercury-wire-format` §2.5](mercury-wire-format.md#25-client-descriptor-table--system-message-handlers) (msg_id 0x0A, `word-prefix`, "Per-entity property delta") and in [`spec.protocol.message-catalog`](../../protocol/message-catalog.md). Mercury does not pin a byte-by-byte payload subsection for `updateEntity` — the payload is the propID-prefixed property delta stream this section documents, fed to the client through the BigWorld→UE bridge below.

The client receiver (`FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0`) operates as a Unreal Engine `FArchive` deserializer — it reads 4 bytes from `this+0x2c` (the UE `FNetworkPropertyChange` header block) then calls `FUN_00485df0` three times to reconstruct the string/value fields. The RTTI descriptor at `ghidra://SGW.exe@0x01e91018` confirms this is `FNetworkPropertyChange` from Unreal's replication system.

The property-change *wire encoder* resides on the BigWorld server (BaseApp/CellApp), not in SGW.exe. Its output is decoded by the BigWorld client message handler before being handed to the Unreal replication layer. The BigWorld→UE bridge function is `FUN_01560ad0 @ ghidra://SGW.exe@0x01560ad0`, which:

1. Reads 4 bytes into `local_cc` (payload length) and 4 bytes into `local_c8` (change type).
2. Dispatches on `local_c8`: case 1 → `FNetworkPropertyChange`, case 2 → `FNetworkActorMove`, case 3 → `FNetworkActorCreate`, case 4 → `FNetworkActorDelete`, case 5 → `FNetworkObjectRename`, case 6 → `FNetworkRemoteConsoleCommand`.

The **propID encoding prefix** that precedes the value bytes is the server-side convention derived from BigWorld 2.0.1. Based on BW 2.0.1 `property_change.hpp` (not on disk in this checkout, but referenced in the old RE doc as a cross-check source):

| Client-property index range | Header size | Encoding |
|-----------------------------|-------------|----------|
| 0–59 | 1 byte | Direct: `propId` |
| 60–315 | 2 bytes | `0x3C`, `propId - 60` |
| 316+ | 2 bytes | `0x3D`, `propId - 316` |

Following the header, a 1-byte change-type: `0 = PROPERTY_CHANGE_TYPE_SINGLE` (full replacement); `1 = PROPERTY_CHANGE_TYPE_SLICE` (array element).

**These threshold values (60, 316) cannot be confirmed from SGW.exe alone** — the encoder is server-side. The client's `FUN_01560ad0` reads the pre-encoded stream opaquely (length + type-tag first, then delegates). The only client-side confirmation is that `EntityDescription_GetClientPropertyByIndex @ ghidra://SGW.exe@0x01590d80` maps a `nClientPropIndex` to a DataDescription. In stock BigWorld this is an index into the `+0x70/+0x74` pointer array; in SGW it indexes the main DataDescription array at `+0x5c/+0x60` (per §1.2's source-doc override, since the `+0x70/+0x74` array is empty in practice). Either way, the wire propID *is* this index. The 0x3C/0x3D threshold values must be verified against the actual server binary or BigWorld 2.0.1 source before promotion to HIGH confidence — see the implementer warning callout above.

> [!WARNING] **Implementer warning — OQ-1 in §1.15.**
> The 60/316 propID thresholds documented above are inherited from BigWorld 2.0.1 source (`property_change.hpp`). SGW.exe contains only the receiver; the encoder is server-side and not in the SGW binary. Server implementers MUST verify these thresholds empirically via wire capture (x64dbg on the 4 bytes at `this+0x2c` during a known property change, or a Mercury pcap with a witnessed property update) before shipping. A server that assumes incorrect thresholds will produce property updates the client misinterprets — silently writing the wrong propID slot, just like the §1.1 "get the table wrong by one slot" failure mode but at the header layer. This warning mirrors the source-doc-override discipline used elsewhere in the chapter for stock-BigWorld-only evidence.

*Source-doc override (old `entity-property-sync.md` finding doc §6):* The old doc cited BigWorld 2.0.1 source directly for the threshold values, which is valid cross-check evidence, but the claim cannot be independently confirmed from the SGW.exe binary alone. Confidence is MEDIUM, not HIGH.

**Wire-capture attempt (V4, audit Appendix C.5).** A `sessions/2026-05-16_08-21.pcap` capture covering character-select and the first seconds of world-entry observed **zero** `updateEntity` (msg_id `0x0A`) messages — no in-world stat updates, no inventory churn, no ability use occurred during the window. No `0x3C` or `0x3D` first-bytes appeared in server-to-client payloads. The 60/316 thresholds remain neither confirmed nor falsified. Path to closure: a capture with 60+ seconds of sustained in-world activity (a `ON_STAT_UPDATE` from a health change is the cheapest probe — method 20 on the client-method index from §2.7), or an x64dbg breakpoint at `FUN_01560ad0` capturing the prefix during a known property change. Tracked as OQ-1 in §1.15.

**Protocol invariants surfaced by Appendix B.** Three claims about *what is **not** in the property-change protocol* matter as much as the bit-layout:

- **No batch-property-change message type.** Each property change is its own InterfaceElement; `FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0` writes one property per call (three helper writes: 4-byte index + two string/value writes, no loop). Multiple simultaneous property changes arrive as **consecutive InterfaceElements in the same Mercury bundle**; the bundle aggregation layer (`spec.protocol.mercury-wire-format` §1) is what makes a server-side burst look batched. A server reimplementation that emits a custom batch-property type is encoding a phantom message that the client has no handler for. (Audit B.11 / G36.)
- **No slice or sub-field updates.** `FNetworkPropertyChange__vfunc_0` writes one complete property per call with no inner-field selector. `FixedDictDataType_ToXml @ ghidra://SGW.exe@0x01598b80` iterates all fields in a flat loop with no slice-index field. A change to any field inside a `FIXED_DICT` property causes the **full property value** to be re-serialized and sent. There is no `PROPERTY_CHANGE_TYPE_SLICE` in the SGW client decoder — only `PROPERTY_CHANGE_TYPE_SINGLE` (full replacement). Server implementers should not optimise toward partial-field deltas; the client cannot decode them. (Audit B.14 / G39.)
- **No client-side default-value filtering.** `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` emits matching DataDescriptions unconditionally — no default-value comparison exists in the loop. For actual property *values*, the client processes whatever the server sends and never compares against the `.def` `<Default>`. Default-omission is a server-side concern; if a server chooses to skip emitting properties at their default, the client just never sees them. If the server emits them, the client will accept them. (Audit B.15 / G40.)

---

### 1.9 Enter/Leave AoI — the deferred-enter cascade

**Confirmed** by decompilation of `EntityManager_EnterAoI @ ghidra://SGW.exe@0x00dd2800` and `GameEntityManager_FinishEntityLoad @ ghidra://SGW.exe@0x00dd27f0` (Ghidra name; same address as the old RE doc's AoI reference).

BigWorld's AoI introduction is not a single message — it is a **deferred-enter countdown** managed by the `GameEntityManager`. The client pre-registers an entity with an `enterCount > 0` (set during the creation message handling). Each `enterAoI` message from the server decrements the counter. When the counter reaches 0, `EntityManager_EnterWorld` is called.

`EntityManager_EnterAoI` decompile confirms three paths:

- Entity found in primary map (`this+0x18`): assert `iter->second->getEnterCount() > 0` (from `entity_manager.cpp` line 735). Decrement `entity+0x10` (enterCount). If zero: call `AddEntityListenerToMaps(this, entity, NULL)` to commit.
- Entity found in secondary map (`this+0x24`): decrement `piVar6[4]+0x10`. If zero and not `CEF_Remote`: `FUN_00c6a1e0` + `GameEntityManager_CancelDeferredNotifications`.
- Not in either map: `LookupOrEmplaceDeferredEntitySlot(this+0x30, entityId)` — decrement deferred slot counter.

*Source-doc override (old `entity-property-sync.md` finding doc §5):* The old doc stated the enter-AoI handler "Increments reference count." **This is wrong.** The actual code DECREMENTS `entity+0x10`. The BigWorld pattern is a countdown: the entity is pre-registered with `enterCount > 0` and decremented to zero as confirmations arrive. The assert `"iter->second->getEnterCount() > 0"` (confirmed verbatim in decompile) makes this unambiguous.

The `CEF_Remote` flag check (confirmed: `entity+0x18 bit 0`) gates whether the entity can complete AoI entry — remote entities are cleared differently (`*(uint *)((int)pvVar1 + 0x18) = *(uint *)((int)pvVar1 + 0x18) & 0x7ffffffd`).

After `EntityManager_EnterWorld` is called, `GameEntityManager_FlushDeferredNotifications @ ghidra://SGW.exe@0x00dd1e40` drains queued notifications (confirmed as a tail call in `GameEntityManager_FinishEntityLoad`).

The full three-phase cascade (CREATE_ENTITY msg, property-delta stream, Python `onVisible(1)` callback) is a server-driven orchestration that the Cimmeria BaseApp must implement. The client side handles the wire pieces; phase sequencing is a server responsibility.

> [!IMPORTANT] **The 7-method `createOnClient` cascade order is server-determined.** A fresh decompile of `EntityManager_HandleEntityCreate @ ghidra://SGW.exe@0x00dd2270` shows no ordered-dispatch table and no client-side enumeration of "the seven methods in order". The dispatcher calls `EntityManager_CreateEntity`, applies an initial world transform via `FUN_00e68a10`, then calls `GameEntityManager_FlushDeferredNotifications` — each incoming method call is routed independently through `GameEntityManager_DispatchEntityRpc @ ghidra://SGW.exe@0x00dd2b80` in arrival order. The 7-method sequence shown in Figure 6's sidebar reflects the order produced by the Python `createOnClient()` chain on the server (`SGWMob.py` → `SGWBeing.py` → `SGWSpawnableEntity.py`); the client decodes whatever the server sends in whatever order it sends it. A server reimplementation owns the ordering contract; the client will not flag a different order as an error. (Audit B.1 / G3.)

![State diagram for the AoI deferred-enter countdown — AwaitingDescription with enterCount initialized, EnteringAoI decrementing on each enterAoI message, EnteredWorld reached when enterCount hits zero, plus a RemoteManaged branch for entities with CEF_Remote set.](figures/entity-property-sync-06-aoi-deferred-enter.svg)

*Figure 6: AoI deferred-enter state machine. The client pre-registers an entity with `enterCount > 0` at `entity+0x10`; each `enterAoI` message decrements the counter; reaching zero fires `EntityManager_EnterWorld` and drains queued notifications. The `CEF_Remote` branch (bit 0 of `entity+0x18`) bypasses the countdown entirely. The 7-method `createOnClient` cascade sidebar (from agent memory `bigworld-engine-advisor/aoi-entity-introduction.md`) is reproduced for SGWMob.*

---

### 1.10 AoI entity creation message — `EntityManager_HandleEntityCreate`

**Confirmed** by function discovery at `EntityManager_HandleEntityCreate @ ghidra://SGW.exe@0x00dd2270` and `EntityManager_OnEntityCreate @ ghidra://SGW.exe@0x00dd20b0`.

The CREATE_ENTITY (msg_id `0x09`) message triggers entity instantiation on the client. The `GameEntityManager_DispatchEntityRpc @ ghidra://SGW.exe@0x00dd2b80` function routes incoming server messages to their handlers.

AoI entity creation (non-player entities entering a player's AoI) differs from `createBasePlayer` / `createCellPlayer` (which are player-specific). The `EntityManager_CreateEntity @ ghidra://SGW.exe@0x00dd09e0` and `EntityManager_EnterWorld @ ghidra://SGW.exe@0x00dd1d00` paths handle general entity lifecycle.

**The `enterAoI` re-entry path** uses the deferred-enter countdown as described in §1.9. The server does not send a fresh `CREATE_ENTITY` for a previously-seen entity re-entering AoI; it sends only AoI introduction messages (`onVisible(1)` Python callback cascade from the CellApp side).

**Leave-AoI — `EntityManager_LeaveAoI @ ghidra://SGW.exe@0x00dd29d0`.** Decompiled in full. The function does **not** decrement a reference count — the prior Ghidra plate comment "decrements reference count" was wrong and is a second annotation bug worth recording in the rename pass (alongside `MethodDescription_Destructor`, OQ-5). What `LeaveAoI` actually does is dispatch or defer a **method call** on leave:

1. If `g_bEntityRpcDebug (DAT_01ef2224)` is set, log entity-ID and space-ID.
2. Search the primary entity map at `GameEntityManager+0x18` for the leaving entity ID.
3. *Path A — entity NOT in primary map*: execute the stream callback directly via `nSpaceId->vtable[2]()`.
4. *Path B — entity IS in primary map*: read the stream byte-count, allocate a `0x20`-byte `MemoryOStream` via `scalable_malloc`, copy the stream data in, then queue it to the **deferred-leave slot at `GameEntityManager+0x3C`** via `LookupOrEmplaceSecondaryListenerSlot` + `FUN_0046eef0`. This is distinct from the deferred-enter slot at `+0x30` covered in §1.9.

There is no entity-table removal or CME `Event_EntityLeftAoI` emission at this call site — deferred leave delivery happens when the slot is flushed, and the entity reference is **not** explicitly freed here. (Audit B.2 / G4.)

---

### 1.11 Data domains for property streaming

**Confirmed** from `EntityDescription_ParseProperties` decompile (client-property filter `flags & 0x06`) and the `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` decompile (which filters by `(*(byte *)((int)pvVar6 + 0x20) & 6) != 0` for the property stream loop).

The BigWorld property streaming system uses a data-domain mask over the same 8-bit `DataDescription+0x20` flag byte from §1.2 to filter which properties are emitted in each message type. The domain mask is **not** a separate concept layered above the property flags — it is a bit-mask applied directly to the parsed flag byte. The three domain values that matter for SGW are:

| Domain constant | Value | Meaning | Bit(s) tested |
|----------------|-------|---------|---------------|
| `CELL_DATA` | `0x01` | Property lives on the cell entity | `DATA_GHOSTED` (the `CELL_PUBLIC` keyword's bit) |
| `CLIENT_DATA` | `0x06` | Property is client-visible | `DATA_OTHER_CLIENT (0x02) \| DATA_OWN_CLIENT (0x04)` |
| `BASE_DATA` | `0x08` | Property lives on the base entity | `DATA_BASE` (the `BASE` keyword's bit) |

Combined masks used by the create-message stream writers:

- `createBasePlayer` stream: `CLIENT_DATA \| BASE_DATA = 0x06 \| 0x08 = 0x0E` — admits OWN_CLIENT, OTHER_CLIENT, and BASE properties.
- `createCellPlayer` stream (if there were one — §1.7 confirms there isn't): `CLIENT_DATA \| CELL_DATA = 0x06 \| 0x01 = 0x07`.

`EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` applies the `0x06` (`CLIENT_DATA`) gate at decompile line `(*(byte *)((int)pvVar6 + 0x20) & 6) != 0`. Before each property's value reaches the wire, `DataDescription_WriteToStream @ ghidra://SGW.exe@0x015958b0` masks the flags byte with **`0x5f`** — clearing `DATA_PERSISTENT (0x20)` and `DATA_ID (0x80)`, since those are parse-time / persistence concerns that should never appear in the wire flag byte.

**SGW divergence reprise.** No SGW `.def` property satisfies `flags & 0x06 != 0` (§1.2 + §2.3): every property in the 18 entity defs uses `CELL_PUBLIC (0x01)`, `BASE (0x08)`, or `CELL_PRIVATE (0x00)`. So in SGW the `createBasePlayer` stream's `CLIENT_DATA | BASE_DATA = 0x0E` mask matches only the `BASE` bit (`0x08`) — `CLIENT_DATA (0x06)` never matches. The mask is correct; the SGW keyword surface just doesn't hit the client-visibility bits. Server implementers should still encode against the 0x0E filter for correctness; once any `.def` adds an `OWN_CLIENT` or `OTHER_CLIENTS` keyword the matching bit will flow through naturally. (Audit B.3 / G5.)

---

### 1.12 DataType two-registry system and alias.xml resolution

**Confirmed** by agent memory `game-archaeology-specialist/datatype-registry-system.md` (W-entity-desc-B findings, 2026-05-13) and the old RE doc §10.

Two separate `std::map<string, DataType*>` registries govern type resolution. Both are in the SGW.exe `.data` segment:

| Symbol | Address | Populated by | Queried by | Role |
|--------|---------|-------------|-----------|------|
| `g_mapDataTypeRegistry` | `DAT_01f126b8` | `DataType_RegisterBuiltins @ ghidra://SGW.exe@0x01596c40` | `DataType_BuildFromSection @ ghidra://SGW.exe@0x01597150` | BUILD path — resolves `<Type>` tags in `.def` files via `alias.xml` |
| `g_pMetaDataTypeRegistry` | `DAT_01f126b4` | `DataType_Register @ ghidra://SGW.exe@0x01597ce0` | `DataType_LookupByName @ ghidra://SGW.exe@0x01595f00` | LOOKUP path — maps C++ type names from 17 `SimpleMetaDataType<T>` static ctors |

`DataType_RegisterBuiltins` reads `entities/defs/alias.xml`. For each child element it calls `DataType_BuildFromSection` recursively and inserts `tag_name → DataType*` into `g_mapDataTypeRegistry`. This is the alias expansion path: `alias.xml` may declare `"INT8"` as an alias for `IntegerDataType<signed_char>`, and that mapping lands here.

`DataType_Register` is called by all 17 `SimpleMetaDataType<T>` constructors during static initialization. It lazy-allocates `g_pMetaDataTypeRegistry` and inserts `TypeName → SimpleMetaDataType<T>*`. Duplicate registration logs `"MetaDataType::addType: %s has already been registered."`.

**W4-B2 ambiguity resolved**: both `g_pMetaDataTypeRegistry` and a previously-hypothesized `g_mapDataTypeRegistryLookup` were thought to be at `DAT_01f126b4`. There is only one object at that address. The correct canonical name is `g_pMetaDataTypeRegistry`.

**17 primitive DataType subclasses** registered: `IntegerDataType<unsigned char>` through `MailBoxDataType`. See old RE doc §10 for the full table with constructor addresses `0x01599150`–`0x0159b510`.

---

### 1.13 MD5 schema fingerprint

**Confirmed** by agent memory `game-archaeology-specialist/datatype-registry-system.md` (W-entity-desc-B, 2026-05-13) and the old RE doc §11.

Each entity type's type schema is fingerprinted with a 16-byte MD5 digest fed from each `DataType::GetTypeName_WriteStream` call. The MD5 infrastructure:

| Address | Function | Notes |
|---------|----------|-------|
| `ghidra://SGW.exe@0x015a3d70` | `MD5_Init` | Sets bit_count=0, digest=[`0x67452301`, `0xefcdab89`, `0x98badcfe`, `0x10325476`] |
| `ghidra://SGW.exe@0x015a3da0` | `MD5_Update` | Thin wrapper → `MD5_Update_Block` |
| `ghidra://SGW.exe@0x015a3c00` | `MD5_Update_Block` | Core block processor; partial-block handling at byte-aligned offsets |
| `ghidra://SGW.exe@0x015a3cd0` | `MD5_Finalize` | Appends padding + 8-byte length, writes 16-byte digest |
| `ghidra://SGW.exe@0x015a3de0` | `MD5_DigestToHexString` | 16-byte digest → 32-char uppercase hex via `"0123456789ABCDEF"` at `DAT_01b1bd40` |

**How the MD5 chain is fed — schema writer confirmed:** `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` calls `MethodDescription_Destructor @ ghidra://SGW.exe@0x015942f0` for each BaseMethods, CellMethods (Exposed), and ClientMethods (Exposed) entry. Despite its Ghidra-assigned name, this function is **not a destructor** — the decompile confirms it:

1. Calls `MD5_Update` (`FUN_015a3da0 @ ghidra://SGW.exe@0x015a3da0`) with the method name string bytes at `this+0x04` (SSO field, length at `this+0x14`).
2. Calls `MD5_Update` with `this+0x1c` and `1` byte (the `<Exposed/>` flag byte).
3. Iterates the args vector `[this+0x24, this+0x28)` and calls `vtable+0x24` (= `DataType::GetTypeName_WriteStream`) on each arg's DataType.

*Source-doc override (OQ-5 resolved):* `MethodDescription_Destructor @ 0x015942f0` is definitively misnamed. Its actual role is `MethodDescription_WriteSchemaToStream` — feeding the method's name, exposed flag, and arg types into the ongoing MD5 schema fingerprint. The `bDeallocate` parameter in the Ghidra signature is actually the stream/MD5 context pointer. This Ghidra annotation is an existing bug that should be corrected in a rename pass.

Each `DataType::GetTypeName_WriteStream` feeds binary type-encoding into the MD5 stream. Type encoding per subclass (from old RE doc §11, sourced from W-entity-desc-B Ghidra work; **not independently re-verified in this pass** — treat as MEDIUM confidence pending a fresh decompile of each `GetTypeName_WriteStream`):

| Type | MD5 stream contribution |
|------|------------------------|
| Integer types (1/2/4/8 byte) | 5-byte prefix + 1 byte size |
| `FloatDataType` | Literal string `"Float"` (6 bytes) |
| `PythonDataType` | Literal string `"Python"` (7 bytes) |
| `VectorDataType<Vector2/3/4>` | `"Vector"` (7 bytes) + 4-byte dimensional marker |
| `BlobDataType` | 5-byte literal at `DAT_01b1ba80` |
| `MailBoxDataType` | Literal string `"MailBox"` (8 bytes) |

The 16-byte digest is the schema-version fingerprint. Server and client must produce the same hash for the same entity's type layout; a mismatch indicates a schema divergence and will cause the client to reject property updates silently.

**Schema-stream virtual is `vtable+0x24`** (slot index 9). Confirmed by `DataDescription_WriteToStream @ ghidra://SGW.exe@0x015958b0` — the indirect call is `(**(code**)(**(int**)(this+0x1c) + 0x24))(stream)`. `FixedDictDataType_ToXml @ ghidra://SGW.exe@0x01598b80` then reveals the `FixedDictDataType` in-memory layout the schema writer iterates over: `+0x10` is the `allowNone` flag byte; `+0x18/+0x1c` are the field-array `begin`/`end` pointers (element stride `0x28` = 40 bytes); each field stores its name string in SSO at `+0x04..+0x18` (with length at `+0x14`) and the nested DataType pointer at `+0x1c` (dispatched recursively through `vtable+0x24`). Wire schema layout per FIXED_DICT field is therefore `[name_bytes][nested_type_descriptor_via_vtable+0x24]`. (Audit B.12 / G37.)

**Runtime value serialization virtual unconfirmed.** `vtable+0x24` is the **schema-descriptor writer** (the MD5-feeding path). The DataType virtual that serializes actual property *values* on the wire is a different slot — likely `+0x28` or `+0x2c` — and was not decompiled in this pass. Promotion of the per-DataType-subclass wire layouts to HIGH confidence is blocked on tracing that slot.

**MailBoxDataType partial coverage.** `FUN_0159b480 @ ghidra://SGW.exe@0x0159b480` is the MailBoxDataType DtorBody and confirms the vtable identity as `SimpleMetaDataType<class_MailBoxDataType>::vftable @ ghidra://SGW.exe@0x0159b850`. The `vtable+0x24` slot was not decompiled in this pass; BW 1.9.1 reference says a mailbox wire value is 8 bytes (`channelId u16 + indexInComponent u16 + spaceId u32`), but that is unverified for SGW. (Audit B.13 / G38.) Tracked as OQ-4 sub-bullet in §1.15.

---

### 1.14 Source-of-truth crosswalk

| Claim | Primary evidence (Ghidra anchor or finding doc) | Cross-check |
|-------|------------------------------------------------|-------------|
| Parse order: `Implements → Properties → ClientMethods → CellMethods → BaseMethods` | `EntityDescription_ParseDef @ ghidra://SGW.exe@0x01593600` (decompile confirms exact call sequence by name) | BW 2.0.1 `entity_description.cpp`; old RE doc §1 |
| `Parent` chain resolved before own sections (`EntityDescription_Parse` recurse) | `EntityDescription_Parse @ ghidra://SGW.exe@0x01593cd0` (recursive call when `<Parent>` section found) | Old RE doc §1 |
| 8-bit property flag layout (0x01..0x80) | `DataDescription_ParseFlags @ ghidra://SGW.exe@0x015974a0` — `*pOutFlags \|= 0x20` for Persistent; `\|= 0x80` for Identifier | BW 2.0.1 `data_description.hpp`; old RE doc §2 |
| `EDITOR_ONLY (0x40)` exclusion from runtime arrays | `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` — `(local_7c >> 6 & 1) == 0` gate | Old RE doc §2 |
| Five reserved-name exclusions | `EntityDescription_ParseProperties` — verbatim string literals in decompile | Old RE doc §7 |
| Sub-slot threshold formula `idBase = 0x3E - (nExposed + 0xC0) / 0xFF` | `EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0` — direct read from decompile | `EntityDescription_DecodeClientMethodId @ ghidra://SGW.exe@0x01590ee0` — inverse confirms formula |
| SGWPlayer idBase = 61 (not 62) | Formula + 157 exposed methods (from old RE doc §13 hierarchy count) | mercury-rust-conformance audit finding #2 — wire-capture confirms 61 |
| Cell-method wire byte: `(methodId & 0x7F) \| 0x80` | `ServerConnection_StartEntityMessage @ ghidra://SGW.exe@0x00dd6a60` — `\| 0x80` confirmed; 7-bit mask via `0x7F` | mercury-wire-format.md §1.5 |
| Base-method wire byte: `(methodId & 0x3F) \| 0xC0` | `ServerConnection_StartProxyMessage @ ghidra://SGW.exe@0x00dd6980` — `\| 0xc0`; 6-bit mask via `0x3F` | mercury-wire-format.md §1.5 |
| Cell has entity-ID write; base does not | `StartEntityMessage` vtable+0x10 allocates 4 bytes after method byte; `StartProxyMessage` omits this step | Contrast of the two decompiles |
| `createBasePlayer` layout: u32 entityId + u16 typeId + property stream | `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` (stores u32 to `+0x16c`, reads u16 next, calls delegate with remainder) | Old RE doc §4; BW 2.0.1 `servconn.cpp` |
| `createCellPlayer` layout: spaceId + vehicleId + Position3D + Rotation(X/Z/Y) | `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0` (Ghidra plate: offsets 0/4/8–16/20–28) | Rotation swap confirmed by `FUN_015846a0` call note |
| `createCellPlayer` has no property stream | Same decompile — 32-byte fixed payload, no variable-length read after rotation | Contradicts old RE doc §4 |
| Buffer rule: `createCellPlayer` before `createBasePlayer` → buffered at `+0xfe0` | `ServerConnection_CreateCellPlayer` (`playerEntityId == 0` → buffer path with assertion `remainingLength() == 0`) | Old RE doc §4; BW 2.0.1 `servconn.cpp` |
| `enterAoI` decrements `entity+0x10` (enterCount), not increments | `EntityManager_EnterAoI @ ghidra://SGW.exe@0x00dd2800` (`*piVar6 = *piVar6 + -1` path; assert `getEnterCount() > 0`) | Contradicts old RE doc §5 which said "increments" |
| Two DataType registries at `DAT_01f126b4` and `DAT_01f126b8` | `DataType_RegisterBuiltins @ ghidra://SGW.exe@0x01596c40`; `DataType_Register @ ghidra://SGW.exe@0x01597ce0` | Old RE doc §10; agent memory `datatype-registry-system.md` |
| Property-change propID threshold values (60/316) | BigWorld 2.0.1 `property_change.hpp` (not in SGW.exe; server-side only) | Old RE doc §6; **NOT confirmed from binary — MEDIUM confidence** |
| `<Implements>` section parsed via `EntityDescription_ParseImplements` before `<Properties>` | `EntityDescription_ParseDef @ ghidra://SGW.exe@0x01593600` — first call is `ParseImplements(this, pvVar4, unaff_EDI)` before `ParseProperties` | Decompile call order confirmed in this pass; function address `0x01593930` freshly resolved |
| `MethodDescription_Destructor @ 0x015942f0` is misnamed — actual role is schema stream writer | Fresh decompile confirms MD5_Update calls (name bytes + exposed flag byte + arg type vtable dispatch) — not destructor teardown | Pattern: feed bytes into ongoing MD5 via `FUN_015a3da0` (= `MD5_Update`); `vtable+0x24` = `DataType::GetTypeName_WriteStream` |
| Flag-keyword parser is a 16-entry table walk (9 primary + 7 deprecated aliases), pure direct-assign | `DataDescription_ParseFlagStr @ ghidra://SGW.exe@0x015959c0` — single `*pOutFlags = table_entry[1]` with no post-OR; table at `ghidra://SGW.exe@0x01e920e0`; primary keyword strings at `ghidra://SGW.exe@0x01b1ae38`; deprecated-alias strings at `ghidra://SGW.exe@0x01b1aeb4`; warning prefix `"DataDescription::parse: Using old Fl..."` at `ghidra://SGW.exe@0x01b1af14` | Audit-confirmed [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 1 + Appendix A |
| Client-property pointer array at `+0x70/+0x74` is effectively empty in SGW | `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` Conditional 2: gate is `flags & 0x06`; SGW `.def` tree uses only `CELL_PUBLIC (0x01)`, `BASE (0x08)`, `CELL_PRIVATE (0x00)`, none of which match the mask | Audit-confirmed [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Appendix A; CME divergence from stock BigWorld |
| `CreateBasePlayer` performs no in-handler typeID validation | `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` — typeID passed directly to delegate at `*(this+0x168)` with no range check, no server-only flag test, no rejection path | Audit-confirmed [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 2; rejection (if any) happens downstream in delegate's entity-description lookup |

---

### 1.15 Open questions

**OQ-1 (HIGH priority): Property-change propID threshold — binary confirmation needed.**
The 0x3C/0x3D prefix and the 60/316 thresholds are sourced from BW 2.0.1 `property_change.hpp` (server-side). SGW.exe contains only the receiver (`FNetworkPropertyChange__vfunc_0 @ 0x015652d0`), which reads an opaque pre-encoded stream. Confirming the actual thresholds requires either (a) a live x64dbg capture of the 4 bytes at `this+0x2c` during a known property change, or (b) access to the server binary / BW 2.0.1 encoder source. Until then, the 60/316 values remain BW-source-only citations at MEDIUM confidence.

A wire-capture attempt against `game/sgw/Working/binaries/sessions/2026-05-16_08-21.pcap` produced **zero** `updateEntity` (msg_id `0x0A`) messages — the session ended before in-world property activity. No `0x3C` / `0x3D` first-bytes observed. Path to closure: a capture with 60+ seconds of sustained in-world activity (an `ON_STAT_UPDATE` from a health change is the cheapest probe, method 20 on §2.7's index), or an x64dbg breakpoint at `FUN_01560ad0`. Audit Appendix C.5 records the attempt. The implementer warning in §1.8 and R6 remain fully justified.

**OQ-X (NEW, HIGH priority): Inbound propID decoder not located.** Linked to F1 in §1.16. The outgoing serializer is at `FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0`; the **inbound** propID decoder (where the bounds check would live) is in an upstream Mercury handler not yet identified by name. Without it, F1's failure mode (out-of-range propID → crash vs. silent drop) cannot be characterized statically. Likely search surface: `ServerConnection_*` inbound dispatchers, callers of `EntityDescription_GetClientPropertyByIndex @ ghidra://SGW.exe@0x01590d80`, and the message-catalog row for msg_id `0x0A`. Closing this OQ also closes F1.

**OQ-Y (NEW, MEDIUM priority): Schema MD5 fingerprint comparison site.** Linked to F3 in §1.16. Hash production (`MethodDescription_WriteSchemaToStream` + `DataType::GetTypeName_WriteStream` chain) is documented; the **comparison** site against a server-provided value is not located. See F3 for starting points.

**OQ-2: PARTIALLY RESOLVED — DataDescription dual name fields at `element+0x24` and `element+0x40`.**
Confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 4: both `StdStringMSVC` fields at `+0x24` and `+0x40` exist in the 0x110-byte parse-time DataDescription (initialised by `DataDescription_Constructor @ ghidra://SGW.exe@0x01591fb0` alongside the `+0x04` field). `EntityDescription_FindAndWritePropertyByName @ ghidra://SGW.exe@0x0158e780` compares the two against each other and only calls `EntityDescription_WriteClientData` when they match — i.e., it skips aliased properties (where the two name fields differ) and writes non-aliased ones.

What is still open: which field carries the internal XML tag name and which carries the client-visible alias. Resolving requires tracing the write sites that populate the 0x110-byte form from the 0x40-byte parse-time form (`FUN_0158f260` only writes `+0x00..+0x3c` in the small form; the 0x110-byte form's `+0x24` and `+0x40` writes have not been traced in this pass). The audit recorded this remainder as OQ-B in its "new open questions" section.

**OQ-3: RESOLVED — `createCellPlayer` property stream is absent; 32 bytes confirmed.**
A fresh decompile of `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0` confirms the exact read sequence: 4 bytes `spaceId` + 4 bytes `vehicleId` + 12 bytes position (read as `posXY` via an 8-byte read plus a 4-byte `posZ`) + 12 bytes rotation via `BundlePrimer__read3` (which applies the X/Z/Y swap internally) = **32 bytes total**, with no tail reads before the function transitions to `GetOrAddEntityTableSlot` bookkeeping. The buffered-message path (when `*(this+0x16c) == 0`) writes the message body into a buffer and returns early — no stream reads in that branch either. Audit-confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 5.

**OQ-4: MD5 type-encoding per-subclass byte sequences.**
The `GetTypeName_WriteStream` encodings for the 17 DataType subclasses are cited from the old RE doc (W-entity-desc-B pass). They were not re-decompiled in this pass. A fresh decompile of each `GetTypeName_WriteStream` function in the `[0x01599150, 0x0159b510]` range should be done before promoting §1.13 to HIGH confidence. Particularly the Integer types' "5-byte prefix + 1 byte size" claim — the exact prefix bytes are not specified.

Sub-bullet (G37 / G38 partials): the DataType **schema** virtual is confirmed at `vtable+0x24` (§1.13 update from audit B.12); the **runtime value** virtual is unconfirmed — likely `+0x28` or `+0x2c`. `MailBoxDataType` vtable identity is confirmed (`ghidra://SGW.exe@0x0159b850`); its `vtable+0x24` slot decompile was not done, so the BW 1.9.1 reference layout (`channelId u16 + indexInComponent u16 + spaceId u32` = 8 bytes) is unverified for SGW. Closing this OQ requires (a) decompiling the runtime-value virtual for at least the four most-used DataType subclasses (`IntegerDataType<*>`, `FloatDataType`, `MailBoxDataType`, `FixedDictDataType`) and (b) decompiling `SimpleMetaDataType<class_MailBoxDataType>::vftable +0x24`.

**OQ-5: RESOLVED — `MethodDescription_Destructor` naming confirmed wrong.**
`MethodDescription_Destructor @ ghidra://SGW.exe@0x015942f0` was suspected of being misnamed. Fresh decompilation in this pass confirms the function is NOT a destructor: it calls `MD5_Update` (via `FUN_015a3da0`) with the method name bytes, then with the exposed-flag byte, then iterates the args vector invoking `vtable+0x24` (`DataType::GetTypeName_WriteStream`) on each arg's DataType. The correct name is `MethodDescription_WriteSchemaToStream`. A Ghidra rename is warranted; see §1.13 inline note. The annotation script that assigned "Destructor" matched the MSVC scalar-destructor call pattern superficially (the args vector cleanup loop resembles destructor teardown) but misidentified the function. Record in `docs/reverse-engineering/annotation-script-shift-bugs.md`.

**OQ-6: AoI property-delta stream — cache-stamp and per-witness versioning.**
The cache-stamp system (described in agent memory `bigworld-engine-advisor/cache-stamp-system.md`) provides per-witness property deltas for AoI introduction. This system involves `createCacheStamp(propertySetId, callback, invalidate)` on the server side, `MaxPropertySets = 2`, and a `CELL_BASE_UPDATE_CACHE_STAMP (0x11)` cell→base message. These are server-side behaviors not visible in SGW.exe. A future pass against the deprecated C++ BaseApp source (`deprecated/cpp/src/baseapp/entity/cached_entity.cpp`) is needed to document the server's side of this cache.

---

### 1.16 Failure modes

Six numbered **failure modes** — what happens when the wire-format contract is violated. These differ from §1.17 gotchas (which are surprises in the *protocol*, valid messages that mislead implementers) in that an F-code names a wire-level invariant *the client can detect a violation of*, and pins the decoder's response. Most are silent drops; one is a buffered hold-indefinitely (F6) which is the real protocol invariant servers must respect.

**F1 — Property update with out-of-range propID — decoder behavior UNVERIFIED.** If a property-change message arrives with a propID exceeding the entity's DataDescription array bounds, the client decoder's behavior is not confirmed. `FNetworkPropertyChange__vfunc_0 @ ghidra://SGW.exe@0x015652d0` is the *outgoing* serializer (it calls Mercury bundle write helpers); the **inbound** propID decoder and bounds checker have not yet been located. A live x64dbg session with a crafted oversized propID would resolve crash vs. silent drop. Pending — see OQ-X below; tracked alongside OQ-1 in §1.15. (Audit B.4 / G13.)

**F2 — Unknown methodID → silent drop with wide-string log.** An incoming method byte that decodes to a methodID with no registered listener results in a **silent drop** after logging `"No client->server entity description mapping found for entity type %d; message id: %d."` (wide string). Confirmed in `ProcessEntityMethodEmission @ ghidra://SGW.exe@0x00c6f8f0` — the function checks `EntityDescription_FindMethodIdByName` for sentinel `0xFFFF` and falls through without dispatch. The log is only active when `g_bEntityRpcDebug (DAT_01ef2224)` is set. **No crash, no disconnect.** A server that emits a methodID outside the entity's table will see no protocol-level error. (Audit B.5 / G14.)

**F3 — Schema MD5 fingerprint mismatch — comparison site UNVERIFIED.** The site where the client compares a schema MD5 fingerprint against a server-provided value has not been located. Searches for `MD5_Finalize`, `MD5_DigestToHexString`, and the related CryptoPP wrappers returned only the Mercury `protocol_digest` machinery (see `spec.protocol.mercury-wire-format` §2), not entity-schema fingerprint logic. Per the `datatype-registry-system.md` agent-memory note, MD5 hashing occurs during `DataType_Register @ ghidra://SGW.exe@0x01597ce0` for each registered type; whether the assembled digest is then compared against a wire-provided value (and what happens on mismatch) is unknown. Starting points for a future pass: callers of the CryptoPP MD5 functions at `ghidra://SGW.exe@0x01604e80`, and the `EntityDescription_WriteClientData` MD5-feed loop documented in §1.13. (Audit B.6 / G15.)

**F4 — Unknown typeID in `createBasePlayer` — outer handler does no validation.** `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` passes the `u16` typeID directly to its entity-creation delegate at `*(this+0x168)` with **no in-handler validation gate** — no range check, no server-only flag test, no rejection path before or after the delegate call. The delegate is a runtime function pointer that statically resolves to nothing in this pass. Failure mode inside the delegate (e.g. typeID has no client-loaded `.def`) is not visible in the outer handler — likely a silent instantiation failure when entity-description lookup misses, but x64dbg confirmation is needed. (Audit B.7 / G16; §2.5 has the same finding from the client-tree side.)

**F5 — Sub-slot decode lands outside exposed-method range → silent drop.** A wire method-byte sequence that decodes through the sub-slot formula `idBase = 0x3E - (nExposed + 0xC0) / 0xFF` (per §1.4, computed at `MethodDescription_ComputeIdBase @ ghidra://SGW.exe@0x01590bb0`) to a method index *exceeding* the entity's exposed-method count results in a red-black tree miss inside `ProcessEntityMethodEmission @ ghidra://SGW.exe@0x00c6f8f0`. The follow-up `EntityDescription_GetExposedClientMethodByIndex @ ghidra://SGW.exe@0x01590f30` returns `0` on out-of-bounds and the dispatch returns without invoking any handler. **Silent drop. No crash, no disconnect.** This is the same protocol-level invisibility as F2, reached via a different decode path. (Audit B.8 / G17.)

**F6 — Property update for entity not in client's table → BUFFERED INDEFINITELY.** This is the load-bearing one. `GameEntityManager_DispatchEntityRpc @ ghidra://SGW.exe@0x00dd2b80` handles "entity not found" by **buffering, not dropping**. When entityID is absent from the primary map at `+0x18` and is not the controlled entity, execution reaches `LAB_00dd2c99` and the dispatcher reads the byte-count, allocates a `0x20`-byte `MemoryOStream` via `scalable_malloc`, copies the message body in, and queues it to the deferred slot at `GameEntityManager+0x3C` via `LookupOrEmplaceSecondaryListenerSlot + FUN_0046eef0`. The message is held **indefinitely** — there is no TTL, no discard path, no upper bound. If the entity never re-enters AoI the buffer is never flushed; if it does re-enter, the buffered payload is replayed against the freshly-instantiated entity (which may or may not be the same logical entity, depending on the server's identity-management choices). **Implication for servers**: a server must guarantee `leaveAoI` always precedes any late property updates for a given entityID, or else the client will deliver the late update against the next instance that ever takes that ID — a ghost-delivery class of bug. The deferred slot at `+0x3C` is the same one §1.10's `LeaveAoI` writes to, so a leave + an immediate property update can race; the leave must commit first. (Audit B.9 / G18.)

> [!IMPORTANT] **Implementer takeaway for F6.** "The client buffered my late update" is not the safety net it looks like. A server that emits property updates after the matching `leaveAoI` is creating a buffered hazard the client cannot reject. The fix is server-side ordering, not retry-on-error logic.

---

### 1.17 Gotchas and surprises

The five gotchas below are the ones a server implementer is likeliest to trip on. Each maps to a numbered S-code so requirements (§1.18) and reviewers can refer back without re-explaining.

**S1 — Sub-slot threshold is method-count-dependent, not a constant `0x3E`.** The threshold `idBase = 0x3E - (nExposedCount + 0xC0) / 0xFF` (§1.4) falls to **61** for SGWPlayer because `nExposedCount = 157` (`iVar2 = 1`). A server that hard-codes `62` will produce a wire byte for SGWPlayer's 62nd exposed client method (`index = 61`) that the client decodes through the single-byte path — but the client's own table has shifted into two-byte encoding at that index, so the decoded methodID points at a different slot. The mismatch is silent: no error, just a wrong method invoked. Verified by `EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0` and §2.7's cascade arithmetic. **Lesson**: compute the threshold from each entity's `nExposedCount`; do not pull `62` from any old doc.

**S2 — `createCellPlayer` carries no property stream.** Older RE docs (`docs/reverse-engineering/findings/entity-property-sync.md` pre-2026-05) listed a `PropertyStream var` field after the rotation triplet. A fresh decompile of `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0` shows the payload is **exactly 32 bytes** — spaceId, vehicleId, position, rotation — and the handler transitions to bookkeeping immediately after. Cell-public property data rides in the `createBasePlayer` stream under the `CLIENT_DATA | CELL_DATA` domain filter (§1.6 + §1.11). Audit-confirmed OQ-3 (§1.15). A server that emits property bytes after the 32-byte `createCellPlayer` payload will desync the bundle parser and the next message in the bundle will be misread. **Lesson**: 32 bytes flat; no tail.

**S3 — Base-method ID space is 6 bits, cell-method ID space is 7 bits.** The cell path masks with `0x7F` before OR-ing `0x80`; the base path masks with `0x3F` before OR-ing `0xC0`. A server that uses `& 0x7F` for base methods will silently corrupt the high two bits of `methodId` (e.g. a base method with index 64 — `0x40` — would survive the mask, OR to `0xC0 | 0x40 = 0x80`, and land in the cell range instead). Confirmed by `ServerConnection_StartEntityMessage @ ghidra://SGW.exe@0x00dd6a60` (cell) and `ServerConnection_StartProxyMessage @ ghidra://SGW.exe@0x00dd6980` (base). **Lesson**: cell = 7 bits = `0x7F`, base = 6 bits = `0x3F`.

**S4 — Cell-vs-base routing is an in-memory flag, not a wire byte.** `RouteOutgoingEntityRpc @ ghidra://SGW.exe@0x00c6fc40` reads bits 0–1 of `pArgData+0x1c` to choose between cell (`0x80..0xBF`) and base (`0xC0..0xFE`); the routing decision is **in process memory**, not on the wire. The wire byte itself is `(methodId & mask) | top_bits` where `top_bits` is the dispatch result, not an independent header. A reimplementation that prefixes a separate "cell or base" byte ahead of the method byte is encoding a phantom field — the client expects the masked-and-ORed byte directly. **Lesson**: there is no separate route header; the route is encoded into the high bits of the method byte itself.

**S5 — The client-property pointer array at `+0x70/+0x74` is binary-correct but empty in SGW.** `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` builds a filtered array using `flags & 0x06 != 0` (bits 1+2 = `OWN_CLIENT | OTHER_CLIENT`). No SGW `.def` keyword sets either bit (§2.3 audit; `CELL_PUBLIC` sets only bit 0, `BASE` sets only bit 3, `CELL_PRIVATE` sets none). The array exists, the filter executes, but the result is always empty. Property updates in SGW route via the **main DataDescription array at `EntityDescription+0x5c/+0x60`** instead — wire propID indexes that array, not the empty filtered one. A server that follows stock BigWorld semantics (filtered array as routing table) will produce property updates that match no client property. **Lesson**: route through `+0x5c/+0x60` in SGW; `+0x70/+0x74` is dead code in this build.

**S6 — `enableEntities` body is 8 bytes of undefined stack-frame slop, not a structured field pair.** The client emits `enableEntities` (client→server, 8 bytes total) and the body is *whatever was on the emitter's stack at send time*. A wire capture (audit Appendix C.9) decrypted 9 `enableEntities` messages: representative payloads include `00 00 00 C0 40 44 00 80`, `73 00 63 00 72 00 69 00` (those last bytes spell `"scri"` in ASCII — a clear fragment of a prior wide-string stack frame contaminating the buffer), and various random-looking byte sequences. No pattern consistent with `[i32 entityId][i32 flag]` or any other structured u32 pair. This matches the mercury chapter §2 W-enable-entities finding: SGW expanded BigWorld's 1-byte `keepBase` field into 8 bytes and never assigned defined semantics to the extra 7. **Lesson**: server-side, ignore the body entirely — do not parse, do not validate against a schema; treat the 8 bytes as opaque. A defensive log if any byte is nonzero is fine; a parse is wasted code.

---

### 1.18 Server requirements

Each requirement names the wire-format invariant a server must satisfy and cross-references the §1 evidence and the matching S-code in §1.17 (or F-code in §1.16). Reviewers can use the R-code as shorthand; implementers can use the citation as the proof.

**R1 — Use the typeID matching the entity name's position in `entities.xml`** (1-based; SGWPlayer = `0x03`). §1.6 reads a `u16 typeId` at offset 4 of `createBasePlayer`; §2.5 confirms the 1-based document-index assignment. A typeID off-by-one resolves to a different entity description on the client, with no validation gate at the message handler (audit Target 2, §1.14 crosswalk). **Citation**: `game/sgw/Common/res/entities/entities.xml:1-32`; `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0`.

**R2 — Compute the sub-slot threshold `idBase` from each entity's exposed-method count.** Per §1.4: `idBase = 0x3E - (nExposedCount + 0xC0) / 0xFF`. The threshold is per-entity, not global. SGWPlayer (`nExposedCount = 157`) has `idBase = 61`; other entity types will differ. See **S1**. **Citation**: `EntityDescription_AssignClientMethodIds @ ghidra://SGW.exe@0x01590df0`.

**R3 — Encode cell-method wire bytes as `(methodId & 0x7F) | 0x80`; encode base-method wire bytes as `(methodId & 0x3F) | 0xC0`.** Cell ID space is 7 bits (128 single-byte cell methods before sub-slot encoding kicks in); base ID space is 6 bits (64 single-byte base methods). See **S3**. **Citation**: `ServerConnection_StartEntityMessage @ ghidra://SGW.exe@0x00dd6a60` (cell); `ServerConnection_StartProxyMessage @ ghidra://SGW.exe@0x00dd6980` (base).

**R4 — Send `createBasePlayer` before `createCellPlayer` for the player entity.** §1.6 buffering rule: if `createCellPlayer` arrives first, the client buffers it into `ServerConnection+0xfe0` and replays once `playerEntityId` is set by `createBasePlayer`. Reliance on the buffer is discouraged — buffered replay is a defensive path, not the intended sequence. **Citation**: `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0` (buffer branch); `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` (replay trigger).

**R5 — Respect the data-domain filters when serializing entity creation property streams.** `createBasePlayer` carries `CLIENT_DATA | BASE_DATA` (`flags & 0x04` for `DATA_OWN_CLIENT` or `flags & 0x08` for `DATA_BASE`); the cell-entity portion rides in the same `createBasePlayer` stream under `CLIENT_DATA | CELL_DATA`, **not** in a separate `createCellPlayer` payload. See **S2**. **Citation**: `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0` (filter `(flags & 6) != 0`); §1.11 domain-constants table.

**R6 — Encode property updates with the propID-prefix scheme `propId < 60 → 1 byte; 60..315 → [0x3C, propId-60]; 316+ → [0x3D, propId-316]`.** The thresholds are inherited from BigWorld 2.0.1 `property_change.hpp` and remain UNCONFIRMED in SGW.exe (OQ-1, §1.15). Until empirically verified the implementer warning in §1.8 applies: **MUST verify via wire capture before shipping**. **Citation**: §1.8 + §1.15 OQ-1.

**R7 — Route property updates via `EntityDescription+0x5c/+0x60` in SGW, not via the stock-BigWorld `+0x70/+0x74` filtered array.** The filtered array is empty in SGW (§1.2 source-doc override; §2.3 audit). See **S5**. **Citation**: audit Appendix A; `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` Conditional 2.

**R8 — Encode the 32-byte `createCellPlayer` payload as `[spaceId u32][vehicleId u32][posX/Y/Z f32×3][rotX/Z/Y f32×3]` with the Y/Z rotation swap.** No property stream. The rotation triplet is X, Z, Y on the wire — `FUN_015846a0` applies the swap internally on the client side, so a server emitting in stock-BW X, Y, Z order will misalign yaw and roll. See **S2**. **Citation**: `ServerConnection_CreateCellPlayer @ ghidra://SGW.exe@0x00dda2e0`; §1.7 byte table.

**R9 — Pre-register entities with a positive `enterCount` and decrement via `enterAoI` messages.** §1.9: the client expects `entity+0x10` to start `> 0` (asserted `getEnterCount() > 0`) and to reach 0 through successive `enterAoI` decrements before `EntityManager_EnterWorld` fires. A server that increments instead of decrements (the old RE doc's documented mistake) will never reach the trigger. **Citation**: `EntityManager_EnterAoI @ ghidra://SGW.exe@0x00dd2800`.

**R10 — Match the client's schema MD5 fingerprint exactly.** The 16-byte MD5 over each entity type's method-and-property name/exposed-flag/arg-type stream (§1.13) is the schema-version contract. A mismatch indicates schema divergence; property updates against a divergent schema may be silently rejected. The MD5 input is the concatenation of each method's `name bytes → exposed flag byte → arg types via vtable+0x24` for BaseMethods + Exposed CellMethods + Exposed ClientMethods. **Citation**: `EntityDescription_WriteClientData @ ghidra://SGW.exe@0x01590fc0`; `MethodDescription_WriteSchemaToStream @ ghidra://SGW.exe@0x015942f0` (formerly mis-named "MethodDescription_Destructor").

---

---

## Section 2 — Client findings

Section 1 reconstructed the parser, the ID-assignment math, and the wire dispatch from the SGW.exe binary. This section flips the lens onto the *files the parser consumes*: the `.def` schema tree under `game/sgw/Common/res/entities/defs/`, plus the two index files (`entities.xml`, `alias.xml`) the binary opens at startup. The client tree is the contract the server must encode against — every propID and methodID on the wire is an ordinal *into a table the client builds at process start from these files*. If the server publishes a `.def` tree that differs from the client's in property order, interface order, or method `<Exposed/>` annotation, the wire byte for "client method 65" will dispatch to a different handler on each side, silently.

The evidence base for this section is the 18 entity `.def` files, the 19 interface `.def` files under `defs/interfaces/`, `entities.xml`, `alias.xml`, and the UE3 configuration files under `game/sgw/Working/SGWGame/Config/`. Line numbers are valid throughout — `game/sgw/` is the immutable 2009 client tree.

### 2.1 The client surface for entity property sync

Entity property sync has no client-side decode logic that you can read by opening a `.cpp` file in the client tree — the decoder is C++ inside `SGW.exe`, reverse-engineered in §1. What the client *does* expose, end-user-readable, is the **schema** the decoder uses to build its propID and methodID tables. That schema is XML, lives in `game/sgw/Common/res/entities/`, and has four entry points:

1. `entities.xml` — the master type table. 18 entries map a typeID ordinal (1-based, by document order) to an entity name. The wire `typeId` field in `createBasePlayer` (§1.6) is an index into this table.
2. `defs/<EntityName>.def` — one XML file per entity, declaring its parent, the interfaces it implements, its properties, and its three method categories (`<ClientMethods>`, `<CellMethods>`, `<BaseMethods>`).
3. `defs/interfaces/<InterfaceName>.def` — interface schemas, structurally identical to entity defs except they have no `<ClientName>` element and are never instantiated directly. They contribute properties and methods to entities that `<Implements>` them.
4. `defs/alias.xml` — the user-defined-type table. Composite types (`FIXED_DICT`, `ARRAY <of>`) referenced by name in `.def` `<Type>` elements resolve through this file.

A fifth file — `defs/enumerations.xml` — defines 128 enumerations used inside the schemas, but enumerations are inlined into property types and do not carry their own wire IDs. They are not part of the property-sync contract; they are part of the type-system contract above it.

The UE3 layer (`game/sgw/Working/SGWGame/Config/*.ini`, `Working/SGWGame/Content/FRScript/*.u`) **does not configure entity property sync.** §2.10 walks the configuration files end-to-end and confirms zero entity-sync-relevant keys; the binding from the UnrealScript layer to BigWorld entity properties happens inside the native glue layer that §1.5 covers, and is configured by the `.def` files, not by INI.

Practical consequence: a server engineer answering "what properties does SGWPlayer carry?" must read `SGWPlayer.def` plus every `<Implements>` interface plus the parent chain back through `SGWBeing.def`, `SGWSpawnableEntity.def`, `SGWEntity.def`. §2.7 walks the full cascade for SGWPlayer end-to-end and produces the resulting 157-entry client-method index.

### 2.2 The `.def` file format

A BigWorld `.def` file is XML with a fixed shape. Every entity and interface uses the same top-level grammar. `SGWBeing.def` is a small, representative example — `game/sgw/Common/res/entities/defs/SGWBeing.def:1-33` is the entire file:

```xml
<root>
    <Parent>SGWSpawnableEntity</Parent>
    <Implements>
        <Interface> SGWBeing </Interface>
        <Interface> SGWAbilityManager </Interface>
        <Interface> SGWCombatant </Interface>
    </Implements>
    <Properties>
    </Properties>
    <ClientMethods>
      <BeingAppearance>
        <Arg> WSTRING                  <ArgName> BodySet       </ArgName> </Arg>
        <Arg> ARRAY <of> WSTRING </of> <ArgName> ComponentList </ArgName> </Arg>
      </BeingAppearance>
    </ClientMethods>
    <CellMethods>
    </CellMethods>
    <BaseMethods>
    </BaseMethods>
    <LoDLevels>
    </LoDLevels>
</root>
```

The six load-bearing sections, in the order the §1.3 parser reads them:

| Section | Optional? | Consumed by | What it contributes |
|---------|-----------|-------------|---------------------|
| `<Parent>` | Yes (root entities omit it) | `EntityDescription_Parse @ ghidra://SGW.exe@0x01593cd0` — recursive parent walk | All properties and methods of the parent, prepended to this entity's tables |
| `<Implements>` | Yes (empty tag = no interfaces) | `EntityDescription_ParseImplements @ ghidra://SGW.exe@0x01593930` | Each `<Interface>` child's properties + methods, in declaration order, before this entity's own |
| `<Properties>` | Yes (empty tag = no properties) | `EntityDescription_ParseProperties @ ghidra://SGW.exe@0x015924a0` | Property entries, parsed sequentially; client-visible ones land in the propID array |
| `<ClientMethods>` | Yes | `EntityDescription_ParseClientMethods @ ghidra://SGW.exe@0x01593420` | Server→client RPCs, sequential index, all implicitly exposed |
| `<CellMethods>` | Yes | `EntityDescription_ParseCellMethods @ ghidra://SGW.exe@0x015934c0` | Client→CellApp RPCs; `<Exposed/>` ones get an exposedID, others are cell-internal |
| `<BaseMethods>` | Yes | `EntityDescription_ParseBaseMethods @ ghidra://SGW.exe@0x01593560` | Client→BaseApp RPCs; same `<Exposed/>` rule |

Three more elements appear at root level and matter to the parse but not directly to wire format:

- `<UnrealProperties>` — UE3-layer integration; declares the UClass the entity binds to (e.g. `Account.def:6-8` binds Account to `GamePawn`). Not consumed by the BigWorld property parser.
- `<Volatile>` — declares position/orientation properties as continuously-updating (e.g. `SGWEntity.def:10-15` marks `position`, `yaw`, `pitch`, `roll` volatile). These properties route through the volatile-update path (see `spec.protocol.position-updates` for the volatile-property wire format), not through the property-change wire format §1.8 covers.
- `<ServerOnly/>` — marks the entity as never having a client-side instance (e.g. `SGWEntity.def:3`). The client still reads the `.def` (so the parse contributes to descendants) but never instantiates the type. `SGWPlayerGroupAuthority`, `SGWSpaceCreator`, `SGWChannelManager` are server-only entities.

A real property example, from `SGWPlayer.def:26-32` (the `playerName` property):

```xml
<playerName>
    <Type>          WSTRING         </Type>
    <Flags>         CELL_PUBLIC     </Flags>
    <Identifier>    true            </Identifier>
    <Persistent>    true            </Persistent>
    <DatabaseLength>    64          </DatabaseLength>
</playerName>
```

The XML element name (`playerName`) is the property's symbolic name, used in the name→propID red-black tree at `EntityDescription+0x7c` (§1.2). The five children are the parse inputs: `<Type>` resolves through `alias.xml` (§2.6) to a primitive or composite DataType; `<Flags>` carries the visibility bits (§2.3); `<Identifier>true</Identifier>` injects bit 7 (`DATA_ID 0x80`); `<Persistent>true</Persistent>` injects bit 5 (`DATA_PERSISTENT 0x20`); `<DatabaseLength>` is a server-side persistence hint and does not reach the wire.

`<Default>` may also appear (e.g. `SGWPlayer.def:97-101` — `respawnTimerID` has no default; `SGWBeing.def`-interface `beingName` defaults to `"An Individual"`). Defaults seed the property's initial value before any server update arrives; they do not affect the wire byte order.

### 2.3 Property visibility flags — the SGW divergence from stock BigWorld

§1.2 documented the 8-bit flag layout the binary writes into `DataDescription+0x20` and named six stock BigWorld `.def` keywords (`CELL_PUBLIC`, `OWN_CLIENT`, `OTHER_CLIENTS`, `BASE`, `CLIENT_ONLY`, `EDITOR_ONLY`). **The SGW client tree uses only three keywords**, confirmed by a grep across all 37 `.def` files in `game/sgw/Common/res/entities/defs/`:

| `.def` keyword | Times used | Stock-BW mapping (per §1.2) | What it means in SGW |
|----------------|-----------|------------------------------|----------------------|
| `CELL_PUBLIC` | ~80 occurrences | `DATA_GHOSTED (0x01)` | Property lives on the cell entity and is synced to all clients in AoI |
| `CELL_PRIVATE` | ~140 occurrences | Primary entry 0 in `DataDescription_ParseFlagStr`'s table; flag value `0x00` (sets no bit) | Property lives on the cell, server-internal only, never reaches a client |
| `BASE` | ~30 occurrences | `DATA_BASE (0x08)` | Property lives on the base entity (per-player, persistent), syncs to its owning client only |

The stock-BW keywords `OWN_CLIENT`, `OTHER_CLIENTS`, `CLIENT_ONLY`, `EDITOR_ONLY` **never appear in any SGW `.def` file**. They do, however, exist in the SGW binary's parser — `DataDescription_ParseFlagStr @ ghidra://SGW.exe@0x015959c0` iterates a 16-entry static table at `ghidra://SGW.exe@0x01e920e0` (9 primary keywords plus 7 deprecated aliases, each with a non-null warning-function pointer that emits `"DataDescription::parse: Using old Fl..."` at `ghidra://SGW.exe@0x01b1af14`). The parser is a pure table-walk with a single direct assignment (`*pOutFlags = table_entry[1]` — no post-OR), so each keyword maps to exactly the bit value stored in its table row. Audit-confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 1 and Appendix A.

The verified keyword → bit-value mapping (primary keywords only — full 16-entry table is in the audit Appendix A):

| Keyword | Flag value | Bits set | Present in SGW `.def`? |
|---|---|---|---|
| `CELL_PRIVATE` | `0x00` | (none) | yes |
| `CELL_PUBLIC` | `0x01` | `DATA_GHOSTED` only | yes |
| `OTHER_CLIENTS` | `0x03` | `DATA_GHOSTED \| DATA_OTHER_CLIENT` | no |
| `OWN_CLIENT` | `0x04` | `DATA_OWN_CLIENT` | no |
| `BASE` | `0x08` | `DATA_BASE` only | yes |
| `BASE_AND_CLIENT` | `0x0c` | `DATA_BASE \| DATA_OWN_CLIENT` | no |
| `CELL_PUBLIC_AND_OWN` | `0x05` | `DATA_GHOSTED \| DATA_OWN_CLIENT` | no |
| `ALL_CLIENTS` | `0x07` | `DATA_GHOSTED \| DATA_OTHER_CLIENT \| DATA_OWN_CLIENT` | no |
| `EDITOR_ONLY` | `0x40` | `DATA_EDITOR_ONLY` | no |

![Graph mapping the nine primary keywords from DataDescription_ParseFlagStr (entry 0 CELL_PRIVATE, entry 1 CELL_PUBLIC, entry 2 OTHER_CLIENTS, entry 3 OWN_CLIENT, entry 4 BASE, entry 5 BASE_AND_CLIENT, entry 6 CELL_PUBLIC_AND_OWN, entry 7 ALL_CLIENTS, entry 8 EDITOR_ONLY) to the eight-bit flag space, with CELL_PUBLIC, BASE, and CELL_PRIVATE highlighted as the only three keywords used in SGW .def files and the other six shown greyed out.](figures/entity-property-sync-07-flag-keyword-surface.svg)

*Figure 7: SGW flag-keyword surface. The 9-row primary keyword table in `DataDescription_ParseFlagStr` lists nine entries (entry 0 = `CELL_PRIVATE`, flag value `0x00`; entry 1 = `CELL_PUBLIC`, `0x01`; through entry 8 = `EDITOR_ONLY`, `0x40`). Only three (`CELL_PRIVATE`, `CELL_PUBLIC`, `BASE`) ever appear in the 37 SGW `.def` files. The greyed-out keywords exist in the parser but are dead surface in the client tree, which is why §1.2's `+0x70/+0x74` filter — looking for bits 1 or 2 — finds nothing to route. `CLIENT_ONLY` is NOT in the parsed-keyword surface.*

Three observations land directly on the wire format:

- `CELL_PUBLIC` sets only bit 0 (`DATA_GHOSTED`). It does **not** set bit 1 or bit 2. The earlier hypothesis that `CELL_PUBLIC` expanded into `DATA_OTHER_CLIENT (0x02)` plus `DATA_OWN_CLIENT (0x04)` is wrong — there is no post-table bit-OR step.
- `BASE` sets only bit 3 (`DATA_BASE`). It does **not** also set `DATA_OWN_CLIENT (0x04)`. The earlier hypothesis was wrong here too.
- `CELL_PRIVATE` sets no client-visibility bit. A `CELL_PRIVATE` property never reaches a client. This matches the §1.2 routing semantics.

This forces a correction to §1.2's framing of the client-property pointer array — flagged inline in §1.2 as a source-doc override. The five reserved-name exclusions documented in §1.2 (`publicReservationData`, `publicMissionData`, `completedMissions`, `aggressionOverrides`, `effectMonikers`) are SGW-specific names. They are not declared in any of the 37 `.def` files in the client tree (a grep confirms zero matches) — they are server-side property names the SGW build chose to exclude from the propagation tables. Their absence from the client tree is consistent with the §1.2 exclusion: if the client never declared them, the client never expects them on the wire.

The `<Identifier>true</Identifier>` and `<Persistent>true</Persistent>` child elements inject flag bits at parse time independently of the `<Flags>` element — confirmed in §1.2 and observable in `SGWPlayer.def:29-30` where `playerName` carries both. They do not affect client visibility (the `0x80` and `0x20` bits are not in the `0x06` mask).

### 2.4 Method declaration semantics

The three method categories in a `.def` file have different wire-direction semantics, and the `<Exposed/>` annotation has different meaning per category.

**`<ClientMethods>` — server→client.** Every method declared here is callable from server to client. The `<Exposed/>` tag is irrelevant in this section: the wire direction is one-way (server→client), and §1.5's wire byte (`0x80..0xBF: (methodId & 0x7F) | 0x80`) does not gate on exposure. ClientMethods get sequential indices in the entity's `internalMethods_` table and, per the agent-memory mapping in §2.7, those indices are also the wire methodIDs the client receives.

A representative entry, from `Communicator.def:48-53`:

```xml
<onPlayerCommunication>
    <Arg>   WSTRING         <ArgName> Speaker       </ArgName></Arg>
    <Arg>   UINT8           <ArgName> SpeakerFlags  </ArgName></Arg>
    <Arg>   UINT8           <ArgName> Channel       </ArgName></Arg>
    <Arg>   WSTRING         <ArgName> Text          </ArgName></Arg>
</onPlayerCommunication>
```

Each `<Arg>` declares one parameter. The `<Type>` of an Arg sits as the text content of the `<Arg>` element (`WSTRING`, `UINT8`, etc.). `<ArgName>` is documentation — it does not reach the wire; the client identifies the argument by its ordinal position, not its name.

**`<CellMethods>` — client→CellApp.** Cell methods need `<Exposed/>` to be callable from client. Unexposed cell methods are cell-internal helpers, invocable only from server-side code (e.g. another entity's CellApp method calling into this one).

From `interfaces/SGWBeing.def:205-208`:

```xml
<setTargetID>
    <Exposed/>
    <Arg>        INT32        <ArgName>aTargetID</ArgName></Arg>
</setTargetID>
```

The `<Exposed/>` element flips bit 2 (`0x04`) of `MethodDescription+0x1c`, confirmed in §1.3 by the `EntityDescription_WriteClientData` filter `(*(byte *)((int)pvVar6 + 0x1c) & 4) != 0`. From the client's perspective: only exposed cell methods receive a wire methodID, only those are dispatchable through the §1.5 cell-method path (`(methodId & 0x7F) | 0x80`).

Counting `<Exposed/>` occurrences across the 19 interface files shows the discipline at work: `interfaces/Communicator.def` declares 15 exposed methods, `interfaces/MinigamePlayer.def` declares 16, `interfaces/ContactListManager.def` declares 6, `interfaces/GateTravel.def` declares 1. The rest of the cell-method bodies in those files are unexposed — server-internal call surface.

**`<BaseMethods>` — client→BaseApp.** Same `<Exposed/>` discipline. `Account.def:59-104` is illustrative — Account is a thin entity used during character-select, so every BaseMethod that the login UI calls is exposed: `logOff` (line 61), `createCharacter` (line 69), `playCharacter` (line 79), `deleteCharacter` (line 85), `requestCharacterVisuals` (line 90), `onClientVersion` (line 97). Unexposed BaseMethods in the same file (e.g. `logOffInternal` at line 65, `onPlayerFailedToLoad` at line 95) are server-only and do not show up in the wire ID space.

Per §1.5, the base-method wire byte uses a 6-bit primary ID space (`(methodId & 0x3F) | 0xC0`), 64 single-byte methods before sub-slot encoding kicks in. The client-method space uses a 7-bit window via cell-method-style dispatch but is then partitioned per-entity by the dynamic sub-slot threshold from §1.4. The two spaces are independent — a base-method index of 5 and a client-method index of 5 refer to different entries, in different categories.

### 2.5 `entities.xml` — the type ID table

`game/sgw/Common/res/entities/entities.xml:1-32` is short enough to reproduce in full. Entries appear in document order; the engine assigns each one a 1-based typeID at startup based on its position:

```xml
<root>
    <SGWSpawnableEntity/>
    <SGWBeing/>
    <SGWPlayer/>
    <SGWGmPlayer/>
    <SGWMob/>
    <SGWPet/>
    <SGWDuelMarker/>
    <SGWBlackMarket/>
    <Account/>
    <SGWEntity/>
    <SGWPlayerGroupAuthority/>
    <SGWSpaceCreator/>
    <SGWSpawnRegion/>
    <SGWSpawnSet/>
    <SGWPlayerRespawner/>
    <SGWCoverSet/>
    <SGWEscrow/>
    <SGWChannelManager/>
</root>
```

18 entries. The typeID assigned to each is its 1-based document index — `SGWSpawnableEntity = 1`, `SGWBeing = 2`, `SGWPlayer = 3`, and so on. §1.6's `createBasePlayer` wire layout reads a `u16 typeId` at offset 4; that value is an index into this table.

Complete catalog (every entry in `entities.xml` line order, with the typeID the engine assigns from its 1-based position):

| typeID | Hex | Entity name | Server-only? | Notes |
|--------|------|-------------|--------------|-------|
| 1 | `0x01` | `SGWSpawnableEntity` | no | Parent of `SGWBeing`; abstract spawnable base (client-instantiable in principle but rarely seen on the wire). |
| 2 | `0x02` | `SGWBeing` | no | Parent of `SGWPlayer`, `SGWGmPlayer`, `SGWMob`, `SGWPet`. |
| 3 | `0x03` | `SGWPlayer` | no | Player entity — the §1.6 / §1.7 worked-example typeID. |
| 4 | `0x04` | `SGWGmPlayer` | no | GM tooling player variant. |
| 5 | `0x05` | `SGWMob` | no | NPC mob — the §1.9 AoI cascade worked example. |
| 6 | `0x06` | `SGWPet` | no | Player pet entity. |
| 7 | `0x07` | `SGWDuelMarker` | no | Duel zone marker. |
| 8 | `0x08` | `SGWBlackMarket` | yes (`<ServerOnly/>` in `SGWBlackMarket.def`) | Black-market vendor entity — server-only despite the player-facing name; UI rides on cell-method RPCs. |
| 9 | `0x09` | `Account` | no | Login / character-select entity (thin, no `SGWBeing` ancestry). |
| 10 | `0x0A` | `SGWEntity` | yes (`<ServerOnly/>` in `SGWEntity.def:3`) | Abstract root entity base. |
| 11 | `0x0B` | `SGWPlayerGroupAuthority` | yes | Server-only entity that manages distribution groups of player entities. |
| 12 | `0x0C` | `SGWSpaceCreator` | yes | Server-only entity that creates and maintains a space. |
| 13 | `0x0D` | `SGWSpawnRegion` | yes | Spawn region geometry (server-side). |
| 14 | `0x0E` | `SGWSpawnSet` | yes | Contains spawn point objects and spawns mobs. |
| 15 | `0x0F` | `SGWPlayerRespawner` | yes | "Basically, a graveyard" per the XML comment. |
| 16 | `0x10` | `SGWCoverSet` | yes | An entity to contain cover node objects. |
| 17 | `0x11` | `SGWEscrow` | yes | An entity to handle item transactions. |
| 18 | `0x12` | `SGWChannelManager` | yes | Channel manager. |

The agent-memory note that "`0x02 = SGWPlayer`" used in §1.6's earlier example was off-by-one. The corrected example value `0x03` is now in §1.6.

Ten of the 18 entries are server-only (each carries `<ServerOnly/>` in its `.def`): `SGWBlackMarket`, `SGWEntity`, `SGWPlayerGroupAuthority`, `SGWSpaceCreator`, `SGWSpawnRegion`, `SGWSpawnSet`, `SGWPlayerRespawner`, `SGWCoverSet`, `SGWEscrow`, `SGWChannelManager` — see the table above for each row's disposition. The client still allocates a typeID for each (the index assignment is purely positional) but never instantiates an entity of those types.

The wire effect is subtler than "the client rejects server-only typeIDs at the handler boundary". A fresh decompile of `ServerConnection_CreateBasePlayer @ ghidra://SGW.exe@0x00dddca0` shows the handler passes the `u16` typeID directly to its entity-creation delegate at `*(this+0x168)` with **no in-handler validation gate** — no range check, no server-only flag test, no rejection path before or after the delegate call. Audit-confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 2. Rejection (if any) happens one level deeper inside the delegate's entity-description lookup: a server-only entity has no client-loaded `.def`, so the delegate cannot resolve the typeID to a description and the instantiation silently fails. The client would accept the wire message and consume its bytes, but no entity would be created — there is no observable error from the protocol's perspective.

The XML comments (e.g. lines 21-22: "Server only entity that manages distribution groups") are documentation only; the parser ignores them.

### 2.6 `alias.xml` — the DataType alias table

`game/sgw/Common/res/entities/defs/alias.xml` defines the project-specific types that `.def` files reference by name in `<Type>` elements. The first two entries (lines 4-7) show the two forms aliases take:

```xml
<CONTROLLER_ID>        INT32        </CONTROLLER_ID>
<DBID>                INT64        </DBID>
```

These are simple aliases: `<CONTROLLER_ID>` in a `.def` `<Type>` element resolves to `INT32`, the BigWorld stock primitive name. `<DBID>` aliases `INT64`. The expansion is one-level; aliases of aliases would chain through, but no SGW alias appears to use that form.

The more interesting entries are composites — `FIXED_DICT` and `ARRAY` definitions. The `CharacterInfo` entry at lines 10-24 is representative:

```xml
<CharacterInfo> FIXED_DICT
    <Properties>
        <playerId><Type>        INT32       </Type></playerId>
        <name><Type>            WSTRING     </Type></name>
        <extraName><Type>       WSTRING     </Type></extraName>
        <alignment><Type>       INT8        </Type></alignment>
        <level><Type>           INT8        </Type></level>
        <gender><Type>          INT8        </Type></gender>
        <worldLocation><Type>   WSTRING     </Type></worldLocation>
        <archetype><Type>       INT8        </Type></archetype>
        <title><Type>           INT8        </Type></title>
        <playerType><Type>      INT32       </Type></playerType>
        <playable><Type>        INT8        </Type></playable>
    </Properties>
</CharacterInfo>
```

When a `.def` file declares `<Type>CharacterInfo</Type>` (e.g. inside `Account.def:33-34` via the `CharacterInfoList` array alias), the parser resolves `CharacterInfo` through `alias.xml`, finds it's a `FIXED_DICT`, and instantiates a `FixedDictDataType` whose schema is the eleven child properties in their declared order. The wire serialization of a `CharacterInfo` is then those eleven fields concatenated in that order.

`<CharacterInfoList>` at line 25 demonstrates array-of-composite: `<CharacterInfoList> ARRAY <of> CharacterInfo </of></CharacterInfoList>` — an array whose element type is the `CharacterInfo` FIXED_DICT defined above.

Inventory of `alias.xml`:

- **Simple aliases (2):** `CONTROLLER_ID → INT32`, `DBID → INT64`, `ItemID → INT32`, `LootTableID → DBID` — four entries, mostly numeric ID aliases.
- **FIXED_DICT entries (~40):** `CharacterInfo`, `VisualChoices`, `LootItemQuantity`, `LootItemDefinition`, `EscrowRecord`, `MessageHeader`, `MessageAttachment`, `MissionReward`, `MissionTaskStatus`, `MissionObjectiveStatus`, `MissionStepStatus`, `MissionStatus`, `DurationType`, `StatType`, `StatUpdate`, `StatList`, `SlotType`, `InventoryBag`, `WeaponSkillType`, `NameValuePair`, `ClientEffectResult`, `DialogChoices`, `InvItem`, `BagInfo`, `LocalTradeItem`, `LocalTradeProposal`, `RemoteTradeProposal`, `ItemCost`, `BuyCost`, `StoreItem`, `ItemCostUpdate`, `TrainerAbility`, `Respawner`, `Waypoint`, `PublicWeaponData`, `PublicCoverNodeReservationData`, `RewardItem`, `ItemGroup`, `Rewards`, `GroupChoice`, `RewardChoices`, `RegionInfo`, `CraftingInfo`, `CraftingOptions`, `NavigationPolygonEdge`, `NavigationPolygon`, `StringToken`, `AuctionItem`, `BMSearchOptions`.
- **ARRAY entries (~10):** `CharacterInfoList`, `LootItemQuantityList`, `LootItemDefinitionList`, `EscrowRecordList`, `MissionStatusList` (implicitly defined inline elsewhere), `ClientEffectResultList`, `StatUpdateList`, `CrystalBoardLayout`, `CrystalAbilityList`, `WaypointList`.

The `DataType_RegisterBuiltins @ ghidra://SGW.exe@0x01596c40` function (§1.12) is what reads this file at process startup and populates `g_mapDataTypeRegistry` at `DAT_01f126b8`. Each top-level XML element under `<root>` becomes one entry in the registry, keyed by the element name. When a later `.def` file is parsed and its `<Type>FIXED_DICT_or_alias_name</Type>` is encountered, `DataType_BuildFromSection @ ghidra://SGW.exe@0x01597150` queries this registry.

`enumerations.xml` (128 enumerations, e.g. `ELocales` at line 3, `ETargetCollectionParams` at line 11) is read by the same path but contributes a different DataType subclass per entry — the entries inline a `<Type>UINT32</Type>` or `<Type>INT32</Type>` underlying primitive plus a `<Tokens>` list mapping symbolic names to integer values. On the wire, enumeration values are encoded as their underlying primitive; the symbolic names are client-side comprehension only.

### 2.7 SGWPlayer worked example — the 157-method client index

The full parse cascade for `SGWPlayer` is the canonical worked example. The entity's `.def` is `SGWPlayer.def`; its `<Parent>` is `SGWBeing`; its `<Implements>` is 11 interfaces. Parse-order recursion produces a 157-entry client-method table.

The cascade, top-down (matching §1.1's `Parent → Implements → own sections` order applied recursively):

1. **`SGWEntity`** (root parent, `SGWEntity.def:1-146`): `<ServerOnly/>` declared at line 3, so no client instances of `SGWEntity` itself ever exist. Implements `DistributionGroupMember` (0 ClientMethods), `EventParticipant` (0 ClientMethods). Own `<ClientMethods>` section is empty (lines 53-55). **Contributes 0 client methods.**
2. **`SGWSpawnableEntity`** (`SGWSpawnableEntity.def:1-226`): Parent is `SGWEntity`. No `<Implements>`. Own `<ClientMethods>` (lines 79-154) declares 12 entries: `onStaticMeshNameUpdate`, `onSequence`, `onEntityMove`, `InteractionType`, `onEntityFlags`, `getInteractions`, `toggleInteractionDebugging`, `onEntityProperty`, `onVisible`, `onKismetEventSetUpdate`, `onEntityTint`, `onBeingNameIDUpdate`. **Contributes indices 0–11.**
3. **`SGWBeing`** (`SGWBeing.def:1-33`): Parent is `SGWSpawnableEntity`. Implements `SGWBeing` (interface, `interfaces/SGWBeing.def`), `SGWAbilityManager`, `SGWCombatant`. The `SGWBeing` interface contributes 8 ClientMethods (`onTimerUpdate`, `onEffectUserData`, `onEffectResults`, `onLevelUpdate`, `onTargetUpdate`, `onBeingNameUpdate`, `onTopSpeedUpdate`, `onStateFieldUpdate` — `interfaces/SGWBeing.def:153-201`); `SGWAbilityManager` contributes 0; `SGWCombatant` contributes 6. Own `<ClientMethods>` declares 1: `BeingAppearance` (lines 17-20). **Contributes indices 12–26 (8 + 6 + 1 = 15 methods, occupying 12 through 26).**

   Breakdown: 12–19 = SGWBeing interface (8), 20–25 = SGWCombatant (6), 26 = SGWBeing.def own (1).

4. **`SGWPlayer`** (`SGWPlayer.def:1-1448`): Parent is `SGWBeing`. Implements 11 interfaces (lines 5-19). Own `<ClientMethods>` is at lines 1111–1443 — 59 methods. Per-interface client-method counts (confirmed by grep):

   | Interface | ClientMethods count | Index range |
   |-----------|---------------------|-------------|
   | `Communicator` | 7 | 27–33 |
   | `OrganizationMember` | 18 | 34–51 |
   | `MinigamePlayer` | 13 | 52–64 |
   | `GateTravel` | 4 | 65–68 |
   | `SGWInventoryManager` | 7 | 69–75 |
   | `SGWMailManager` | 4 | 76–79 |
   | `Missionary` | 5 | 80–84 |
   | `ContactListManager` | 5 | 85–89 |
   | `SGWBlackMarketManager` | 6 | 90–95 |
   | `ClientCache` | 2 | 96–97 |
   | `SGWPoller` | 0 | (no contribution) |

   Note the count above sums to 71, occupying indices 27–97. `SGWPoller` is in the `<Implements>` list but contributes zero ClientMethods (its `interfaces/SGWPoller.def` declares only cell/base methods), so its slot in the parse traversal is a no-op.

   **Contributes indices 27–156** — 71 from interfaces plus 59 from own = 130 methods, picking up where SGWBeing left off at index 26.

**Total: 12 + 15 + 130 = 157 client methods, indices 0..156.**

This produces the agent-memory table summarised in §2.6 of `.claude/agent-memory/bigworld-engine-advisor/sgwplayer-method-index-table.md`. Verified high-frequency indices that the V5 audit confirmed live: 20 = `onStatUpdate`, 21 = `onStatBaseUpdate`, 24 = `onAlignmentUpdate`, 25 = `onFactionUpdate`, 26 = `BeingAppearance`, 28 = `onPlayerCommunication`, 31 = `onChatJoined`, 65 = `setupStargateInfo`, 69 = `onBagInfo`, 72 = `onUpdateItem`, 75 = `onCashChanged`, 101 = `onKnownAbilitiesUpdate`, 102 = `onTimeofDay`, 105 = `onDialogDisplay`, 109 = `onStoreOpen`, 115 = `onPlayerDataLoaded`, 117 = `onClientMapLoad`, 122 = `setupWorldParameters`, 125 = `addClientHintedGenericRegion`, 141 = `onAbilityTreeInfo`, 152 = `onDuelEntitiesRemove`, 155 = `onPlayMovie`.

Plugging `nExposedCount = 157` into §1.4's sub-slot threshold formula:

```text
iVar2  = (157 + 0xC0) / 0xFF = 0x15D / 0xFF = 1
idBase = 0x3E - 1            = 61
```

So for SGWPlayer specifically: client methods 0–60 use single-byte wire encoding; methods 61–156 use two-byte encoding. This is the §1.4 SGWPlayer-specific threshold, derived directly from the client tree's method count. The first method that crosses into two-byte territory is **index 61 = `minigameCallDisplay`**, the 10th `<ClientMethods>` entry (offset 9, zero-based) inside `defs/interfaces/MinigamePlayer.def`, which occupies indices 52–64. Audit-confirmed by [`entity-property-sync-section2-audit-2026-05-16.md`](../../audits/entity-property-sync-section2-audit-2026-05-16.md) Target 3 via a depth-tracking parse of the full 17-file cascade.

![Horizontal cascade tree of the SGWPlayer parse — SGWEntity (0 methods) to SGWSpawnableEntity (12 methods, indices 0..11) to SGWBeing (15 methods, indices 12..26) to the eleven implemented interfaces (Communicator through ClientCache, indices 27..97) to SGWPlayer.def own (59 methods, indices 98..156), with the sub-slot boundary between indices 60 and 61 highlighted inside the MinigamePlayer range.](figures/entity-property-sync-08-sgwplayer-parse-cascade.svg)

*Figure 8: SGWPlayer parse cascade producing the 157-entry client-method index. The parent chain (SGWEntity → SGWSpawnableEntity → SGWBeing) contributes indices 0–26; the eleven `<Implements>` interfaces in XML order contribute indices 27–97; SGWPlayer's own `<ClientMethods>` block contributes 98–156. The sub-slot boundary from Figure 3 (idBase = 61) lands inside `MinigamePlayer` (indices 52–64) — index 61 = `minigameCallDisplay` is the first two-byte-encoded method.*

### 2.8 Interfaces vs entities

Interface `.def` files under `defs/interfaces/` are structurally identical to entity `.def` files except they have no `<ClientName>` element and are never named in `entities.xml`. They contribute schema to entities that `<Implements>` them.

The 19 interface files present in the SGW client tree:

| Interface | Lines | Notes |
|-----------|-------|-------|
| `ClientCache.def` | 43 | 2 ClientMethods, 2 Exposed |
| `Communicator.def` | 247 | 7 ClientMethods, 15 Exposed cell methods |
| `ContactListManager.def` | 106 | 5 ClientMethods, 6 Exposed |
| `DistributionGroupMember.def` | 93 | 0 ClientMethods — interface contributes properties only |
| `EventParticipant.def` | 35 | 0 ClientMethods |
| `GateTravel.def` | 95 | 4 ClientMethods, 1 Exposed |
| `GroupAuthority.def` | 68 | 0 ClientMethods, 0 Exposed — server-only interface |
| `Lootable.def` | 88 | 0 ClientMethods, 0 Exposed — server-only |
| `MinigamePlayer.def` | 542 | 13 ClientMethods, 16 Exposed |
| `Missionary.def` | 191 | 5 ClientMethods, 3 Exposed |
| `OrganizationMember.def` | 454 | 18 ClientMethods |
| `SGWAbilityManager.def` | 310 | 0 ClientMethods — contributes properties only |
| `SGWBeing.def` (interface) | 303 | 8 ClientMethods — distinct from the entity `SGWBeing.def` |
| `SGWBlackMarketManager.def` | 114 | 6 ClientMethods |
| `SGWCombatant.def` | 286 | 6 ClientMethods |
| `SGWInventoryManager.def` | 222 | 7 ClientMethods |
| `SGWMailManager.def` | 107 | 4 ClientMethods |
| `SGWPoller.def` | 28 | 0 ClientMethods, 0 properties — pure cell-method aggregator |

Two interfaces — `GroupAuthority` and `Lootable` — have zero `<Exposed/>` annotations and zero ClientMethods. They contribute only cell-internal methods and (in `GroupAuthority`'s case) properties. They are server-only schema contributions; a server engineer asking "does the client see this?" can short-circuit when reading these files.

**Interfaces can implement interfaces.** `interfaces/SGWBeing.def:3-4` declares an empty `<Implements>` block, but interface-of-interface chains are valid and used elsewhere in the BigWorld engine. None of the SGW interface files currently chain — the 19 interface files all have either empty or absent `<Implements>` blocks. Confirmed by inspection of every interface file's first 20 lines.

The naming collision between `defs/SGWBeing.def` (the entity) and `defs/interfaces/SGWBeing.def` (the interface that the entity declares it implements) is intentional in BigWorld and not a bug: the entity declares `<Implements><Interface>SGWBeing</Interface></Implements>` at the top of its own file. The parser opens the same-name file under `interfaces/` to resolve the interface. The entity contributes one own ClientMethod (`BeingAppearance`) and inherits 8 from the interface, producing the §2.7 cascade indices 12–19 (interface) + 26 (own).

### 2.9 UnrealScript binding surface

The FRScript layer (`game/sgw/Working/SGWGame/Content/FRScript/*.u`, compiled UnrealScript packages) is the UI and gameplay-logic layer above the BigWorld entity system. It does **not** see propIDs or methodIDs directly. The UnrealScript side references entity properties by symbolic name (`playerName`, `bStateField`, `targetID`); the binding from UScript symbolic name to BigWorld propID happens inside the C++ native glue layer in `SGW.exe`, primarily via the `EntityDescription_FindMethodIdByName @ ghidra://SGW.exe@0x0158e710` and `EntityDescription_FindAndWritePropertyByName @ ghidra://SGW.exe@0x0158e780` lookups documented in §1.5.

The compiled `.u` packages are not human-readable in the client tree; only their CookedPC versions are shipped. The symbolic-name → propID resolution is recomputed at process start from the `.def` tree, so the FRScript layer's source-code names must match the `.def` element names exactly. A property renamed in the `.def` without the matching UScript symbol update would surface as a runtime "property not found" failure inside the C++ glue, not as a wire-format error.

The propID itself is therefore not a UScript-layer concern. UScript-layer changes do not affect entity-property-sync wire format; only `.def`-tree changes do.

### 2.10 UE3 INI knobs

A grep across `game/sgw/Working/SGWGame/Config/*.ini` for the keywords `entity`, `property`, `replication`, `BigWorld`, `BWNet` returns **zero matches** in the two engine-relevant configs (`DefaultEngine.ini`, `DefaultGame.ini`). No INI key in the SGW client touches entity-property-sync behavior.

This mirrors the mercury chapter's §2.2 "configuration is a red herring" finding: entity-property-sync, like the mercury wire format, is hard-coded into the binary's parsers and serializers. The schema lives in the XML tree under `Common/res/entities/`; the constants live in `SGW.exe`; no INI tunable exists for either propagation thresholds, sub-slot encoding boundaries, or the schema fingerprint algorithm.

The single INI key that touches BigWorld behavior in any sense is `NetworkDevice=IpDrv.BWNetDriver` in `BaseEngine.ini`'s `[Engine.Engine]` section — and that key selects the *driver class*, not any entity-sync parameter. See the mercury chapter §2.2 for the full walk of that key's effect.

Practical consequence: a server engineer changing entity-sync behavior cannot do so by shipping a config file; the change must reach the schema files (`.def`, `entities.xml`, `alias.xml`) and the C++ parsers in the binary. The schema files are the lever.

### 2.11 Source-of-truth crosswalk

| Claim | Primary client artifact | Cross-check |
|-------|--------------------------|-------------|
| 18 entity types registered in `entities.xml`, 1-based document-index typeID | `game/sgw/Common/res/entities/entities.xml:1-32` | §1.6 reads u16 typeId field; agent-memory `protocol-comparison.md` |
| `.def` parse order: `<Parent>` → `<Implements>` → `<Properties>` → `<ClientMethods>` → `<CellMethods>` → `<BaseMethods>` | Inspection of all 18 entity defs + 19 interface defs — every file follows this section order | §1.3 confirms the parser's call sequence in `EntityDescription_ParseDef` |
| `<Parent>` element is single-valued (one parent), recursive | `SGWPlayer.def:4` (`<Parent>SGWBeing</Parent>`); `SGWBeing.def:3` (`<Parent>SGWSpawnableEntity</Parent>`); `SGWSpawnableEntity.def:3` (`<Parent>SGWEntity</Parent>`); `SGWEntity.def` has no `<Parent>` — root | §1.1 confirms recursive parent walk in `EntityDescription_Parse @ ghidra://SGW.exe@0x01593cd0` |
| Only three `<Flags>` keywords used in SGW: `BASE`, `CELL_PRIVATE`, `CELL_PUBLIC` | grep across all 37 `.def` files: `find game/sgw/Common/res/entities/defs/ -name '*.def' -exec grep -h '<Flags>' {} \;` produces only these three after whitespace normalization | §1.2's 8-bit flag table includes `OWN_CLIENT`, `OTHER_CLIENTS`, `CLIENT_ONLY`, `EDITOR_ONLY` — those four stock-BW keywords are not used in the SGW client tree |
| `<Identifier>true</Identifier>` and `<Persistent>true</Persistent>` inject flag bits at parse time | `SGWPlayer.def:29-30` (`playerName` has both) | §1.2 confirms `*pOutFlags \|= 0x20` (Persistent) and `\|= 0x80` (Identifier) in `DataDescription_ParseFlags` |
| `<Exposed/>` discipline for cell+base methods, not client methods | Counts across `defs/interfaces/*.def`: 15 in `Communicator.def`, 16 in `MinigamePlayer.def`, 6 in `ContactListManager.def`, etc. ClientMethods sections never carry `<Exposed/>`. | §1.3 confirms `MethodDescription+0x1c bit 2 = 0x04` is the `<Exposed/>` flag, read in `EntityDescription_WriteClientData` filter |
| SGWPlayer has 157 client methods total | Full parse cascade walked in §2.7 — sum of interface contributions + own from `SGWPlayer.def:1111-1443` | Agent-memory table `.claude/agent-memory/bigworld-engine-advisor/sgwplayer-method-index-table.md`; §1.4 produces `idBase = 61` from this number |
| Sub-slot threshold for SGWPlayer = 61 | `nExposedCount = 157`, `iVar2 = (157+0xC0)/0xFF = 1`, `idBase = 0x3E - 1 = 61` | §1.4 formula; matches mercury-rust-conformance audit finding |
| `alias.xml` is read by `DataType_RegisterBuiltins` at startup | `game/sgw/Common/res/entities/defs/alias.xml` contains ~50 entries (simple aliases + FIXED_DICT + ARRAY) | §1.12 confirms `DataType_RegisterBuiltins @ ghidra://SGW.exe@0x01596c40` populates `g_mapDataTypeRegistry @ DAT_01f126b8` from this file |
| `enumerations.xml` contributes 128 enumerations | `game/sgw/Common/res/entities/defs/enumerations.xml` — grep `ENUMERATION` returns 128 | Enumerations resolve through the same DataType registry as alias.xml entries; see §1.12 |
| `<Volatile>` declarations route through volatile-update path, not property-change | `SGWEntity.def:10-15` declares `position`, `yaw`, `pitch`, `roll` volatile | See `spec.protocol.position-updates` (out of scope for this chapter) |
| 19 interface files; 17 contribute ClientMethods, 2 (`GroupAuthority`, `Lootable`) are pure server-side | `ls game/sgw/Common/res/entities/defs/interfaces/*.def \| wc -l`; per-file grep | §1.3's `EntityDescription_ParseImplements` walks each interface in declared order |
| Zero entity-sync-relevant INI keys | Grep for `entity`, `property`, `replication`, `BigWorld`, `BWNet` across `DefaultEngine.ini` + `DefaultGame.ini` returns zero matches | Consistent with mercury chapter §2.2's "configuration is a red herring" pattern |
| `createCellPlayer` has no property stream (OQ-3) | `SGWPlayer.def` has zero `OWN_CLIENT`/`OTHER_CLIENTS` flags — all client-visible properties are `CELL_PUBLIC` or `BASE`. CELL_PUBLIC properties land in the `CLIENT_DATA \| CELL_DATA` filter that rides in `createBasePlayer`, not in a separate `createCellPlayer` payload. | §1.7 confirms 32-byte fixed `createCellPlayer` payload; §1.11 confirms `CLIENT_DATA \| BASE_DATA` filter for `createBasePlayer` stream |
| Five reserved-name exclusions not present in any client `.def` | grep `publicReservationData publicMissionData completedMissions aggressionOverrides effectMonikers` across `defs/` returns zero matches | §1.2 — these names are SGW-specific exclusions registered in the reserved-name set; their absence from the client tree is consistent with the exclusion |

The crosswalk's load-bearing observation: **the client tree's schema is exhaustive.** Every wire field on the entity-property-sync wire derives from these XML files. A server reimplementation that parses the same files in the same order with the same flag interpretation produces the same propID and methodID tables — bit-identical. The wire format is not negotiated; it is computed from the schema, and the schema is checked in to the client tree.

---

### 2 Appendix A — SGWPlayer property catalog

Full property catalog for `SGWPlayer` produced by walking the parse cascade (`<Parent>` recursively, then `<Implements>` in XML order, then own `<Properties>`) per §1.1 + §2.7. **Total: 244 properties, propIDs 0..243**, mapped to source `.def` files. Generated programmatically by parsing every file in the SGWPlayer cascade chain (`SGWEntity.def` → `SGWSpawnableEntity.def` → `SGWBeing.def` plus 17 distinct interface files). This is the canonical ordering — a server's DataDescription table for SGWPlayer must produce these 244 entries in this propID order to stay bit-compatible with the client.

**Filter destination summary** (apply the §1.2 / §2.3 keyword → bit mapping, then §1.11 domain mask):

- **All-properties array at `EntityDescription+0x5c/+0x60`**: all 244 entries land here (zero properties carry `EDITOR_ONLY (0x40)`, so the non-EDITOR_ONLY filter doesn't exclude anyone).
- **Client-property pointer array at `+0x70/+0x74`** (`flags & 0x06`): **0 entries**. Confirms §1.2 SGW divergence — no SGW `.def` keyword sets bits 1 or 2.
- **Keyword distribution**: 192× `CELL_PRIVATE`, 43× `CELL_PUBLIC`, 9× `BASE`. No `OWN_CLIENT`, `OTHER_CLIENTS`, `CLIENT_ONLY`, `EDITOR_ONLY` instances.
- **Single property with both `<Identifier>true</Identifier>` and `<Persistent>true</Persistent>`**: `playerName` (propID 176).
- **Reserved-name exclusions hit**: 4 of the 5 §1.2 names appear in this catalog (`publicMissionData` propID 165, `completedMissions` propID 166, `effectMonikers` propID 70, `aggressionOverrides` not in any SGWPlayer-cascade file). They are flagged at parse time as "should not propagate to client" warnings (per §1.2 reserved-name logic) but still land in the main array — the reserved-name set is checked at the property-stream encoder, not at parse.

`Pers` and `Ident` columns flag `<Persistent>true</Persistent>` and `<Identifier>true</Identifier>` injections (bits 5 and 7 of the flag byte, per §1.2). Empty cells mean no injection.

| propID | Name | Type | Flags | Source | Pers | Ident |
|-------:|------|------|-------|--------|:----:|:-----:|
| 0 | `groups` | `PYTHON` | `CELL_PRIVATE` | `interfaces/DistributionGroupMember.def` |  |  |
| 1 | `groupInfoUpdateTimers` | `PYTHON` | `CELL_PRIVATE` | `interfaces/DistributionGroupMember.def` |  |  |
| 2 | `pendingJoin` | `PYTHON` | `CELL_PRIVATE` | `interfaces/DistributionGroupMember.def` |  |  |
| 3 | `baseEvents` | `PYTHON` | `BASE` | `interfaces/EventParticipant.def` |  |  |
| 4 | `cellEvents` | `PYTHON` | `CELL_PRIVATE` | `interfaces/EventParticipant.def` |  |  |
| 5 | `dbID` | `DBID` | `CELL_PRIVATE` | `SGWEntity.def` |  |  |
| 6 | `nextRequestID` | `INT32` | `CELL_PRIVATE` | `SGWEntity.def` |  |  |
| 7 | `pendingRequests` | `PYTHON` | `CELL_PRIVATE` | `SGWEntity.def` |  |  |
| 8 | `timers` | `PYTHON` | `CELL_PRIVATE` | `SGWEntity.def` |  |  |
| 9 | `createOnCell` | `MAILBOX` | `BASE` | `SGWEntity.def` |  |  |
| 10 | `kismetEventSetId` | `INT32` | `CELL_PUBLIC` | `SGWSpawnableEntity.def` |  |  |
| 11 | `SpawnSetID` | `INT32` | `BASE` | `SGWSpawnableEntity.def` |  |  |
| 12 | `staticMeshName` | `WSTRING` | `CELL_PUBLIC` | `SGWSpawnableEntity.def` |  |  |
| 13 | `bodySet` | `WSTRING` | `CELL_PUBLIC` | `SGWSpawnableEntity.def` |  |  |
| 14 | `mobId` | `INT32` | `CELL_PUBLIC` | `SGWSpawnableEntity.def` |  |  |
| 15 | `baseMobId` | `INT32` | `BASE` | `SGWSpawnableEntity.def` |  |  |
| 16 | `minigamePlayers` | `ARRAY<INT32>` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 17 | `entityVariables` | `PYTHON` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 18 | `interactDebug` | `INT8` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 19 | `shouldSendKismet` | `INT8` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 20 | `craftingStationControllerID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 21 | `spaceCreatorMailbox` | `MAILBOX` | `CELL_PRIVATE` | `SGWSpawnableEntity.def` |  |  |
| 22 | `beingName` | `WSTRING` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 23 | `level` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 24 | `visibilityID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 25 | `targetID` | `INT32` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 26 | `bStateField` | `INT32` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 27 | `primaryColorId` | `UINT32` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 28 | `secondaryColorId` | `UINT32` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 29 | `skinColorId` | `UINT32` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 30 | `currentComponentList` | `ARRAY<WSTRING>` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 31 | `disguiseEnabled` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 32 | `disguiseStaticMeshName` | `WSTRING` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 33 | `disguiseBodySet` | `WSTRING` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 34 | `disguiseComponentList` | `ARRAY<WSTRING>` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 35 | `disguiseFaction` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWBeing.def` |  |  |
| 36 | `disguiseTimerId` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 37 | `disguiseReduction` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 38 | `disguiseVisionId` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 39 | `petList` | `ARRAY<MAILBOX>` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 40 | `movementType` | `UINT8` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 41 | `detectors` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 42 | `visionChangeCallbacks` | `ARRAY<PYTHON>` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 43 | `visionExceptions` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 44 | `deathAbilityId` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWBeing.def` |  |  |
| 45 | `warmupTimer` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 46 | `bIsWarmingUp` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 47 | `lastWarmUpInterruptTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 48 | `warmUpRuntimeParams` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 49 | `pulsedEffects` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 50 | `durationEffects` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 51 | `abilityAdjustments` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 52 | `abilityCooldowns` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 53 | `categoryCooldowns` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 54 | `bDmgOff` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 55 | `bGodMode` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWAbilityManager.def` |  |  |
| 56 | `bInfiniteAmmo` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWAbilityManager.def` |  |  |
| 57 | `bIgnoreHealth` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 58 | `bIgnoreFocus` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 59 | `bNoAggro` | `INT8` | `CELL_PUBLIC` | `interfaces/SGWAbilityManager.def` |  |  |
| 60 | `bCombatDebug` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 61 | `bCombatVerboseDebug` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 62 | `debugAbilityList` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 63 | `debugEffectList` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 64 | `debugAbilityLevel` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 65 | `debugAbilityTargetID` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 66 | `channeledAbilityData` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 67 | `channeledData` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 68 | `lastChannelInterruptTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 69 | `effectComponents` | `ARRAY<PYTHON>` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 70 | `effectMonikers` | `ARRAY<PYTHON>` | `CELL_PUBLIC` | `interfaces/SGWAbilityManager.def` |  |  |
| 71 | `effectSequenceId` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 72 | `diminishingReturns` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 73 | `immuneToEffects` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 74 | `pendingAbilities` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWAbilityManager.def` |  |  |
| 75 | `entitiesDetectedStealth` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 76 | `stealthTimer` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 77 | `stealthDefaultDetector` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 78 | `revealDefaultDetector` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 79 | `Alignment` | `UINT8` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 80 | `faction` | `UINT8` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 81 | `Archetype` | `UINT8` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 82 | `threatenedMobs` | `ARRAY<INT32>` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 83 | `lastCombatTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 84 | `lastRegenTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 85 | `regenTimerID` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 86 | `NearCoverSetIDs` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 87 | `successiveShots` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 88 | `successiveShotsTarget` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 89 | `lastSuccessiveShotTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 90 | `bHealDebug` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 91 | `statsBaseMin` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 92 | `statsBaseCurrent` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 93 | `statsBaseMax` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 94 | `statsMin` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 95 | `statsCurrent` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 96 | `statsMax` | `StatList` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 97 | `reloadTimerId` | `CONTROLLER_ID` | `CELL_PUBLIC` | `interfaces/SGWCombatant.def` |  |  |
| 98 | `currentAmmoType` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWCombatant.def` |  |  |
| 99 | `ignoredList` | `ARRAY<WSTRING>` | `BASE` | `interfaces/Communicator.def` |  |  |
| 100 | `channels` | `ARRAY<PYTHON>` | `CELL_PRIVATE` | `interfaces/Communicator.def` |  |  |
| 101 | `AFK` | `UINT8` | `BASE` | `interfaces/Communicator.def` |  |  |
| 102 | `DND` | `UINT8` | `BASE` | `interfaces/Communicator.def` |  |  |
| 103 | `records` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 104 | `squad` | `INT32` | `CELL_PUBLIC` | `interfaces/OrganizationMember.def` |  |  |
| 105 | `strikeTeamTimers` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 106 | `pendingPvPTimers` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 107 | `pendingGroups` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 108 | `pendingJoins` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 109 | `pendingInvitesByType` | `PYTHON` | `CELL_PRIVATE` | `interfaces/OrganizationMember.def` |  |  |
| 110 | `minigame` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 111 | `pendingInstance` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 112 | `pendingMinigamePosition` | `VECTOR3` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 113 | `pendingItem` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 114 | `pendingMob` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 115 | `pendingSeed` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 116 | `pendingTC` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 117 | `minigameMobAttemptTracker` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 118 | `minigameItemAttemptTracker` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 119 | `minigameRegistrationCost` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 120 | `minigameRegistered` | `UINT8` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 121 | `minigameRegisteredWantsRequests` | `UINT8` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 122 | `minigameRegisteredNote` | `WSTRING` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 123 | `minigameRegisteredRange` | `UINT8` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 124 | `minigameRegistrationAvailable` | `UINT8` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 125 | `pendingHelper` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 126 | `pendingHelperBase` | `MAILBOX` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 127 | `pendingHelperExpires` | `FLOAT` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 128 | `pendingHelperTip` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 129 | `pendingHelperTicket` | `STRING` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 130 | `pendingMinigameRequests` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 131 | `currentMinigameRequest` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 132 | `minigameCallTracker` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 133 | `minigameWaitingOnCash` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 134 | `minigameSavedTimeInfo` | `FLOAT` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 135 | `minigameSavedRegistrationInfo` | `STRING` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 136 | `minigameSavedRegistrationNote` | `WSTRING` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 137 | `minigameContacts` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 138 | `minigameRequestTimer` | `INT32` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 139 | `minigameNextNPCRequest` | `FLOAT` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 140 | `pendingContactList` | `PYTHON` | `CELL_PRIVATE` | `interfaces/MinigamePlayer.def` |  |  |
| 141 | `knownStargateAddresses` | `ARRAY<PYTHON>` | `CELL_PRIVATE` | `interfaces/GateTravel.def` |  |  |
| 142 | `oldWorldID` | `INT32` | `CELL_PRIVATE` | `interfaces/GateTravel.def` |  |  |
| 143 | `gateCounter` | `INT32` | `CELL_PRIVATE` | `interfaces/GateTravel.def` |  |  |
| 144 | `destinationGate` | `INT32` | `CELL_PRIVATE` | `interfaces/GateTravel.def` |  |  |
| 145 | `destinationGateArrivalTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/GateTravel.def` |  |  |
| 146 | `playerBags` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 147 | `activeSlots` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 148 | `inventoryAdjustments` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 149 | `pendingItemTransactions` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 150 | `cash` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 151 | `weaponActivationTimerID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 152 | `weaponDeactivationTimerID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 153 | `weaponActivated` | `UINT8` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 154 | `inventoryComponents` | `ARRAY<WSTRING>` | `CELL_PUBLIC` | `interfaces/SGWInventoryManager.def` |  |  |
| 155 | `knownAmmoTypes` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 156 | `racialParadigmLevels` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 157 | `appliedSciencePoints` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 158 | `knownDisciplines` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 159 | `knownCrafts` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/SGWInventoryManager.def` |  |  |
| 160 | `mailMessages` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWMailManager.def` |  |  |
| 161 | `pendingMailMessages` | `PYTHON` | `CELL_PRIVATE` | `interfaces/SGWMailManager.def` |  |  |
| 162 | `lastMailGetTime` | `FLOAT` | `CELL_PRIVATE` | `interfaces/SGWMailManager.def` |  |  |
| 163 | `haveMailMessages` | `UINT8` | `CELL_PRIVATE` | `interfaces/SGWMailManager.def` |  |  |
| 164 | `missions` | `PYTHON` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 165 | `publicMissionData` | `ARRAY<PYTHON>` | `CELL_PUBLIC` | `interfaces/Missionary.def` |  |  |
| 166 | `completedMissions` | `ARRAY<PYTHON>` | `CELL_PUBLIC` | `interfaces/Missionary.def` |  |  |
| 167 | `currentMissionLoot` | `PYTHON` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 168 | `bShowMissionDebug` | `INT8` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 169 | `missionProcessQueue` | `PYTHON` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 170 | `pendingMissionAccepts` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 171 | `pendingMissionShares` | `PYTHON` | `CELL_PRIVATE` | `interfaces/Missionary.def` |  |  |
| 172 | `interactFlag` | `INT8` | `CELL_PRIVATE` | `interfaces/SGWPoller.def` |  |  |
| 173 | `lastPollTime` | `INT32` | `CELL_PRIVATE` | `interfaces/SGWPoller.def` |  |  |
| 174 | `contactLists` | `PYTHON` | `CELL_PRIVATE` | `interfaces/ContactListManager.def` |  |  |
| 175 | `watchedItems` | `ARRAY<INT32>` | `CELL_PRIVATE` | `interfaces/SGWBlackMarketManager.def` |  |  |
| 176 | `playerName` | `WSTRING` | `CELL_PUBLIC` | `SGWPlayer.def` | Y | Y |
| 177 | `extraName` | `WSTRING` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 178 | `account` | `MAILBOX` | `BASE` | `SGWPlayer.def` |  |  |
| 179 | `areaName` | `WSTRING` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 180 | `areaKey` | `WSTRING` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 181 | `experience` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 182 | `trainingPoints` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 183 | `knownAbilities` | `ARRAY<INT32>` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 184 | `knownPetAbilities` | `ARRAY<INT32>` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 185 | `pvpFlag` | `INT8` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 186 | `isBankingOverride` | `INT8` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 187 | `respawnTimerID` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 188 | `unstuckTimerID` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 189 | `rezDebuff` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 190 | `lastPrimaryUpdateTime` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 191 | `lastSecondaryUpdateTime` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 192 | `gainLevelLock` | `INT8` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 193 | `currentLoginTime` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 194 | `totalTimePlayed` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 195 | `timeLastLevelled` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 196 | `timeSpentThisLevel` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 197 | `interactionList` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 198 | `interactionTimer` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 199 | `logoutTimer` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 200 | `queuedAbility` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 201 | `autoCycleTimerID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 202 | `playerAvatarSetID` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 203 | `accessLevel` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 204 | `designerFlags` | `STRING` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 205 | `playerReadyFlags` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 206 | `loadTimerID` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 207 | `currentGateAddress` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 208 | `ImmunityDict` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 209 | `worldinstance` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 210 | `worldID` | `INT32` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 211 | `currentWaypoints` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 212 | `hasWitness` | `UINT8` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 213 | `bXPOff` | `UINT8` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 214 | `bInteractDebug` | `UINT8` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 215 | `perfStatsByChannel` | `UINT8` | `BASE` | `SGWPlayer.def` |  |  |
| 216 | `currentRingTransporterDestination` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 217 | `pendingRingTeleport` | `INT8` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 218 | `spaceCreatorID` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 219 | `worldinstanceMapResetDate` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 220 | `canBeSeenOld` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 221 | `systemOptions` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 222 | `playerRespawners` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 223 | `spawnRegionUpdates` | `ARRAY<MAILBOX>` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 224 | `craftingEntityFlags` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 225 | `craftingOptions` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 226 | `craftTimer` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 227 | `craftQueue` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 228 | `duelState` | `INT8` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 229 | `duelEntities` | `ARRAY<MAILBOX>` | `CELL_PUBLIC` | `SGWPlayer.def` |  |  |
| 230 | `duelEntitiesDefeated` | `ARRAY<MAILBOX>` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 231 | `duelMarker` | `MAILBOX` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 232 | `duelChallengeTimer` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 233 | `lastNoiseTime` | `FLOAT` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 234 | `gender` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 235 | `regions` | `ARRAY<INT32>` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 236 | `pvpTimer` | `CONTROLLER_ID` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 237 | `pendingPlayerFlags` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 238 | `availableDialogs` | `PYTHON` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 239 | `clientVersion` | `WSTRING` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 240 | `languageId` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 241 | `numRespecAbility` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 242 | `numRespecCrafting` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |
| 243 | `initialBandolierSlot` | `INT32` | `CELL_PRIVATE` | `SGWPlayer.def` |  |  |

**Cross-link to client-method index** — §2.7 covers the parallel 157-entry SGWPlayer **client-method** cascade. The property cascade above and the method cascade in §2.7 walk the same parse chain but emit into distinct ID spaces (§1.1). Both must be reproduced bit-identically by a server reimplementation.

---

### 2 Appendix B — 18-entity method-count table

Cascade-walked totals (parent recursive + implements + own) for every entity in `entities.xml`, in document order. Counts produced programmatically by the same walker that produced Appendix A; "Exposed" subscript counts the methods that get a wire methodID (cell/base methods with `<Exposed/>`). Server-only entities are flagged `[SO]` after the name.

| TypeID | Entity | Parent chain | Props | ClientMethods | CellMethods (Exposed) | BaseMethods (Exposed) |
|-------:|--------|--------------|------:|--------------:|----------------------:|----------------------:|
| 0x01 | `SGWSpawnableEntity` | SGWEntity | 22 | 12 | 43 (0) | 2 (0) |
| 0x02 | `SGWBeing` | SGWSpawnableEntity → SGWEntity | 99 | 27 | 89 (8) | 5 (0) |
| 0x03 | `SGWPlayer` | SGWBeing → SGWSpawnableEntity → SGWEntity | 244 | 157 | 315 (109) | 82 (30) |
| 0x04 | `SGWGmPlayer` | SGWPlayer → SGWBeing → SGWSpawnableEntity → SGWEntity | 246 | 163 | 433 (226) | 90 (30) |
| 0x05 | `SGWMob` | SGWBeing → SGWSpawnableEntity → SGWEntity | 198 | 29 | 132 (8) | 5 (0) |
| 0x06 | `SGWPet` | SGWMob → SGWBeing → SGWSpawnableEntity → SGWEntity | 210 | 32 | 140 (8) | 5 (0) |
| 0x07 | `SGWDuelMarker` | SGWSpawnableEntity → SGWEntity | 24 | 12 | 44 (0) | 2 (0) |
| 0x08 | `SGWBlackMarket` `[SO]` | (root) | 1 | 0 | 0 (0) | 6 (0) |
| 0x09 | `Account` | GamePawn (UE3 — not a BW def) | 2 | 6 | 0 (0) | 10 (8) |
| 0x0A | `SGWEntity` `[SO]` | (root) | 10 | 0 | 32 (0) | 2 (0) |
| 0x0B | `SGWPlayerGroupAuthority` `[SO]` | SGWEntity | 13 | 0 | 32 (0) | 6 (0) |
| 0x0C | `SGWSpaceCreator` `[SO]` | SGWEntity | 19 | 0 | 35 (0) | 22 (0) |
| 0x0D | `SGWSpawnRegion` `[SO]` | SGWEntity | 37 | 0 | 32 (0) | 17 (0) |
| 0x0E | `SGWSpawnSet` `[SO]` | SGWEntity | 36 | 0 | 32 (0) | 12 (0) |
| 0x0F | `SGWPlayerRespawner` `[SO]` | SGWEntity | 14 | 0 | 33 (0) | 2 (0) |
| 0x10 | `SGWCoverSet` `[SO]` | SGWEntity | 16 | 0 | 34 (0) | 2 (0) |
| 0x11 | `SGWEscrow` `[SO]` | SGWEntity | 12 | 0 | 34 (0) | 2 (0) |
| 0x12 | `SGWChannelManager` `[SO]` | (root) | 0 | 0 | 0 (0) | 15 (0) |

**Reading the table.** `Props` is the count of properties contributed to the all-properties array at `EntityDescription+0x5c/+0x60` after the parse cascade. `ClientMethods` is the size of the client-method ordinal table — this is the number that plugs into §1.4's `idBase` formula for that entity. `CellMethods (Exposed)` and `BaseMethods (Exposed)` give total / exposed pairs; the exposed count is what determines the wire methodID space (cell `0x80..0xBF`, base `0xC0..0xFF`). Most server-only entities have zero exposed methods — they don't communicate with clients at all.

`SGWGmPlayer` is notable for inheriting from `SGWPlayer` (not from `SGWBeing` directly) and adding 6 own ClientMethods → 163 total. Its `idBase = 0x3E - (163 + 0xC0) / 0xFF = 0x3E - 1 = 61` is the same as SGWPlayer's because `163` and `157` both fall in the same `iVar2 = 1` bucket. `SGWPet` (32 client methods) and `SGWMob` (29 client methods) both have `idBase = 62` — they sit just below the sub-slot threshold; no two-byte encoding required.

`Account` is a thin login-flow entity with no `SGWBeing` ancestry — it has a UE3 `GamePawn` parent listed in its `<Parent>` tag, which is not a BW def file and is ignored by the entity-description parser. Its 6 ClientMethods + 8 exposed BaseMethods (login, character-select, character-creation flow) constitute the only wire-visible methods until a player entity exists.

---

### 2 Appendix C — Interface dependency inventory

The 19 interface `.def` files in `defs/interfaces/`. Each row is one interface; `Implements` lists the other interfaces (if any) this interface declares via its own `<Implements>` section. **Graph shape: FLAT** — no SGW interface implements another interface. BigWorld supports interface-of-interface chaining at the parser level (per §1.3), but none of the 19 SGW interface files declares an `<Implements>` block with any interface inside, so the dependency graph is a single layer.

Practical consequence: an entity that declares `<Implements><Interface>Foo</Interface></Implements>` gets exactly `Foo`'s properties and methods — no recursive chain to walk. The 11 interfaces SGWPlayer declares (§2.7) are independent contributors to its ID tables.

| Interface | Implements | Props | ClientMethods | CellMethods (Exposed) | BaseMethods (Exposed) |
|-----------|-----------:|------:|--------------:|----------------------:|----------------------:|
| `ClientCache` | (none) | 0 | 2 | 0 (0) | 2 (2) |
| `Communicator` | (none) | 4 | 7 | 1 (0) | 21 (15) |
| `ContactListManager` | (none) | 1 | 5 | 6 (6) | 2 (0) |
| `DistributionGroupMember` | (none) | 3 | 0 | 6 (0) | 0 (0) |
| `EventParticipant` | (none) | 2 | 0 | 1 (0) | 0 (0) |
| `GateTravel` | (none) | 5 | 4 | 5 (1) | 2 (0) |
| `GroupAuthority` | (none) | 3 | 0 | 0 (0) | 4 (0) |
| `Lootable` | (none) | 4 | 0 | 5 (0) | 0 (0) |
| `MinigamePlayer` | (none) | 31 | 13 | 35 (15) | 19 (1) |
| `Missionary` | (none) | 8 | 5 | 17 (3) | 0 (0) |
| `OrganizationMember` | (none) | 7 | 18 | 37 (12) | 4 (4) |
| `SGWAbilityManager` | (none) | 30 | 0 | 16 (3) | 0 (0) |
| `SGWBeing` (interface) | (none) | 23 | 8 | 16 (2) | 3 (0) |
| `SGWBlackMarketManager` | (none) | 1 | 6 | 6 (6) | 6 (0) |
| `SGWCombatant` | (none) | 24 | 6 | 14 (3) | 0 (0) |
| `SGWInventoryManager` | (none) | 14 | 7 | 13 (7) | 0 (0) |
| `SGWMailManager` | (none) | 4 | 4 | 10 (9) | 1 (0) |
| `SGWPoller` | (none) | 2 | 0 | 0 (0) | 0 (0) |

Three observations land directly on the wire:

- **`SGWPoller` is a pure aggregator** — 2 properties, no methods of any kind. It exists only to thread `interactFlag` (propID 172) and `lastPollTime` (propID 173) into entities that need polling. Its presence in an `<Implements>` list contributes propIDs but nothing methodID-bearing.
- **`Lootable` and `GroupAuthority` have zero exposed methods** — they are server-side schema contributions only. An entity that `<Implements>` either gets cell-internal helpers but no wire surface.
- **The "Communicator declares 15 Exposed cell methods" claim in earlier drafts is wrong** — the 15 Exposed annotations in `Communicator.def` are on **BaseMethods**, not CellMethods. Communicator's one CellMethod is unexposed; its 21 BaseMethods include 15 Exposed. Server implementers should consult `defs/interfaces/Communicator.def` directly rather than rely on the count breakdown in §2.4 (which is otherwise accurate for the patterns it describes).

There is exactly one SGW interface that contributes to nothing in any SGWPlayer-cascade chain: `Lootable` (zero appearances in `<Implements>` blocks of any of the 18 entity defs). `GroupAuthority` likewise has zero entity-side consumers in the current tree — both are dead interfaces in this build but kept in the schema for forward compatibility.

---

## Section 3 — Deprecated server

§3 (Deprecated Server), §4 (Expected Implementation in Rust), and §5 (Actual Implementation in Rust) are pending §1+§2 sign-off and will land in follow-up PRs after the equivalent entity-property-sync conformance audit (mirroring how [`mercury-rust-conformance-2026-05-15.md`](../../audits/mercury-rust-conformance-2026-05-15.md) came after `spec.protocol.mercury-wire-format`).

N/A — pending §1+§2 sign-off. `deprecated/cpp/src/baseapp/entity/` and `deprecated/python/base/*.py` carry the legacy server's emit + property-cache logic; section 3 will reconstruct it after the protocol invariants in §1+§2 are stable.

---

## Section 4 — Expected implementation in Rust

N/A — pending §1+§2 sign-off. Will name the Rust symbols that must encode each wire pipe on the server side, using the no-line-numbers rule (`cimmeria-services::base::world_entry::create_base_player::serialize`, `cimmeria-services::mercury::aoi::introduction::send_create_then_deltas`, etc.).

---

## Section 5 — Actual implementation in Rust

N/A — pending §1+§2 sign-off. The audit at [`docs/audits/mercury-rust-conformance-2026-05-15.md`](../../audits/mercury-rust-conformance-2026-05-15.md) is the gap-analysis seed; the §5 authoring pass will pull from it once §1+§2 are signed off and the equivalent entity-property-sync audit has run. The conformance audit will be its own PR (mirroring how `mercury-rust-conformance-2026-05-15.md` landed separately from `spec.protocol.mercury-wire-format`).
