---
name: reference_lab_target_timeout_cellblock_route_2026_10_13
description: Lab targeting timeouts and the observed Castle Cellblock Region8 walking route (2026-10-13)
metadata:
  type: reference
---

On 2026-10-13, `client_target`'s `read_mouse_over` bridge operation took 57.457 s before returning a dispatch timeout in a live Region8 hostile encounter. The client remained alive while combat continued. The lab target implementation now times out one mouse-over read after 1 s and caps the hover/view-retry phase at 10 s. A read timeout is an error with attempted pixel and remaining deadline, not `Hover::Nothing`; the tool exits before clicking, preserving the current target. The timeout behavior is unit-tested with an unresponsive simulated `WorldIo::slots()` future; live client validation remains outstanding.

The same run found that `(-338.4, 74.4, -213.9)` collides with a stasis pod. The walking route observed from near the guard body was approximately `(-334,73.47,-226) -> (-322,73.47,-225) -> (-321,73.47,-210) -> (-314,74.2,-186.7)`. These are observations, not a general pathfinding guarantee; spawn-dependent and live repeatability checks remain open.