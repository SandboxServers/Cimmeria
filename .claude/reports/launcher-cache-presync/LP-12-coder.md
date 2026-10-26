# LP-12 coder report

Status: done. Docs only; no tests apply (one comment-only hunk in `crates/server/src/main.rs`).

## Changed
- `docs/engine/cooked-data-pipeline.md`: new "Launcher presync" section (catalog build, routes, reply counter, Play blocked on failure, `cooked_sync` kept but never a fallback, `catalog_failed` for every launcher, kill switch, shared-IP quota F20, alert recipe, `cooked-pak-tool`); category 7 note fixed.
- `docs/engine/cooked-data-pak-format.md`: "Presync catalog formats" (`write_pak`, index JSON, `CPS1`, `CPD1`, integrity not authenticity).
- `docs/tools/admin-api.md`: both routes, both env vars, source-files row.
- `docs/architecture/observability-target-catalog.md`: summary row + `cooked_presync` section (all server rows), outside gen blocks.
- `docs/operations/telemetry.md`: env vars + resyncs avoided vs fired recipe. `container.md`, `colo-deploy.md`, `crates/server/src/main.rs` env tables.

## Doc-update-map rows
58 (admin-api), 36 (observability catalog), 20 (cooked-pak-tool: documented in the pipeline doc; the tools catalog is a gen block, not edited), engine cooked-data rows, ops env rows.

## Flags
- The 429 carries `Retry-After: 60`, over the launcher's 30 s wait cap (D-LP20), so a quota refusal always fails the launcher with `rate_limited`; documented. Consider a shorter server `Retry-After` (F20).
- Pre-existing over-cap files: `cooked-data-pak-format.md` (was 845 lines, now ~920) and `cooked-data-pipeline.md` (~700). Needs a split along the new section seams.
- The `.claude/reports/` dir did not exist; facts come from the merged code.
- `regen.py --check` reports stale gen blocks from other packets (tests totals, tools catalog, crate graph); not touched, not committed.
- Cooked routes are plain HTTP on the login port, not behind the colo https proxy (LP-F2).
- CRLF check clean on all edited docs.