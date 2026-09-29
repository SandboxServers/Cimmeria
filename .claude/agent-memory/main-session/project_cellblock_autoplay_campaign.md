---
name: project_cellblock_autoplay_campaign
description: Cellblock autoplay campaign (2026-09-29) — plan and packets in docs/analysis/cellblock-autoplay/; nothing built yet; installed labd predates #1099-#1102
metadata:
  type: project
---

Campaign to drive the whole Castle Cellblock tutorial from the live research lab as one repeatable `lab_uat_run` section. Planned 2026-09-29 at `d8bff1baa`: [README](../../../docs/analysis/cellblock-autoplay/README.md) (goal, decisions D-AP1..11, ledger), [work-packets.md](../../../docs/analysis/cellblock-autoplay/work-packets.md), [scenario-map.md](../../../docs/analysis/cellblock-autoplay/scenario-map.md), [livewire-autosolve.md](../../../docs/analysis/cellblock-autoplay/livewire-autosolve.md).

Facts found in the research that matter before any live work:

- The labd under `%LOCALAPPDATA%\cimmeria-lab\bin` was built at or before #1094: no world, combat, UI or UAT tools. AP-00 adds an install script.
- `client_move_to` is straight-line steering, not pathing; the Cellblock needs recorded routes (AP-11).
- Livewire is won by clicking goal wires; the lab needs the server's board plus a hit map from the client's `Livewire.upk` (MG-L0..L5).
- Cover in the Cellblock is server-only proximity (5 m of a cover node, 1 Hz); there is no client key.

**How to apply:** resume from the README ledger; all D-AP rows were PROPOSED, awaiting the owner, when planned.
