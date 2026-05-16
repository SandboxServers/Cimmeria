---
name: mercury-section-2-track-b-evidence
description: Track B discovery manifest for Mercury §2 — Ghidra evidence for R11–R15 error behaviors, INI key reality, system-message handler RTTI catalog, full error/log string catalog, and protocol_digest computation chain. Feeds Track C doc-writer pass.
metadata:
  type: project
---

# Mercury §2 Track B — Discovery Manifest

**Date**: 2026-05-15
**Investigator**: Game Archaeology Specialist
**Purpose**: Fills gaps identified by reviewer MyPalClara ("a survey, not a spec"). Track C (doc-writer) consumes this to bring §2 to white-paper depth matching §1. Chapter NOT modified.

---

## Investigation 1 — R11–R15 Client Behavior Under Violation

### Finding 1.1 — R11: No recv-side size gate in Mercury layer

**Location**: `Mercury_Nub_ProcessPendingEvents` at `0x01581ab0`; `Mercury_Nub_ProcessFilteredPacket` at `0x01580840`

**Evidence**: `processFilteredPacket` validates minimum 2 bytes ("received undersize packet (%d bytes)" at `0x01b17ee0`) but places NO upper bound on incoming packet size. The 1453-byte cap is documented only for the send path (`Bundle::newMessage`). The `processPendingEvents` recv loop passes the raw `recvfrom` byte count directly to the packet parser.

**Behavior**: Client does NOT reject oversized packets at the Mercury recv layer. A server sending a packet larger than 1453 bytes will be parsed without error — the constraint is purely a send-side invariant. The R11 requirement in the spec is a server obligation with no client enforcement.

**Closes**: R11 ("max packet size 1453")
**Suggested-footnote-slug**: `fn-r11-no-recv-gate`

---

### Finding 1.2 — R12: Out-of-order sequence ID handling — four distinct behaviors

**Location**: `UnAckedHandler::queueAckForPacket` (string refs at `0x01b19e78`–`0x01b1a040`), within `FUN_0158cba0`

**Evidence** (all strings from `FUN_0158cba0`):
- `0x01b19e78`: "Got out-of-range incoming seq #%d (inSeqAt: #%d)" — range check failure
- `0x01b19ed8`: "Pushing %d unsent ACKs due to inactivity" — inactivity-triggered flush
- `0x01b19f30`: "Discarding already-seen packet #%d below inSeqAt #%d" — below-window discard
- `0x01b19f90`: "Sequence number #%d is way out of window #%d!" — far-out-of-window warning
- `0x01b19fe8`: "Discarding already-buffered packet #%d" — duplicate-in-buffer discard
- `0x01b1a040`: "Buffering packet #%d above #%d" — out-of-order buffering (held for reorder)

**Behavior**: Client implements a sliding-window reorder buffer. Packets below `inSeqAt` are silently discarded. Packets "way out of window" generate a warning log but are not immediately fatal. Packets above expected are buffered. The client tolerates out-of-order delivery within the window rather than disconnecting.

**Closes**: R12 ("out-of-order sequence ID handling")
**Suggested-footnote-slug**: `fn-r12-reorder-window`

---

### Finding 1.3 — R13: Fragment reassembly abandonment — arrival-triggered, not timer-triggered

**Location**: `Mercury_Nub_ProcessPacket` at `0x0157fd20` (stale-overlapping path); `Mercury_Channel_cleanup` at `0x0158d050` (channel teardown)

**Evidence**:
- `0x01b18868`: "Discarding abandoned stale overlapping fragmented bundle from seq %d to %d" — triggered when new fragmented bundle range overlaps an in-progress reassembly
- `0x01b18928`: "Discarding unreliable fragment #%d (#%d,#%d) while waiting for reliable chain (#%d,#%d) to complete" — mixed-reliability conflict path
- `0x01b189a8`: "Mangled fragment footers, lastFragment(%d) != p->fragEnd()(%d)" — footer consistency check
- `0x01b18a04`: "Discarding duplicate fragment #%d" — dedup
- `0x01b1a090`: "Channel::~Channel( %s ): Forgetting %d unprocessed packets in the fragment chain" — channel teardown frees incomplete reassemblies

**Behavior**: Stale fragment abandonment is triggered on the NEXT arrival of an overlapping bundle, not by a periodic timer sweep. The chapter's claim of "stale assemblies swept at 30s" is NOT confirmed by this binary — no periodic sweep timer was found in the Nub tick loop. Incomplete reassemblies are also freed on channel destruction. **Pending verification**: whether a separate sweep timer exists elsewhere in the Nub tick path.

