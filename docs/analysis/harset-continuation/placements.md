# Harset continuation placement evidence

> Type: reference. Audience: placement/mission coordinator. Updated: 2026-10-14.
> Baseline: `d3efb7d41aad0b6008397f112d224ce7430d7acf`.
> Historical detail: [placement A](../harset-rebuild/placements/A-arrival-and-travel.md), [B](../harset-rebuild/placements/B-world57-population-and-regions.md), [C](../harset-rebuild/placements/C-interiors-68-69-70.md).

## Meaning of the coordinate classes

**Proved (seed)** means the exact current row/script relation is verified; it does not prove a retail placement or live-client safety. **Inferred** means the destination, actor assignment, region edge or encounter role was reconstructed from map geometry/landmarks/telemetry. **New** means a proposed addition without any coordinate assigned yet. Physical floor/mesh evidence can corroborate an inferred coordinate without turning the story assignment into restoration. All new coordinates stay BlockedEvidence until their source and checks are recorded. The coordinates below are display-rounded to six decimals; the named seed is authoritative for exact floating-point literals.

No coordinate was walked in HA-00b. Historical placement files contain mixed-date findings; the current seed and later NA26/NA29/NA36/NA40 artifacts take precedence. All four Harset worlds are now advisory and have `.nav` files; a missing mesh return in a guard cannot count as a pass. The old “no interior mesh”, 1,939-component snapshot and Storage component-36/upper-component-6 assertion do not describe the current rebuild. Re-measure current connectivity before accepting private encounters.

## Travel and recovery

| Anchor | World | Current X, Y, Z / heading | Class | Evidence / current guard |
|---|---:|---|---|---|
| Stargate 3 arrival | 57 | -0.076, -67.274002, 38.011002 / 3.141 | Proved (seed) | Gate row; temporary -5/-68.99/33 pin removed by NA29. `cell-world/.../harset_placement_tests.rs` pins row arrival, on-mesh disc and inert entry in point set 1001. |
| Command Center entry (6006) | 68 | 0, 0.355, -20 / 0 | Proved (reference script) | `deprecated/python/cell/spaces/Harset.py`; current space chain and `harset_space.rs` exact teleport/clearance guards. |
| Harset return (6007) | 57 | 0, -67.6, -231 / 0 | Proved (reference script) | `Harset_CmdCenter.py`; enabled seed, floor corroboration and opposing-box clearance; current mesh check required before enforcement. |
| Respawner 20 | 57 | -8, -68.99, 34 | Inferred | Geometry-derived plaza recovery, not original player start; current `harset_placement_tests.rs` ground guard. |
| Respawner 22 | 69 | 48, 3.61, 78 | Inferred | Floor/light-derived Market recovery; current mesh exists. |
| Respawner 23 | 70 | 50, 0, 44 | Inferred | Storage floor recovery; current mesh/topology guard. |
| Ring region 4 / `HarsetRingLeftBottomRegion` | 57 | -25.641001, -67.828003, 15.249 | Proved (seed), geometry corroborated | Existing pad; point set 2052; current guard asserts all five pads on rebuilt mesh. |
| Ring region 5 / `HarsetRingRightBottomRegion` | 57 | 25.738001, -67.828003, 15.35 | Proved (seed), geometry corroborated | Existing pad; point set 2053; current guard asserts all five pads on rebuilt mesh. |
| Ring region 7 / `HarsetRingLeftTopRegion` | 57 | -171.656998, -27.014999, 240.016006 | Proved (seed), geometry corroborated | Existing pad; point set 2055; current guard asserts all five pads on rebuilt mesh. |
| Ring region 8 / `HarsetinRingRightRegion` | 57 | 218.513, -34.125, 36.272999 | Proved (seed), geometry corroborated | Existing pad; point set 2056; current guard asserts all five pads on rebuilt mesh. |
| Ring region 6 / `HarsetRingLeftRegion` | 57 | -197.268997, -40.167, 84.349998 | Proved (seed), geometry corroborated | Existing pad; point set 2054; current guard asserts all five pads on rebuilt mesh. |

