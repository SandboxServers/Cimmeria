# Livewire spike scripts

> Type: reference. Audience: whoever picks up packets MG-L4 (hit atlas) and MG-L2 (solver) in [work-packets.md](../work-packets.md).
> Updated: 2026-09-29. Companion: [livewire-autosolve.md](../livewire-autosolve.md).

The shape parser began as a research spike. `atlas.py` is now the lab's private hit-atlas generator. Python 3 and the standard library are required. The generated atlas and extracted movie remain outside Git.

The remaining research scripts read an extracted Scaleform movie. `atlas.py` reads `CookedPC/UI/Flash/Livewire.upk` directly through the existing `tools/upk_parser.py`. The client is not in Git.

| Script | Use |
|---|---|
| `bounds_core.py`, `bounds.py` | Parse DefineShape records and print bounding boxes; `bounds_core.py` is `exec`'d by `shapes.py` |
| `shapes.py` | Shape parsing, flattening to edges and point hit tests. Needs `GFX=<movie.gfx>` in the environment |
| `probe_static.py` | Named root placements with their matrices: `start_btn`, electrodes `pe*`, `wireCover` |
| `atlas.py <Livewire.upk> <output.json>` | Extract the movie from the installed package and rasterise its wire/electrode libraries at a 5-unit step. `lab install` and lab daemon start run this automatically into the private lab cache. |
| `test_atlas.py` | Per-library golden hit-point and shape-bounds checks against a locally installed client; set `CIMMERIA_LIVEWIRE_UPK` to run it. |
| `solver_sim.py`, `clearance.py` | Offline simulation of the planned-click solver over random boards, and the per-goal clearance statistics quoted in the design |

`as2dis.py` moved to [tools/minigame-swf](../../../../tools/minigame-swf/README.md) (MG-R0).

Run from this directory:

```bash
CIMMERIA_LIVEWIRE_UPK=/path/to/Livewire.upk python test_atlas.py
python atlas.py /path/to/Livewire.upk /path/to/private/livewire_atlas.json
python solver_sim.py
```