**Closes**: R13 ("fragment reassembly abandonment")
**Suggested-footnote-slug**: `fn-r13-arrival-triggered-abandon`

---

### Finding 1.4 — R14: Retransmit abort — 5-retry cap, NOT 20

**Location**: `FUN_0158c420` (= `UnAckedHandler::checkResendTimers`) at `0x0158c420`

**Evidence**:
- String ref at `0x0158c5a2` → `0x01b19dd8`: "UnAckedHandler::checkResendTimers( %s ): Aborting due to failed resend for #%d (%s)"
- Float constant `_DAT_01e91e00` at `0x01e91e00`: bytes `00 00 A0 40` = IEEE 754 **5.0**
- Comparison: `if (_DAT_01e91e00 < fVar2)` where `fVar2 = (float)local_20` and `local_20` counts processed retransmit entries per tick
- Abort path calls `LookupDisconnectReasonName(iVar5)` then returns `iVar5` (the disconnect reason code)

**Behavior**: `checkResendTimers` iterates the unacked-packet list. When `local_20 > 5.0f`, it caps iteration and returns 0 (no disconnect from this function alone — the cap is a per-tick work budget, not a retry-count disconnect). The actual abort is triggered when `ChannelInternal__processIncomingPacketEntry` returns non-zero for a resend attempt. **Correction to chapter**: the "20 retries" figure is NOT confirmed; the float constant is 5.0, which appears to be a per-tick resend processing cap rather than a lifetime retry limit. The abort-on-failed-resend string exists but the max-retry-count check that produces a "disconnect after N failures" was not found in this pass.

**Closes**: R14 ("client retransmit loop and max-retry disconnect") — partially; lifetime retry cap address still unknown
**Suggested-footnote-slug**: `fn-r14-retry-cap-5f`

---

### Finding 1.5 — R15: Protocol version / channel establishment — SOAP-layer, not Mercury-layer

**Location**: `FUN_015f8410` (SOAP login session) at `0x015f8410`; `FUN_00ddf580` (`logOnBegin`) at `0x00ddf580`

**Evidence**:
- `0x01b2507c`, `0x01b25384`, `0x01b25ad8`: three occurrences of `"sgwLogin:ProtocolDigest"` — SOAP XML field name used in login request body; xref to `FUN_015f8410`
- `0x019ab2b0`: `"LoginMessage_LoginBadProtocolVersion"` — login reply code; xref at `0x01df2784` (message-name enum table)
- `0x019ab408`: `"LoginMessage_LoginRejectedBadDigest"` — distinct reply code for digest mismatch
- `0x01df15dc`: `.?AUEvent_Net_GetProtocolDigest@@` — CME event type for querying digest (signals the digest was also surfaced via event system)
- Full LoginMessage enum at `0x019aaf34`–`0x019ab460` (31 entries)

**Behavior** (corrected 2026-05-15 — supersedes the single-hash narrative below in Investigation 5): Protocol version / digest validation is entirely SOAP-layer. The wire `protocol_digest` is supplied to `logOnBegin` as `param_4` from the CME `Event_Net_GetProtocolDigest` event listener (MD5, 32-char hex) and then embedded in the SOAP `sgwLogin:ProtocolDigest` XML field. A SEPARATE CryptoPP HexEncoder pipeline inside `logOnBegin` computes an internal SHA-1 dispatch-table commitment at `ServerConnection+0x130` (40-char hex) — that value is NOT sent on the wire. A mismatch produces `LoginMessage_LoginBadProtocolVersion` or `LoginMessage_LoginRejectedBadDigest` in the SOAP reply — NO Mercury packets are exchanged before this check succeeds. There is no Mercury-layer handshake version field. See Investigation 5 for the full two-hash decomposition.

**Closes**: R15 ("channel establishment / protocol version check")
**Suggested-footnote-slug**: `fn-r15-soap-only-version`

---

## Investigation 2 — §2.3 INI Key Behavior

### Finding 2.1 — Confirmed: No INI key directly tunes Mercury

**Location**: Phase A manifest (`mercury-section-2-discovery.md`), confirmed by this session's `FUN_005dc280` analysis

**Evidence**: `TcpNetDriver` INI key registration (`MaxClientRate`, `MaxInternetClientRate`, etc.) applies only to UE3's TCP driver path, which is bypassed by `NetworkDevice=IpDrv.BWNetDriver`. `NetInactivityTimeout=15` remains the only INI-adjacent wire parameter with any Mercury relevance (UE3-layer only, not Mercury-layer).

**Behavior**: §2.3 hard-coded constants table (not an INI section) is accurate. No additions needed.
**Suggested-footnote-slug**: `fn-i2-ini-is-dead` (already in Phase A manifest)

