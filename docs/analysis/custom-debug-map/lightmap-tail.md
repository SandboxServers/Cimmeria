---
title: "CM-00b: SGC floor component lightmap tail"
type: analysis
audience: engineers, map authors
last_updated: 2026-10-06
---

# CM-00b: cooked floor lightmap tail

The first direct clone of `SGC_Interior.SGC-RoundRoom_Floor00` stopped on
276 post-property bytes in `SGC-00000002.umap` export 1877, the
`StaticMeshComponent` belonging to actor export 1132. This was a correct
failure: three values inside the tail are **source-package export refs**.
Copying them into a new package would point to unrelated objects.

## Byte layout

`upk_info --tail 1877` found the property terminator at component offset 472.
Offsets below are relative to the subsequent 276-byte tail:

| Offset | Size | Value / interpretation |
|---|---:|---|
| `0x00` | 4 | LOD entry count = 1 |
| `0x04` | 4 | shadow-map count = 0 |
| `0x08` | 4 | shadow-vertex-buffer count = 0 |
| `0x0c` | 4 | lightmap type = 2 (`FLightMap2D`) |
| `0x10` | 4 | light GUID count = 12 |
| `0x14` | 192 | 12 × 16-byte GUIDs |
| `0xd4`, `0xe4`, `0xf4` | 4 each | texture refs 233, 234, 235 |
| after each texture ref | 12 each | three float coefficients |
| `0x104` | 16 | four final floats |

The three refs resolve to `LightMapTexture2D` exports 232–234 (0-based),
each with 175,206 serialized bytes. Ghidra `FLightMap2D__vfunc_3` at
`0x008e7810` calls `FUN_00603600` at `0x00603600` for the 16-byte GUID
array, then serializes three object pointers and their three float values,
followed by four floats. The observed tail length is exactly
`20 + 12×16 + 3×16 + 16 = 276`. The existing package-format reference
documents the component LOD wrapper and the known unlit form.

## Guarded direct-route experiment

`upk_patch clone-objects --strip-lightmaps` recognizes **only** the exact
one-LOD/no-shadow/type-2 structure above and checks that all three refs name
`LightMapTexture2D` exports. It replaces the entire native tail with the
known unlit LOD form, three zero words after the count (`1,0,0,0`). Other
baked-lighting shapes still fail closed. This removes the source map's
lighting textures; it does not attempt to clone their bulk data or preserve
baked illumination.

Against the disposable `Cimmeria_Lab1` sublevel, four copies of the SGC
floor actor were placed at UE `(0,0,-128)`, `(2048,0,-128)`, `(0,2048,-128)`
and `(2048,2048,-128)`. The Level actor list grew **2 → 6** (including the
earlier cover actor), adding eight exports total. The output reopens and its
eight new client-loaded objects pass the property-name audit. The hermetic
regression test confirms default cloning rejects the lit tail and the opt-in
path writes the exact unlit tail.

**This is package-level feasibility, not a playable map.** The scratch output
is not installed or wired to a world. Whether these floor meshes render,
retain collision, or receive acceptable dynamic light is untested. The rest
of the level still lacks rooms, outdoor ground, a gate, cover metadata,
navigation, and a verified spawn. The next gate is an isolated client-load
test of a deliberately minimal floor package, then a visibility/collision
check before expanding the layout.
