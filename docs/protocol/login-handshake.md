# Login Handshake Protocol

> **Last updated**: 2026-03-01
> **RE Status**: Fully documented -- working end-to-end
> **Sources**: `src/authentication/`, `src/baseapp/mercury/sgw/`, `config/AuthenticationService.config`, `docs/connection-flow.md`

---

## Overview

The login handshake has three distinct phases. Phase 1 and 2 use HTTP/SOAP over TCP. Phase 3 uses Mercury over UDP. This document covers the protocol details of each phase.

## Phase 1: Authentication (SOAP/HTTP)

### Request: Login

```
POST /SGWLogin/UserAuth HTTP/1.1
Content-Type: text/xml

<SOAP envelope with username and password>
```

The client sends credentials to the Authentication Server on port 8081 (configurable via `logon_service_port` in `AuthenticationService.config`).

### Response: Login Success

Returns:
- Account information (account ID, access level)
- Shard list (available servers with name, status, population)
- Session token for Phase 2

### Response: Login Failure

Returns one of 31 error codes:

| Category | Examples |
|----------|---------|
| Connection | DNS lookup failed, TCP connection failed |
| Authentication | Wrong password, user not found, account locked |
| Server | Database not ready, no BaseApps available |
| Protocol | Version mismatch, malformed request |

### Protocol Digest Verification

The Authentication Server validates the client's protocol version against a stored MD5 digest:

```xml
<!-- AuthenticationService.config -->
<protocol_digest>58AFA196AD3AC4F65CADD99BFF23B799</protocol_digest>
```

This digest is computed from the entity definitions and ensures the client and server agree on the entity format.

## Phase 2: Server Selection (SOAP/HTTP)

### Request: Select Shard

```
POST /SGWLogin/ServerSelection HTTP/1.1
Content-Type: text/xml

<SOAP envelope with shard choice and session token>
```

### Server-Side Processing

1. Auth Server receives shard selection
2. Auth Server contacts the BaseApp via internal Mercury protocol (`unified_protocol.hpp`):
   - Sends `FES_REQUEST_LOGON (0x17)` to the BaseApp's service port (13002)
   - BaseApp responds with `FES_LOGON_ACK (0x18)` or `FES_LOGON_NAK (0x19)`
3. Auth Server generates a 64-character hex encryption key (the "session key")
4. Auth Server sends `FES_LOGON_NOTIFICATION (0x13)` to the BaseApp with the player's session info

### Auth-BaseApp Internal Protocol

From `src/mercury/unified_protocol.hpp`:

| Opcode | Value | Direction | Description |
|--------|-------|-----------|-------------|
| `FES_REGISTER_SHARD` | 0x10 | Base->Auth | Register this shard |
| `FES_REGISTER_SHARD_ACK` | 0x11 | Auth->Base | Shard registration confirmed |
| `FES_UPDATE_SHARD_STATUS` | 0x12 | Base->Auth | Update population/status |
| `FES_LOGON_NOTIFICATION` | 0x13 | Auth->Base | Player logged in |
| `FES_LOGOFF_NOTIFICATION` | 0x14 | Auth->Base | Player logged out |
| `FES_KICK_PLAYER` | 0x15 | Auth->Base | Force disconnect player |
| `FES_KICK_PLAYER_ACK` | 0x16 | Base->Auth | Kick acknowledged |
| `FES_REQUEST_LOGON` | 0x17 | Auth->Base | Request login approval |
| `FES_LOGON_ACK` | 0x18 | Base->Auth | Login approved |
| `FES_LOGON_NAK` | 0x19 | Base->Auth | Login rejected |
| `GENERIC_KEEPALIVE` | 0xFF | Both | Connection keepalive |

### Response: Server Select Success

Returns to client:
- BaseApp IP address (`shard_external_address` from `BaseService.config`)
- BaseApp port (default: 32832, from `shard_port`)
- Session key (64-character hex string = 256-bit AES key)
- Login ticket (proof of authentication)

## Phase 3: World Entry (Mercury/UDP)

### Step 1: BaseApp Login

The client opens a Mercury channel to the BaseApp:

```
Client -> BaseApp:  baseAppLogin message
  - Login ticket (from Phase 2)
  - Session key (from Phase 2)

BaseApp -> Client:  Ticket echo
  - Echoes the ticket back to prove server identity
```

