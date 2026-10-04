# Repair Settings journey handoff

> **Type:** Reference
> **Audience:** Launcher coordinator and reviewers
> **Date:** 2026-10-04
> **Companions:** [Delegation](../launcher-delegation-plan.md), [requirements](../launcher-implementation-plan.md), [repair contract](../../../../../crates/launcher/desktop/docs/repair.md)

## Revision boundary

- Common base: `0b10d869c869793ab506dbf9215ddb91714a244b`.
- Reserved integration dependency: `622f7fff759ccc49cb314396fad0d5758effe3fb`.
- Owned implementation head: `d00f6bc09ddafc77c8682c64473f8624f9a55222`.
- Branch: `launcher/repair-ui`. This handoff note follows the implementation head;
  it does not change runtime behavior. Integrate the complete chain once.

The reserved commit changes only `crates/launcher/desktop/shell/Cargo.toml`, moving
Tokio into normal dependencies for retained coordination. No lockfile change,
`contract.ts`/`workflows.ts` edit, engine rewrite or new telemetry was needed.

## Delivered behavior and owned files

`crates/launcher/desktop/shell/src/host/repair/{mod,tests}.rs` connects the existing
engine admission, retained preparation-to-commit handoff, progress, cancellation,
recovery, abandonment and current-backup cleanup. `host.rs` and
`host/install/{mod,contract}.rs` register the strict commands/status. Unsupported
platforms stay gated; dispatch failures expose recovery immediately. Active
observation avoids reopening the worker-owned marker on Windows.

`frontend/src/repair-view.ts`, `install-view.ts`, `install-workflow.ts`,
`install-view.test.ts` and `frontend/ui/index.html` connect Settings confirmation,
saved directory identity, duplicate suppression, authoritative observation and
explicit recovery consequences. Lost replies require inspection rather than
mutation replay. Success does not enable Play. `frontend/repair-uat.mjs` exercises
the real Effect/view program through the native persistence fixture.

`docs/repair.md` explains the command boundary and user journey.
`.claude/agent-memory/main-session/reference_launcher_repair_ui.md` records the
ownership and evidence caveats. Coordinator owns README, campaign ledger and
shared index reconciliation. No visible UI was opened by this worker.

## Validation

Run from the project root, unless the command includes a directory explicitly:

```bash
npm ci --prefix crates/launcher/desktop/frontend --no-audit --no-fund
npm run build --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
npm run uat:install --prefix crates/launcher/desktop/frontend
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine repair
```

- Frontend build/typecheck, **38 tests**, existing install logic UAT, formatting
  and `git diff --check`: passed.
- Native shell: **26 passed, 3 ignored**, lane `20261004-103922-95150`.
  Ignored cases require packaged resources or explicit JSON-lines UAT execution.
- Strict shell clippy: passed, lane `20261004-103956-95722`.
- Existing engine repair fault matrix: **39 passed, 3 ignored, 267 filtered**,
  lane `20261004-103712-87051`.

The repair logic UAT used the native shell test executable printed by the lane:

```bash
REPAIR_UAT_BINARY="$PWD/crates/launcher/desktop/target/debug/deps/cimmeria_launcher_desktop-ed41a9f018264e5c" node crates/launcher/desktop/frontend/repair-uat.mjs
```

Use the newly printed test executable path if its Cargo hash changes. The prior
`uat:install` command builds `.test-build/install-view.mjs`, consumed by this pass.
The script invokes only `repair_uat_bridge` with `--ignored --nocapture` and sends
JSON lines; it does not open Tauri or a game window.

Passed: confirmation/dismissal, showing the saved installed directory instead of
the selected preference, one mutation under duplicate clicks, native durable
cancellation, lost reply/reopen without replay, refused unsupported recovery,
explicit pre-checkpoint abandonment and unchanged consent/preferences after reopen.
The fixture holds preparation at admission; its controlled cancellation advances
the durable engine journal. This pass proves persistence/workflow handling, not a
real reconstruction worker or helper lifecycle. Separate engine fixtures cover
replacement and cleanup checkpoints; the frontend cleanup test covers confirmed
command routing and no Play inference.

## Remaining evidence and next action

The coordinator should review this fixed base-to-head diff, integrate serially,
update reserved indices/ledger, run packaged visual/keyboard UAT and own draft PR
publication. This worker made no push, merge, deployment or release.

Real signed-client reconstruction under Wine, native Windows locking/rename and
power-loss behavior, and packaged visual/focus/layout UAT were not performed here.
Unknown helper outcomes remain gated by native evidence. Historical stage/backup
cleanup is outside this assignment. These are release evidence gates, not claims
established by portable ZIP fixtures or the JS persistence harness.
