# Wave 8c4 review: LA-16 (commit 7ccb84540, worktree la-16, base b33093898)

## Verdict

decision

(N1 to N4 are fix notes the coder can apply; D1 needs an architect call on copy the spec does not give. Ship after N1 to N4; D1 can be a follow-up if the architect accepts it.)

Method: the frontend was copied to the scratchpad (node_modules junctioned), `tsc --noEmit` exit 0, the 49 adoption tests pass, then each new behaviour was reverted one at a time and the suite re-run. The worktree was not touched.

Revert results (what fails when the line is reverted):
- `several_game_folders` branch at `adoption-view.ts:93` removed: the choose-path test fails.
- `entry!=='.'` rule removed: the reasons test and the choose-path test fail.
- `checking` text removed: the first-press test and the updated old first-press test fail.
- `notice='No folder chosen.'` removed: the first-press test (cancel half) fails.
- `review?.light?'Use this folder'` removed: the light-review test fails.
- The `skipped_total===0` hide condition removed: the no-skipped test fails.
- The `folderName(...)` call removed: the light-review test fails.
- The `patches_pending` figure swapped for another: the light-review test fails.
- `${doneText}` removed from the in-place completed text: the updated old test fails.
- **`notice=null;` removed from `run()` (`adoption-view.ts:245`): NO test fails.** This is N1.

Other checks, all clean:
- All five changed files are CRLF and match the base (no churn).
- `package.json` already lists both test files; no new test file was added.
- The commit holds only the five frontend files (no `dist`, `node_modules` or `launcher_targets.txt`).
- `adoption-view.ts` is 280 lines (cap 300 for the split rule).
- No IP, secret or absolute path was added.
- The `.mjs` UAT scripts pin no changed wording: `adoption-in-place-uat.mjs:96` (`/Adopted .* in place.*uninstalling never removes them/`) still matches because the done text is appended; `:53`, `:54`, `:62` and `adoption-native-uat.mjs:63` use unchanged strings. None of them pins "Working on verified adoption" or the old mode-row labels.

## Fix notes

N1. crates/launcher/desktop/frontend/src/adoption-view.test.ts:469 (insert a new test before it, after line 468) — the stale-notice reset in `run()` (`adoption-view.ts:245`, `notice=null;`) has no test. Verified by revert: with it removed all 49 tests still pass, and the real effect is that after a cancelled pick, the next pick that opens a review keeps showing "No folder chosen." as the status line. R-LA 5 requires every new behaviour's test to fail when reverted. — Add this test (CRLF, same terse style):

    test('a cancel notice is cleared by the next press',async()=>{
     let n=0;
     const ui=mount(request=>request==='choose'?(n++===0?idle():{...reviewing('again'),backend:'in_place',modes:['light','copy'],review:lightReview()}):idle());
     try{await ui.app.ready;await ui.settle();
      await ui.click('choose-adoption-destination');assert.equal(ui.text(),'No folder chosen.');
      await ui.click('choose-adoption-destination');assert.notEqual(ui.text(),'No folder chosen.');
     }finally{await ui.app.dispose();}
    });

— verify: `npm test` is green; then delete `notice=null;` from `run()` at `adoption-view.ts:245` and the new test (and only it) fails; restore. (I ran exactly this test in a scratch copy: passes as written, fails with the line removed.)

N2. crates/launcher/desktop/frontend/src/adoption-view.ts:155 — the skipped list is capped at 500 by the engine (`LIGHT_LIST_CAP`) but `skipped_total` can be larger, and the view silently shows 500 with no hint, while the sibling unsafe list (`unsafeItems`, `:64`) says "…and N more not listed here." — replace line 155
`list.replaceChildren(...light.skipped.map(s=>{const item=document.createElement('li');item.textContent=`${s.path} — ${s.patch}`;return item;}));`
with these three lines:
`const rows=light.skipped.map(s=>`${s.path} — ${s.patch}`);`
`if(light.skipped_total>light.skipped.length)rows.push(`…and ${light.skipped_total-light.skipped.length} more not listed here.`);`
`list.replaceChildren(...rows.map(text=>{const item=document.createElement('li');item.textContent=text;return item;}));`
Add a test in `adoption-view.test.ts` using `lightReview(600)` (its `skipped` has 1 entry, so the tail reads "…and 599 more not listed here."): `assert.match(ui.get('adoption-light-skipped').textContent!,/…and 599 more not listed here\./)`. — verify: the new test passes; removing the `if(...)rows.push` line makes it fail.

