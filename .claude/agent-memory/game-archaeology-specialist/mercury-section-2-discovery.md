---
name: mercury-section-2-discovery
description: Phase A discovery manifest for Mercury chapter Section 2 (Client findings) — full client-tree sweep, binary string inventory, and Section 1 footnote re-classification
metadata:
  type: project
---

# Mercury Wire Format — Section 2 Discovery Manifest

**Phase A discovery pass — 2026-05-14**

## Scope

Full inventory of every client-side artifact bearing on the Mercury wire format or its configuration,
drawn from:
- `game/sgw/Working/` (all INI, bat, config, and XML files)
- `game/sgw/Working/binaries/` (AtreaLoader.config.xml, SGWLogConfig.xml, launcher scripts)
- SGW.exe string scan via Ghidra MCP (Mercury-specific strings, network-driver strings, protocol-diagnostic strings)
- Cross-reference of client binary to the 86 Section 1 footnotes

---

## Category 1 — Hard-coded client constants

These values are baked into SGW.exe binary code or its statically-registered descriptors.
The server has no INI-side leverage to negotiate them.

### C1-A: Maximum packet size — 1453 bytes (0x5AD)

- **Path**: SGW.exe (binary), assertion at `ghidra://SGW.exe@0x017ffe80`
- **Snippet**: string `"PacketSize >= 0 && PacketSize < 4096"` at `0x017ffe80`; string `"PacketSize > 4"` at `0x017ffe50`
- **Context**: `DebugCommunication.cpp` assertions in `FUN_0047c3e0`/`FUN_0047c2f0`. The per-bundle space check
  in `Mercury_Bundle_newMessage` at `0x0157ac90` (Section 1 anchor `[^bundle-new-message]`) enforces the 1453-byte cap.
  The `DebugCommunication.cpp` assertions are in the Cimmeria-side packet replay/sniffer tool, not in the main
  Mercury path, but confirm the `< 4096` sanity bound and `> 4` minimum are compile-time constants.
