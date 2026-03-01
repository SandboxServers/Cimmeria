# Secondary Systems Wire Formats

> **Date**: 2026-03-01
> **Phase**: 4 — Secondary Systems RE
> **Confidence**: HIGH (derived from `.def` files + `alias.xml` + universal RPC dispatcher architecture)
> **Sources**: `MinigamePlayer.def`, `Communicator.def`, `SGWMailManager.def`, `SGWBlackMarketManager.def`, `ContactListManager.def`, `SGWPlayer.def`, `alias.xml`

---

## Table of Contents

1. [Minigame System](#1-minigame-system)
2. [Chat / Communication System](#2-chat--communication-system)
3. [Mail System](#3-mail-system)
4. [Black Market (Auction House)](#4-black-market-auction-house)
5. [Contact List System](#5-contact-list-system)
6. [Trade System](#6-trade-system)
7. [Duel System](#7-duel-system)
8. [Pet System](#8-pet-system)

---

## 1. Minigame System

**Interface**: `MinigamePlayer` (implemented by `SGWPlayer`)

Minigames are web-based (Flash/browser) games embedded in the client. The server manages matchmaking, call requests, and result processing.

### Client → Server (Exposed Cell Methods)

#### `startMinigame` — Begin a Minigame

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `endCurrentMinigame` — End Current Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `debugStartMinigame` — Debug: Start Specific Game

| Field | Type | Size |
|-------|------|------|
| `aGameId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `requestSpectateList` — Request Spectatable Players

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `spectateMinigame` — Spectate a Player

| Field | Type | Size |
|-------|------|------|
| `playerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `registerToMinigameHelp` — Register as Helper

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |

#### `updateRegisterToMinigameHelp` — Update Registration

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `note` | `WSTRING` | 4B len + N×2B |
| `inRangeOnly` | `UINT8` | 1B |
| `wantsRequests` | `UINT8` | 1B |

#### `minigameCallAccept` / `minigameCallDecline` — Respond to Call

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `minigameCallAbort` — Cancel Active Call

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameStartCancel` — Cancel Starting Game

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

#### `minigameContactRequest` — Request NPC Contact

| Field | Type | Size |
|-------|------|------|
| `ContactId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

### Client → Server (Exposed Base Methods)

#### `minigameCallRequest` — Request Help from Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `RemotePlayerName` | `WSTRING` | 4B len + N×2B |
| `TipAmount` | `INT32` | 4B |

### Server → Client

#### `onStartMinigame` — Launch Game URL

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `URL` | `WSTRING` | 4B len + N×2B — Flash/browser game URL |

#### `onStartMinigameDialog` — Show Game Selection

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Name` | `WSTRING` | 4B len + N×2B |
| `Difficulty` | `WSTRING` | 4B len + N×2B |
| `TCLevel` | `INT32` | 4B |
| `Verb` | `WSTRING` | 4B len + N×2B |
| `ArchetypeBitfield` | `INT32` | 4B |
| `CanPlay` | `UINT8` | 1B |
| `CanCall` | `UINT8` | 1B |
| `CanSpectate` | `UINT8` | 1B |

#### `onStartMinigameDialogClose` / `onEndMinigame`

*(No arguments)* — 1 byte each

#### `onSpectateList` — Available Players to Watch

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `playerIds` | `ARRAY<INT32>` | 4B count + N×4B |
| `playerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onMinigameRegistrationPrompt` — Registration Cost

| Field | Type | Size |
|-------|------|------|
| `Cost` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `minigameRegistrationInfo` — Current Registration State

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Registered` | `UINT8` | 1B |
| `InRangeOnly` | `UINT8` | 1B |
| `WantsRequests` | `UINT8` | 1B |
| `Note` | `WSTRING` | 4B len + N×2B |

#### `addOrUpdateMinigameHelper` — Helper List Entry

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `PlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Note` | `WSTRING` | 4B len + N×2B |
| `Level` | `UINT8` | 1B |
| `Archetype` | `UINT8` | 1B |
| `Friend` | `UINT8` | 1B |

#### `removeMinigameHelper`

| Field | Type | Size |
|-------|------|------|
| `PlayerId` | `INT32` | 4B |

#### `minigameCallDisplay` — Incoming Call Request

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `CallingPlayerId` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Archetype` | `INT32` | 4B |
| `Level` | `INT32` | 4B |
| `TipAmount` | `INT32` | 4B |
| `ExpiresAt` | `INT32` | 4B |
| `GameName` | `WSTRING` | 4B len + N×2B |
| `GameDifficulty` | `WSTRING` | 4B len + N×2B |
| `GameVerb` | `WSTRING` | 4B len + N×2B |
| `GameTC` | `INT32` | 4B |
| `NPCTitle` | `WSTRING` | 4B len + N×2B |

#### `minigameCallResult` — Call Outcome

| Field | Type | Size |
|-------|------|------|
| `ResultCode` | `INT32` | 4B |
| `StartTime` | `FLOAT` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `minigameCallAbort` — Call Cancelled

| Field | Type | Size |
|-------|------|------|
| `CallingPlayerId` | `INT32` | 4B |

#### `showMinigameContact` — NPC Contact Card

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Id` | `INT32` | 4B |
| `Name` | `WSTRING` | 4B len + N×2B |
| `Title` | `WSTRING` | 4B len + N×2B |
| `Icon` | `WSTRING` | 4B len + N×2B |
| `Time` | `INT32` | 4B |
| `Success` | `INT32` | 4B |
| `Cost` | `INT32` | 4B |

---

## 2. Chat / Communication System

**Interface**: `Communicator` (implemented by `SGWPlayer`)

### Client → Server (Exposed Base Methods)

#### `sendPlayerCommunication` — Send Chat Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `Channel` | `UINT8` | 1B | EChannel enum (say, tell, party, etc.) |
| `Target` | `WSTRING` | 4B len + N×2B | Target player (for tells) or empty |
| `Text` | `WSTRING` | 4B len + N×2B | Message text |

#### `chatJoin` — Join Chat Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelName` | `WSTRING` | 4B len + N×2B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `chatLeave` — Leave Chat Channel

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatSetAFKMessage` / `chatSetDNDMessage`

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `message` | `WSTRING` | 4B len + N×2B |

#### `chatIgnore` — Ignore/Unignore Player

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B — 1=ignore, 0=unignore |

#### `chatFriend` — Add/Remove Friend

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNick` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatList` — Request Channel Member List

| Field | Type | Size |
|-------|------|------|
| `aChannelID` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `chatMute` — Mute/Unmute in Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatKick` — Kick from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatOp` — Grant Operator Status

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |

#### `chatBan` — Ban/Unban from Channel

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aFlag` | `UINT8` | 1B |

#### `chatPassword` — Set Channel Password

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aChannelID` | `UINT8` | 1B |
| `aChannelPassword` | `WSTRING` | 4B len + N×2B |

#### `petition` / `announcePetition` — GM Petition

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aMessage` | `WSTRING` | 4B len + N×2B |

### Server → Client

#### `onSystemCommunication` — System Message

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `TextType` | `INT32` | 4B | Message type enum |
| `StringId` | `INT32` | 4B | Localized string ID |
| `Speaker` | `WSTRING` | 4B len + N×2B | NPC name (for NPC speech) |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken | Token substitutions |

**`StringToken` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stringID` | `INT32` | 4B |
| `literal` | `WSTRING` | 4B len + N×2B |

#### `onPlayerCommunication` — Player Chat Message

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B — ESpeakerFlags |
| `Channel` | `UINT8` | 1B — EChannel |
| `Text` | `WSTRING` | 4B len + N×2B |

#### `onLocalizedCommunication` — Localized Player Chat

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `Speaker` | `WSTRING` | 4B len + N×2B |
| `SpeakerFlags` | `UINT8` | 1B |
| `Channel` | `UINT8` | 1B |
| `Text` | `WSTRING` | 4B len + N×2B |
| `tokenList` | `ARRAY<StringToken>` | 4B count + N×StringToken |

#### `onTellSent` — Tell Confirmation

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aTarget` | `WSTRING` | 4B len + N×2B |
| `aText` | `WSTRING` | 4B len + N×2B |

#### `onChatJoined` — Channel Join Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |
| `ChannelID` | `UINT8` | 1B |

#### `onChatLeft` — Channel Leave Notification

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ChannelName` | `WSTRING` | 4B len + N×2B |

#### `onNickChanged` — Nickname Update

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aPlayerNickname` | `WSTRING` | 4B len + N×2B |
| `aAddRemoveFlag` | `UINT8` | 1B |

---

## 3. Mail System

**Interface**: `SGWMailManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `sendMailMessage` — Send Mail

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `RecipientFlags` | `INT32` | 4B | Recipient type flags |
| `Recipients` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) | Recipient names |
| `Subject` | `WSTRING` | 4B len + N×2B | Mail subject |
| `Body` | `WSTRING` | 4B len + N×2B | Mail body text |
| `Cash` | `INT32` | 4B | Attached currency |
| `bCOD` | `UINT8` | 1B | Cash on delivery flag |
| `ItemId` | `INT32` | 4B | Attached item instance ID |
| `ItemQuantity` | `INT32` | 4B | Stack size of attached item |

#### `requestMailHeaders` — Fetch Mail List

| Field | Type | Size |
|-------|------|------|
| `bArchive` | `UINT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `requestMailBody` — Fetch Mail Content

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `deleteMailMessage` / `archiveMailMessage` / `returnMailMessage`

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `takeCashFromMailMessage` / `payCODForMailMessage`

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

#### `takeItemFromMailMessage` — Take Attached Item

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `MailId` | `INT32` | 4B | |
| `ContainerId` | `INT32` | 4B | Destination bag |
| `SlotId` | `INT32` | 4B | Destination slot |

**Total wire size**: 1B header + 12B = **13 bytes**

### Server → Client

#### `onMailHeaderInfo` — Mail Header List

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `ResetCategory` | `UINT8` | 1B | Clear existing headers flag |
| `bArchive` | `UINT8` | 1B | Archive vs inbox |
| `MessageHeaders` | `ARRAY<MessageHeader>` | 4B count + N×MessageHeader | Mail headers |
| `MessageAttachments` | `ARRAY<MessageAttachment>` | 4B count + N×MessageAttachment | Attachment info |

**`MessageHeader` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `id` | `INT32` | 4B |
| `fromText` | `WSTRING` | 4B len + N×2B |
| `fromId` | `INT32` | 4B |
| `subjectText` | `WSTRING` | 4B len + N×2B |
| `subjectId` | `INT32` | 4B |
| `cash` | `INT32` | 4B |
| `sentTime` | `FLOAT` | 4B |
| `readTime` | `FLOAT` | 4B |
| `flags` | `INT32` | 4B |

**`MessageAttachment` FIXED_DICT layout** (20 bytes):

| Field | Type | Size |
|-------|------|------|
| `id` | `INT32` | 4B |
| `itemId` | `INT32` | 4B |
| `stackSize` | `INT32` | 4B |
| `durability` | `INT32` | 4B |
| `charges` | `INT32` | 4B |

#### `onMailHeaderRemove` — Remove Mail from List

| Field | Type | Size |
|-------|------|------|
| `MailId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onMailRead` — Mail Body Content

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `MailId` | `INT32` | 4B |
| `BodyText` | `WSTRING` | 4B len + N×2B |
| `BodyId` | `INT32` | 4B |
| `ToText` | `WSTRING` | 4B len + N×2B |

#### `sendMailResult` — Send Outcome

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `ResultCode` | `UINT8` | 1B |
| `FailedRecipients` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |
| `FailedRecipientFlags` | `INT32` | 4B |

---

## 4. Black Market (Auction House)

**Interface**: `SGWBlackMarketManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `BMSearch` — Search Auctions

| Field | Type | Wire Encoding | Notes |
|-------|------|---------------|-------|
| `searchOptions` | `BMSearchOptions` | FIXED_DICT | Search filters |

**`BMSearchOptions` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `sortId` | `UINT8` | 1B |
| `clientKey` | `INT32` | 4B |
| `sequenceId` | `INT32` | 4B |
| `bForward` | `UINT8` | 1B |
| `sellerName` | `STRING` | 4B len + N×1B (UTF-8) |
| `bidderName` | `STRING` | 4B len + N×1B |
| `itemName` | `STRING` | 4B len + N×1B |
| `minTC` | `INT32` | 4B |
| `maxTC` | `INT32` | 4B |
| `quality` | `INT32` | 4B |
| `monikerCRC` | `INT32` | 4B |

**Note**: `STRING` (not `WSTRING`) — uses UTF-8, not UTF-16LE.

#### `BMCreateAuction` — List Item for Sale

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `itemInstanceId` | `INT32` | 4B | Item to auction |
| `buyoutPrice` | `INT32` | 4B | Buyout price (0 = no buyout) |
| `auctionLength` | `UINT8` | 1B | Duration enum |
| `startingPrice` | `INT32` | 4B | Starting bid |

**Total wire size**: 1B header + 13B = **14 bytes**

#### `BMPlaceBid` — Bid on Auction

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |
| `bidAmount` | `INT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `BMCancelAuction` — Cancel Own Auction

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `BMStartWatchingItem` / `BMStopWatchingItem` — Watch List

| Field | Type | Size |
|-------|------|------|
| `itemDefId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes** each

### Server → Client

#### `onBMOpen` — Open Black Market UI

| Field | Type | Size |
|-------|------|------|
| `entityId` | `INT32` | 4B — NPC entity ID |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMError` — Error Notification

| Field | Type | Size |
|-------|------|------|
| `errorId` | `INT32` | 4B — EBlackMarketError enum |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMAuctions` — Search Results

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `auctionItems` | `ARRAY<AuctionItem>` | 4B count + N×AuctionItem |
| `totalResults` | `INT32` | 4B |
| `clientKey` | `INT32` | 4B |

**`AuctionItem` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `sequenceId` | `INT32` | 4B |
| `itemDefId` | `INT32` | 4B |
| `stackSize` | `INT32` | 4B |
| `durability` | `INT32` | 4B |
| `charges` | `INT32` | 4B |
| `currentBid` | `INT32` | 4B |
| `buyoutPrice` | `INT32` | 4B |
| `endTimeValue` | `UINT8` | 1B |
| `nextMinBidPrice` | `INT32` | 4B |
| `sellerName` | `STRING` | 4B len + N×1B (UTF-8) |

#### `onBMAuctionRemove` — Auction Ended/Cancelled

| Field | Type | Size |
|-------|------|------|
| `sequenceId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onBMAuctionUpdate` — Auction Status Change

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `auctionItem` | `AuctionItem` | FIXED_DICT (variable) |

#### `onBMWatchedItemsUpdate` — Watch List Sync

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `itemList` | `ARRAY<INT32>` | 4B count + N×4B |

---

## 5. Contact List System

**Interface**: `ContactListManager` (implemented by `SGWPlayer`)

### Client → Server (Exposed Cell Methods)

#### `contactListCreate` — Create List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aName` | `WSTRING` | 4B len + N×2B |
| `aFlags` | `UINT32` | 4B |

#### `contactListDelete` — Delete List

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `contactListRename` — Rename List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aName` | `WSTRING` | 4B len + N×2B |

#### `contactListFlagsUpdate` — Update List Flags

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |
| `aFlags` | `UINT32` | 4B |

**Total wire size**: 1B header + 8B = **9 bytes**

#### `contactListAddMembers` — Add Players to List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `contactListRemoveMembers` — Remove Players from List

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

### Server → Client

#### `onContactListUpdate` — List Created/Updated

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aName` | `WSTRING` | 4B len + N×2B |
| `aFlags` | `UINT32` | 4B |

#### `onContactListDelete` — List Deleted

| Field | Type | Size |
|-------|------|------|
| `aListId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `onContactListAddMembers` — Members Added

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onContactListRemoveMembers` — Members Removed

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aListId` | `INT32` | 4B |
| `aPlayerNames` | `ARRAY<WSTRING>` | 4B count + N×(4B+str) |

#### `onContactListEvent` — Online/Offline Notification

| Field | Type | Size | Notes |
|-------|------|------|-------|
| `aPlayerName` | `WSTRING` | variable | Player who triggered event |
| `aEventId` | `UINT32` | 4B | Event type (login, logout, etc.) |
| `aDataValue` | `INT32` | 4B | Additional data |

---

## 6. Trade System

Defined directly on `SGWPlayer.def`.

### Client → Server (Exposed Cell Methods)

#### `tradeRequest` — Initiate Trade

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B — target entity |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |

**`LocalTradeProposal` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `version` | `INT32` | 4B |
| `items` | `ARRAY<LocalTradeItem>` | 4B count + N×8B |
| `cash` | `INT32` | 4B |
| `lockState` | `INT8` | 1B |

**`LocalTradeItem` FIXED_DICT layout** (8 bytes):

| Field | Type | Size |
|-------|------|------|
| `instanceId` | `INT32` | 4B |
| `slotId` | `INT32` | 4B |

#### `tradeUpdateProposal` — Update Trade Offer

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |

#### `tradeRequestCancel` — Cancel Trade

| Field | Type | Size |
|-------|------|------|
| `EntityId` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

#### `tradeLockState` — Lock/Unlock/Confirm

| Field | Type | Size |
|-------|------|------|
| `LocalVersionId` | `INT32` | 4B |
| `RemoteVersionId` | `INT32` | 4B |
| `LockState` | `INT8` | 1B |

**Total wire size**: 1B header + 9B = **10 bytes**

### Server → Client

#### `onTradeState` — Trade Window Update

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `EntityId` | `INT32` | 4B |
| `LocalProposal` | `LocalTradeProposal` | FIXED_DICT |
| `RemoteProposal` | `RemoteTradeProposal` | FIXED_DICT |

**`RemoteTradeProposal` FIXED_DICT layout** (variable):

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `version` | `INT32` | 4B |
| `items` | `ARRAY<InvItem>` | 4B count + N×InvItem (variable — ammoTypes array) |
| `cash` | `INT32` | 4B |
| `lockState` | `INT8` | 1B |

**Note**: `RemoteTradeProposal` uses `InvItem` (full item data) while `LocalTradeProposal` uses `LocalTradeItem` (just instance + slot IDs). This is because you already have your own items in your inventory — you only need full data for the other player's items.

#### `onTradeResults` — Trade Complete/Cancelled

| Field | Type | Size |
|-------|------|------|
| `EntityId` | `INT32` | 4B |
| `Result` | `INT32` | 4B — success/failure code |

**Total wire size**: 1B header + 8B = **9 bytes**

---

## 7. Duel System

Defined directly on `SGWPlayer.def`.

### Client → Server

#### `sendDuelChallenge` (Base Method, Exposed) — Issue Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aPlayerName` | `WSTRING` | 4B len + N×2B |
| `aSquadDuel` | `INT8` | 1B — 1 for squad duel |

#### `sendDuelResponse` (Cell Method, Exposed) — Accept/Decline

| Field | Type | Size |
|-------|------|------|
| `aResponse` | `INT8` | 1B |

**Total wire size**: 1B header + 1B = **2 bytes**

#### `duelForfeit` (Cell Method, Exposed) — Surrender

*(No arguments)*

**Total wire size**: 1B header = **1 byte**

### Server → Client

#### `onDuelChallenge` — Incoming Challenge

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityId` | `INT32` | 4B — challenger entity ID |
| `aSquadList` | `ARRAY<INT32>` | 4B count + N×4B — challenger's squad |

#### `onDuelEntitiesSet` — Duel Participants

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `aEntityList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onDuelEntitiesRemove` — Participant Defeated

| Field | Type | Size |
|-------|------|------|
| `aEntityId` | `INT32` | 4B |

#### `onDuelEntitiesClear` — Duel Ended

*(No arguments)* — 1 byte

---

## 8. Pet System

Defined on `SGWPet.def` (entity type, parent: `SGWMob`).

### Server → Client

#### `onPetAbilityList` — Pet's Available Abilities

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `abilitiesList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onPetStanceList` — Available Stances

| Field | Type | Wire Encoding |
|-------|------|---------------|
| `stanceList` | `ARRAY<INT32>` | 4B count + N×4B |

#### `onPetStanceUpdate` — Current Stance Changed

| Field | Type | Size |
|-------|------|------|
| `stance` | `INT32` | 4B |

**Total wire size**: 1B header + 4B = **5 bytes**

### Client → Server (via SGWPlayer.def — separate from pet entity)

Pet commands are sent as player cell methods, not pet entity methods:

| Method | Args | Notes |
|--------|------|-------|
| `petInvokeAbility` | `INT32 abilityId`, `INT32 targetId` | Use pet ability |
| `petAbilityToggle` | `INT32 abilityId`, `UINT8 toggle` | Toggle auto-cast |
| `petChangeStance` | `INT32 stanceId` | Change pet stance |

---

## Implementation Notes

1. **Chat routing**: Most chat goes through base methods (name-based lookup needed). Only `processPlayerCommunication` is a cell method for local area chat.
2. **Mail attachments**: Single item per mail (itemId + quantity). Cash can be attached separately. COD (Cash on Delivery) is supported.
3. **Black Market uses STRING** (UTF-8) for seller/bidder/item names, unlike most other systems that use `WSTRING` (UTF-16LE).
4. **Trade asymmetry**: Your items are `LocalTradeItem` (instance+slot), other player's items are `InvItem` (full data). This reduces bandwidth since you already have your own item data.
5. **Minigame URLs**: Games are launched via web URLs — the client embeds a browser/Flash player.
6. **Pet commands**: Sent as player methods, not pet entity methods — the server routes to the pet internally.
7. **Contact lists**: Generic list system (not just friends) — supports custom lists with flags.