---

## Investigation 3 — §2.5 System-Message Descriptor Table

### Finding 3.1 — Handler RTTI catalog: 46 ClientInterface system messages confirmed

**Location**: RTTI strings at `0x01e52088`–`0x01e53050` (46 entries); populated at runtime via `PopulateMessageTypeTable` at `0x00dd63d0` from `DAT_01ef2518` (BSS vec, zero at static analysis time)

**Evidence**: `DAT_01ef2518` is a BSS-allocated vec (all zeros in static image). Its contents are populated at startup by `InterfaceElementVec__copyAllTo` (`0x01577f20`) iterating at stride `0x1c`, handler pointer at vec-entry offset `+0x0c`. The RTTI table (`0x01e52088`–`0x01e53050`) reveals the full system-message handler name set. The msg_id-to-name mapping requires a live-memory dump or reconstruction from the registration order.

**RTTI-confirmed system message handlers** (from `0x01e52088`, in address order):
| Address | Handler name |
|---|---|
| 0x01e52088 | `ClientMessageHandler<bandwidthNotificationArgs>` |
| 0x01e520e0 | `ClientMessageHandler<updateFrequencyNotificationArgs>` |
| 0x01e52138 | `ClientMessageHandler<setGameTimeArgs>` |
| 0x01e52180 | `ClientMessageHandler<resetEntitiesArgs>` |
| 0x01e521d0 | `ClientMessageHandler<spaceViewportInfoArgs>` |
| 0x01e52220 | `ClientMessageHandler<entityInvisibleArgs>` |
| 0x01e52270 | `ClientMessageHandler<tickSyncArgs>` |
| 0x01e522b8 | `ClientMessageHandler<setSpaceViewportArgs>` |
| 0x01e52308 | `ClientMessageHandler<setVehicleArgs>` |
| 0x01e52350 | `ClientMessageHandler<avatarUpdateNoAliasFullPosYawPitchRollArgs>` |
| 0x01e523b8 | `ClientMessageHandler<avatarUpdateNoAliasFullPosYawPitchArgs>` |
| 0x01e52418 | `ClientMessageHandler<avatarUpdateNoAliasFullPosYawArgs>` |
| 0x01e52478 | `ClientMessageHandler<avatarUpdateNoAliasFullPosNoDirArgs>` |
| 0x01e524d8 | `ClientMessageHandler<avatarUpdateNoAliasOnChunkYawPitchRollArgs>` |
| 0x01e52540 | `ClientMessageHandler<avatarUpdateNoAliasOnChunkYawPitchArgs>` |
| 0x01e525a0 | `ClientMessageHandler<avatarUpdateNoAliasOnChunkYawArgs>` |
| 0x01e52600 | `ClientMessageHandler<avatarUpdateNoAliasOnChunkNoDirArgs>` |
| 0x01e52660 | `ClientMessageHandler<avatarUpdateNoAliasOnGroundYawPitchRollArgs>` |
| 0x01e526c8 | `ClientMessageHandler<avatarUpdateNoAliasOnGroundYawPitchArgs>` |
| 0x01e52728 | `ClientMessageHandler<avatarUpdateNoAliasOnGroundYawArgs>` |
| 0x01e52788 | `ClientMessageHandler<avatarUpdateNoAliasOnGroundNoDirArgs>` |
| 0x01e527e8 | `ClientMessageHandler<avatarUpdateNoAliasNoPosYawPitchRollArgs>` |
| 0x01e52850 | `ClientMessageHandler<avatarUpdateNoAliasNoPosYawPitchArgs>` |
| 0x01e528b0 | `ClientMessageHandler<avatarUpdateNoAliasNoPosYawArgs>` |
| 0x01e52908 | `ClientMessageHandler<avatarUpdateNoAliasNoPosNoDirArgs>` |
| 0x01e52968 | `ClientMessageHandler<avatarUpdateAliasFullPosYawPitchRollArgs>` |
| 0x01e529d0 | `ClientMessageHandler<avatarUpdateAliasFullPosYawPitchArgs>` |
| 0x01e52a30 | `ClientMessageHandler<avatarUpdateAliasFullPosYawArgs>` |
| 0x01e52a88 | `ClientMessageHandler<avatarUpdateAliasFullPosNoDirArgs>` |
| 0x01e52ae8 | `ClientMessageHandler<avatarUpdateAliasOnChunkYawPitchRollArgs>` |
| 0x01e52b50 | `ClientMessageHandler<avatarUpdateAliasOnChunkYawPitchArgs>` |
| 0x01e52bb0 | `ClientMessageHandler<avatarUpdateAliasOnChunkYawArgs>` |
| 0x01e52c08 | `ClientMessageHandler<avatarUpdateAliasOnChunkNoDirArgs>` |
| 0x01e52c68 | `ClientMessageHandler<avatarUpdateAliasOnGroundYawPitchRollArgs>` |
| 0x01e52cd0 | `ClientMessageHandler<avatarUpdateAliasOnGroundYawPitchArgs>` |
| 0x01e52d30 | `ClientMessageHandler<avatarUpdateAliasOnGroundYawArgs>` |
| 0x01e52d90 | `ClientMessageHandler<avatarUpdateAliasOnGroundNoDirArgs>` |
| 0x01e52df0 | `ClientMessageHandler<avatarUpdateAliasNoPosYawPitchRollArgs>` |
| 0x01e52e50 | `ClientMessageHandler<avatarUpdateAliasNoPosYawPitchArgs>` |
| 0x01e52eb0 | `ClientMessageHandler<avatarUpdateAliasNoPosYawArgs>` |
| 0x01e52f08 | `ClientMessageHandler<avatarUpdateAliasNoPosNoDirArgs>` |
| 0x01e52f60 | `ClientMessageHandler<detailedPositionArgs>` |
| 0x01e52fb0 | `ClientMessageHandler<forcedPositionArgs>` |
| 0x01e53000 | `ClientMessageHandler<controlEntityArgs>` |
| 0x01e53050 | `ClientMessageHandler<loggedOffArgs>` |

