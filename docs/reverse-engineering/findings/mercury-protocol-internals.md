# Mercury Protocol Internals — Client Binary Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation (48 functions renamed)
> **Confidence**: HIGH — full call chains decompiled, constants extracted, RTTI validated

---

## Architecture Overview

SGW uses BigWorld's **Mercury** networking layer with the legacy **Nub** naming convention (no "NetworkInterface" strings). Protocol is UDP with custom reliability, sequencing, fragmentation, piggybacking, and acknowledgments.

### Outgoing Call Chain
```
ServerConnection::send (0x00dd8930)
  → Mercury::Channel::send (0x01576f90)
    → Mercury::Bundle::finalise (0x0157a7a0)
    → Mercury::Nub::send (0x01582160)
      → Mercury::Nub::writeConnection (0x01583a90) [sendto()]
```

### Incoming Call Chain
```
Mercury::Nub::processPendingEvents (0x01581ab0) [recvfrom()]
  → Mercury::Nub::processFilteredPacket (0x01580ad4)
    → Mercury::Nub::processFilteredPacket_inner (0x01580840)
      → Mercury::Nub::processPacket (0x0157fd20) [fragment reassembly]
        → Mercury::Nub::processOrderedPacket (0x0157c820) [dispatch by msg ID]
          → Mercury::Nub::handleMessage (0x0157bd30) [reply matching]
```

---

## 4 Target Functions (Previously TODO)

### 1. Mercury::Channel::send — `0x01576f90`

Channel-level outgoing dispatch. Checks if bundle has data, calls `Bundle::finalise`, delegates to `Nub::send_setupReplyHandlers` (`0x0157ec70`). Actual UDP write goes through `Nub::send` → `Nub::writeConnection` → `sendto()`.

### 2. Mercury::Nub::processOrderedPacket — `0x0157c820`

Incoming message processing. After packet reassembly and sequencing, iterates messages in a bundle, looks up handler by message ID from interface table, dispatches to registered handler. Handles corrupted headers, unhandled IDs, verifies all data consumed.

Related: `Nub::_processMessage` (`0x0157e480`) handles Channel register/deregister. `Nub::handleMessage` (`0x0157bd30`) matches reply IDs to pending request handlers.

### 3. Mercury::Bundle::newMessage — `0x0157ac90`

Begins constructing a new message. Takes InterfaceElement + payload length. Computes header size via `0x0158aa40`, increments message count, checks space (max 0x5AD bytes), allocates new packet if needed via `Bundle::reserve`, writes message ID byte, returns payload pointer.

Wrappers:
- `startMessage_fixed` (`0x0157ad80`) — fixed-length messages
- `startMessage_request` (`0x0157adc0`) — request messages expecting replies

### 4. Mercury::Bundle::addBlob — `0x0157a990`

Adds raw data to bundle. `(this, void* data, size_t length)`. Copies data into current packet buffer, auto-allocates new packets when full. Handles data spanning multiple packets by splitting at boundaries.

---

## Protocol Constants

| Constant | Value | Description |
|----------|-------|-------------|
| Max packet payload | `0x5AD` (1453 bytes) | Standard Mercury UDP packet size |
| Sequence number mask | `0x0FFFFFFF` | 28-bit sequence numbers |
| Null sequence number | `0x10000000` | Sentinel value |

### Packet Flags Byte (offset 0x54 in packet struct)

| Bit | Mask | Flag |
|-----|------|------|
| 0 | 0x01 | `FLAG_HAS_FIRST_REQUEST_OFFSET` |
| 1 | 0x02 | `FLAG_HAS_PIGGYBACKS` |
| 2 | 0x04 | `FLAG_HAS_ACKS` |
| 3 | 0x08 | `FLAG_ON_CHANNEL` |
| 4 | 0x10 | `FLAG_IS_RELIABLE` |
| 5 | 0x20 | `FLAG_HAS_SEQUENCE_NUMBER` |
| 6 | 0x40 | `FLAG_HAS_REQUESTS` |
| 7 | 0x80 | `FLAG_IS_FRAGMENT` |

---

## All Mercury Functions (48 renamed in Ghidra)

### Bundle

| Address | Function | Description |
|---------|----------|-------------|
| `0x0157aa40` | `Bundle__Bundle` | Constructor |
| `0x0157a2f0` | `Bundle__dtor` | Destructor — frees packets, piggybacks, reply handlers |
| `0x0157a440` | `Bundle__clear` | Reset state, allocate first packet |
| `0x0157ac90` | `Bundle__newMessage` | Start new message (write msg ID, compute header) |
| `0x0157ad80` | `Bundle__startMessage_fixed` | Fixed-length message wrapper |
| `0x0157adc0` | `Bundle__startMessage_request` | Request message (reserves reply handler space) |
| `0x0157a150` | `Bundle__endMessage` | End message (compress length into header) |
| `0x0157a0a0` | `Bundle__endPacket` | End packet (reserve footer space) |
| `0x0157a7a0` | `Bundle__finalise` | Finalize for sending (set flags, iterate packets) |
| `0x0157a990` | `Bundle__addBlob` | Add raw data (memcpy with auto-split) |
| `0x0157a5d0` | `Bundle__reserve` | Reserve N bytes (new packet if needed) |
| `0x0157ad40` | `Bundle__reserveInline` | Fast-path reserve |

### Nub (NetworkInterface)

