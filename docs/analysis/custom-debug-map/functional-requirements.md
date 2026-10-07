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
| **NPC gallery: specimen room** | Show NPC archetypes individually in safe display bays. Validate mesh, animation, nameplate, faction, interaction, idle AI and appearance updates without combat or neighboring NPCs confounding the result. |
| **NPC gallery: cover lanes** | Use several short, separated lanes with low and high cover and fixed player/NPC starts. Test acquisition, facing, line of sight, movement to cover, peek/fire behavior and server cover-node selection one arrangement at a time. |
| **Encounter chamber** | Run bounded combat compositions: spawn, aggro, abilities, threat, damage, death, loot, respawn and full reset. Include a spectator/control position outside the fight and prevent hostile AI from reaching the safe hub. |
| **Fabrication lab** | Exercise artifact and machinery interactions, power states, visual/audio effects, animation, console use and failure/recovery chains. Leave a clear path around devices. |
| **Archive / fixture hall** | Provide individually labeled door, container, mission, dialog and inventory/item-use stations. Test open/closed/locked state, ownership, persistence and reset after reconnect. |
| **Transfer terrace** | Test ring or other transport animation, departure/arrival positions, return trip, blocked destination and recovery. The transport is a real server transition, not only a visual prop. |
| **Observation mezzanine** | Prove ramps/stairs, upper-floor navigation, camera clearance, vertical line of sight, overhead markers and safe descent. It also offers a view of test activity without joining an encounter. |

## Why the NPC gallery has separate rooms

A single large NPC room would make several tests ambiguous. Nearby NPCs can enter each other's awareness and threat ranges, share cover, obstruct shots, or join an encounter. Their spawns and effects can also hide a rendering or performance fault. Separate specimen bays let us inspect one archetype and its idle/interaction behavior. Separate cover lanes give every tactic test a known distance, angle, obstruction and reset state. The encounter chamber then tests group behavior deliberately. Doors and sealed walls make the boundaries real for both client collision and server AI; merely drawing floor lines would not isolate these systems.

The gallery rooms should be small enough to reset independently but connected by a safe viewing corridor. A scenario must not leak NPCs, projectiles, threat, loot or witness updates into its neighbor. The same room can host different NPC fixtures over time, but its geometry and named start points stay stable so regressions remain comparable.

## World-wide acceptance

- Floors, ramps, thresholds and platforms are visible, joined and walkable. Indoor walls and ceilings seal the route; outdoor boundaries prevent accidental escape. No spawn, reset or arrival point drops a player into the void.
- Lighting, textures, sky, effects and sound load correctly. Fixtures have aligned collision and interaction bounds, and no prop blocks the intended route.
- Navmesh, cover nodes, line of sight and NPC spawn sets are built from final geometry. The client cannot claim movement, cover, shots or interactions through a wall or outside the test area.
- The map/minimap covers the whole world and aligns player, NPC, objective and fixture markers with actual locations.
- Every scenario has a reset and safe return. A failed transport, encounter or mission state must not strand the player.

The current cooked Worldforge probes demonstrate visual assembly and some walkable Ancient sections. They do not yet prove the behaviors above; each station needs a client walkthrough and a server-side regression test before it counts as functional.
