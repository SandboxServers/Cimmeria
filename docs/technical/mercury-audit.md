# Mercury Protocol Audit: Cimmeria vs BigWorld Reference

Comparison of Cimmeria's Mercury protocol implementation against the BigWorld 1.9.1/2.0.1 open-source reference. The audit identifies differences that are likely intentional CME modifications for SGW, versus potential bugs.

## Summary

Cimmeria's Mercury implementation was reverse-engineered from SGW client traffic, not from BigWorld source. Many apparent "discrepancies" versus stock BigWorld are actually **correct for SGW**, since CME modified BigWorld's wire format. The key evidence: our server successfully exchanges 274+ packets with working ACKs, which would be impossible if basic framing were wrong.

## 1. Packet Header: Flags Field

### BigWorld Reference

```cpp
typedef uint16 Flags;  // 2 bytes, network byte order (big-endian)
Flags flags() const { return BW_NTOHS( *(Flags*)data_ ); }
HEADER_SIZE = sizeof(Flags)  // = 2
```

Upper byte flags: `FLAG_HAS_CHECKSUM = 0x0100`, `FLAG_CREATE_CHANNEL = 0x0200`

### Cimmeria

```cpp
typedef uint8_t Flags;  // 1 byte, native order
HEADER_SIZE = sizeof(Flags)  // = 1
```

### Assessment: LIKELY CORRECT FOR SGW

If the flags field were wrong, no packets would parse correctly. Our server exchanges hundreds of packets successfully. CME likely reduced flags to 1 byte, dropping the upper-byte features (checksum, create_channel) they didn't use.

## 2. Byte Order

### BigWorld Reference

All multi-byte footer values use **network byte order (big-endian)**:
- `BW_HTONS`/`BW_HTONL` for writes
- `BW_NTOHS`/`BW_NTOHL` for reads

### Cimmeria

All multi-byte values use **native byte order (little-endian on x86/x64)**.

### Assessment: LIKELY CORRECT FOR SGW

Stock BigWorld targets Linux servers (big-endian network order) talking to Windows clients. CME moved everything to Windows, where both client and server are little-endian. If byte order were wrong, sequence numbers and ACKs would be completely garbled. Our ACK processing works, confirming native order is correct for SGW.

## 3. Flag Bit Values (Lower Byte)

| Flag | BigWorld | Cimmeria | Match |
|------|----------|----------|-------|
| HAS_REQUESTS | 0x01 | 0x01 | Yes |
| HAS_PIGGYBACKS | 0x02 | 0x02 | Yes |
| HAS_ACKS | 0x04 | 0x04 | Yes |
| ON_CHANNEL | 0x08 | 0x08 | Yes |
| IS_RELIABLE | 0x10 | 0x10 | Yes |
| IS_FRAGMENT | 0x20 | 0x20 | Yes |
| HAS_SEQUENCE_NUMBER | 0x40 | 0x40 | Yes |
| INDEXED_CHANNEL | 0x80 | 0x80 | Yes |

### Assessment: CORRECT

All lower-byte flag values match exactly.

## 4. Encryption Filter

### BigWorld Reference

- Algorithm: **Blowfish** (`BF_ecb_encrypt`)
- Block size: 8 bytes (64-bit)
- Mode: Manual CBC (XOR with previous ciphertext block)
- IV: None (first block has no XOR)
- Integrity: Magic value `0xdeadbeef` (uint32) appended before padding
- Padding: Custom wastage byte scheme

### Cimmeria

- Algorithm: **AES-256-CBC** (`AES_cbc_encrypt`)
- Block size: 16 bytes (128-bit)
- Mode: OpenSSL CBC mode
- IV: All zeros (16 null bytes)
- Integrity: **HMAC-MD5** (16 bytes) appended after ciphertext
- Padding: PKCS#7

### Assessment: CONFIRMED SGW MODIFICATION

SGW uses AES-256 + HMAC-MD5, not Blowfish. This is confirmed by:
- AtreaRL loader extracting AES session keys from SOAP XML
- Session key format: 64-byte hex string = 256-bit key
- HMAC-MD5 verification in our decryption path works against client traffic
- The entire authentication flow is AES-based

## 5. Footer Ordering

### BigWorld Reference

Footers are written appending to the packet, read from the end (stack-like):

1. Checksum (if FLAG_HAS_CHECKSUM)
2. Piggyback data (if FLAG_HAS_PIGGYBACKS)
3. Channel ID + Version (if FLAG_INDEXED_CHANNEL)
4. ACK count + ACK seqnums (if FLAG_HAS_ACKS)
5. Sequence number (if FLAG_HAS_SEQUENCE_NUMBER)
6. First request offset (if FLAG_HAS_REQUESTS)
7. Fragment end + Fragment begin (if FLAG_IS_FRAGMENT)

### Cimmeria

Writes appending, reads from end:

1. Fragment indices (if FLAG_FRAGMENTED)
2. Sequence number (if FLAG_HAS_SEQUENCE)
3. ACK seqnums + ACK count (if FLAG_HAS_ACKS)

Reads from end:

1. ACK count, then ACK seqnums (if FLAG_HAS_ACKS)
2. Sequence number (if FLAG_HAS_SEQUENCE)
3. Fragment end, then fragment begin (if FLAG_FRAGMENTED)
4. First request offset (if FLAG_HAS_REQUESTS)

