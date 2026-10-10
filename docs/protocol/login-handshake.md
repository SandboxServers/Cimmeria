---
title: "Login Handshake Protocol"
type: reference
audience: engineers
last_updated: 2026-05-27
---

# Login Handshake Protocol

> **Last updated**: 2026-03-01
> **RE Status**: Fully documented -- working end-to-end
> **Sources**: `deprecated/cpp/src/authentication/`, `deprecated/cpp/src/baseapp/mercury/sgw/`, `deprecated/cpp-config/config/AuthenticationService.config`, `docs/connection-flow.md`

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
| `Password` | string | Either a 40-char uppercase-hex SHA-1 hash (original client) **or** a plaintext password (patched client over TLS only). See *Credential formats* below. |
| `ProtocolDigest` | string | 32-character MD5 hex string. Must match server's `protocol_digest` config |

**Attribute encoding.** Every attribute the server reads is ordinary XML: the
server decodes it once, with quick-xml's attribute-value normalization, before
any check runs. Attributes it does not read are only syntax-checked. The predefined entities (`&amp;` `&lt;` `&gt;` `&quot;` `&apos;`) and character
references (`&#38;`, `&#x26;`) become the characters they stand for, and a raw
tab, CR or LF becomes a space (XML 1.0 §3.3.3). A client sending the plaintext
password `a&b` writes `Password="a&amp;b"`; `&amp;amp;` decodes to the literal
text `&amp;`. A malformed value (an unknown entity such as `&bogus;`, a bare
`&`, a bad character reference) or broken attribute syntax on the request
element, including a duplicated attribute, fails the request with the
`Internal error.` login error, and the server logs `reason` and `attribute`
without the value. The SHA-1 hex the original client sends has no `&`, so
decoding leaves it unchanged. The `ServerSelection` attribute in Phase 2 is
decoded the same way.

**Credential formats (dual acceptance):**

The server accepts two credential shapes and classifies by the supplied value:

- **Legacy SHA-1 hash** — a 40-char all-hex string, as the original (unpatched)
  client sends. Accepted on the plain-HTTP **and** TLS listeners.
- **Plaintext password** — anything else. Accepted **only** over the TLS
  listener (the entire argon2id rationale depends on the transport being
  TLS-wrapped); a plaintext password offered over plain HTTP is rejected with
  the malformed-password error. Bounded to 1–128 bytes.

Stored passwords use one of two schemes per account, selected by
`account.password_algo` (`1`=legacy SHA-1 in `account.password`, `2`=argon2id PHC
string in `account.password_hash_v2`). A legacy account that authenticates with a
**plaintext** password is verified against its SHA-1 hash and then
**opportunistically migrated** to argon2id in the same login (algo flips to 2,
`password_hash_v2` populated, `password` NULLed). Migration failure is logged but
never fails an already-verified login. See
[../architecture/encryption-modernization.md](../architecture/encryption-modernization.md)
Phase 2.

**Server-side validation order:**

1. SKU must equal `"SGW_BETA"` (else `InvalidService`)
2. AccountName must be 3-20 chars from `[0-9a-zA-Z_-]` (else `MalformedUserId`).
   This runs before the name reaches the request span or any audit row, so a
   control character (raw, or decoded from `&#10;`) is never logged or stored.
3. Classify the credential: a 40-char hex string is a legacy hash; anything else
   is plaintext and requires TLS, else `MalformedPassword`
4. Database lookup: `SELECT account_id, password, password_hash_v2, password_algo, accesslevel, enabled FROM account WHERE account_name = :accname`
5. Credential verified per `(password_algo, credential shape)`: argon2id verify, legacy uppercase-hex compare, or legacy plaintext → SHA-1 recompute + compare (then migrate)
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

From `deprecated/cpp/src/mercury/unified_protocol.hpp`:

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

#### Cross-IP session binding (server-observed, warn-only)

The server records the source IP each SID (Phase 1) and ticket (Phase 2) is issued
to. At Phase 2 the SID is normally consumed over the same TCP connection that
produced it, and at Phase 3 the Mercury `baseAppLogin` datagram is normally sent
from that same host. The consumption seams log a `WARN` with `reason =
"session_ip_mismatch"` (Phase 2) or `reason = "ticket_ip_mismatch"` (Phase 3)
when the request source IP differs from the issuing IP.

