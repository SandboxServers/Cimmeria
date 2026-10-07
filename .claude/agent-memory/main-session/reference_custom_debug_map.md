# Purpose-built Debug Map research (2026-10-06)

- The user's intended map is a **newly constructed level** assembled from existing SGW assets, with custom rooms, outdoor ground, Stargate and cover. A renamed/copied Harset map does not satisfy it.
- Static Ghidra evidence in `docs/analysis/debug-area/custom-map-editor-feasibility.md` confirms New Level, BigWorld chunk save, `BUILDCOVER`, `DEFINEPATHS` and map-thumbnail generator code in QA `SGW.exe`; it does **not** confirm that saved packages load in the game. The first work packet must perform isolated editor-save and clean-client load.
- A full QA `Working` tree is roughly 9.1 GiB, so copying it blindly for editor experiments is costly. Use a verified disposable install/copy before allowing editor saves; the adjacent QA installation is a source, not a scratch target.
- The existing `crates/upk` append-only patcher modifies cooked packages and `tools/SceneEditor` saves JSON; neither is a full new-map cooker. Historical CellBlocks prove distinct client folder/world routing, not newly authored geometry.
- Campaign and acceptance are in `docs/analysis/custom-debug-map/README.md` and `work-packets.md`. CM-00 is the editor-to-game gate; then palette, geometry, world wiring, nav/cover/occ, gate/fixtures, map art and full local UAT.
