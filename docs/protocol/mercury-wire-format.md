# Mercury Wire Format

> **Last updated**: 2026-03-01
> **RE Status**: Partially documented from BigWorld 2.0.1 source + Cimmeria implementation
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/network/`, `src/mercury/`

---

## Overview

Mercury is BigWorld's custom reliable UDP protocol. All game traffic between the client and the BaseApp (after the SOAP login phase) uses Mercury. It provides ordered reliable delivery, fragmentation, acknowledgements, and channel-based multiplexing over standard UDP datagrams.

SGW (Stargate Worlds) uses BigWorld ~1.9.x Mercury. The Cimmeria server reimplements the protocol in `src/mercury/` using Boost.Asio rather than raw sockets.

## Packet Structure

A Mercury packet fits within a single UDP datagram. The maximum size is constrained by MTU:

```
UDP MTU:           1500 bytes (Ethernet) / 1492 bytes (ADSL)
IP header:         20 bytes
UDP header:        8 bytes
Available payload: ~1472 bytes (BigWorld MAX_SIZE) / ~1456 bytes (Cimmeria MAX_RECEIVE_LENGTH)
```

### Packet Layout (On the Wire)

```
+-------------------+-------------------------------------------+-------------------+
|   Packet Header   |              Message Body                 |  Packet Footer    |
|   (variable)      |       (one or more messages)              |  (variable)       |
+-------------------+-------------------------------------------+-------------------+
```

### Packet Header

The header begins with a flags field that determines what other header/footer fields are present:

| Field | Size | Condition | Description |
|-------|------|-----------|-------------|
| Flags | 1 byte (Cimmeria) / 2 bytes (BW) | Always | Bitmask of packet properties |
| Sequence ID | 4 bytes | `FLAG_HAS_SEQUENCE` | Packet ordering number |
| Fragment Begin | 4 bytes | `FLAG_FRAGMENTED` | First fragment sequence ID |
| Fragment End | 4 bytes | `FLAG_FRAGMENTED` | Last fragment sequence ID |

**Cimmeria simplification**: Cimmeria uses a 1-byte (uint8) flags field instead of BigWorld's 2-byte (uint16) field. The indexed channel flag (0x80) is unused because SGW has no indexed channels.

### Cimmeria Packet Byte Ordering (Confirmed via Ghidra + Source)

The Cimmeria implementation uses **native (little-endian) byte ordering** for all multi-byte packet fields, in contrast to BigWorld which uses **network byte order (big-endian)** with `BW_HTONS`/`BW_HTONL` macros.

Evidence from Cimmeria source (`src/mercury/packet.cpp`):

**Sending (finalize)**:
```
// Flags: written as raw uint8 (no byte-swap needed for single byte)
*(Flags *)&buffer_[0] = flags_;

// Fragment footers: written as native uint32 directly into buffer
*(SequenceId *)&buffer_[off] = sequenceId - fragmentIndex_;
*(SequenceId *)&buffer_[off + sizeof(SequenceId)] = sequenceId - fragmentIndex_ + fragments_;

// Sequence ID: written as native uint32
*(SequenceId *)&buffer_[off] = sequenceId;

// ACKs: streamed via operator<< (native byte order)
buf << acks_[i];

// ACK count: written as native uint8
buf << acks;
```

**Receiving (unserialize)** -- footers are stripped from the **end** of the packet backward using `pop()`:
```
// Flags: read first from offset 0
buf >> flags_;

// ACK count: popped from end of packet first
buf.pop(ack_count);
// ACK sequence IDs: popped next (ack_count * 4 bytes)
buf.pop(&acks_[0], sizeof(SequenceId) * ack_count);

// Sequence ID: popped next
buf.pop(sequenceId_);

// Fragment IDs: popped last (note: lastFragmentId first, then firstFragmentId)
buf.pop(lastFragmentId_).pop(firstFragmentId_);

// First request offset: popped from end
buf.pop(firstRequestOffset_);
```

**Cimmeria on-wire footer layout** (read from end of packet backward):

```
End of packet:
  [ACK count: uint8]                    (if FLAG_HAS_ACKS)
  [ACK seq IDs: uint32 * ack_count]     (if FLAG_HAS_ACKS)
  [Sequence ID: uint32]                 (if FLAG_HAS_SEQUENCE)
  [Last Fragment ID: uint32]            (if FLAG_FRAGMENTED)
  [First Fragment ID: uint32]           (if FLAG_FRAGMENTED)
  [First Request Offset: uint16]        (if FLAG_HAS_REQUESTS)
  ... message body ...
  [Flags: uint8]                        (always, at offset 0)
