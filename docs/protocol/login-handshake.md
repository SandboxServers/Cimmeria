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
Content-Length: <body length>

<sgwLogin:SGWLoginRequest
    xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin"
    SKU="SGW_BETA"
    AccountName="player1"
    Password="A1B2C3D4E5F6A1B2C3D4E5F6A1B2C3D4E5F6A1B2"
    ProtocolDigest="58AFA196AD3AC4F65CADD99BFF23B799" />
```

The client sends credentials to the Authentication Server on port 8081 (configurable via `logon_service_port` in `AuthenticationService.config`).

**Request attributes:**

| Attribute | Type | Description |
|-----------|------|-------------|
| `SKU` | string | Product identifier. Must be `"SGW_BETA"` |
| `AccountName` | string | 3-20 characters, alphanumeric plus `-` and `_` |
| `Password` | string | 40-character uppercase hex string (SHA-1 hash of plaintext password) |
| `ProtocolDigest` | string | 32-character MD5 hex string. Must match server's `protocol_digest` config |

**Server-side validation order:**

1. SKU must equal `"SGW_BETA"` (else `InvalidService`)
2. Password must be exactly 40 hex characters `[0-9A-F]` (else `MalformedPassword`)
3. AccountName must be 3-20 chars from `[0-9a-zA-Z_-]` (else `MalformedUserId`)
4. Database lookup: `SELECT account_id, upper(password), accesslevel, enabled FROM account WHERE account_name = :accname`
5. Password compared against stored SHA-1 hash (uppercase hex comparison)
6. Account must be enabled (`enabled = 't'`)

### Response: Login Success

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns2:SGWLoginResponse
    xmlns:ns2="http://www.stargateworlds.com/xml/sgwlogin"
    xmlns:ns3="http://www.cheyenneme.com/xml/cmebase"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin">
    <SGWLoginSuccess>
        <AccountInfo ExpireDate="0000-00-00T00:00:00.000Z" AccountId="12345" />
        <SGWShardListResp>
            <Shard ServerName="Shard" Fullness="LOW" Busy="LOW" />
        </SGWShardListResp>
    </SGWLoginSuccess>
</ns2:SGWLoginResponse>
```

The response includes an HTTP `Set-Cookie: SID=<40-char-session-id>` header. This session cookie is required for Phase 2.

**Response elements:**

| Element/Attribute | Description |
|------------------|-------------|
| `AccountInfo/@AccountId` | Numeric account ID from database |
| `AccountInfo/@ExpireDate` | Always `"0000-00-00T00:00:00.000Z"` (unused) |
| `Shard/@ServerName` | Name of each registered shard |
| `Shard/@Fullness` | Always `"LOW"` (hardcoded) |
| `Shard/@Busy` | Always `"LOW"` (hardcoded) |

### Response: Login Failure

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns2:SGWLoginResponse
    xmlns:ns2="http://www.stargateworlds.com/xml/sgwlogin"
    xmlns:ns3="http://www.cheyenneme.com/xml/cmebase"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin">
    <SGWLoginError ns3:ErrorStr="The user name or password is invalid." ns3:ErrorNum="1" />
</ns2:SGWLoginResponse>
```

Returns one of 18 error codes (the `FailureCode` enum in `logon_queue.hpp`):

| Code | Name | Error String |
|------|------|-------------|
| 0 | `Success` | (No error) |
| 1 | `MalformedUserId` | The specified account name is invalid. |
| 2 | `MalformedPassword` | The specified password has is invalid. |
| 3 | `InvalidService` | The specified service does not exist. |
| 4 | `BadUserPassword` | The user name or password is invalid. |
| 5 | `AccountDisabled` | Your account is disabled. |
| 6 | `AccessDenied` | Access is denied. |
| 7 | `NoServersAvailable` | No shards are available to the authentication server. |
| 8 | `NoSuchServer` | No such shard. |
| 9 | `ServerOffline` | The requested shard is offline. |
| 10 | `DbRequestFailed` | A request to the database server failed. |
| 11 | `ShardRequestTimedOut` | BaseApp timed out while waiting for a response... |
| 12 | `ShardLost` | Lost connection to BaseApp while waiting... |
| 13 | `InternalError` | Internal error. |
| 14 | `ShardRejected` | The BaseApp rejected your logon request. |
| 15 | `SessionExpired` | Your logon session has expired. Please log in again. |
| 16 | `NoCellAppsAvailable` | No CellApps are available to the BaseApp. |
| 17 | `VersionMismatch` | Protocol version mismatch; your client version is not supported... |

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
Content-Length: <body length>
Cookie: SID=<40-char-session-id-from-Phase-1>

<sgwLogin:SGWSelectServerRequest
    xmlns:sgwLogin="http://www.stargateworlds.com/xml/sgwlogin"
    ServerSelection="Shard" />
```

