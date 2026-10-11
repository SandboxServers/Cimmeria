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
| Address space | 4 GB for a Large Address Aware 32-bit process on 64-bit Windows (2 GB on 32-bit Windows); the flag is set in the stock exe | static | `SGW.exe` file header `Characteristics` `0x0122` at file offset `0x13E` (`IMAGE_FILE_LARGE_ADDRESS_AWARE` `0x0020` set) | n/a | n/a | Same on the untouched install and on a launcher-set-up copy: the launcher edits only `DllCharacteristics` (offset `0x186`, `0x0040` stock, `0x0000` after setup). Nothing to patch. Runtime cross-check: `total_virtual_mb` in `client.engine.memory` (CL-05), about 4096 with the flag, about 2048 without. Under Wine `WINE_LARGE_ADDRESS_AWARE` is a separate switch. |
| Working-set peak per world | not measured | n/a | n/a | n/a | n/a | CL-05: `peak_working_set_mb` per world from SigNoz; CL-08: the lab, for worlds with no data. |
| Free address space warning | below 256 MB | config | `crates/client-telemetry/src/hooks/seams/frame_health.rs` `memory_level` (literal `256 * 1024 * 1024`) | n/a | n/a | A memory sample under this level is sent at `warn`. |
| Lowest free address space seen | not measured | n/a | n/a | n/a | n/a | CL-05: minimum `avail_virtual_mb` per world. |
| Cache archives held open | 22 `Cache.en-US` paks, open for writing with read-only sharing | measured | lab, 2026-10-10, single observation, #1312 ([live-research-lab.md § Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five)) | n/a | n/a | Held for the client's whole life, so two clients cannot share one Firesky folder. |