- **Implication**: The server MUST emit packets no larger than 1453 bytes of Mercury plaintext. Cipher overhead
  (AES-256-CBC PKCS#7 pad + 16-byte HMAC) means the actual UDP datagram is at most ~1485 bytes. The `< 4096`
  sniffer bound is a debug-tool assertion and does not describe the Mercury max — it confirms the sniffer treats
  anything ≥ 4096 as corrupt.
- **Confidence**: High — independently confirmed by Section 1 `[^bundle-new-message]`.
- **Cross-ref**: Supplements Section 1 §1.1 and `[^bundle-new-message]`. No contradiction.

### C1-B: Flags byte is 1 byte (uint8)

- **Path**: SGW.exe RTTI and decompile evidence (Section 1 `[^v5-mercury-internals]`, `[^flags-decoder]`)
- **Implication**: A server sending a `uint16` flags prefix will be silently misinterpreted. The bit mask table
  in §1.2 (8 bits, `0x01`–`0x80`) is a hard client constant.
- **Confidence**: High — binary-confirmed.

### C1-C: Footer endianness — all footer fields are little-endian

- **Path**: SGW.exe decompile evidence (Section 1 `[^v5-mercury-internals]`, `[^stockbw-packet-cpp]`)
- **Implication**: A server that writes footer fields in network byte order (as stock BW 2.0.1 does) will
  produce silently wrong sequence/ack values in the client parser.
- **Confidence**: High — binary-confirmed.

### C1-D: Sequence number space — 28-bit, sentinel 0x10000000

- **Path**: SGW.exe decompile, Section 1 `[^v5-mercury-internals]`
- **Implication**: The server may not use `0x10000000` as a real sequence ID. Sequences wrap at `0x0FFFFFFF`.
- **Confidence**: High.

### C1-E: Outstanding-ack bitmap — 32-bit (max 32 in-flight reliable packets)

- **Path**: `UnAckedHandler__buildAndSendAckBundle` at `ghidra://SGW.exe@0x0158b2d0` — loop iterates
  `iVar2 = 0, 8, 16, 24; iVar2 < 0x20`. Section 1 `[^ack-bitmap]`.
- **Implication**: The server may not have more than 32 reliable packets simultaneously awaiting acknowledgement
  on a single channel. Exceeding this fills the bitmap and triggers incorrect ack behavior.
- **Confidence**: High.

### C1-F: Receive-side dedup table — 512 entries, mask 0x1FF

- **Path**: `FUN_0158c170` at `ghidra://SGW.exe@0x0158c170`. Section 1 `[^channel-hash-alloc]`.
- **Implication**: Duplicate detection operates modulo 512. A server sending more than 512 distinct sequence IDs
  between acks may produce false-duplicate drops at the client.
- **Confidence**: High.

### C1-G: Max retries — 20 (strict greater-than check)

- **Path**: Section 1 §1.7 and `[^v5-mercury-internals]`. Inherited from stock BW 2.0.1; SGW divergence not
  enumerated.
- **Implication**: A channel disconnects if a reliable packet is retransmitted more than 20 times without ack.
- **Confidence**: Medium (inherited from stock BW; no SGW-specific byte confirmed).
- **FORWARD CORRECTION (2026-05-15)**: A targeted disassembly pass in [[mercury-section-2-track-b-evidence]]
  Finding 1.4 located a `5.0f` constant at `ghidra://SGW.exe@0x01e91e00` (loaded by `FUN_0158c420` =
  `UnAckedHandler::checkResendTimers`) that gates the **per-tick work budget**, NOT the lifetime cap.
  The "20 retries" lifetime cap remains an inherited claim from upstream BigWorld 2.0.1 — no SGW-specific
  byte was located that confirms it on this build. The two values are distinct invariants (per-tick
  budget vs lifetime cap); see Track B Finding 1.4 and chapter §2.4.1 R14 for the disambiguated framing.

### C1-H: Resend timeout — ~700 ms

- **Path**: Section 1 §1.7 and `[^v5-mercury-internals]`. Inherited from stock BW 2.0.1.
- **Implication**: The client schedules acks and triggers retransmit on this cadence. Server resend timers must
  be calibrated not to retransmit before this window elapses on the client side.
- **Confidence**: Medium (inherited; no SGW-specific binary measurement available).

### C1-I: NetInactivityTimeout — 15 seconds (INI-readable, client-side UE3 hook)

- **Path**: `game/sgw/Working/Engine/Config/GameplayEngine.ini`, `[Engine.Engine]` section:
  `NetInactivityTimeout=15`
- **Also in binary**: string `"NetInactivityTimeout"` at `ghidra://SGW.exe@0x019abb7c`, xref into
  `ConstructEntityRpcRegistry` — this key is read by the SGW game-layer (not the Mercury layer directly) and
  used to gate the `REASON_INACTIVITY` disconnect path.
- **Snippet**: `NetInactivityTimeout=15`
- **Implication**: The UE3 game layer fires an inactivity disconnect after 15 seconds of no meaningful traffic
  (entity updates, etc.). This is a UE3-layer timer, not the Mercury-layer keepalive timer. The Mercury layer
  (`KeepAliveTime`, C1-J below) operates on a shorter cycle; the UE3 timer is the outer safety net.
  `REASON_INACTIVITY` at `ghidra://SGW.exe@0x019d11f0` is emitted by `LookupDisconnectReasonName` —
  the code path that selects this disconnect reason. Confidence: high.

### C1-J: AES-256-CBC + HMAC-MD5 cipher — hard-coded, no negotiation

- **Path**: RTTI strings `"AES-256-CBC"` at `ghidra://SGW.exe@0x01b29b1c`, `"AES-128-CBC"` at `0x01b29b28`
  (the 128-CBC string is present because CryptoPP includes it, but the active path is 256-CBC per Section 1);
  RTTI `".?AVPacketFilter@Mercury@@"` at `0x01e93b2c`; `"Not sending packet because of encryption error: %s\n"`
  at `0x01b27218`.
- **Implication**: The cipher suite is fixed at AES-256-CBC + HMAC-MD5 via CryptoPP. There is no cipher
  negotiation on the Mercury wire. The key is delivered out-of-band (SOAP auth response). Section 1 §1.4 is
  the authoritative reference.
- **Confidence**: High.

### C1-K: BW NetDriver replaces UE3 TcpNetDriver for game traffic

- **Path**: `game/sgw/Working/Engine/Config/BaseEngine.ini`: `NetworkDevice=IpDrv.BWNetDriver`
  Also: `game/sgw/Working/SGWGame/Config/DefaultEngine.ini`: `[Configuration] BasedOn=..\Engine\Config\GameplayEngine.ini`
  (inherits BaseEngine.ini through the chain)
  Binary RTTI: `.?AVUBWNetDriver@@` at `ghidra://SGW.exe@0x01dae780`, `.?AVUBWConnection@@` at `0x01dae79c`
  Binary string: `"BWNetDriver"` at `0x01801436`, `".\\Src\\BWNetDriver.cpp"` at `0x01801450`,
  `"IpDrv.BWNetDriver"` at `0x018e92bc`
  INI xref string: `"engine-ini:Engine.Engine.NetworkDevice"` at `ghidra://SGW.exe@0x018380a0`
- **Implication**: The standard UE3 net driver stack (`IpDrv.TcpNetDriver`) is bypassed entirely for game
  traffic. `IpDrv.BWNetDriver` (BigWorld's UDP Mercury driver) owns all connection and packet handling. The
  standard UE3 connection lifecycle (channel replication, actor relevancy, etc.) does NOT apply to SGW's game
  traffic. The [IpDrv.TcpNetDriver] section in the INI applies to editor/debug modes only, not the production
  game client.
- **Confidence**: High — confirmed by both INI and RTTI.

### C1-L: DebugCommunication — packet sniffer in Cimmeria's AtreaLoader

- **Path**: `game/sgw/Working/binaries/AtreaLoader.config.xml`
- **Snippet** (NVP section):
  ```xml
  <NVP Name="Sniffer" Value="true" />
  ```
- **Also**: `DebugCommunication.UDP` string at `ghidra://SGW.exe@0x017ffc08`/`0x017ffc50` — BigWorld's
  internal UDP debug communication class. The `DebugCommunication.cpp` source path string confirms this is
  BigWorld's debug packet-capture subsystem (not the Mercury core).
- **Implication**: The AtreaLoader (the community-built SGW.exe loader) has a packet sniffer that dumps `.pcap`
  files and AES key material to `sessions/DATE.pcap` and `sessions/DATE-keys.txt`. This is a
  client-side artifact of the community tooling, not an SGW game-design artifact, but confirms that the cipher
  material can be recovered from a running client session for wire-capture verification.

---

## Category 2 — INI-tunable parameters

These are keys the client reads from INI files at startup. They are NOT Mercury wire-format constants —
they control the UE3 net driver (which is bypassed for game traffic by `BWNetDriver`) or the UE3 game layer.

**Finding: No INI keys directly tune Mercury wire-format parameters.**

BigWorld's Mercury wire format (packet size, ack scheme, reliability, sequence numbers, cipher parameters)
is controlled entirely by compile-time constants in `Mercury::Nub`, `ChannelInternal`, `PacketEncrypter`,
and the `InterfaceElement` dispatch table — none of which are INI-readable in this binary.

The INI keys below are the complete set of network-adjacent keys found. Each is annotated with whether it
affects Mercury behavior at all.

### I2-A: Engine-level net driver keys (from BaseEngine.ini)

From `game/sgw/Working/Engine/Config/BaseEngine.ini`, section `[IpDrv.TcpNetDriver]`:

| Key | Value | Mercury relevance |
|---|---|---|
| `ConnectionTimeout` | `30.0` | UE3 TcpNetDriver only — bypassed by BWNetDriver. NOT a Mercury timeout. |
| `InitialConnectTimeout` | `200.0` | UE3 TcpNetDriver only — bypassed. |
| `AckTimeout` | `1.0` | UE3 TcpNetDriver only — bypassed. NOT the Mercury 700 ms ack timeout. |
| `MaxClientRate` | `15000` | UE3 throttle — irrelevant to Mercury bandwidth. |
| `MaxInternetClientRate` | `10000` | UE3 throttle — irrelevant. |
| `NetServerMaxTickRate` | `30` | UE3 server tick rate — irrelevant; Mercury tick rate is separate. |
| `LanServerMaxTickRate` | `35` | UE3 LAN tick rate — irrelevant. |
| `NetConnectionClassName` | `"IpDrv.TcpipConnection"` | UE3 TcpNetDriver class — bypassed. |
| `ConfiguredInternetSpeed` | `10000` | UE3 speed cap — irrelevant to Mercury. |
| `ConfiguredLanSpeed` | `20000` | UE3 LAN speed cap — irrelevant. |
| `MaxChannels` | `32` | UE3 channel count — NOT Mercury channels. |

Binary cross-check: `FUN_005dc280` (the UE3 NetDriver INI-property registration function) reads `ConnectionTimeout`,
`InitialConnectTimeout`, `KeepAliveTime`, `RelevantTimeout`, `MaxClientRate`, `MaxInternetClientRate`,
`NetServerMaxTickRate`, `NetConnectionClassName`. This confirms these keys are read and registered — but against
the UE3 `TcpNetDriver` class (section `[IpDrv.TcpNetDriver]` in the INI), which is NOT the active driver.

`KeepAliveTime` (string at `ghidra://SGW.exe@0x0184755c`) is a UE3 TcpNetDriver key with no corresponding
key in `[IpDrv.BWNetDriver]` or the game INIs. The Mercury keepalive is handled internally by
`UnAckedHandler__sendAckBundle2` (Section 1 `[^send-ack-bundle2]`), not by any INI-tunable timer.

### I2-B: UE3 game-layer inactivity timeout (from GameplayEngine.ini)

From `game/sgw/Working/Engine/Config/GameplayEngine.ini`, section `[Engine.Engine]`:

| Key | Value | Mercury relevance |
|---|---|---|
| `NetInactivityTimeout` | `15` | YES — UE3 game layer fires `REASON_INACTIVITY` disconnect after 15s idle. |

This is the only INI key with any indirect Mercury-wire-format relevance: a server that stops sending Mercury
traffic for 15+ seconds will trigger a client-side `REASON_INACTIVITY` disconnect (binary: `"REASON_INACTIVITY"`
string at `0x019d11f0`). The 15-second value is not tunable by the server side.

### I2-C: UDPBeacon keys (from BaseEngine.ini)

From `[IpDrv.UdpBeacon]`:

| Key | Value | Mercury relevance |
|---|---|---|
| `BeaconTimeout` | `5.0` | UE3 LAN beacon — no Mercury relevance |
| `ServerBeaconPort` | `8777` | UE3 LAN beacon port — no Mercury relevance |
| `BeaconPort` | `9777` | UE3 LAN beacon port — no Mercury relevance |

### I2-D: Suppress net traffic logging

From `BaseEngine.ini`: `Suppress=DevNetTraffic` — UE3 network traffic logging is suppressed by default.
This means client-side UE3 logs will not show net-traffic noise, but BigWorld's `MercuryLogger` (see
Category 6 below) is a separate log path.

**Summary for Category 2**: No INI key directly tunes Mercury wire format, ack timing, packet size, channel
parameters, or cipher settings. The Mercury layer's parameters are hard-coded constants (Category 1).
The INI layer controls UE3's bypassed TCP driver and a 15-second UE3-layer inactivity timer.

---

## Category 3 — UnrealScript-visible Mercury surface

**Finding: N/A — confirmed empty.**

The compiled UnrealScript packages (`game/sgw/Working/SGWGame/Content/FRScript/*.u`) are:
`Core.u`, `Editor.u`, `Engine.u`, `GFxUI.u`, `GFxUIEditor.u`, `IpDrv.u`, `SGWGame.u`, `UnrealEd.u`

These are binary UScript packages (not text-readable source). The INI search for any network-related class names
in text files returned zero Mercury-specific hits. No text-readable UScript source files exist under
`game/sgw/Working/SGWGame/Classes/` (that directory does not exist in the client tree).

Mercury is a C++ BigWorld library (`BWNetDriver.cpp`), not a UScript layer. The UScript layer communicates
with the game server exclusively via CME events (which fire through the C++ `ServerConnection` class),
with no direct UScript-to-Mercury API surface. The `IpDrv.BWNetDriver` INI reference confirms BigWorld's driver
is registered as a UE3 net driver class, but the actual Mercury socket/packet/channel code is C++ only.

This is consistent with the BigWorld architecture: Mercury is a low-level C++ transport that sits *below*
the UE3 scripting layer. UScript calls game-layer RPCs (CME events), which the C++ `ServerConnection` serializes
into Mercury bundles. No part of that serialization is accessible to UScript.

---

## Category 4 — What the client REQUIRES the server to do (wire-format invariants)

Violations of these produce silent drops, disconnects, or visible misbehavior.

### R1 — Flags byte at offset 0, always 1 byte (REQUIRED)

**Evidence**: `Mercury_Nub_ProcessFilteredPacket` at `0x01580840` reads offset 0 unconditionally as `uint8`.
Packet with a `uint16` flags prefix would be misinterpreted — the second byte would be read as the first byte
of the message body.

### R2 — Footer fields little-endian (REQUIRED)

**Evidence**: All footer pop operations use LE reads. String literal `"Nub::processFilteredPacket( %s ): received packet with bad flags %x\n"` at `0x01b17e98` is emitted on any flags byte the client cannot parse — a network-order flags byte would match this path immediately.

### R3 — FLAG_HAS_ACKS must not be set with 0 acks (REQUIRED)

**Evidence**: `"Nub::processFilteredPacket( %s ): Packet with FLAG_HAS_ACKS had 0 acks\n"` at `0x01b18110`.
Client emits a warning and the packet is dropped.

### R4 — Sequence numbers required for on-channel reliable packets (REQUIRED)

**Evidence**: `"Nub::processFilteredPacket( %s ): We should not be receiving a packet without a sequence number\n"`
at `0x01b183b0`.
`"Nub::processFilteredPacket( %s ): Dropping packet due to receiving a packet with null sequence number\n"`
at `0x01b18418`.
`"Nub::processFilteredPacket( %s ): Dropping packet due to receiving a packet with sequence number outside valid range\n"`
at `0x01b18480`.
These are HARD drops — the client discards the packet entirely.

### R5 — Fragmented bundles must have correct frag-begin/end footers (REQUIRED)

**Evidence**: Multiple drop strings:
`"Nub::processFilteredPacket( %s ): Not enough data for fragment end in footer"` at `0x01b18750`
`"Nub::processFilteredPacket( %s ): Not enough data for fragment begin in footer"` at `0x01b187b0`
`"Nub::processPacket( %s ): Dropping fragment due to illegal bundle fragment count (%d)"` at `0x01b18810`
`"Nub::processPacket( %s ): Mangled fragment footers, lastFragment(%d) != p->fragEnd()(%d)"` at `0x01b189a8`

### R6 — AES session key must match (REQUIRED)

**Evidence**: `"ServerConnection::authenticate: Unexpected key! (%s, wanted %s)\n"` at `0x019d0030`.
Authentication failure via `AuthenticateKeyComparison` at `0x00dd8510` results in connection teardown.
`"Not sending packet because of encryption error: %s\n"` at `0x01b27218` — encryption failure also drops.

### R7 — Channel must be registered before indexed-channel packets arrive (REQUIRED)

**Evidence**: `"Nub::processFilteredPacket( %s ): Client got indexed channel packet with no finder registered\n"`
at `0x01b18058` — hard drop.

### R8 — Piggyback chain must be well-formed if FLAG_HAS_PIGGYBACKS is set (REQUIRED)

**Evidence**: Multiple drop strings at `0x01b17f28`, `0x01b17f80`, `0x01b17ff8`.
Note: The Cimmeria server explicitly does not send piggybacks (`WARN_BAD_PACKET("Piggybacked packets are not supported")`) —
this is a server-side policy, not a client limitation. The client DOES parse piggybacks.

### R9 — Once-off reliable packets must follow the reliability rules (REQUIRED)

**Evidence**: `"Nub::processFilteredPacket( %s ): Dropping illegal once-off-reliable packet\n"` at `0x01b18568`.

### R10 — Server must keep Mercury traffic alive within ~15 seconds (REQUIRED for session persistence)

**Evidence**: `NetInactivityTimeout=15` in `GameplayEngine.ini`; `"REASON_INACTIVITY"` at `0x019d11f0`.
The client will emit a `loggedOff`-equivalent disconnect reason after 15 seconds of no received Mercury traffic.

### R11 — resetEntities must be sent in its own flushed bundle (REQUIRED)

**Evidence**: Section 1 §1.9 cites `bundle.beginMessage(BASEMSG_RESET_ENTITIES, Bundle::FLUSH)` — the
`FLUSH` argument means any prior bundle is sent before this message and a new bundle starts after. A server
that co-bundles `resetEntities` with other messages violates the wire-visible constraint.
`"ServerConnection::processInput: Dropped corrupted incoming packet\n"` at `0x019cfd18` would be the observable
failure mode.

### R12 — Time between packets is monitored and logged (INFORMATIONAL)

**Evidence**: `"ServerConnection::processInput: There were %d ms between packets\n"` at `0x019cfdb0`.
This is an INFO log, not a drop. The client observes inter-packet intervals; large gaps (>15s) trigger R10.

---

## Category 5 — What the client TOLERATES (variable behaviors)

### T1 — Tick rate variation

**Evidence**: `"updateFrequencyNotification"` (msg_id `0x02`, Section 1 §1.9.2) carries the tick rate as a
single byte. The client uses the advertised value to scale timers. As long as the server sends a consistent
`updateFrequencyNotification` on connect and maintains it, the client will tolerate any tick rate representable
in a `uint8` (1–255 ticks/second).

### T2 — Piggybacking is TOLERATED (not required)

**Evidence**: `"Nub::send( %s ): Piggybacked #%u (%d bytes) onto outgoing bundle\n"` at `0x01b17c78` — the
client's own send path does piggyback. The server is not required to piggyback; sending standalone ack bundles
is equally valid.

### T3 — Standalone ack packets (not piggybacked) are TOLERATED

**Evidence**: `UnAckedHandler__sendAckBundle2` at `0x0158bbc0` — the client sends standalone ack bundles when
there's no other traffic to piggyback onto. The server can expect both piggybacked and standalone acks.

### T4 — Out-of-order createBasePlayer / createCellPlayer within a bundle

**Evidence**: `"ServerConnection::createBasePlayer: Playing buffered createCellPlayer message"` at
`0x019d0110` — the client buffers an early `createCellPlayer` and replays it after `createBasePlayer` lands.

### T5 — bandwidthNotification value is IGNORED

**Evidence**: Section 1 §1.9.1 — "Not used by SGW (no bandwidth mutator)". The server must emit the message
(descriptor is registered), but the value has no effect.

### T6 — restoreClient is TOLERATED but not exercised in normal gameplay

**Evidence**: Section 1 §1.9.5 marks this "Untested" — the message is registered and the handler is decompiled,
but it is not observed in normal pcap traffic.

---

## Category 6 — Client-side diagnostic / logging surface

### D1 — MercuryLogger function

- **Path**: `game/sgw/Working/binaries/AtreaLoader.config.xml`
- **Snippet**:
  ```xml
  <Symbol Name="MercuryLogger" Address="0x0041C2E0" Group="Mercury" Patch="false" />
  ```
- **Also**: `EnableUnicodeLogger` patch at `0x01AF2224` — enables BW Unicode message logging. The patch
  changes a 4-byte value from `00 00 00 00` to `01 00 00 00` at that address.
- **Implication**: The client binary has a Mercury-specific logger at `0x0041C2E0` that the AtreaLoader can
  optionally enable. The `EnableUnicodeLogger` patch at `0x01AF2224` is the toggle for BigWorld's internal
  message logging. When enabled, the `AnsiLogger` at `0x00635210` and the Mercury-specific logger at
  `0x0041C2E0` both become active. **This address (`0x0041C2E0`) is a previously undocumented symbol —
  it is not in the Section 1 footnotes and not in the address-map.**
- **Confidence**: High (explicit XML annotation from the Cimmeria community tooling).

### D2 — Packet sniffer + AES key dumping

- **Path**: `game/sgw/Working/binaries/AtreaLoader.config.xml`
- **Snippet**:
  ```xml
  <NVP Name="Sniffer" Value="true" />
  ```
  Comment: `<!-- Saves dumps to sessions/DATE.pcap and AES keys to sessions/DATE-keys.txt -->`
- **Implication**: The AtreaLoader can capture full Mercury traffic to PCAP format and dump the AES session
  key. The sessions directory contains log files dated 2013-05-21 and 2026-04-27. This confirms wire-capture
  verification of Section 1 claims is possible against real traffic from a running SGW session.
- **Confidence**: High.

### D3 — SGWDebugLog and log4j logging

- **Path**: `game/sgw/Working/binaries/SGWLogConfig.xml`
- **Snippet**: log4j configuration writing to `SGWDebugLog.log` at priority `all`.
- **Implication**: The game launcher uses a Java-based log4j logger (SGW's launcher is `AtreaLoader.exe` — a
  Java app based on the log4j reference). SGW.exe's own debug output is captured separately by the `AnsiLogger`
  and `MercuryLogger` paths.

### D4 — processInput inter-packet timing

- **Path**: SGW.exe at `0x019cfdb0`
- **Snippet**: `"ServerConnection::processInput: There were %d ms between packets\n"`
- **Implication**: The client measures and logs the gap between received packets. Server developers can monitor
  this via the client-side log to detect protocol-level delivery gaps.

### D5 — Protocol digest logging at connection start

- **Path**: SGW.exe at `0x019cf1f8` / `0x019cf248`
- **Snippet**: `"ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s\n"`
- **Implication**: The client logs the server address, username, and a protocol digest at the start of
  connection. The `protocol_digest` field (also referenced at `0x019cff70` indirectly through the SOAP auth
  path) is likely the MD5/SHA hash of the entity-method interface table that guarantees both sides have the
  same RPC definitions. A mismatch here would cause a disconnect before Mercury traffic begins.

### D6 — Disconnect reason logging

- **Path**: SGW.exe at `0x019d0768` / `0x019d11f0`
- **Snippet**: `"ServerConnection::loggedOff: The server has disconnected us. reason = %d\n"` / `"REASON_INACTIVITY"`
- **Implication**: The client logs the numeric disconnect reason. `REASON_INACTIVITY` is the inactivity case;
  `LookupDisconnectReasonName` at `0x00de1623` maps other reason codes. Server developers can read the reason
  from the client log.

---

## Category 7 — Surprises / other findings

### S1 — AtreaLoader.config.xml is a binary-patching framework with explicit Mercury group

The AtreaLoader is a full SGW.exe patch tool that applies byte patches to the loaded binary at runtime.
The patches include editor-mode flags (`GIsServer`/`GIsEditor` manipulation), splash screen swaps, and crucially
a **Mercury group** with the `EnableUnicodeLogger` patch that enables internal BigWorld message logging.
The `MercuryLogger` symbol address `0x0041C2E0` is documented in this XML but is not in the Section 1
footnote catalog — it should be added to the address-map.

**Why this is a surprise**: The AtreaLoader.config.xml is a community-authored reverse-engineering tool that
provides independently-derived symbol names and binary addresses. These are a second source of Ghidra
annotations that complements the V5 RE campaign. The `MercuryLogger` symbol in particular is not in the
findings docs at all.

### S2 — The production launcher scripts pass a `-s PRODLIVE` / `-s PRODTEST` flag to Launcher.exe

- **Path**: `game/sgw/Working/Launcher-Production_Live.bat` → `start .\Launcher.exe -s PRODLIVE`
- **Path**: `game/sgw/Working/Launcher-Production_Test.bat` → `start .\Launcher.exe -s PRODTEST`
- **Implication**: The launcher binary (`AtreaLoader.exe` / `Launcher.exe`) switches between production-live
  and production-test server environments via a command-line flag. This suggests the Mercury connection target
  (server address/port) was determined by the launcher, not baked into SGW.exe. This is consistent with
  Mercury's `ServerConnection::logOnBegin` logging the server address at connect time.

### S3 — AtreaLoader.config.xml documents the EditorMode patch to flip GIsServer/GIsEditor/GIsClient flags

The patch at `0x00018AF0` manipulates the `GIsClient`, `GIsServer`, `GIsEditor`, `GIsUCC`, `GIsGame` global
flags by patching the `mov` instruction targets from their client-mode values to editor-mode values. This
confirms that SGW.exe contains a full Unreal Editor build in a dormant state — the same binary can run as
either the game client or the editor depending on these flag values. The Mercury layer in editor mode may
behave differently (the `IpDrv.TcpNetDriver` path may be active instead of `IpDrv.BWNetDriver`).

### S4 — Protocol digest (`protocol_digest`) at logon

The string `"ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s\n"` appears TWICE
at `0x019cf1f8` and `0x019cf248` (two different call sites for the same log). This suggests two code paths
call `logOnBegin` — one for the initial login attempt and one for a retry or re-auth case. The `protocol_digest`
is a hash of the interface descriptor table that ensures client/server protocol alignment. It is NOT a Mercury
wire-format field (it's part of the SOAP auth flow, not the Mercury packet format), but it does gate whether
Mercury traffic begins at all. If the server emits a wrong `protocol_digest` in its auth response, the client
will disconnect before any Mercury packets are exchanged.

### S5 — bandwidthFromServer mechanism is non-functional in SGW

- **Path**: SGW.exe at `0x019cff70`
- **Snippet**: `"ServerConnection::bandwidthFromServer: Cannot comply since no mutator set with 'setBandwidthFromServerMutator'\n"`
- **Implication**: The BigWorld base `bandwidthNotification` message (Section 1 §1.9.1) is wired into the
  client but the SGW game layer never calls `setBandwidthFromServerMutator`. Any attempt to throttle the client's
  send rate from the server via the `bandwidthNotification` message will be silently no-op'd. The message must
  still be emitted (its descriptor is registered) but its value has no behavioral effect.

---

## Section 1 Footnote Re-classification

86 unique footnote keys, classified against the "client requirement" lens.
Legend:
- REQUIRED — server MUST honor this for the client to function
- RECOMMENDED — best-practice; server can deviate and client copes
- TOLERATED — variable behavior client adapts to
- CLIENT-ONLY — artifact internal to the client, not part of the wire contract

| Footnote key | Classification | Rationale |
|---|---|---|
| `[^ack-bitmap]` | REQUIRED | 32-bit outstanding-ack bitmap governs server send window. Server must not exceed 32 in-flight reliable packets per channel. |
| `[^authenticate-handler]` | REQUIRED | Key mismatch causes hard disconnect. |
| `[^broadcast-entity-activation]` | REQUIRED | Client auto-emits `enableEntities` (8 bytes); server must understand the 8-byte body. |
| `[^bundle-add-blob]` | CLIENT-ONLY | Bundle payload copy implementation detail. |
| `[^bundle-clear]` | CLIENT-ONLY | Client-side bundle reset. |
| `[^bundle-ctor]` | CLIENT-ONLY | Client-side bundle construction. |
| `[^bundle-finalise]` | REQUIRED | Footer write order is the wire contract. Server must parse in the correct pop-order. |
| `[^bundle-new-message]` | REQUIRED | Max packet size 1453 bytes. |
| `[^bundle-reserve]` | CLIENT-ONLY | Client-side fragmentation policy. |
| `[^bundle-start-msg-fixed]` | REQUIRED | `CONSTANT_LENGTH` messages (like `enableEntities = 8`) must be emitted without a length prefix. |
| `[^bundle-start-msg-request]` | RECOMMENDED | Request messages use the request-chain linked list. Server must honor the `firstRequestOffset` footer field if it processes requests. |
| `[^channel-ctor]` | CLIENT-ONLY | Client-side channel setup — 512-entry dedup table is a client receive-side detail. |
| `[^channel-hash-alloc]` | CLIENT-ONLY | Client-side 512-entry receive dedup. |
| `[^channel-internal-ctor]` | CLIENT-ONLY | Client-side channel object initialization. |
| `[^channel-send]` | CLIENT-ONLY | Client-side send entry. |
| `[^check-nub-exception]` | REQUIRED | Resend/keepalive timer. Server must respond to reliable packets within ~700ms×20 retries or the channel will disconnect. |
| `[^cipher-hash-filter]` | REQUIRED | HMAC-MD5 filter must be honored exactly. |
| `[^cipher-stream-filter]` | REQUIRED | AES-256-CBC filter must be honored exactly. |
| `[^cipher-vtable]` | CLIENT-ONLY | Client-side vtable metadata. |
| `[^cipher-vtable-blocksize]` | CLIENT-ONLY | Client-internal OptimalBlockSize. |
| `[^cipher-vtable-dtor]` | CLIENT-ONLY | Client-internal destructor. |
| `[^cme-playchar]` | CLIENT-ONLY | CME event trigger for playCharacter — client-internal event system. |
| `[^compress-length-family]` | REQUIRED | The per-`InterfaceElement` fixed-width length encoding is a wire contract — server must match. |
| `[^cpp-client-handler]` | RECOMMENDED | Deprecated server C++ patterns confirm emit order; server should follow but client is robust to ordering variations. |
| `[^cpp-messages]` | REQUIRED | `messages.cpp` message descriptor registrations define the wire contract for each message ID. |
| `[^create-base-player-handler]` | REQUIRED | `createBasePlayer` wire layout (6 bytes: u32 entityId + u16 classId) is REQUIRED. |
| `[^create-cell-player-handler]` | REQUIRED | `createCellPlayer` wire layout (32 bytes, Y/Z rotation swap) is REQUIRED. |
| `[^cryptopp-rtti]` | CLIENT-ONLY | CryptoPP RTTI is a client binary implementation detail; the wire invariant is the cipher algorithm name, not the RTTI string. |
| `[^detailed-pos-handler]` | REQUIRED | `detailedPosition` (msg_id `0x30`) wire layout is REQUIRED if server emits this message. |
| `[^disconnect-handler]` | CLIENT-ONLY | Client-side disconnect logic. The server should not expect a courtesy disconnect from the client when it sends `loggedOff`. |
| `[^enable-entities-init]` | REQUIRED | `enableEntities` = CONSTANT_LENGTH = 8 bytes. This is the most-contested claim in the project; confirmed REQUIRED. |
| `[^event-net-proxy-data]` | CLIENT-ONLY | CME `Event_Net_ProxyData` is a client-internal callback event. |
| `[^flags-decoder]` | REQUIRED | The entire packet flags byte contract — bit assignments, pop order — is REQUIRED. |
| `[^forced-pos-handler]` | REQUIRED | `forcedPosition` wire layout (49 bytes; offsets 24-35 are previous-position reference not velocity) is REQUIRED. |
| `[^game-archaeology-2026-05-14]` | CLIENT-ONLY | Methodological reference to this RE session. |
| `[^gsoap-hex-dispatcher]` | REQUIRED | Key delivery via SOAP `xsd:hexBinary` (64-char hex → 32-byte key) is REQUIRED for cipher initialization. |
| `[^gsoap-type-dispatcher]` | REQUIRED | gSOAP type dispatch case `0x26` for `xsd:hexBinary` — REQUIRED. |
| `[^interface-element-size]` | CLIENT-ONLY | Client-internal `InterfaceElement` descriptor strides (`0x1c` vec / `0x24` dispatch). Not a wire invariant. |
| `[^logged-off-handler]` | REQUIRED | `loggedOff` wire layout (1-byte reason, `CONSTANT_LENGTH = 1`) is REQUIRED. Client tears down silently — server should not expect a reply. |
| `[^machguard-master-deserialize]` | RECOMMENDED | MachineGuard protocol — the client must be able to handle MachineGuard responses. RECOMMENDED if MachineGuard is used. |
| `[^machguard-send-raw]` | RECOMMENDED | Same — client-side MachineGuard send path. |
| `[^machguard-sendandrecv]` | REQUIRED | MachineGuard port = 20022 (`0x4E36`) is REQUIRED — this is a hard-coded constant in the binary. |
| `[^nub-add-listen-socket]` | CLIENT-ONLY | Client-side UDP socket setup. |
| `[^nub-ctor]` | CLIENT-ONLY | 24-step Nub construction — client-internal. |
| `[^nub-handle-message]` | RECOMMENDED | Request/reply matching via Nub — RECOMMENDED if the server uses the request/reply mechanism. |
| `[^nub-init-connmap]` | CLIENT-ONLY | Client-internal connection map initialization. |
| `[^nub-process-pending]` | CLIENT-ONLY | Client recv loop implementation. |
| `[^nub-send]` | CLIENT-ONLY | Client-side send entry. |
| `[^nub-write-connection]` | CLIENT-ONLY | Final `sendto()` on the client side. |
| `[^packed-string-reader]` | REQUIRED | The packed-string encoding (1-byte length, `0xFF`-escape to 3 bytes) is REQUIRED for the session key string in `AUTHENTICATE`. |
| `[^packet-chain-stamp-time]` | CLIENT-ONLY | Client-internal packet-chain traversal. |
| `[^packet-encrypter-ctor]` | REQUIRED | Zero IV per packet (not per-session, not randomly generated) is REQUIRED. Server must emit the same zero-IV ciphertext the client expects. |
| `[^packet-encrypter-recv]` | REQUIRED | AES-256-CBC decrypt + HMAC-MD5 verify is REQUIRED on both sides. |
| `[^packet-encrypter-send]` | REQUIRED | AES-256-CBC encrypt + HMAC-MD5 tag is REQUIRED on both sides. |
| `[^process-incoming-entry]` | CLIENT-ONLY | Client-internal `+0x58/+0x5c` send-alive timing stamp. |
| `[^process-ordered-packet]` | REQUIRED | Single-array dispatch by `msg_id` byte is the wire contract's delivery mechanism; server must emit valid `msg_id` bytes. |
| `[^purge-rebuild-handler]` | REQUIRED | `resetEntities` (msg_id `0x04`, CONSTANT_LENGTH = 1) wire layout is REQUIRED. |
| `[^queue-ack-for-packet]` | REQUIRED | Reliable packet ack scheduling is a wire contract obligation — receiver must ack or the sender retransmits. |
| `[^rdtsc-write-site]` | CLIENT-ONLY | Client-internal rdtsc baseline stamp. |
| `[^request-chain-walk]` | RECOMMENDED | Request linked-list walk — RECOMMENDED if server uses the request/reply mechanism. |
| `[^reset-entities-init]` | REQUIRED | `resetEntities` = CONSTANT_LENGTH = 1 byte (`keepBase u8`). REQUIRED. |
| `[^resource-fragment-handler]` | REQUIRED | `RESOURCE_FRAGMENT` (msg_id `0x36`) wire layout is REQUIRED if server sends PAK resources. |
| `[^restore-client-ack-descriptor]` | RECOMMENDED | `restoreClientAck` auto-reply — RECOMMENDED if `restoreClient` is used (server must register receipt of the ack reply). |
| `[^restore-client-handler]` | TOLERATED | `restoreClient` is marked untested in V5; not exercised in normal play. |
| `[^rotation-reader]` | REQUIRED | Y/Z rotation swap in `createCellPlayer` (`rotX, rotZ, rotY`) is REQUIRED. Wrong order produces wrong client orientation. |
| `[^send-ack-bundle2]` | CLIENT-ONLY | Client-side keepalive emit — client-internal policy. The server should accept the keepalive but not depend on a specific cadence. |
| `[^server-connection-send]` | CLIENT-ONLY | Client-side send chain entry. |
| `[^space-data-handler]` | TOLERATED | `spaceData` (msg_id `0x07`) is unused in current SGW builds. Server need not emit it. |
| `[^space-viewport-info-handler]` | REQUIRED | `spaceViewportInfo` (msg_id `0x08`, CONSTANT_LENGTH = 13) wire layout is REQUIRED. |
| `[^start-entity-message]` | REQUIRED | Cell-method wire shape (`msg_id | 0x80` + `word_len u16` + `entityId u32` + args) is REQUIRED. |
| `[^start-proxy-message]` | REQUIRED | Base-method wire shape (`msg_id | 0xC0` + `word_len u16` + args, NO entityId) is REQUIRED. Getting this wrong corrupts base-method arguments. |
| `[^stockbw-baseapp-ext]` | CLIENT-ONLY | Stock BW reference material — not a wire contract for SGW. |
| `[^stockbw-encryption]` | CLIENT-ONLY | Stock BW Blowfish — replaced in SGW, not relevant to wire contract. |
| `[^stockbw-interfaces]` | CLIENT-ONLY | Stock BW `InterfaceElement` sizes — inherited by SGW but not independently confirmed. |
| `[^stockbw-method-desc]` | CLIENT-ONLY | Stock BW sub-slot threshold reference. |
| `[^stockbw-packet-cpp]` | CLIENT-ONLY | Stock BW packet footer logic — SGW diverges (little-endian). Not a wire contract for SGW. |
| `[^stockbw-packet-hpp]` | CLIENT-ONLY | Stock BW `uint16` flags — SGW diverges (1-byte). Not a wire contract for SGW. |
| `[^subslot-threshold]` | REQUIRED | Sub-slot encoding threshold at 62 (`0x3E`) is REQUIRED. Method indices ≥ 62 use the `0xBD`/`0xFD` sentinel + sub_index encoding. |
| `[^v5-entity-creation]` | CLIENT-ONLY | V5 source-doc reference — not itself a wire constraint. |
| `[^v5-entity-property-sync]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^v5-mercury-internals]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^v5-position-movement]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^v5-space-viewport]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^v5-system-protocol]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^v5-world-entry]` | CLIENT-ONLY | V5 source-doc reference. |
| `[^write-components-varlen]` | REQUIRED | MachineGuard single-threshold component-ID encoding (`≤ 0xfe` → 1 byte, `> 0xfe` → `0xff` + 3 bytes) is REQUIRED for MachineGuard interop. |

### Re-classification summary (86 footnotes)

| Classification | Count |
|---|---|
| REQUIRED | 38 |
| RECOMMENDED | 7 |
| TOLERATED | 3 |
| CLIENT-ONLY | 38 |
| **Total** | **86** |

---

## Notes for Phase B (Documentation Writer)

1. **No INI key directly tunes Mercury** — Section 2 prose should be explicit that the Mercury wire format
   is entirely hard-coded. The INI keys belong to the UE3 TcpNetDriver that is bypassed at runtime.
   Section 2 is primarily a negative finding: "confirmed no INI surface for Mercury parameters."

2. **The `NetInactivityTimeout=15` key is the only client-configurable wire-adjacent parameter** and it
   controls the UE3-layer inactivity timer, not the Mercury keep-alive. The Chapter should distinguish
   the Mercury keepalive (100–700ms, binary-internal) from the UE3-layer disconnect (15s, INI-tunable).

3. **MercuryLogger at `0x0041C2E0`** is not in the Section 1 footnote catalog or the address-map. Phase B
   should flag this as a new symbol to add to `docs/reverse-engineering/address-map.md`.

4. **AtreaLoader.config.xml** is a first-class community-RE artifact. The `EnableUnicodeLogger` patch
   toggle at `0x01AF2224` has a precise binary address; the Chapter should mention it as the mechanism
   for enabling Mercury debug output from a running client.

5. **packetSizeMultiplier at `0x01acdb70`** — this string landed in a PhysX `NxFluidDesc` serializer
   context (`FUN_012487d0`), not in any Mercury code. The string is a false positive from BigWorld's
   physics-fluid descriptor (`kernelRadiusMultiplier`, `motionLimitMultiplier`, `packetSizeMultiplier`
   are all PhysX fluid simulation parameters). Section 2 should NOT cite this as a Mercury parameter.
   Confidence: high that this is a false positive.

6. **Controversial classification call**: `[^send-ack-bundle2]` is classified CLIENT-ONLY because it describes
   the *client's* keepalive emit policy. The server does not need to replicate that exact policy — it just
   needs to accept keepalive bundles. The server's own keepalive obligations are covered by R10 (NetInactivity
   timeout). The user should bless this distinction before authoring.

7. **The `0x708` and `0xa28` constants in `UNetConnectionBase__vfunc_75`** deserve a footnote:
   - LAN speed floor: `0x708` = 1800 bits/sec
   - Internet default when zero: `0xa28` = 2600 bits/sec
   These are UE3 net connection speed settings applied to the `UBWConnection` (the UE3 wrapper around the BW
   Mercury channel). They may affect UE3's own throttle behavior but do not directly gate Mercury packet
   emission rates. Low confidence that these matter for Mercury behavior in practice.

8. **Phase B should explicitly acknowledge Category 3 as confirmed empty.** The absence of any UnrealScript
   Mercury surface is a strong architectural finding: BigWorld's transport is completely invisible to UScript.
   This is worth a sentence in the chapter.

---

## Directories and files scanned

- `game/sgw/Working/Engine/Config/` — all 14 INI files
- `game/sgw/Working/SGWGame/Config/` — all 13 INI files (+ Linux subdirectory)
- `game/sgw/Working/SGWGame/Content/XML/` — 3 XML files (SystemOptions.xml, BindableActions*.xml)
- `game/sgw/Working/SGWGame/Content/FRScript/` — 8 compiled .u packages (binary, not text-readable)
- `game/sgw/Working/binaries/` — AtreaLoader.config.xml, SGWLogConfig.xml, launcher .bat files
- `game/sgw/Working/binaries/sessions/` — 2 dated log files, 1 empty Rashalgal.txt
- SGW.exe string scan via Ghidra MCP — ~400+ strings examined; ~60 directly relevant to Mercury
- Section 1 footnotes — all 86 unique keys re-classified

**Total hit files**: 7 files with meaningful findings (GameplayEngine.ini, BaseEngine.ini,
AtreaLoader.config.xml, SGWLogConfig.xml, Launcher-Production_Live.bat, Launcher-Production_Test.bat,
SystemOptions.xml for negative confirmation).

**Total distinct findings**: 31 (C1-A through C1-L + I2-A through I2-D + R1-R12 + T1-T6 + D1-D6 + S1-S5,
minus overlaps with Section 1).
