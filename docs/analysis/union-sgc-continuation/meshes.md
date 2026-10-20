# SGC_W1 Union: meshes, name_ids, dial sequences, crate room and chair (US-02)

> **Type:** Reference (RE packet, S0)
> **Packet:** [US-02](../union-zone-planning/sgc-packets/US-02.md) · **Index:** [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md)
> **Evidence rule:** every fact cites its source. `RESTORED_SOURCE` = read from the client build or the seeds. `CIMMERIA_RECONSTRUCTION` = a choice made from that evidence. Anything not read from data in this pass is marked **inferred** or **lab check owed**.
> **Client read:** cooked tree `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC` (read only).

## Templates (5800, 5802, 5803)

| Template | Role | static_mesh | name_id | Moniker and text | Status |
|---|---|---|---|---|---|
| 5800 | Failsafe Device | `'Em-Props.EM-SGC_Failsafe00'` | **10726** | `DN_Ms_A01_SGC2_L_Failsafe`, text "Failsafe" | Mesh export found in `Packages\Em-Props.upk` (string scan, export name `EM-SGC_Failsafe00`). Moniker from `db/resources/Texts/Seed/texts.sql` (10726). The text is a short fit ("Failsafe" for the device, not "Failsafe Device"). |
| 5802 | Pressable crate (1566 requisitions) | `'EM_Earth_Military.EM-Crate_Wooden01'` | **8368** | `DN_Ob_SGCW1_int_LevelUpCrate`, text empty in `texts.sql` | RESTORED_SOURCE (seed): the SGC_W1 crate that already stands in the requisitions room is `entity_templates.sql` template 36 "SGC Crate" (mesh `EM_Earth_Military.EM-Crate_Wooden01`, name_id 8368, spawn 64 at (258.119, -10.848, 40.086), tag `SGC_W1_Crate`). 5802 should copy that row, not the string-scan guess `Em-Props.EM-SupplyCrate00` (rejected: nothing places it in SGC_W1). The moniker text is empty in the seed, so the nameplate is blank: **lab check owed** (blank versus 8369 "Stargate Control", which is wrong for a crate). No "Supply Crate" moniker exists (only "Sanderson's Ambition" in 10168, unrelated). |
| 5803 | Carter terminal | `'Em-Props.EM-ViewScreen02'` | **7550** | `DN_Ob_Sc_HumanScreen_Cellblock_SecurityTerminal`, text "Terminal" | The mesh string is used by seeded templates 19, 37, 173, 247, 248, 303, 310-313 (`db/resources/Entities/Seed/entity_templates.sql`), so the format matches. 7550 "Terminal" is the closest client text ("Computer Terminal" does not exist; 7720 "Communications Terminal" belongs to the Castle comm terminal) and is already template 19's name_id on this mesh. Fallback 8369 "Stargate Control" is the Gate Control terminal (template 37) and is wrong for Carter's terminal. |

**Mesh evidence.** `Packages\Em-Props.upk` contains the export names `EM-SGC_Failsafe00` and `EM-SGC_Failsafe_Console00` (string scan of the package bytes). The screen mesh `EM-ViewScreen02` (with `_FX-MI` and `_Pf0` variants) is in the same package. The SGC_W1 map tile `SGC_W1-0000ffff.umap` names `ViewScreen02`, and `SGC_W1-fffe0001.umap` names `Failsafe`; the full export name is not spelled out in the map bytes, so the Failsafe actor's package link is taken from the package name, not from the placed actor (**inferred**). The index places the Failsafe actor at (151.18, 1.28, -193.38) and the ViewScreen02 actor near (-49.48, 2.32, 36.38) (`sgc-work-packets.md`, placement table P04 and P06). The actor JSON from `extract_actors --json` carries class, location and rotation only, not the mesh name. **Seed cross-check (pass 2, RESTORED_SOURCE):** `db/resources/Visuals/Seed/static_meshes.sql` already lists `Em-Props.EM-SGC_Failsafe00` (line 9999), `Em-Props.EM-SGC_Failsafe_Console00` (8109), `Em-Props.EM-ViewScreen02` (7215), `Em-Props.EM-Office_Chair00` (2315) and `Em-Props.EM-SupplyCrate00/01/02` (7079/7077/7075), so the 5800 and 5803 `static_mesh` strings are valid mesh names in the seed's own list. Which export the placed Failsafe actor uses (device `EM-SGC_Failsafe00` versus `EM-SGC_Failsafe_Console00`) stays **inferred**; the P06 note in the index names `EM-SGC_Failsafe00`.

