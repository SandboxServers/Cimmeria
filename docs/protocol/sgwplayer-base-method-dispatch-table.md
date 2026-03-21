# SGWPlayer Exposed BaseMethod Dispatch Table

Client-to-server base method calls for the SGWPlayer entity type (in-world).
Only methods with `<Exposed/>` in the .def file get a wire index.

## Wire Encoding

Base methods use "proxy" encoding: `msg_id = index | 0xC0`
- `[msg_id: u8][word_len: u16][args...]` (no entity_id prefix, unlike cell methods)

## Flattening Order

Parent interfaces first (from SGWBeing.def Implements), then SGWPlayer interfaces
(from SGWPlayer.def Implements), then SGWPlayer own BaseMethods. Only `<Exposed/>`
methods are counted.

**Note**: SGWBeing interface, SGWAbilityManager, and SGWCombatant have 0 exposed
base methods, so interfaces start with Communicator.

---

## Interface BaseMethods (Indices 0-21)

### Communicator — 15 exposed (indices 0-14)

Source: `entities/defs/interfaces/Communicator.def`

| Index | Wire | Method | Args |
|-------|------|--------|------|
| 0 | 0xC0 | chatJoin | WSTRING channelName, WSTRING password |
| 1 | 0xC1 | chatLeave | UINT8 channelId |
| 2 | 0xC2 | sendPlayerCommunication | UINT8 channel, WSTRING target, WSTRING text |
| 3 | 0xC3 | chatSetAFKMessage | WSTRING message |
| 4 | 0xC4 | chatSetDNDMessage | WSTRING message |
| 5 | 0xC5 | chatIgnore | WSTRING playerName |
| 6 | 0xC6 | chatFriend | WSTRING playerName |
| 7 | 0xC7 | chatList | UINT8 channelId |
| 8 | 0xC8 | chatMute | WSTRING playerName, UINT8 channelId |
| 9 | 0xC9 | chatKick | WSTRING playerName, UINT8 channelId |
| 10 | 0xCA | chatOp | WSTRING playerName, UINT8 channelId |
| 11 | 0xCB | chatBan | WSTRING playerName, UINT8 channelId |
| 12 | 0xCC | chatPassword | UINT8 channelId, WSTRING password |
| 13 | 0xCD | petition | WSTRING text |
| 14 | 0xCE | announcePetition | WSTRING text |

### OrganizationMember — 4 exposed (indices 15-18)

Source: `entities/defs/interfaces/OrganizationMember.def`

| Index | Wire | Method | Args |
|-------|------|--------|------|
| 15 | 0xCF | organizationInvite | WSTRING playerName, INT32 orgId |
| 16 | 0xD0 | organizationInviteByType | WSTRING playerName, INT32 orgType |
| 17 | 0xD1 | organizationKick | WSTRING playerName, INT32 orgId |
| 18 | 0xD2 | organizationRankChange | WSTRING playerName, INT32 orgId, INT32 rank |

### MinigamePlayer — 1 exposed (index 19)

Source: `entities/defs/interfaces/MinigamePlayer.def`

| Index | Wire | Method | Args |
|-------|------|--------|------|
| 19 | 0xD3 | minigameCallRequest | INT32 gameDefId, WSTRING targetName |

### GateTravel — 0 exposed
### SGWInventoryManager — 0 exposed
### SGWMailManager — 0 exposed
### Missionary — 0 exposed
### SGWPoller — 0 exposed
### ContactListManager — 0 exposed
### SGWBlackMarketManager — 0 exposed

### ClientCache — 2 exposed (indices 20-21)

Source: `entities/defs/interfaces/ClientCache.def`

| Index | Wire | Method | Args |
|-------|------|--------|------|
| 20 | 0xD4 | versionInfoRequest | UINT32 versionSeed, STRING clientVersion, STRING language |
| 21 | 0xD5 | elementDataRequest | UINT16 categoryId, UINT32 key |

**Note**: These are protocol-level messages that share wire IDs with the Account
entity namespace (0xC0, 0xC1). However, in the SGWPlayer flattened index they
appear at 20-21 (0xD4-0xD5). The connect loop handles 0xC0/0xC1 specially before
checking the entity type, so both Account and SGWPlayer coexist correctly.

---

## SGWPlayer Own BaseMethods (Indices 22+)

Source: `entities/defs/SGWPlayer.def` lines 448-562

| Index | Wire | Method | Args | .def line |
|-------|------|--------|------|-----------|
| 22 | 0xD6 | logOff | INT8 Disconnect | 450 |
| 23 | 0xD7 | cancelLogOff | (none) | 456 |
| 24 | 0xD8 | onClientReady | (none) | 484 |
| 25 | 0xD9 | sendDuelChallenge | WSTRING playerName, INT8 squadDuel | 509 |
| 26 | 0xDA | onSpaceQueueStatus | (none) | 515 |
| 27 | 0xDB | onSpaceQueueReadyResponse | INT8 accept | 519 |
| 28 | 0xDC | onSpaceQueuedResponse | INT8 accept | 524 |
| 29 | 0xDD | perfStats | 12×FLOAT | 529 |

---

## Summary

| Range | Source | Exposed Count |
|-------|--------|---------------|
| 0-14 | Communicator | 15 |
| 15-18 | OrganizationMember | 4 |
| 19 | MinigamePlayer | 1 |
| 20-21 | ClientCache | 2 |
| 22-29 | SGWPlayer (own) | 8 |
| **Total** | | **30** |

## Key Methods for Server Implementation

| Index | Wire | Method | Notes |
|-------|------|--------|-------|
| 0 | 0xC0 | chatJoin | Channel join (stub — auto-joined) |
| 1 | 0xC1 | chatLeave | Channel leave |
| 2 | 0xC2 | sendPlayerCommunication | Chat message (spatial broadcast via CellService) |
| 22 | 0xD6 | logOff | Disconnect=0 → char select, Disconnect=1 → full exit |
| 23 | 0xD7 | cancelLogOff | Cancel pending logoff timer |
| 24 | 0xD8 | onClientReady | World entry finalization trigger |

## Verification

- `onClientReady = 0xD8` matches existing Rust constant `sgw_player_base::ON_CLIENT_READY`
- Communicator indices 0-4 match existing constants (CHAT_JOIN through CHAT_SET_DND)
- Derived by counting `<Exposed/>` BaseMethods across the full entity hierarchy
