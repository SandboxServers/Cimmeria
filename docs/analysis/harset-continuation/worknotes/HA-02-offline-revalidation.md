# HA-02 offline navmesh revalidation (Harset rebuilt meshes)

Scope: the "Revalidation required by HA-02" list in `placements.md` and README HA-D08,
measured offline against the shipped rebuilt meshes for worlds 57, 68, 69 and 70.
No lab, no client, no live DB, no lab-server nav_path. Repo HEAD c4762fcaa, nothing committed.

## Method

- No existing path probe could be reused offline (the only one is the forbidden lab-server `nav_path`).
  A scratch crate `navprobe` (scratchpad only, not in the repo) links `crates/entity` and calls the
  real `NavMesh::diagnose_point` and `NavMesh::find_path`.
- Reachability = `find_path` status Ok **and** the LAST waypoint within 2 m XZ of the request
  (same predicate as `path_reaches` in `interior_regions.rs`). Partial paths are therefore failures.
- On-mesh = `diagnose_point` valid (horizontal gate 1.2 m, jump tolerance 4.0). "h" is the
  horizontal distance to the nearest polygon surface.
- Component membership came from `nav_inspect` (polygon-adjacency graph). Its `locate` is
  Y-biased and gives false large dy values, so it was used for components only.
- Verdicts: PASS = valid and reachable. PASS-nudge = reachable but off-mesh by 1.2 to 2 m; use the
  snap coordinate. FAIL = partial path or off-mesh beyond 2 m.
- Origins: respawner positions and landings (named per table). They are not the real door
  landings of Market/Storage (unproved, HA-01a).

## World 69 (harset_cmdcenter.nav): E01-E07, Market pocket

Origins: R22 and PK_19_15. Landing-box result: the mesh covers the pocket only about x 15-25.

| Cand | On-mesh | Reachable | Last waypoint gap | Verdict | Conf |
|---|---|---|---|---|---|
| E01 | yes | yes | ~0 | PASS | high |
| E02 | yes | yes | ~0 | PASS | high |
| E03 | no (h 3.84) | NO, partial, last (93.3, 3.76, 40.2) | 4.85 m | FAIL | high |
| E04 | h 1.44 | yes | snap (91.8, 3.76, 60.0) | PASS-nudge | medium |
| E05 | yes | yes | ~0 | PASS | high |
| E06 | h 1.78 | yes | snap (39.01, 4.41, 61.25) | PASS-nudge | medium |
| E07 | h 1.56 | yes | snap (21.55, 3.93, 33.09) | PASS-nudge | medium |
| Market pocket box x[12,26] z[9,21] | 131 valid / 27 reachable-invalid / 5 valid-unreachable / 32 neither of 195 grid cells | partial | n/a | PARTIAL, shrink | medium |
| Market pocket safe sub-box x[17,24] z[9,21] | 104/104 valid and reachable | yes | 0 | PASS | medium |

Recommendation: shrink the Market landing box to about x[16,24] z[9,22]; move or drop E03.

## World 70 (harset_storagerm.nav): E08-E15, Storage ceiling 4.00

| Cand | On-mesh | Reachable | Gap | Verdict | Conf |
|---|---|---|---|---|---|
| E08 | marginal, h 1.20 | yes | 1.20 | PASS (marginal) | medium |
| E09 | yes | yes | ~0 | PASS | high |
| E10 | yes | yes | ~0 | PASS | high |
| E11 | yes | yes | ~0 | PASS | high |
| E12 | yes | yes | ~0 | PASS, but 0.3 m south of the region Z min 38 | medium |
| E13 | h 1.35 | yes | snap (50.65, 5.24, 5.67) | PASS-nudge | medium |
| E14 | yes | yes | ~0 | PASS | high |
| E15 | yes | yes | ~0 | PASS | high |

Grid check, pen region x[19,83] z[38,96]: 972 valid-reachable, 18 reachable-invalid, 0 unreachable
(all usable as the ceiling-4.00 landing box). Upper-wing box x[40,76] z[0,16]: 158 valid-reachable,
4 reachable-invalid, 9 neither.

### Task 2: pen floor to upper wing

CONNECTED on the rebuilt mesh. Pen floor (E08-E12) and upper wing (E13-E15) are one `nav_inspect`
component (component 4). `find_path` is Ok in both directions via ramps near z 36-38 at x 33-43
and x 59-70. The outdoor strip (component 5) is an unreachable control; component 6 is an
orphan sliver. Caveat: the Storage region (ceiling 4.00 plus the 1.5 m pad) excludes the upper
wing at y about 5.2, so the E13-E15 spawns sit outside the region even though they are reachable.

## World 68 (6006 landing)

Landing (0, 0.355, -20) is valid and reaches the Lab, spawns 340-349 and the region-2079 centre.
Region-2079 southern corners are off-mesh by 2.1 to 2.9 m (do not place on them). Verdict: PASS for
the landing, FAIL for the southern corners. Confidence: medium.

## World 57 (6007 landing)

Landing (0, -67.6, -231) and the region-2078 box are all valid and mutually reachable with the
Stargate arrival point in both directions (gap 0). Verdict: PASS, confidence high (world 57 also has telemetry).

## Caveats

- A passing path proves mesh connectivity, not that the level geometry is correct.
- Telemetry exists only for worlds 57 and 70. Worlds 68 and 69 are mesh-only evidence.
- Telemetry points at +1 m above the floor fail the 0.5 m start box (no_start_poly); that is a
  probe artifact, not a mesh fault.
- Spawn 308 and the five raised-platform points were NOT re-measured here.

## Commands run (exact strings are scratchpad-local; shapes below)

Scratchpad: `C:\Users\Steve\AppData\Local\Temp\claude\C--Users-Steve-source-projects-Cimmeria\59d78b93-1f5a-423e-9d89-f54aa42dc016\scratchpad`

```
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo build --manifest-path <scratchpad>\navprobe\Cargo.toml
Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo run -p navmesh-extractor --bin nav_inspect -- <nav>   (storagerm, cmdcenter/market)
B:\targets\zone-harset\debug\navprobe.exe <nav> <points.txt> <originsCSV|->      (WAYPOINTS=1 adds W lines)
python <scratchpad>\gridmap.py <nav forward-slash path> ...                         (ASCII reachability maps)
```

Full nav_inspect logs: `C:\Users\Steve\AppData\Local\cimmeria-build\logs\zone-harset\20261018-092321-50436.log` (storagerm), `...092324-35412.log` (market).

## Needs a human

1. Lab confirmation of the real Market and Storage door landings (HA-01a); this note used proxy origins.
2. Raised-platform and ordinary-player containment UAT in world 70.
3. Decide E03 (move or drop), the Market landing-box shrink, and whether the Storage region should be
   raised so E13-E15 are inside it.
4. Re-measure spawn 308 and the five raised points (not covered here).