**Request attributes:**

| Attribute | Type | Description |
|-----------|------|-------------|
| `ServerSelection` | string | Name of the shard to join (must match a registered `Shard/@ServerName`) |

The session cookie from Phase 1 is sent via the HTTP `Cookie` header. If the cookie is missing or expired, the server returns `SessionExpired`.

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

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns3:SGWServerLocationResponse
    xmlns:ns3="http://www.stargateworlds.com/xml/sgwlogin"
    xmlns:ns1="http://www.cheyenneme.com/xml/cmebase"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin">
    <ServerLocation
        SessionKey="0A1B2C3D...64 hex chars..."
        Port="32832"
        IP="127.0.0.1"
        BWMailBox="1">
        <TICKET Ticket="A1B2C3D4E5F6A1B2C3D4" />
    </ServerLocation>
</ns3:SGWServerLocationResponse>
```

| Attribute | Type | Description |
|-----------|------|-------------|
| `SessionKey` | string | 64-character hex string = 256-bit AES key for Mercury encryption |
| `Port` | uint16 | BaseApp UDP port (default: 32832, from `shard_port` config) |
| `IP` | string | BaseApp IP address (`shard_external_address` from `BaseService.config`) |
| `BWMailBox` | uint32 | Internal mailbox ID for routing |
| `TICKET/@Ticket` | string | 20-character hex ticket ID (proof of authentication) |

### Response: Server Select Failure

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<ns3:SGWServerLocationResponse
    xmlns:ns1="http://www.cheyenneme.com/xml/cmebase"
    xmlns:ns3="http://www.stargateworlds.com/xml/sgwlogin"
    xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
    xsi:schemaLocation="sgwLogin http://www.stargateworlds.com/xml/sgwlogin">
    <ServerSelectionError ns1:ErrorStr="No such shard." ns1:ErrorNum="1" />
</ns3:SGWServerLocationResponse>
```

Uses the same `FailureCode` error strings as Phase 1. Common Phase 2 errors: `NoSuchServer`, `ServerOffline`, `AccessDenied` (for protected shards with `accessLevel < 2`), `SessionExpired`, `ShardRequestTimedOut`, `ShardLost`, `NoCellAppsAvailable`.

## Phase 3: World Entry (Mercury/UDP)

### Step 1: BaseApp Login

The client opens a Mercury channel to the BaseApp with a single `baseAppLogin` message (message ID `0x00` in the client message table). This is the first and only unencrypted Mercury packet sent by the client.

#### baseAppLogin Message (Client -> BaseApp)

The message is sent as a Mercury reliable request (packet flags: `FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE`). The packet must contain exactly one message.

**Binary format:**

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Account ID | Player's account ID from Phase 1 |
| 4 | 1 | uint8 | Ticket Length | Length of ticket string. Must be `20` |
| 5 | 20 | char[20] | Ticket | ASCII ticket ID from Phase 2 `TICKET/@Ticket` |

Total payload: **25 bytes** (variable-length format, `WORD_LENGTH` in the message table).

Note: The message table comment in `messages.cpp` describes the format as `uint32 AccountID` + `byte[32] AES key`, but the actual implementation in `connect_handler.cpp` reads `uint32 accountId` + `uint8 ticketLength` + `char[ticketLength] ticket`. The AES session key is **not** sent in this message -- it was already delivered to the BaseApp by the Auth Server via `FES_LOGON_ACK` and stored in the `ShardLogonQueue`.