Ring switch routing 6001–6005 still maps exact tags to these pads. Gate arrival inside point set 1001 must not trigger someone else's pending dial. Reversing the NA29 pin decision requires a new reviewed reason, not an old M0 blocker.

## Current spawn coordinates

This table includes **every current spawn in worlds 57/68/69/70**, including old guards/switches. `spawnlist.sql` supplies identity/position/template/tag; historical A/B/C supply the inference rationale. Existing legacy coordinates are proved as current seed values, with original lineage unproved. Rows 300–314 and 340–349 are reconstructed placement assignments. Anat 222 and FirstBug 224 retain existing coordinates. Story actors with no row are listed separately below.

| Spawn | World | Tag / template | Current X, Y, Z / heading | Class | Placement source |
|---|---:|---|---|---|---|
| 4 | 57 | `HarsetRingLeftBottom` / 3 | -25.756998, -67.828003, 15.136 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 37 | 57 | `Harset_DHD` / 1 | 4.407, -68.957001, 31.525 / 3.139963 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 127 | 57 | `HarsetRingRightBottom` / 3 | 26.118998, -67.828003, 15.471 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 128 | 57 | `HarsetRingLeft` / 3 | -194.459, -40.167, 81.275002 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 129 | 57 | `HarsetRingLeftTop` / 3 | -171.774002, -27.014999, 235.979996 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 130 | 57 | `HarsetRingRight` / 3 | 215.817993, -34.125, 39.417999 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 222 | 68 | `CmdCenter_Anat` / 43 | 61.96904, 1.7715, 77.171577 / 3.117049 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 223 | 57 | `NULL` / 163 | -165.512543, -41.269627, 99.415474 / 3.215224 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 224 | 57 | `FirstBug` / 164 | -176.697678, -41.254002, 125.271324 / 5.939574 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 225 | 57 | `NULL` / 160 | -18.7497, -68.922798, 19.3561 / 1.546157 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 226 | 57 | `NULL` / 160 | -21.5497, -68.922798, 22.161198 / 6.258546 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 227 | 57 | `NULL` / 160 | -29.567801, -68.922798, 22.331799 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 228 | 57 | `NULL` / 160 | 18.6381, -68.922798, 19.436899 / 4.687845 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 229 | 57 | `NULL` / 160 | 21.5998, -68.922798, 22.186701 / 6.258546 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 230 | 57 | `NULL` / 160 | 29.662399, -68.922798, 22.382198 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 231 | 57 | `NULL` / 160 | 18.5478, -68.922798, 11.354899 / 4.736837 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 232 | 57 | `NULL` / 159 | 4.695996, -58.654087, -188.246613 / 0 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 233 | 57 | `NULL` / 159 | -5.131343, -58.65913, -188.338028 / 6.258642 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 234 | 57 | `NULL` / 160 | -18.643, -68.922798, 11.3282 / 1.59534 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 235 | 57 | `NULL` / 159 | -4.436214, -67.608292, -231.103455 / 1.570796 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 236 | 57 | `NULL` / 159 | 4.332336, -67.608292, -231.151123 / 4.712389 | Proved (seed) | Existing seed; reference script only for switch/door logic |
| 300 | 57 | `Harset_Hansen` / 212 | -24, -68.922798, 14 / 0.7836 | Inferred | PL-B-01 |
| 301 | 57 | `Harset_Jacobs` / 213 | -26.5, -68.922798, 18.5 / 0.935 | Inferred | PL-B-02 |
| 302 | 57 | `Harset_Lorak` / 201 | 22, -68.9, -30 / 5.709 | Inferred | PL-B-03 |
| 303 | 57 | `Harset_FormerRaJaffa` / 204 | -160, -41.28, 84 / 4.6335 | Inferred | PL-B-04 |
| 304 | 57 | `Harset_FormerRaJaffa2` / 204 | -160, -41.28, 89 / 4.4921 | Inferred | PL-B-05 |
| 305 | 57 | `Harset_SuspiciousJaffa` / 205 | -159, -41.28, -26 / 5.5862 | Inferred | PL-B-06 |
| 306 | 57 | `SecondBug` / 164 | -186, -41.28, 118.5 / 2.8993 | Inferred | PL-B-07 |
| 307 | 57 | `ThirdBug` / 164 | -147, -41.28, 104.5 / 3.6932 | Inferred | PL-B-08 |
| 308 | 57 | `Harset_ShieldTower1` / 243 | -223, -41.36, 37.72 / 4.7124 | Inferred | PL-B-09 |
| 309 | 57 | `Harset_ShieldTower2` / 243 | -168, -28.25, 233.5 / 1.2094 | Inferred | PL-B-10 |
| 310 | 57 | `Harset_ShieldTower3` / 243 | -3, -30.72, 285.5 / 0.7378 | Inferred | PL-B-11 |
| 311 | 57 | `Harset_ShieldControls` / 248 | -90, -30.7, 213.9 / 1.5557 | Inferred | PL-B-12 |
| 312 | 57 | `Harset_BankAnchor` / 248 | -184.5, -41.28, 162.27 / 4.7492 | Inferred | PL-B-13 |
| 313 | 57 | `Harset_PetbeQuarters` / 244 | -160.5, -28.25, 232.6 / 5.0039 | Inferred | PL-B-15 |
| 314 | 57 | `Harset_StorageLotaur` / 219 | -186, -41.28, 160 / 0.5814 | Inferred | PL-B-14 |
| 340 | 68 | `CmdCenter_Baal` / 42 | 2, 1.95, 88 / 3.141593 | Inferred | PL-C-01 |
| 341 | 68 | `CmdCenter_RoyalGuard` / 209 | 58.5, 1.75, 77 / 3.141593 | Inferred | PL-C-02 |
| 342 | 68 | `CmdCenter_SymbioteTank` / 245 | 66, 1.75, 77 / 3.141593 | Inferred | PL-C-03 |
| 343 | 68 | `CmdCenter_Mohkatan` / 54 | -5, 0.37, -8 / 2.746801 | Inferred | PL-C-04 |
| 344 | 68 | `CmdCenter_Marsh` / 10 | -49.5, -0.59, -14 / 1.570796 | Inferred | PL-C-05 |
| 345 | 68 | `CmdCenter_Copplemann` / 48 | -35.5, -0.59, -14 / 4.712389 | Inferred | PL-C-06 |
| 346 | 68 | `CmdCenter_Blackstock` / 214 | -50, -0.59, 6 / 2.761092 | Inferred | PL-C-07 |
| 347 | 68 | `CmdCenter_Nerus` / 53 | 34.8, 0.37, -25.5 / 0.18963 | Inferred | PL-C-08 |
| 348 | 68 | `CmdCenter_Opheltes` / 215 | -10, 0.45, 36 / 3.141593 | Inferred | PL-C-09 |
| 349 | 68 | `CmdCenter_Athena` / 44 | -18, 1.95, 84 / 2.498092 | Inferred | PL-C-10 |

