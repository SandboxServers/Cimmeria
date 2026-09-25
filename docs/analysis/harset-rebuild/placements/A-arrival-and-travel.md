# Placement cluster A — arrival and travel

> Type: reference (ledger). Audience: the owner correcting coordinates after a playtest, and the placement coordinator. Written 2026-09-19 under [METHOD.md](METHOD.md); every row is a **guess with a stated evidence class**, not a pin.
> Evidence sources: `archetype_census` / `obj_slab` / `nav_inspect` on the cooked maps, the **cleaned** telemetry probe files `harset_last_valid_probes.txt` (38 points) and `harset_storagerm_last_valid_probes.txt` (20) with `harset_suspicious_points.txt`, and the rebuilt `mse13.nav` / `mse25.nav` as a reachability second opinion. The older probe file under `placements/data/` is superseded — prefer the cleaned files.
> Branch `harset/placement-A`. Commits: `35da8ec8` (PL-A-01..05), `3ab9dbbf` (PL-A-06..07).

Scope: the gate-3 arrival, the three missing respawner rows, the five ring pads, and the Command Center return door plus the mission chains that were waiting on it. Population, named regions and the Market/Storage door pairs are **not** in this cluster — the last of those is in [No idea](#no-idea).

## Ledger

| ID | What | World | X, Y, Z, heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|
| PL-A-01 | Gate arrival pin — `stargates.stargate_id = 3`, columns `arrival_x/y/z/yaw`. **Dropped 2026-09-25 (NA29): travellers now arrive on the gate row; see [the NA29 finding](#pl-a-01--the-pin-was-dropped-na29)** | 57 | -5.0, -68.99, 33.0, yaw 3.141 | MAP-LANDMARK (the two `GA-Torch00` gate-mouth props), MAP-GEOMETRY (`obj_slab` floor + navmesh component), AUTHORED (yaw repeats the gate row's own) | HIGH | On-mesh, interior to hub component **187**, whole agent-radius disc on-mesh (13/13 ring samples at r = 0.6); floor `-68.99` topmost up-facing surface in the column, second sheet at `-69.31`; 1.1 m from the west torch; 6.35 m clear of point set 1001 `Harset.Stargate`; `validate_gate_arrival` returns `Validated` under `enforce` | Dial Harset from anywhere. You should materialise standing on the plaza flagstones a few metres off the gate's left shoulder, facing down the plaza (away from the ring), able to walk immediately | `db/resources/Worlds/Seed/stargates.sql`, the `arrival_*` values on the `'Harset'` row (all four or none — there is a CHECK) |
| PL-A-02 | `respawners` row 20 "Harset Gate Plaza Respawn" | 57 | -8.0, -68.99, 34.0 | MAP-GEOMETRY | HIGH | On-mesh, interior to component 187, **37/37** samples on the r = 0.6 and r = 1.2 rings also interior; floor `-68.99` (lower sheet `-69.31`); 8.33 m clear of point set 1001; the length of the zone clear of point set 2078 | Die anywhere in Harset and accept the respawn. You should come back on the gate plaza, 3 m west of where a gate traveller lands | `db/resources/Worlds/Seed/respawners.sql`, row 20 |
| PL-A-03 | `respawners` row 22 "Harset Market Respawn" | 69 | 48.0, 3.61, 78.0 | MAP-GEOMETRY (floor only), MAP-LANDMARK (`EM-StandingLight05`) | **MEDIUM** | Floor `3.61` is the topmost up-facing surface in the columns at (48, 78), (47, 77) and (49, 79), on a 16 m² / 34-triangle patch spanning x[46, 50] z[76, 80], nothing overhead; 3.0 m from the authored floor lamp at (47.96, 3.48, 81.04). **No on-mesh check — world 69 has no `.nav` file** | Die inside the Market. You should come back on the market floor near a standing lamp, not in a wall and not falling | `db/resources/Worlds/Seed/respawners.sql`, row 22 |
| PL-A-04 | `respawners` row 23 "Harset Storage Room Respawn" | 70 | 50.0, 0.0, 44.0 | MAP-GEOMETRY | HIGH | On-mesh in **component 36** of `harset_storagerm.nav` (3,055 m² / 314 polys at y 0.2–1.2, the room's own floor rather than the 82,249 m² whole-map ground sheet that is component 0); floor `0.00` exactly, matching all nineteen `GA-Fence00/03` prop origins; inside the fence-free band (every fence has z ≥ 51.2) | Die inside the Storage Room. You should come back in the open northern half, clear of the crate/fence maze | `db/resources/Worlds/Seed/respawners.sql`, row 23 |
| PL-A-05 | Ring pads, regions 4–8 — **investigated, deliberately NOT changed** | 57 | unchanged | AUTHORED, corroborated by MAP-GEOMETRY | HIGH (that the rows are right) | See [the ring-pad finding](#pl-a-05--the-ring-pad-rows-are-right-the-navmesh-is-not) | Ring between all five Harset pads. All five hops should land you standing on a ring platform | `db/resources/Worlds/Seed/ring_transport_regions.sql` — **but read the finding first**; the rows are not the defect |
| PL-A-06 | Chain 6007 Command Center → Harset return door: `enabled` false → **true**, coordinate unchanged | 57 (arrival) | 0.0, -67.6, -231.0, heading 0 (the executor sends `rotation [0,0,0]`; 0 = +Z, which is away from the door) | RECOVERED_SCRIPT (`Harset_CmdCenter.py:15`) + MAP-GEOMETRY (the floor measurement that unblocked it) | HIGH | `obj_slab` up-facing floor at `-67.64` in the columns at (0, -231), (±1, -231) and (0, -230), stable across 4/4/6 loaded chunks; nearest ceiling `-61.13`, so 6.5 m of headroom; the seeded y sits **0.04 m** above the floor; **7.41 m** clear of point set 2078's AABB in Z. Still **off-mesh** — see [the finding](#pl-a-06--the-return-door-is-open-and-still-off-mesh) | Walk into the Command Center's Harset door. You should arrive in the southern Harset corridor facing away from the door, on a floor, and be able to walk back through the door deliberately (not be bounced) | `db/resources/Content/Seed/harset_space_chains.sql`, chain 6007. Disabling it again means disabling 6511/6512/6513/6528 in the same change |
| PL-A-07 | Mission 1361 acceptance trio 6511/6512/6513 `enabled` false → **true**; new abandon twin chain **6528** | 68 | no coordinates | n/a (chain enablement, gated on PL-A-06) | HIGH | `praxis_acceptance_is_enabled_iff_the_return_door_is` widened to all four chains; 6528's clear-before-repaint ordering pinned | Talk to Marsh in the Command Center as a Human after handing over Frost's letter — a "?" should appear and the Praxis briefing should be acceptable. Abandon it and the "?" should come back | `db/resources/Content/Seed/harset_opcore_chains.sql`, chains 6511-6513 and 6528 |

## Second opinion: the rebuilt meshes

The Castle-nav session's rebuilt meshes (`mse13.nav` and `mse25.nav`, under the shared temp tree at `cimmeria-castle/navmesh/harset/Harset/`) have 374 components against the shipped mesh's 1,939 and were built for a humanoid agent — `height 1.8 / climb 0.6` rather than the shipped mesh's `0.6 / climb 0.9`, which is very likely a large part of why the shipped one shatters. They were used as a **reachability second opinion only**. They are not in `data/spaces`, nothing loads them, and no test references them — the guards keep validating against the shipped `data/spaces/harset.nav`, which is what `validate_gate_arrival` and `check_arrival` actually load.

Every point in this cluster is on-mesh on **every mesh that exists for its world**:

| Point | shipped `harset.nav` | `mse13.nav` | `mse25.nav` |
|---|---|---|---|
| PL-A-01 gate pin `(-5.0, -68.99, 33.0)` | component **187**, h 0.00, dy -0.19 | component **11**, h 0.00, dy -0.16 | component **11**, h 0.19, dy -0.19 |
| PL-A-02 respawner 20 `(-8.0, -68.99, 34.0)` | **187**, h 0.00, dy -0.19 | **11**, h 0.00, dy -0.16 | **11**, h 0.00, dy -0.16 |
| PL-A-04 respawner 23 `(50.0, 0.0, 44.0)` | `harset_storagerm.nav` component **36**, h 0.00, dy -0.20 | Storage `mse13.nav` component **4**, h 0.00, dy -0.10 | — |
| PL-A-06 chain 6007 arrival `(0, -67.6, -231)` | **off-mesh** (nearest poly 28.6 m above) | **11**, h 0.00, **dy +0.03** | **11**, h 0.00, dy +0.03 |
| Ring pads 4 / 5 / 6 / 7 / 8 | only pad 4 on-mesh | **all five in 11**, h 0.00, dy ≤ 0.13 | same |
| The raw gate row `(-0.076, -67.274, 38.011)` | off-mesh by 4.60 m in XZ | **11**, h 0.00, dy -0.04 | same |

Three conclusions, in order of how much they change:

1. **PL-A-05 is settled.** All five ring pad rows are on-mesh within 0.13 m on a correctly built mesh. The rows were never the defect; the shipped mesh is, and "the nearest polygon is 9–238 m away" is a build artefact, not geometry.
2. **PL-A-06 is independently confirmed.** `obj_slab` put a floor 0.04 m under the recovered arrival; the rebuilt mesh puts a *walkable surface* 0.03 m under it. Two tools, two methods, same answer — the 2009 coordinate is correct and always was.
3. **PL-A-01's pin is a workaround for the shipped mesh, not a correction of the data.** On a rebuilt mesh the raw gate row is on-mesh too (dy -0.04, i.e. standing on the dais). So once `harset.nav` is rebuilt the owner may prefer to **drop the four `arrival_*` values** and arrive on the gate itself, which is what the 2009 server did. The pin should be treated as reversible, not as new canon. `harset_gate_arrival_pin_is_on_the_mesh_and_the_gate_row_is_not` had a control assertion that failed the moment the shipped mesh made the row standable. **It fired on 2026-09-25:** NA26 shipped the rebuilt `harset.nav`, and the raw gate row is on it (dy 0.04). The test is now `harset_gate_arrival_pin_and_the_gate_row_are_both_on_the_mesh`; the pin stays until the owner decides whether to drop the four `arrival_*` values. **Decided the same day (NA29):** the owner said "as long as the arrival is on the navmesh and doesn't cause issues put it in the original location", and it passed every check, so the pin is gone; see [the NA29 finding](#pl-a-01--the-pin-was-dropped-na29).

**PL-A-04 survived a scare worth recording.** The cleaned Storage telemetry puts real players on at least five storeys (y -1.68, 1.2-1.6, 6.1-7.1, 9.5-9.7, 13.0, 15.5-17.7), and the two nearest anchors to respawner 23's XZ are `(52.26, 7.06, 43.36)` and `(51.39, 6.22, 23.04)` — about 7 m *above* it. That reads at first like the row is on a floor nobody uses, or under one. It is not: respawner 23 is in component **36** on the shipped mesh alongside 7 of the 20 anchors, and in component **4** on `mse13` alongside **14 of the 20**, including both of those y≈6-7 points — `mse13`'s component 4 spans y 0.0-6.2, so the lower floor and the walkway are one connected space. One real anchor, `(51.71, -1.68, 53.30)` with 172 rows, sits on the *lower* of the two floors `obj_slab` found at that column (-1.28 and 0.00), so both are used and the row is on the upper one.

Also worth a note for whoever rebuilds: `(50, 2, 50)` accounts for 7,958 of world 70's rejects and is on the synthetic list, so the round `x = 50` in respawner 23 is a coincidence of the room's geometry, not an inherited default. Its evidence is `obj_slab` plus component membership, never telemetry.

**Do not read the rebuilt mesh as strictly better.** Scored against the 38 cleaned real-player positions at roughly `is_point_valid` tolerance: the shipped mesh accepts **38/38** but spreads them over **12 components**; `mse13`/`mse25` accept **24/38** in just **3 components**. Permissive-and-disconnected versus connected-and-missing-ground; neither is finished. The useful property of these placements is that they survive both.

## Findings

### PL-A-01 — the pin was dropped (NA29)

On 2026-09-25 the four `arrival_*` values on the `'Harset'` row were set back to `NULL`, so `desired_arrival` returns the gate row `(-0.076, -67.274, 38.011)` and its own yaw 3.141 — the 2009 `moveTo(addr.xPos, …)` behaviour. The pin above is kept in the ledger as history. Every check ran against the shipped NA26 `data/spaces/harset.nav`:

| Check | Result |
|---|---|
| On the mesh | `is_point_valid` true, h 0.00, dy -0.04 (standing on the gate dais) |
| Agent-radius disc | 13/13 samples on-mesh at r = 0.6, and 13/13 at r = 1.2 |
| Hub component | `find_path` from the row returns `Ok` to the plaza gateway (1.0, -68.92, 2.9), respawner 20, the DHD (spawn 37), all five ring pads (4–8) and the Command Center door (0, -67.6, -231) |
| `validate_gate_arrival` | `Validated` under `enforce`, position and yaw verbatim |
| Facing | yaw 3.141 → (0.0006, -1.0): -Z, out of the gate and down the plaza |
| Trigger volumes | **Inside** point set 1001 `Harset.Stargate` (cylinder r 2.5, h 10 at (-0.372, -67.364, 37.353); the row is 0.72 m off its axis and 0.09 m above its base). Harmless, see below |

**Why landing inside the gate volume is harmless.** The client's first region hint after arrival can be an enter on 1001. The server resolves it against the server-known position, so it is accepted, and then does three things, none of which move the traveller:

1. `handle_stargate_region_entered` looks up a dial keyed on *the traveller's own entity*. The traveller has none: the crossing cancels the dial before `perform_gate_travel`, and `destroy_entity` scrubs `pending_gate_dials` with the old cell entity. Another player's open gate is theirs alone (`SGWPlayer` kept `dialedAddress`/`gatePassable` on the player too), so an arrival can never be sent straight back through a wormhole someone else opened.
2. `fire_enter_region("Harset.Stargate")` matches no chain: no `content_triggers` row is keyed on that tag.
3. The ring FSM ignores it: no `ring_transport_regions` row points at set 1001.

No destination-side gate sequence is emitted either. The 2009 server landed travellers in the same place, in the same volume.

**Guards.** `harset_gate_arrival_is_the_gate_row_on_the_mesh_and_inert_in_the_gate_volume` (replacing `harset_gate_arrival_pin_and_the_gate_row_are_both_on_the_mesh`) asserts no pin, the disc, the facing, and both data halves of "inert". It fails if anyone authors a chain or ring pad on that volume, since every traveller would fire it on landing. It also has a control that fails if the row leaves the volume, because the inertness checks would then stop protecting anything. `a_traveller_arriving_while_another_player_holds_an_open_dial_is_not_crossed` pins the per-entity dial. `validate_gate_arrival_accepts_the_harset_gate_row_verbatim` pins `Validated`. To go back to a pin, set all four columns (the CHECK requires all or none) and rewrite the first guard.

### PL-A-05 — the ring pad rows are right, the navmesh is not

The five `ring_transport_regions` rows for world 57 (regions 4–8, tags `HarsetRing*`) are the pad arrival coordinates: `ring_transport/runtime.rs` copies `dst.x/y/z` straight out of the row. All five are **correct**. `obj_slab` finds an up-facing ring-platform surface — a disc about 1.1 m above the surrounding floor — within **0.04 m** of every one of the five authored `y` values:

| Region | Row `y` | Surrounding floor | Ring-platform top | Nearest `harset.nav` polygon | Verdict |
|---|---|---|---|---|---|
| 4 `HarsetRingLeftBottom` | -67.828 | -69.31 / -68.95 | **-67.86** | component **187**, 1.02 m below the row | on-mesh (inside the +4.0 jump tolerance) |
| 5 `HarsetRingRightBottom` | -67.828 | -69.31 / -68.95 | **-67.86** | component 858, **237.6 m above** — a different storey | off-mesh |
| 6 `HarsetRingLeft` | -40.167 | -41.31 | **-40.20** | component 653, 9.4 m above | off-mesh |
| 7 `HarsetRingLeftTop` | -27.015 | -30.98 / -28.22 | **-27.05** | component 2, 11.8 m above | off-mesh |
| 8 `HarsetinRingRight` | -34.125 | -35.27 | **-34.16** | component 1184, 13.4 m below | off-mesh |

So nothing here wants re-pinning — and the rebuilt meshes settle it: all five pads are on-mesh there within 0.13 m (see [Second opinion](#second-opinion-the-rebuilt-meshes)). `harset.nav` simply has no polygon at the correct floor height for four of the five pads — the same class of coverage defect as the Command Center door below, and the one H53 works around.

**Why nobody has noticed.** World 57 is `navmesh_mode = 'advisory'`, so `check_arrival` answers `Unvalidated` for every pad and `audit_ring_pads` prints nothing at boot. The advisory mode is the *only* thing keeping those four ring destinations alive: the day someone sets world 57 to `enforce` without rebuilding the mesh, `ring_transport::runtime::tick` starts aborting every trip to pads 5/6/7/8 and releasing the passengers. `four_of_the_five_harset_ring_pads_survive_only_because_world_57_is_advisory` failed first and said so. Since NA26 (2026-09-25) the shipped `harset.nav` covers all five pads, and the guard is now `all_five_harset_ring_pads_are_on_the_mesh_and_advisory_refuses_none`.

Pads 5 and 8 deserve a second mention: their nearest polygons are on the *wrong storey* (+237.6 m and -13.4 m). Any future recovery code that reaches for `NavMesh::get_nearest_point` around a ring pad would teleport the passenger into another floor of the map.

### PL-A-06 — the return door is open, and still off-mesh

H10 shipped chain 6007 disabled and asked M0 for a new coordinate. Three things were open; the cooked map answered all three without a walk.

1. **Is there a floor at (0, -231)?** Yes — `obj_slab` reads an up-facing surface at `-67.64` with 6.5 m of headroom, and the row's `-67.600` is 0.04 m above it. Independently confirmed after the fact: the rebuilt `mse13.nav` puts a walkable polygon 0.03 m under the same point. This was the last reason the walk was needed. (One earlier `obj_slab` invocation reported *only* down-facing surfaces at that column; that run's box was centred 10 m away at (-0.25, -240.9) and loaded a different chunk set. The reading above is stable across boxes that load 4, 4 and 6 chunks. If you re-check this, vary the box.)
2. **Does it ping-pong?** No. Point set 2078 `Harset.CommandCenterTransition` is an AABB over z -243.52…-238.41; the arrival at z -231 is 7.41 m north of its nearest face. H10 asked for "about 8 units" and the recovered coordinate already had it, so it was **not moved** — the floor measurement and the clearance are both properties of that exact point, and nudging it 1 m to make a prose number exact would be the kind of invention METHOD forbids.
3. **The navmesh?** Still off-mesh, and now demonstrably a mesh defect rather than a reason to keep the door shut. Nothing within ~20 m of that door is on-mesh at the real floor height: the nearest polygon to the arrival is **28.6 m above** it and at the door threshold **51.7 m above** it. There was no on-mesh coordinate to move to, so "wait for an on-mesh pin" was waiting on a navmesh rebuild, not on a playtest.

Off-mesh is survivable for two independent reasons, and the guard now pins both instead of the old `enabled ⟹ on-mesh` implication: world 57 is `advisory` so the movement validator fails open, and respawner row 20 (PL-A-02) now exists so even an enforcing world recovers to the gate plaza instead of returning `UnrecoverableOffMesh`. Flip either and `harset_return_arrival_is_offmesh_but_survivable` fails.

**Facing is correct by luck, not design.** `cross_world_teleport` cannot carry a yaw — the executor sends `rotation [0, 0, 0]`. yaw is `atan2(dx, dz)` with 0 = +Z, the door is at z ≈ -241 and the arrival at z -231, so +Z happens to be away from the door. Worth knowing before someone "fixes" the executor.

### PL-A-07 — what the blocked chains were actually waiting on

| Item | Stated blocker | Resolution |
|---|---|---|
| Chains 6511/6512/6513 (1361 acceptance) | chain 6007 — a biconditional pinned by `praxis_acceptance_is_enabled_iff_the_return_door_is` | **Enabled.** The blocker was 6007, which PL-A-06 opened |
| 1361's abandon twin | "1361 cannot be accepted, so there is nothing to abandon" (H54) | **Authored as chain 6528**, modelled on 6308, and added to the 6007 biconditional — which closes the gap H54 named (the twin's `enabled` flag was not covered by anything) |
| Chain 6331 (1326 Lan'toc offer) | **not** a coordinate and **not** 6007 | **No change.** 6331 already ships `enabled = true`. The coordinator note asked whoever opened 6007 to consider *disabling* it, on the rule "a mission that cannot be started beats one that cannot be finished" — opening 6007 removes that reason, because 1326's 68 → 57 leg (step 3960) now works. Its remaining blocker is that mission 1325 (packet H21) is unauthored, so 1326 is unreachable upstream; that is H21's to fix |

### Other things that block arrival or travel

1. **The plaza around the gate is shredded.** `harset.nav` has 1,939 components. The strip on the gate centreline (x -4…+4, z 26…38) has no mesh at all, and the immediate plaza is split across components 1072, 1081, 1114, 1133, 1145, 1146, 1157, 1162 and 1172. The hub component 187 only reaches the gate from the west, which is why PL-A-01 is offset 7 m. `obj_slab` reads **one flat floor** across x[-10, +3] z[20, 41], so this is a build artefact, not geometry. Consequence: no NPC can path across the gate plaza today.
2. **The zone's busiest point is not in the hub component.** `(1.0, -68.92, 2.9)` — the plaza gateway between the two `GA-GuardPost00` props — carries 6,049 of the 28,988 reject rows that quote a *clean* accepted position, the largest of the 38. It survives the synthetic-point filter (`harset_suspicious_points.txt` excludes only `(0,0,0)` ×100,649 and `(1,1,1)` ×1,256 for Harset, so despite the round `x = 1.00` this is a real player), and four more clean anchors sit within 5 m of it (`(2.90, -67.97, 2.38)`, `(1.93, -67.95, 4.22)`, `(1.73, -68.74, 3.47)`, `(2.27, -67.92, 6.00)`). It is in component **1028** on the shipped mesh, which is why PL-A-02 is not there; on `mse13` that whole cluster joins component 11, so it becomes the obvious re-pin the day the mesh is rebuilt.
3. **World 70 `Harset_StorageRm` is `enforce` against a mesh whose largest component is a whole-map ground plane.** Component 0 is 82,249 m² of flat surface at y 0.2–0.4 spanning x[-99, 199] z[-99, 199] — far larger than the room. Any containment check there will accept positions well outside the playable space. PL-A-04 deliberately targets component 36 (the room's own floor) instead.
4. **World 69 `Harset_Market` has a degenerate AABB** — all four bounds are `0` in `entities/spaces.xml:15` (audit defect H-B11, still open). Inert only because `WorldDef.min_x..max_y` are parsed and never read. Confirmed again here.

## No idea

### H14 / H15 — the Market and Storage door pairs

**Not seeded.** METHOD requires MAP-GEOMETRY evidence on *both* sides of a door pair before seeding one, and the interior side has none.

What was searched, and what was missing:

- **World-57 side — ambiguous, not absent.** There are 20+ `GA-Interior:GA-*_doorway_*` meshes in the Harset map (`GA-large_doorway_open_a_00` ×4 at x 232.6, `GA-large_doorway_close_a_00` ×5, `GA-small_doorway_close_a_00` ×3, `GA-small_doorway_open_a_00` ×13) and 255 `TriggerVolume` actors. Every TriggerVolume is named exactly `TriggerVolume` and every one sampled sits on a prop origin — the guard posts at (±12.97, -67.8, 3.5), the doorway meshes at (232.60, -67.72, 76.84) and (232.60, -67.72, 84.52), the awnings, the fences. They are per-prop collision volumes, not authored door triggers. **Nothing in the map data, the seed, or the surviving Python says which doorway leads to the Market and which to the Storage Room.** Picking one would be a coin flip dressed as evidence, and the Castle Romney lesson is exactly that.
- **Market side (world 69) — no doorway at all.** `archetype_census` finds **three** distinct StaticMesh archetype families in the whole map: `JF-WallTorch01`, `EM-StandingLight05`, `JF-Brazier00`. No doorway mesh, no door frame, no threshold prop. `extract_actors` finds exactly **one** `TriggerVolume`, at BigWorld (68.34, **-2.07**, 58.87) — about 5.7 m *below* the 3.61 interior floor, so it is not a doorway threshold. `extract_kismet` finds 451 nodes across 20 sequences and they are ambient-sound plumbing (`SeqAct_PlaySound`, `SeqAct_Gate`, `SeqAct_Delay`, `SeqAct_StringOperations`); there is no door-open sequence and no teleport action.
- **Storage side (world 70) — no doorway either.** Two archetype families in the whole map (`GA-Fence00`, `GA-Fence03`). Of its 22 `TriggerVolume` actors, 19 sit exactly on fence-prop origins (e.g. (48.04, 1.28, 76.95) on the fence at (48.040, 0.000, 76.950)) and pair with the `SeqEvent_Touch` + `SeqAct_GetVelocity` + `SeqCond_CompareFloat` ambience sequences. Three more — (53.92, 3.84, 5.76), (27.68, 3.84, 40.96), (55.36, 3.84, 15.36) — are at a different height and north of the fenced area, and are **unexplained**. They *might* be a door. "An unexplained volume at a plausible height" is INFERRED, which METHOD caps at LOW, and a door pair is the one placement class where a wrong guess is a soft-lock rather than a cosmetic error.

What would unblock it, cheapest first:

1. One in-client walk: stand in the Harset doorway that loads the Market and read the coordinate; repeat for the Storage Room. Two numbers close both packets.
2. Failing that, decode the interiors' **BSP** rather than their StaticMeshes. Both maps are almost entirely BSP with prop dressing, so the door opening is a `UModel` brush face, not a mesh — and that is where the threshold would be found. (Note the Castle experience: brush-owned `Model` exports there all decoded to stubs, cause unproven.)
3. The three unexplained Storage TriggerVolumes are worth one `obj_slab` pass each if someone is already in that map: an up-facing floor with a wall-height gap on one side would turn INFERRED into MAP-GEOMETRY.

Interior arrival points for both doors are unseeded for the same reason. Point-set ids 2100-2149 and coordinate ids 2500-2799 remain free.
