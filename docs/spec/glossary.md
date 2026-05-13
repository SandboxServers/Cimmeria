---
title: Bible Glossary
chapter_id: spec.meta.glossary
status: verified
last_verified: 2026-05-13
verified_by: documentation-writer
audience: anyone reading a bible chapter
type: reference
---

# Bible Glossary

Bible vocabulary. Every term used in a canonical chapter is defined here, with a cross-reference to the chapter where the term is load-bearing.

This is the **bible-specific** glossary. Project-wide vocabulary (build system, test types, agent personas, CI workflows) lives in [`../glossary.md`](../glossary.md) — the two glossaries cross-reference; terms defined in one are linked in the other.

If you are reading a chapter and hit a term not defined here, that is a bug. Either the term belongs in this glossary (add it in your next PR) or the chapter is using non-bible vocabulary (replace with a defined term).

Chapter cross-references use `chapter_id`. `→ N/A (no chapter yet)` means the term is bible-level vocabulary that does not yet have a single canonical chapter — likely because the chapter has not been authored.

---

## Engine and runtime

**Cell** — The simulation-side of an entity's split-brain representation in BigWorld. Holds position, AoI, physics, combat state. Runs on the cell process. Authority over space-bound behavior. Counterpart: `Base`. → `spec.engine.cme-event-signal`, `spec.world.world-entry`.

**Base** — The persistence-and-RPC-routing side of an entity's split-brain representation. Holds inventory, missions, anything space-independent. Runs on the base process. Authority over persistent state. Counterpart: `Cell`. → `spec.world.world-entry`.

**CellApp** — The process hosting cell entities. In Cimmeria the cell and base apps run in-process; in the original BigWorld topology they were separate processes. → N/A (no chapter yet).

**BaseApp** — The process hosting base entities. Owns the Mercury client connection. → `spec.protocol.mercury-wire-format`.

**AoI** (Area of Interest) — The radius around an entity within which it receives updates about nearby entities. Drives `ghost entity` creation/destruction and witness lists. See also: `witness`, `ghost entity`. → `spec.world.world-entry`.

**ghost entity** — A read-only client-side or cell-side mirror of an entity that lives in a different cell or is outside the local AoI. Created when AoI overlap begins, destroyed when AoI overlap ends. The countdown counter at AoI entry is *decremented*, not incremented — a finding from V5 W4-C that contradicts older docs. → `spec.engine.cme-event-signal`.

**witness** — An entity (typically a player) that is observing another entity for AoI purposes. The witness list on an entity is the inverse of that entity's AoI. → `spec.world.world-entry`.

**propID** — The 1- or 2-byte property ID on the wire that identifies which entity property is being updated. Properties 0–59 use a 1-byte ID; properties 60+ use a 2-byte ID. SGW's ammo type lives at `propID = 3`. See also: `methodID`. → `spec.protocol.entity-property-sync`.

**methodID** — The 1-byte method ID on the wire that identifies which entity method is being invoked. SGW's `moveItem` is `methodID = 38`; `onEntityProperty` is `methodID = 7`. Resolved via `EntityDescription_FindMethodIdByName` at `ghidra://SGW.exe@0x0158e710`. See also: `universal RPC dispatcher`. → `spec.protocol.message-catalog`.

**NetOut** — Client-to-server message. The client emits `Event_NetOut_*` signals; the universal RPC dispatcher converts them to Mercury entity-method calls. 253 NetOut messages catalogued. → `spec.protocol.message-catalog`.

**NetIn** — Server-to-client message. 167 NetIn messages catalogued. → `spec.protocol.message-catalog`.

**Mercury bundle** — A reliable-UDP payload unit in the Mercury protocol. Bundles carry one or more interface elements (entity-method calls, property updates, control messages). See also: `Mercury channel`, `nub`, `InterfaceElement length encoding`. → `spec.protocol.mercury-wire-format`.

**Mercury channel** — A reliable, ordered communication channel between two Mercury endpoints. Tracks sequence numbers, retransmits, and the AES/HMAC keys. → `spec.protocol.mercury-wire-format`.

**nub** — The Mercury endpoint. Each process has exactly one nub. The 24 init steps for `Mercury::Nub::Nub` are at `ghidra://SGW.exe@0x015841d0`. → `spec.protocol.mercury-wire-format`.

**ENABLE_ENTITIES** — The SGW-custom 8-byte Mercury control message that switches the channel from auth-mode to entity-mode, after which entity property/method traffic is allowed. Not part of stock BigWorld. → `spec.world.world-entry`.

**EntityDescription** — The per-entity-type metadata block that lists the entity's properties and methods with their wire-format IDs and types. Parsed in fixed order: Implements → Properties → Methods. → `spec.engine.entity-description-parse-chain`.

**EntityType** — One of the 18 entity types in SGW (Player, NPC, Vehicle, etc.). Each has a unique `typeID` and an `EntityDescription`. → `spec.engine.entity-description-parse-chain`.

**EntityID** — The 4-byte runtime identifier for an entity instance, assigned by the BaseApp on creation. → N/A (no chapter yet).

