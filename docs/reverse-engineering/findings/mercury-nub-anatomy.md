# Mercury Nub Anatomy — Client Binary Analysis

> **Last updated**: 2026-05-23
> **Source**: SGW.exe Ghidra decompilation (Nub family, 22 functions analysed; image base `0x00400000`, 173,501 functions)
> **Confidence**: HIGH — every address verified against `SGW.exe`; every layout reached by Ghidra decompilation of the cited function, not pattern-matched from strings

---

## Overview

The C++ `Mercury::Nub` is a 0x180-byte God object that mediates BigWorld's explicit **game-thread vs network-thread** split: register/deregister and high-level channel API run on the game thread, while bind/recv/send/retransmit/timer dispatch run on a dedicated network thread, with two TBB lock-free queues bridging the two. Per-peer state is sharded across **two parallel maps** (`Channel` for game-thread API, `ChannelInternal` for network-thread internals) plus a per-interface `Connection` write-queue layer.

Cimmeria's Rust server has no game-thread / network-thread split — tokio's single-runtime, await-driven model eliminates the entire scaffolding the Nub was built to support. The Nub's responsibilities are correctly distributed across `service.rs` (bind), `connect_loop` (recv), `tick_sync.rs` (per-session tick), `ConnectedClientState` (peer state), and `Channel` (per-peer reliable stream). The orphan `crates/mercury/src/nub.rs` was deleted in [PR #358](https://github.com/SandboxServers/Cimmeria/pull/358); `TickActions` and the tick-driver ordering contract relocated into `crates/mercury/src/channel/mod.rs` next to `check_timeouts` / `keepalive_due` / `is_timed_out`.

Cross-reference: [`mercury-protocol-internals.md`](mercury-protocol-internals.md) covers the call chains, Bundle anatomy, packet flags, and 48 renamed functions; this doc is the missing class-anatomy companion.

Two latent **wire-format** gaps surface from this analysis. Neither blocks current traffic but both should be tracked if we ever need server-initiated RPC or want bit-perfect ACK cadence:

1. **REPLY_EXPECTED piggyback with XOR-inverted length** (Section 12 D). Server-side gap.
2. **ACK batching once per 10 ms network tick** vs. our inline-on-handle (Section 8 closing note). Latency-profile difference, client-accepted either way.

A separate non-gap recalibration: our `MAX_RETRIES = 20` constant is **not binary-evidenced** — the C++ side has no per-packet retry counter and gives up via rdtsc-driven inactivity instead (Section 11). The Cimmeria constant is a defensible invention but doesn't have a direct binary correspondent.

---

## 1. Function inventory

All symbols verified renamed in Ghidra.

| Symbol | Address |
|---|---|
| `Mercury_Nub_Nub` (Nub constructor) | `0x015841d0` |
| `Mercury_BaseNub` constructor (called from Nub ctor) | `0x01577960` |
| `Mercury_Nub_RegisterChannel` | `0x0157e920` |
| `Mercury_Nub_RegisterChannelInternal` | `0x0157e480` |
| `Mercury_Nub_DeregisterChannel` | `0x0157eb00` |
| `Mercury_Nub_AddChannelToConnection` | `0x0157d9a0` |
| `Mercury_Nub_RemoveChannelFromConnection` | `0x0157db80` |
| `Mercury_Nub_addListeningSocket` | `0x01583440` |
| `Mercury_BaseNub_RecreateListeningSocket` | `0x01577b80` |
| `Mercury_Nub_Send` | `0x01582160` |
| `Mercury_Nub_ProcessFilteredPacket` | `0x01580840` |
| `Mercury_Nub_ProcessPacket` | `0x0157fd20` |
| `Mercury_Nub_ProcessOrderedPacket` | `0x0157c820` |
| `Mercury_Nub_ProcessPendingEvents` | `0x01581ab0` |
| `Mercury_Nub_writeConnection` | `0x01583a90` |
| `Mercury_Nub_CloseConnectionInternal` | `0x01583820` |
| `Mercury_Nub_handleMessage` | `0x0157bd30` |
| `Nub__networkThreadLoop` | `0x01583bf0` |
| `Nub__Connection__ctor` | `0x01583070` |
| `ChannelInternal__ctor` | `0x0158c7b0` |
| `ChannelInternal__checkAndSendNubException` | `0x0158bed0` |
| `UnAckedHandler__checkResendTimers` (`FUN_0158c420`) | `0x0158c420` |

