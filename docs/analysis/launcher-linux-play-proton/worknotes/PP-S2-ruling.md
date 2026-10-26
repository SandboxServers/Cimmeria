# PP-S2 ruling (architect, 2026-10-25)

Input: `.claude/reports/launcher-linux-play-proton/PP-S2-findings.md` (the
orchestrator copies it to `worknotes/PP-S2.md`). Checked against the raw
output on the Deck, not taken on trust:

- `~/pp-spike/logs/s2a-s7-probe-excerpt.txt`, relay lines 49835743-49835754:
  `SetDefaultDllDirectories(0xc00)` returns 1, then
  `AddDllDirectory(L"\\?\Z:\…\Working\binaries")` → `LdrAddDllDirectory`
  returns `c000000d` → `AddDllDirectory` returns 0. The `MsiInstallProductW`
  and `CreateFileW` calls before it take the same `\\?\` form and succeed.
- `s2a-run2-s7.out`, `s2c-exp-s7.out`, `s2c-p11-s7.out`: all
  `{"state":"failed","reason":"probe"}`. `s2c-p9-s7.out`: 0 bytes (the hang).
- `SGWGame.b1` holds no `SavedSystemOptions.xml`; `SGWGame.b2/SavedSystemOptions.xml`
  is byte-identical to `s2b-b2-seed.xml` (441 bytes, one CRLF after the
  declaration, no final newline).
- `s2b-b{1,2,3}.out` and `s2c-p11-s{3,4,5}.out`: `process_started` then
  `process_exited` code 0. `s2f2.out`: `process_started` only.
- `s2d.out`: `{"event":"unknown","stage":"inject","win32":5}`; `s2d.err`:
  `import_dll Library XINPUT1_3.dll … not found`, `c0000135`.

Draw / no-draw rows rest on the owner's words and the photos; the X geometry
(1024x688+45+37) is the spike's own reading.

## What PhysX needs under Proton

1. The MSI works as it is. `MsiInstallProductW` returns 0 under GE11, and
   it accepts the `\\?\` path.
2. **The probe fails on one call** (F-24). Rust's `canonicalize` always
   returns the verbatim `\\?\Z:\…` form. Wine's `LdrAddDllDirectory` takes
   only drive-absolute (`X:\…`) and rooted paths, and refuses that form with
   `STATUS_INVALID_PARAMETER`. Windows takes both forms, which is why the
   Windows launcher never hit this. The fix (PP-08W, D-PP28): give
   `AddDllDirectory` the plain `X:\…` form. Nothing else in the probe changes.
3. **Not shown yet**: whether the rest of the probe passes under Proton. That
   is the activation context, the CRT loads, the `PhysXLoader.dll` load by
   its verbatim full path, and `NxCreatePhysicsSDK`/release. It is likely,
   because the game initialises PhysX in the same prefix (PP-S1) and Wine's
   `CreateFileW` and MSI accept the `\\?\` form. The PP-08W Deck check
   proves it.
4. Proton 9.0: the prerequisite worker hangs before msiexec starts (PP-S1
   and PP-S2). PP-08's 180 s deadline turns that into stamp `Failed` and
   `proton_setup_failed`, so no code is needed. FU-12 records it.

## Pass/fail per success criterion

| Criterion (PP-S2 "Rulings the findings must state") | Result | Ruling |
|---|---|---|
| A: name the failing probe call and its path | **Pass.** windows.rs 97-100, `AddDllDirectory` with `\\?\Z:\…\Working\binaries`; the second run is the same | F-24, D-PP28, PP-08W |
| B2: a seeded partial profile draws on a first Play | **Pass.** Only `SavedSystemOptions.xml`, `windowedMode` true, draws with no touch; the game leaves it as it is | D-PP27, PP-17 |
| B1 (premise of B2): the game writes the file itself | **No.** A first run writes `Config/*.ini` and the rest, but no `SavedSystemOptions.xml`, and the window stays black until a touch | F-25 |
| B3: 1024x768 windowed draws | **Pass** at 1024x688+45+37, no touch. The ini size wins over `ResX/ResY` | F-25 |
| C: pass table per Proton | **Partial.** GE-Proton11-11 and Proton 11.0 pass steps 3-5. Experimental and 9.0 are untested (the owner's scope); 9.0's worker hangs | D-PP4 kept, FU-12 |
| D: is native xinput1_3 needed | **Yes**: without it, `c0000135` and no window | C-8 unchanged |
| F: is fullscreen usable | **No.** It never draws, and Alt+Tab leaves touch input dead until `wineserver -k` | D-PP13 final, FU-11 |

## Rulings

| Decision | Ruling |
|---|---|
| D-PP7 | **Acceptance unchanged** (`probed` + `initialized_and_released` only). The cause is found (F-24). PP-08 ships as specified. PP-08W fixes the probe, and its Deck check is the acceptance. |
| D-PP28 (new) | `AddDllDirectory` gets the plain drive form of the canonical game folder: a pure `dll_search_directory` turns `\\?\X:\…` into `X:\…`. Anything else is passed through unchanged: a non-drive verbatim path, MAX_PATH or longer, or a component that Win32 would trim. Every other call keeps the verbatim path. There is **no** detail code on `Failure::Probe`, because the result wire is `deny_unknown_fields` on three hosts. Instead the worker writes the static probe code to stderr, which PP-08 already keeps as `physx.log`. |
| D-PP22 / D-PP27 (new) | First-Play rule: when the prefix profile has no `SavedSystemOptions.xml`, the display step **creates it** from the exact B2 seed, with `valueBool` set from the effective mode, and then edits it on every later Play. It does **not** create `Config/SGWEngine.ini`. A partial UE3 user ini is untested, and the game regenerates a full one at 1024x768. So the first Play runs at 1024x768 (the launcher default) whatever size was chosen, and the chosen size applies from the second Play (FU-10). The seed is refused if any folder between `pfx` and the profile is a symlink. |
| D-PP13 | **Windowed only on Linux for this campaign**, no longer "until PP-S2 step F". Fullscreen under Proton is FU-11. PP-17 step 2 stands. |
| D-PP4 | **Order kept** (GE, Experimental, newest numbered Valve). GE11 and Proton 11.0 pass steps 3-5. Experimental is untested, but it runs the same worker path (its `failed/probe` matches the others). Demoting it would reopen PP-01, which a coder is working on now, without evidence that it fails. |
| D-PP20 / C-8 | **Unchanged.** Both DLLs stay native. Wine's builtin `xinput1_3` is untested, and nothing is gained by dropping a pinned 81 KB file. |
| Arguments | `display::arguments` still passes `-windowed ResX= ResY=` (PP-17 step 3). The client ignores the size when the ini exists, and Linux never takes the fullscreen branch, so F1's "map `ResX=1280`" dialog cannot happen. |

## Amendments made

- `work-packets-w2b.md`: PP-08W fully specced (Ready). The header merge note
  is updated.
- `work-packets-w3.md` PP-17: step 3 filled (seed, `apply_proton`). The file
  list gains `events.rs` line 251 and a new `display_proton_tests.rs`. Tests
  are updated. PP-13 gains the doc points below.
- `README.md`: F-24..F-27; status rows for D-PP4, D-PP7, D-PP13 and D-PP22;
  new D-PP27, D-PP28 and OD-4; the packet table; U3; FU-10..FU-12.
- `RESUME.md`: state and next action.
- **Not changed:** C-8, PP-15, PP-16, PP-08 (its acceptance and 180 s
  deadline already cover F-24 and the 9.0 hang), PP-01 and PP-02 (in
  flight), and PP-03. The worker pin is computed at build time
  (`option_env!("CIMMERIA_PREREQUISITE_HELPER_SHA256")`, set from the CI
  artifact), so PP-08W's new bytes need no pin edit.

## Schedule risk

**Narrowed, not closed.** The cause is known and the fix is about 25
lines. Two things remain open:

- PP-08W's i686 code cannot compile on this host.
- `launcher runtime probe` runs only on `pull_request` and pushes to `main`
  (no `workflow_dispatch`), and `launcher-release` refuses refs other than
  `main`.

So the first build of the fixed worker, and with it the proof that the
rest of the probe passes under Proton, depends on OD-4. Without an answer,
the Deck check moves into PP-U U3 on the final PR's artifact. A failure
would then surface only at wave 4.

## BlockedDecision

- **OD-4**: How does a branch build of the i686 worker get made before the
  final PR?
  - (a) Open the one final PR now as a **draft** from the integration
    branch. Every merge then runs CI and yields `windows-runtime-probe`, and
    D-PP17 still has one PR. **Recommended.**
  - (b) Open a CI-only draft PR for `pp-08w` and close it unmerged.
  - (c) Wait for the final PR and run the Deck check in U3.

  PP-08W is coded and merged either way. Only the timing of its Deck check
  depends on OD-4. The default is (c).