N3. crates/launcher/desktop/frontend/src/adoption-view.ts:152 and adoption-view.test.ts (the light-review test, regex at about line 431) — with `skipped_total` 1 the note reads "1 files differ", and the test pins that wording. — In `:152` replace `${light.skipped_total} files differ from the originals the patches expect.` with `${light.skipped_total===1?'1 file differs':`${light.skipped_total} files differ`} from the originals the patches expect.` In the light-review test regex replace `1 files differ from the originals the patches expect` with `1 file differs from the originals the patches expect`. In the N2 test (`lightReview(600)`) also assert `/Some patch changes can't be applied: 600 files differ from the originals/` on `adoption-light-skipped-note`. — verify: `npm test` green; reverting the ternary makes the singular assertion fail.

N4. .claude/reports/launcher-telemetry/LA-16-coder.md (in the la-16 worktree, not committed, per R-LA 9) — no coder report exists, and R-LA 5 and R-LA 9 require one that says, per test, which reverted line makes it fail. — Write it from the revert table in the Verdict section above, and state deviations (a), (b), (c) and the (d) grep result (no UAT script pins changed wording; `npm run uat:adoption` and the native scripts were not run). — verify: the file exists and each new test (6 in `adoption-view.test.ts` counting N1/N2, 1 in `adoption-reasons.test.ts`) has a "fails when ..." line.

## Not fixed

D1 (decision, architect). Text accuracy for a light review: the spec's copy is applied as written, but the lines the spec did not touch still describe strict in-place adoption and contradict the light engine. The engine (`desktop/engine/src/storage/adoption/light.rs`, `light_comparison`) puts every scanned file into the moved tree (Matched, KnownTransform or NeedsPatching, never Extra), keeps modified and unlisted files, and never blocks. The view still shows, for a light review or a completed light adoption:
- `adoption-view.ts:118`, `consequences` (in-place branch): "Files you added inside the game folders, such as PAKs placed by hand in SourceCache.en-us, stay where they are and the game will no longer use them." For light, those files move into `game` and stay in use.
- `adoption-view.ts:94`, completed text: "files that are not part of the release were left where they were, and uninstalling never removes them" now sits directly before the new Done sentence. For light it is false (nothing is left outside `game`). The frontend cannot tell a completed light adoption from a strict one (`completed` has no mode field), so the fix needs either a shell field or a spec decision on dropping the old sentence for every in-place completion.
- `adoption-view.ts:117`, `counts`: a modified file is counted as "identical" in a light review, and `needs_patching` is not mentioned.
The spec's LA-16 text names none of these, so a coder cannot choose the wording. Ask the architect for the light `consequences`, `counts` and completed wording, and whether `completed` gets a mode flag (a shell change, or LA-18).

Deviation (a), "Checking your game folder…" set when the press starts: accepted. The native command opens the picker and replies only after the preview, so "on picker return" cannot be observed by the view; setting it on the press is the only way to meet "every button press gets visible feedback on the first press". It also covers the preview wait, which is the long part. One cost: the text shows while the native dialog is still open. The reset paths are correct (cleared after a reply, after a failed reply, and on a cancel); a revert of the reset fails four tests.

Deviation (b), label kept "Make a verified copy" plus the spec's sentence: accepted as a spec slip, not a coder error. The spec's "Make a checked copy" names a row whose existing label is "Make a verified copy" (its existing sentence says "checked copy"); the confirm button ("Create verified copy"), `adoptionText` and the UAT scripts all say "verified copy", so renaming one label would make the page inconsistent. The architect should amend LA-16 line 796 to "Make a verified copy". The appended sentence is the spec's, verbatim.

Deviation (c), the old first-press test now expects "Checking your game folder…": correct and required by the spec (the first press now shows that text).

Deviation (d), UAT scripts not run: not required here (they need `ADOPTION_UAT_BINARY`). The grep found no pinned wording that changed (see the Verdict section).

Test-strength note, not a fix: "Use this folder stays enabled with skipped ops" cannot fail today because no code gates the button on `skipped_total`. It would catch a future gate, so it is kept. The `folderName` backslash path (a Windows source) has no direct test; the revert of `folderName` is caught only through the forward-slash fixture. Optional: add a Windows-path case to the light-review test.

The "Recheck adoption status" press leaves "No folder chosen." showing (the notice is cleared only by the next `run()`). Harmless and not in the spec.