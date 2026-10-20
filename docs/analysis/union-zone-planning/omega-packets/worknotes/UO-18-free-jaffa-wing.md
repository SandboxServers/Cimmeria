# UO-18: navmesh-sealed Free Jaffa wing placements (partial, part 1)

> **Type:** Reference (worknote) · **Packet:** [UO-18](../UO-18.md) · **Status:** partial, coder session 1 stopped on context. Provenance: CIMMERIA_RECONSTRUCTION (coordinates from the shipped navmesh); art areas are RESTORED_SOURCE layout intent only (D-UO15).

## Done

- Built the probe tool through the lane (`cimmeria-navmesh-extractor`, bin `nav_inspect`). Run the built exe directly: `B:\targets\uo-18\debug\nav_inspect.exe`.
- Anchors re-probed on `data/spaces/omega_site.nav`: `modi_seed78` 39.54 -51.91 -129.92 exit 0 (component 60, h 0.00, dy -0.09); `pad14` 34.56 -36.48 55.68 exit 0 (component 60, h 0.00, dy -0.79).
- Grid scan of x 200..400, z -140..-20 (2 m) at six y guesses (-62..-112): the wing is on component 60. Scan files are in `.claude\reports\union-omega\probes\` (not committed).
- Spacing table built from the seed rows (`spawnlist_union_omega.sql`: 63820-63821, 63840, 63880-63897), the respawner 180 (34.70, -138.00, rule 3.0), the Modi and DHD spawns, the ring beams (region 10, 11, 12, 14, 16), and the UO-01 points.

### Points found so far (probed on the mesh, `h` 0.00, component 60; exit codes against both anchors NOT yet run)

| Key | Spawn | Template | Tag | x | y | z | Heading | Source |
|---|---|---|---|---|---|---|---|---|
| `fj.jweap.t1` | 63853 | 63831 | `UnionOmega_JaffaWeaponsT1` | 390.35 | -84.80 | -44.56 | 4.7124 | UO-01 `svc.jaffa_weapons` reused, exact probe |
| `fj.jweap.t2` | 63854 | 63832 | `UnionOmega_JaffaWeaponsT2` | 387.85 | -85.02 | -44.56 | 4.7124 | ring search, 2.5 m from t1 |
| `fj.jweap.t3` | 63855 | 63833 | `UnionOmega_JaffaWeaponsT3` | 384.85 | -85.02 | -45.56 | 4.7124 | ring search, 3.0 m from t2 |
| `fj.jweap.t4` | 63856 | 63834 | `UnionOmega_JaffaWeaponsT4` | 381.85 | -84.80 | -44.56 | 4.7124 | ring search, 3.0 m from t3 |
| `fj.jarmor.t1` | 63859 | 63835 | `UnionOmega_JaffaArmorT1` | 216.50 | -63.59 | -66.17 | 0.0000 | UO-01 `svc.jaffa_armor` reused, exact probe |
| `fj.jarmor.t2` | 63860 | 63836 | `UnionOmega_JaffaArmorT2` | 213.50 | -63.19 | -66.17 | 0.0000 | ring search |
| `fj.jarmor.t3` | 63861 | 63837 | `UnionOmega_JaffaArmorT3` | 210.50 | -63.63 | -66.17 | 0.0000 | ring search |
| `fj.jarmor.t4` | 63862 | 63838 | `UnionOmega_JaffaArmorT4` | 207.50 | -63.25 | -66.17 | 0.0000 | ring search |
| `fj.craft.biomedical` | 63968 | 63839 | `UnionOmega_JaffaCraftingBiomed` | 291.14 | -60.90 | -109.35 | 1.5708 | UO-01 `svc.jaffa_crafting` reused, exact probe |

Heading for the tiers copies the UO-01 heading of the reused point (weapons and armor rows face the same way as their t1). Heading is not yet derived from a facing target.

## Left

- `fj.craft.electronics`, `fj.craft.power`, `fj.craft.materials` (spawns 63969-63971, templates 63840-63842): no point found. The four axes (+x, -x, +z, -z) at 3 m from biomedical with a 1.5 m ring search returned nothing. Widen the ring search to 2.5-3.5 m, or allow more y guesses.
- `fj.trainer.jaffa` (63808, template 63808, tag `UnionOmega_JaffaTrainer`): no point. The art's trainer sits at ring centre; the grid scan shows no walkable cell at (200..210, -122), so the ring centre is not located by the scan.
- `fj.consumable.1` (63871) and `fj.consumable.2` (63872), template 63828: no point. Both depend on the trainer position.
- Landmarks: no Omega_Site `.umap` was found on disk (a search of `C:\Users\Steve\source` timed out and returned nothing), so `extract_actors` could not run. The table of art spot to world landmark is therefore not written.
- Verification: exit codes for every chosen point against both anchors (30 runs), with `h` 0.00.
- SQL blocks (`### For UO-12`, `### For UO-28`, `### For UO-29`, `### For UO-30`, `### For UO-31`), § Rejected, § Art conflicts, § nav_inspect summaries.

## Next step

1. Widen the crafting ring search (radius 2.5-3.5 m) for biomedical t2..t4 in order, then electronics, power and materials.
2. Probe the trainer, using multiple y guesses across the wing's floor range, near the art's ring centre, then the two consumables 3-4 m from it.
3. Run each chosen point through `nav_inspect` against both anchors; each set exits 0 with `h` 0.00.
4. Write the SQL, the landmarks (or note `.umap` unavailable), and the rest of the sections; then the commit.

## Last check

`lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect` status=ok exit=0 (build only; the lane logs the run, the probe output comes from the exe).
