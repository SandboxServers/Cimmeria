---
title: "Client limits: process"
type: reference
audience: engineers, the lab-chaos campaign
last_updated: 2026-10-11
---

# Client limits: process

The SGW client process's memory bounds: address space, working sets, free address space and the cooked-data archives it holds open. Grades, column rules and the summary are in the [limits index](README.md); the memory samples come from `client.engine.memory` ([client-telemetry.md § Subsystem seams](../../architecture/client-telemetry.md#subsystem-seams)).

| Bound | Value | Grade | Source | Safe level | Break level | Notes |
|---|---|---|---|---|---|---|
| Address space | 4 GB on 64-bit Windows (Large Address Aware set in the stock exe) | static | `SGW.exe` file header `Characteristics` `0x0122` at file offset `0x13E` | n/a | n/a | Confirm with `total_virtual_mb` in SigNoz (CL-05): about 4096 with the flag, about 2048 without. |
| Working-set peak per world | not measured | n/a | n/a | n/a | n/a | CL-05: `peak_working_set_mb` per world from SigNoz; CL-08: the lab, for worlds with no data. |
| Free address space warning | below 256 MB | config | `crates/client-telemetry/src/hooks/seams/frame_health.rs` `memory_level` (literal `256 * 1024 * 1024`) | n/a | n/a | A memory sample under this level is sent at `warn`. |
| Lowest free address space seen | not measured | n/a | n/a | n/a | n/a | CL-05: minimum `avail_virtual_mb` per world. |
| Cache archives held open | 22 `Cache.en-US` paks, open for writing with read-only sharing | measured | lab, 2026-10-10, single observation, #1312 ([live-research-lab.md § Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five)) | n/a | n/a | Held for the client's whole life, so two clients cannot share one Firesky folder. |
