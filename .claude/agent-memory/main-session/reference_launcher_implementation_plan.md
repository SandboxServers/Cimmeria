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


- Effect installation controls now call restricted native IPC on supported
  Windows builds; Mac install/resume remains blocked. Pre-mutation inspection,
  no write replay, bounded read retries and polling outside the command semaphore
  preserve native ownership and allow cancel. Uncertain persistence stops polling.
  Recovery inspect/resume is explicit; success means content prepared, not Play.
  Terminal failed/cancelled retry and cleanup remain unavailable. Twenty-three
  frontend tests, TS/build and install DOM/Effect UAT passed; install IPC was
  mocked, so no filesystem/Wine/visual/game proof. Separate real state_bridge
  settings-disk/restart UAT also passed. Native visual/installation IPC UAT remains
  pending; the approved settings preview predates this installation UI.
- Resume CI 37188326146 passed both platforms at bf8029e28; newer shell CI
  37189445603 at 17b949f4c was still running when this evidence was recorded.


- Seed-only extraction seam: install_all_with_seed_extractor accepts a native
  backend and cache separate from fresh content. Shared hash verification precedes
  dispatch; patch overlay/preparation/ledger stay shared. Uncertain extraction
  retains evidence and maps to reconciliation. Existing callers stay unchanged;
  no production backend or Wine runtime is selected/invoked yet. Engine tests181
  and enhanced three-test seed subset passed; strict engine clippy/root fmt passed.


- mac_runtime now prepares a pinned managed cache on macOS: OS lock, bounded
  HTTPS archive, size/hash validation, staged fixed-tar extraction and full-tree
  digest verification on publication/reuse. Corrupt cache is preserved. Blocking
  extraction owns its lock/staging across observer loss; cancel gates publication.
  Explicit pinned-archive smoke passed extraction/tree verification without Wine
  execution. Eight ordinary runtime tests included in 189 passing engine tests; four ignored
  entries, with runtime-archive smoke explicitly passed. Strict engine clippy passed.
  No production caller/prefix/helper or game prerequisites connected.


- ExtractionBackend is immutable intent input: Native default omitted to preserve
  schema-1 digest; Wine binds runtime/helper hashes. Native dispatch/resume/recovery
  refuses Wine, including no-output recovery. Helper journal records attempt,
  intent digest, phase and separately retained PID. run_owned commits launch,
  host-before-request and result-before-return; caller gates journal failures.
  No automatic replay or PID kill/guest-death inference. Production Wine adapter
  and prefix remain unconnected. Tests195 pre-wrapper and five journal tests
  passed; all thirteen real-stdio scenarios, strict clippy and fmt passed.


- Experimental mac_wine seed adapter added (not shell/coordinator-selected):
  artifact identity, Rosetta probe, new private headless prefix, path mapping and
  durable supervisor. Initial ZIP smoke timed out. Minimal diagnostic exposed missing C-drive/system32;
  adding drive_c and dosdevices/c: -> ../drive_c produced successful extraction.
  Original Windows CI helper ZIP smoke then passed in 19.574 seconds; C-drive unit
  regression observed red before fix. Six ordinary adapter tests include concurrent
  ownership; final library suite: 201 passed, five ignored and strict all-target clippy
  passed after mapping fix. No test Wineboot/wineserver processes remained.
  No real RAR/FDI chain or gameplay proof. Mac installation stays disabled.


- Original client RAR/chained-cabinet managed-Wine smoke PASSED 2026-10-04:
  299.64s test, 301.146s lane, Windows debug helper 17b949f4c. Authenticated manifest
  and source size/hash; cabinet progress 5983/5983, MZ SGW.exe, SGWGame directory,
  no .tmp-unpack and durable helper Completed. Private tree auto-cleaned. Exact
  artifact identity and command: desktop/docs/wine-validation.md. No patching,
  prerequisites, launch/login/gameplay or release-performance proof; Mac UI stays
  disabled. Helper-journal CI 37191310279 passed both platforms at ef10c31f9.


- Retained dispatch_wine validates durable backend/runtime/helper identity before
  Running, claims destination and runs Wine seed then native patch/setup/promotion.
  Explicit fixture passed in 22.596 seconds after observer drop; strengthened duplicate and
  pre-runtime cancellation checks passed in 19.090 seconds. Strict engine clippy passed;
  thirteen shell tests passed; final engine suite passed 204 tests with eight
  ignored entries. Frontend 25
  tests/check/build plus sequential failure/consent UAT passed. No visual UAT.
  Mac shell stays disabled pending trusted resource binding; Wine recovery refused.


- Packaged helper binding now conditionally enables Mac content install: fixed
  resource path plus compile-time expected digest, reverified before fetch/admit.
  Stage tool validates trusted hash/revision and AMD64 PE; receipt is not trust.
  Missing helper leaves settings/notes only. Wine recovery capability flags false;
  no native fallback/replay. Dev .app built, not opened. Resolver/admit/cancel
  fixture1.961s passed; frontend 26 tests/build/UAT and stage guards passed;
  final engine205/eight ignored and shell15/one ignored passed, plus strict
  engine/shell clippy and separately run resource smoke. Final bundle/hash passed;
  packaged permission check pending. No visual/final startup proof.


- Durable install-result.json binds outcome to operation ID, intent digest and
  exact terminal revision; written before terminal journal commit. Active/recovery/
  requires-reopen states hide it; old reconciliation revisions cannot reuse it.
  Legacy absence allowed. Shell drop/reopen preserves confirmed failure reason.
  Shell 15/one ignored and engine 209/eight ignored passed; combined strict
  clippy and mock-install JS UAT passed. Windows launcher shared install API
  is now public for lint parity; native Windows verification pending.
  This does not implement Wine recovery or terminal retry.


