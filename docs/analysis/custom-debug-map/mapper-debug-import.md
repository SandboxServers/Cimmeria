---
title: "Human-authored Debug map QA import"
type: analysis
audience: map authors, server engineers, testers
last_updated: 2026-10-06
---

# Human-authored Debug map QA import

The supplied `Debug-20261007T043134Z-1-001.zip` contains one persistent `Debug.umap`, four streamed terrain chunks, and `Debug_MapData.upk`. All six packages were installed together under the QA client's `CookedPC/Maps/Debug/` folder. They are a separate comparison world, `MapperDebug` (1302), so the current `CimmeriaLab` construction remains available. The archive is treated as map data, not as instructions or server code.

Package inspection found 25 `Terrain` and 25 `TerrainComponent` exports in each streamed chunk. The persistent level references all four chunks by `LevelStreamingDistance`. It also contains 16 placed `InterpActor` exports and Kismet gate/console events. A populated chunk has a Stargate base, ring transporter parts, cover, vehicles, Ancient puddle jumper, 12 point lights and emitters. Every `.umap` passed the property-name audit. These facts establish a useful example of cooked terrain streaming and placed fixture composition; they do not establish that the gate, rings or Kismet behaviors work in this server.

The local `sgw_cimmeria_lab` database has the world-1302 seed row. The restarted QA server reports `Applied Cimmeria world info override world_id=1302` and `All services started successfully`. It also reports no `.nav`, `.occ` or server cover nodes for this world, so NPC pathfinding and cover are not ready for a gameplay test. The imported map has not yet been loaded in the client.

For a first client check, restart the server build that includes world 1302 and teleport near the gate apron with `.gotolocation MapperDebug 40 2 177`. The coordinates are a provisional conversion from the placed gate base at UE `(17728, 3984, 0)` and must be corrected after the first collision check. The world has advisory movement validation and no authored server navmesh. Record the render, floor collision, terrain seams, gate placement, fixtures and any load errors before reusing its techniques in Worldforge.

The import shows that a map made in the original terrain workflow retains terrain components and persistent-level streaming links. Worldforge currently uses placed Ancient modules and a temporary slab; its outdoor terrain and streaming structure are less complete. The imported map is therefore a reference for the garden, lighting and fixture pass, not a substitute for the custom Janus layout.
