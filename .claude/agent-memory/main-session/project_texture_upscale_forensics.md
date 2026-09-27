---
name: project-texture-upscale-forensics
description: "External 2x character-texture upscale attempt (2026-09-21): the 512x256, 10-mip DXT1 shape is proven valid by stock cooked data, so a silently broken upscale means bad per-mip data below mip 0; upk-objects' pixel-format enum is off by two (#839)"
metadata:
  type: project
---

On 2026-09-21 an external contributor asked why their 2x upscaled head texture (`Head01_D`) rendered wrong after replacement. Their `replace_texture2d_*` / `export_texture` tools live in their own checkout, not in this repo.

**Established from a stock client:**

- `BS_HF_Torso00_D` is cooked 512x256 DXT1 with 10 mips, MipTailBaseIdx 9, NeverStream and LODGroup 3. That is exactly the shape a 2x head needs, so "the engine needs extra metadata" is ruled out.
- A Texture2D export serialises as: properties, empty SourceArt, NumMips, then per mip a 16-byte bulk header, an LZO payload, SizeX and SizeY. Nothing trails it. The last three mips are stored as 4x4.
- The QA client has a live `check(BulkDataOffsetInFile == Ar.Tell())` (UnBulkData.cpp, line `0x286`), so stale offsets crash rather than render wrongly. A silent failure therefore points at per-mip data or dimensions below mip 0. The contributor's verifier only checked mip 0.
- There are no `Texture2DComposite` instances anywhere in CookedPC.
- Format byte 5 is `PF_DXT1` (hair uses 7, `PF_DXT5`). `crates/upk-objects/src/texture2d.rs` maps these off by two; tracked in #839.

**If this comes back:** first ask for a dump of every mip for a known-good stock texture, a same-size replacement and the 2x replacement, plus a screenshot and the client's Launch.log. A standalone forensic dumper was written for the contributor but lives outside this repo; landing one under `tools/` would be worthwhile.
