# LA-19 report: light-adoption docs

Commit `5acf46eac` on `launcher-telemetry/light-adoption` (worktree `lt-light-integration`, from `54f1fd21e`). Not pushed. Six docs files, CRLF kept, no `<!-- gen: -->` block touched, no IPs or machine paths. The repo markdown lint shows nothing for these files.

## Tested vs never tested (stated in the docs too)

Nothing light-adoption related has had live UAT: no real game folder, no live signed release, no packaged launcher, no SigNoz row from a real run. The docs describe code covered by unit and captured-event tests only. The player guide says so in its own words. `adoption-in-place-uat.mjs` drives the hidden verified in-place mode (`begin_adoption_in_place`), not Light; no script drives Light.

## What changed

- `crates/launcher/desktop/docs/migration.md` (455 to 484 lines of text, under the 500 soft cap): modes are `[light, copy]`; verified in place hidden (D-LA12, `UnsupportedMode`); one-level-down search and `several_game_folders`; `Progress.item = scan`; new "Light mode: what it keeps, what it shows" (every file moves, index comparison informational, `Review.light`, `completed.light`, `LaunchStatus.patches_incomplete` / `ops_skipped`, Repair and full Update rebuild from the seed and leave the player's files only in the old tree, checked against `repair.md`); new "Seedless (incremental) Update: light-adopted installs only" (D-LA14 gate on `check = light`, plan conditions, write set, hard links, Repair as the full rebuild); test and UAT paragraph corrected.
- Stale lines fixed: `migration.md` ~187 ("In place, the folder must be writable" and the "Adopt it in place" mode list), ~342 (the UAT bridge sentence), and `launch.md` ~473 (launch events: `adoption_check`, `patches_incomplete`, `launch_tags_unavailable`). Also the "Before you confirm in place" paragraph now says Light moves added files and the game keeps them.
- `docs/operations/telemetry.md`: new section "Launcher light-adoption and update events" with all nine event rows: the six C-LA rows plus `update_seedless_declined` (`previous_release_unverified`, `ledger_mismatch`), `update_incremental_failed` (`fetch_failed`, `hash_mismatch`, `write_set_unreadable`, `seed_stage_failed`, `rename_failed`) and `launch_tags_unavailable`. It also records the sample rule (D-LA17) and that `mode_chosen` / `folder_chosen` count Light as `in_place` (`shell/src/main.rs`; `events.rs` unchanged).
- `docs/client/launcher-guide.md` (the player guide; the spec's `docs/guides/` grep finds nothing, this is the file): Light mode row, one-level-down pick, the Light review, "What 'can't be applied' means", Repair, added-files wording.
- `crates/launcher/desktop/README.md`: `choose_existing_game_folder` row (`light` / `copy`). Implementation ledger: one dated entry.
- `completed.light` and `scan` are documented in `migration.md` (there is no separate "C16 table" in `crates/launcher/desktop/docs/`; the status contract lives in that file).

## Accepted deviations recorded (not fixed)

1. File sizes over the 500 soft cap: `inventory.rs` 502, `install.rs` ~577, `patchset/src/tests.rs` 510, `adoption-view.test.ts` over 500 (all under the 700 hard cap).
2. The "no recipes in the repo" claim is wrong: the recipes are `data/client-patches/001..016`. I did not find the claim in the docs I edited; correct it wherever the packets or worknotes repeat it.
3. D2 open: `inPlaceMaintenance.abandon` says "Files that are not part of the release were never moved" even after a light abandon; the fix needs a shell field. Written into `migration.md` as an open item.
4. D-LA12 (verified in place hidden) is to be asked of the owner at close-out; the docs state it as the current behaviour, not as a settled decision.
5. Light counts as `in_place` in the shell's telemetry rows (documented above).

## Doc-update-map rows touched

- Desktop launcher workflows, IPC, packaging or release (row for `crates/launcher/desktop/`): desktop README, `migration.md`, `launch.md`, player launcher guide, implementation ledger. `macos.md` unchanged (Light is Windows and Linux; macOS offers the copy only). Self-update and distribution docs unaffected.
- Dev-session / launcher telemetry code row: `docs/operations/telemetry.md`. `dev-session-telemetry.md` and the `main.rs` env-var rows not touched (no secret or endpoint change). Not checked: `observability-target-catalog.md` has no `launcher.game_update` / `launcher.launch` entries, so no row to add.
- Campaign packet scope row: this worknote.
- Not touched, on purpose: project status and gap analysis (this is not the close-out), UAT guide (no UAT happened), `CLAUDE.md`, `.github/copilot-instructions.md`, `docs/readme.md` (no new doc added).

## Documentation debt

- The choose button still reads "Choose the folder to adopt in place..." for Light; the guide describes the button generically.
- The guide's claim that the main view shows the skipped-change count comes from the LA-15 spec; I did not run the launcher to confirm the wording.