**String anchor block:** 77 `Nub::*` log strings at `0x01b16ee8`–`0x01b18f78`.

---

## 2. Class layout — `Mercury::BaseNub` (base class, 0x70 bytes)

Established by decompiling `FUN_01577960`. The first 0x70 bytes of any `Nub` is the `BaseNub` portion.

| Offset | Field | Size | Notes |
|---|---|---|---|
| `+0x00` | vtable ptr | 4 | `Mercury::BaseNub::vftable` |
| `+0x04` | socket | 4 | Initialised to `INVALID_SOCKET` (`0xffffffff`) |
| `+0x08` | `MachineGuard::InterfaceTable` | 0x10 | Config table, capacity 0x100 |
| `+0x18` | `TimerEntry` (recurring tick timer) | 4 | Cleared by `TimerEntry__clear` |
| `+0x2c` | `min_port` | 2 | Port range lo |
| `+0x2e` | `max_port` | 2 | Port range hi |
| `+0x30`–`+0x36` | secondary port ranges | 6 | Pairs for retry ranges |
| `+0x38` | reserved | 4 | Zeroed |
| `+0x40` | interface name string (SSO) | 0x10 | `std::string` with SSO buffer |
| `+0x50` | `pXferHandler` | 4 | Packet-filter / event-loop re-register hook |
| `+0x54` | SSO capacity field | 4 | `0x0F` = inline |
| `+0x58` | socket-info vector begin | 4 | |
| `+0x5c` | socket-info blob (assigned addr+port) | varies | `getsockname` result written here |
| `+0x60` | socket-info end ptr | 4 | |
| `+0x64` | socket-info capacity | 4 | |
| `+0x6c` | advertised port | 2 | |

---

## 3. Class layout — `Mercury::Nub` (derived, total ~0x180 bytes)

Established by decompiling `FUN_015841d0`. First 0x70 is the `BaseNub` (above); fields below start at `+0x70`.

