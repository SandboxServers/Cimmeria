# Tauri launcher implementation plan — 2026-10-04

The [implementation plan](../../../docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-plan.md)
records the requested Tauri + Effect direction, with Rust authoritative for
install/launch mutation and real Effect workflow coordination. Self-contained
startup checks are deferred until last but remain a release gate. No production
code, deployment, WireGuard connection or live observability probe belongs to
this planning packet.

Inspected contracts: launcher `telemetry/install_result.rs` queues only when
opted in and uploads in the next game telemetry session, leaving failures before
game launch unseen remotely. Server `telemetry/replay.rs` routes ClientNative to
structured `client.native` logs, not native phase spans. Existing game consent
is saved for launch-time snapshots; do not claim immediate DLL revocation.
The plan proposes narrow bounded launcher summaries, independent export consent,
acknowledged queue and validated lifted fields; these are not yet implemented.

2026-10-04 implementation update: `crates/launcher/desktop/engine` now provides
a tested operation contract (nine Mac tests), with an injected journal trait.
It is a separate workspace, requiring explicit manifest checks. No file journal,
Effect integration or mutation worker is connected yet; see the plan ledger.

2026-10-04 foundation update: desktop `storage/` now implements process ownership,
bounded persisted state and uncertain-commit gating. `frontend/` pins Effect
4.0.0; headless UAT uses real Rust `commands.rs` through the `state_bridge`
example and proves preference persistence across restart. Tauri UI, native
app-data selection, migration, game workers and exporter remain unconnected.
Standalone CI is `.github/workflows/launcher-desktop.yml`; root tests omit it.

2026-10-04 shell update: `desktop/shell` now connects native-selected app data
to the approved interface and Effect settings workflow. File-manager reveal
accepts only the saved existing directory, avoiding arbitrary-path IPC and
file-association launch. CI run37181383914 proved engine/JS persistence on
Windows+Mac before the shell; shell/visual/game gates remain separate.

- Windows shell CI requires `shell/icons/icon.ico` even for tests; the initial
  PNG-only shell passed Mac CI but failed Windows tauri-build resource creation
  (run `37182338053`). The existing launcher ICO is now included explicitly.

- Desktop catalog shares the existing manifest source via a Rust path module,
  with its own bounded HTTPS fetcher and notes-only IPC. Live probe authenticated
  seven patches on 2026-10-04 using the handoff's release public key; development
  fallback keys cannot authenticate production content. Notes are available
  release information, not evidence of installation. Native catalog UI UAT open.

- Desktop engine shares install/unpack/client-setup algorithms and patchset
  dependency. ProgressSink::latest retains one observation; egui's adapter keeps
  its old stream. A ZIP fixture covers full pipeline/idempotence. Native Mac
  cannot expand the real spanning cabinets yet; Windows FDI helper is the planned
  route. Legacy successful install is not readiness (SGW.exe may be absent), and
  launcher-installed.json is not deletion ownership. Runtime inventory and open
  redistribution/prerequisite gates are in runtime-provisioning.md.

- Archive helper protocol is bounded NDJSON with hash-before-new-output, UUID
  controls, EOF cancellation and bounded terminal delivery. Windows deny-write/
  delete sharing holds the verified file stable through path-based extraction.
  Parent must keep stdin open/drain stdout and reconcile partial output. Mac
  tests exercise portable mechanics; native Windows process/sharing results and
  Wine/real-CAB UAT remain separate gates. No host invocation is wired yet.

- Native helper supervisor now requires matching terminal identity, process exit
  and EOF. A native-only callback records the host PID before dispatch; durable
  coordinator wiring remains pending. Cancellation writes share the active
  deadline and cleanup cannot renew that budget. Direct-child kill/OS-lock
  release is tested, but does not prove Wine guest death. No UI worker is wired.

- Native install admission binds the exact verified release digest and saved
  destination/server configuration to an operation. Intent records are named by
  operation UUID: a failed subsequent journal commit must not overwrite prior
  retry evidence. Restart never replays work. First-install path checks accept
  absent/empty directories but do not reserve them; mutation ownership and
  readiness remain separate coordinator gates. Restricted install IPC was added
  later in this sequence; frontend controls remain disconnected.

- Native first-install worker dispatches only after durable Running; it owns a
  create-new marker/lock, stages content and promotes to selected-directory/game
  after bounded ledger/executable checks. Receipt/journal uncertainty requires
  reconciliation. Observer disposal does not cancel the detached native task.
  Failures retain partial files; recovery and UI dispatch remain pending.
- Shared install cancellation now interrupts stalled HTTP headers/body and maps
  UnpackError::Cancelled to InstallError::Cancelled. The previous conversion
  incorrectly treated extraction cancellation as generic patch failure. Actual
  partial-ZIP checkpoint and stalled-response regressions cover both fixes.

- Native interrupted-content reconciliation requires exact release identity,
  matching marker ownership/lock and receipt plus current content checks before
  success. Missing/empty output resolves failure; partial output stays gated.
  Decode marker JSON through the locked handle: a second read handle conflicts
  with Windows exclusive file locking. No automatic resume, cleanup, offline
  signed-release cache or Wine guest-lifecycle recovery is implemented yet.

- Install admission now persists original signed release bytes before intent and
  operation commits. cached_install_release verifies the current signing policy
  and exact durable-intent digest without network fallback. This supplies offline
  reconciliation input; key changes can reject old evidence. Cache/orphan cleanup
  and automatic resume remain pending.

- Explicit native resume now requires the current reconciliation ID/revision,
  reverified cached release, locked matching ownership and validated staging.
  It commits Running before Range continuation. Automatic replay and retries
  of terminal cancelled/failed attempts remain unsupported. Fixture coverage
  proves interrupted download recovery, not every extraction/patch checkpoint.
- Desktop CI retains active native runs (cancel-in-progress false), preventing
  milestone pushes from repeatedly cancelling Windows checks. Latest pending
  revision is queued; evidence must still be attributed to its exact commit.


- Restricted shell install IPC now exposes inspect/install/cancel/resume/reconcile
  with native release/settings ownership and shared retained worker state.
  Identical current-operation retries reverify cached evidence without fetching
  the mutable release URL; successful reconciliation clears old worker observations.
  Mac install/resume stays blocked before network until the Wine adapter exists.
  Frontend controls remain disabled/unconnected. Thirteen local shell tests passed;
  native Tauri interaction and this packet's Windows CI remain separate gates.