Current guards: `crates/cell-combat/src/cell/spawner_tests/harset/{world57_placement,cmdcenter_population,templates,interior_regions}.rs` and `crates/cell/src/cell/service/ticks/npc_respawn/tests/harset_respawn.rs`. They pin tag/template relationships, nonhostile hub cast, region geometry and actual mesh reach. NA29 made 303/304/306/307/313 mobile. Spawn 308 remains stationary with a known ~3.44 m vertical gap; that is a calibration/raised-geometry question, not proof to move its mesh floor. Ordinary-player platform UAT remains open.

## Current named region coordinates

All listed region edges are **Inferred** unless explicitly inherited from a reference-script transition. Point rows are verified against the seed; geometry/telemetry corroborate physical volumes. Edge choice and gameplay meaning remain reconstructed. A cylinder is center + radius/height; a box shows min/max of actual point coordinates, not a new guess.

| Set / name | World | Current center or X/Y/Z extents | Points | Class |
|---|---:|---|---|---|
| 2055 / `HarsetRingLeftTopPS` | 57 | center (-171.656998, -27.014999, 240.016006) r=2.53 h=1.77 | 2332 | Proved (seed) |
| 2056 / `HarsetRingRightPS` | 57 | center (218.513, -34.125, 36.272999) r=2.53 h=1.77 | 2333 | Proved (seed) |
| 2054 / `HarsetRingLeftPS` | 57 | center (-197.268005, -40.167, 84.349998) r=2.53 h=1.77 | 2331 | Proved (seed) |
| 2078 / `Harset.CommandCenterTransition` | 57 | X[-7.069999, 6.579999]; Y[-67.659996, -60.539997]; Z[-243.519989, -238.409988] | 2381, 2382, 2383, 2384 | Proved (seed) |
| 2079 / `Harset_CmdCenter.HarsetTransition` | 68 | X[-13.059998, 10.63]; Y[0.32, 10.02]; Z[-37.579998, -29.48] | 2385, 2386, 2387, 2388 | Proved (seed) |
| 1001 / `Harset.Stargate` | 57 | center (-0.372, -67.363998, 37.352997) r=2.5 h=10 | 2392 | Proved (seed) |
| 2053 / `HarsetRingRightBottomPS` | 57 | center (25.737999, -67.828003, 15.349999) r=2.53 h=1.77 | 2393 | Proved (seed) |
| 2052 / `HarsetRingLeftBottomPS` | 57 | center (-25.641001, -67.828003, 15.248999) r=2.53 h=1.77 | 2394 | Proved (seed) |
| 2100 / `Harset.JaffaZone` | 57 | X[-208, -128]; Y[-42, -30]; Z[-70, 140] | 2500, 2501, 2502, 2503 | Inferred |
| 2101 / `Harset.OpCoreZone` | 57 | X[126, 256]; Y[-42, -30]; Z[-64, 164] | 2504, 2505, 2506, 2507 | Inferred |
| 2102 / `Harset.Bank` | 57 | X[-195, -178]; Y[-42, -34]; Z[155, 170] | 2508, 2509, 2510, 2511 | Inferred |
| 2103 / `Harset.PetbeQuarters` | 57 | X[-176, -154]; Y[-29, -21]; Z[222, 246] | 2512, 2513, 2514, 2515 | Inferred |
| 2104 / `Harset.ShieldControls` | 57 | center (-88.34, -30.700001, 213.925003) r=6 h=4 | 2516 | Inferred |
| 2105 / `Harset.ShieldTower1` | 57 | center (-226.039993, -41.360001, 37.719997) r=8 h=6 | 2517 | Inferred |
| 2106 / `Harset.ShieldTower2` | 57 | center (-166.117996, -31.129999, 234.796997) r=5 h=6 | 2518 | Inferred |
| 2107 / `Harset.ShieldTower3` | 57 | center (0, -30.719999, 288.799988) r=8 h=6 | 2519 | Inferred |
| 2120 / `Harset_CmdCenter.Lab` | 68 | X[27.5, 69]; Y[0.32, 10.02]; Z[-31, 11] | 2540, 2541, 2542, 2543 | Inferred |
| 2121 / `Harset_Market.Marketplace` | 69 | X[30, 100]; Y[3.6, 13.6]; Z[30, 100] | 2544, 2545, 2546, 2547 | Inferred |
| 2122 / `Harset_StorageRm.Storage` | 70 | X[19, 84]; Y[0.3, 4]; Z[38, 97] | 2548, 2549, 2550, 2551 | Inferred |

