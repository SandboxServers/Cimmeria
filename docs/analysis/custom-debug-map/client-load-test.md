---
title: "CM-00c: first local client-load test"
type: how-to
audience: engineers, playtesters
last_updated: 2026-10-06
---

# CM-00c: first local client-load test

World `CimmeriaLab` (ID 1301) loads client map `Cimmeria_Lab1`. It is a
temporary, shared test world separate from DebugArea. The server advertises
it in category-12 `CookedWorldInfo`, loads it through the world-to-map table,
and creates a shared cell space. The seed row has advisory navmesh mode while
the map has no `.nav` file.

Install these three files in `<QA Working>/SGWGame/CookedPC/Maps/Cimmeria_Lab1/`:

| File | SHA-256 |
|---|---|
| `Cimmeria_Lab1.umap` | `CD9210B7E44631BE9F59A3DB298096BF5CFB2FDE9E80E27BAE14640DD991EF25` |
| `Cimmeria_Lab1-00000000.umap` | `122E45D5D9FEE5FE2A4BF5D15182D195031AF78FEC69C9CB2CDEADAC6511172C` |
| `Cimmeria_Lab1_MapData.upk` | `7E20D9D30EE0728883DEB3BEB3DB871F0F33467B54D104A90EE79F76117AFA0C` |

The persistent level is a stripped technical scaffold based on the small
Tollana_Curia package. Its Level actor list retains only `WorldInfo`; the
streamed sublevel places one cross-package cover mesh and four separate SGC
floor mesh actors at UE `(0,0,-128)`, `(2048,0,-128)`, `(0,2048,-128)`, and
`(2048,2048,-128)`. Stock exports still exist as unreferenced package data.
The first floor is at game X/Z `(0,0)` using the package-to-server axis
mapping `(game x,y,z) = (UE y,z,x)/100`.

After the updated server and world seed row are active, enter on a GM
character from a known working world:

```text
.gotolocation CimmeriaLab 0 2 0
```

This sends the character above the first floor actor to allow gravity to
settle it. If the package loads, check whether the floor appears, whether
the character lands on it, and whether the cover mesh renders. Exit with
`.gotolocation DebugArea` if possible. Capture the client log and server log
around a failure. A parser reopening the package and a successful teleport
packet do not establish that Unreal streamed the level or that its mesh has
working collision.

## First in-client result, 2026-10-06

The player reached `CimmeriaLab`; the screenshot showed the world name and a
black viewport with the avatar and HUD visible. The player stood, jumped and
landed on an invisible surface, walked for several seconds, then fell off its
edge. This proves that at least one collision surface loads at the test spawn.
It does **not** prove the floor mesh renders or that all four copies have
collision. The boundary and slight drop need a coordinate survey after the
level is visible. The minimap displayed old artwork, as expected for a
renamed scaffold.

The persistent package has a `SkyLight` export (374) and component (375), but
the stripped Level actor list retained only `WorldInfo`. That omission is a
plausible cause of the black level, especially since the new floor components
had their baked lightmaps stripped. An isolated `upk_patch clone-objects`
experiment cloned that existing skylight into the persistent level at UE
`(0,0,200)`. The candidate reopened, grew the actor list 1→2 and passed the
new-object property-name audit. It was **not installed during the live test**:
Windows held the persistent package open while `SGW.exe` was running. The
rendering hypothesis remains unverified until the game exits, the candidate
is installed, and the client is restarted.

After SGW exited on 2026-10-06, the skylight candidate replaced only
`Cimmeria_Lab1.umap` in the QA install. The original persistent package was
backed up locally outside the repository. Installed SHA-256:
`5ED2BA3875789EDBCE4C2759114332C91F75933A783B5DF086105A6DE9E14FB3`.
The map parser sees the original SkyLight export and the newly cloned export
382; the patcher's Level actor list check reported 1→2. **Rendering has not
yet been retested in the client.** Relaunch and use
`.gotolocation CimmeriaLab 0 2 0`; look for a visible floor and cover object,
then test the edge without assuming all four floor copies line up.

## Skylight retest, 2026-10-06

Three in-client screenshots after the skylight install show the SGC floor
material and the cover mesh visibly rendered in `CimmeriaLab`. The avatar
stands on or immediately over the cover at the test arrival point. This
closes the minimal **client load + visible mesh + collision** gate for this
experimental package. The background remains black because the scaffold has
no finished sky or surrounding environment. The floor's curved edges and
painted markings are plainly visible; the player's earlier edge fall is
consistent with walking beyond this finite mesh footprint, but the exact
edge position has not been surveyed.

The screenshots do not prove that every floor copy has collision, that the
floor is continuously walkable between pieces, or that the cover object has
usable cover nodes. The arrival point needs to move off the cover mesh. Next
construction passes should use newly authored continuous ground, an Ancient
asset palette, an outdoor sky/light rig, and a measured safe spawn before
adding rooms and interactions.

The local 2026-10-06 test uses a fresh `sgw_cimmeria_lab` database built from
the current `db/database.sql` because the existing `sgw` database predates
`resources.worlds.navmesh_mode`. The older database was left untouched.
The release server was started against the test database with
`DB_URL=host=127.0.0.1 port=5433 user=postgres dbname=sgw_cimmeria_lab`.
Its startup log reported `world="CimmeriaLab" world_id=1301` and
`All services started successfully`; auth listens on port 8081. A restart
must use the same `DB_URL`, or the default points back to the older database.
The fresh seed includes the documented `test` development account at GM
access level; a character may need to be created before running the command.

```powershell
$env:DB_URL = 'host=127.0.0.1 port=5433 user=postgres dbname=sgw_cimmeria_lab'
.\cimmeria-server.exe
```

This first test **does not** verify rooms, outdoor terrain, Stargate, cover
nodes, navmesh, minimap, or persistence. Those remain campaign packets after
the map-load and floor-collision gate.