| Offset | Field | Size | Notes |
|---|---|---|---|
| `+0x00`–`+0x6f` | BaseNub fields (see Section 2) | 0x70 | |
| `+0x70` | `InputMessageHandler` vtable ptr | 4 | Overwritten with `Mercury::Nub::vftable` in ctor |
| `+0x74` | `TimerExpiryHandler` vtable ptr | 4 | Overwritten with `Mercury::Nub::vftable` in ctor |
| `+0x78` | interface element vector (listening socket table) | 4 | `addListeningSocket` appends here |
| `+0x7c`–`+0x84` | connection list begin/end/cap | 12 | `std::vector<Connection*>` of active connections |
| `+0x88`–`+0x8c` | **connection map** (by composite key) | 8 | `std::unordered_map<CompositeKey, Connection*>` — key built by `FUN_0157bc70` as IP + sessionID, **not** just IP |
| `+0x90`–`+0x93` | connection map end sentinel | 4 | |
| `+0x94` | reply-ID counter | 4 | Seeded from `rdtsc mod 100000 + 0x2775` |
| `+0x98` | reply-ID increment step | 4 | = 1 |
| `+0x9c`–`+0xa4` | reply-handler pending map (by reply ID) | 8 | `std::map<uint32, ReplyHandler>` — request/reply matching |
| `+0xa8` | reply map end ptr | 4 | |
| `+0xac` | **channel packet-queue** (256-slot) | varies | `PacketQueue` with randomizer |
| `+0xbc` | thread sleep time (ms) | 4 | From `Nub::Nub` `param_6`. The log `Nub::Nub() using thread sleep time of 10` reads this field. |
| `+0xc0` | flags byte 0 | 1 | Zeroed |
| `+0xc1` | flags byte 1 | 1 | Zeroed |
| `+0xc2` | abort flag | 1 | Zeroed; `!= 0` signals network thread should exit |
| `+0xc4` | observer pointer | 4 | `NubEventObserver*`; zeroed in ctor |
| `+0xc8`–`+0xcf` | **external channel map** (by NetworkAddress) | 8 | `std::unordered_map<NetworkAddress, Channel*>` — game-thread-facing |
| `+0xd4`–`+0xdb` | **ChannelInternal map** (by addr) | 8 | `std::unordered_map<NetworkAddress, ChannelInternal*>` — network-thread-facing |
| `+0xe0` | max event count per recv loop | 4 | Bounds `processPendingEvents` loop |
| `+0xe4` | stat: bytes sent (wire) | 4 | Written by `writeConnection` |
| `+0xe8` | stat: bytes received | 4 | Written by `processPendingEvents` |
| `+0xec` | stat: packets sent | 4 | |
| `+0xf0` | stat: packets received | 4 | |
| `+0xf4`–`+0xf8` | stat: requests sent | 8 | |
| `+0xfc` | stat: bundles sent | 4 | |
| `+0x100`–`+0x11c` | stats: aborted, duplicate, errors, etc. | varies | |
| `+0x120` | self-pointer cell (for cross-thread ref) | 4 | `scalable_malloc(4)`; `*ptr = this` |
| `+0x124` | NetworkThread pointer | 4 | 0x34-byte object |
| `+0x128` | recurring-timer handle | 4 | Set by `TimerHeap__add(period=0x1e8480 µs=2s)` |
| `+0x12c` | channel list (select/poll fd array) | varies | Initialised by `Nub__ChannelList__init` |
| `+0x138` | **incoming event queue** (TBB `concurrent_queue`) | 0x18 | `NubException`s pushed from network thread, consumed by game thread |
| `+0x150` | **outgoing command queue** (TBB `concurrent_queue`) | 0x18 | `registerChannel`/`deregisterChannel` messages from game thread, consumed by network thread |
| `+0x168` | `TimerMap` / `TimerEntry` | varies | Internal timers |

---

## 4. Class layout — `Mercury::Nub::Connection` (0x24 bytes)

Established by decompiling `FUN_01583070`. A `Connection` corresponds to **one bound UDP socket on one network interface** — not a logical session.

| Offset | Field | Size | Notes |
|---|---|---|---|
| `+0x00` | vtable ptr | 4 | `Mercury::Nub::Connection::vftable` |
| `+0x04` | refcount | 4 | = 0 initially |
| `+0x08` | socket handle | 4 | = `INVALID_SOCKET` (`0xFFFFFFFF`) until `addListeningSocket` populates it |
| `+0x0c` | IP address | 4 | Network byte order, set by `addListeningSocket` |
| `+0x10` | port | 2 | |
| `+0x12` | padding | 2 | |
| `+0x14` | channel list head ptr | 4 | Linked list of `Channel` objects multiplexed over this socket |
| `+0x18` | packet queue head | 4 | Drained by `writeConnection` |
| `+0x1c` | pending-send count | 4 | Drained by `writeConnection` |
| `+0x20` | channel active-count | 4 | `addChannelToConnection` increments; `removeChannelFromConnection` decrements. **This is the value the log line `addChannelToConnection: ... N channels are using it` reads.** |

---

## 5. Class layout — `Mercury::ChannelInternal` (0x180 bytes)

Established by decompiling `ChannelInternal__ctor` at `0x0158c7b0`. The `ChannelInternal` is the **network-thread-owned shadow** of a `Channel` — separate object to allow thread-safe split ownership.

Key fields:

| Offset | Field | Notes |
|---|---|---|
| `+0x2c` | channel address (`NetworkAddress`) | |
| `+0x34` | unacked-packet doubly-linked list head | Per-channel retransmit buffer. `FUN_0158c2a0` appends here when `Nub::send` handles a `FLAG_RELIABLE` packet. |
| `+0x40` | packet hash table | Slot store sized from runtime config at `[ChannelInternal+0x2C]`; capacity defaults to **32** in the unmodified binary (matches our `TX_WINDOW_SIZE` pin). |
| `+0x80` | address string (`std::string`) | |
| `+0x9c` | outbound bundle (ACK accumulator) | The "send pending ACKs as a bundle" path |
| `+0x110` | has-pending-ack flag | Set by receiver-side code; drained by network thread loop step 4 |
| `+0x114`, `+0x118` | network address fields | Used by `checkAndSendNubException` |
| `+0x160` lo / `+0x164` hi | rdtsc inactivity-timeout threshold | Set at construction via `FUN_012379f6()` (frequency-scaled time query). The `NetInactivityTimeout` INI key at `0x019abb7c` is one input but is mediated by the entity-config layer before reaching the Nub. |
| `+0x168` lo / `+0x16c` hi | rdtsc keepalive-send threshold | |
| `+0x170` lo / `+0x174` hi | rdtsc last-receive timestamp | Compared against `+0x160`/`+0x164` to fire `REASON_INACTIVITY` |

**Critical implementation detail:** all timing is rdtsc-based (not millisecond-based) in the C++ code. The conversion happens at `ChannelInternal` construction. Cimmeria's Rust port uses `std::time::Instant` — the correct 2026 equivalent.

---

## 6. Two-channel-map design (key architectural finding)

The Nub holds **two parallel channel maps:**

| | `Mercury::Channel` | `Mercury::ChannelInternal` |
|---|---|---|
| Owner thread | Game thread / application | Network thread |
| Lives in map at | `Nub+0xC8` (external) | `Nub+0xD4` (internal) |
| Keyed by | `NetworkAddress` (ip+port) | `NetworkAddress` (via `ChannelInternal__getUnAckedHandlerOffset`) |
| Holds | Application-level send/recv API | Retransmit state, seq numbers, rdtsc timers, ACK bundle, inactivity thresholds |
| Size | Unknown (external object) | 0x180 bytes |
| Cross-thread signal | Pushes to outgoing queue `Nub+0x150` | None — stays on network thread |

**This is why the log shows both `Nub::registerChannel: registering channel ...` (game thread) and `Nub::_processMessage: registering ChannelInternal from address ...` (network thread) for the same connect — they are two phases of the same lifecycle, separated by a TBB queue hop.**

---

## 7. registerChannel / deregisterChannel lifecycle

### `Nub::registerChannel` (`0x0157e920`) — called on game thread

1. Asserts via `FUN_0157c740` that no existing channel is registered at the address (idempotent re-register is safe — same-pointer is OK).
2. Cancels any pending timer for that address slot via `FUN_015847f0`.
3. Inserts `Channel*` into external channel map at `Nub+0xC8` (keyed by `NetworkAddress`).
4. Allocates `ChannelInternal` (0x180 bytes) via `ChannelInternal__ctor`.
5. Wraps the `ChannelInternal` in a `ClientChannelRegMessage` (0x14 bytes) and pushes to outgoing command queue at `Nub+0x150` (TBB `concurrent_queue` — lock-free cross-thread).

### `Nub::RegisterChannelInternal` (`0x0157e480`) — called on network thread

Drains the queue from step 5 above. Decompilation shows two cases keyed on `param_1+8`:

- **Non-zero** → register path: gets the `ChannelInternal`'s address via `ChannelInternal__getUnAckedHandlerOffset`, cancels any prior timer at that address, upserts into internal channel map at `Nub+0xD4`. Logs `"Nub::_processMessage: registering ChannelInternal from address %s"`.
- **Zero** → unregister path: calls `Mercury_Nub_RemoveChannelFromConnection` to detach from the connection, then erases from `Nub+0xD4`. Logs `"Nub::_processMessage: deregistering ChannelInternal from address %s"`.

### `Nub::deregisterChannel` (`0x0157eb00`)