For sets 2078/2079, source script proves the transition behavior, while the seed proves current trigger geometry. The new regional edges 2100–2107/2120–2122 do not become original placements because their names match a mission. Storage set 2122 keeps Y 0.30–4.00 to avoid accepting the deck above; `interior_regions.rs` tests real inside/outside telemetry controls and path endpoints, not just `find_path().is_some()` (Detour can return partial paths).

## Candidate encounter points: coordinates remain inferred

These historical cover-derived coordinates are **unseeded** mission-spawn candidates. Recheck against the current mesh and sealed encounter graph before use; a cover-node cluster is not an original encounter source. Prefer pen-floor sites for Storage fights and prove endpoints reach the actual player floor. No timing/wave-size is inferred from these points.

| Historical anchor | World | X, Y, Z | Class / gate |
|---|---:|---|---|
| PL-C-E01 | 69 | 81.9, 3.65, 40.6 | Inferred; HA-05/12 |
| PL-C-E02 | 69 | 59.9, 3.65, 39.0 | Inferred; HA-05/12 |
| PL-C-E03 | 69 | 94.5, 3.65, 44.9 | Inferred; HA-05/12 |
| PL-C-E04 | 69 | 92.6, 3.65, 58.8 | Inferred; HA-05/12 |
| PL-C-E05 | 69 | 81.8, 3.65, 95.2 | Inferred; HA-05/12 |
| PL-C-E06 | 69 | 39.3, 3.65, 63.0 | Inferred; HA-05/12 |
| PL-C-E07 | 69 | 20.0, 3.84, 32.9 | Inferred; vestibule interpretation unproved |
| PL-C-E08 | 70 | 26.4, 0.35, 71.0 | Inferred; HA-08/13b |
| PL-C-E09 | 70 | 72.6, 0.30, 69.7 | Inferred; HA-08/13b |
| PL-C-E10 | 70 | 49.3, 0.42, 59.5 | Inferred; HA-08/13b |
| PL-C-E11 | 70 | 52.7, 0.35, 93.9 | Inferred; HA-08/13b |
| PL-C-E12 | 70 | 58.0, 0.30, 37.7 | Inferred; **0.3 m south of set 2122's Z minimum 38**, despite old blanket “all inside” claim; seal entry timing/region intentionally |
| PL-C-E13 | 70 | 49.3, 5.22, 5.8 | Inferred upper-wing candidate; prove current route to target floor |
| PL-C-E14 | 70 | 70.7, 5.17, 9.5 | Same gate |
| PL-C-E15 | 70 | 50.8, 5.22, 40.7 | Same gate |

