# Client limits and bounds

> Type: ledger. Audience: the coordinator, RE and measurement workers, and
> reviewers. Opened 2026-10-10 against `main` @ `6996c9403`. Prefix `CL-`.
> Effort 5, Phase A of the [lab roadmap](../lab-roadmap/handoff.md). Phase B
> is [lab-chaos](../lab-chaos/README.md), which reads this campaign's numbers.
> Related: [#1341](https://github.com/SandboxServers/Cimmeria/issues/1341)
> (first-login flush tail drop). Packet specs: [work-packets.md](work-packets.md).
>
> **Campaign status (2026-10-10): planned, nothing built.** CL-01, CL-02,
> CL-03, CL-04, CL-05, CL-06 and CL-11 can start now; none of them uses the
> lab. Every live packet (CL-08, CL-09, CL-10) needs the user's OK before it
> starts. Four decisions (D-CL4, D-CL5, D-CL7, D-CL8) gate the raise
> candidates.

## Purpose

The owner's rule: understand the client's limits before deciding how much
chaos to cause, and work out which limits can be raised. This campaign
measures and documents every bound the server, the lab and chaos mode run
into, each with its source, and turns the raisable ones into candidate
packets with a risk note and a rollback.

It is mostly docs. **The acceptance is the committed bounds table** under
`docs/client/limits/` (D-CL1), every row with a value or an explicit "not
measured", and a source for every value.

Out of scope: fixing #1341 (its mitigation is decided through
[D-WC8](../wireclient-ci/README.md#decisions) and the issue), shipping any
client patch to players, and the chaos tooling itself (lab-chaos).

## What is already known

Against `main` @ `6996c9403`. Sources are repo paths; "RE" means
[client-mercury-receive-path.md](../../reverse-engineering/findings/client-mercury-receive-path.md)
unless another finding is named.

| # | Finding | Packets |
|---|---|---|
| F1 | The client's Mercury socket reader `FUN_0158a200` calls `recvfrom` with a 1472-byte (`0x5c0`) buffer. The server's `PACKET_MAX_SIZE` is 1472 (`crates/mercury/src/lib.rs`). A larger datagram should fail with `WSAEMSGSIZE`; no live client has reported it yet. | CL-05, CL-08 |
| F2 | Two server causes of oversized reliable datagrams are fixed: uncapped piggybacked ACKs (2026-10-03) and data-sized single sends such as an NPC `createOnClient` cascade (2026-10-05, now fragmented). Rule: [mercury-wire-format.md § Reliable datagram size budget](../../protocol/mercury-wire-format.md#reliable-datagram-size-budget). The live check (`oversize_fragmented` logged, no `tx_hole_stall`) is still open. | CL-05 |
| F3 | The client's reorder window is 512 slots (`Channel+0x2c = 0x200`, read at `0x0158C801`). The server's `TX_WINDOW_SIZE` is 32 with no client reason (#353). | CL-14 |
| F4 | The client accepts **one open fragment group per channel** (`Channel+0x124`), matched by `lastFrag` alone, and applies **no fragment-count cap** in `Nub::processPacket` (`0x0157fd20`). The server caps at `MAX_FRAGMENTS = 64`. The largest bundle seen is 15 fragments, 18,367 bytes. A stale group is discarded after 60 s of TSC. | CL-02 |
| F5 | The client ACKs every in-window packet before deciding anything (`queueAckForPacket`, `0x0158cba0`), including fragments it later drops. "All ACKed" proves nothing about delivery. | CL-02 |
| F6 | **The #1341 residue.** `Bundle::iterator`'s copy constructor (`0x01578e90`) never writes `+0x14`, the next-request offset, so the message loop starts with a stack residue. It is sticky per client process, always a multiple of 8 (11 readings, 128 to 848), and only live in the first packet of a chain. When it equals a message cursor the rest of the bundle is lost. Whether a client without our telemetry DLL has a residue is unknown; the decisive probe (a non-freezing breakpoint at `0x0157c9dc` logging `[esp+0x64]`) has never run. | CL-10 |
| F7 | The first-login phase-1 flush is one single-packet bundle of 37-byte records (`CREATE_ENTITY` 11 + `UPDATE_AVATAR` 26): 24 NPCs make the 889-byte packet #1341 hits. The phase-2 cascade is a separate fragmented bundle (`crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs`). | CL-09, lab-chaos |
| F8 | An idle in-world client sends about 6 packets/s and `perfStats` every 15.0 s. `NetInactivityTimeout=15` is the client's tolerance of **server** silence (`UE3_INACTIVITY_TIMEOUT_MS`); the server's peer-dead timer is 300 s ([idle send cadence note](../../../.claude/agent-memory/main-session/reference_client_idle_send_cadence.md)). | CL-01 |
| F9 | Server resend constants: `ACK_TIMEOUT_MS` 700, `MAX_RETRIES` 20, `RETRANSMIT_BUDGET_PER_TICK` 5, `MAX_UNSENT_PACKETS` 1024, `KEEPALIVE_INTERVAL_MS` 1000. The client's own resend timer for its reliable sends is not documented. | CL-02 |
| F10 | AoI: `PLAYER_AOI_RADIUS` 150 m, `AOI_LEAVE_MARGIN` 25 m, NPC default 100 m (`crates/entity/src/cell_entity/`). The deferred buffer drops past `MAX_DEFERRED_AOI_MSGS` = 512 with a WARN. The first-login hold is 16 s (`HOLD_DURATION`); the appearance resend runs every 100 ms for 20 s (`cinematic.rs`). No client-side entity-count limit is documented. | CL-03, CL-09 |
| F11 | A lab session with 161 extra NPCs (Debug Area lineup) ran after the cascade fragmenting fix; nobody has looked for the client's ceiling. | CL-09 |
| F12 | The DLL already samples memory every 30 s: `client.engine.memory` carries `working_set_mb`, `peak_working_set_mb`, `private_mb`, `avail_virtual_mb`, `total_virtual_mb` and warns under 256 MB of free address space; `client.engine.hitch` fires on a tick gap of 200 ms or more ([client-telemetry.md § Subsystem seams](../../architecture/client-telemetry.md#subsystem-seams)). `total_virtual_mb` answers "is the client Large Address Aware" from SigNoz alone: about 2048 without the flag, about 4096 with it on 64-bit Windows. Nothing reports ordinary frame times. | CL-05, CL-06 |
| F13 | A client not in the foreground runs at below-normal priority with a 5 ms sleep per `FEngineLoop::Tick`; the lab's virtual focus defeats it ([live-research-lab.md](../../guides/live-research-lab.md), "Focus"). The call site and any ini switch for it are not documented. | CL-04 |
| F14 | The launcher already edits one PE header byte of `SGW.exe` (clears `DYNAMIC_BASE`, offset `0x186`; `crates/launcher/src/client_setup/aslr.rs`). Setting Large Address Aware is the same kind of edit, in the file header's `Characteristics` (`0x0020`), and is still a client patch. | CL-04, CL-12 |
| F15 | No repo doc records `TextureStreaming` `PoolSize`, `MaxSmoothedFrameRate` or `bSmoothFrameRate`. The client reads `Config/SGWEngine.ini` from the user folder (`crates/lab/src/supervisor/instance_profile.rs` seeds it per lab instance); `DefaultEngine.ini` is based on `Engine/Config/GameplayEngine.ini`. | CL-04, CL-13 |
| F16 | Server tick 10 Hz (`UPDATE_FREQUENCY_HZ`, `crates/wire/src/mercury/game_clock/mod.rs`), sent to the client at login. World entry `playCharacter` to `onClientReady` took about 2.4 s for Castle_CellBlock on one player machine. Client interpolation constants and teleport-to-AoI latency are not documented. | CL-03, CL-11 |
| F17 | Lab capacity: `CEILING_MAX_CLIENTS` = 5, the five seeded lab accounts (`crates/lab/src/supervisor/instance.rs`); window 1280x720 windowed (`process.rs` `DEFAULT_WINDOW`); a client's bridge took about 25 s to come up with four others running ([parallel clients note](../../../.claude/agent-memory/main-session/reference_lab_parallel_clients_2026_10_10.md)). An idle lab client whose display the screensaver takes loses its D3D device after about 10 minutes (`D3DERR_NOTAVAILABLE`). | CL-01, CL-15 |
| F18 | `crates/mercury/src/lossy_transport.rs` and the `chaos-testing` feature on `cimmeria-base` already give a seeded loss, latency, jitter, duplicate and reorder wrapper around the real transport ([network-chaos-testing.md](../../architecture/network-chaos-testing.md)). Lab-chaos's `net` profile builds on it. | lab-chaos |

## Bounds table skeleton

CL-01 moves this table into `docs/client/limits/`, one file per area; the
packet in the last column fills or confirms the row. Grades (D-CL2):
**S** static (a Ghidra address), **C** config (a constant or ini key with its
path), **M** measured (run, date and sample size), **I** inferred. An empty
"known" cell means nothing is known yet.

### Network

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Largest datagram the client receives | 1472 B | S, `FUN_0158a200` (F1) | Oversized datagram outcome on a live client (`client.mercury.socket_recv` error 10040) | CL-05, CL-08 |
| Server datagram, fragment body, plaintext before ACKs | 1472 / 1300 / 1411 B | C, `crates/mercury/src/lib.rs`, `packet/build.rs`, `packet/ack_budget.rs` | none | CL-01 |
| Fragments per bundle | client: no cap; server: 64 | S `0x0157fd20`; C `MAX_FRAGMENTS` | Largest assembled bundle the client processes; any limit in `Bundle::iterator::data()`'s temp buffer | CL-02, CL-09 |
| Open fragment groups per channel | 1 | S, `Channel+0x124` | none | CL-01 |
| Client reorder window | 512 packets | S, `0x0158C801` | none | CL-01 |
| Server send window | 32 packets | C, `TX_WINDOW_SIZE` | Throughput at 32 against 64 and 128 (raise candidate) | CL-14 |
| Server resend timing | 700 ms, 20 tries, 5 per tick | C, `crates/mercury/src/lib.rs` | none | CL-01 |
| Client resend timing for its own reliable sends | | | RE: the client channel's RTO and retry cap | CL-02 |
| Client socket receive buffer | | | RE: any `setsockopt(SO_RCVBUF)`; else the Windows default | CL-02 |
| Packets the client processes per tick | | | RE: whether the game-thread queue drains fully per tick | CL-02 |
| Client idle send rate | about 6 pkt/s, `perfStats` every 15 s | M, colo 2026-09-19 (F8) | none | CL-01 |
| Client tolerance of server silence | 15 s | C, `NetInactivityTimeout`; `UE3_INACTIVITY_TIMEOUT_MS` | none | CL-01 |
| Next-request offset residue (#1341) | multiples of 8, 128 to 848, sticky per process | M, SigNoz 2026-09-29 to 2026-10-10 (F6) | Residue on a client without the DLL against a default client | CL-05, CL-10 |
| Residue exposure | first packet of a chain only | S, `0x01579cd0` | none | CL-01 |

### AoI and entities

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Player and NPC AoI radius, leave margin | 150 m / 100 m, +25 m | C, `crates/entity/src/cell_entity/` | none (the client has no radius of its own; confirm) | CL-03 |
| Entities the client holds at once | | | RE: any fixed-size entity table; live: count at which creates fail | CL-03, CL-09 |
| Creates in one flush the client absorbs | 24 in one packet seen; 28-NPC instances flush in about 11 packets | M, #1341 evidence; C, `deferred_flush.rs` doc | K at which `client.entity` creates go missing, per bundle shape | CL-09 |
| Deferred buffer | 512 messages, then drop | C, `MAX_DEFERRED_AOI_MSGS` | Whether a dense map reaches it | CL-05 |
| First-login hold | 16 s | C, `HOLD_DURATION` | none | CL-01 |
| Entities per `requestEntityUpdate` | 64 (server cap) | C, `request_entity_update.rs` | What the client sends after a burst | CL-09 |

### Process

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Address space | 2 GB unless Large Address Aware | I, 32-bit image | The PE flag on the stock exe; `total_virtual_mb` from SigNoz | CL-04, CL-05 |
| Working-set peak per world | | | `peak_working_set_mb` per world, from SigNoz first, lab for worlds with no data | CL-05, CL-08 |
| Lowest free address space seen | warning below 256 MB | C, `frame_health.rs` | `avail_virtual_mb` minimum per world | CL-05 |
| Cache archives held open | 22 `Cache.en-US` paks, read/write | M, #1312 | none | CL-01 |

### Engine

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Frame time | hitch threshold 200 ms | C, `HITCH_MS` | p50, p95, max per 30 s window (needs CL-06) | CL-06, CL-08 |
| Texture streaming pool | | | `[TextureStreaming]` `PoolSize` in the shipped ini chain | CL-04 |
| Frame smoothing | | | `bSmoothFrameRate`, `MaxSmoothedFrameRate`, `MinSmoothedFrameRate` | CL-04 |
| Background throttle | below-normal priority, 5 ms sleep per tick | M, lab guide (F13) | RE: the call site; any ini switch | CL-04 |
| Idle display loss | about 10 min, screensaver | M, lab operator note | none | CL-01 |

### Timing

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Server tick | 10 Hz, 100 ms | C, `game_clock/mod.rs` | none | CL-01 |
| Client interpolation and filter | | | RE: the avatar filter's time constants; check `docs/drafts/spec/position-updates.md` first | CL-03 |
| World entry | about 2.4 s Castle_CellBlock | M, colo 2026-09-19 | Per world, from SigNoz | CL-05 |
| Teleport to AoI refresh | | | `gmGotoXYZ` to the first create, from the committed tap fixture | CL-11 |

### Lab

| Bound | Known | Grade, source | To measure | Packet |
|---|---|---|---|---|
| Clients per daemon | 5 | C, `CEILING_MAX_CLIENTS` | Machine headroom at 5 (CPU, memory, GPU) | CL-08 |
| Bridge up with other clients running | about 25 s; watchdog grace 90 s | M, #1312 | none | CL-01 |
| Window | 1280x720 windowed | C, `DEFAULT_WINDOW` | Cost per client at 640x360 (raise candidate) | CL-15 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-CL1 | PROPOSED (coordinator) | **The bounds table lives in `docs/client/limits/`:** `README.md` (the one-page summary table and the grade key) plus `network.md`, `aoi.md`, `process.md`, `engine.md`, `timing.md` and `lab.md`. Each packet owns the area file its section names. Lab-chaos reads its safe and break levels from these files. | Six area files from day one (the foresight rule), and parallel packets never edit the same file. `docs/client/` is the doc-update map's home for client-side analysis; addresses also go into the RE findings they belong to. |
| D-CL2 | PROPOSED (coordinator) | **Every value carries a grade and a source:** static (address), config (constant or ini key with path), measured (run or query, date, sample size), inferred. No source, no value: the cell says "not measured". | The acceptance is a table people can trust without re-deriving it. |
| D-CL3 | PROPOSED (coordinator) | **Cheapest evidence first:** repo docs, then SigNoz (the DLL already reports memory, hitches and Mercury faults from player and lab sessions), then static RE through the `re-lookup` order, and only then the live lab. Every live packet waits for the user's OK. | The lab is shared, and most process numbers are already in SigNoz. |
| D-CL4 | **BlockedDecision** (maintainer). Recommended: **decide after CL-05** | **Large Address Aware.** (a) Set the flag on lab installs only, as a trial (CL-12), if any world's free address space falls under 512 MB; ship to players only through a later client-patch decision. (b) Do not set it. | It is a client patch (rules-and-gotchas). It only matters if the client gets near 2 GB; CL-05 says whether it does. |
| D-CL5 | **BlockedDecision** (owner). Recommended: **(a)** | **Lab ini overrides.** (a) A lab instance profile may carry ini overrides (pool sizes, smoothing) from a committed file, applied after seeding and logged; distributing any of them to players is a separate client-patch decision. (b) No ini change anywhere without a maintainer decision. | (a) lets the lab measure the effect of a raise without touching player installs. |
| D-CL6 | PROPOSED (coordinator) | **Server pacing changes are server-authoritative candidates,** each behind a measurement and a `bigworld-engine-advisor` review, with the old value one constant away. The first-login flush shape stays with #1341 and D-WC8; this campaign only measures it. | Server-side, no client patch (rules-and-gotchas). Two campaigns must not both change the flush. |
| D-CL7 | **BlockedDecision** (owner). Recommended: **defer** | **Lab capacity above five clients** (more seeded lab accounts, a daemon on a second machine). Defer until lab-chaos or lab-golden shows a need. The low-resolution window (CL-15) does not need this decision. | The ceiling is the seeded accounts; raising it is an account and machine decision. |
| D-CL8 | **BlockedDecision** (owner). Recommended: **(a)** | **The stock-client residue probe (CL-10).** (a) Run it on a lab client the user frees, with x64dbg non-freezing breakpoints only, once on a client with the DLL's hooks off and once on a default client. (b) Skip it and let #1341 choose its fix without knowing. | It is the decisive question for #1341's fix (server mitigation against a client patch of `0x01578e90`), and it needs a live client under a debugger. |

## Packets

| ID | Packet | Implementer | Size | Wave | Depends on | Status |
|---|---|---|---|---|---|---|
| CL-01 | Bounds docs skeleton under `docs/client/limits/` | documentation-writer | S | 1 | none | Ready |
| CL-02 | RE: client receive and resend limits | game-archaeology-specialist | M | 1 | none | Ready |
| CL-03 | RE: client entity capacity and interpolation | game-archaeology-specialist, bigworld-engine-advisor review | M | 1 | none | Ready |
| CL-04 | Static: PE flags, engine ini chain, background throttle | game-archaeology-specialist | S | 1 | none | Ready |
| CL-05 | SigNoz mining: memory, hitches, residues, flush faults | coordinator (`telemetry-triage` skill) | S | 1 | none | Ready |
| CL-06 | DLL: frame-time percentiles on `client.engine.memory` | packet-coder | S | 1 | none | Ready |
| CL-07 | Measurement spec `client-limits.toml` | packet-coder | S | 2 | CL-06 | BlockedDependency |
| CL-08 | Live: per-world memory and frame time, one and five clients | coordinator + Haiku lab-driver | M | 3 | CL-07, user's OK | BlockedDependency |
| CL-09 | Live: create-burst ceiling | coordinator + Haiku lab-driver | M | 3 | CL-01, user's OK | BlockedDependency |
| CL-10 | Live: stock-client residue probe | game-archaeology-specialist | M | 3 | D-CL8, user's OK | BlockedDecision |
| CL-11 | Teleport and world-entry timing from captures | packet-coder | S | 1 | none | Ready |
| CL-12 | Raise candidate: Large Address Aware lab trial | packet-coder | S | 4 | D-CL4, CL-05 | BlockedDecision |
| CL-13 | Raise candidate: lab ini overrides | packet-coder | M | 4 | D-CL5, CL-04 | BlockedDecision |
| CL-14 | Raise candidate: server send window | rust-gameserver-dev | M | 4 | CL-09 | BlockedDependency |
| CL-15 | Raise candidate: lab low-resolution window | packet-coder | S | 4 | CL-08 | BlockedDependency |
| CL-16 | Close-out | documentation-writer | S | 5 | all | BlockedDependency |

Size: S under about 40k tokens, M 40k to 70k, L 70k to 100k.

Waves (packets in one wave touch disjoint files):

1. CL-01 (`docs/client/limits/` skeleton), CL-02 (`network.md` RE rows, RE finding), CL-03 (`aoi.md`, `timing.md` interpolation rows), CL-04 (`process.md`, `engine.md` static rows), CL-05 (worknote only), CL-06 (`frame_health.rs`), CL-11 (`timing.md` capture rows). CL-02 to CL-05 and CL-11 write their rows into `worknotes/<packet>.md` if CL-01 has not merged yet; the coordinator folds them in.
2. CL-07.
3. CL-08, CL-09, CL-10: live, one at a time, each after the user's OK.
4. CL-12 to CL-15, each when its decision or dependency clears.
5. CL-16.

## Dispatch rules

- **Workers.** One packet each, in its own worktree:
  `pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 client-limits/<packet>-<slug> <worktree>`.
  The brief carries the worktree path, the packet section, the D-CL2 grade
  rule and the commit subject with the attribution lines.
- **RE packets** follow the `re-lookup` skill's order: repo docs and dispatch
  tables, then the Ghidra MCP, then headless Ghidra. Never x64dbg on a live
  client except in CL-10, after the user's OK, with non-freezing breakpoints
  only (condition `0` plus a log; fast resume off).
- **Live packets** (CL-08, CL-09, CL-10) follow the `lab-uat` skill: ask the
  user, take the lease yourself, brief a Haiku `lab-driver` with the gotcha
  sheet, never lend it a lease, and check its claims against the server log
  and the packet tap.
- **Review.** Each code packet gets a Sonnet `packet-reviewer`. RE packets
  get a second read by `bigworld-engine-advisor` on any claim about engine
  behaviour. CL-14 also gets `aoi-witness-broadcast`.
- **Shell.** PowerShell only: no bash, no direct `cargo`, no
  `git worktree prune`, no `git stash`. Every compiling command goes through
  `pwsh -NoProfile -File tools/build-lane/lane.ps1`.
- **Ship.** `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`, then
  `python tools/build-lane/ship.py merge <PR> --retire <worktree>`.
- **Shared files.** Only CL-16 edits `docs/gap-analysis*`,
  `docs/project-status.md` and `docs/guides/unified-uat.md`. Each packet owns
  only the `docs/client/limits/` file its section names.
- **Public repo.** No IPs, account names, machine names or local paths in any
  value or worknote. A measured value names the session id prefix and date,
  never the player.

## Review outcomes

None yet. Where merged work differs from the packet specs, record it here.
