# Wave 8c5 review: LA-16b (commit fd19480cb, worktree la-16, base 6f6f337ca)

## Verdict

decision

(No fix notes: nothing in the commit needs a coder change. The two items under "Not fixed" are wording the spec does not give, so they need an architect call; LA-16b can merge as it stands.)

Method: the frontend `src` and `ui` were copied to the scratchpad (node_modules junctioned), `tsc --noEmit` exit 0, then 54 tests of `adoption-reasons`, `adoption-workflow` and `adoption-view` were run (all pass) and each new behaviour was reverted one at a time in the copy. The worktree was not touched.

Revert results (frontend, actually run):
- `counts:r.light?` branch disabled: only "a light review says every game file moves into game and stays in use" fails.
- `consequences:r.light?lightConsequences:` prefix removed: the same test fails (the consequences assert).
- `if(status.completed?.light)return` line disabled: only "a completed light adoption says the added files moved with it..." fails.
- `light:Schema.optionalKey(Schema.Boolean)` removed from the schema: "a completed light adoption decodes its flag..." and the completed-view test fail (excess property).

Revert results (Rust, by reading, not run: cargo goes through the lane and the brief forbids touching the worktree):
- `in_place_tests.rs` light==true assert: fails if `status.rs:278` is set to `light: false` (the key is absent, `["completed"]["light"]` reads `null`, not `true`).
- `in_place_tests.rs` no-light-key assert: fails if the `skip_serializing_if` line in `contract.rs:260` is removed (`"light": false` is sent, `.get("light")` is `Some`). It cannot pass vacuously: the `completed` object must exist or the next line, `Some(folder)`, fails.

Other checks, all clean:
- Spec match: the six strings (`lightConsequences`, `lightDoneText`, the counts template, the completed line, the schema line, both Rust lines) and the three new frontend tests match the spec byte for byte. `std::ops::Not::not` compiled, so no `is_false` helper was needed. The Rust test literals in `tests.rs`, `lifecycle_tests.rs` and `wine_tests.rs` gain only `light: false`. No other `Completed {` literal exists in `crates/launcher`.
- Wording vs the engine (`desktop/engine/src/storage/adoption/light.rs`): every scanned file goes into the tree as Matched, KnownTransform (stock cooked tier renamed to `SourceCache.en-us`) or NeedsPatching, none is dropped or left outside, modified and unlisted files are kept, the comparison never blocks. "moves into the game folder with no patch changes", "updated by the patches", "bundled data files move to the folder the game loads them from", "files you added ... are moved, not copied" and "Files that differ from what the patches expect are kept as they are" are all accurate. The `light` flag comes from `record.plan.provenance.check`, which `in_place.rs:192` sets to `Light` only for a light preview, so a copy never reads `light: true`.
- R8: `contract.rs` `Completed.light` and the `AdoptionStatus` schema `light` key land in the same commit; `onExcessProperty` is exercised by the workflow test (valid, absent and non-boolean cases).
- R-LA 10: the strict in-place, copy and downloaded texts are unchanged (the diff only adds lines before the strict `completed` branch; the old test at `/Adopted \/old-game in place.*uninstalling never removes them/` still passes). A strict or copy completion serialises exactly as before.
- R5: every new test fails when its change is reverted (above). The coder report states the revert proof "by construction" and did not run a revert; the table above is the run evidence.
- Files: 10 files, no `dist`, `node_modules`, `.test-build` or `launcher_targets.txt` (gitignored or absent from the commit). All ten are CRLF before and after; no LF lines; `git diff --check` is empty; no churn beyond the listed lines. `adoption-view.ts` 285 lines (cap 300 for the split rule), `in_place_tests.rs` 330, `contract.rs` 288, `adoption-view.test.ts` 517 (already 501 at the base; over the 500 soft cap, under the 700 hard cap).
- `in_place_tests.rs` grew by 10 lines, not the spec's "at most +3": rustfmt expanded the asserts. Accepted.
- No `docs/` change, as the spec says (LA-19 documents `completed.light`). No secrets, IPs or absolute paths added.

## Fix notes

None.

## Not fixed

D1 (decision, architect; low). The light `consequences` and `counts` are silent about the launcher's own setup. `light.rs` `apply_setup` always rewrites `LoginInternal.lua` and rewrites `SGW.exe` with ASLR off after the move, so a hand-edited login file or a differently patched executable does not "stay as it is". The strict text says "Files changed only by the old launcher’s setup get the launcher’s own setup again"; the light text has no equivalent, and "Files that differ from what the patches expect are kept as they are" reads as covering these two files. Needs wording from the architect (for example appending "The launcher rewrites its login server file and turns off address randomisation in SGW.exe." to `lightConsequences`). Not a coder fix: the spec fixes the string and the coder matched it.

D2 (decision, architect; low, LA-18 or later). A strict-wording text remains that also appears for a light adoption: `adoption-view.ts:114`, `inPlaceMaintenance.abandon` says "Files that are not part of the release were never moved." For a light adoption that is false (every file moved). The maintenance view cannot tell a light operation from a strict one (`Reconciliation::Copy` carries no mode), so it needs a shell field like `Completed.light` plus wording, or a neutral sentence for every in-place abandon. It is outside the LA-16b ruling, which named three texts. `inPlaceMaintenance.recover` ("already verified release files") is loose but not false.

Test-size note, not a fix: `adoption-view.test.ts` is 517 lines (soft cap 500). Split it along the light tests when LA-18 next edits it; no action in LA-16b.