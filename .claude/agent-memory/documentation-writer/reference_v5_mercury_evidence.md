---
name: reference-v5-mercury-evidence
description: Map of which V5 finding docs canonize which slices of Mercury wire format. Cite the densest doc for each topic.
metadata:
  type: reference
---

The seven V5 finding docs that together canonize the Mercury wire format. For each, the topics it owns:

- `docs/reverse-engineering/findings/mercury-protocol-internals.md` — packet flags byte map, footer parse order, cipher chain (AES-256-CBC + HMAC-MD5, zero IV, no KDF), Nub construction, MachineGuard, sequence-number constants, InterfaceElement function addresses. The canonical entry point.
- `docs/reverse-engineering/findings/world-entry-pipeline.md` — Phase-by-phase sequence; the doc that originally surfaced the ENABLE_ENTITIES 8-byte payload reconciliation. Its sub-slot threshold arithmetic (`117 - 61`) is **inconsistent** with the W-entity-desc-B finding — prefer entity-property-sync §13 for the threshold.
- `docs/reverse-engineering/findings/entity-creation-wire-formats.md` — byte-level layouts for RESET_ENTITIES, CREATE_BASE_PLAYER, CREATE_CELL_PLAYER, SPACE_VIEWPORT_INFO, FORCED_POSITION, CREATE_ENTITY, ENTITY_INVISIBLE, LEAVE_AOI, TICK_SYNC. Includes the two distinct forcedPosition C++ call sites (world-entry vs standalone).
- `docs/reverse-engineering/findings/system-protocol-wire-formats.md` — Ghidra decompilation evidence for system messages. Owns the `startEntityMessage` vs `startProxyMessage` distinction (cell writes entityID, base does not). Owns the AUTHENTICATE 0x00 DWORD_LENGTH evidence.
- `docs/reverse-engineering/findings/position-movement-wire-formats.md` — 32 avatarUpdate variants, detailedPosition, forcedPosition. The doc that names the trailing byte at forcedPosition offset 48 as `physics` (movement mode), not "flags".
- `docs/reverse-engineering/findings/space-viewport-wire-formats.md` — complete server-to-client message catalogue 0x00–0xFF. Owns the RESOURCE_FRAGMENT (0x36) byte-level layout, fragment-flags bitfield, 21 resource category IDs, FragmentSize=1000, and `REPLY_MESSAGE (0xFF) = WORD_LENGTH`.
- `docs/reverse-engineering/findings/entity-property-sync.md` §13 — sub-slot client method encoding. Threshold is `0x3e = 62` per `EntityDescription_AssignClientMethodIds` at `0x01590df0`. Matches BigWorld 2.0.1's `checkExposedForSubSlots()` exactly. For method 117 the sub_index is `117 - 62 = 55`, not 56.

**When two V5 docs disagree:** prefer the one with a Ghidra anchor over the one with prose summary. `entity-property-sync.md` §13 cites `0x01590df0` directly; `world-entry-pipeline.md`'s threshold-of-61 arithmetic appears to be a derived claim with no anchor of its own.

**When V5 names a different abstraction layer than the chapter:** server-source C++ writes (`(u8)classId << (u8)propCount`) and client Ghidra reads (`MOVZX EAX, word ptr [EAX]` = u16) can both be true. Pin the wire format to what the client reads — that is the actual contract. Document the server-source layer as an aside, not as a competing interpretation.

Related: [[feedback-section1-evidence]].
