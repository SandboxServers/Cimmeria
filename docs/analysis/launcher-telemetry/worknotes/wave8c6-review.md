# Wave 8c6 review: LA-18 (commit 9af99e998, worktree la-18, base 54f1fd21e)

## Verdict

decision

(Four small fix notes N1-N4 below are mechanical and can be applied now. The "decision" part is Not fixed 1: the files list shows every non-stock class, which does not match the "only Changed/Missing, never Extra" check in the brief.)

Method: `src`, `ui`, `package.json`, `tsconfig.json` and `adoption-light-uat.mjs` were copied to the scratchpad (node_modules junctioned), `tsc --noEmit` exit 0, `npm test` 223 pass, `npm run uat:adoption-light` passes offline (canned replies, no spawn, no network). Then each new behaviour was reverted one at a time in a fresh copy and `npm test` plus the UAT were run. The worktree was not touched. These are real runs, not reasoning.

Revert results (unit = `npm test`; UAT = `npm run uat:adoption-light`):

| Reverted | Unit tests that fail | UAT |
|---|---|---|
| D1 sentence removed from `lightConsequences` | "a light review says every game file moves into game and stays in use" | passes (does not pin it) |
| `renderIndex(...)` call removed from `adoption-view.ts:230` | **none** | fails |
| not_published / unreachable / invalid text changed (each) | "each index state that did not compare says why, and lists no files" | fails |
| `index_invalid` case returns the unreachable text | same test | fails |
| `??0` default removed | "a compared folder names each count, and a count missing from the list is 0" | fails |
| count order swapped (missing/extra) | same test | passes |
| files list shown for a non-compared state | "each index state that did not compare..." | passes |
| cap line removed / wrong number | "the files list shows each path with its class and says how many are not listed" | passes |
| `hidden=!light` removed | "no light review renders no index line and no files list" | passes |
| summary text changed | the files-list test | passes |
| `index_files.length>0` check removed (empty details shown for a clean folder) | **none** | passes |
| main-view hint removed | "an install with skipped patch changes names them in the main view, and Play stays enabled" | fails |
| hint shown for every install / for `>=0` | "an install with no skipped patch changes reads exactly Ready to Play." | passes |
| `installation_id` check removed from the hint | **none** | passes |

Three reverts pass every unit test (theatre under R-LA 5): the `renderIndex` call (caught by the UAT only), the empty-list guard, the `installation_id` guard. N2-N4 close them.

Other checks, clean:
- Wording vs `light.rs`: the comparison never blocks (`light.rs:104`; `index_compare.rs` returns an `IndexState` for every non-cancel outcome); the four state sentences match the spec byte for byte and are accurate (unreachable also covers a cache write failure, `index_compare.rs:88`, which "could not be downloaded" slightly misdescribes: spec wording, see Not fixed 3). The `Compared` counts come from `IndexComparison.counts` (every class present; the missing-key default is correct). The D1 sentence is accurate: `apply_setup` (`light.rs:315-363`) always rewrites `LoginInternal.lua` and rewrites `SGW.exe` with ASLR off; "address randomisation" is ASLR. It is appended to `lightConsequences` only, so strict, copy and downloaded texts are unchanged (R-LA 10).
- D2 (`adoption-view.ts:114`, `inPlaceMaintenance.abandon`) is untouched, as ruled.
- R8 / contract: no shell change in this commit; the `index_*` schema fields were already decoded by LA-15 (`adoption-workflow.ts:8`). The panel reads them as typed.
- Main-view hint text matches the spec byte for byte; the Play logic in `render` is untouched; `initial()` already carries `patches_incomplete`/`ops_skipped`.
- Files: `adoption-view.ts` 287 lines (was 285), `adoption-view.test.ts` 517 (count unchanged; one line edited), `adoption-index-view.ts` 49, its test 40, UAT 86, `launch-view.test.ts` 142. All CRLF throughout; `git diff --check` empty. The commit holds exactly the eight listed files; no `dist`, `node_modules`, `.test-build` or `launcher_targets.txt`.
- `package.json`: `adoption-index-view.test.ts` is in the `test` list; `uat:adoption-light` builds both `adoption-view` and `launch-view` into `.test-build` (outdir) and the UAT imports them, so it runs standalone.
- UAT: it pins only unchanged LA-16 wording (`Some patch changes can't be applied: 2 files differ`, `did not pick one`, `more than one Stargate Worlds install`, `No folder chosen.`) plus the new index and hint texts. The two-installs scenario asserts the choose button is enabled before the press after it (`choose` asserts `disabled===false`). Each scenario mounting a fresh view: accepted, it removes cross-scenario state. The cap line `…and N more not listed here.` matches the existing skipped-list line (`adoption-view.ts:160`): accepted.
- Coder deviation "mutation checks only reasoned": superseded by the table above. The reasoned claim was wrong for the three rows marked "none".

## Fix notes

