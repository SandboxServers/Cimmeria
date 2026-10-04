# Desktop Play integration

> **Type:** Reference (bounded worknote)
> **Audience:** Launcher coordinator and reviewers
> **Last updated:** 2026-10-04
> **Companion:** [Launch contract](../../../../../crates/launcher/desktop/docs/launch.md)

Base: `c04bf5a97a8073aa11e67ac602a9c78ec66c74e3`.
Implementation and reserved integration commit SHAs are supplied in the handoff;
this note is committed with the implementation.

The shell now binds fixed resource names to compile-time digests, requires client
patches, admits identity/revision-only Play requests and retains its native worker.
The Effect/DOM control provides immediate feedback, observes persisted lifecycle,
never replays lost mutations and keeps unknown/running attempts blocked. Settings
refreshes on operation revisions and directory changes are disabled while owned.
The repair worker owns install-view suppression/copy for Launch; integrate that
packet alongside this one.

Owned additions: shell `host/launch/`, frontend `launch-workflow.ts`,
`launch-view.ts`, tests and `launch-uat.mjs`; corresponding launch docs and unique
project memory. Wiring touches frontend app/settings view/UI and shell host/main.
Reserved integration files: desktop `Cargo.lock`, `shell/Cargo.toml` and
`frontend/package.json`. The two new Rust dependencies are test-only.

Validation: shell unit/integration tests passed 30, with four pre-existing or
interactive-fixture tests ignored. Frontend tests passed 42. TypeScript check,
frontend build, strict shell all-target Clippy and native-persistence Play JS UAT passed. The UAT covers native
admission/persistence, duplicate clicks, first-click feedback, running gate, early
exit, lost reply without replay, reopen unknown and unchanged consent. Rust also
exercises actual retained dispatch with an inert runtime fixture to persist known
NotStarted and proves identical retries do not restart the worker.

Run from repo root through `tools/build-lane/lane.sh`:

```bash
bash tools/build-lane/lane.sh cargo test --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop --no-default-features --bin cimmeria-launcher-desktop
bash tools/build-lane/lane.sh cargo clippy --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop --no-default-features --all-targets -- -D warnings
```

From `crates/launcher/desktop/frontend`, run `npm run check`, `npm test`,
`npm run build`, and `LAUNCH_UAT_BINARY=<emitted shell test binary> npm run uat:launch`.
Initial shell compilation required building frontend dist first; the fixture also
needed the public installed-content lookup rather than a private engine helper.
Those errors were corrected before the passing checks.

Excluded: real Windows helper/injection, Wine process execution, D3D9/x87 behavior,
login/world entry and packaged visual/keyboard UAT. No visible app, live telemetry,
WireGuard, merge, push, deployment or release was performed. Coordinator owns
artifact staging, native Windows builds and final packaging. No migration/updater
behavior changed.