At this point, both sides activate the AES-256 encryption filter using the session key. All subsequent Mercury packets are encrypted with:
- **Encryption**: AES-256-CBC
- **Integrity**: HMAC-MD5
- **Key**: 32 bytes decoded from the 64-char hex session key

This is a CME modification -- standard BigWorld uses Blowfish encryption.

### Step 2: Time Synchronization

The server sends three messages in a single flushed packet:

| Message | Description |
|---------|-------------|
| `updateFrequency` | Server update rate (10 Hz = 100ms ticks) |
| `tickSync` | Current game tick number |
| `gameTime` | Absolute time baseline |

These are flushed immediately as a reliable bundle before any other data.

### Step 3: Enable Entities

```
Client -> BaseApp:  enableEntities
```

The client signals it has finished initialization and is ready to receive entity data. The server **must wait** for this before sending entity information.

### Step 4: Create Player Entity

The server sends four messages in sequence:

```
BaseApp -> Client:  createBasePlayer
  - Entity type ID (SGWPlayer)
  - Entity ID
  - Base properties (account data, persistent state)

BaseApp -> Client:  spaceViewportInfo (CME custom)
  - Space/zone identifier
  - World parameters

BaseApp -> Client:  createCellPlayer
  - Entity ID
  - Space ID
  - Position (x, y, z)
  - Direction (yaw, pitch, roll)
  - Cell properties (name, abilities, etc.)

BaseApp -> Client:  forcedPosition
  - Entity ID
  - Position with velocity
```

The `spaceViewportInfo` message is a CME extension not present in standard BigWorld. It must arrive between `createBasePlayer` and `createCellPlayer` for the client to properly initialize the map viewport.

### Step 5: Game Loop

After the cell player is created, the connection enters regular operation:

- Server sends `tickSync` updates at the configured rate (default 10 Hz)
- Server sends entity updates (AoI enter/leave, position, property changes)
- Client sends movement updates and RPC calls
- Game messages flow bidirectionally

## Connection Parameters

From `config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `shard_port` | 32832 | UDP port for client connections |
| `client_inactivity_timeout` | 300000 ms | Disconnect idle clients |
| `tick_rate` | 100 ms | Server tick duration |
| `nub_tickrate` | 25 ms | Channel flush interval |
| `grid_vision_distance` | 3 chunks | AoI visibility range |

## Sequence Diagram

```
Client                     Auth Server              BaseApp
  |                            |                       |
  |                            |<-- FES_REGISTER_SHARD |
  |                            |-- FES_REGISTER_ACK -->|
  |                            |                       |
  |-- HTTP Login ------------->|                       |
  |<-- Shard list + account ---|                       |
  |                            |                       |
  |-- HTTP Select shard ------>|                       |
  |                            |-- FES_REQUEST_LOGON ->|
  |                            |<-- FES_LOGON_ACK -----|
  |                            |-- FES_LOGON_NOTIF --->|
  |<-- BaseApp addr + key ----|                       |
  |                            |                       |
  |== UDP Mercury (encrypted) ========================|
  |-- baseAppLogin ----------->|                       |
  |<-- ticket echo ------------|                       |
  |<-- time sync (3 msgs) ----|                       |
  |-- enableEntities --------->|                       |
  |<-- createBasePlayer -------|                       |
  |<-- spaceViewportInfo ------|                       |
  |<-- createCellPlayer -------|                       |
  |<-- forcedPosition ---------|                       |
  |                            |                       |
  |<======= game loop =======>|                       |
```

## Related Documents

- [Connection Flow](../connection-flow.md) -- High-level overview
- [Mercury Wire Format](mercury-wire-format.md) -- Packet-level protocol
- [Service Architecture](../architecture/service-architecture.md) -- Server roles

## TODO

- [ ] Document the exact SOAP XML schema for login request/response
- [ ] Document the exact baseAppLogin message binary format
- [ ] Verify the client challenge/response protocol (onClientChallenge / onClientChallengeResponse events)
- [ ] Document error recovery (what happens on Phase 3 failure)
- [ ] Document the version info exchange (versionInfoRequest / onVersionInfo events)