**Columns US-11 should copy (partly lab check owed).**
- Copy from the existing SGC_W1 rows in `db/resources/Entities/Seed/entity_templates.sql`. 5802 copies template 36 ("SGC Crate": `body_set` `GLB_Components.WorldObject_Small`, `components` NULL, `flags` 0, `interaction_type` 0, `level`/`alignment`/`faction` NULL, `class` `being`). 5803 and 5800 copy template 37 ("SGC Gate Control", the pressable terminal on `Em-Props.EM-ViewScreen02`: `body_set` `GLB_Components.WorldObject_Small`, `components` NULL, `flags` 4, `interaction_type` 0, `level` 1, `alignment` 0, `faction` 1, `class` `being`), with the mesh and `name_id` from the table above and `name` set to the packet's own name. Template 36 carries `flags` 0 and no faction (the crate); template 37 carries `flags` 4 and faction 1 (a terminal). Whether `flags` 4 matters for a press is **lab check owed**.
- `interaction_type` stays `0` on every SGC_W1 template (F1, `sgc-work-packets.md`); a press works only through a mission world-object mask (`1073741824`) that a chain sets. US-11 does not set `interaction_type`.
- (The `body_set`/`components`/`flags` comparison above was added in review from the seed rows; the coder pass had not done it.)

## Dial sequences (10047 and 10060)

**Source rows.** `db/resources/Events/Seed/sequences.sql`: `10047` → event `6100`, kismet `SGC_W1.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq` (line 3671); `10060` → event `6113`, same kismet name (line 3697). Event set 10004 holds 10047 to 10060 (one row per event type, 14 rows). The other SGC_W1 set members are 10048 (6101) to 10059 (6112).

**What each event is (RESTORED_SOURCE, SGW.exe via existing findings).**
- `6100` = `Stargate_MakeGate`: the gate vortex forms after a successful dial. Emitted by the server on the next 100 ms cell tick after dialing (`docs/gameplay/gate-travel.md`, `docs/gameplay/cinematic-system.md` section "Stargate Events (6100-6113)"; NA35 finding `stargate-dial-travel-timing-na35.md`).
- `6101`-`6104`: `MakeGateFull`, `MakeGateNow`, `DestroyGate`, `DestroyGateNow` (gate open/close variants, not sent by the server).
- `6105` = `DialFailure`.
- `6106`-`6112` = `Stargate_DHD1`-`DHD7`, the individual chevron locks. These are played by the client's own DHD UI. The server has no wire-level sight of chevron progress, so the server cannot key them (NA35, §1.4; `stargate-dial-travel-timing-na35.md`).
- `6113` = `Stargate_CrossGate`: the player passes the event horizon. Sent to the crossing player and to every witness on `Stargate_CrossGate` (`docs/gameplay/gate-travel.md`).

**So 10047 is the open (vortex forms) and 10060 is the kawoosh-and-cross moment.** Neither is a "dial" sequence in the strict sense. The dial itself is client-side and single-shot (NA35 §1.1): the server gets the finished address once and has no per-chevron event to drive.

**Kismet check (partly done).** `extract_kismet --graph` (`crates/upk/src/bin/extract_kismet.rs`) and `tools/kismet_extractor.py --json` both read SGC_W1 (37 tiles, 278 sequences). Both list two `GLB-Stargate_Prefab_Seq` instances (166 and 190 children). The Python decoder fails to unpack `SeqEvent_Stargate` exports (`unpack_from requires a buffer of at least 602 bytes ... offset 598`, one error per Stargate event export), so the event IDs and the target gate actor are **not read from the Kismet event properties**. Gate 27 and the 6100/6113 binding inside the prefab are therefore **inferred**, not proven. A `SeqAct_Delay` of 2.6 s sits in the prefab graph of tile `SGC_W1-00000001.umap` (chain 10 of the extractor JSON). Its trigger is `SeqEvent_Console`, not the Stargate event, so it is not proof of the dial-to-cross gap, only a candidate for it.

**Recommended pair and gap (CIMMERIA_RECONSTRUCTION).** `10047` (6100, open) at 0 ms, then `10060` (6113, cross) at 2500 ms. This is the D-US4 default that US-24, US-30 and US-32 already carry. The 2.5 s gap sits close to the 2.6 s delay in the prefab graph (inferred match). No better server-playable "dial then open" pair exists in set 10004: the chevron sequences 10053-10059 (6106-6112) are client-driven and have no server trigger (NA35 §1.4), and `6101`/`6102` (`MakeGateFull`/`MakeGateNow`) are alternative opens with no evidence they are used on SGC_W1.

**Does `play_sequence` need the gate in AoI?** The sequence is broadcast by the cell to the dialer and to every witness (`docs/gameplay/gate-travel.md`). The Gate Room player (C1, C2, C4) is a witness of the gate. At C2b the player stands by Harriman in the Control Room at P01, above the gate. Whether the client plays the gate's sequence from the Control Room is **lab check owed**.