This is telemetry-only, intentionally **not** a rejection: NAT and IPv4/IPv6
dual-stack can surface a different source IP for the same physical client, so a
hard fail would lock out legitimate logins. The mismatch rows exist to measure
the false-positive rate before the seam hardens to a config-gated rejection.
Phase 2 (TCP to TCP) is the high-confidence seam and the one to harden first;
Phase 3 compares a TCP source against a UDP source, which carrier-grade NAT and
true dual-stack clients can legitimately split, so it should stay warn-only.

### Step 2: Time Synchronization

The server sends three messages in a single flushed packet:

| Message | Description |
|---------|-------------|
| `updateFrequency` | Server update rate (10 Hz = 100ms ticks) |
| `tickSync` | Current game tick number |
| `gameTime` | Absolute time baseline |

These are flushed immediately as a reliable bundle before any other data.

#### Reliability of the reply and the time-sync bundle

The reply is reliable sequence 1 and the time-sync bundle reliable sequence 2, both with flags `0x58` (`HAS_SEQUENCE | RELIABLE | ON_CHANNEL`). The server's own reliable counter then starts at 3. The legacy C++ server sent both through the new channel's reliable bundle (`connect_handler.cpp`, `ClientHandler::onConnected`), so both sat on the channel's resend timers until acked.

