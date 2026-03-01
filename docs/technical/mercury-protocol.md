# Mercury Protocol

CME's proprietary networking layer, built on top of BigWorld Technology's transport. Mercury handles all client-server communication in Stargate Worlds.

## Class Hierarchy (from RTTI)

All RTTI addresses are in the sgw.exe `.rdata` section.

### Core Transport
| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `Mercury::BaseNub` | 0x01e919ac | Base network endpoint |
| `Mercury::Nub` | 0x01e91b88 | Full network endpoint (extends BaseNub) |
| `Mercury::Nub::NetworkTask` | — | Async network task (thread pool) |
| `Mercury::Bundle` | 0x01e919e8 | Message container (multiple messages per bundle) |
| `Mercury::Packet` | 0x01e91dc8 | Raw network packet |
| `Mercury::Channel` | 0x01e9188c | Logical connection channel |
| `Mercury::ChannelInternal` | 0x01e91e10 | Internal channel implementation |
| `Mercury::Nub::Connection` | 0x01e91b60 | Per-peer connection state |
| `Mercury::PacketFilter` | 0x01e93b2c | Packet filter (encryption, compression) |

### Message Types
| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `Mercury::ClientMessage` | 0x01e91e38 | Base client message |
| `Mercury::ClientExceptionMessage` | 0x01e91e5c | Exception/error message |
| `Mercury::ClientNetMessage` | 0x01e91e8c | Network-layer message |
| `Mercury::ClientOutgoingMessage` | 0x01e91eb4 | Outbound message |
| `Mercury::ClientIncomingMessage` | 0x01e91f0c | Inbound message |
| `Mercury::ClientChannelRegMessage` | 0x01e91f3c | Channel registration |
| `Mercury::ClientInactivityDetectMessage` | 0x01e91f70 | Keepalive/timeout detection |
| `Mercury::ClientResetMessage` | 0x01e91f9c | Connection reset |
| `Mercury::ClientChannelRequestStatsMessage` | 0x01e91fd4 | Channel statistics request |
| `Mercury::ClientChannelStatMessage` | — | Channel statistics response |

### Handler Types
| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `Mercury::InputMessageHandler` | 0x01e51fa0 | Incoming message dispatcher |
| `Mercury::ReplyMessageHandler` | 0x01e51f24 | Reply correlation handler |
| `Mercury::TimerExpiryHandler` | 0x01e51f50 | Timer expiry callback |
| `Mercury::BundlePrimer` | 0x01e51f7c | Bundle initialization |
| `Mercury::NubException` | 0x01e53394 | Network exception |
| `Mercury::BaseNub::ProcessMessageHandler` | 0x01e91900 | Message processing |
| `Mercury::BaseNub::QueryInterfaceHandler` | 0x01e91934 | Interface discovery |
| `Mercury::Nub::ReplyHandlerElement` | 0x01e91a84 | Reply handler registration |

### Threading
| Class | RTTI Address | Purpose |
|-------|-------------|---------|
| `CME::Thread<NetworkTask, Win32ThreadTraits, Win32ThreadEx>` | 0x01e91a08 | Network thread |
| `tbb::concurrent_queue<RefCountedObj<ClientMessage>>` | 0x01e91ab8 | Thread-safe message queue |

## Debug Strings (Evidence)

These debug/error strings reveal internal Mercury behavior:

### Bundle Operations
```
Mercury::Bundle::newMessage: tried to add a message with an unknown length format %d
```
Address: 0x01b174b8 — Messages have typed length formats; invalid format triggers this error.

### Nub Operations
```
Mercury::Nub::Nub                                        @ 0x01b17768
Mercury::Nub::addListeningSocket                         @ 0x01b17870
Mercury::Nub::writeConnection                            @ 0x01b17de8
Mercury::Nub::handleMessage: received the wrong kind of message!
Mercury::Nub::handleMessage: received a corrupted reply message (length %d)!
Mercury::Nub::handleMessage: Couldn't find handler for reply id 0x%08x (maybe it timed out?)
Mercury::Nub::handleMessage: Got reply to request %d from unexpected source %s
```
Addresses: 0x01b18be0 - 0x01b18cd8

### InterfaceElement (Message Format)
```
Mercury::InterfaceElement::compressLength( %s ): length %d exceeds maximum of length format %d
Mercury::InterfaceElement::compressLength( %s ): data not in any packets
Mercury::InterfaceElement::compressLength( %s ): head not in packets
Mercury::InterfaceElement::compressLength( %s ): Fixed length message has wrong length (%d instead of %d)
Mercury::InterfaceElement::compressLength( %s ): Possible overflow in length (%d bytes) for variable length message
Mercury::InterfaceElement::compressLength( %s ): Unrecognised length format %d
Mercury::InterfaceElement::expandLength( %s ): Overflow in calculating length of variable message!
Mercury::InterfaceElement::expandLength( %s ): unrecognised length format %d
```
Addresses: 0x01b19710 - 0x01b19b50

## Message Framing

From the debug strings, Mercury messages use a length-prefix format system:
1. Messages are contained within **Bundles**
2. Each message has a **length format** identifier (at least 3 types based on error variants)
3. Fixed-length messages validate exact byte count
4. Variable-length messages use compressed length encoding
5. The `compressLength` / `expandLength` pair handles serialization

## Threading Model

Mercury uses a dedicated network thread (via CME::Win32ThreadEx) with:
- Intel TBB `concurrent_queue` for cross-thread message passing
- `RefCountedObj<ClientMessage>` wrapper for safe message lifecycle
- `NetworkTask` runnable for the network thread's main loop

## Connection Flow

The Nub handles connections through:
1. `addListeningSocket` — Opens a UDP socket for incoming traffic
2. `writeConnection` — Establishes outbound connections
3. `handleMessage` — Dispatches incoming messages to registered handlers
4. Reply correlation via request IDs (0x%08x format)
5. Timeout detection for pending replies

## Server Topology Integration

Mercury interfaces with BigWorld's server topology:

### BaseApp Interface
| String | Address | Purpose |
|--------|---------|---------|
| `BaseAppExtInterface` | 0x019d086c | External interface definition |
| `baseAppLogin` | 0x019d0880 | Login RPC to BaseApp |
| `restoreBaseApp` | 0x019d0f50 | Reconnection to BaseApp |
| `BaseAppLoginHandler` (RTTI) | 0x01e533f8 | Login handler class |

### Server Roles
From `LoginMessage_` error codes:
- **LoginApp**: Initial authentication
- **BaseAppMgr**: BaseApp allocation and load balancing
- **BaseApp**: Player session host
- **CellApp**: Spatial simulation
- **DBMgr**: Database operations

### Supersede Method
```
only baseApp and cellApp can call supersede method. Ignored
```
Address: 0x01b1a8d4 — Indicates entity migration between BaseApp and CellApp.