**Server-side processing** (`connect_handler.cpp`):

1. Validate packet flags are `FLAG_HAS_REQUESTS | FLAG_HAS_SEQUENCE`
2. Verify only one message exists in the bundle
3. Verify message ID is `0x00`
4. Read `accountId` (uint32) and `ticketLength` (uint8)
5. Validate `ticketLength == 20`
6. Read ticket string (20 bytes)
7. Look up the pending login in `ShardLogonQueue::openClientSession(accountId, ticket)`
8. If found: create channel with encryption, send reply, begin session
9. If not found: log warning, silently drop (no reply sent)

#### BaseApp Login Reply (BaseApp -> Client)

The reply uses the special reply message ID `0xFF` (`BASEMSG_REPLY_MESSAGE`) with the format descriptor from `ServerMessageList[BASEMSG_AUTHENTICATE]`:

**Binary format:**

| Offset | Size | Type | Field | Description |
|--------|------|------|-------|-------------|
| 0 | 4 | uint32 | Request ID | Echo of the Mercury request ID from the client's packet |
| 4 | 1 | uint8 | Ticket Length | Length of ticket string (`20`) |
| 5 | 20 | char[20] | Ticket | Echo of the ticket from the client's request |

This reply is flushed as a separate packet before any other data is sent.

#### Encryption Activation

At this point, both sides activate the AES-256 encryption filter using the session key. The server decodes the 64-character hex session key into 32 raw bytes and constructs an `EncryptionFilter`. All subsequent Mercury packets are encrypted with:

- **Encryption**: AES-256-CBC with zero IV, PKCS#7 padding
- **Integrity**: HMAC-MD5 (16-byte MAC appended to each encrypted packet)
- **Key**: 32 bytes decoded from the 64-char hex session key
- **Minimum packet size**: 32 bytes (16 bytes ciphertext + 16 bytes HMAC minimum)
- **Packet size constraint**: Must be a multiple of 16 bytes

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

## Client Integrity Challenge Protocol

After normal gameplay begins, the server can send periodic integrity challenges to verify the client has not been tampered with. This is an anti-cheat mechanism, **not** part of the login handshake itself.

### Event Flow

```
BaseApp -> Client:  onClientChallenge (entity RPC on SGWPlayer)
Client -> BaseApp:  onClientChallengeResponse (exposed cell method on SGWPlayer)
```

### onClientChallenge (Server -> Client)

Defined in `SGWPlayer.def` under `<ClientMethods>`:

```xml
<onClientChallenge>
    <Arg> INT32 <ArgName>aClientChallenge</ArgName></Arg>    <!-- challenge nonce -->
    <Arg> INT32 <ArgName>aChallengeType</ArgName></Arg>      <!-- EClientChallengeType enum -->
    <Arg> WSTRING <ArgName>aChallengeObject</ArgName></Arg>  <!-- target file/object to hash -->
    <Arg> INT32 <ArgName>aChallengeID1</ArgName></Arg>       <!-- context parameter 1 -->
    <Arg> INT32 <ArgName>aChallengeID2</ArgName></Arg>       <!-- context parameter 2 -->
</onClientChallenge>
```

### onClientChallengeResponse (Client -> Server)

Defined in `SGWPlayer.def` under `<CellMethods>` with `<Exposed/>`:

```xml
<onClientChallengeResponse>
    <Exposed/>
    <Arg> INT32 <ArgName>aClientChallenge</ArgName></Arg>    <!-- echo of challenge nonce -->
    <Arg> WSTRING <ArgName>aClientVersion</ArgName></Arg>    <!-- client version string -->
    <Arg> INT32 <ArgName>aChallengeType</ArgName></Arg>      <!-- echo of challenge type -->
    <Arg> WSTRING <ArgName>aChallengeObject</ArgName></Arg>  <!-- echo of target object -->
    <Arg> INT32 <ArgName>aChallengeID1</ArgName></Arg>       <!-- echo of context param 1 -->
    <Arg> INT32 <ArgName>aChallengeID2</ArgName></Arg>       <!-- echo of context param 2 -->
    <Arg> WSTRING <ArgName>aChallengeValue</ArgName></Arg>   <!-- computed hash result -->
</onClientChallengeResponse>
```

