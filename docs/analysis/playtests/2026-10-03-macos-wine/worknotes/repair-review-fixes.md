# Repair review fixes

> **Type:** Reference
> **Audience:** Launcher coordinator and reviewers
> **Date:** 2026-10-04
> **Branch:** `launcher/repair-review-fixes`
> **Base:** `d40388a21a23459eaab773a1adc74808029fd747`

## Delivered behavior

The current successful repair reports backup evidence separately from its durable
operation result. Engine observation validates the repair plan, published commit,
backup role and cleanup record. A missing-content repair has no delete control;
cleanup immediately reports removal, including after reopening. Partial cleanup
remains explicitly resumable even when directory removal precedes its last record.
Settings copy follows this evidence and does not infer Play readiness.

Real-host integration guards enter through `install_command`, retaining production
admission, cancellation, progress and the preparation-to-commit coordinator. They
verify signed inert ZIP reconstruction, duplicate suppression, old-tree retention,
pre-commit cancellation, actual after-promotion recovery, cleanup, unchanged
preferences, missing content and persisted status. The shared synthetic fixture
builder replaces duplicated setup; no SGW downloads or Wine runtime are needed.
The old admission-only JS bridge remains labeled as such. A new native JS bridge
and `frontend/repair-native-uat.mjs` exercise the actual host commit/cleanup journey.
The existing recovery `spawn_blocking` join now has a five-second deadline.

The coordinator also requested install-view cooperation with the Play worker:
Launch hides the install primary control and leaves lifecycle copy to launch-view.
Three presentation tests cover running, succeeded and reconciliation-required.
No launch engine or host launch behavior changed in this packet.

## Commit and ownership boundary

- `bbc25bff`: reserved manifests, lockfile and one `cfg(test)` host fixture field.
  The engine `test-support` feature is enabled only as a shell dev-dependency.
- `de733dc2`: repair implementation, guarded portable test adapters, tests,
  frontend copy/UAT and repair documentation. No production OS eligibility gate
  was relaxed. Test adapters call the same preparation/commit/recovery/cleanup
  algorithms; they never manufacture successful journal states for repair.
- This worknote records subsequent verification; integrate the branch once.

No shared index, README, campaign ledger, app/main registration, launch/migration
file or memory frontmatter was changed. Existing repair memory received a bounded
body addendum only.

## Validation

Run from the project root:

```bash
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine repair
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings
npm run build --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
npm run uat:install --prefix crates/launcher/desktop/frontend
```

Use the shell test executable printed by the lane as `REPAIR_UAT_BINARY`, then:

```bash
node crates/launcher/desktop/frontend/repair-native-uat.mjs
```

- Frontend build/typecheck and 41 tests passed. Existing install logic UAT passed.
- Shell: 29 passed, 4 ignored (`20261004-105413-12343`).
- Engine repair: 39 passed, 3 ignored (`20261004-105441-13688`).
- New JS logic UAT passed actual signed ZIP preparation, retained host commit,
  duplicate suppression, explicit cleanup, accurate copy and delete capability
  after cleanup/reopen, and unchanged preferences.

## Evidence limits

The portable native algorithm seam proves host coordination and durable state;
it does not prove Wine/helper supervision or Windows file-lock/rename behavior.
No real SGW content, power-loss experiment, packaged visual/focus/layout UAT,
telemetry, WireGuard, application opening, push, merge, deployment or release was
performed. The coordinator owns those separate release evidence gates.

## Revert verification

Work was committed before each mutation, and only the host repair module was
restored afterward. Removing the successful preparation handoff's commit arm made
`retained_host_handoff_commits_signed_seed_and_reports_current_backup_after_cleanup`
fail at its five-second durable-success deadline (`20261004-105543-15899`).
Restoring the historical success-only cleanup capability made that same guard
fail on `!status.repair.cleanup` after real cleanup (`20261004-105735-18966`).
Both deliberate mutations were restored. An earlier clippy invocation overlapped
the intentional handoff removal and reported the resulting unused coordinator
symbols; the final restored-code strict check below is authoritative.

Final restored-code checks passed: shell 29 passed/4 ignored
(`20261004-105806-19170`), engine repair 39 passed/3 ignored
(`20261004-105810-19286`), strict engine + shell all-target clippy
(`20261004-105836-19149`), formatting and whitespace checks. Both the earlier
admission/persistence JS UAT and the new real-host JS UAT passed against the final
shell executable. The earlier bridge's cancellation remains deliberately
journal-controlled; only the new host tests establish actual worker cancellation.