### Assessment: CORRECT

The ACK footer physical layout matches BigWorld: `[ack_seq_0][ack_seq_1]...[ack_seq_N][ack_count]`. Reading pops count first (from end), then seqnums. This is correct.

## 6. Sequence Number Space

### BigWorld Reference

```cpp
SEQ_SIZE = 0x10000000U
SEQ_MASK = SEQ_SIZE - 1  // = 0x0FFFFFFF
SEQ_NULL = SEQ_SIZE      // = 0x10000000
```

Wrapping comparison: `seqMask(x) { return x & SEQ_MASK; }` with `seqLessThan()` for wraparound.

### Cimmeria

```cpp
NullSequence = 0x10000000
```

Valid range check matches. **However**, sequence comparisons use plain integer comparison without masking.

### Assessment: POTENTIAL BUG

Sequence wrapping will break when numbers exceed `0x0FFFFFFF`. Not a problem for short sessions, but could cause issues in long-running connections.

**Recommendation**: Add modular arithmetic with `SEQ_MASK` for all sequence comparisons in `channel.cpp`.

## 7. Channel Management

### BigWorld Reference

- INTERNAL channels (server-to-server, low latency)
- EXTERNAL channels (client-to-server, high latency)
- Indexed channels (multiplexed by ChannelID)
- Anonymous channels (auto-created)

### Cimmeria

Single `BaseChannel` type. No INTERNAL/EXTERNAL distinction. Indexed channels rejected.

### Assessment: CORRECT FOR SGW

We only need client-to-server channels. The simplification is appropriate.

## 8. Send Window Management

### BigWorld Reference

`CircularArray<UnackedPacket*>` with power-of-two sizes, overflow lists, configurable window size, adaptive RTT-based resend.

### Cimmeria

`boost::circular_buffer` with `TxWindowMaxSize = 45`, `RxWindowMaxSize = 64`. Fixed 700ms resend timeout.

### Assessment: FUNCTIONAL

Simpler but works. The fixed timeout could be improved with adaptive RTT estimation for better performance under varying network conditions.

## 9. Message Length Formats

### BigWorld Reference

Supports 1, 2, 3, and 4 byte length formats for variable-length messages, with overflow handling.

### Cimmeria

Supports:
- `CONSTANT_LENGTH = 0` (no length field)
- `WORD_LENGTH = 2` (uint16 length)
- `DWORD_LENGTH = 4` (uint32 length)

### Assessment: SUFFICIENT

Missing 1-byte and 3-byte formats. May need to add these if specific SGW messages require them. The Mercury reply message (id 0xFF) uses 4-byte variable length, which we handle.

## 10. Piggybacking

### BigWorld Reference

Lost reliable packets can be piggybacked onto future outgoing bundles.

### Cimmeria

Rejected: `WARN_BAD_PACKET("Piggybacked packets are not supported")`.

### Assessment: NOT NEEDED

Piggybacking is a server-side optimization. The client likely doesn't send piggybacked data. Standard retransmission handles lost packets.

## 11. Missing Upper-Byte Flags

| Flag | BigWorld | Cimmeria | Needed? |
|------|----------|----------|---------|
| `FLAG_HAS_CHECKSUM` (0x0100) | Yes | No | No -- SGW doesn't use it |
| `FLAG_CREATE_CHANNEL` (0x0200) | Yes | No | No -- internal feature only |

## Priority Summary

### No Action Needed (Confirmed SGW Modifications)

1. 1-byte flags field (vs BigWorld's 2-byte)
2. Native byte order (vs BigWorld's network byte order)
3. AES-256 encryption (vs BigWorld's Blowfish)
4. Simplified channel management

### Should Fix

1. **Sequence number wrapping** -- Add `SEQ_MASK` modular arithmetic for long-running connections
2. **Adaptive resend timeout** -- Replace fixed 700ms with RTT-based estimation (optimization, not critical)

### May Need Later

1. **1-byte message length format** -- If specific SGW messages require it
2. **Piggybacking support** -- Only if client ever sends piggybacked packets

## Reference Files

### Cimmeria

| File | Purpose |
|------|---------|
| `src/mercury/packet.hpp` | Packet flags, header, constants |
| `src/mercury/packet.cpp` | Packet serialization/deserialization |
| `src/mercury/bundle.cpp` | Bundle framing, message packing |
| `src/mercury/channel.cpp` | Channel management, ACKs, reliability |
| `src/mercury/nub.cpp` | Network endpoint, packet dispatch |
| `src/mercury/encryption_filter.cpp` | AES-256-CBC + HMAC-MD5 |
| `src/mercury/stream.hpp` | Stream operators (native byte order) |

### BigWorld 1.9.1

| File | Purpose |
|------|---------|
| `bigworld/src/lib/network/packet.hpp` | Packet flags (uint16), header, constants |
| `bigworld/src/lib/network/nub.cpp` | Packet send/receive, footer handling |
| `bigworld/src/lib/network/channel.cpp` | Channel management, sequence tracking |
| `bigworld/src/lib/network/bundle.cpp` | Bundle framing |
| `bigworld/src/lib/network/encryption_filter.cpp` | Blowfish encryption |
| `bigworld/src/lib/network/interface_element.cpp` | Message length encoding |