Mirrors register. Removes from `Nub+0xC8`, clears the `ChannelInternal` back-pointer stored in the Channel at offset `+0x0c`, pushes a deregister-variant `ClientChannelRegMessage` to `Nub+0x150`. The zero-vs-non-zero check is how `RegisterChannelInternal` distinguishes register from deregister.

---

## 8. Network thread loop — `Nub__networkThreadLoop` (`0x01583bf0`)

Decompiled iteration body:

```
loop {
  // 1. Drain outgoing command queue (Nub+0x150) — register/deregister msgs
  //    from game thread. For each msg: (*vtable+4)(this) — dispatches to
  //    RegisterChannelInternal.

  // 2. TimerHeap__processExpired(this+0x168) — fire expired per-channel +
  //    Nub-level timers (keepalive-send, inactivity check).

  // 3. FUN_0158bf90(this) — internal maintenance (likely ChannelInternal
  //    latency recording).

  // 4. Iterate ChannelInternal map (this+0xD4 to this+0xD8):
  //    for each ChannelInternal:
  //      FUN_0158b980(channelInternal)
  //        — if has-pending-ack flag (+0x110) is set: call Mercury_Nub_Send
  //          with the ACK bundle at ChannelInternal+0x9c, address from +0x114,
  //          then clear the flag.
  //      ChannelInternal__recordLatency(channelInternal).

  // 5. Iterate Connection vector (this+0x7c to this+0x80):
  //    for each Connection:
  //      Mercury_Nub_writeConnection(this, conn)
  //        — drains pending-send queue, calls sendto().
  //      Mercury_Nub_ProcessPendingEvents(this, conn)
  //        — recvfrom() loop (bounded by Nub+0xe0), decrypt/demux each to
  //          ProcessFilteredPacket.
  //    if both returned 0 (no work done): Sleep(Nub+0xbc ms).  // The 10ms
  //                                                            // idle from
  //                                                            // the log.

  // 6. Check abort flag at Nub+0xc1; if non-zero: break.
}
```

**ACK batching observation:** the receiver-side code sets the has-pending-ack flag at `ChannelInternal+0x110`; the network thread loop drains it once per iteration. So the C++ client receives piggybacked ACKs **at most once per 10 ms**, not immediately per inbound message. Cimmeria's Rust `tick_sync.rs` sends ACKs inline during message handling — wire-equivalent for the client (it accepts both cadences), but a measurable latency-profile difference. See Section 12 for the full delta and Cimmeria gap-tracking issue.

---

## 9. `Nub::send` (`0x01582160`) — four-phase pipeline

This is **not** a thin `sendto()` wrapper. It's a deeply-staged pipeline.

### Phase 1 — packet finalisation and footer serialisation

`Mercury_Bundle_Finalise(bundle)` stamps the bundle closed. Then for each `Packet` in the chain (a bundle may span multiple UDP datagrams via `ChannelInternal__getNextChannelInternal`):

- `FLAG_REPLY (0x02)` → serialise reply-handler entries into the packet tail. Each entry: 2-byte length field (written then **XOR-inverted for obfuscation** via `*puStack_88 = ~*puStack_88`), 1-byte type, handler data via `memcpy`, 4-byte reply ID. **This XOR inversion is wire-observable** — receivers detect the negative high bit and XOR back. Cimmeria's Rust server does not implement REPLY_EXPECTED at all; gap only matters if we ever do server-initiated RPC.
- `FLAG_INDEXED (0x04)` → ACK count byte then 4-byte ACK sequence numbers at packet tail.
- `FLAG_FINALIZED (0x40)` → 4-byte sequence number at tail. If `FLAG_RELIABLE = 0x10` is also set, calls `FUN_0158c2a0` to append the packet to the `ChannelInternal`'s unacked retransmit list at `ChannelInternal+0x34`.
- `FLAG_REPLY_EXPECTED (0x01)` → 2-byte expected-reply-ID at tail.
- `FLAG_PIGGYBACK (0x20)` → 4-byte sequence range at tail.
- `FLAG_HAS_CHANNEL (0x08)` → sets or clears the channel-address flag in the packet header depending on whether `param_3` (`PacketFilter`) is null.

