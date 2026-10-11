---
title: "Client limits: engine"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: engine

The SGW client's Unreal Engine 3 bounds: frame time, the texture pool, frame smoothing, the background throttle and the display loss that stops an idle lab client. Grades, column rules and the summary are in the [limits index](README.md).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Hitch threshold | 200 ms | config | `crates/client-telemetry/src/hooks/seams/frame_health.rs` `HITCH_MS` | n/a | n/a | `client.engine.hitch` fires on a tick gap at or above it. |
| Frame time (p50, p95, max per 30 s window) | not measured | n/a | n/a | n/a | n/a | Nothing reports ordinary frame times yet. CL-06 adds the fields; CL-08 measures them per world. |
| Busy cores at which frame time passes the hitch threshold | not measured | n/a | n/a | not measured | not measured | Chaos profile `cpu`. CL-08 fills the safe level from the five-client p95. |
| Texture streaming pool | not measured | n/a | n/a | n/a | n/a | CL-04: `[TextureStreaming]` `PoolSize` in the shipped ini chain. |
| Frame smoothing | not measured | n/a | n/a | n/a | n/a | CL-04: `bSmoothFrameRate`, `MinSmoothedFrameRate`, `MaxSmoothedFrameRate`. |
| Background throttle | below-normal priority, 5 ms sleep per `FEngineLoop::Tick` | measured | lab, #1312 ([live-research-lab.md § Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five)) | n/a | n/a | Applies to a client not in the foreground; the lab's virtual focus defeats it. CL-04: the call site and any ini switch. |
| Idle display loss | about 10 min, then `D3DERR_NOTAVAILABLE` | measured | lab ([live-research-lab.md § The display](../../guides/live-research-lab.md#the-display-screensaver-and-d3d)) | n/a | n/a | The screensaver takes the display because lab input never resets the idle timer. The lab supervisor now holds the display awake. |