```

**BigWorld on-wire footer layout** (read from end of packet backward, network byte order):

```
End of packet:
  [Checksum: uint32, network order]     (if FLAG_HAS_CHECKSUM)
  [Channel ID: int32, network order]    (if FLAG_INDEXED_CHANNEL)
  [Channel Version: uint32, native]     (if FLAG_INDEXED_CHANNEL)
  [Piggyback data]                      (if FLAG_HAS_PIGGYBACKS)
  [Cumulative ACK: uint32, net order]   (if FLAG_HAS_CUMULATIVE_ACK)
  [ACK count: uint8]                    (if FLAG_HAS_ACKS)
  [ACK seq IDs: uint32 each, net order] (if FLAG_HAS_ACKS)
  [Sequence ID: uint32, network order]  (if FLAG_HAS_SEQUENCE_NUMBER)
  [Frag End: uint32, network order]     (if FLAG_IS_FRAGMENT)
  [Frag Begin: uint32, network order]   (if FLAG_IS_FRAGMENT)
  [First Request Offset: uint16, net]   (if FLAG_HAS_REQUESTS)
  ... message body ...
  [Flags: uint16, network order]        (always, at offset 0)
```

### Packet Flags

| Flag | BW Value | Cimmeria Value | Description |
|------|----------|----------------|-------------|
| `FLAG_HAS_REQUESTS` | 0x0001 | 0x01 | Packet contains request-reply messages |
| `FLAG_HAS_PIGGYBACKS` | 0x0002 | 0x02 | Piggyback packets appended |
| `FLAG_HAS_ACKS` | 0x0004 | 0x04 | ACK list in footer |
| `FLAG_ON_CHANNEL` | 0x0008 | 0x08 | Sent on a channel (vs standalone) |
| `FLAG_IS_RELIABLE` | 0x0010 | 0x10 | Must be acknowledged |
| `FLAG_IS_FRAGMENT` | 0x0020 | 0x20 | Part of a fragmented bundle |
| `FLAG_HAS_SEQUENCE_NUMBER` | 0x0040 | 0x40 | Has a sequence number |
| `FLAG_INDEXED_CHANNEL` | 0x0080 | 0x80 | Indexed channel (unused in SGW) |
| `FLAG_HAS_CHECKSUM` | 0x0100 | N/A | CRC32 checksum (BW only) |
| `FLAG_CREATE_CHANNEL` | 0x0200 | N/A | First packet on a new channel (BW only) |
| `FLAG_HAS_CUMULATIVE_ACK` | 0x0400 | N/A | Cumulative ACK (BW only) |

### Packet Footer

Footers are appended after the message body and read from the end of the packet backward:

| Field | Size | Condition | Description |
|-------|------|-----------|-------------|
| ACK Count | 1 byte | `FLAG_HAS_ACKS` | Number of ACKs following |
| ACK Sequence IDs | 4 bytes each | `FLAG_HAS_ACKS` | Sequence IDs being acknowledged |
| First Request Offset | 2 bytes | `FLAG_HAS_REQUESTS` | Offset to first request message |

In BigWorld, additional footer fields include Channel ID/Version and a checksum. Cimmeria omits these.

## Message Format

Messages are packed sequentially into the packet body. Each message consists of a header and a payload:

### Message Header

| Field | Size | Description |
|-------|------|-------------|
| Message ID | 1 byte | Identifies the message type (0x00-0xFE) |
| Length | 0, 2, or 4 bytes | Depends on message format (see below) |

Message ID `0xFF` is reserved for reply messages (request-response pattern).

### Message Length Types

Defined in `InterfaceElement` / `Message::Format`:

| Type | Value | Description |
|------|-------|-------------|
| `CONSTANT_LENGTH` | 0 | No length field; size is fixed and known from the message table |
| `WORD_LENGTH` | 2 | 2-byte (uint16) length prefix |
| `DWORD_LENGTH` | 4 | 4-byte (uint32) length prefix |

### Entity Messages

Entity messages use a special format with an entity ID prefix. They represent RPC calls to entity methods:

```
+------------+-----------+------------------+
| Message ID | Entity ID | Method Arguments |
| (1 byte)   | (4 bytes) |   (variable)     |
+------------+-----------+------------------+
```

In Cimmeria, entity messages are distinguished by the `isEntityMessage` flag in the `Message::Format` table. There are two sub-types:

- **Base entity messages**: Call methods on the base entity (server-side persistent)
- **Cell entity messages**: Call methods on the cell entity (spatial simulation)

The method ID within the entity message determines which Python method is invoked.

### Entity Message ID Encoding (Cimmeria)

From `src/mercury/bundle.cpp`, entity messages use a compact encoding scheme:

**Base entity messages** (message IDs 0x80-0xBD):
- Method IDs 0x00-0x3C: `messageId = methodId + 0x80`, followed by `entityId` (uint32)
- Method IDs 0x3D-0x13C: `messageId = 0xBD` (extended), followed by `entityId` (uint32) then `methodId - 0x3D` (uint8)

**Cell entity messages** (message IDs 0xC0-0xFD):
- Method IDs 0x00-0x3C: `messageId = methodId + 0xC0`, followed by `entityId` (uint32)
- Method IDs 0x3D-0x13C: `messageId = 0xFD` (extended), followed by `entityId` (uint32) then `methodId - 0x3D` (uint8)

All entity messages use `WORD_LENGTH` (uint16 length prefix) regardless of the message table.

## Request-Reply Protocol

Mercury supports a request-reply pattern where the sender registers a handler that is called when the reply arrives or a timeout expires.

### How Requests Work

1. **Sender starts a request**: Calls `Bundle::startRequest()` with an `InterfaceElement` and a `ReplyMessageHandler`.
2. **Extra header fields**: The bundle allocates space for a `ReplyID` (int32) and a `nextRequestOffset` (uint16) immediately after the message header.
3. **Request linking**: Packets maintain a linked list of request offsets. The `firstRequestOffset` footer points to the first request message in the packet. Each request's `nextRequestOffset` field points to the next request, with 0 marking the last request.
4. **Reply ID assignment**: The `ReplyID` is **not** assigned by the bundle. It is written later by `RequestManager::addReplyOrders()` during `NetworkInterface::send()`. The ID is stored in network byte order (`BW_HTONL`).
5. **Packet flag**: The packet's `FLAG_HAS_REQUESTS` flag is set.

### Reply ID Allocation

From `request_manager.cpp`:
- Reply IDs are allocated from a counter initialized to `(timestamp % 100000) + 10101`.
- IDs wrap around at `REPLY_ID_MAX` (1,000,000), resetting to 1.
- Each pending request is stored in a map keyed by reply ID.

### How Replies Work

1. **Responder starts a reply**: Calls `Bundle::startReply(replyID)`.
2. **Special message ID**: The reply uses message ID `0xFF` (`REPLY_MESSAGE_IDENTIFIER`) with `DWORD_LENGTH` (4-byte length prefix).
3. **Reply ID in body**: The `replyID` from the original request is streamed as the first 4 bytes of the reply message body.
4. **Receiver dispatches**: The `RequestManager::handleMessage()` method reads the `replyID` from the reply body, looks up the pending request, and calls the handler.

### Request Message On-Wire Layout

```
+------------+----------+------------+---------------------+------------------+
| Message ID | Length   | Reply ID   | Next Request Offset | Method Arguments |
| (1 byte)   | (varies) | (4 bytes) | (2 bytes)           | (variable)       |
+------------+----------+------------+---------------------+------------------+
```

### Reply Message On-Wire Layout

```
+------+----------+------------+-----------+
| 0xFF | Length   | Reply ID   | Reply Data |
|      | (4 bytes)| (4 bytes)  | (variable) |
+------+----------+------------+-----------+
```

### Cimmeria Request Handling

In Cimmeria's `BundleUnpacker::next()`, when the current message offset matches `nextRequestOffset_`, the unpacker reads:
- `requestId` (uint32) -- the reply ID
- `nextRequestOffset_` (uint16) -- offset to the next request message (0 = last)

The `requestId` is stored on the `Message` object and available via `Message::requestId()`.

## Piggyback Packet Format

Piggyback packets allow embedding previously sent packets inside a new outgoing packet. This is used (in BigWorld only; Cimmeria rejects them) to retransmit reliable messages that may have been lost.

### BigWorld Piggyback Format

Piggyback data is stored in the **footer area** of the packet, read backward from the end (before ACKs/sequence footers). The `FLAG_HAS_PIGGYBACKS` (0x02) flag indicates their presence.

**On-wire layout** (footer, read from end of packet backward):

```
+--------------------+-------------------+---+--------------------+-------------------+
| Packet N data      | Packet N length   |...| Packet 1 data      | Packet 1 length   |
| (packetN_len bytes)| (int16, negative) |   | (packet1_len bytes)| (int16, positive) |
+--------------------+-------------------+---+--------------------+-------------------+
```

**Processing** (from `Packet::processPiggybackPackets()` in `packet.cpp`):

1. Strip an `int16` length from the end of the packet.
2. If the length is **negative**, this is the **last** piggyback packet. Bitwise-NOT the value (`~len`) to get the actual length.
3. If the length is **positive**, there are more piggyback packets to follow.
4. Extract `len` bytes from the end of the packet body as the piggybacked packet's data.
5. Create a new `Packet` from the extracted data and process it as a complete independent packet.
6. Repeat until a negative-length marker is encountered.

**Key properties**:
- Piggyback packets are processed **before** the main packet's messages (they are older).
- Piggybacked packets cannot themselves carry piggybacks.
- Piggybacked packets carry their own flags field but do not have independent ACKs, sequence IDs, or fragment IDs.
- Each `BundlePiggyback` object stores: the original `Packet`, its `Flags`, `SeqNum`, `len` (int16), and a `ReliableVector`.

### Cimmeria Piggyback Status

Cimmeria **does not support** piggyback packets. In `ReceivedPacket::unserialize()`:
```cpp
if (flags_ & FLAG_PIGGYBACK)
{
    WARN_BAD_PACKET("Piggybacked packets are not supported");
    return false;
}
```

The SGW client does not appear to send piggyback packets, and the Cimmeria server does not generate them. The `FLAG_PIGGYBACK` (0x02) bit is defined but treated as an error condition on receive.

## Bundles

A bundle is a logical grouping of messages that may span multiple packets. Bundles provide the unit of reliability.

### Bundle Fragmentation

When a bundle exceeds the maximum packet size, it is fragmented:

```
Fragment 0:  [Header] [Messages...] [Fragment: index=0, total=3]
Fragment 1:  [Header] [Messages...] [Fragment: index=1, total=3]
Fragment 2:  [Header] [Messages...] [Fragment: index=2, total=3]
```

- Maximum fragments per bundle: 64 (`Packet::MaxFragmentsPerBundle`)
- Each fragment has `FLAG_FRAGMENTED` set and carries fragment begin/end sequence IDs
- The receiver reassembles the bundle from fragments before processing messages

### Reliability Model

Mercury supports three reliability levels (from `bundle.hpp`):

| Level | Value | Behavior |
|-------|-------|----------|
| `RELIABLE_NO` | 0 | Not reliable, may be dropped |
| `RELIABLE_DRIVER` | 1 | Reliable; drives the bundle to be sent |
| `RELIABLE_PASSENGER` | 2 | Only sent if a DRIVER is present in the bundle |
| `RELIABLE_CRITICAL` | 3 | Like DRIVER but marks bundle as critical |

In Cimmeria, bundles are either reliable or unreliable. Each channel maintains separate reliable and unreliable bundles:
- `bundle_` -- reliable message accumulator
- `unreliableBundle_` -- unreliable message accumulator

## Acknowledgement Model

### Cimmeria: Individual ACKs Only

Cimmeria uses **individual (selective) ACKs**. Each ACK explicitly names the sequence ID of the packet being acknowledged. There is no cumulative ACK mechanism.

**Evidence from `src/mercury/channel.cpp`**:

**Sending ACKs**: When a reliable packet is received, the receiver adds its sequence ID to the unreliable bundle's ACK list:
```cpp
// In handlePacket(), for FLAG_RELIABLE packets:
unreliableBundle()->addAck(p.sequenceId());
```

ACKs are piggybacked on the next outgoing unreliable bundle. The `Packet::addAcknowledgement()` method appends each individual sequence ID to the packet's ACK vector.

**Receiving ACKs**: The `processAck()` method handles each ACK individually by looking up the sequence ID in the send window:
```cpp
void BaseChannel::processAck(Packet::SequenceId seq)
{
    // Calculate index into send window
    uint32_t swndIndex = seq - (nextSendSequenceId_ - sendWindow_.size());
    // Clear the packet and cancel its resend timer
    sendWindow_[swndIndex].packet.reset();
    deleteResendTimer(sendWindow_[swndIndex].timerId);
    // If head of window was acked, slide window forward
    if (swndIndex == 0) {
        while (!sendWindow_.empty() && sendWindow_[0].packet.get() == nullptr)
            sendWindow_.pop_front();
        // Admit queued packets into newly freed send window slots
    }
}
```

**ACK processing is done even on duplicate/out-of-order packets**: The `handlePacket()` method processes ACKs from all incoming packets regardless of whether the packet's own sequence ID is within the receive window. This prevents lost ACKs from causing unnecessary retransmissions.

### BigWorld: Individual ACKs + Cumulative ACKs

BigWorld supports both individual and cumulative ACKs:

- **Individual ACKs** (`FLAG_HAS_ACKS`, 0x0004): Same as Cimmeria -- explicit per-sequence-ID acknowledgement stored in a `std::set<SeqNum>`.
- **Cumulative ACKs** (`FLAG_HAS_CUMULATIVE_ACK`, 0x0400): A single `SeqNum` in the footer that acknowledges all packets up to and including that sequence ID. Used on internal channels for efficiency. Handled by `Channel::handleCumulativeAck()`.

Cimmeria does not implement `FLAG_HAS_CUMULATIVE_ACK` (the flag does not exist in the Cimmeria `Packet` enum). The `FLAG_INDEXED_CHANNEL` (0x80) occupies the last bit of the 8-bit flags field, leaving no room for BW's 0x0400 flag without expanding to a 16-bit flags field.

## BundleUnpacker Reassembly Algorithm

The `BundleUnpacker` class in `src/mercury/bundle.hpp` / `bundle.cpp` handles reassembling fragmented bundles and extracting individual messages from the packet chain.

### Construction and Fragment Collection

1. **First fragment arrives**: A `BundleUnpacker` is created with the first `ReceivedPacket`. If the packet has `FLAG_FRAGMENTED`, the unpacker reads the fragment range (`firstFragmentId`, `lastFragmentId`) and allocates a vector of `(lastFragment - firstFragment + 1)` slots. The packet is placed at index `(sequenceId - firstFragment)`. For non-fragmented packets, the vector has a single slot.

2. **Subsequent fragments**: `addPacket()` validates that the incoming packet's fragment range matches the unpacker's range, then places it at the correct index. A `receivedFragments_` counter is incremented.

3. **Completeness check**: `isComplete()` returns true when `fragments_.size() == receivedFragments_` -- all slots are filled.

### Message Extraction Algorithm

When the bundle is complete, `beginUnpack()` is called, followed by repeated calls to `next()`:

```
BundleUnpacker::beginUnpack():
  1. Assert all fragments received
  2. Set unpackPacket_ = 0 (start at first fragment)
  3. If first fragment has FLAG_HAS_REQUESTS:
       nextRequestOffset_ = first fragment's firstRequestOffset
