---
name: faction-alignment-system
description: Faction/alignment system binary layout — EFaction enum, EAlignment enum, GameBeing field offsets, hostile sentinel, wire format
metadata:
  type: project
---

> [!NOTE] PROMOTION TARGET: spec.player.faction-alignment
>
> Triaged 2026-05-13 (Phase −0.5 step 4). V5-confirmed against `findings/faction-alignment-system.md`. EFaction 34-value enum, hostile sentinel ordinal=10, GameBeing+0x134/+0x135 layout, 1-byte wire format, combat gate logic — all canonical. Cross-link with `spec.combat.threat-and-aggro` for the faction-10 gate test.

## Faction / Alignment System (recovered 2026-05-13)

### EFaction enum (DB `resources.EFaction`, 34 values)
0-based ordinal = wire byte. Key values:
- 0 = FACTION_None (default neutral)
- 1 = FACTION_SGC (neutral NPCs in entity_templates)
- 3 = FACTION_Free_Jaffa (hardcoded as player's faction on login)
- 10 = FACTION_Burtonol — **the hostile combat sentinel** (only value the combat gate tests)

Entity templates: 83 mobs have faction=10, 56 have faction=1, 7 have faction=0, 2 have faction=3.

### EAlignment enum (DB `resources.EAlignment`, 7 values)
- 0 = Undefined, 1 = Praxis (Castle_CellBlock), 2 = SGU (SGC_W1), 3-5 = reserved, 6 = End
- Fixed at char creation; constraint `alignment in [0..5]` in sgw_player.

### Wire format
Both `onAlignmentUpdate` (idx 24, byte 0x98) and `onFactionUpdate` (idx 25, byte 0x99):
```
1 byte INT8 — enum ordinal value
```

### Binary field layout on GameBeing
- `GameBeing+0x134` = `mFaction` (INT8) — written by `FUN_00e02280` (GameBeing.cpp:0x3f6)
- `GameBeing+0x135` = `mAlignment` (INT8) — written by `FUN_00e02180` (GameBeing.cpp:0x3ef)
- `GameBeing+0x158` = `bStateField` (UINT32) — state flags

### Key function addresses
- `0x00e02180` — onAlignmentUpdate GameBeing handler (reads "alignment" → +0x135)
- `0x00e02280` — onFactionUpdate GameBeing handler (reads "faction" → +0x134)
- `0x00e6e330` — GameBeing_OnDeadStateChanged (called by both handlers; triggers visual refresh)
- `0x00d86b60` — register_NetIn_onAlignmentUpdate (name stub)
- `0x00d86e00` — register_NetIn_onFactionUpdate (name stub)
- `0x00d96a30` — register_NetOut_GiveFaction (name stub)
- `0x00d96cd0` — register_NetOut_SetFaction (name stub)

### Combat gate
faction == 10 AND !is_player AND !dead → attack target. No matrix, no relationship table on client.

### Open questions
- GiveFaction/SetFaction argument lists not in .def
- Nameplate color function not traced
- Player login faction=3 hardcode origin in Python server
- disguiseFaction INT8 (default -1) behavior not traced

See `docs/reverse-engineering/findings/faction-alignment-system.md` for full findings.
See `docs/reverse-engineering/address-map.md` "Faction / Alignment System" section.