**Note**: `DAT_01ef2518` is BSS-zero at static analysis time; the actual msg_id assignment order requires live-memory inspection or tracing the registration loop in `PopulateMessageTypeTable`. The 46 handler names above are confirmed from RTTI. Entity messages (`0x80`–`0xFE`) are all registered to the same generic handler (`PTR_vftable_01e51cbc`).

**Closes**: Investigation 3 (§2.5 descriptor table) — handler names confirmed; msg_id ordering is an open question
**Suggested-footnote-slug**: `fn-i3-bss-vec-runtime-only`

---

## Investigation 4 — Error/Log String Catalog

### Finding 4.1 — Complete Nub::processFilteredPacket string catalog

**Location**: `0x01b17e98`–`0x01b18568` (24 strings)

| Address | String |
|---|---|
| 0x01b17e98 | "received packet with bad flags %x" |
| 0x01b17ee0 | "received undersize packet (%d bytes)" |
| 0x01b17f28 | "Not enough data for piggyback length (%d bytes left)" |
| 0x01b17f80 | "Packet too small to contain piggyback message of length %d (only %d bytes remaining)" |
| 0x01b17ff8 | "Got an exception whilst processing piggyback packet: %s" |
| 0x01b18058 | "Client got indexed channel packet with no finder registered" |
| 0x01b180b8 | "Not enough data for ack count footer (%d bytes left)" |
| 0x01b18110 | "Packet with FLAG_HAS_ACKS had 0 acks" |
| 0x01b18158 | "Not enough footers for %d acks (have %d bytes but need %d)" |
| 0x01b181b8 | "Not enough data for ack in footer (%d bytes left)" |
| 0x01b18210 | "delResendTimer() failed for #%d" |
| 0x01b18258 | "Not enough data for ack(noncontiguous) in footer (%d bytes left)" |
| 0x01b182c0 | "Recieved a once off reliable message" [sic — original typo] |
| 0x01b18308 | "Got %d acks without a channel" |
| 0x01b18350 | "Not enough data for sequence number footer (%d bytes left)" |
| 0x01b183b0 | "We should not be receiving a packet without a sequence number" |
| 0x01b18418 | "Dropping packet due to receiving a packet with null sequence number" |
| 0x01b18480 | "Dropping packet due to receiving a packet with sequence number outside valid range" |
| 0x01b184f8 | "Dropping incoming packet #%d of size %d due to absence of local channel" |
| 0x01b18568 | "Dropping illegal once-off-reliable packet" |
| 0x01b18750 | "Not enough data for fragment end in footer (%d bytes left)" |
| 0x01b187b0 | "Not enough data for fragment begin in footer (%d bytes left)" |

### Finding 4.2 — Nub::processPacket fragment string catalog

**Location**: `0x01b18698`–`0x01b18a04` (7 strings)

| Address | String |
|---|---|
| 0x01b18698 | "Not enough data for first request offset (%d bytes left)" |
| 0x01b186f0 | "Not enough footers for fragment spec (have %d bytes but need %d)" |
| 0x01b18810 | "Dropping fragment due to illegal bundle fragment count (%d)" |
| 0x01b18868 | "Discarding abandoned stale overlapping fragmented bundle from seq %d to %d" |
| 0x01b18928 | "Discarding unreliable fragment #%d (#%d,#%d) while waiting for reliable chain (#%d,#%d) to complete" |
| 0x01b189a8 | "Mangled fragment footers, lastFragment(%d) != p->fragEnd()(%d)" |
| 0x01b18a04 | "Discarding duplicate fragment #%d" |

