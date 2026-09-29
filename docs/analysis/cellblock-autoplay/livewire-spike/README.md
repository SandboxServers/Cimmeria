# Livewire spike scripts

> Type: reference. Audience: whoever picks up packets MG-L4 (hit atlas) and MG-L2 (solver) in [work-packets.md](../work-packets.md).
> Updated: 2026-09-29. Companion: [livewire-autosolve.md](../livewire-autosolve.md).

Throwaway Python written during the Livewire auto-solve research. They are committed so the packets can port them, not to be run by CI or imported by anything. Python 3, standard library only.

The scripts read an extracted Scaleform movie. The client is not in git, so extract it from your own copy first: the movie is the one GFX blob inside `CookedPC/UI/Flash/Livewire.upk` (search the package for the `GFX` signature and cut from there to the end of the export; `crates/upk-objects` can list the export).

| Script | Use |
|---|---|
| `as2dis.py <movie.gfx> <out.txt>` | Disassemble every AS2 action block (the Livewire logic and the embedded original server extension) |
| `bounds_core.py`, `bounds.py` | Parse DefineShape records and print bounding boxes; `bounds_core.py` is `exec`'d by `shapes.py` |
| `shapes.py` | Shape parsing, flattening to edges and point hit tests. Needs `GFX=<movie.gfx>` in the environment |
| `probe_static.py` | Named root placements with their matrices: `start_btn`, electrodes `pe*`, `wireCover` |
| `atlas.py` | Rasterise every wire library (`g*`, `o*`, `m*`, `p*`) at a 5-unit step into `livewire_atlas.json` in the working directory |
| `solver_sim.py`, `clearance.py` | Offline simulation of the planned-click solver over random boards, and the per-goal clearance statistics quoted in the design |

Run from this directory:

```bash
GFX=/path/to/Livewire.gfx python atlas.py
python solver_sim.py
```