### Phase 2 — actual transmission

- If `param_3 == null` (normal path): calls `Nub__addChannelToExistingConnection` (`0x015814c0`), which builds an iovec and appends to the Connection's pending-send queue at `Connection+0x18` with the count at `Connection+0x1c`. **The actual `sendto()` happens later, in `Mercury_Nub_writeConnection` during the network thread loop step 5.** This is a write-queue, not an immediate send.
- If `param_3 != null` (encryption path): dispatches synchronously through the `PacketFilter` vtable `(*param_3+4)()`. This is where the AES-256-CBC `PacketEncrypter` plugs in (see [`mercury-protocol-internals.md`](mercury-protocol-internals.md) for the encryption details).

### Phase 3 — statistics

Increments `Nub+0xfc` (bundles sent), `Nub+0x104` (bytes sent), `Nub+0x108` (requests sent).

### Phase 4 — reliable NAK handling

If any reliable-channel send detected a NAK (`bVar7 == true`), constructs `NubException(REASON_RELIABLE_SEND_FAILED = 0xFFFFFFFA)` and pushes to the incoming event queue at `Nub+0x138`.

**Behavioral note:** `Nub::send` is a **network-thread** function under normal operation. Game code calls `Channel::send()` → `Bundle::finalise()`; the bundle goes into `ChannelInternal+0x9c`; the network thread loop step 4 detects the has-pending-ack flag and calls `Nub::send` from there. The log line `"BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send"` at string `0x019d1a5c` is an exceptional bootstrapping path (login handler ctor, before the channel is fully registered).

Cimmeria's Rust `UdpTransport::send_to` calls the OS `send_to` directly — the correct equivalent because we don't have a separate network thread.

---

## 10. `addListeningSocket` / `recreateListeningSocket`

### `Mercury_Nub_addListeningSocket` (`0x01583440`)

The Nub supports **multiple bound sockets per process**. Maintains a vector of `Connection` objects (interface element table) at `Nub+0x78`. Each call allocates a new 0x24-byte `Connection`, appends it, creates a new `SOCK_DGRAM` socket via `socket(AF_INET, SOCK_DGRAM, 0)`, and calls `Mercury_BaseNub_RecreateListeningSocket` to bind it.

The Nub constructor iterates `param_5` (a vector of interface descriptors, each 7 DWORDs = 28 bytes) and calls `addListeningSocket` once per entry. A multi-homed host binds one UDP socket per interface. Typical client session log shows `172.26.240.1:60486` — one socket on one interface.

### `Mercury_BaseNub_RecreateListeningSocket` (`0x01577b80`)

Bind sequence:

1. If interface string is empty (`FUN_004242c0` returns 0): calls `Mercury_BaseNub_QueryMachineInterface` to ask `BWMachined` (the BigWorld machine daemon) for the local interface. If machined isn't running (the game client case, since players don't run machined), falls back to logging `"No address received from machined so binding to all interfaces"` and binds `0.0.0.0`.
2. If interface string is non-empty: resolves via `Endpoint__findIndicatedInterface`.
3. Calls `Endpoint__bindInRange` with port range `[BaseNub+0x2C, BaseNub+0x2E]`. Port 0 → OS assigns.
4. Calls `getsockname` to read the assigned addr+port. If IP is `0.0.0.0`, resolves hostname to get the advertised address (this is what the log prints as `Advertised address ...`).
5. Sets non-blocking via `ioctlsocket(FIONBIO)`.
6. If `BaseNub+0x50` (`pXferHandler`) is non-null, re-registers with the event loop.

**Trigger:** "recreate" means in-place fd swap when the bind address changes, not on interface configuration change per se.

**Server-side relevance:** none. Cimmeria's server doesn't need per-interface multi-socket binding. `UdpSocket::bind` in `service.rs` is the correct equivalent for the server side.

---

## 11. Inactivity and retransmit

The inactivity timeout lives in **`ChannelInternal`**, not directly in `Nub`.