### Finding 4.3 — UnAckedHandler string catalog

**Location**: `0x01b19dd8`–`0x01b1a040` (8 strings)

| Address | String |
|---|---|
| 0x01b19dd8 | "UnAckedHandler::checkResendTimers( %s ): Aborting due to failed resend for #%d (%s)" |
| 0x01b19e30 | "UnAckedHandler::resetLocalPart( %s ): Forgetting %d unacked packet(s)" |
| 0x01b19e78 | "UnAckedHandler::queueAckForPacket( %s ): Got out-of-range incoming seq #%d (inSeqAt: #%d)" |
| 0x01b19ed8 | "UnAckedHandler::queueAckForPacket( %s ): Pushing %d unsent ACKs due to inactivity" |
| 0x01b19f30 | "UnAckedHandler::queueAckForPacket( %s ): Discarding already-seen packet #%d below inSeqAt #%d" |
| 0x01b19f90 | "UnAckedHandler::queueAckForPacket( %s ): Sequence number #%d is way out of window #%d!" |
| 0x01b19fe8 | "UnAckedHandler::queueAckForPacket( %s ): Discarding already-buffered packet #%d" |
| 0x01b1a040 | "UnAckedHandler::queueAckForPacket( %s ): Buffering packet #%d above #%d" |

### Finding 4.4 — Channel teardown and NubException disconnect strings

| Address | String |
|---|---|
| 0x01b1a090 | "Channel::~Channel( %s ): Forgetting %d unprocessed packets in the fragment chain" |
| 0x01b19248 | "Wrote MGMPacket %d bytes long/%d messages. You need to implement MGM fragmentation" |

### Finding 4.5 — NubException reason-code strings

**Location**: `Mercury_Nub_ProcessPendingEvents` at `0x01581ab0`; `ChannelInternal__checkAndSendNubException` at `0x0158bed0`

| Reason code | Meaning | Trigger |
|---|---|---|
| `0xFFFFFFFE` | REASON_NETWORK_UNREACHABLE | WSA errors 0x274d (WSAEHOSTUNREACH), 0x2751 (WSAENETUNREACH), 0x2746 (WSAECONNRESET) |
| `0xFFFFFFFD` | REASON_GENERAL_NETWORK | Other WSA errors; log: "Nub::processPendingEvents: Throwing REASON_GENERAL_NETWORK (1)- %s" |
| `0xFFFFFFF9` | (receive-timeout) | Thrown by `checkAndSendNubException` when rdtsc delta at `+0x174/+0x170` exceeds threshold at `+0x164/+0x160` |

### Finding 4.6 — LoginMessage full enum (31 entries confirmed)

**Location**: `0x019aaf34`–`0x019ab460` (31 strings)

Key entries for Mercury chapter:
| Address | Name |
|---|---|
| 0x019ab2b0 | `LoginMessage_LoginBadProtocolVersion` |
| 0x019ab138 | `LoginMessage_DefsDigestMismatch` |
| 0x019ab408 | `LoginMessage_LoginRejectedBadDigest` |
| 0x019aaf34 | `LoginMessage_LoggedOn` |
| 0x019aaf60 | `LoginMessage_ConnectionFailed` |

### Finding 4.7 — protocol_digest / authentication string catalog

| Address | String / RTTI |
|---|---|
| 0x019cf1f8 | "ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s\n" |
| 0x019cf248 | "ServerConnection::logOnBegin: server:%s username:%s protocol_digest: %s\n" (duplicate — two call sites) |
| 0x01b2507c | "sgwLogin:ProtocolDigest" (SOAP XML field; xref → `FUN_015f8410`) |
| 0x01b25384 | "sgwLogin:ProtocolDigest" (second occurrence) |
| 0x01b25ad8 | "sgwLogin:ProtocolDigest" (third occurrence) |
| 0x01b260c8 | "ProtocolDigest" |
| 0x01b26104 | "ProtocolDigest" |
| 0x01df1590 | `.?AV?$CallbackImpl@UEvent_Net_GetProtocolDigest@@@EventSignal@CME@@` (RTTI) |
| 0x01df15dc | `.?AUEvent_Net_GetProtocolDigest@@` (RTTI — event struct) |

---

## Investigation 5 — `logOnBegin` digest pipeline (CORRECTED 2026-05-15)