| Address | Function | Description |
|---------|----------|-------------|
| `0x015841d0` | `Nub__Nub` | Constructor — socket, network thread, timers |
| `0x01582160` | `Nub__send` | Send bundle (serialize headers, write) |
| `0x01581ab0` | `Nub__processPendingEvents` | Main recv loop (recvfrom) |
| `0x01580840` | `Nub__processFilteredPacket_inner` | Parse flags, acks, piggybacks, seq# |
| `0x01580ad4` | `Nub__processFilteredPacket` | Outer packet processing |
| `0x0157fd20` | `Nub__processPacket` | Fragment reassembly → dispatch |
| `0x0157c820` | `Nub__processOrderedPacket` | Dispatch messages to handlers by ID |
| `0x0157bd30` | `Nub__handleMessage` | Match reply IDs to request handlers |
| `0x0157e480` | `Nub___processMessage` | Channel register/deregister |
| `0x01583a90` | `Nub__writeConnection` | sendto() + byte/packet counters |
| `0x01583440` | `Nub__addListeningSocket` | Create UDP socket, bind, register |
| `0x0157e920` | `Nub__registerChannel` | Register Channel with Nub |
| `0x0157eb00` | `Nub__deregisterChannel` | Unregister Channel |
| `0x0157db80` | `Nub___processMessage_removeChannel` | Remove channel during processing |
| `0x01580620` | `Nub__initConnectionMap` | Initialize connection lookup map |
| `0x0157ec70` | `Nub__send_setupReplyHandlers` | Set up reply handlers for requests |

### Channel / Connection

| Address | Function | Description |
|---------|----------|-------------|
| `0x01577960` | `Channel__Channel` | Constructor (init interface table, traits) |
| `0x01576f90` | `Channel__send` | Channel-level send |
| `0x01583680` | `Connection__send` | Connection-level send |

### InterfaceElement

| Address | Function | Description |
|---------|----------|-------------|
| `0x0158acc0` | `InterfaceElement__compressLength` | Compress variable-length header |
| `0x0158b770` | `InterfaceElement__expandLength` | Read variable-length header |
| `0x0158b120` | `InterfaceElement__compressLength_write` | Write length (1/2/3/4 byte) |

### UnAckedHandler (Reliability)

| Address | Function | Description |
|---------|----------|-------------|
| `0x0158b980` | `UnAckedHandler__sendIfReady` | Check ack bundle ready, send |
| `0x0158bbc0` | `UnAckedHandler__sendAckBundle` | Build and send ack-only bundle |
| `0x0158c420` | `UnAckedHandler__checkResendTimers` | Check timeouts, trigger resends |
| `0x0158cba0` | `UnAckedHandler__queueAckForPacket` | Queue ack for reliable packet |
| `0x0158c5d0` | `UnAckedHandler__resetLocalPart` | Reset ack state |

### MGMPacket / Packet / ChannelInternal

| Address | Function | Description |
|---------|----------|-------------|
| `0x01589290` | `MGMPacket__read` | Deserialize MGM packet |
| `0x01585260` | `MGMPacket__write` | Serialize MGM packet |
| `0x0157df90` | `Packet__release` | Free packet |
| `0x0158d050` | `ChannelInternal__resetRemotePart` | Reset (free fragments, buffers) |

### ServerConnection Wrappers

| Address | Function | Description |
|---------|----------|-------------|
| `0x00dd8930` | `ServerConnection__send` | Game-level send |
| `0x00dd9280` | `ServerConnection__enableEntities` | Enable entities message |
| `0x00de2a90`–`0x00de2b80` | `ServerConnection__startMessage_1-4` | Start message variants |
| `0x00de2bd0` | `ServerConnection__startMessage_and_reserve` | Start + reserve |

---

## RTTI Classes

Mercury::Channel, Mercury::Bundle, Mercury::Nub, Mercury::Packet, Mercury::ChannelInternal, Mercury::ClientMessage, Mercury::ClientNetMessage, Mercury::ClientOutgoingMessage, Mercury::ClientIncomingMessage, Mercury::ClientExceptionMessage, Mercury::ClientChannelRegMessage, Mercury::ClientInactivityDetectMessage, Mercury::ClientResetMessage, Mercury::ClientChannelRequestStatsMessage, Mercury::ClientChannelStatMessage, Mercury::BaseNub, Mercury::BundlePrimer, Mercury::InputMessageHandler, Mercury::ReplyMessageHandler, Mercury::TimerExpiryHandler, Mercury::NubException, Mercury::PacketFilter, Mercury::Nub::Connection, Mercury::Nub::ReplyHandlerElement, Mercury::ProcessMessageHandler@BaseNub, Mercury::QueryInterfaceHandler@BaseNub

---

## Implications for Cimmeria

1. **Mercury uses Nub naming** throughout — no "NetworkInterface" in the binary.
2. **Packet flags byte** at offset 0x54 controls all optional features (piggybacks, acks, fragments, etc.).
3. **Max packet size is 1453 bytes** — messages larger than this are automatically fragmented.
4. **28-bit sequence numbers** with windowed ack tracking for reliability.
5. **Variable-length message headers** — InterfaceElement compresses length into 1-4 bytes.
6. **UnAckedHandler** manages resend timers — Cimmeria should implement equivalent timeout/resend logic.
7. **ServerConnection** wraps Channel::send with game-specific message construction helpers.
