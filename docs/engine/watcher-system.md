# BigWorld Watcher System

> **Last updated**: 2026-03-01
> **RE Status**: Documented from BigWorld 2.0.1 source; not implemented in Cimmeria
> **Sources**: `external/engines/BigWorld-Engine-2.0.1/src/lib/server/watcher_protocol.hpp`, `external/engines/BigWorld-Engine-2.0.1/src/lib/network/watcher_*.hpp`, `external/engines/BigWorld-Engine-2.0.1/src/lib/cstdmf/watcher.hpp`

---

## Overview

The Watcher system is BigWorld's built-in monitoring and debugging infrastructure. It provides a hierarchical tree of observable values that can be read (and sometimes written) remotely at runtime. Think of it as a live `/proc` filesystem for game servers.

Cimmeria does **not** currently implement the Watcher system. This document describes the BigWorld reference implementation for future use.

## Architecture

### Watcher Tree

The watcher tree is a hierarchical namespace of observable values:

```
/
+-- stats/
|   +-- numEntities
|   +-- numPlayers
|   +-- ticksPerSecond
|   +-- bandwidth/
|       +-- bitsPerSecondIn
|       +-- bitsPerSecondOut
+-- entities/
|   +-- <entityId>/
|       +-- position
|       +-- properties/
|           +-- health
|           +-- name
+-- network/
|   +-- channels/
|   +-- packetsPerSecond
+-- config/
    +-- tickRate
    +-- maxPlayers
```

### Watcher Types

From `cstdmf/watcher.hpp`:

| Type | Description |
|------|-------------|
| `DataWatcher` | Monitors a C++ variable directly |
| `MemberWatcher` | Monitors a class member variable |
| `FunctionWatcher` | Calls a function to get the current value |
| `DirectoryWatcher` | Container for sub-watchers (tree node) |
| `CallableWatcher` | Read-write watcher that can be set remotely |
| `ProfileWatcher` | Performance profiling data |
| `Sequence<>Watcher` | Monitors containers (vectors, maps) |

### Watcher Modes

| Mode | Description |
|------|-------------|
| `WT_READ_ONLY` | Value can only be read |
| `WT_READ_WRITE` | Value can be read and modified remotely |
| `WT_DIRECTORY` | This node is a directory containing sub-watchers |

## Watcher Protocol

### Protocol Version 2

From `watcher_protocol.hpp`, the watcher protocol uses a binary format:

```
WatcherProtocolDecoder methods:
    decode()          -- Decode a complete watcher response
    decodeNext()      -- Decode the next value in a stream
    readSize()        -- Read a size field

Value type handlers:
    intHandler()      -- Integer values
    uintHandler()     -- Unsigned integer values
    floatHandler()    -- Floating point values
    boolHandler()     -- Boolean values
    stringHandler()   -- String values
    tupleHandler()    -- Tuple/compound values
```

### Network Transport

The watcher system uses a dedicated network channel separate from game traffic:

| Component | File | Description |
|-----------|------|-------------|
| `WatcherNub` | `network/watcher_nub.hpp` | Dedicated socket for watcher queries |
| `WatcherConnection` | `network/watcher_connection.hpp` | Handles watcher client connections |
| `WatcherPacketHandler` | `network/watcher_packet_handler.hpp` | Processes watcher request/response packets |
| `WatcherGlue` | `network/watcher_glue.hpp` | Connects watchers to the network layer |

Watcher packets use a large buffer size:
- UDP: `WN_PACKET_SIZE = 0x10000` (64 KB)
- TCP: `WN_PACKET_SIZE_TCP = 0x1000000` (16 MB)

### Watcher Forwarding

In multi-process BigWorld deployments, watcher queries can be forwarded between processes:

| Component | File | Description |
|-----------|------|-------------|
| `WatcherForwarding` | `server/watcher_forwarding.hpp` | Forward queries to other processes |
| `WatcherForwardingCollector` | `server/watcher_forwarding_collector.hpp` | Collect responses from multiple sources |

This allows a single monitoring tool to query all server processes through one endpoint.

## Integration Points

### Server Apps

Each BigWorld server app registers watchers for its key metrics:

```cpp
// Example from BigWorld source
#if ENABLE_WATCHERS
    MF_WATCH( "stats/numEntities", numEntities_ );
    MF_WATCH( "config/tickRate", tickRate_,
              Watcher::WT_READ_WRITE );
    MF_WATCH( "network/bytesSent", bytesSent_ );
#endif
```

### Channels

From `channel.hpp`, channels expose watchers for:
- Packets sent/received
- Bytes sent/received
- Packets resent
- Round trip time
- Send window usage

### Interface Elements

From `interface_element.hpp`, each message type tracks:
- Messages received count
- Bytes received count
- Max message size
- Average messages per second
- Average bytes per second

## Conditional Compilation

The watcher system is compiled conditionally:

```cpp
#if ENABLE_WATCHERS
    // Watcher registration and updates
    static WatcherPtr pWatcher();
    ProfileVal profile_;
#endif
```

This allows the system to be disabled in production builds for performance.

## Potential Cimmeria Integration

The watcher system could be valuable for Cimmeria for:

1. **Live monitoring**: Track entity counts, network stats, tick performance
2. **Remote debugging**: Inspect entity properties without stopping the server
3. **Performance tuning**: Profile message handling, identify bottlenecks
4. **GM tools**: Expose configuration values for runtime adjustment

### Implementation Approach

A simplified watcher system could be built using:
- Cimmeria's Python console (port 8989) as the query interface
- Boost.Python to expose C++ values to Python
- Periodic sampling for statistics

This would provide similar functionality without implementing the full BigWorld watcher protocol.

## Related Documents

- [BigWorld Architecture](bigworld-architecture.md) -- Server component overview
- [Service Architecture](../architecture/service-architecture.md) -- Cimmeria configuration
- [BigWorld Reference Index](../analysis/bigworld-reference-index.md) -- Source file locations

## TODO

- [ ] Determine if SGW.exe includes watcher client code (check Ghidra for WatcherNub references)
- [ ] Evaluate whether a simplified Python-based monitoring system is sufficient
- [ ] Document the `MF_WATCH` macro usage patterns in BigWorld source
- [ ] Investigate if the watcher protocol is used for any client-server communication in SGW
- [ ] Design a monitoring dashboard for Cimmeria server health
