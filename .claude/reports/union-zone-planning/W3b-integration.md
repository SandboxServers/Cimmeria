# W3b integration

Merged origin/main (5228c72a4, #1910) into uo-09, then uo-14 (932dc5f97) with --no-ff. Both merges were clean: no conflicts, no markers.

Checks (all green, logs under C:/Users/Steve/AppData/Local/cimmeria-build/logs/uo-09/):
- cargo fmt --all --check: ok
- clippy --all-targets -D warnings, cimmeria-cell-content + cimmeria-cell-world: ok
- nextest dialog_button_linter + interact_tag_linter (content-engine): 32 passed
- live-db chain_replay_tests::union_omega: 24 passed
- live-db chain_replay_tests::mission_abandoned: 10 passed
- live-db spawner_tests::union_omega: 11 passed
- live-db live_db_union_omega_services: 3 passed
- live-db live_db_seed_sequences: 1 passed

Note: reoffer.rs shows modified in the worktree (line-ending only, empty with --ignore-cr-at-eol); it predates this task and was left untouched.