- Mac Wine reconciliation now supports no-journal/no-spawn or observed-finished
  helper evidence with host absence. Signal0 checks, never kills PIDs; live/reused
  PID and LaunchIntent/HostStarted/Uncertain stay gated. Exact prefix ownership and
  verified cached runtime locks survive bounded -k/-w and content/journal checks.
  No download/delete/resume/shared profiles. ZIP reopen/stop smoke passed in 22.685s;
  final engine212/eight ignored, shell15/one ignored, combined strict clippy and
  mocked-IPC reconciliation/no-resume/no-success-inference JS UAT passed.


- Explicit confirmed failed/cancelled cleanup now binds ID/revision, canonical
  owner lock and allowlisted recursive preflight. Promoted/foreign/symlink/reparse
  content vetoes before deletion. Stage/cache then marker removed; destination
  empty, old terminal persists. Separate retry fresh UUID; another empty folder
  preserves old files. Runtime/prefix retained. Engine 217/shell 16 and frontend 26
  plus confirmation/retry/consent UAT passed; combined strict clippy passed. Windows junction and
  visual checks pending. Actual failed-Wine cleanup passed24.096s after helper
  completion/content rejection; destination empty, retry enabled, consent unchanged. Reconcile/cleanup IPC 35 seconds.


- Fresh revision-zero/no-operation settings save app-local-data/Stargate Worlds once,
  consent off; no game directory/download/install starts. Existing saved/cleared
  settings remain. Complete-size hash-verified seed cache avoids HTTP; confirmed complete hash mismatch
  removes cache before non-Range HTTP; partial cache retained (fixture passed). Real signed
  release RAR+seven-patches+setup+promotion smoke running, outcome pending.


- Original signed-release full-content test passed314.73s: seed+seven patches,
  production claim/shared setup/promotion/content receipt, consentfalse. Overall
  initial lane failed later: missing --lib ran DEV-signature process harness with
  production key. Correct --lib rerun pending; ordinary suites require no prodkey.
  Strict clippy passed. No launch/login/gameplay claim.


- File-cap split: requirements remain launcher-implementation-plan.md; dated
  packet evidence moved intact to launcher-implementation-ledger.md beside it.
  Existing launcher self-update contract moved to docs/client/launcher-self-update.md;
  old launcher heading/anchor remains as a pointer. Campaign and docs indexes link
  both. Continue recording new outcomes in the ledger, not the requirements plan.

- Next prerequisite seam: shared `unpack::unpack` extracts the entire RAR to
  `.tmp-unpack`, expands the installer cabinet set into the game destination,
  then removes staging. The external seed seam also deletes the verified archive
  after successful extraction. Original prerequisite installers therefore need an
  explicit preservation/extraction contract before either cleanup. Issue #1121
  identifies candidates under `Data/Prerequisites`; its package/import inventory
  remains a claim until checked against archive/binary evidence. Do not infer
  redistribution permission or game readiness from content extraction success.
- Before repair/uninstall/launch: `DesktopState::install_intent` currently resolves
  only the current operation when its kind is Install. A subsequent Launch or
  Repair operation must not erase access to installation ownership/version.
  Introduce a durable installed-content reference independent of the active
  operation slot, reconciled against the existing intent/receipt and signed
  release evidence, rather than treating a launch outcome as install state.


- Corrected exact --lib real signed-release content smoke passed312.55s,
  lane315.308s exit0: original seed+all seven patches+content validation+receipt,
  consentfalse. Ordinary engine218/ten ignored, shell17/one ignored and combined
  strict clippy passed. Latest-source Mac development bundle passed with known
  STATIC_VCRUNTIME deprecation; final packaging/startup/gameplay remain unproven.
  The earlier non-lib invocation's fixture-key failure is historical, not current.

- Prerequisite retention seam implemented in shared unpack: optional original
  `Data/Prerequisites` moves beside `Working` as `.cimmeria-prerequisites` before
  staging cleanup. Exact path/type/size/SHA inventory allows identical retry;
  extra/different/link/special/reparse trees are refused, with cancellation checks.
  No probing/execution. Independent verified-seed extraction found 119 files,
  160,028,982 bytes. Staged Windows helper is still old; rebuild/real retention
  smoke pending. Existing completed receipts do not prove retention. Combined
  ordinary checks passed 239/11 ignored and combined strict clippy passed; no
  frontend change or JS UAT required. Enhanced real-release smoke now asserts the
  retained tree and four executable hashes; prior 312.55-second proof predates it.
  Windows junction fixture separator correction still needs native validation.

- Independent installed-content seam is now implemented: schema-1
  `installed-content.json` is published after promotion/receipt and before success,
  including recovery. Read rechecks saved per-ID intent, signed evidence, owned
  root, marker and receipt; missing game files or the entire game directory retain
  Repair identity. Existing game directories must be ordinary/non-reparse; missing
  ownership records or signed evidence still error. Recovery reuses the verified
  owner under the held lock. Legacy current successful Install can migrate without adopting
  selected folders. Publication failure retains recovery. This does not implement
  Repair or grant launch/delete permission; current-operation gates remain required.

  Local engine/shell246/11ignored and strict combined clippy passed; native Windows
  held-lock recovery verification pending. Existing recovery regression now reads
  installed identity after reopen; a new publication failure guard prevents success.
