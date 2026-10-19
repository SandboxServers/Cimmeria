# Minigame client packages

> Type: reference. Audience: minigame researchers (wave 0B) and port reviewers.
> Written 2026-10-19 by MG-R0 with [tools/minigame-swf](../../../tools/minigame-swf/README.md), from the user's client copy at `..\SGW\Client\game\Working\SGWGame\CookedPC\UI\Flash\`. Companion: [campaign README](README.md#what-the-client-has).

Each of the 11 packages under the client's `UI\Flash\` holds exactly one uncompressed Scaleform movie (`GFX` signature, none `CFX`). Nothing extracted from them is in git (D-MG10). Rerun the table with:

```powershell
python -I tools/minigame-swf/extract_gfx.py <client>\CookedPC\UI\Flash --out <dir-outside-the-repo>
python -I tools/minigame-swf/list_functions.py <dir>\<Name>.gfx
```

## Inventory

Offset and length are the movie's, in bytes, as `extract_gfx.py` reports them; the length is the GFX header's u32 and equals the stored size. The md5 is of the movie bytes (the `.gfx` file), so it identifies the movie whatever the package around it. The function count is the number of lines `list_functions.py` prints: every `DefineFunction` and `DefineFunction2`, named and anonymous, across the root timeline, sprites and `DoInitAction` blocks.

| Package | Package size | GFX offset | GFX length | Movie md5 | SWF version | Functions | Extension present | Shell |
|---|---|---|---|---|---|---|---|---|
| `Activate.upk` | 291,469 | 1,583 | 55,840 | `74520bfa6aed2496e3a9109f71737842` | 9 | 183 | trivial stub only | yes |
| `Alignment.upk` | 841,494 | 4,628 | 293,868 | `f996c4242dec54bfa3cd6689afd7c2e1` | 9 | 410 | yes | no |
| `Analyze.upk` | 291,208 | 1,575 | 55,806 | `8207148dc178533f5a94af97114fa76a` | 9 | 183 | trivial stub only | yes |
| `Bypass.upk` | 291,580 | 1,567 | 55,887 | `846b86e358ef93cfa5246d36ecd59093` | 9 | 183 | trivial stub only | yes |
| `Converse.upk` | 291,393 | 1,583 | 55,795 | `25931bc75ec2d06289057f530b0af6f8` | 9 | 183 | trivial stub only | yes |
| `ConverseBasicHumanoid.upk` | 1,345,189 | 5,649 | 227,957 | `12553e9639b3fae147e3f0944c7f77c5` | 9 | 382 | yes | no |
| `CrystalGame.upk` | 1,814,466 | 7,508 | 94,203 | `9c1930c0818222986678cecaf280d581` | 8 | 233 | yes | no |
| `DHD.upk` | 3,094,137 | 2,916 | 93,455 | `561beb0d9816c1b31c97941383ed5e75` | 9 | 199 | no | n/a (not a minigame) |
| `GoauldCrystals.upk` | 2,356,155 | 1,637 | 154,559 | `81c70b6d28b47455c3023ef0a9010ddb` | 8 | 248 | yes | no |
| `Hack.upk` | 291,135 | 1,551 | 55,760 | `ab34a898ce64a27f9e783142724aa3b6` | 9 | 183 | trivial stub only | yes |
| `Livewire.upk` | 5,935,167 | 11,232 | 410,721 | `7fbe1510a58b120c7776f4b6d78d62f2` | 8 | 391 | yes | no |

The GFX lengths match the architect's throwaway scan in the [campaign README](README.md#what-the-client-has) byte for byte.

## How the two right-hand columns were decided

- **Extension present: yes.** The movie defines the SmartFox server-extension entry point `handleRequest` and its `sendResponse`, plus game rules. Named rule functions found by `list_functions.py`:
  - `Alignment`: `handleRequest`, `checkForVictory`, `getHint`, `doCascade`, `swapPieces`.
  - `ConverseBasicHumanoid`: `handleRequest`, `checkForVictory`.
  - `CrystalGame`: `handleRequest`, `processMove`, `checkForVictory`.
  - `GoauldCrystals`: `handleRequest` only among these names; no `checkForVictory` function is defined (MG-R3 looks for its victory test elsewhere).
  - `Livewire`: `handleRequest`, `processMove`, `checkForVictory`.
- **Trivial stub only, shell: yes.** The five near-identical 55.8 kB movies define the same 183 functions: the stock SmartFoxClient library (`init14`..`init23`), a local server stand-in on the root timeline (`sendResponse`, `getRoom`, `isGame` and the like), a countdown timer (`startTimer`, `timerCallback`, `onTimerUpdate`), `onVictory`/`onFailure`, and an `onRelease` button handler. The constant pool holds `win_btn` and `victory`. No `handleRequest` function is defined (the name only appears as a string), and there is no rules code. MG-R5 owns the full shell verdict.
- **DHD** has neither; it is the dialling UI, not a minigame.

## Caveats

- Most AS1 functions are anonymous (`obj.name = function (...)`). `list_functions.py` prints them as `~<hint>`, where the hint is the last string pushed before the definition. The hint is usually the member name but can be wrong (the SmartFoxClient library shows several `~x(x)`); every rule function named above is a named definition on the root timeline's frame 3, not a hint.
- The function counts include nested and library functions, so they compare movies of the same family, not game complexity.
