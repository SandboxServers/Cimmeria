---
title: "Janus Worldforge functional requirements"
type: reference
audience: map authors, server engineers, testers
last_updated: 2026-10-06
---

# Janus Worldforge: functional requirements

The Worldforge is a permanent, custom-built proving ground disguised as one of Janus's research annexes. A tester should arrive through the gate, choose a system from the Index, run a repeatable scenario, reset it, and return without relogging or using ghost mode. Its architecture should make failures easy to reproduce and locate.

## Test pattern

Each station has a clear starting mark, expected result, failure signal and reset control. Fixtures use stable IDs and named locations so an automated test and a human can target the same object. Safe circulation bypasses hostile rooms. Doors, walls, range and sightlines isolate tests; decoration must not change their measured geometry. The Index records which scenario is active and offers a safe return point.

The map must support manual and server-driven tests of movement and collision; gate and ring travel; NPC spawn and behavior; cover and line of sight; combat, death and loot; mission, dialog and interaction chains; inventory and item use; effects, lighting and audio; streaming, minimap and markers; persistence and reset after reconnect. Client visuals must agree with server-authoritative collision, cover, position and interaction ranges.

## Spaces and intended scenarios

| Space | Why it exists and what it must prove |
|---|---|
| **Gate court** | Safe spawn and return. Dial a real gate, traverse it, arrive at the right facing and floor height, and recover from a failed travel attempt. The DHD, event horizon and sound must match the server transition. |
| **Terraforming garden** | Exercise outdoor terrain, slopes, foliage, water, weather/sky, daylight and long sightlines. Include a fixed movement route and safe edge so traversal, projectile visibility and terrain streaming are measurable. |
| **Index / control hub** | Orient the player, select and reset scenarios, expose status at consoles and provide a noncombat route to every station. This is the stable baseline after death, teleport or reconnect. |
| **NPC gallery: lineup halls** | Provide five sealed display halls sized to fit Humans 42, male Jaffa 44, female Jaffa 30, Goa'uld/Asgard/children 28, and creatures/machines 17 (161 total across the halls). These are **NPC capacity targets, not altar or fixture counts**. Use a small number of repeatable display features; spawn positions can be data-driven. Validate mesh, animation, nameplate, faction, interaction, idle AI and appearance updates. Keep a separate empty isolation room for diagnosing one troublesome appearance. |
| **NPC gallery: cover lanes** | Use several short, separated lanes with low and high cover and fixed player/NPC starts. Test acquisition, facing, line of sight, movement to cover, peek/fire behavior and server cover-node selection one arrangement at a time. |
| **Encounter chamber** | Run bounded combat compositions: spawn, aggro, abilities, threat, damage, death, loot, respawn and full reset. Include a spectator/control position outside the fight and prevent hostile AI from reaching the safe hub. |
| **Fabrication lab** | Exercise artifact and machinery interactions, power states, visual/audio effects, animation, console use and failure/recovery chains. Leave a clear path around devices. |
| **Archive / fixture hall** | Provide individually labeled door, container, mission, dialog and inventory/item-use stations. Test open/closed/locked state, ownership, persistence and reset after reconnect. |
| **Transfer terrace** | Serve as the ring-network interchange. Proposed remote pads sit at the garden edge, outside the NPC-gallery lobby, at the lab/archive junction and on the observation level. Test destination selection, animation, departure/arrival positions, return trip, blocked destination, spectator view and recovery. The transport must become a real server transition, not only a visual prop. Ordinary walking routes remain available. |
| **Observation mezzanine** | Provide a useful upper-floor test route: ramps/stairs, upper-floor navigation, camera clearance, vertical line of sight, overhead markers and safe descent. A protected overlook into the fabrication lab and Index lets testers inspect machine effects, fixture states and vertical visibility from above. Its walls must block direct sight into the NPC lineup halls; an elevated view must not bypass their isolation. A proposed ring pad tests upper/lower-floor arrival and return. |

## Why the NPC gallery has separate rooms

A single large NPC room would make several tests ambiguous and could exhaust the client. Ihpet's 161-appearance lineup was split into five exclusive spawn sets after the full lineup ran the 32-bit client out of texture memory. Size the five halls for the populations above, with walkable inspection aisles and extra clearance around large bodies. **Do not hand-place 161 altars.** A few representative display platforms or a reusable room motif are enough; the NPC standing locations belong in spawn data. All groups start inactive and each hall can be activated, cleared and measured independently. One active group is the proven starting limit; Cimmeria optimizations and client patches can raise concurrency after measured client-memory, packet-delivery and frame-time acceptance. Keep the sixth room empty for one-at-a-time diagnosis. The [lineup evidence](../debug-area/README.md) records the group counts and client load behavior.

Build full-height opaque walls, sealed ceilings and offset entrance vestibules so no direct sightline passes between lineup halls, the safe hub, cover lanes or the encounter chamber. Verify that the final client and server occluders agree at doors, corners and the mezzanine. These barriers isolate visibility and combat behavior, but **occlusion alone does not cap client entity load**: nearby active NPCs can still enter the player's area of interest. Independent spawn-set controls provide a measured load ceiling, and the test flow must allow cleared textures to release before rapid switching until client memory behavior is improved and verified. Each room needs its own reset/status control and stable room and spawn-point IDs. A scenario must not leak NPCs, projectiles, threat, loot or witness updates into its neighbor.

## World-wide acceptance

- Floors, ramps, thresholds and platforms are visible, joined and walkable. Indoor walls and ceilings seal the route; outdoor boundaries prevent accidental escape. No spawn, reset or arrival point drops a player into the void.
- Lighting, textures, sky, effects and sound load correctly. Fixtures have aligned collision and interaction bounds, and no prop blocks the intended route.
- Navmesh, cover nodes, line of sight and NPC spawn sets are built from final geometry. The client cannot claim movement, cover, shots or interactions through a wall or outside the test area.
- The map/minimap covers the whole world and aligns player, NPC, objective and fixture markers with actual locations.
- Every scenario has a reset and safe return. A failed transport, encounter or mission state must not strand the player.

The current cooked Worldforge probes demonstrate visual assembly and some walkable Ancient sections. They do not yet prove the behaviors above; each station needs a client walkthrough and a server-side regression test before it counts as functional.
