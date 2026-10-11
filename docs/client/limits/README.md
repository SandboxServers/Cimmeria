---
title: "Client limits"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits

Every bound the server, the lab and chaos mode run into on the SGW client, each with its value, how sure we are of it, and where the number comes from. The [client-limits campaign](../../analysis/client-limits/README.md) fills it in, and [lab-chaos](../../analysis/lab-chaos/README.md) reads its safe and break levels from the area files.

## Grade key

Every value carries a grade and a source (decision D-CL2). A bound with no source says `not measured`, with the packet that will fill it in Notes, and its Grade and Source cells say `n/a`.

| Grade | Means | Source looks like |
|---|---|---|
| `static` | Read from the client binary | A Ghidra address (`0x0157fd20`) or a PE header offset |
| `config` | A constant or ini key | A repo path and constant (`crates/mercury/src/lib.rs` `TX_WINDOW_SIZE`), or an ini file and key |
| `measured` | Observed on a running client or server | Where, the date and the sample size (`SigNoz, 2026-10-12, 14 sessions`) |
| `inferred` | Reasoned from other facts, not observed | The facts it rests on |

No source names an IP, an account, a machine or a local path. A measured value names a session by its 8-character id prefix and date, never the player.

## Area files

Each area file has one table with exactly these columns, so lab-chaos can read it:

```markdown
| Bound | Value | Grade | Source | Safe level | Break level | Notes |
```

Safe level and Break level are filled only on the chaos-driven rows (the ones a lab-chaos profile drives, named in their Notes); every other row says `n/a`.

| File | Covers |
|---|---|
| [network.md](network.md) | Datagrams, windows, resend, fragments, bundles, the #1341 residue |
| [aoi.md](aoi.md) | AoI radius, entity count, create bursts, the deferred buffer, the first-login hold |
| [process.md](process.md) | Address space, working sets, cache archives |
| [engine.md](engine.md) | Frame time, texture pool, smoothing, background throttle, display loss |
| [timing.md](timing.md) | Server tick, interpolation, world entry, teleport to AoI |
| [lab.md](lab.md) | Clients per daemon, bridge start, window size |

## Summary

One row per bound. The area file holds the grade, the source and the notes.

| Bound | Value | Area file |
|---|---|---|
| Largest datagram the client receives | 1472 B | [network.md](network.md) |
| Server datagram maximum | 1472 B | [network.md](network.md) |
| Server fragment body | 1300 B | [network.md](network.md) |
| Server data plaintext before piggybacked ACKs | 1411 B | [network.md](network.md) |
| Fragments per bundle, client | no cap | [network.md](network.md) |
| Fragments per bundle, server | 64 | [network.md](network.md) |
| Open fragment groups per channel | 1 | [network.md](network.md) |
| Client reorder window | 512 packets | [network.md](network.md) |
| Server send window | 32 packets | [network.md](network.md) |
| Server resend timeout | 700 ms | [network.md](network.md) |
| Server resend tries | 20 | [network.md](network.md) |
| Server resends per tick | 5 | [network.md](network.md) |
| Client resend timeout for its own reliable sends | max(2.5 x smoothed RTT, 50 ms); first RTO 2.5 s | [network.md](network.md) |
| Client resend retry cap | none | [network.md](network.md) |
| Client resends per pass per channel | 6 | [network.md](network.md) |
| Client idle keepalive | empty bundle after 1.5 s without a send | [network.md](network.md) |
| Client socket receive buffer | OS default (client sets none); size not measured | [network.md](network.md) |
| Datagrams the network thread reads per pass | 64 per socket | [network.md](network.md) |
| Network thread idle wait | about 10 to 15.6 ms | [network.md](network.md) |
| Packets the client processes per tick | whole queue per frame, no budget | [network.md](network.md) |
| Largest message body the bundle loop can copy | no fixed cap | [network.md](network.md) |
| Packets in a chain between processPacket and processOrderedPacket | no cap | [network.md](network.md) |
| Client idle send rate | about 6 packets/s; `perfStats` every 15.0 s | [network.md](network.md) |
| Client tolerance of server silence | 15 s | [network.md](network.md) |
| Next-request offset residue (#1341) | multiples of 8, 128 to 848 B, sticky per process | [network.md](network.md) |
| Residue exposure | first packet of a chain only | [network.md](network.md) |
| Server-side added latency and jitter the session survives | not measured | [network.md](network.md) |
| Server-side loss rate the session survives | not measured | [network.md](network.md) |
| Flush delay after the hold before the client times anything out | not measured | [network.md](network.md) |
| Player AoI radius | 150 m | [aoi.md](aoi.md) |
| NPC default AoI radius | 100 m | [aoi.md](aoi.md) |
| AoI leave margin | 25 m | [aoi.md](aoi.md) |
| Entities the client holds at once | no fixed limit | [aoi.md](aoi.md) |
| Creates in one flush the client absorbs | not measured | [aoi.md](aoi.md) |
| Deferred buffer | 512 messages, then drop with a WARN | [aoi.md](aoi.md) |
| First-login hold | 16 s | [aoi.md](aoi.md) |
| Entities per `requestEntityUpdate` | 64 | [aoi.md](aoi.md) |
| Address space | 4 GB on 64-bit Windows (Large Address Aware set in the stock exe) | [process.md](process.md) |
| Working-set peak per world | not measured | [process.md](process.md) |
| Free address space warning | below 256 MB | [process.md](process.md) |
| Lowest free address space seen | not measured | [process.md](process.md) |
| Cache archives held open | 22 `Cache.en-US` paks, open for writing with read-only sharing | [process.md](process.md) |
| Hitch threshold | 200 ms | [engine.md](engine.md) |
| Frame time (p50, p95, max per 30 s window) | not measured | [engine.md](engine.md) |
| Busy cores at which frame time passes the hitch threshold | not measured | [engine.md](engine.md) |
| Texture streaming pool | not measured | [engine.md](engine.md) |
| Frame smoothing | not measured | [engine.md](engine.md) |
| Background throttle | below-normal priority, 5 ms sleep per `FEngineLoop::Tick` | [engine.md](engine.md) |
| Idle display loss | about 10 min, then `D3DERR_NOTAVAILABLE` | [engine.md](engine.md) |
| Server tick | 10 Hz (100 ms) | [timing.md](timing.md) |
| Server position-update spacing | 100 ms | [timing.md](timing.md) |
| Client render delay of remote entities (BigWorld filter class) | target 0.1 to 1.0 s, 8-frame ring | [timing.md](timing.md) |
| Client teleport snap (SGW filter class) | 2500 position units/s | [timing.md](timing.md) |
| World entry, Castle_CellBlock | about 2.4 s | [timing.md](timing.md) |
| Teleport to AoI refresh | not measured | [timing.md](timing.md) |
| Clients per daemon | 5 | [lab.md](lab.md) |
| Bridge up with other clients running | about 25 s | [lab.md](lab.md) |
| Watchdog grace before a first bridge answer | 90 s | [lab.md](lab.md) |
| Window | 1280x720 windowed | [lab.md](lab.md) |

## How to update

1. Change the row in its area file. Keep the grade and the source with the value: a new number with no source goes in as `not measured`, with the reason in Notes.
2. When a better grade replaces a worse one (a measurement confirming a static read, say), keep the stronger evidence in Source and move the other to Notes.
3. Update the matching row in the summary above in the same PR.
4. A new static address also goes into the RE finding it belongs to under [reverse-engineering/findings/](../../reverse-engineering/findings/), such as [client-mercury-receive-path.md](../../reverse-engineering/findings/client-mercury-receive-path.md).