The Rust server sends both before the session `Channel` exists, then registers the two exact datagrams in its TX window (`new_client_channel_with_handshake`, `crates/base/src/base/login/mod.rs`, #842). A lost one is resent verbatim on RTO like any other reliable packet. Before #842 they were never tracked, so a lost reply left the client re-sending `baseAppLogin` every 300 ms (logged as `reason = login_retry_on_channel`) and a lost time-sync left a permanent gap at the head of the client's reliable stream.

What the client does with them, from the five logins decoded from the captures under `debug/` (`tools/pcap_to_session.py`):

| Captures | Client behaviour |
|---|---|
| 3 of 5 | Acks both at once, about 10 ms after they arrive: an ack-only packet with flags `0x4C` (HAS_SEQUENCE, ON_CHANNEL and HAS_ACKS; unreliable), sequence 2 on the counter its `baseAppLogin` used (sequence 1), `acks [2, 1]`. Its first channel packet (AUTHENTICATE + ENABLE_ENTITIES, reliable sequence 0) follows. |
| 2 of 5 | Never acks them. Its first channel packet is reliable sequence 0 with no acks, and later footers ack only sequence 3 and up. |

For the second kind the server resends both once, about 1.5 s after login (the initial RTO). By then the client's channel exists, and the client acks every reliable packet that carries a valid sequence before it checks `inSeqAt` (`UnAckedHandler::queueAckForPacket`, `ghidra://SGW.exe@0x0158cba0`), so the resend is acked and dropped as a duplicate. A pair of `mercury.retransmit` rows for seqs 1 and 2 right after login is therefore normal and does not mean loss. That the real client acks the resend is inferred from that function, not yet observed on the wire.

In case a client ignores the resends too, both packets are capped at `HANDSHAKE_RETRANSMIT_CAP` (6) resends. Every other reliable packet resends until acked. The resends follow the channel's RTO backoff (on a quiet channel 1.5 s, doubling to the 4 s ceiling, so the last one goes out about 20 s after login). After the cap the channel drops the entry and logs one `reliable_resend_abandoned` WARN (`reason = retransmit_cap_reached`, `account_id`, `seq`); see [negative-logging-convention.md](../architecture/negative-logging-convention.md#abandoned-reliable-resends-842).

The `login_retry_on_channel` row carries `reply_outstanding`: `true` means the server has not seen the client's ACK of the reply, so the reply was probably lost and a resend is pending; `false` means the client acked the reply and is retrying anyway, the stuck-client case where a resend would change nothing.

#### A client relaunched on the same address:port

The SGW client binds a fixed UDP port (63888 on the colo). A client that is killed and relaunched therefore comes back on the **same** address:port as the session it left behind, and that session stays registered until the 60 s inactivity reap. Every datagram from a registered address goes to its encrypted channel, so before the fix the relaunched client's plaintext `baseAppLogin` failed to decrypt and was dropped as `login_retry_on_channel`. Meanwhile the old session's tick-sync loop kept sending packets under the old key, which the new client logged as "Dropped corrupted incoming packet". The player sat at "Logging in..." until the old channel timed out (colo, release v2026-10-05.1, DA-06 lab run).

The base now tells a relaunch from a retransmit by the ticket. Tickets are single-use, so the retransmit of the login that created the channel carries a ticket that was consumed when the channel was registered. A relaunched client went back through SOAP login and carries a fresh, unconsumed one. A plaintext `baseAppLogin` on an established channel goes to `handle_login` when all of these hold (`crates/base/src/base/login/relaunch.rs`):

1. It is a well-formed plaintext `baseAppLogin` and does **not** decrypt under the live session's key.
2. Its ticket is still unconsumed in `pending_logins`.

`handle_login` then checks the ticket's age (it must be younger than `TICKET_TTL`, 30 s; an older one is burned with `reason = ticket_expired`) and asks `relaunch::address_claim` what holds the address:

3. A session of the **same account**: a relaunch. The old session is evicted.
4. A session of **another account** that registered with a ticket issued to a different IP than its source address, while the incoming ticket was issued to this address's IP: a squatter. The squatter is evicted (`reason = address_reclaimed`).
5. Anything else: refused (`reason = relaunch_account_mismatch`, WARN), the ticket burned, the live session kept. A poisoned session lock also refuses.

Rule 3 stops a spoofed source address from kicking a **live** player. Anyone can spoof a live player's address:port, and anyone with an account can get a ticket for their own account. Without rule 3 that pair would let any account holder kick any player whose address they know. With it, a takeover needs a fresh ticket for the victim's own account, which takes the victim's password, and that already lets an attacker evict the victim from any address through the duplicate-login path.

Rule 3 does not protect a **free** address. An attacker can spoof the victim's address:port while no session holds it (before the victim logs in, or after a reap) and register there with a ticket for their own account. The SOAP request behind that ticket is TCP, which cannot be spoofed, so the ticket names the attacker's real IP, and the registration logs `ticket_ip_mismatch` (#442, still warn-only). Each session records whether its ticket IP matched (`relaunch::TicketIpBinding`, in the session's extensions). Rule 4 lets the victim's own login, whose ticket was issued to the victim's IP, evict such a squatter instead of being refused and burned. Rule 4 never evicts a session whose ticket matched its address. A session behind carrier-grade NAT can register with a mismatched ticket IP legitimately; another account's ticket issued to the same public IP can then evict it, which needs a second client behind the same NAT.

The new client cannot prove it holds the new session key before the takeover. Phase 3 is plaintext, and every new channel is registered on the ticket alone. The ticket and the key come from the same SOAP reply, so waiting for a first datagram under the new key would add nothing. There is no separate rate limit, because each takeover consumes a ticket and each ticket costs a full SOAP login.

On a takeover, `login/eviction.rs` evicts the old session with `disconnect_reason = relaunch_takeover` through `destroy_client_entities`, the same teardown a duplicate login uses. The old character is announced offline, the cell is told to disconnect it (and persists its position before confirming), and the old tick-sync loop is cancelled. No `LOGGED_OFF` goes out: the old client is dead, and the packet would reach the new client under the old key. The new session, with its new key and a fresh channel, then replaces the old one in a single `connected` map write. The old loop sends at most one more tick under the old key before it sees its cancel flag, and its own teardown is owner-checked (`destroy_owned_client_entities`), so it cannot remove the new session. One WARN row marks the takeover: `reason = relaunch_takeover` with `account_id`, `account_name`, `player_id`, `player_name` and `old_session_secs`.

Before anything is evicted, `handle_login` checks once more that the address is free, held by the same account, or held by a squatter rule 4 allows it to evict (`reason = address_still_occupied`, ERROR, otherwise). The address claim already rules that case out, so the check only guards against a bug in it, and it runs before the account's sessions on other addresses are logged off.

Other tasks that resolved the address before the takeover must not act on the new session. The old tick-sync loop re-checks its cancel flag before its retransmit scan. Teardown also stops a first-login cinematic's appearance re-send loop (`cinematic_spam_cancel`), which otherwise reads whatever session holds the address. An in-flight gate transfer checks that the session still has its player entity before sending `RESET_ENTITIES`, storing the destination world entry or abandoning the session (`gate_travel/session_owner.rs`, `reason = session_replaced`).

The inactivity clock (`last_recv`) refreshes only for a datagram that decrypts under the session key **and** brings something new (`connect_loop/encrypted/liveness.rs`): a reliable packet the receive gate accepts as new, an ACK footer that retires an outstanding packet, or an unreliable packet whose sequence number advances past any the session has seen (the client numbers its unreliable packets on their own counter). So garbage, a spoofed source, a relaunched client's plaintext retries, and a replay of a captured datagram cannot keep a dead session alive, while an idle in-world client (about six sequenced unreliable packets a second) stays alive. A retransmit of the original login keeps the behaviour described above: no teardown, one `login_retry_on_channel` row.

Killing the client in the middle of a fight now takes the old character out of the world as soon as the relaunched client logs in, after roughly 15 to 30 s, instead of after the 60 s reap. That is intentional. The teardown is the same one an inactivity timeout runs (`destroy_client_entities`, then the cell's `DisconnectEntity`, which carries no reason), so death, duel and despawn rules match a timeout: a duel counts the disconnect as a forfeit either way. The server has no in-combat logout rule for this to bypass, and a player could already end the session at once by logging in from another address (duplicate-login eviction), so it opens no new combat escape.

The guards are in `crates/base/src/base/connect_loop/relaunch_tests.rs`, `connect_loop/encrypted/liveness_tests.rs`, `login/tests/ticket_age.rs`, `crates/base-session/src/base/helpers/session_teardown_tests.rs` and `crates/base-world-entry/src/base/world_entry/gate_travel/tests/session_replaced.rs`.

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

The server can trigger a challenge at any time via the Python console command `clientChallenge(player, target, challenge, type, object, id1, id2)` defined in `deprecated/python/cell/commands/Net.py`. The client receives the challenge as an entity RPC via the CME event system (`Event_NetIn_onClientChallenge`), computes a SHA-1 hash of the specified file/object, and responds with `onClientChallengeResponse` via the event `Event_NetOut_onClientChallengeResponse`.

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

From `deprecated/cpp-config/config/BaseService.config`:

| Parameter | Default | Description |
|-----------|---------|-------------|
| `shard_port` | 32832 | UDP port for client connections |
| `client_inactivity_timeout` | 300000 ms | Disconnect idle clients |
| `tick_rate` | 100 ms | Server tick duration |
| `nub_tickrate` | 25 ms | Channel flush interval |
| `grid_vision_distance` | 3 chunks | AoI visibility range |

From `deprecated/cpp-config/config/AuthenticationService.config`:

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

- [x] ~~Document the exact SOAP XML schema for login request/response~~ → Full XML schemas documented for all four SOAP messages: `SGWLoginRequest`, `SGWLoginResponse`, `SGWSelectServerRequest`, `SGWServerLocationResponse`. Includes all attributes, namespace URIs, and error formats. Source: `deprecated/cpp/src/authentication/logon_connection.cpp`
- [x] ~~Document the exact baseAppLogin message binary format~~ → Binary field layout documented with offset table for both client message (25-byte payload: uint32 accountId + uint8 ticketLength + char[20] ticket) and server reply (25-byte payload: uint32 requestId + uint8 ticketLength + char[20] ticket echo). Corrects inaccurate comment in `messages.cpp`. Source: `deprecated/cpp/src/baseapp/mercury/sgw/connect_handler.cpp`
- [x] ~~Verify the client challenge/response protocol (onClientChallenge / onClientChallengeResponse events)~~ → Confirmed: this is a post-login anti-cheat integrity check, not part of the login handshake. Server sends `onClientChallenge` entity RPC with challenge type (SHA1_CS or SHA1_UnrealScript), client responds with `onClientChallengeResponse` including computed hash. Sources: `entities/defs/SGWPlayer.def`, `deprecated/python/cell/SGWPlayer.py`, `deprecated/python/cell/commands/Net.py`, client binary strings + Ghidra event registrations
- [x] ~~Document error recovery (what happens on Phase 3 failure)~~ → Documented six failure scenarios: invalid/expired ticket (silent drop), entity creation failure (LOGGED_OFF + condemn), inactivity timeout (5min default), server-initiated disconnect, condemned channel lifecycle, BaseApp-Auth reconnection (30s retry), and Auth Server request timeout (5s). Sources: `deprecated/cpp/src/baseapp/mercury/sgw/connect_handler.cpp`, `deprecated/cpp/src/baseapp/mercury/sgw/client_handler.cpp`, `deprecated/cpp/src/mercury/channel.cpp`, `deprecated/cpp/src/mercury/condemned_channels.cpp`, `deprecated/cpp/src/authentication/shard_client.cpp`, `deprecated/cpp/src/authentication/frontend_connection.cpp`
- [x] ~~Document the version info exchange~~ → ClientCache `versionInfoRequest`/`onVersionInfo` fully documented in `findings/entity-types-wire-formats.md`
