# Rust Gameserver Dev Memory

## Project Context
- Stargate Worlds MMO server emulator (Cimmeria project)
- Rust rewrite on branch `feature/rust-rewrite`
- Key files: `crates/services/src/base.rs`, `crates/services/src/mercury_ext.rs`, `crates/services/src/auth.rs`
- C++ reference: `src/baseapp/mercury/sgw/` (client_handler.cpp, connect_handler.cpp, messages.cpp)

## Audit Findings (2026-03-04)
See [audit-findings.md](audit-findings.md) for full details.

### Critical Bug (FIXED)
- **RESOURCE_FRAGMENT (0x36) was using u32 length prefix instead of u16** — now fixed. Unit test `resource_fragment_uses_u16_length_prefix` guards this.

### Packet Layout Gotcha
- `build_outgoing` layout: `[flags:u8][body][footers]` — seq_id and acks are FOOTERS appended after the body, NOT between flags and body. Body starts at offset 1 in the plaintext.

### Entity Class IDs (verified from entities.xml + ServerOnly flags)
- SGWSpawnableEntity=0, SGWBeing=1, SGWPlayer=2, SGWGmPlayer=3, SGWMob=4, SGWPet=5, SGWDuelMarker=6, Account=7
- SGWBlackMarket is ServerOnly (skipped)

### Account Method Indices (verified from defs.cpp inheritance + Account.def + ClientCache.def)
- Exposed base methods: ClientCache first (versionInfoRequest=0xC0, elementDataRequest=0xC1), then Account (logOff=0xC2, createCharacter=0xC3, playCharacter=0xC4, deleteCharacter=0xC5, requestCharacterVisuals=0xC6, onClientVersion=0xC7)
- Client methods: ClientCache first (onVersionInfo=0x80, onCookedDataError=0x81), then Account (onCharacterList=0x82, onCharacterCreateFailed=0x83, onCharacterVisuals=0x84, onCharacterLoadFailed=0x85)

### Wire Format Notes
- connect_reply: uses DWORD_LENGTH (u32 prefix) because it maps to ServerMessageList[BASEMSG_AUTHENTICATE] = {DWORD_LENGTH, 0, "AUTHENTICATE"}
- RESOURCE_FRAGMENT: uses WORD_LENGTH (u16 prefix) in ServerMessageList
- Both createCellPlayer and forcedPosition use swapped rotation: rotX, rotZ, rotY

### C++ Account.py createCharacter Flow
- Validates charDef, visual choices, name length/uniqueness, skin tint range
- Stores: alignment, archetype, gender, bodyset, startingWorld, startingX/Y/Z, world_id, components, skin_color_id, abilities, access_level
- Creates inventory items from visual choices
- Rust version is missing most of this (only basic fields, pos=0,0,0, no items/abilities/world_id)

### C++ Account.py requestCharacterVisuals Flow
- Queries sgw_inventory for equipped items
- Looks up item defs for visualComponent
- Sends primaryTint=0xFF, secondaryTint=0xFF, skinTint=SKIN_TINTS[skin_color_id]
- Rust sends primaryTint=0, secondaryTint=0, skinTint=skin_color_id (raw index, not mapped)
