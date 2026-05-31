# Evidence Standards for Reverse Engineering Findings

This document defines the confidence tiers, citation formats, and documentation templates used across all Cimmeria reverse engineering work. Every finding committed to the project must follow these standards so that future contributors can evaluate the reliability of claims and trace them back to primary sources.

---

## Confidence Tiers

Every finding is assigned one of three confidence levels. The tier determines how much trust implementers should place in the finding when writing server code.

### HIGH Confidence

**Definition:** The finding is supported by decompiled code from SGW.exe AND corroborated by at least two independent sources.

A finding reaches HIGH confidence when you can point to a specific decompiled function in Ghidra and then confirm the behavior through additional evidence such as entity .def files, XSD schemas, BigWorld reference source, Python scripts, live network captures, or enumerations.xml constants.

**What qualifies as corroboration:**
- Ghidra decompilation shows a function serializing three INT32 fields, AND the corresponding .def method declares three INT32 arguments, AND BigWorld reference source shows the same pattern in the equivalent function.
- Ghidra decompilation shows a specific message ID, AND a .def file declares the matching method, AND live testing confirms the client sends/receives the message in the expected format.

**When to use:** Implementing core protocol handling, combat damage formulas, entity property synchronization, or any path where incorrect behavior causes client disconnects or data corruption.

**Example scenario:** You decompile `Event_NetOut_UseAbility` in Ghidra and see it serializes (abilityID: INT32, targetID: INT32, location: VECTOR3). The SGWAbilityManager.def file declares `useAbility` with matching argument types. BigWorld 2.0.1 `connection_model.cpp` shows the same serialization order for exposed method calls. This is HIGH confidence.

---

### MEDIUM Confidence

**Definition:** The finding is supported by a single source only, without independent corroboration.

A MEDIUM finding comes from one piece of evidence: a .def file alone, a BigWorld analogy alone, a string reference in Ghidra without full decompilation, or a single Python script's behavior. The information is likely correct but has not been verified from a second angle.

**What qualifies:**
- A .def file declares a method with specific argument types, but you have not decompiled the corresponding client handler to confirm the wire format matches.
- BigWorld reference source shows how a subsystem works, but you have not confirmed SGW follows the same implementation (it may have been customized by CME/Cheyenne Mountain).
- A string reference like `"Event_NetOut_UseAbility"` is found near a function in Ghidra, giving you the function's purpose, but you have not fully decompiled it to confirm argument layout.
- A Python script calls `self.invokeAbility(id, target, 0, None, {})`, telling you the argument order, but the actual wire serialization might differ.

**When to use:** Planning implementation work, writing stubs, understanding system architecture. MEDIUM findings should be verified before being used for critical protocol-level code.

---

### LOW Confidence

**Definition:** The finding is inferred from patterns, incomplete decompilation, external community reports, or structural reasoning rather than direct evidence.

LOW confidence findings are hypotheses. They provide direction for further investigation but should not be directly implemented without verification.

**What qualifies:**
- A function's purpose is guessed from its call graph neighbors (it is called by a known combat function, so it is probably combat-related).
- Decompilation is partial or garbled and you are interpreting fragments.
- Information comes from community wikis, forum posts, or player observations about the live game's behavior (useful but not authoritative about the protocol).
- A pattern observed in one system is assumed to apply to another system by analogy.
- Vtable slot assignment is inferred from index position without RTTI confirmation.

**When to use:** Guiding Ghidra exploration, forming hypotheses to test, understanding the general shape of unanalyzed systems.

---

## Citation Formats

Every source reference in a finding must use one of the following citation formats. Citations appear in square brackets and should be specific enough that another researcher can navigate directly to the evidence.

### Ghidra Addresses

Format: `[SGW <address>]`

The address is the virtual address in SGW.exe as loaded in Ghidra. Use the full hex address including the image base.

