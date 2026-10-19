# W2 integration

Merged --no-ff into uo-00 (no conflicts): uo-03, uo-04, uo-05, uo-06, uo-10, then origin/main (5 commits, clean).

Checks (all green, logs C:/Users/Steve/AppData/Local/cimmeria-build/logs/uo-00/): fmt --check; clippy --all-targets -D warnings on cell-catalog, cell-content, cell-world, content-engine; nextest dialog_button_linter + interact_tag_linter (content-engine, 32 pass); live-db union_omega, live_db_debug_registrars, live_db_seed_sequences (21 pass).

No integration fixes needed. Not pushed.
