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
| Working-set peak per world | see [per world](#per-world-memory) | measured | SigNoz `client.engine.memory`, 2026-10-11, 30 days, 45 sessions | n/a | n/a | Typical player working set 1.2 to 2.0 GB in the world. `peak_working_set_mb` is cumulative per process, so read it as "peak so far". CL-08 adds lab runs for worlds with little data. |
| Free address space warning | below 256 MB | config | `crates/client-telemetry/src/hooks/seams/frame_health.rs` `memory_level` (literal `256 * 1024 * 1024`) | n/a | n/a | A memory sample under this level is sent at `warn`. |
| Address space seen at runtime | 4096 MB in every sample | measured | SigNoz `client.engine.memory` `total_virtual_mb`, 2026-10-11, 5,669 samples, 45 sessions (16 lab, 29 player) | n/a | n/a | Confirms the Large Address Aware row above on every client that reported. |
| Lowest free address space seen | 135.5 MB, Ihpet_Crater_Light, player | measured | SigNoz `avail_virtual_mb` minimum, 2026-10-11, 45 sessions | n/a | n/a | All 7 `warn` samples (under 256 MB) in 30 days came from Ihpet_Crater_Light player sessions on 2026-10-05 and 2026-10-06; seven sessions fell under 600 MB, all there. Lab minimum: 807.5 MB, also Ihpet. Any texture-streaming override (CL-13) must be measured there first. |
| Largest committed (private) memory seen | 3540.4 MB, Ihpet_Crater_Light, player | measured | SigNoz `private_mb` maximum, 2026-10-11, 45 sessions | n/a | n/a | The address-space pressure is committed memory, not working set. |
| Cache archives held open | 22 `Cache.en-US` paks, open for writing with read-only sharing | measured | lab, 2026-10-10, single observation, #1312 ([live-research-lab.md § Parallel clients](../../guides/live-research-lab.md#parallel-clients-up-to-five)) | n/a | n/a | Held for the client's whole life, so two clients cannot share one Firesky folder. |

## Per world memory

SigNoz `client.engine.memory` over the 30 days to 2026-10-11. Each sample is attributed to the latest visible level package in its session. Read player rows for typical numbers: idle lab clients sit unfocused for long stretches.

| World | Kind | Samples | Sessions | Min free MB | Max private MB | Median working set MB |
|---|---|---|---|---|---|---|
| Ihpet_Crater_Light | player | 1,105 | 8 | 135.5 | 3540.4 | 1784 |
| Ihpet_Crater_Light | lab | 227 | 7 | 807.5 | 2858.0 | 1182 |
| Castle_CellBlock | player | 568 | 17 | 1754.2 | 1875.0 | 1238 |
| Castle_CellBlock | lab | 3,259 | 11 | 1634.0 | 2029.7 | 198 |
| Castle | player | 86 | 4 | 1372.8 | 1959.0 | 1532 |
| SGC_W1 | player | 32 | 2 | 1055.8 | 1819.1 | 989 |
| SGC_W1 | lab | 22 | 5 | 2119.0 | 1506.4 | 980 |
| Agnos | player | 13 | 1 | 1136.1 | 1810.0 | 1434 |
| Dakara_E1 | player | 4 | 1 | 1113.3 | 2494.5 | 1999 |
| Harset | lab | 3 | 1 | 1771.9 | 1897.6 | 1253 |
| Tollana | player | 2 | 1 | 1092.6 | 2012.3 | 1756 |

Method and queries: [CL-05 worknote](../../analysis/client-limits/worknotes/CL-05.md).