```

```
BundleUnpacker::next():
  1. If current packet has no remaining bytes:
       a. If this is the last packet in the chain: return nullptr (end of bundle)
       b. Otherwise: advance to next packet (unpackPacket_++)

  2. Record messageStart = current packet's read offset

  3. Read messageId (uint8) from current packet

  4. Determine message length format:
       a. If messageId >= 0x80: use WORD_LENGTH (entity message, 16-bit length)
       b. Else if messageId < table size: use format from message table
       c. Else: warn "unhandled message", return nullptr

  5. Read length field according to format:
       a. WORD_LENGTH: read uint16 length
       b. DWORD_LENGTH: read uint32 length (reject if > 0xFFFF)
       c. CONSTANT_LENGTH: use fixed length from table

  6. If nextRequestOffset_ == messageStart:
       a. Read requestId (uint32) from stream
       b. Read nextRequestOffset_ (uint16) for next request in chain

  7. Extract message body (three code paths):
       a. FAST PATH: Entire body fits in current packet's remaining bytes
          - Create Message referencing buffer directly, advance offset
       b. NEXT-PACKET PATH: Current packet empty, body fits entirely in next packet
          - Advance to next packet, create Message from next packet's buffer
       c. CROSS-PACKET PATH: Body spans multiple packets
          - Allocate Message buffer of 'length' bytes
          - Loop over packets, copying available bytes until length is reached
          - If packets run out before length is satisfied: warn, return nullptr

  8. Set messageId and requestId on the Message object
  9. Return the Message