### Challenge Types (`EClientChallengeType` enum)

| Value | Name | Description |
|-------|------|-------------|
| 0 | `SHA1_CS` | SHA-1 hash of a C# assembly/file |
| 1 | `SHA1_UnrealScript` | SHA-1 hash of an UnrealScript file |

### Protocol Details

The server can trigger a challenge at any time via the Python console command `clientChallenge(player, target, challenge, type, object, id1, id2)` defined in `python/cell/commands/Net.py`. The client receives the challenge as an entity RPC via the CME event system (`Event_NetIn_onClientChallenge`), computes a SHA-1 hash of the specified file/object, and responds with `onClientChallengeResponse` via the event `Event_NetOut_onClientChallengeResponse`.

The Cimmeria server's `SGWPlayer.py` handler currently logs the response parameters for debugging but does not enforce any validation. The original server would record challenge results to a `client_challenge` database table (per the client binary's SQL strings: `insert into client_challenge (challenge_type, challenge_string, challenge_id_1, challenge_id_2, challenge_result ...)`).

Client-side validation strings found in the binary: `"invalid challenge length"` and `"challenge is different"` indicate the client performs its own sanity checks on incoming challenges before computing the hash.

## Error Recovery

### Phase 1/2 Failure (SOAP/HTTP)

Phase 1 and 2 failures are fully recoverable -- the client simply displays the error string and returns to the login or shard selection screen. The HTTP connection is stateless between requests (session state is maintained via the `SID` cookie).

### Phase 3 Failure Scenarios (Mercury/UDP)

#### Invalid Ticket or Expired Session

If the client sends a `baseAppLogin` with an unknown or expired ticket:
- The `ShardLogonQueue::openClientSession()` lookup fails
- The server logs a warning: `"Unable to authenticate ticket ... (maybe it expired?)"`
- **No reply is sent** -- the client receives no response and must time out
- The ticket expires after `ShardLogonQueue::TicketExpiration` (30,000 ms = 30 seconds) if the client never connects

#### Entity Creation Failure

If the `Account` entity fails to create during `ClientHandler::onConnected()` (e.g., Python exception, missing entity definition):
- The server sends `BASEMSG_LOGGED_OFF` (message `0x37`) with reason byte `0x00`
- The channel is flushed and condemned (graceful shutdown)
- The channel waits for ACK of the logoff message before closing

#### Client Inactivity Timeout

If the client stops sending traffic after connecting:
- The `BaseChannel::tick()` method checks `lastPeerActivity_` against `InactivityTimeout`
- Default: `client_inactivity_timeout = 300000` ms (5 minutes) in `BaseService.config`
- When triggered, the channel is closed: `BaseChannel::close()`
- The entity is destroyed via `ClientHandler::onDisconnected()` -> `disconnectEntity(false)`

#### Server-Initiated Disconnect

The server can force a disconnect via `ClientHandler::disconnectEntity(killConnection=true)`:
1. Entity system is disabled (`entitySystemEnabled_ = false`)
2. Python `detachedFromController()` is called on the entity
3. Entity is destroyed
4. Pending bundles are flushed
5. `BASEMSG_LOGGED_OFF` (reason `0x00`) is sent and flushed
6. Channel is condemned via `channel_->condemn()`

#### Condemned Channel Lifecycle

When a channel is condemned:
1. All pending messages are flushed immediately
2. No new reliable messages can be queued
3. The channel only processes incoming ACKs (ignores new messages from peer)
4. Keepalive messages stop being sent
5. `CondemnedChannels::pollChannels()` periodically checks if all reliable packets have been ACKed or if the channel has been inactive
6. Once all ACKs are received or the `InactiveChannelTimeout` expires, the channel is fully closed

#### BaseApp-Auth Server Reconnection