```
[SGW 0x14001a2b0]          -- a specific function entry point
[SGW 0x14001a2b0+0x42]     -- a specific instruction within a function
[SGW vtable@0x141bc5e00]   -- a vtable address
[SGW string@0x141a03f20]   -- a string reference address
```

When citing a decompiled function, include the function name if it has been annotated:

```
[SGW 0x14001a2b0 CME::EntityManager::createEntity]
```

### Entity Definition Files

Format: `[DEF <filename>:<element>]`

Reference entity .def files and their specific properties or methods.

```
[DEF SGWPlayer.def:playerName]                      -- a property
[DEF SGWPlayer.def:useAbility]                       -- a method (search CellMethods/BaseMethods/ClientMethods)
[DEF interfaces/SGWCombatant.def:onAttacked]         -- an interface method
[DEF interfaces/SGWCombatant.def:statsBaseCurrent]   -- an interface property
[DEF alias.xml:StatList]                             -- a custom type alias
[DEF enumerations.xml:EAbilityFlags]                 -- an enumeration
[DEF entities.xml:SGWMob]                            -- entity type registration
```

### BigWorld Reference Source

Format: `[BW <filepath>:<line>]`

Reference files from the BigWorld Engine reference source (1.9.1 or 2.0.1). Paths are relative to the BigWorld source root.

```
[BW connection_model.cpp:342]                            -- specific line
[BW src/lib/connection/server_connection.cpp:891]        -- full relative path
[BW src/lib/entitydef/entity_description.cpp:215-240]   -- line range
[BW src/server/cellapp/entity.cpp:onMethodCall]          -- function name instead of line
```

When the 1.9.1 and 2.0.1 versions differ, specify the version:

```
[BW 2.0.1 src/lib/connection/login_handler.cpp:156]
[BW 1.9.1 src/lib/network/mercury.cpp:423]
```

### Python Scripts

Format: `[PY <filepath>:<line>]`

Reference Python scripts from the Cimmeria codebase. Paths are relative to the `python/` directory.

```
[PY cell/AbilityManager.py:156]              -- specific line in a cell script
[PY cell/SGWMob.py:onAttacked]               -- function name
[PY base/Account.py:42]                      -- base script
[PY common/defs/Ability.py:DamageCalc]       -- common module, class name
[PY common/Config.py:getConfig]              -- common utility
```

### XSD Schemas

Format: `[XSD <filename>:<element>]`

Reference XSD schemas from the client's cooked data definitions.

```
[XSD Ability.xsd:AbilityDef]                 -- schema root element
[XSD Effect.xsd:EffectDef/DamageType]        -- nested element
[XSD CookedData/Item.xsd:ItemDef]            -- subdirectory schema
```

### Live Testing

Format: `[TEST: <description>]`

Reference observations from live testing with the client connected to the Cimmeria server. Include enough detail to reproduce.

```
[TEST: client sends UseAbility(id=1001, target=0x1A3, loc=(0,0,0)) on button press]
[TEST: server reply with onEffectResults causes client animation to play]
[TEST: sending malformed onStatUpdate with wrong stat count causes client disconnect]
[TEST: client expects TimerUpdate within 100ms of UseAbility or shows "ability failed" UI]
```

### Enumerations

Format: `[ENUM <enumeration>:<token>]`

Reference specific enumeration values from `enumerations.xml`.

```
[ENUM EAbilityFlags:WeaponBar]               -- value=1
[ENUM EGender:GENDER_Male]                   -- value=1
[ENUM ETargetType:TargetTarget]              -- value=2
[ENUM EWeaponState:...]                      -- general reference
```

### Community Sources

Format: `[COMMUNITY: <source>]`

Reference information from community wikis, forums, or other external sources. These always carry LOW confidence on their own.

```
[COMMUNITY: SGW wiki "Combat" page, archived 2009]
[COMMUNITY: MMO-Champion forum post by user X, 2008-11-15]
[COMMUNITY: YouTube gameplay video showing gate travel UI, timestamp 4:32]
```

