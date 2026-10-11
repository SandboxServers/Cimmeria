---
title: "Client limits: timing"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: timing

The clocks the SGW client and the server run to: the server tick, the client's interpolation, world entry and the teleport-to-AoI latency. Grades, column rules and the summary are in the [limits index](README.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Server tick | 10 Hz (100 ms) | config | `crates/wire/src/mercury/game_clock/mod.rs` `UPDATE_FREQUENCY_HZ` | n/a | n/a | Sent to the client at login. |
| Server position-update spacing | 100 ms | config | `crates/cell/src/cell/service/message_loop.rs` (tick interval); `tick_rate` 100 in `crates/wire/src/mercury/game_clock/mod.rs` | n/a | n/a | The server's cadence, not a client constant. |
| Avatar filter ring buffer (BigWorld class) | 8 frames | static | `DAT_01e69c8c`; `Input` `0x00e81030`, `Output` `0x00e824f0` | n/a | n/a | Floor assert `FrameCount > 4`. Frame stride 0x38. At 10 Hz the ring spans about 0.7 s. Two filter classes exist; which one an entity type uses is open ([CL-03](../../reverse-engineering/findings/client-entity-capacity.md)). |
| Render delay of remote entities (BigWorld class) | target 0.1 to 1.0 s | static | `DAT_01e69c84` / `DAT_01e69c88` in `0x00e824f0` | n/a | n/a | Target is `clamp(now - T, 0.1, 1.0)` with `T` the time of the sample before the newest, recomputed on the first `Output` after each accepted `Input`. "Tracks network latency" is inferred: if `Input` is given client receipt time, the target sits near the 0.1 floor except after hitches. Oldest-frame clamp: for entities with flag `+0x1b0` bit 0 the delay is capped at the buffer span (about 0.7 s at 10 Hz). Differs from stock BigWorld (CME change). |
| Render-delay convergence | 0.3 s per second | static | `DAT_01e69c80` in `0x00e824f0` | n/a | n/a | Linear, both directions: 0.1 to 1.0 takes 3 s. |
| Idle-velocity cut | mean frame speed 100.0 | static | `DAT_0181418c` in `0x00e824f0`; sum kept at `+0x220` / `+0x224` | n/a | n/a | At or below the cut the output uses 1 sample and writes zero velocity to the entity. The unit is the filter's position unit per second. |
| Extrapolation past the newest frame | BigWorld class holds position; SGW class extrapolates linearly, no cap | static | `0x00e81430` (velocity x 0.0), `0x00e81dc0` (only `dt > 0` tested) | not measured | not measured | Which filter class each entity type uses is open. |
| Teleport snap threshold (SGW class) | 2500.0 position units per second | static | `DAT_01e69c90` in `0x00e81970` | n/a | n/a | Test is `distance(newPos, framePos) / dt > 2500`, or reset bit 1. Unit inferred to be UE3 units (25 m/s): integer rotators, UObject filters, 100 UU per metre in the repo. Faster steps snap with no blend. Stock BigWorld has no velocity snap, so this is a CME change; do not revert it to stock. |
| Velocity smoothing (SGW class) | `v = (v_prev + 2 * v_obs) / 3` | static | `DAT_01816a84`, `DAT_01903cd4` in `0x00e81970` | n/a | n/a | Low-pass on position and angle rates. The angle step folds by `0x4001` (not a modulo), so a yaw jump above about 90 degrees between updates gives a wrong angular velocity. |
| Same-timestamp position update | discarded whole | static | `0x00e81970`, body inside `if (t - newestFrameTime != 0.0)` | n/a | n/a | Inferred consequence, if the time is per-packet receipt time: two updates for one entity in one bundle keep the stale one, and a tiny non-zero `dt` can inflate speed into a spurious teleport snap. Server advice: coalesce to the latest position per entity per bundle. |
| World entry (`playCharacter` to `onClientReady`) | p50 4.5 s, p95 24.6 s, all worlds | measured | SigNoz server log, 2026-10-11, 30 days, 251 entries | n/a | n/a | Per world in [World entry](#world-entry). The fastest Castle_CellBlock entry is 2.6 s; the earlier 2.4 s was one colo session on 2026-09-19. |
| Teleport to AoI refresh | not measured | n/a | n/a | n/a | n/a | The committed Praxis tap (`crates/wireclient/tests/fixtures/praxis_start_tap.json`, captured 2026-10-10) has two `gmGotoXYZ` teleports but no `CREATE_ENTITY` or `onStaticMeshNameUpdate` record, so there is no pair to time. Needs a tap that covers the AoI refresh. |
| Hold as observed (`onClientReady` to the first introduction) | not measured | n/a | n/a | n/a | n/a | The Praxis tap has no `onClientReady` record. The configured hold is in [aoi.md](aoi.md). |
| Largest gap between two server-to-client records | 15.7 s | measured | Praxis tap fixture, captured 2026-10-10, 34 gaps over 35 outbound records | n/a | n/a | `onKnownAbilitiesUpdate` to `onSequence`, during the opening cinematic; the next largest is 15.5 s. A gap between tapped records is not silence on the wire: the server's 1 s keepalive (`KEEPALIVE_INTERVAL_MS`) keeps feeding the client's 15 s inactivity timer ([network.md](network.md)). |

## World entry

`Client requests playCharacter` to `SGWPlayer.onClientReady received`, paired per client address, kept when 0 to 120 s apart (251 of 269). SigNoz, 30 days to 2026-10-11.

| World | Entries | p50 s | p95 s | Min s | Max s |
|---|---|---|---|---|---|
| Castle_CellBlock | 189 | 4.15 | 18.36 | 2.60 | 42.01 |
| DebugArea | 21 | 11.37 | 24.95 | 3.63 | 26.44 |
| SGC_W1 | 20 | 5.18 | 52.25 | 3.27 | 97.45 |
| Castle | 12 | 5.85 | 9.33 | 3.48 | 12.70 |
| Ihpet_Crater_Light | 3 | 15.61 | 23.46 | 8.67 | 24.33 |