If the BaseApp loses its connection to the Authentication Server:
- `ShardClient::onDisconnected()` is called
- The BaseApp waits `ConnectionRecoveryTimeout` (30,000 ms = 30 seconds)
- Then attempts to reconnect and re-register the shard
- During the disconnect, no new players can be assigned to this shard
- Players already connected are **not** affected -- their Mercury channels are independent

#### Auth Server Request Timeout

If the Auth Server sends `FES_REQUEST_LOGON` to the BaseApp and no `FES_LOGON_ACK`/`FES_LOGON_NAK` is received:
- `FrontendConnection::onRequestTimeout()` fires after `LoginRequestTimeout` (5,000 ms = 5 seconds)
- The pending login request is removed
- The client receives the `ShardRequestTimedOut` error via SOAP response
- The client can retry shard selection

## Connection Parameters

From `config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `shard_port` | 32832 | UDP port for client connections |
| `client_inactivity_timeout` | 300000 ms | Disconnect idle clients |
| `tick_rate` | 100 ms | Server tick duration |
| `nub_tickrate` | 25 ms | Channel flush interval |
| `grid_vision_distance` | 3 chunks | AoI visibility range |

From `config/AuthenticationService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `logon_service_port` | 8081 | HTTP port for client login |
| `base_service_port` | 13001 | TCP port for BaseApp connections |
| `protocol_digest` | (MD5 hash) | Entity definition version check |

Internal timeouts (hardcoded):

| Constant | Value | Location | Description |
|----------|-------|----------|-------------|
| `LoginRequestTimeout` | 5,000 ms | `frontend_connection.hpp` | Auth->Base logon request timeout |
| `TicketExpiration` | 30,000 ms | `shard_client.hpp` | Pending login ticket expiry |
| `ConnectionRecoveryTimeout` | 30,000 ms | `shard_client.hpp` | BaseApp->Auth reconnect delay |
| `InactivityKeepaliveInterval` | (channel const) | `channel.hpp` | Keepalive send interval |

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

- [x] ~~Document the exact SOAP XML schema for login request/response~~ → Full XML schemas documented for all four SOAP messages: `SGWLoginRequest`, `SGWLoginResponse`, `SGWSelectServerRequest`, `SGWServerLocationResponse`. Includes all attributes, namespace URIs, and error formats. Source: `src/authentication/logon_connection.cpp`
- [x] ~~Document the exact baseAppLogin message binary format~~ → Binary field layout documented with offset table for both client message (25-byte payload: uint32 accountId + uint8 ticketLength + char[20] ticket) and server reply (25-byte payload: uint32 requestId + uint8 ticketLength + char[20] ticket echo). Corrects inaccurate comment in `messages.cpp`. Source: `src/baseapp/mercury/sgw/connect_handler.cpp`
- [x] ~~Verify the client challenge/response protocol (onClientChallenge / onClientChallengeResponse events)~~ → Confirmed: this is a post-login anti-cheat integrity check, not part of the login handshake. Server sends `onClientChallenge` entity RPC with challenge type (SHA1_CS or SHA1_UnrealScript), client responds with `onClientChallengeResponse` including computed hash. Sources: `entities/defs/SGWPlayer.def`, `python/cell/SGWPlayer.py`, `python/cell/commands/Net.py`, client binary strings + Ghidra event registrations
- [x] ~~Document error recovery (what happens on Phase 3 failure)~~ → Documented six failure scenarios: invalid/expired ticket (silent drop), entity creation failure (LOGGED_OFF + condemn), inactivity timeout (5min default), server-initiated disconnect, condemned channel lifecycle, BaseApp-Auth reconnection (30s retry), and Auth Server request timeout (5s). Sources: `src/baseapp/mercury/sgw/connect_handler.cpp`, `src/baseapp/mercury/sgw/client_handler.cpp`, `src/mercury/channel.cpp`, `src/mercury/condemned_channels.cpp`, `src/authentication/shard_client.cpp`, `src/authentication/frontend_connection.cpp`
- [x] ~~Document the version info exchange~~ → ClientCache `versionInfoRequest`/`onVersionInfo` fully documented in `findings/entity-types-wire-formats.md`