### `ChannelInternal__checkAndSendNubException` (`0x0158bed0`)

Uses rdtsc for all timing. Two thresholds:

- **Inactivity (receive side):** compares current rdtsc against `ChannelInternal+0x174`/`+0x170` (last receive timestamp) and `+0x164`/`+0x160` (inactivity threshold). If `(now - last_recv) >= inactivity_threshold` → pushes `NubException(REASON_INACTIVITY = 0xFFFFFFF9)` to incoming queue `Nub+0x138`.
- **Keepalive-send (write side):** compares against `ChannelInternal+0x16c`/`+0x168` (keepalive threshold) and `+0x5c`/`+0x58` (last send timestamp). If overdue → calls `UnAckedHandler__sendAckBundle2` (sends an empty ACK bundle as keepalive).

This function is called from network thread loop step 4 (inside the `ChannelInternal` iteration), so the inactivity check fires **every 10 ms tick**.

### `UnAckedHandler__checkResendTimers` (`FUN_0158c420`, `0x0158c420`)

The retransmit scanner. Uses rdtsc against each packet's `timer+0x4` (lo) / `timer+0x8` (hi) — the per-packet last-send-timestamp. The retry budget is the global float at `_DAT_01e91e00` — confirmed via decompile: `if (_DAT_01e91e00 < fVar2) return 0`, where `fVar2` increments by `1.0` per entry processed. The budget caps how many retransmits can fire in a single scan (Cimmeria's equivalent is `RETRANSMIT_BUDGET_PER_TICK = 5`).

The scanner iterates the unacked list. For each entry whose rdtsc delta exceeds its timer, it calls `ChannelInternal__processIncomingPacketEntry` to retransmit. On failure, returns an error code that propagates as a `NubException` with the log string `"UnAckedHandler::checkResendTimers(%s): Aborting due to failed resend for #%d"` at `0x01b19dd8`.

### Recalibration: our `MAX_RETRIES = 20` is not binary-evidenced

The C++ side has **no explicit per-packet retry counter**. Retransmits continue as long as the rdtsc threshold is exceeded and the budget allows. The channel gives up when the `ChannelInternal` inactivity threshold fires (producing `REASON_INACTIVITY`), not on a retry count. Cimmeria's `MAX_RETRIES = 20` in `crates/mercury/src/lib.rs` is a defensible invention — it provides an additional safety net beyond inactivity — but doesn't have a direct binary correspondent. The accurate C++ semantic is "die when inactivity threshold exceeded." Either model is correct in practice; flagging here so a future reviewer doesn't mistake the constant for a binary-anchored value.

### `NetInactivityTimeout` INI key (`0x019abb7c`)

Referenced from `ConstructEntityRpcRegistry`. It's an INI-configurable value flowing through the entity-configuration layer, not directly into the Nub's rdtsc thresholds. The `ChannelInternal` thresholds at `+0x160`/`+0x164` and `+0x168`/`+0x16c` are set in the constructor at `0x0158c7b0` from rdtsc-frequency-scaled values produced by `FUN_012379f6()`. The mapping from INI ms-value to rdtsc ticks happens at `ChannelInternal` construction.

---

## 12. Surprising findings

### A. The Nub has TWO concurrent queues, not one

- `Nub+0x138` = **incoming event queue** — `NubException`s from network thread → game thread.
- `Nub+0x150` = **outgoing command queue** — register/deregister msgs from game thread → network thread.

Both are TBB `concurrent_queue<CME::RefCountedObj<Mercury::ClientMessage>>`. Cimmeria's Rust server has no equivalent because the architecture is synchronous on the tokio event loop. **This is the foundational reason the C++ Nub looks like a God object** — it's mediating the game-thread vs network-thread split that tokio eliminates.

### B. `Nub::send` is normally a network-thread function

Game code calls `Channel::send` which writes into `ChannelInternal+0x9c`. The network thread loop step 4 detects pending data and invokes `Nub::send` from there. The `BaseAppLoginHandler::BaseAppLoginHandler: calling Nub::send` log line is a special bootstrap case, not the steady-state path.

