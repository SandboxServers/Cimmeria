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

- [ ] Confirm exact byte ordering of Cimmeria packet header fields via Ghidra packet capture
- [ ] Document the request-reply protocol (request ID assignment, reply routing)
- [ ] Document the piggyback packet format (how packets are embedded in other packets)
- [ ] Verify whether Cimmeria uses cumulative ACKs or individual ACKs
- [ ] Map the exact message IDs used by the client for each `Event_NetOut` / `Event_NetIn`
- [ ] Document the `BundleUnpacker` reassembly algorithm in detail