**MD5 type hash** — The 16-byte hash the client uses to verify an `EntityDescription` matches the one the server is offering. Computed over the property and method declarations in their declared order. A mismatch aborts world entry. → `spec.engine.entity-description-parse-chain`.

**DataType registry** — The two-registry model SGW uses for wire-format types: one registry for primitive scalars, one for user-defined composites (FIXED_DICT, ARRAY, etc.). The sub-slot encoding threshold is 62. → `spec.engine.entity-description-parse-chain`.

**CME** (`CmeEventSignal`) — The Cheyenne Mountain Entertainment event-signal subsystem that fronts every `Event_NetOut_*` and `Event_NetIn_*` on the client side. Two emit patterns: Pattern A (most events) and Pattern B (a handful, including `callForAid`, `SetRingTransporterDestination`, `SlashCmd_EmitSetRingTransporterDestination`). See also: `Pattern A emit`, `Pattern B emit`. → `spec.engine.cme-event-signal`.

**Pattern A emit** — The default CME emit shape: get-system → lookup-by-name → set-fields → vtable-dispatch. Used by most of the 420 `Event_*` messages. → `spec.engine.cme-event-signal`.

**Pattern B emit** — The alternate CME emit shape used by ~3 emitters in the lower binary half (`0x00400000–0x00b00000`) and one dense cluster at `0x00573d70–0x005aXXXX`. Same wire effect as Pattern A. → `spec.engine.cme-event-signal`.

**vfunc_3** — The RTTI accessor on `_MemberCallback__vfunc_3` template instantiations. Returns a `TypeDescriptor*` (not a name string). Names look like `MemberCallbackRtti_<Event>__<Subscriber>` after the W-rename pass. 1,176 functions in this family. See also: `vfunc_5`, `MemberCallback`. → `spec.engine.cme-event-signal`.

**vfunc_5** — `CmeEventSignal_InvokeMemberCallback` at `ghidra://SGW.exe@0x00e04570`. The actual handler dispatch — shared across 10 vtables. The bound method pointer lives at `this+0x8` on the callback object. See also: `vfunc_3`. → `spec.engine.cme-event-signal`.

**MemberCallback** — The CME template type that binds a member function to an event subscription. One instantiation per `(event, subscriber-class)` pair. The handler body is at `this+0x8`. → `spec.engine.cme-event-signal`.

**universal RPC dispatcher** — The single function at `ghidra://SGW.exe@0x00c6fc40` through which every NetOut entity-method call routes. Looks up the method ID via `EntityDescription_FindMethodIdByName` and serializes arguments into a Mercury bundle. → `spec.engine.universal-rpc-dispatcher`.