```

### Channel Integration

The `BaseChannel` manages `BundleUnpacker` instances in its `unpackers_` vector:

1. **Non-fragmented reliable packet**: A temporary `BundleUnpacker` is created and immediately processed.
2. **Fragmented reliable packet**: The channel searches `unpackers_` for an existing unpacker with the same `firstFragmentId`. If none exists, a new one is created and stored.
3. **Completion trigger**: When an unpacker is complete AND the receive window head matches the unpacker's first fragment, the bundle is processed.
4. **Ordered delivery**: `processBufferedMessages()` walks the receive window from the head. For each contiguous packet/bundle at the window head, it processes the bundle and slides the window forward.

## Channels

A channel is a persistent communication endpoint between two addresses. It maintains ordering, reliability, and flow control state.

### Channel Types

| Type | BigWorld Name | Description |
|------|--------------|-------------|
| Internal | `INTERNAL` | Server-to-server, low latency, low loss |
| External | `EXTERNAL` | Client-to-server, higher latency, higher loss |

Cimmeria uses `BaseChannel` for client connections. There are no indexed channels (used in BW for base-cell entity channels); instead, the Base-Cell link uses a separate TCP-like Mercury channel over `unified_connection`.

### Channel State

From `src/mercury/channel.hpp`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| Receive window | 64 packets | Max buffered out-of-order packets |
| Send window | 45 packets | Max unacknowledged packets in flight |
| Send queue | 1024 packets | Max packets waiting to enter send window |
| Max retries | 20 | Retransmissions before disconnect |
| ACK timeout | 700 ms | Time before retransmission |
| Keepalive interval | 1000 ms | Idle keepalive frequency |
| Burst interval | 200 ms | Minimum time between send bursts |
| Inactivity timeout | Configurable | Disconnect after no packets received |

### Acknowledgement Flow

1. Sender assigns a sequence ID to each reliable packet
2. Receiver processes packet and adds sequence ID to its ACK set
3. ACKs are piggybacked on the next outgoing packet (in the footer)
4. Sender receives ACKs and removes packets from its send window
5. If no ACK received within `AckTimeout`, the packet is retransmitted
6. After `TxMaxRetries` retransmissions, the channel is closed

### Sequence Numbers

- Type: `uint32_t`
- Null value: `0x10000000`
- Sequence space: 28-bit with wraparound (BW uses `SEQ_SIZE = 0x10000000`)
- Comparison uses modular arithmetic to handle wrapping

## Client Event-to-Message ID Mapping

The SGW client uses a named event system (`Event_NetOut_*` / `Event_NetIn_*`) to dispatch network messages. Each event name corresponds to a message that is serialized with a specific message ID on the wire. The message ID is determined by the message's position in the client's message table, not by the event name.

### How Message IDs are Assigned

Message IDs for non-entity messages (0x00-0x7F) are assigned by their **index in the message table** (`Message::Table`). The table is a static array of `Message::Format` structs. The client and server must agree on the table layout.

For entity messages (0x80-0xFF), the message ID encodes the entity method ID as described in the Entity Message ID Encoding section above.

### Client NetOut Events (Client-to-Server)

The following `Event_NetOut_*` events were identified via Ghidra function search (`register_NetOut_*`). These are serialized as messages on the client's outgoing bundle. The exact message ID for each depends on its position in the client's outgoing message table, which is compiled into the binary and not directly recoverable from the `register_NetOut_*` stub functions alone (these functions simply return the event name string for the event system registration).

**Gameplay commands** (~170 events identified):

| Category | Events |
|----------|--------|
| Character | `CreateCharacter`, `DeleteCharacter`, `PlayCharacter`, `LogOff`, `Disconnect`, `ClientReady`, `Respawn` |
| Movement | `Goto`, `GotoLocation`, `GotoXYZ`, `SetCrouched` |
| Combat | `UseAbility`, `useAbilityOnGroundTarget`, `SetTarget`, `SetTargetID`, `Kill`, `callForAid`, `DuelChallenge`, `DuelForfeit`, `DuelResponse` |
| Items | `MoveItem`, `RemoveItem`, `UseItem`, `LootItem`, `GiveItem`, `GiveAmmo`, `RepairItem`, `RepairItems`, `RechargeItem`, `RechargeItems`, `GetItemInfo`, `PurchaseItems`, `SellItems`, `BuybackItems` |
| Missions | `AbandonMission`, `MissionAbandon`, `MissionAdvance`, `MissionAssign`, `MissionClear`, `MissionClearActive`, `MissionClearHistory`, `MissionComplete`, `MissionDetails`, `MissionList`, `MissionListFull`, `MissionReset`, `MissionSetAvailable`, `ShareMission`, `ShareMissionResponse`, `ChosenRewards` |
| Chat | `sendPlayerCommunication`, `ChatJoin`, `ChatLeave`, `ChatList`, `ChatFriend`, `ChatIgnore`, `ChatMute`, `ChatOp`, `ChatKick`, `ChatBan`, `ChatPassword`, `ChatSetAFKMessage`, `ChatSetDNDMessage`, `SendGMShout` |
| Minigames | `StartMinigame`, `EndMinigame`, `MinigameComplete`, `MinigameStartCancel`, `MinigameCallRequest`, `MinigameCallAccept`, `MinigameCallDecline`, `MinigameCallAbort`, `MinigameContactRequest`, `RegisterToMinigameHelp`, `UpdateRegisterToMinigameHelp`, `SpectateMinigame`, `GiveMinigameContact`, `RemoveMinigameContact` |
| Stargate | `DHD`, `onDialGate`, `GiveStargateAddress`, `RemoveStargateAddress` |
| Organizations | `OrganizationCreation`, `OrganizationInvite`, `OrganizationInviteByType`, `OrganizationInviteResponse`, `OrganizationKick`, `OrganizationLeave`, `OrganizationMOTD`, `OrganizationNote`, `OrganizationOfficerNote`, `OrganizationRankChange`, `OrganizationSetRankName`, `OrganizationSetRankPermissions`, `OrganizationTransferCash`, `SquadSetLootMode` |
| Mail | `SendMailMessage`, `RequestMailHeaders`, `RequestMailBody`, `ArchiveMailMessage`, `DeleteMailMessage`, `ReturnMailMessage`, `PayCODForMailMessage`, `TakeCashFromMailMessage`, `TakeItemFromMailMessage` |
| Crafting | `Craft`, `Alloy`, `Research`, `ReverseEngineer`, `SpendAppliedSciencePoint`, `GiveBlueprint` |
| Trading | `TradeProposal`, `TradeRequestCancel`, `TradeLockState` |
| Contacts | `contactListCreate`, `contactListDelete`, `contactListRename`, `contactListAddMembers`, `contactListRemoveMembers`, `contactListFlagsUpdate` |
| Debug/Admin | `GiveXp`, `GiveAbility`, `GiveAllAbilities`, `SetLevel`, `SetHealth`, `SetHealthMax`, `SetFocus`, `SetFocusMax`, `SetSpeed`, `SetGodMode`, `SetNoAggro`, `SetNoXP`, `Kill`, `Despawn`, `Spawn`, `Summon`, `Invisible`, `XRayEyes`, `Users`, `Who`, `ShowPlayer`, `ShowInventory`, `ListItems`, `ListAbilities`, `PrintStats`, `PerfStats`, `PerfStatsByChannel`, `Physics`, various `Debug*` commands |
| System | `SystemOptions`, `PvPOrganizationLeaveResponse`, `onClientChallengeResponse`, `onClientVersion`, `versionInfoRequest`, `elementDataRequest`, `requestItemData`, `onSpaceQueueReadyResponse`, `onSpaceQueuedResponse`, `onStrikeTeamResponse`, `onSpaceQueueStatus`, `TestLOS`, `ToggleCombatLOS`, `ShowNavigation`, `TriggerClientHintedGenericRegion` |

### Client NetIn Events (Server-to-Client)

The following `Event_NetIn_*` events were identified (~130 events). These are deserialized from the server's outgoing message table:

| Category | Events |
|----------|--------|
| Player Data | `onPlayerDataLoaded`, `onCharacterLoadFailed`, `CharacterList`, `CharacterVisuals`, `CharacterCreateFailed`, `RequestCharacterVisuals` |
| Entity | `EntityFlags`, `EntityProperty`, `EntityTint`, `onRemoteEntityCreate`, `onRemoteEntityMove`, `onRemoteEntityRemove`, `onEntityMove`, `onVisible` |
| Combat/Stats | `onEffectResults`, `EffectUserData`, `ConfirmEffect`, `onStatUpdate`, `onStatBaseUpdate`, `onStateFieldUpdate`, `onLevelUpdate`, `onExpUpdate`, `onMaxExpUpdate`, `onMeleeRangeUpdate` |
| Abilities | `KnownAbilitiesUpdate`, `AbilityTreeInfo`, `PetAbilities`, `PetStances`, `PetStanceUpdate` |
| Items | `onUpdateItem`, `onRemoveItem`, `onRefreshItem`, `onContainerInfo`, `CashChanged`, `onActiveSlotUpdate` |
| Missions | `onMissionUpdate`, `onObjectiveUpdate`, `onStepUpdate`, `onTaskUpdate`, `MissionOffer`, `MissionSharedOffer`, `MissionRewards` |
| Chat | `onPlayerCommunication`, `onLocalizedCommunication`, `onSystemCommunication`, `onChatJoined`, `onChatLeft`, `onNickChanged`, `onTellSent` |
| Stores/Trading | `onStoreOpen`, `onStoreUpdate`, `onStoreClose`, `TradeState`, `TradeResults` |
| Mail | `onMailHeaderInfo`, `onMailHeaderRemove`, `onMailRead`, `onSendMailResult` |
| Minigames | `onStartMinigame`, `onEndMinigame`, `onStartMinigameDialog`, `onStartMinigameDialogClose`, `onSpectateList`, `onMinigameRegistrationPrompt`, `onMinigameRegistrationInfo`, `AddOrUpdateMinigameHelper`, `RemoveMinigameHelper`, `MinigameCallDisplay`, `MinigameCallResult`, `MinigameCallAbort`, `ShowMinigameContact` |
| Organizations | `onOrganizationInvite`, `onOrganizationJoined`, `onOrganizationLeft`, `onOrganizationRosterInfo`, `onMemberJoinedOrganization`, `onMemberLeftOrganization`, `onMemberRankChangedOrganization`, `onOrganizationNameUpdate`, `onOrganizationExperienceUpdate`, `onOrganizationMOTDUpdate`, `onOrganizationNoteUpdate`, `onOrganizationOfficerNoteUpdate`, `onOrganizationCashUpdate`, `onOrganizationRankUpdate`, `onOrganizationRankNameUpdate`, `onLaunchOrganizationCreation` |
| Stargate | `onStargatePassage`, `setupStargateInfo`, `updateStargateAddress`, `StargateRotationOverride`, `StargateTriggerFailed`, `onDisplayDHD`, `onDHDReply`, `ShowNavigation` |
| World | `onClientMapLoad`, `MapInfo`, `ResetMapInfo`, `setupWorldParameters`, `onTimeofDay`, `onArchetypeUpdate`, `onKismetEventSetUpdate`, `onCookedDataError`, `AddClientHintedGenericRegion`, `ClearClientHintedGenericRegions` |
| UI/Dialog | `DialogDisplay`, `InitialInteraction`, `InteractionType`, `LootDisplay`, `onTrainerOpen`, `onCraftingRespecPrompt`, `PlayMovie`, `StopMovie`, `onBeginAidWait`, `onEndAidWait`, `onErrorCode` |
| Contacts | `onContactListUpdate`, `onContactListDelete`, `onContactListAddMembers`, `onContactListRemoveMembers`, `onContactListEvent` |
| PvP/Duels | `onDuelChallenge`, `onDuelEntitiesSet`, `onDuelEntitiesRemove`, `onDuelEntitiesClear`, `PvPOrganizationLeaveRequest`, `onSquadLootType`, `onStrikeTeamUpdate`, `onAggressionOverrideUpdate`, `onAggressionOverrideCleared` |
| System | `onVersionInfo`, `onSequence`, `onOverridePerfStatsRate`, `onClientChallenge`, `onSpaceQueueReady`, `onSpaceQueued`, `BeingAppearance`, `onBeingNameUpdate`, `onBeingNameIDUpdate`, `onTargetUpdate`, `onSetTarget`, `onExtraNameUpdate`, `onAlignmentUpdate`, `onFactionUpdate`, `onStaticMeshNameUpdate`, `onThreatenedMobsUpdate`, `onDisableShowPath`, `onShowPath`, `onShowCommandWaypoints`, `onRingTransporterList` |
| Vaults | `onVaultOpen`, `onTeamVaultOpen`, `onCommandVaultOpen`, `onClearOrgVaultInventory` |
| Crafting | `onUpdateDiscipline`, `onUpdateRacialParadigmLevel`, `onUpdateKnownCrafts`, `onUpdateCraftingOptions`, `onDisciplineRespec` |
| Marketplace | `BMOpen`, `BMAuctions`, `BMAuctionUpdate`, `BMAuctionRemove`, `BMError` |

## Encryption

After the initial login handshake, all Mercury traffic is encrypted:

- **Algorithm**: AES-256-CBC
- **Integrity**: HMAC-MD5 (16 bytes appended to each packet)
- **Key**: 32 bytes derived from the 64-character hex session key
- **Padding**: PKCS #7

The encryption is applied as a `MessageFilter` that wraps each packet before sending and unwraps on receipt. This reduces the effective payload by 32 bytes (16 for HMAC + up to 16 for AES padding).

See `src/mercury/encryption_filter.hpp` for implementation details.

## Base-Cell Protocol

The BaseApp and CellApp communicate using a dedicated Mercury-like protocol over TCP (not the same as the client protocol). See `src/mercury/base_cell_messages.hpp`.

Key differences from client protocol:
- Uses handshake with magic values (`0xD293BF1E` for BaseApp, `0x70CDB965` for CellApp)
- Protocol version: 391
- Fixed message IDs (0x00-0x11 for Cell->Base, 0x00-0x09 for Base->Cell)
- Not encrypted (internal network only)

## Related Documents

- [Login Handshake](login-handshake.md) -- Initial connection and encryption setup
- [Position Updates](position-updates.md) -- Movement update message format
- [Entity Property Sync](entity-property-sync.md) -- Property change protocol
- [Connection Flow](../connection-flow.md) -- End-to-end login sequence

## TODO

- [x] Confirm exact byte ordering of Cimmeria packet header fields via Ghidra packet capture --> Cimmeria uses native (little-endian) byte order for all fields; BW uses network (big-endian) order with BW_HTONS/BW_HTONL
- [x] Document the request-reply protocol (request ID assignment, reply routing) --> ReplyID (int32) assigned by RequestManager during send, message ID 0xFF for replies, linked list of request offsets in packet
- [x] Document the piggyback packet format (how packets are embedded in other packets) --> BW embeds packets in footer area with int16 length markers (negative = last); Cimmeria rejects piggybacks as unsupported
- [x] Verify whether Cimmeria uses cumulative ACKs or individual ACKs --> Cimmeria uses individual (selective) ACKs only; BW additionally supports cumulative ACKs (FLAG_HAS_CUMULATIVE_ACK 0x0400)
- [x] Map the exact message IDs used by the client for each `Event_NetOut` / `Event_NetIn` --> ~170 NetOut and ~130 NetIn events catalogued; IDs assigned by message table index (not recoverable from register stubs alone)
- [x] Document the `BundleUnpacker` reassembly algorithm in detail --> Fragment collection by sequence range, three-path message extraction (single-packet, next-packet, cross-packet), ordered delivery via receive window