## New placements and exact evidence gates

| New anchor | Coordinate state | Gate / packet |
|---|---|---|
| Harset↔Market, Harset↔Storage | New; all four thresholds and safe arrivals unassigned | HA-01a: owner observation or proved BSP doorway mapping, opposing-volume clearance and both-direction/relog test |
| Kaerlia by Harset gate | New; unassigned | HA-01b: Texts moniker 27315 proves Harset name, but no current template, costume or coordinate. Dialog 6169/DSM7016/speaker3219 is only a candidate join. |
| Bar / HoldingPens | New; unassigned | HA-06b/09: named place and mission consumer proof; historical reserved sets 2108/2109 |
| Haughty Goa'uld / Angry Jaffa / Lethander stall | New; unassigned | HA-09/04/13c: door/stall relation and safe shared-hub placement |
| Six scarab / four OP-CORE anchors | New; unassigned | HA-11/07: exact story/geometry and objective-action mapping; geometry landmarks are candidate sites only |
| Lab clickable consoles/sarcophagus | New interactable assignment; map renders four consoles at X 32.12/33.95/35.68/37.43, Y 0.32, Z -27.88 | Console positions are proved historical map measurements; no clickable template/story binding sealed. No sarcophagus position proved. |
| Vendors/trainers/ambient | New; unassigned | HA-15/16 policy and role/placement approvals; never duplicate quest speaker templates for ambient dialogue |

## Revalidation required by HA-02

`git log` identifies NA26 mesh replacement `936581a7e` (#794), NA36 class decoding `8ade99f0c` (#815), and NA40 opt-in static InterpActors `d02501165` (#833). NA40 records an unchanged accepted/rejected set for 67 seeded probes and 41 telemetry probes, and leaves five clustered raised points plus spawn 308 unresolved. The old “rebuilt mesh loses 12 positions” is not a current measured count. Re-run the existing nav_inspect/probe pipeline against the shipped mesh before proposing a repair; no new decoder/script is authorized here.

HA-02 must preserve all currently accepted telemetry positions, prove actual path endpoints for Market/Storage encounters and classify raised layers. A nav file existing, a partial path or advisory movement acceptance does not prove coverage. Keep advisory modes until independently accepted ordinary-player containment UAT. No current ledger row authorizes a lab lease or client/server change.