---

## Wire Format Documentation Template

When a finding involves network message serialization, document the wire format using a table with the following columns. All offsets are relative to the start of the message payload (after any Mercury framing).

```
| Offset | Size | Type    | Field Name    | Description                                  |
|--------|------|---------|---------------|----------------------------------------------|
| 0x00   | 4    | INT32   | abilityId     | ID of the ability being used                 |
| 0x04   | 4    | INT32   | targetId      | Entity ID of the target (0 = no target)      |
| 0x08   | 4    | FLOAT32 | locationX     | Ground target X coordinate                   |
| 0x0C   | 4    | FLOAT32 | locationY     | Ground target Y coordinate                   |
| 0x10   | 4    | FLOAT32 | locationZ     | Ground target Z coordinate                   |
```

**Conventions:**
- Offsets in hex, sizes in decimal bytes.
- Type column uses the BigWorld type names (INT8, INT16, INT32, UINT8, UINT16, UINT32, FLOAT32, FLOAT64, STRING, WSTRING, ARRAY, FIXED_DICT, PYTHON, MAILBOX, VECTOR3).
- For variable-length fields (STRING, WSTRING, ARRAY), note the length prefix format:
  - STRING: 1-byte length prefix if length < 256, otherwise 1-byte 0xFF marker followed by 4-byte length.
  - WSTRING: same length prefix scheme, but length counts UTF-16 code units, and payload is UTF-16LE encoded.
  - ARRAY: 4-byte element count prefix, followed by that many serialized elements.
- For FIXED_DICT types, expand each field inline or reference the alias.xml definition.
- For VECTOR3: 12 bytes, three consecutive FLOAT32 values (x, y, z).

---

## Finding Documentation Template

Every finding should be documented as a standalone section or file using the following structure.

### Title

A clear, specific title describing what the finding covers. Avoid vague titles like "combat stuff" -- use titles like "UseAbility Client-to-Server Message Format" or "Stat Regeneration Tick Rate and Formula".

### Metadata

```
Confidence: HIGH | MEDIUM | LOW
Last verified: YYYY-MM-DD
Sources:
  - [SGW 0x14001a2b0] -- UseAbility handler entry point
  - [DEF interfaces/SGWAbilityManager.def:useAbility]
  - [BW src/lib/connection/server_connection.cpp:891]
  - [TEST: client sends UseAbility on hotbar press]
Related findings: <links to other findings that depend on or inform this one>
Implementation status: Not started | Stub | Partial | Complete
```

### Description

A prose description of the finding. Explain what was discovered, how it works, and why it matters for the server implementation. Include any caveats, uncertainties, or areas that need further investigation.

### Wire Format (if applicable)

Use the wire format table template from above.

### Decompiled Code (if applicable)

Include relevant excerpts from Ghidra decompilation. Use C-style formatting. Annotate unclear variables with comments explaining your interpretation.

```c
// [SGW 0x14001a2b0] -- UseAbility handler
void __thiscall SGWAbilityManager::useAbility(
    SGWAbilityManager *this,
    int abilityId,       // maps to ability database ID
    int targetEntityId,  // 0 means no target
    Vector3 *groundPos   // only used for ground-targeted abilities
) {
    // ... decompiled body ...
}
```

### Implementation Impact

Describe what this finding means for the Cimmeria server implementation:
- Which C++ or Python files need changes?
- What message handlers need to be written or modified?
- Are there dependencies on other findings?
- What could go wrong if this is implemented incorrectly?

---

## Complete Finding Example

Below is a full example of a documented finding following these standards.

---

### UseAbility Client-to-Server Message Format

