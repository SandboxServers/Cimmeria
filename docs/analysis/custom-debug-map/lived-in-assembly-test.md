---
title: "Janus lab: first lived-in assembly test"
type: analysis
audience: map authors, playtesters
last_updated: 2026-10-06
---

# First lived-in assembly test

The QA client currently has a larger **visual and collision probe** of
`CimmeriaLab` installed. Its sublevel was assembled by cloning individual
cooked actors from several Agnos chunks onto the existing technical map
scaffold. It is not yet a newly authored package. The client has loaded it,
and the user has supplied screenshots from a partial walkthrough.

| Area | UE center | Purpose and placed fixtures |
|---|---:|---|
| Gate apron | `(0,0,-128)` | Temporary single SGC slab, visible stock gate/spinners, cover and research terminal |
| Central crossroads | `(0,2220,-128)` | Previously tested walkable Ancient four-way hall; connects the apron to three wings |
| East workshop | `(2048,2220,-128)` | Ancient hall, mainframe, bench and research terminal; an equipment work area |
| West proving bay | `(-2048,2220,-128)` | Ancient hall, two Ancient shield cover meshes and two benches |
| North power/research bay | `(0,4268,-128)` | Ancient hall, power generator and research terminal |

All four halls are copies of the *mesh actor*, placed on a 2048-unit grid.
This demonstrates assembly from client assets, but the repeated four-way kit
and its uncapped exits are temporary. The user should look for floor seams,
overlapping trims, missing materials, and safe walking routes with ghost mode
off. The gate remains visual only. The cover meshes are obstacles; native
cover nodes and NPC use have not been verified.

The exact installed sublevel SHA-256 is
`55D5E32C43C2B614ADFCA0D16EBCD7FAA2C9449776D963F1C0F8D98BA6FA3E61`.
The previous working hall package was backed up under the system temporary
directory as
`Cimmeria_Lab1-00000000.before-lived-in-probe.20261006-223852.umap`.
`upk_patch audit-names` found zero unloadable property names in 118 audited
client-loaded exports.

## Client observations

The user's screenshots show the Ancient hall grid, cyan floor and ceiling
materials, mainframe, power generator and other fixtures rendering in the QA
client. The long corridor and adjoining rooms are visible. The surrounding
world and uncapped hall exits remain black, and the junction to the temporary
round gate apron is visibly unfinished. Screenshots alone do not establish
collision throughout every wing or prove that the cover meshes function as
cover nodes. The user is continuing the walkthrough in control of the client.

A lab-managed client also entered world 1301 at `(0,2,0)`. An automated
`client_move_to` toward the central hall steered off the gate apron and fell;
the tool's coordinate handling is suspect, so this is not evidence of a hall
collision failure. The test character was teleported back before control was
handed to the user.

## Deferred fixtures

`AN-TheChair00`, a drone rack, and a medium shield cover donor stopped at
`DetachedNodeArray`, an array property the patcher cannot safely remap. They
were left out. This is a byte-format/actor-creation task, not evidence those
meshes are unusable. The chair is still wanted for Janus's workspace.

## Playtest

1. Log in through **Local** and use `.gotolocation CimmeriaLab 0 2 0`.
2. With ghost mode off, walk through the central hall into each of the east,
   west and north wings. Check the grid seams and whether fixtures rest on
   the floor without blocking the route.
3. Record screenshots from each wing. Report any fall, collision wall,
   clipping, flicker, or missing mesh/material.

The next construction pass should vary the hall shapes and close unused
openings, then replace the temporary round SGC gate slab with an Ancient
court. Functional Stargate travel, cover nodes, navmesh, minimap and sky
remain separate acceptance gates in [the campaign](work-packets.md).
