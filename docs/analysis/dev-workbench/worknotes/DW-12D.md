# DW-12D companion mission X-ray view

> 2026-10-16. Branch `dev-workbench/dw-12d-mission-view`, worktree `dw-12d-mission-view`.

## Implemented

- Added a first-class mission view to the local companion. It persists only the selected mission ID in instance-scoped browser session storage. Switching or clearing the ID cancels pending reads and removes prior evidence immediately.
- Rendered authored definition, ordered steps, objectives, mission-scoped chain references, live server state (unknown until a bound reader exists), persisted server state, decoded packet evidence, client mission state and visible tracker as distinct sections. Every section retains source, sample age and missing-source reason. Displayed ages advance on a one-second timer from the response age plus elapsed local monotonic time, and samples older than five seconds become stale without another read. A late reply for a prior mission is discarded.
- The comparison uses only the API's uniquely bound tracker match and selected-mission server state. Unknown server state leaves active step, objective progress and disagreement unknown. No tracker text populates server progress.
- Extended the read-only mission API projection with objective rows joined to selected mission steps, with an independent gap when the objective query fails. This does not change mission mechanics or write persistence.
- Registered the new script through the cookie-gated static asset route and added an asset-gate regression. The static assets and selected-mission HTTP route live in focused modules, with the mission route test beside its handler, keeping API files under the 500-line soft cap.

## Checks and UAT

- `node --test` in `crates/lab/workbench`: `status=ok`, 32 passed.
- Lane `cargo fmt -p cimmeria-lab`: `status=ok`.
- Lane `cargo clippy -p cimmeria-lab --all-targets -- -D warnings`: `status=ok`. The first attempt failed because this worktree lacked its required `external` junction; restoring the junction fixed the dependency build.
- Lane `cargo test -p cimmeria-lab`: `status=ok`, 1,550 passed.
- JS REPL-style logic UAT: passed. It exercised per-instance mission-ID persistence and clear, mission switch with a late response, missing objective DB layer, tracker/server disagreement while verifying persisted step ID remained server-sourced, authored disabled flags, and a clock-controlled stale transition that updates badge text in place.
- Not covered by the REPL pass: visual browser layout, a live authenticated daemon, a real client tracker, server binding, decoded packets and live mission persistence. No lab lease was taken.

## Packet scope deviation

The packet named only `app.js` for registration, but the workbench needs `index.html` for tab/panel markup and `api.rs`/`mod.rs` for a cookie-gated script route. `api_tests.rs` guards that route. The merged DW-06 API omitted authored objective rows, so `mission.rs` and `mission_tests.rs` add the minimal read-only projection and its separate failure state. These are required to make the specified view reachable and display actual authored objectives.
