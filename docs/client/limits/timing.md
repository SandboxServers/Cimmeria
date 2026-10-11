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
| Client interpolation and filter | not measured | n/a | n/a | n/a | n/a | CL-03: the avatar filter's time constants; [position-updates.md](../../drafts/spec/position-updates.md) first. |
| World entry, Castle_CellBlock | about 2.4 s | measured | SigNoz, 2026-09-19, 1 colo session | n/a | n/a | `playCharacter` to `onClientReady`. CL-05 measures it per world. |
| Teleport to AoI refresh | not measured | n/a | n/a | n/a | n/a | CL-11: `gmGotoXYZ` to the first create, from the committed Praxis tap fixture. |
