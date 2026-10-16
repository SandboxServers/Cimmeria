# DW-12B companion Lua probe view

> 2026-10-16. Branch `dev-workbench/dw-12b-probe`, worktree `dw-12b`.

## Implemented

- Added a first-class Lua probes tab, saved-definition editor, read/mutate badge, JSON parameters, leased run action and bounded global/CEGUI object browser to the local companion window.
- Catalog reload requires explicit discard of dirty edits. Saves use the prior SHA-256 and a changed name creates a new definition. Results clear on selection/edit and show Lua errors, lease refusals, partial/truncated/cyclic trees and gaps as text.
- Registered the new session-gated JavaScript asset in the Rust host and extended its route regression.

## Checks and UAT

- Lane `cargo fmt -p cimmeria-lab`: `status=ok`.
- Lane `cargo clippy -p cimmeria-lab --all-targets -- -D warnings`: `status=ok`.
- Lane `cargo test -p cimmeria-lab`: `status=ok`, 1,531 passed.
- `node --test` in `crates/lab/workbench`: 21 passed, including dirty save/reload, unleased refusal, Lua error, truncated/cyclic rendering and late-run discard.
- JS REPL-style logic UAT: exercised dirty/save/reload, local refusal without a lease ID, Lua error status, and visible cyclic/truncated tree markers; all passed.
- Not covered by the REPL pass: visual browser layout, a live authenticated daemon session, a real client Lua runtime, CEGUI object reads, and lease enforcement by the daemon against a live client. No lab lease was taken; DW-35 remains the live acceptance gate.

## Deviation

The packet named only `app.js` for route registration, but `index.html` and `app.css` needed minimal tab/panel markup and styling for a first-class view. The Rust host only serves explicit static assets, so `api.rs` and `mod.rs` needed the probe script route and `api_tests.rs` needed its gate regression. The coordinator approved this scope extension.
