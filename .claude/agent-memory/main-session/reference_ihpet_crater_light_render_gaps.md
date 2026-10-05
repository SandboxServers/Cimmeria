---
name: reference-ihpet-crater-light-render-gaps
description: "2026-10-05 lab: in this client the Debug Area map (Ihpet_Crater_Light) draws outdoor terrain white, the north palace terrace white with magenta streaks and the terrace east of it as grey void; paving and buildings draw fine. Read before placing anything in world 1300 or 73."
metadata:
  type: reference
---

Seen in the lab client on 2026-10-05 while placing the System Lords' summit (DA-09):

- **Outdoor terrain draws white** across Ihpet_Crater_Light (snow-like, faint magenta caustics near water). Static-mesh paving and buildings draw properly, so place showcase NPCs on paving. The occluder tells them apart: a column top of `LayerKind::Geometry` is paving, `LayerKind::Terrain` is terrain.
- **The north palace terrace** (y ~30.9, around (218, -532) and east to x ~380) draws its floor white with magenta streaks, and the terrace east of it, around (377, -545), is grey void with floating geometry. It is on the navmesh and the occluder calls it open terrain, so a grid search alone picks it. Don't place things there.
- **Good paved spots:** the south compound courtyard (x 214-290, z -908 to -958; the services plaza, the summit at (282, 6.9, -944)), the gate apron (x 232-262, z -974 to -996), and the hex-paved east wing around (332, -965) by the pavilion bonfire.
