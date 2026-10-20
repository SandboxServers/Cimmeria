# UO-19: Common Area vendor placements (partial worknote, part 2)

> **Type:** Reference (worknote, handoff) · **Packet:** [UO-19](../UO-19.md) · **Status:** in progress; no point sealed yet, no SQL yet

## Done

- Read UO-19 and UO-01's worknote and `.claude/agent-memory/movement-teleport-advisor/omega-placements.md`.
- Built `extract_actors` (`cimmeria-upk`), `archetype_census` (`cimmeria-navmesh-extractor`), `build-package-index` (`cimmeria-upk-objects`) and `nav_inspect` (`cimmeria-navmesh-extractor`, part 2) through `lane.ps1`. Built exes are under `B:\targets\uo-19\debug\`.
- Client data: the cooked client is at `C:\Program Files (x86)\FireSky\Stargate Worlds-QA\Working\SGWGame\CookedPC`. `CIMMERIA_COOKED_PC` is not set on this machine.
- Package index: `.claude\reports\union-omega\probes\package_index.bin` (5019 packages). Not committed.
- `extract_actors` output: `probes\omega_actors.json`. Generic names, UE units. Do not use for coordinates.
- `archetype_census` output (BigWorld metres): `probes\census_meshes.tsv` and `probes\census_positions.tsv` (261 rows; only hub-adjacent meshes, no counters, consoles aside, no vendors). Inside the hub box (x -110..170, z -230..70) there are 106 rows: consoles (`ASG-ComputerConsole01/02`, `ASG-Hologram_Console00`), `ASG-Sign00/01`, `ASG-Post00`, `ASG-Light02`, `ASG-TeleportBase00` (pad 14 prop at 34.56, -36.48, 55.68), `EM-Quartermaster01` (bank prop at -88.06, -48.29, -101.17), `EM-Tent_*`, `JF-Tent03` (24 instances, not on the hub box). The census does not name the Auction House, the counters or the pods, so no world landmark exists for them in the census.
- `nav_inspect` built. Anchor check (both anchors): component 60, `h` 0.00, exit 0. `data/spaces/omega_site.nav` has 247 components; component 60 is the main walkable mesh (6224 polys, bounds x[-434.9, 419.5] z[-252.7, 104.9]).
- Grid probe over the hub box (2 m grid, x -110..170, z -230..70, probe y -40): `probes\grid_out.txt` (not committed, 21291 probes, about 177 s). Rendered to `scratchpad\hub_map.png`. Findings from the render:
  - The hub walkable ring (component 60, grey) spans roughly x -50..122, z -42..70, centred on x about 36 (the N-S axis). Pad 14 (34.56, 55.68) sits at the north end, next to the Stargate.
  - A non-60 teal component (about x 18..54, z 0..40) fills the middle of the hub, round the Stargate/Battlemaster area. It is not on Modi's component, so points there fail the anchors.
  - The north-west and east pods are on other components (east pod is brown, about x 75..90, z 0..30), or are off the grid entirely (nothing walkable within 15 m of the NW pod approx, x -96..-100, z -36..-41).
- Art → world (approximate linear map, used only to pick a start for the search; the art does not affine-map, F34): `x = 36 + (px - 570) * 0.17`, `z = 70 - (py - 140) * 0.17`, from the enlargement `omega-site-common-area.webp`.
- First candidate pass (2 m grid cells, component 60, `h` ≤ 0.5, 2 m clearance on the 4 orthogonal cells, 2.5 m from each other and from the fixed beams (34.575, 55.698) and (34.41, -75.46)). Picks in `scratchpad\pick.txt`:
  - `ca.hweap.t1` (80, -8), `ca.harmor.t1` (72, 8), `ca.jweap.t1` (32, -2), `ca.jarmor.t1` (52, -8), `ca.consumable.1` (58, 18), `ca.consumable.2` (32, -6), `ca.consumable.3` (50, -10), `ca.auctioneer` (32, -10).
  - The picks are 5-10 m from their art spots: the art is denser than the mesh allows. Several picks sit on the same walkway, so the spacing rule needs a second pass.
  - `ca.hweap.t4` and `ca.harmor.t4` (NW large pod): no cell within 15 m. Per the packet, move them to the nearest hub walkway toward the pod (not yet searched).
  - Craft group (biomedical, electronics, power, materials) west of the Science Center pod (approx (38, -28)): not yet searched. Guides must sit at approx (32, -21) Biology and (44, -21) Physics.
- Seeded world-18 rows (`spawnlist.sql`, `spawnlist_union_omega.sql`, grep `, 18, [0-9]+, '`): beams 135 (34.575, 55.698, pad14 end), 132 (34.41, -75.46), 133 (-18.56, -207.52), 134 (-210.15, -92.33), 136 (191.98, -122.78); Modi 78 (39.54, -129.92); DHD 41 (42.60, -134.31); UO-01 banker 63820 (-88.06, -102.92), team registrar 63821 (30, -130), command registrar 63840 (28, -126). Ambients 63880-63901 are all x ≤ -79 or z ≤ -230 or x ≥ 215, so none is within 3 m of the hub. Respawner 180 (34.70, -138.00) and arrival (34.70, -142.00) are far south of the hub box.

## Left

1. **Pods.** Search the nearest comp-60 walkway toward `ca.hweap.t4` (-96, -41) and `ca.harmor.t4` (-100, -36) (the art's NW large pod). Record the key, the walkway and the distance. Repeat for the east pod keys if the pick (80, -8) is not on the east walkway.
2. **Crafting group.** Search the west side of the pod (x 14..30, z -40..-18) for four cells 2.5-3.5 m apart in the order biomedical, electronics, power, materials. Keep Biology (32, -21) and Physics (44, -21) free (2.5 m).
3. **Spacing pass.** Re-run the picks with the crowding rule (each key at least 2.5 m from every other key and the fixed beams). Pick the closer alternative if a key moves.
4. **Probe file.** Write `probes\ca_vendors.txt` (not committed): anchors first, then the 14 points, `NAME X Y Z`, Y = floor (`-40 - dy` from `grid_out.txt`). Run `nav_inspect` (built exe `B:\targets\uo-19\debug\nav_inspect.exe`) with stderr sent to a file (the component list is long). Run twice: once with `modi_seed78` first, once with `pad14` first. Keep only exit 0 with `h` 0.00. Re-snap any y that fails, then re-probe.
5. **Headings.** `atan2(dx, dz)` from the NPC to the hub axis point (34.5, z) that the vendor faces (a deviation: no prop is named for the counters). Normalise to [0, 2π), 4 decimals.
6. **Spacing check.** Against the rows above (2.5 m) and the ambients (3.0 m).
7. Write §§ Points, SQL (one `sql` block per consumer), Science Center, Landmarks, Rejected, Art conflicts, nav_inspect summaries (packet § Steps 6).

## Next step

Left 1 (pods). Use `scratchpad\hub_map.png` only as a visual check. The grid picks in `scratchpad\pick.txt` are a start, not sealed points. Do not reuse the pick script's output without the probe run.

## Lane status

No `status=` check line has been run yet for this packet's checks (nav_inspect probe sets are not run yet). Builds only: `extract_actors`, `archetype_census`, `build-package-index`, `nav_inspect` each `status=ok exit=0`.