```
Confidence: HIGH
Last verified: 2026-03-01
Sources:
  - [SGW 0x14001a2b0] -- Event_NetOut_UseAbility handler
  - [DEF interfaces/SGWAbilityManager.def:useAbility]
  - [BW 2.0.1 src/lib/entitydef/method_description.cpp:callMethod]
  - [PY cell/AbilityManager.py:useAbility]
  - [TEST: client sends message on hotbar button press, server logs show correct parse]
Related findings: onEffectResults wire format, TimerUpdate wire format
Implementation status: Complete
```

**Description:**

When a player activates an ability via the hotbar, the client serializes a `useAbility` exposed cell method call. The message is sent as a Mercury reliable message to the CellApp hosting the player's entity. The CellApp deserializes the arguments and dispatches them to the Python `useAbility` method on the player's `SGWAbilityManager` interface script.

The method is declared in `SGWAbilityManager.def` as an `<Exposed/>` cell method with five arguments. The `<Exposed/>` tag means the client is permitted to call this method directly. Non-exposed methods cannot be invoked by the client.

The Ghidra decompilation at `0x14001a2b0` confirms the client serializes exactly these five fields in order. The BigWorld reference source confirms that exposed method arguments are serialized in declaration order with no additional framing beyond the method index.

**Wire format:**

| Offset | Size | Type    | Field Name               | Description                                        |
|--------|------|---------|--------------------------|----------------------------------------------------|
| 0x00   | 4    | INT32   | aAbilityId               | Database ID of the ability to invoke               |
| 0x04   | 4    | INT32   | aTargetId                | Entity ID of the target, 0 if self/ground          |
| 0x08   | 4    | INT32   | originatingSubsystemID   | Subsystem that triggered the ability (0 = player)  |
| 0x0C   | var  | PYTHON  | aLocation                | Pickled tuple (x,y,z) for ground target, or None   |
| var    | var  | PYTHON  | userData                 | Pickled dict of additional parameters, or {}       |

Note: The PYTHON-typed arguments are serialized using Python's pickle protocol. The size is variable -- a 1-byte length prefix for small payloads, or a 0xFF marker + 4-byte length for larger ones. The `None` value pickles to a fixed 4-byte sequence.

**Implementation impact:**

- The `CellApp` message handler must deserialize these five fields in order and dispatch to `python/cell/AbilityManager.py:useAbility()`.
- The `aAbilityId` must be validated against the player's known abilities list before execution.
- If `originatingSubsystemID` is non-zero, the ability was triggered by another system (e.g., an effect or pet command) rather than direct player input.
- Incorrect deserialization order will cause either a Python exception (wrong types) or silent data corruption (swapped integer fields).

---

## Guidelines for Applying These Standards

1. **Start at MEDIUM, escalate to HIGH.** Most initial discoveries from a single source are MEDIUM. Actively seek a second source to upgrade to HIGH before implementation.

2. **Never implement LOW-confidence wire formats.** LOW findings about wire format can cause client disconnects. Upgrade them first. LOW findings about game mechanics (damage formulas, cooldown durations) are safer to implement tentatively.

3. **Cite everything.** Even if a fact seems obvious from the .def file alone, cite it. Future researchers may not have the same context.

4. **Date your findings.** The Ghidra database changes as annotations accumulate. A finding verified on one date may reference function names that get updated later.

5. **Link related findings.** The combat system, ability system, and effect system are deeply interconnected. A finding about `onEffectResults` should reference findings about `resolveAbility` and `adjustStats`.

6. **Separate observation from interpretation.** In the description, clearly distinguish what the decompiled code shows (observation) from what you believe it means for the server (interpretation). An RTTI string proves a class name exists. It does not prove the class is used at runtime.

7. **Document negative findings.** If you investigated something and found that SGW does NOT follow the BigWorld reference implementation, document that too. "SGW does not use the standard BigWorld watcher system" is a valuable finding that prevents wasted effort.

8. **Keep wire format tables up to date.** If further decompilation reveals an additional field or corrects a type, update the table and bump the "Last verified" date. Do not leave stale tables in the documentation.
