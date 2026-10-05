---
name: world-map-art-layout
description: How the client lays out world-map art against world coords (MapData.upk WorldMapCollection, chunk-grid tiles, world__default_); Ihpet_Crater_Light's default texture is a 2x top-left crop (stock defect); patch 013 rebuilds it; lab load pitfalls
metadata:
  type: project
---

Verified 2026-10-05 (static upk decode, headless Ghidra, then live lab on the colo test server).

- World map data lives in `CookedPC/Maps/<Map>/<Map>_MapData.upk` (not umap, not server). Exports: `WorldMapCollection "Maps"` (UWorldMapCollection, vfunc_12 @0x008ba470 serializes; element struct 0x48 via FUN_009e8a60), `MapLayerCollection "Layers"`, Texture2D `thumb_WorldMap_<hi16><lo16>` (256x256 DXT1, LZO bulk, one per 100 m chunk) and one 1024x1024 `world__default_`.
- Map element on disk: name "_default_", ints (minLo, minHi, maxLo, maxHi) chunk bounds, layer-name array, floats (uExtent, vExtent) = smaller/larger dimension ratio. Ihpet_Crater_Light: lo -3..7 (11 cols), hi -13..0 (14 rows), floats 0.7857 and 1.0.
- lo = BigWorld x (horizontal), hi = BigWorld z (vertical, +z up). Pad/icon frame fits the tile grid exactly (100 m chunk, 73.14 texels per chunk in the 1024 texture).
- Lua `worldToUnified` = FUN_00ad70b0 -> FUN_00de52a0 (linear, flips x). WorldMapMod.updateWorldPos: UE3 coords /500. Ring icons: RingTransporterMapMode.lua renderDestinationList, server-sent dest.Location.
- The client draws ONLY `world__default_` (getWorldMapLayers() = POI, Mission Waypoints, Player Location, Squad Locations; no tile layer, no zoom). The tiles are never drawn. Confirmed live, stock: the player marker at Z1 sits on bare ground.
- Ihpet's `world__default_` is a 1.99x zoom of the top-left, no translation (blue/brown border at texel row 437 vs 219 true). Same upk for world 73 and 1300: stock defect. Other maps' default textures are patchwork too (Castle, Menfa). Padding colour in stock defaults is magenta (255,0,255).
- Texture2D export layout (ver 486): props, None, then 3 ints, absolute file offset of the mip array, mip count, then per mip: flags 0x10 (LZO), elem count, size on disk, absolute payload offset, payload (0x9E2A83C1 chunk header, block table, LZO blocks), sizeX, sizeY. Mips 8..10 are 4x4 tail blocks.
- Patch 013-ihpet-world-map (tools/client-patches/ihpet_world_map.py) rebuilds it; README section in data/client-patches/README.md.

Lab pitfalls found doing this (colo test server, shared machine):

- The lab watchdog kills the client after 5 consecutive missed heartbeats (MAX_HEARTBEAT_FAILS, crates/lab/src/supervisor/mod.rs) unless another bridge call completed within 5 s. A DebugArea world load blocks the main thread for 40-60 s, and probe calls queue behind it ("dispatch timeout"), so unattended loads die.
- A direct login into a character saved in DebugArea died 6 of 6 times with stock and patched files alike; entering via Castle_CellBlock then `.gotolocation DebugArea` loaded fine with the same polling-heavy pattern. Do not blame a patch for a direct-entry death before running the stock control on the same path.
- Server console `.gotolocation` through lab-server needs the entity to be `in_world` (not `loading`), else "not authorized"; a character saved in DebugArea cannot be taken back to the cellblock, so delete and recreate the lab character for each cellblock entry.
- Kill Ribbons.scr before starting the client (D3DERR_NOTAVAILABLE).