**cooked data** — The PAK-packaged content artifacts the client loads at runtime: items, abilities, effects, missions, animations, etc. 21 categories (not 22 — `behavior_event` shifts the count depending on which doc you read; see bug [#267](https://github.com/SandboxServers/Cimmeria/issues/267)). Subscribed in a 5-event pattern per `LibCategory`. → `spec.engine.cooked-data-pipeline`.

**PAK** — A ZIP-format archive containing cooked data. Versioned via `MetaData` keys that persist across PAK swaps. Cimmeria's mission overrides patch the canonical PAKs by injecting per-key invalidations. → `spec.engine.cooked-data-pipeline`.

**LibCategory** — One of the 21 cooked-data categories. Each registers via `CookedData_RegisterAllLibCategories` at `ghidra://SGW.exe@0x00420074` with a 5-event subscription pattern. → `spec.engine.cooked-data-pipeline`.

---

## Protocol

**AES-256-CBC + HMAC-MD5** — The SGW Mercury cipher chain. CryptoPP, zero IV, no KDF — the key is used directly. HMAC-MD5 authenticates each packet. → `spec.protocol.mercury-wire-format`.

**zero IV** — The literal-zero initialization vector used for AES-256-CBC in Mercury. A deliberate choice in 2009; a footgun by modern standards but matched for wire compatibility. → `spec.protocol.mercury-wire-format`.

**MachineGuard** — The SGW machine-level UDP protocol on port `0x4e36` (19510). 13 message types, master deserializer at `ghidra://SGW.exe@0x01588530`. Used for service discovery and cross-host coordination. → N/A (no chapter yet).

**RESOURCE_FRAGMENT** — A Mercury control message type that carries one fragment of a cooked-data resource being streamed from server to client. Reassembled before passing to the cooked-data pipeline. → N/A (no chapter yet).

**packet flags byte** — The first byte of a Mercury packet header, encoding reliability/ordering/control bits. See also: `footer byte order`. → `spec.protocol.mercury-wire-format`.

**footer byte order** — Mercury packet footers contain ACK/NACK ranges and the HMAC. Byte order matters for the HMAC validation; getting it wrong produces silent packet drops. → `spec.protocol.mercury-wire-format`.

**InterfaceElement length encoding** — Mercury's variable-length prefix scheme for interface elements: 1-byte for lengths < 248, 2-byte / 3-byte / 4-byte switch above. The switch points are not contiguous — see the finding doc. → `spec.protocol.mercury-wire-format`.

**WSTRING** — The 16-bit-character wide-string wire-format type. Length-prefixed in code units, not bytes. → `spec.protocol.entity-property-sync`.

---

## State and lifecycle

**BSF_\*** — The "Being State Flags" bitfield on a `GameBeing`. One byte at `entity+0x158`. The OnStateFieldUpdate dispatch on the client only handles bits 0–7; bit 8 (`BSF_HOLSTER`) is *not* in the dispatch table — holster state is set via a separate path through the posture byte. → `spec.player.state-fields`.

**BSF_InCombat** — Combat-state bit in the BSF flags. Set on first damage, cleared after the leash-distance timer expires with no aggro. → `spec.combat.combat-lifecycle`.

**BSF_Dead** — Death bit at `entity+0x158 bit 0`. Death is an in-place flag flip; the corpse is *not* a separate entity. → `spec.player.death-respawn`.

**BSF_Holster** — The holster bit. Not in the OnStateFieldUpdate bit-0-through-7 dispatch; written via the posture byte path (`entity+0x3D2`). See also: `posture byte`. → `spec.player.state-fields`.

**posture byte** — The byte at `entity+0x3D2`. Sole writer is `CompositedAppearanceProxy::ApplyToPawn` at `ghidra://SGW.exe@0x00ec0840`. Encodes posture/stance/holster state distinct from the BSF flag byte. → `spec.player.animations`, `spec.player.state-fields`.

**Faction** (`EFaction` enum) — One of 34 factions, but only ordinal 10 matters for client combat. Binary gate; no matrix. Stored at `entity+0x134`. → `spec.player.faction-alignment`.

**Alignment** — Fixed at character creation. Stored at `entity+0x135`. → `spec.player.faction-alignment`.

**QR** (Quality Rating) — The 20-entry result code table for ability/effect outcomes. Codes 0–14 are canonical; the remainder are designer channels. → `spec.combat.damage-pipeline`.

**TCM** (target/cast mode) — The enum encoding ability targeting modality (self, target, cone, AoE, etc.). Drives the timer-type-14 dispatch in the ability resolution pipeline. → `spec.combat.ability-resolution`.

**SecondaryId** — The key under which effects refresh-stack on a target. Two effects with the same `SecondaryId` on the same target replace each other; different `SecondaryId`s stack. → `spec.combat.effects-execution`.

**SourceID** — The ability-source identifier used by the CooldownManager to gate which cooldowns prevent which casts. → `spec.combat.ability-resolution`.

**threat list** — The per-NPC ordered list of (entity, threat-value) pairs that drives target selection. The threat formula is server-only; the client never sees it. → `spec.combat.wire-formats`.

**leash distance** — The maximum distance from an NPC's spawn point at which it will continue combat. Crossing the leash clears the threat list and resets combat state. → `spec.npcs.movement-and-pathfinding`.

---

## Inventory and combat

**moveItem** (method 38) — The single entity method that all inventory transitions route through: equip, unequip, container-to-container moves, bandolier shuffles. Container IDs 4–14 are equipment slots; container ID 3 is the bandolier. There is *no* separate equip wire. → `spec.inventory.containers-and-equip`.

**bandolier** — Container ID 3. 1-indexed on the wire (a known SGW quirk). 14 subscription points on the `Inventory` class touch the bandolier. → `spec.inventory.containers-and-equip`.

**container ID** — The 1-byte identifier of an inventory container in a `moveItem` call. 3 = bandolier; 4–14 = equipment slots. → `spec.inventory.containers-and-equip`.

**ammo type ID** (propID 3) — The propID under which a weapon's currently-loaded ammo type is synced. Distinct from `methodID = 7` (`onEntityProperty`) — the two were conflated in pre-V5 docs, fixed in PR #168. → `spec.combat.weapons-and-ammo`.

**reload sequence** — The fixed 7-message emit order for a weapon reload. → `spec.combat.weapons-and-ammo`.

**ConfirmEffect** — The wire message used to confirm a channeled ability is still active. Channeled-cancel uses this same message with a cancel flag — not a separate cancel wire. → `spec.combat.ability-resolution`.

**OnEffectResults** — The wire message carrying per-effect outcome data (QR result code, damage, status changes) back to the client after an ability resolves. → `spec.combat.damage-pipeline`.

---

## Cross-glossary references

Terms defined in the project-wide [`../glossary.md`](../glossary.md) that the bible cites:

- **live-DB test** — A Cimmeria-specific test type, defined in the project glossary. The bible cites it in chapter prose but the canonical definition is project-wide. See `../glossary.md`.
- **Diátaxis** — The documentation classification (tutorial / how-to / reference / explanation). The bible *is* reference; the bible's how-to-{read,write} docs are how-to. See `../glossary.md`.
- **chain-replay test** — Test type for content engine. See `../glossary.md`.

When adding to either glossary, cross-link if the term has weight in both worlds. Avoid double-defining; one canonical definition, references everywhere else.
