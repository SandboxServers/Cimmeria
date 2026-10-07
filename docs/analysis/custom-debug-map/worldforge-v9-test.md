---
title: "Worldforge v9 assembly and client check"
type: testing
audience: map authors, QA testers
last_updated: 2026-10-06
---

# Worldforge v9 assembly and client check

The 66-step `tools/map-lab/assemble_worldforge_probe.ps1` manifest assembles a fresh candidate from the grounded base probe. It retains the nine joined Ancient sections and adds an Ancient ramp to a four-section upper observation loop, an outdoor courtyard boundary, a visual ring transport rig, six garden plants, room props and corner lights, an electric spark in fabrication and a waterfall effect pair in the garden. The script uses independent placed actors from the stock Agnos, Harset and Castle maps; it does not copy a whole stock room tile.

The package build, structural verification and property-name audit passed. The result contains 75 level actors and 228 audited client-loaded exports, with no unloadable property names. Its QA-installed sublevel SHA-256 is `D1D7FD9DC656BC7BD0BD7244149EA1AB269C0B19DB32EE272A7AB6C016DE0C7B`. The prior v6 sublevel was backed up as `%TEMP%/Cimmeria_Lab1-00000000.before-worldforge-v9.20261006.umap`.

## Manual client pass

1. Use `.gotolocation CimmeriaLab 0 2 0` with ghost mode off. Check that the gate court and exterior slabs have visible floor and no seam, flicker or fall-through.
2. Walk the courtyard perimeter and garden. Check plants, ring hardware and waterfall effect, including collision and whether any visual fixture blocks the safe route. Report open perimeter gaps.
3. Walk all nine Ancient sections and the north observation ramp. Cross each upper-floor join and descend without ghost mode. Check ceiling and camera clearance.
4. Inspect archive, fabrication, Index, cover gallery and encounter chamber for prop placement, missing textures, excessive brightness or effects, and blocked walkways.

This is a visual and collision probe. Gate dialing, ring travel, NPC spawns and cover logic, water simulation, server navmesh and minimap are not proven by the package audit. The waterfall is an emitter pair, not yet a finished water feature. The outdoor slab remains temporary geometry; the separately imported human-authored `Debug` map is a terrain-streaming reference for replacing it.