> ⚠️ **CORRECTION**: An earlier version of this investigation asserted that `logOnBegin`'s internal `HexEncoder` pipeline produces the wire `protocol_digest` and stores it at `this+0x130`. **That conflated two distinct hashes.** Investigation 6 (later same session) plus the AteraLoader session log (literal `"protocol_digest: 58AFA196AD3AC4F65CADD99BFF23B799"`) corrected the model:
>
> - The **wire `protocol_digest`** is **MD5 (32 chars)**, sourced from the CME `Event_Net_GetProtocolDigest` event listener (raised by `SGWNetworkManager` at `0x00c6e3a0` upstream of `logOnBegin`) and passed as `param_4` to `logOnBegin`. It is embedded in the SOAP body as `sgwLogin:ProtocolDigest`.
> - The **40-char SHA-1 stored at `this+0x130`** is the **internal dispatch-table commitment**, computed by the CryptoPP `HexEncoder` pipeline INSIDE `logOnBegin` (after `PopulateMessageTypeTable` runs). It is **NOT sent on the wire**.
>
> Both hashes are uppercase hex output from CryptoPP, and both code paths live inside `logOnBegin`, which is why earlier passes conflated them. The breakthrough was the AteraLoader log capturing the 32-char wire value verbatim.

### Finding 5.1 — Internal dispatch-table hash pipeline

**Location**: `FUN_00ddf580` (`logOnBegin`) at `0x00ddf580`; `PopulateMessageTypeTable` at `0x00dd63d0`; `ConstructHexEncoder` at `0x00de41a0`

**Evidence chain** (produces the **internal** SHA-1 at `this+0x130`, NOT the wire MD5):

1. `logOnBegin` checks `*(int *)(this + 0x30c) == 0` (connection-state gate; must be zero = not yet connected)
2. Calls `PopulateMessageTypeTable(this, this+400, ...)` — builds InterfaceElement dispatch table in `this+0x190`
3. `PopulateMessageTypeTable` calls `InterfaceElementVec__copyAllTo(&DAT_01ef2518, pThis)` then loops `0x80..0xFE` registering entity handlers
4. Creates `CryptoPP::StringSinkTemplate` to receive hex output (`local_ac`)
5. Calls `ConstructHexEncoder(local_f4, pThis, 1, 0, ...)` — CryptoPP HexEncoder configured with `Uppercase=true`
6. Executes encoder via vtable `(**(code **)(*piVar3 + 0x2c))()`
7. Stores result via `std::basic_string::operator=` at `this+0x130` — **this is the internal `dispatch_table_digest` field**, NOT the wire `protocol_digest`
8. Calls `SetupSGWLoginRequestCurlSession` — **passes `param_4` (the MD5 wire digest from upstream)**, NOT `this+0x130`, as the SOAP body's `sgwLogin:ProtocolDigest` field

**ConstructHexEncoder details** (`0x00de41a0`): allocates `BaseN_Encoder` (0x3c bytes) + `Grouper` (0x38 bytes), stamps `CryptoPP::HexEncoder::vftable`, sets Uppercase=true.

### Finding 5.2 — Wire `protocol_digest` (MD5) provenance

The wire `protocol_digest` arrives in `logOnBegin` as `param_4` from the caller `SGWNetworkManager` at `ghidra://SGW.exe@0x00c6e3a0`. That caller asserts `"digestEvent.isHandled"` from `.\Src\SGWNetworkManager.cpp:0x21f`, confirming the digest is produced by a CME `Event_Net_GetProtocolDigest` event listener (RTTI at `0x01df15dc`). The listener itself (which performs the MD5 computation over the entity-defs) was not directly decompiled in this pass — see Section 2 §2.4 R16 follow-up for that gap.

### Mismatch behavior

The server validates the digest in the SOAP reply. On mismatch, the server returns `LoginMessage_LoginBadProtocolVersion` (`0x019ab2b0`) or `LoginMessage_LoginRejectedBadDigest` (`0x019ab408`). The client receives this as a SOAP response and surfaces it via the `LoginMessage` enum — no Mercury connection has been established at this point. There is NO Mercury-layer protocol version handshake. The client does not attempt reconnect after a bad-digest SOAP rejection.

### Second branch

`*(int *)(this + 0x30c) != 0`: calls `ConstructLoginReplyHandlerMinimal` instead — handles the reconnect / already-in-progress case.

### `Event_Net_GetProtocolDigest` (`0x01df15dc`)

The CME event used by `SGWNetworkManager` to obtain the wire MD5 digest from a listener. Game-layer code can also query the current digest via the same event without going through `logOnBegin` directly. **The listener that produces the digest** (server-side of the event bus) is the location of the actual MD5 computation; finding it is open work tracked in Section 2 §2.4 R16.

