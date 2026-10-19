# minigame-swf

> Type: how-to. Audience: minigame researchers, port packet authors and reviewers.
> Added by MG-R0 (2026-10-19). Inventory of the client's movies: [client-packages.md](../../docs/analysis/minigames/client-packages.md).

Tools that pull the Scaleform GFX movies out of the client's `CookedPC\UI\Flash\*.upk` packages and list or disassemble their ActionScript. Every claim a minigame spec makes about a movie can be rerun with them. Python 3 and the standard library are all they need.

| Script | Use |
|---|---|
| `extract_gfx.py <package-dir-or-file> --out <dir>` | Find the `GFX` (or zlib `CFX`) movie in each package and write `<Name>.gfx`, `<Name>.swf` (signature rewritten to `FWS`/`CWS`) and `<Name>.json` (offset, length, md5, version) |
| `list_functions.py <movie>` | One line per `DefineFunction`/`DefineFunction2`: `<timeline> <frame> <name>(<args>)`. The timeline is `root`, `sprite<id>` or `init<id>` |
| `list_functions.py <movie> --strings` | Dump every constant pool |
| `list_functions.py <movie> --function <name>` | Disassemble the body of each function with that name |
| `as2dis.py <movie> <out.txt>` | Disassemble every action block in the movie (moved here from the cellblock-autoplay Livewire spike) |

Anonymous functions, which is most of the AS1 code (`obj.name = function (...)`), print as `~<hint>`. The hint is the last string pushed before the definition, usually the member the function is assigned to, so treat it as a lead and confirm it with `--function`.

## Example

```powershell
$out = "$env:TEMP\sgw-gfx"
python -I tools/minigame-swf/extract_gfx.py ..\SGW\Client\game\Working\SGWGame\CookedPC\UI\Flash --out $out
python -I tools/minigame-swf/list_functions.py $out\Alignment.gfx
python -I tools/minigame-swf/list_functions.py $out\Alignment.gfx --function checkForVictory
```

## Client data is untrusted and stays out of git

- The packages and everything extracted from them are client data (D-MG10). Write output outside the repo. `extract_gfx.py` refuses an `--out` inside the checkout; keep `list_functions.py` and `as2dis.py` output there too.
- Run the scripts with `python -I`, so nothing in the output directory or the current directory is imported.
- Specs quote at most 20 lines of client code per excerpt; no full disassembly goes in git.

## Tests

```powershell
python -m unittest discover -s tools/minigame-swf/tests
```

The tests build synthetic packages in memory and use no client data.
