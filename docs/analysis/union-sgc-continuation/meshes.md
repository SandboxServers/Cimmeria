# SGC_W1 Union: meshes, name_ids, dial sequences, crate room and chair (US-02)

> **Type:** Reference (RE packet, S0)
> **Packet:** [US-02](../union-zone-planning/sgc-packets/US-02.md) · **Index:** [sgc-work-packets.md](../union-zone-planning/sgc-work-packets.md)
> **Evidence rule:** every fact cites its source. `RESTORED_SOURCE` = read from the client build or the seeds. `CIMMERIA_RECONSTRUCTION` = a choice made from that evidence. Anything not read from data in this pass is marked **inferred** or **lab check owed**.
> **Client read:** cooked tree `C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC` (read only).

## Templates (5800, 5802, 5803)

| Template | Role | static_mesh | name_id | Moniker and text | Status |
|---|---|---|---|---|---|
| 5800 | Failsafe Device | `'Em-Props.EM-SGC_Failsafe00'` | **10726** | `DN_Ms_A01_SGC2_L_Failsafe`, text "Failsafe" | Mesh export found in `Packages\Em-Props.upk` (string scan, export name `EM-SGC_Failsafe00`). Moniker from `db/resources/Events/Seed/texts.sql` (10726). The text is a short fit ("Failsafe" for the device, not "Failsafe Device"). |
| 5802 | Pressable crate (1566 requisitions) | `'Em-Props.EM-SupplyCrate00'` (provisional) | **8369** (fallback) | `DN_Ob_SGCW1_sc_GateControlTerminal`, text "Stargate Control" | No "Supply Crate" moniker exists in `texts.sql` (only "Sanderson's Crates" in 10168, unrelated). 8369 is the index default fallback and reads wrongly for a crate; **the nameplate text is a known mismatch**. Mesh choice is inferred, see Crates. |
| 5803 | Carter terminal | `'Em-Props.EM-ViewScreen02'` | **8369** (fallback) | as above | No "Computer Terminal" moniker exists in `texts.sql`. The mesh string is already used by a seeded template (`entity_templates.sql`, `'Em-Props.EM-ViewScreen02'`), so the format matches. Fallback name is wrong for a terminal; the fallback is named here as the packet allows. |

**Mesh evidence.** `Packages\Em-Props.upk` contains the export names `EM-SGC_Failsafe00` and `EM-SGC_Failsafe_Console00` (string scan of the package bytes). The screen mesh `EM-ViewScreen02` (with `_FX-MI` and `_Pf0` variants) is in the same package. The SGC_W1 map tile `SGC_W1-0000ffff.umap` names `ViewScreen02`, and `SGC_W1-fffe0001.umap` names `Failsafe`; the full export name is not spelled out in the map bytes, so the Failsafe actor's package link is taken from the package name, not from the placed actor (**inferred**). The index places the Failsafe actor at (151.18, 1.28, -193.38) and the ViewScreen02 actor near (-49.48, 2.32, 36.38) (`sgc-work-packets.md`, placement table P04 and P06). The actor JSON from `extract_actors --json` carries class, location and rotation only, not the mesh name.

**Columns US-11 should copy (partly lab check owed).**
- Copy from the existing template rows in `db/resources/Content/Seed/entity_templates.sql`: `static_mesh`, `name_id`, `name`.
- `interaction_type` stays `0` on every SGC_W1 template (F1, `sgc-work-packets.md`); a press works only through a mission world-object mask (`1073741824`) that a chain sets. US-11 does not set `interaction_type`.
- `body_set`, `components` and `flags` for a pressable prop: **not compared in this pass** (the template rows were not diffed against an existing pressable template). **Lab check owed**, or US-11 copies them from the same row as the mesh.

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
| P10a | (257.157, 41.74) | -10.95 | `sgc-work-packets.md` placement table (proposal) |
| P10b | (259.131, 42.326) | -10.95 | same |
| P10c | (260.237, 43.42) | -10.95 | same |
| P10d | (255.923, 43.406) | -10.95 | same |
| P10e | (257.726, 43.679) | -10.95 | same |
| P10f | (254.369, 43.338) | -10.95 | same |

- **Positions** are the index proposals. They are not yet re-read from the SGC_W1 map. The six points form a cluster about 6 m wide (x 254.4 to 260.2) and 2 m deep (z 41.7 to 43.7), so one room reach is **inferred** (not measured). **Lab check owed** for reachability in one room.
- **Mesh for 5802.** `Packages\Em-Props.upk` has `EM-SupplyCrate00`, `EM-SupplyCrate01`, `EM-SupplyCrate02`, `EM-Cafeteria_Crate00/01`, `EM-Wooden_Crate_props00/01` (string scan). `EM_Earth_Military.upk` has `EM-Crate_Wooden00/01`. No SGC_W1 map tile names `SupplyCrate` or the `Wooden_Crate` names in the scan, so the placed crate mesh is not read. The provisional choice is `Em-Props.EM-SupplyCrate00` (inferred). **Lab check owed** to confirm the placed mesh in the requisitions room.

## Chair and floor

**Harriman's chair (P01).** The Control Room tile `SGC_W1-fffd0001.umap` names `EM-Office_Chair`; the package holds `Em-Props.EM-Office_Chair00`. The index gives yaw -180 at (172.082, -6.399, 46.335) (`sgc-work-packets.md`, P01 and chair row). A yaw of -180 is heading 3.1416 (the P01 seat heading), consistent with the packet. The seated pose is **lab check owed**.

**Gate Room floor in front of gate 27.** The gate row is (172.79, -8.47, 6.014) (`sgc-work-packets.md`, H11 and K5). The index proposes the floor at y -10.9 (H3 = P13 (172.79, -10.9, 12.0)). The floor height was **not** read from collision or BSP in this pass. The 2.4 m gap between the gate row and the floor is the index's own figure (**inferred**). **Lab check owed**: stand at P13 in the client and confirm the floor height.

## Lab checks owed

1. Placed crate mesh in the requisitions room (`Em-Props.EM-SupplyCrate00` provisional) and the one-room reach of P10a-f.
2. Harriman's seated pose on `Em-Props.EM-Office_Chair00` at P01, heading 3.1416.
3. Gate Room floor height at P13 (y -10.9), against the gate row y -8.47.
4. Whether the client plays sequence 10047 (MakeGate) and 10060 (CrossGate) from the Control Room (C2b, P01), and the gate 27 binding of 6100/6113 in the prefab.
5. The body_set, components and flags of a pressable prop (US-11 copies them, or the lab confirms the row).
6. Nameplate text for 5802 and 5803: both fall back to 8369 ("Stargate Control"), which is the wrong name for a crate and a terminal; a moniker from the client text set is wanted.

## Packet checks

- `docs/analysis/union-sgc-continuation/meshes.md` created (this file), CRLF.
- No client file, seed or code changed. The client was read only.
- Tools run: `extract_kismet --graph`, `tools/kismet_extractor.py --json` (stdout only, no repo output), `extract_actors --json` (through the lane). No new tool was written.