### C. PacketFilter is the encryption seam in the C++ code

`Nub::send`'s `param_3` is a `PacketFilter*` — if non-null, the encrypted path dispatches through `(*param_3+4)()` instead of the connection write-queue path. This is where AES-256-CBC `PacketEncrypter` plugs in. Cimmeria's Rust port inlines encryption into `UdpTransport::send_to` — functionally equivalent, structurally different.

### D. XOR-inverted reply-length is wire-observable obfuscation

The 2-byte length field for reply handlers is written then `~`-inverted (`*puStack_88 = ~*puStack_88`). Receivers detect the negative high bit and XOR back. **This is a real wire-format detail** that must be preserved if we ever implement REPLY_EXPECTED channels server-side.

### E. Reliable retransmit buffer is per-`ChannelInternal`, not per-Nub

The `FUN_0158c2a0` call from `Nub::send` (when `FLAG_RELIABLE` is set) appends to `ChannelInternal+0x34` (doubly-linked list). The scanner iterates this list per `ChannelInternal`. Cimmeria's Rust `Channel::tx_window` is the correct structural equivalent — same per-channel granularity.

### F. The Connection's write-queue buffers sends between game and network ticks

`Nub::send` → `Nub__addChannelToExistingConnection` adds packets to `Connection+0x18`/`+0x1c`. `Mercury_Nub_writeConnection` drains it on the network thread with the actual `sendto()`. The C++ client may batch multiple sends from one game tick into a single network tick drain — wire latency from `Channel::send` is up to 10 ms. Cimmeria's Rust server sends immediately (lower latency, also correct — the client doesn't require a delay).

### G. Reply-handler map for request/reply RPC matching

`Nub::handleMessage` at `0x0157bd30` looks up reply handlers in `std::map` at `Nub+0x9c` keyed by reply ID. Reply IDs come from the RDTSC-seeded counter at `Nub+0x94`. Cimmeria's Rust server has no equivalent because the server never sends RPCs expecting callbacks — gap only matters for hypothetical future server-initiated RPC (see Section 12 D + 8 closing note).

---

## 13. Implementation impact

Cimmeria's Rust port distributes the Nub's responsibilities across:

| C++ Nub responsibility | Cimmeria Rust home |
|---|---|
| UDP socket bind | [`crates/services/src/base/service.rs`](../../../crates/services/src/base/service.rs) |
| `recv_from` loop | [`crates/services/src/base/connect_loop/mod.rs`](../../../crates/services/src/base/connect_loop/mod.rs) |
| Per-session tick (retransmit + keepalive) | [`crates/services/src/base/tick_sync.rs`](../../../crates/services/src/base/tick_sync.rs) |
| Per-peer state | `ConnectedClientState` in [`crates/services/src/base/`](../../../crates/services/src/base/) |
| Per-peer reliable stream | [`Channel`](../../../crates/mercury/src/channel/mod.rs) (per-peer, equivalent to `Mercury::Channel` + `ChannelInternal` merged) |
| Encrypted outbound | [`UdpTransport::send_to`](../../../crates/mercury/src/transport.rs) (encrypt inlined; PacketFilter seam not needed) |
| Tick driver contract (`TickActions`, ordering invariant) | [`channel/mod.rs`](../../../crates/mercury/src/channel/mod.rs) — preserved as documentation for any future registry-style driver |

The orphan `crates/mercury/src/nub.rs` was deleted in [PR #358](https://github.com/SandboxServers/Cimmeria/pull/358).

### Two latent wire-format gaps to track

1. **REPLY_EXPECTED piggyback with XOR-inverted length** (Section 9 / Section 12 D). Server-side gap; only relevant if we ever need server-initiated RPC.
2. **ACK batching once per 10 ms network tick** (Section 8 / Section 12 F). Cimmeria sends ACKs inline; the C++ client batches them per tick. Latency-profile difference, client-accepted either way.

Both are tracked in a follow-up issue filed alongside PR #358.
