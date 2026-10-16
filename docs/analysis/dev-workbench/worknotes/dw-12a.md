# DW-12A companion dossier implementation

> 2026-10-16. Branch `dev-workbench/dw-12a-dossier`, worktree `dw-12a`.

## Implemented

- Local companion dossier tab with a per-instance, session-persisted numeric runtime target and refresh/clear controls.
- Read-only dossier presentation from `{schema_version,context,data,gaps}` with source, sample time, age, unknown/stale badges and explicit gaps. A changed or unconfirmed target hides all values.
- Request abort and sequence guard discard an old reply after target change or a newer refresh. Failed reads clear previous values.
- Session-gated static route for `views/dossier.js`, host regression, Node state and dossier tests, and operator guide updates.

## Checks and UAT

- Lane `cargo fmt -p cimmeria-lab`: `status=ok`.
- Lane `cargo clippy -p cimmeria-lab --all-targets -- -D warnings`: `status=ok`.
- Lane `cargo test -p cimmeria-lab`: `status=ok`, 1,521 passed.
- `node --test crates/lab/workbench/app.test.js crates/lab/workbench/tests/*.test.js`: 11 passed.
- JS REPL-style logic UAT: reloaded view/target persistence, stale response and clear transition passed. Fake-DOM render test confirmed an old target's content is removed during a switch; values use text nodes.
- Native visual verification was unavailable: the in-app browser rejected a local-file URL by policy and forbade workaround. No browser layout, authenticated daemon session or real client was exercised. DW-35 remains the live gate.

## Handoff

Review the field labels and stale/sample wording, then rebase onto current main. `crates/lab/src/daemon/workbench/api.rs` and `docs/guides/live-research-lab.md` are likely overlap points with DW-05/DW-06; this packet adds only one static asset constant/function/route and replaces the prior dossier placeholder sentence. Rerun the four checks above after a conflict resolution. No lab lease was taken.