**Closes**: Investigation 5 (corrected, two-hash model)
**Suggested-footnote-slug**: `fn-i5-two-digest-pipeline`

---

## New Binary Symbols Discovered This Session

| Symbol | Address | Notes |
|---|---|---|
| `UnAckedHandler::checkResendTimers` | `0x0158c420` | resend iteration + abort; retransmit cap = 5.0f float at `0x01e91e00` |
| `UnAckedHandler::queueAckForPacket` | `0x0158cba0` | reorder-window logic; sliding-window reorder buffer |
| `FUN_015f8410` (SOAP login session) | `0x015f8410` | builds SOAP request with `sgwLogin:ProtocolDigest` field |
| `ConstructLoginReplyHandlerMinimal` | (callee of `logOnBegin` second branch) | handles reconnect path |
| `LookupDisconnectReasonName` | (callee in `0x0158c420`) | maps reason code to name string |
| `LoginMessage` enum | `0x019aaf34`–`0x019ab460` | 31 entries confirmed; `LoginRejectedBadDigest` is separate from `LoginBadProtocolVersion` |
| RTTI: `Event_Net_GetProtocolDigest` | `0x01df15dc` | CME event for digest query |

---

## Open Questions for Track C

1. **R13 sweep timer**: The chapter claims "stale assemblies swept at 30s." This was NOT confirmed — only arrival-triggered abandonment found. Track C should either verify the timer exists (needs Nub tick-loop decompile) or correct the claim to "arrival-triggered only."

2. **R14 lifetime retry cap**: The 5.0f constant at `0x01e91e00` appears to be a per-tick work budget, not a lifetime retry limit. The actual disconnect-after-N-failures path may be in `ChannelInternal__processIncomingPacketEntry` (callee of the checkResendTimers loop). Track C should note this is unconfirmed and flag as "hypothesis pending verification."

3. **§2.5 msg_id ordering**: The 46 `ClientInterface` handler names are confirmed from RTTI, but the exact msg_id → handler mapping requires live-memory inspection of `DAT_01ef2518` after `PopulateMessageTypeTable` runs. The table is BSS-zero in the static image.

4. **`protocol_digest` hash algorithm**: ~~The CryptoPP encoder is a HexEncoder (uppercase hex output). But what is the *input hash*?~~ **CLOSED — see Investigation 6 below.**

---

## Investigation 6 — protocol_digest is SHA-1 (live x64dbg capture, 2026-05-15) — **SUPERSEDED**