## Crates (requisitions room, template 5802)

| Position | (x, z) | y | Source |
|---|---|---|---|
| P10a | (257.157, 41.74) | -10.85 | `placement.md` (US-01 seal; the index proposal said -10.95) |
| P10b | (259.131, 42.326) | -10.85 | same |
| P10c | (260.237, 43.42) | -10.85 | same |
| P10d | (255.923, 43.406) | -10.85 | same |
| P10e | (257.726, 43.679) | -10.85 | same |
| P10f | (254.369, 43.338) | -10.85 | same |

- **Positions** are the index proposals; the y is the US-01 sealed value (`placement.md`): the seeded crate (spawn 64) is at y -10.848 and the navmesh floor there is y about -10.76 (component 47, the Gate Room's own component), so -10.95 was about 0.1 m low. XZ were not re-read from the SGC_W1 map. The six points form a cluster about 6 m wide (x 254.4 to 260.2) and 2 m deep (z 41.7 to 43.7); US-01 found the room floor (y -10.85) is one navmesh component shared with the Gate Room, and standing points at z 40.0 reach all six within 4 m, so one-room reach is measured on the navmesh (the crate centres themselves sit at the floor edge). Placed crate meshes: **lab check owed**. **Lab check owed** for reachability in one room.
- **Mesh for 5802 (see the Templates row; the seeded crate wins).** `Packages\Em-Props.upk` has `EM-SupplyCrate00`, `EM-SupplyCrate01`, `EM-SupplyCrate02`, `EM-Cafeteria_Crate00/01`, `EM-Wooden_Crate_props00/01` (string scan). `EM_Earth_Military.upk` has `EM-Crate_Wooden00/01`. No SGC_W1 map tile names `SupplyCrate` or the `Wooden_Crate` names in the scan, so the placed crate mesh is not read. The string scan is superseded by the seeded SGC_W1 crate template 36 (`EM_Earth_Military.EM-Crate_Wooden01`). **Lab check owed** to confirm the placed mesh in the requisitions room.

## Chair and floor

**Harriman's chair (P01).** The Control Room tile `SGC_W1-fffd0001.umap` names `EM-Office_Chair`; the package holds `Em-Props.EM-Office_Chair00`. The index gives yaw -180 at (172.082, -6.399, 46.335); US-01 sealed Harriman at (172.08, -6.40, 47.30) because the chair centre is a navmesh hole (`sgc-work-packets.md`, P01 and chair row). A yaw of -180 is heading 3.1416 (the P01 seat heading), consistent with the packet. The seated pose is **lab check owed**.

**Gate Room floor in front of gate 27.** The gate row is (172.79, -8.47, 6.014) (`sgc-work-packets.md`, H11 and K5). The index first proposed the floor at y -10.9 (H3 = (172.79, -10.9, 12.0)); the later seal is P13 (167.00, -10.80, 18.00) in `placement.md`. The floor height was **not** read from collision or BSP. Navmesh evidence (US-01, `placement.md`, `data/spaces/sgc_w1.nav`): the Gate Room floor polys sit at y about -10.1 to -10.8 (component 47) and the gate-axis strip x 171-177, z 7-19 has no poly (the first seal was (170.79, -10.90, 11.00) at its edge; the final P13 is (167.00, -10.80, 18.00)). That agrees with y about -10.9 but is not a collision read, and the 2.4 m gate-row gap is the index's own figure (**inferred**). **Lab check owed**: stand at P13 in the client and confirm the floor height.

## Lab checks owed

1. Placed crate mesh in the requisitions room (seeded `EM_Earth_Military.EM-Crate_Wooden01`; blank nameplate of moniker 8368) and the crate clearance against the wall.
2. Harriman's seated pose on `Em-Props.EM-Office_Chair00` at P01, heading 3.1416.
3. Gate Room floor height at P13 (y -10.9), against the gate row y -8.47.
4. Whether the client plays sequence 10047 (MakeGate) and 10060 (CrossGate) from the Control Room (C2b, P01), and the gate 27 binding of 6100/6113 in the prefab.
5. Whether `flags` 4 (template 37) versus 0 (template 36) changes how a press or nameplate behaves.
6. Nameplate for 5802: moniker 8368 has empty text in the seed; confirm what the client shows. 5803 uses 7550 "Terminal".

## Packet checks

- `docs/analysis/union-sgc-continuation/meshes.md` created (this file), CRLF.
- No client file, seed or code changed. The client was read only.
- Tools run: `extract_kismet --graph`, `tools/kismet_extractor.py --json` (stdout only, no repo output), `extract_actors --json` (through the lane). No new tool was written.
