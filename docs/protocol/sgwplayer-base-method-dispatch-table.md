---
title: "SGWPlayer Exposed BaseMethod Dispatch Table"
type: reference
audience: engineers
last_updated: 2026-09-28
---

# SGWPlayer Exposed BaseMethod Dispatch Table

Client-to-server base method calls for the SGWPlayer entity type (in-world).
Only methods with `<Exposed/>` in the .def file get a wire index.

**Verified continuously** by `cimmeria-wire`'s `mercury::def_conformance` ([crates/wire/src/mercury/def_conformance/](../../crates/wire/src/mercury/def_conformance/), #801): every `sgw_player_base` constant and
the base-method constants in `cimmeria-wire::base` are checked against the
flattened exposed BaseMethods. So is `cimmeria_wire::base::names::base_method_name`,
the index-to-name table the base plugin registry (#962 step 5,
[plugin-architecture.md §4.5](../architecture/plugin-architecture.md#45-step-5-first-part-the-baseplugin-core))
uses to refuse a registration for an index that is not in this table.

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
| 5 | 0xC5 | chatIgnore | WSTRING aPlayerName, UINT8 aFlag (1 ignore, 0 stop ignoring). Handled: `dispatch/ignore.rs` (SS-C1) |
| 6 | 0xC6 | chatFriend | WSTRING aPlayerName, WSTRING aPlayerNick, UINT8 aFlag. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 7 | 0xC7 | chatList | UINT8 aChannelID. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 8 | 0xC8 | chatMute | UINT8 aChannelID, WSTRING aPlayerName, UINT8 aFlag. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3); the GM mute is `.mute` |
| 9 | 0xC9 | chatKick | UINT8 aChannelID, WSTRING aPlayerName. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 10 | 0xCA | chatOp | UINT8 aChannelID, WSTRING aPlayerName. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 11 | 0xCB | chatBan | UINT8 aChannelID, WSTRING aPlayerName, UINT8 aFlag. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 12 | 0xCC | chatPassword | UINT8 aChannelID, WSTRING aChannelPassword. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 13 | 0xCD | petition | WSTRING aMessage. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |
| 14 | 0xCE | announcePetition | WSTRING aMessage. Not implemented: answers with a feedback line, `dispatch/communicator_unsupported.rs` (SS-C3) |

### OrganizationMember — 4 exposed (indices 15-18)

Source: `entities/defs/interfaces/OrganizationMember.def`

| Index | Wire | Method | Args |
|-------|------|--------|------|
| 15 | 0xCF | organizationInvite | INT32 aOrganizationId, WSTRING aPlayerName |
| 16 | 0xD0 | organizationInviteByType | UINT8 aOrganizationType, WSTRING aPlayerName |
| 17 | 0xD1 | organizationKick | INT32 aOrganizationId, WSTRING aPlayerName |
| 18 | 0xD2 | organizationRankChange | INT32 aOrganizationId, WSTRING aPlayerName, UINT8 aRank |

Argument order and types are from `OrganizationMember.def:418-449` (corrected 2026-09-27; this table previously listed the name first and every numeric field as `INT32`).

Handled in `crates/base/src/base/dispatch/organization.rs`. Squads: 0xD0 with type 0 (after the base's Ignore check) and 0xD1 with a squad-range id are forwarded to the cell (ORG-03, `organization_squad.rs`). Teams and Commands: 0xCF, 0xD0 with type 1 or 2, 0xD1 and 0xD2 go to the base handlers under ORG-LOCK (ORG-07, [organization-system.md § Invite, kick and rank change](../gameplay/organization-system.md#invite-kick-and-rank-change-org-07)). A type above 2 is refused; 0xD2 with a squad id answers "not available yet".

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
| 20 | 0xD4 | versionInfoRequest | INT32 CategoryId, INT32 Version |
| 21 | 0xD5 | elementDataRequest | INT32 CategoryId, INT32 Key |

**Note** (corrected 2026-09-28, #840): the arguments are `ClientCache.def`'s two INT32s. The old row's `UINT16 categoryId` explains why logged in-world keys looked shifted left by 16 bits (dialog 60100 logged as 3938713600). These are the same ClientCache methods the Account entity exposes as `0xC0`/`0xC1`, but in-world they are `0xD4`/`0xD5`, and `0xC0`/`0xC1` are `chatJoin`/`chatLeave`. The connect loop routes `0xC0`/`0xC1` to the cache handlers only while the session has no player entity. In-world `0xD5` is served like a pre-world `0xC1`: the entry goes out next as a `resourceFragment` transfer, ahead of any background resync (`account_arms.rs` → `cooked_data::handle_element_data_request` → `cooked_sync::serve_miss`). `0xD4` has not been seen from the client and is not handled.

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
| 0 | 0xC0 | chatJoin | Creates or joins a named user channel; replies `onChatJoined` (issue #1039) |
| 1 | 0xC1 | chatLeave | Leaves a user channel by display id; replies `onChatLeft` |
| 2 | 0xC2 | sendPlayerCommunication | Chat message (spatial broadcast via CellService) |
| 22 | 0xD6 | logOff | Disconnect=0 → char select, Disconnect=1 → full exit |
| 23 | 0xD7 | cancelLogOff | Cancel pending logoff timer |
| 24 | 0xD8 | onClientReady | World entry finalization trigger |

## Verification

- `onClientReady = 0xD8` matches existing Rust constant `sgw_player_base::ON_CLIENT_READY`
- Communicator indices 0-4 match existing constants (CHAT_JOIN through CHAT_SET_DND)
- Derived by counting `<Exposed/>` BaseMethods across the full entity hierarchy