N1. crates/launcher/desktop/frontend/package.json:19 — unrelated churn: the `uat:server-list` line lost the space after its colon — change `"uat:server-list":"esbuild src/server-list-view.ts` back to `"uat:server-list": "esbuild src/server-list-view.ts` (keep CRLF) — verify: `git diff 54f1fd21e..HEAD -- crates/launcher/desktop/frontend/package.json` shows only the `test` line and the new `uat:adoption-light` line.

N2. crates/launcher/desktop/frontend/src/adoption-view.test.ts:453 — nothing but the UAT covers the `renderIndex(document,get('adoption-light-skipped'),review.light)` call at `adoption-view.ts:230`; reverting it passes all 223 unit tests — insert after the test that ends at line 453 (the `});` of 'a light review says every game file moves...'):

```
test('a light review shows the index comparison and its files list through the adoption view',async()=>{
 const light={...lightReview(0).light!,index_counts:{changed:1},index_files:[['Working/a.upk','changed']] as [string,'changed'][],index_files_total:1};
 const ui=mount(request=>request==='choose'?{...reviewing('light-6'),backend:'in_place',modes:['light','copy'],review:{...lightReview(0),light}}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.equal(ui.get('adoption-index').textContent,'Compared with the published file list: 0 original, 0 already patched, 0 prepared by the launcher, 1 changed, 0 missing, 0 extra.');
  assert.equal(ui.get('adoption-index-files').hidden,false);
 }finally{await ui.app.dispose();}
});
```

(adjust only the tuple typing if `tsc` objects; the file grows to about 527 lines, under the 700 cap) — verify: `npm run check; npm test` pass, and with `adoption-view.ts:230` deleted in a scratch copy this test fails (`get('adoption-index')` is null).

N3. crates/launcher/desktop/frontend/src/adoption-index-view.test.ts:40 and crates/launcher/desktop/frontend/adoption-light-uat.mjs:~48 — a clean folder (compared, `index_files:[]`) must show no files list; reverting the `light.index_files.length>0` guard at `adoption-index-view.ts:40` passes everything — (a) append to the test file:

```
test('a compared folder with no listed files shows the line and no files list',()=>{
 const {get}=mountIndex(light({index_counts:{stock:40}}));
 assert.equal(get('adoption-index').hidden,false);
 assert.equal(get('adoption-index-files').hidden,true);
});
```

(b) in UAT scenario 1, after the line starting `assert.equal(get('adoption-index').textContent,'Compared with`, add `assert.equal(get('adoption-index-files').hidden,true);` — verify: with `&&light.index_files.length>0` removed in a scratch copy, the new unit test fails.

N4. crates/launcher/desktop/frontend/src/launch-view.test.ts:137 — the `status.installation_id&&` half of the hint condition (`launch-view.ts:19`, required by the spec) is untested; removing it passes everything — add after the 'reads exactly Ready to Play.' test:

```
test('without an installation the skipped-change hint is not shown',()=>{
 const status={...initial(),installation_id:null,patches_incomplete:1,ops_skipped:3} as any;
 assert.equal(launchText({status,pending:false,uncertain:false,error:null}),'Finish installation and compatibility checks to enable Play.');
});
```

— verify: with `status.installation_id&&` removed in a scratch copy this test fails; with it present it passes.

## Not fixed

1. (decision, architect; medium) The files list shows every non-stock class, not only Changed/Missing. `IndexComparison.files` is "every non-`Stock` file" (`crates/launcher-core/src/file_index/compare.rs:38`), `light.rs:247` passes its first 500 by path, and `adoption-index-view.ts:45` renders all of them; the new test pins an `extra` row. The panel spec (`work-packets-light-adoption.md:927`) says "path and class", and the "Never `Extra`" rule (`:157`) is written for the telemetry samples only, so the code matches the spec. Two real consequences: the player's own extra files are named in the panel (local only, no telemetry), and the 500-path cap is sorted by path across Patched/Prepared/Extra/Changed/Missing, so on a folder with many already-patched files the Changed and Missing rows can fall past 500 and show only as "and N more". Decide: keep as is, or filter to Changed/Missing in the engine (`light.rs:247`, with `index_files_total` the filtered count; an LA-15 change) and update the frontend test. A frontend-only filter is wrong: it would miscount "and N more".
2. (decision, trivial) `launch-view.ts:19` reads "1 patch changes could not be applied" when `ops_skipped` is 1; the spec fixes the string. Say if the architect wants a singular form (`adoption-view.ts` already has `plural`).
3. (out of scope, low) `IndexUnreachable` also covers a verified index that could not be cached (`index_compare.rs:88`); the spec sentence says "could not be downloaded". Accurate enough; wording is spec-fixed.
4. (out of scope) The planning copy of `lightConsequences` at `work-packets-light-adoption.md:848,884` lacks the D1 sentence; update it with the close-out docs (LA-19), not in this commit.