> ⚠ **CORRECTION 2026-05-15 LATER SESSION**: The 40-char value captured here at `ServerConnection+0x130` is **NOT** the `protocol_digest`. It is a separate internal SHA-1 hash of the post-PMT dispatch table.
>
> The **actual `protocol_digest` sent in the SOAP body is MD5 (32 hex chars)**, computed via CME `Event_Net_GetProtocolDigest` event and passed as `param_4` to `logOnBegin`. The AteraLoader session log directly shows: `"protocol_digest: 58AFA196AD3AC4F65CADD99BFF23B799"` (32 chars).
>
> See `[[mercury-section-2-live-capture-findings]]` "Digest #1" and "Digest #2" sections for the corrected two-hash architecture.
>
> The captured data below is still valid as evidence that **a second 40-char SHA-1 hash exists at `ServerConnection+0x130` for internal table commitment**. Only its IDENTITY was wrong (it's not protocol_digest).

### Finding 6.1 — Captured digest value via silent breakpoint at PMT entry

**Method**: x64dbg log-breakpoint at `PopulateMessageTypeTable` (runtime `0x00E563D0`, static `0x00DD63D0`, ASLR offset +0x80000). BP halts briefly, emits log line with register placeholders, then auto-resumes via `commandText="go"`. Brief halt (sub-second) does not trip the server's 15s inactivity timeout.

**BP wiring** (all four `SetBreakpoint*` calls succeeded; the silent-only path with `fastResume=1` did NOT emit log text on this x64dbg-automate build, so the halt-and-resume pattern was required):

```
SetBreakpointFastResume 0xe563d0, 0
SetBreakpointCondition  0xe563d0, 1
SetBreakpointLog        0xe563d0, "PMT_HIT: this=0x{ecx} t130=0x{[ecx+0x130]} t134=0x{[ecx+0x134]} t190=0x{[ecx+0x190]} t194=0x{[ecx+0x194]}"
SetBreakpointLogCondition 0xe563d0, 1
SetBreakpointCommand    0xe563d0, "go"
SetBreakpointCommandCondition 0xe563d0, 1
```

**Captured log** (two PMT hits, identical):

```
PMT_HIT: this=0xEEE2C100 t130=0x0 t134=0x9869568 t190=0x1B98ED4 t194=0xFFFFFFFF
PMT_HIT: this=0xEEE2C100 t130=0x0 t134=0x9869568 t190=0x1B98ED4 t194=0xFFFFFFFF
```

**Memory at `0x09869568`** (heap-allocated digest data):

```
0x9869568  41 39 34 41 38 46 45 35 43 43 42 31 39 42 41 36   A94A8FE5CCB19BA6
0x9869578  31 43 34 43 30 38 37 33 44 33 39 31 45 39 38 37   1C4C0873D391E987
0x9869588  39 38 32 46 42 42 44 33 00 00 00 00 00 00 00 00   982FBBD3........
```

**Captured digest value**: `A94A8FE5CCB19BA61C4C0873D391E987982FBBD3` (40 chars uppercase hex).

### Finding 6.2 — basic_string layout at `ServerConnection+0x130`

**Memory at `0xEEE2C230`** (ServerConnection + 0x130, 32 bytes — full basic_string):

```
0xEEE2C230  00 00 00 00 68 95 86 09 00 00 00 00 00 00 00 00
0xEEE2C240  00 00 80 3F 28 00 00 00 2F 00 00 00 00 00 00 00
```

Confirms MSVC VS2005 `std::basic_string<char>` layout:

| ServerConnection offset | basic_string offset | Value | Field |
|---|---|---|---|
| `+0x130` | `+0x00` | `0x00000000` | `_Myproxy` (debug iterator, null in release) |
| `+0x134` | `+0x04` | `0x09869568` | `_Bx._Ptr` → heap-allocated 40-char digest |
| `+0x138..+0x143` | `+0x08..+0x13` | (unused) | rest of `_Bx` union (inline-buf slot, ignored in heap form) |
| `+0x144` | `+0x14` | `0x00000028` = **40** | `_Mysize` (matches SHA-1 hex length) |
| `+0x148` | `+0x18` | `0x0000002F` = **47** | `_Myres` (capacity, 16-aligned) |

### Finding 6.3 — Algorithm identification

**Length math**: 40 hex characters = 160 bits = **SHA-1** digest length.

Other candidates ruled out:
- MD5 → 32 hex chars (rejected)
- SHA-256 → 64 hex chars (rejected)
- CRC32 → 8 hex chars (rejected)

**Pipeline confirmed** (CORRECTED 2026-05-15 — the original "into the sgwLogin SOAP body" claim was wrong; the SHA-1 hex value is INTERNAL and never sent on the wire):

```text
getProtocolVersionHash()          → 20 raw bytes (SHA-1 of entity-def table)
  → CryptoPP::HexEncoder(Uppercase=true)
  → std::basic_string<char>       → "A94A8FE5...982FBBD3"
  → operator= into ServerConnection+0x130    (INTERNAL dispatch-table commitment)
  → (NOT used in SOAP — that field is populated separately from CME
     Event_Net_GetProtocolDigest via param_4 to logOnBegin; that path
     produces an MD5 32-char hex distinct from this SHA-1 40-char hex)
```

See Finding 5.2 below for the wire MD5 provenance.

**Closes**: Open Question #4 (hash algorithm).
**Suggested-footnote-slug**: `fn-i6-sha1-uppercase-hex`

### Finding 6.4 — Operational notes for future live captures

1. **ASLR offset**: SGW.exe loaded at `0x00480000` runtime vs Ghidra static base `0x00400000`. All static addresses need `+0x80000` to convert to runtime. Documented in BP setup commentary above.

2. **Silent BP gotcha**: Setting `SetBreakpointFastResume=1` with `condition=0` causes the BP to fire (hit counter increments) but suppresses `logText`/`commandText` emission entirely on this x64dbg-automate build. The reliable pattern is `condition=1` + `commandText="go"` for brief-halt-then-resume.

3. **Server inactivity timeout**: 15 seconds. Brief BP halts (sub-second to issue `go`) are safe. Manual pauses lasting >5 seconds risk disconnect.

4. **ServerConnection lifetime**: The object at `0xEEE2C100` is freed when the client disconnects. After-the-fact memory reads at the same address return reused heap data (a small `float 1.0` + various ints) — capture must happen during an active session.

5. **`+0x190` is the InterfaceElement dispatch table sub-object**, not part of the digest. PMT writes a back-pointer at `*(this+0x190+0x38) = *(this+0x1C8) = this`. The 127-iteration loop registers msg_id handlers `0x80..0xFE`.
