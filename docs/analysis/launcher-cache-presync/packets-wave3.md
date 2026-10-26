# launcher-cache-presync work packets: waves 3 to 5

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26 (re-specced for the owner's D-LP4 reversal: a failed presync blocks Play, with Retry and the exact cause)
> **Companions:** [Ledger](README.md), [work packets: dispatch rules, rules R1-R9, checks, contract, wave 1](work-packets.md), [wave 2](packets-wave2.md), [resume card](RESUME.md)

Every packet here carries rules R1-R9, the checks table and the contract excerpts it names from [work-packets.md](work-packets.md). The orchestrator gives the packet-coder both: this packet's section, and the rules, checks and contract sections there. The ledger's D-LP18..D-LP29 are the reasons; the contract is the spec.

**The one behaviour to protect:** from LP-27 on, a presync that ends `failed` must never start the game, and the player must see which step failed, on what, and why, with a **Retry** button (D-LP4 as reversed, D-LP18, D-LP29). Only `up_to_date`, `synced` and `skipped` continue.

## Wave 3 (into `lcp-w3`; gated on the wave 1 merge and LP-06 pass, which ruling LP-06-1 granted on 2026-10-26)

### LP-20 `launcher-core`: fetcher with failure detail

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-20-fetch`, at `main` · **Worktree:** `lcp-20`
- **Commit:** `feat(launcher-core): fetch the cooked-data presync index and bodies (LP-20)`
- **Rules:** R1-R9. Contract: § Launcher `io_detail.rs` and `fetch.rs`, the `io_kind` codes, the Index JSON, the `CPS1` body, and the routes table.

Files and changes:

1. `crates/launcher-core/Cargo.toml`: add `cimmeria-cooked-pak = { path = "../cooked-pak" }` (a path dep, like `patchset`; F13).
2. `crates/launcher-core/src/lib.rs`: `pub mod cooked_presync;`. New `src/cooked_presync/mod.rs` (`//!` summary of the module and the D-LP18 rule in two sentences; `pub mod fetch; pub mod io_detail;`).
3. New `io_detail.rs`: `io_detail_of` maps `raw_os_error()` 32, 33, 112 first, then `ErrorKind` (`StorageFull`, `PermissionDenied`, `NotFound`, `ConnectionRefused`, `ConnectionReset`, `TimedOut`, `HostUnreachable`, `NetworkUnreachable`), else `other`. `os_code` is `raw_os_error()`. `io_detail_in` walks `source()` and downcasts to `std::io::Error`.
4. New `fetch.rs`, per the Contract:
   - The client: `reqwest::Client::builder()`, `connect_timeout(CONNECT_TIMEOUT)`, `redirect::Policy::none()`, no total timeout. Also add `pub struct Timeouts { pub connect: Duration, pub index: Duration, pub idle: Duration }` with `Timeouts::DEFAULT` from the three constants, and `Fetcher::with_timeouts(login_url, login_servers, Timeouts)`; `new` calls it with `DEFAULT`. Tests and LP-22 use short timeouts through it.
   - Index: the whole request and read inside `tokio::time::timeout(index)`; elapsed gives `Timeout`.
   - Body: the send inside `timeout(idle)`, then each `response.chunk()` inside `timeout(idle)`; elapsed gives `Timeout`. Read streamed and stop at the cap (`TooLarge`).
   - A `reqwest::Error` with `is_timeout()` gives `Timeout`; with `is_connect()` (or any other send error) gives `ConnectFailed(io_detail_in(&error))`.
   - The index is checked: `schema == 1`, unique `category_id`s, each `pak` matching `^[A-Za-z0-9]{1,64}\.pak$` (check by hand, no regex crate), and each `digest` parsing. Anything else is `BadIndex`.
   - `Retry-After` is read as whole seconds; an HTTP date or junk is `None`.
   - Map responses to `FetchError` (the 503 and 429 bodies are `{"error":"<code>"}`; an unreadable body counts as no code):

     | Response | Error |
     |---|---|
     | 404 on the index | `ServerUnsupported` |
     | 404 on a body | `UnknownCategory` |
     | 503 `disabled` | `Disabled` |
     | 503 `catalog_failed` | `CatalogFailed` |
     | 503 otherwise | `CatalogUnavailable { retry_after_s }` |
     | 429 | `RateLimited { retry_after_s }` |
     | 409 | `VersionChanged` |
     | other non-2xx | `HttpError(status)` |

   - A body is decoded with `decode_body(.., &Limits::DEFAULT)`, else `BadBody`; it also has to match the requested id and version, else `BadBody`.
   - `server_label`: `reqwest::Url::parse`, then `host_str()` and `port_or_known_default()` as `host:port`; `None` if either is missing.
5. Lockfiles: the root `Cargo.lock` updates through M2. Update the desktop lock once with `pwsh tools/build-lane/lane.ps1 cargo check --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine` (without `--locked`). Commit both lockfiles.

Tests, in `fetch.rs` and `io_detail.rs` (wiremock is a dev-dep already):

- `index_and_body_decode`.
- `status_codes_map_to_their_errors`: a table over every row above, asserting `reason()`, `http_status()` and `retry_after_s()`. It fails if `catalog_failed` falls into `CatalogUnavailable` (the launcher would wait on a server that will never be ready).
- `a_closed_port_is_connect_failed_with_its_os_error`: bind a `127.0.0.1:0` `TcpListener`, note the port, drop it, fetch; expect `ConnectFailed(Some(IoDetail { kind: "connection_refused", .. }))`.
- `a_silent_body_times_out_on_the_idle_timer`: wiremock `set_delay(500 ms)` with `idle = 100 ms`; expect `Timeout`.
- `an_oversize_index_stops_at_the_cap`.
- `a_bad_pak_name_is_bad_index`: `../x.pak`, `a/b.pak` and `x.exe`.
- `a_body_for_another_version_is_bad_body`.
- `an_endpoint_outside_the_login_servers_is_refused`.
- `server_label_names_host_and_port`: `http://play.example` gives `play.example:80`; `http://127.0.0.1:8081/` gives `127.0.0.1:8081`.
- `io_detail_maps_the_windows_codes`: `io::Error::from_raw_os_error(32)` gives `sharing_violation`, 33 `lock_violation`, 112 `storage_full`.

Each fails if its check is removed. Checks: M1, M2, M3 `cimmeria-launcher-core cooked_presync`; D2e (proves the desktop lock). Docs rows: none (LP-25).

### LP-21 `launcher-core`: stage, commit, sweep

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-21-apply`, at LP-20's reviewed head · **Worktree:** `lcp-21`
- **Commit:** `feat(launcher-core): stage and commit composed cooked-data PAKs whole (LP-21)`
- **Rules:** R1-R9. Contract: § Launcher `apply.rs`, and `read_version`, `decide` and `compose` in § Crate `cimmeria-cooked-pak`.

The rule (D-LP22): no PAK is ever partly written, and no failure before commit changes the cache.

Files and changes:

1. New `crates/launcher-core/src/cooked_presync/apply.rs`; `mod.rs` gains `pub mod apply;`.
   - `plan`: read both versions with `read_version`; an unreadable cache PAK counts as None. Then call `decide`.
   - `stage`:
     1. Refuse a `pak` with a separator or `..` (`UnsafePath`).
     2. `read_pak(source_dir/pak)`: `NotFound` gives `SourceMissing`, other IO errors `SourceUnreadable(io_detail_of(..))`.
     3. `compose`; map `BaseMissing { missing, first }` and `DigestMismatch` through.
     4. `create_dir_all(cache_dir)`, then write `<pak>` + `TEMP_SUFFIX` with `write_pak`, then `sync_all`. An IO error gives `WriteFailed(io_detail_of(..))`, and the temp is removed before returning.
   - `Staged` holds the temp path and a `committed` flag; `impl Drop` removes the temp unless committed, best effort.
   - `commit(staged, cache_dir)`, staged sorted by `category_id`:
     1. Pre-check: for each one whose destination exists, open it with `OpenOptions::new().read(true).write(true)` plus, on Windows, `std::os::windows::fs::OpenOptionsExt::share_mode(0)`, then close it. An error gives `CommitError { committed: 0, .., error: PakInUse(detail) }` when the kind is `sharing_violation`, `lock_violation` or `permission_denied`, else `WriteFailed(detail)`. Nothing is renamed.
     2. Rename each temp over its destination in order (`std::fs::rename` replaces on Windows), and mark it committed. A failure returns `CommitError { committed: <renamed so far>, .. }`; the remaining `Staged` drop and delete their temps.
   - `sweep_temps`: remove files directly in `cache_dir` whose name ends with `TEMP_SUFFIX`; nothing else, no recursion. Return the count.
   - `ensure_not_linked`: refuse when `firesky`, `SGWGame` or `Cache.en-US` is a symlink or junction (`symlink_metadata().file_type().is_symlink()`; on Windows also `MetadataExt::file_attributes() & 0x400`). A missing folder is fine. Don't edit `client_maintenance`.
2. Keep `apply.rs` under 500 lines; tests in `cooked_presync/tests/apply.rs` through `#[path]` if needed.
3. `crates/cooked-pak/src/pak.rs` tests module only (ruling LP-06-1, F21): add `read_version_reads_a_deflated_metadata`. Build a zip with `zip::ZipWriter` holding one `_1` entry and a `MetaData` entry written with `CompressionMethod::Deflated` (the client's rewrite after `up_to_date`), then assert `read_version` gives `Some(v)` and `read_pak(..).version == Some(v)`. No change to `pak.rs` code or to `write_pak` (it keeps `MetaData` stored, the stock layout the client accepted in LP-06).

Tests (temp dirs; PAKs built in memory with `write_pak`; no network):

- `stage_then_commit_gives_the_served_version`: after commit, `read_version(cache/pak) == Some(served)` and `read_pak` equals the composed map; no temp remains.
- `a_digest_mismatch_stages_nothing_and_leaves_the_old_pak`: the old bytes are unchanged and no temp exists.
- `a_dropped_stage_deletes_its_temp`: stage, drop the `Staged`, the temp is gone. It fails if `Drop` is removed.
- `a_locked_second_destination_renames_nothing` (`#[cfg(windows)]`): stage categories 4 and 7; hold 7's destination open with `share_mode(0)`; commit gives `PakInUse` with `committed == 0`, category 7 and `io_kind` `sharing_violation`; category 4's PAK still has its old bytes. It fails if the pre-check is removed (4 would be renamed first).
- `sweep_removes_only_presync_temps`: a `.tmp` with another suffix and a `.pak` survive.
- `a_linked_cache_is_unsafe_path` (`#[cfg(windows)]`, skipped if creating the link is not permitted).
- `a_missing_source_pak_is_source_missing`.
- `a_client_rewritten_cache_pak_is_up_to_date` (ruling LP-06-1): a cache PAK whose `MetaData` is deflated at the served version, a source PAK at an older version; `plan` gives `UpToDate` and stages nothing. It fails if an unreadable or re-encoded cache `MetaData` counts as None (the launcher would then rewrite the PAK on every Play).

The client rewrites `MetaData` after every run (F21), so never compare a cache PAK with a composed PAK by file bytes or size; compare versions, or entries with `read_pak`.

Checks: M1, M2, M3 `cimmeria-launcher-core cooked_presync`; M1, M2, M3 `cimmeria-cooked-pak pak`; D2e. Docs rows: none (LP-25).

### LP-26 `launcher-core`: run, retries, deadline and the failure report

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-26-run`, at LP-21's reviewed head · **Worktree:** `lcp-26`
- **Commit:** `feat(launcher-core): run the cooked-data presync with a closed failure report (LP-26)`
- **Rules:** R1-R9. Contract: § Launcher `run.rs`, the failure table, the non-failure reasons and the `run` order (all of it; this packet implements that list).

Files and changes:

1. New `crates/launcher-core/src/cooked_presync/run.rs`; `mod.rs` gains `pub mod run;`.
   - A private `async fn guarded<T>(fut, deadline: Instant, cancel: &CancellationToken) -> Result<T, Stop>` with `enum Stop { Deadline, Cancelled }`, built on `tokio::select!` over the future, `tokio::time::sleep_until(deadline.into())` and `cancel.cancelled()`. Every network call and every `Retry-After` wait goes through it. `spawn_blocking` stage work also goes through it (a stopped stage task finishes in the background, and its `Staged` drops and deletes the temp).
   - Steps 1 to 6 of the contract's `run` order, in that order. The failure for each exit is built with the step it happened in and the fields the failure table names. `Failure::deadline_s` is the whole run's limit in seconds (`(deadline - started).as_secs()` rounded up), set only for `deadline`.
   - `Report::failed(failure)` and `Report::skipped(reason)` build a report with no categories, so LP-22's early exits share the shape.
   - `categories` lists every index entry once: `UpToDate`, `Written` (after commit) or `Failed` (the one that failed). `bytes_downloaded` sums the body bytes read; `duration_ms` is from the start of `run`.
2. Keep `run.rs` under 500 lines; put the tests in `cooked_presync/tests/run.rs` through `#[path]`.

Tests (wiremock serving bodies built with `encode_body`; temp dirs; short deadlines):

- `a_stale_category_is_synced_and_written`.
- `matching_versions_are_up_to_date_and_write_nothing`.
- `source_newer_fails_at_plan_before_any_download`: wiremock expects 0 body requests; failure `plan`/`source_newer` with both versions.
- `a_500_on_a_body_fails_at_download_naming_the_category`: `http_status = 500`, `category_id`, `pak`; the cache is unchanged.
- `a_digest_mismatch_fails_at_stage_and_changes_nothing`: two stale categories, the second's body has a wrong digest; the first category's PAK is unchanged and no temp exists. It fails if staging and commit are merged back into one write per category.
- `catalog_unavailable_waits_retry_after_then_succeeds`: the first index answer is 503 `Retry-After: 1` (mounted with `up_to_n_times(1)`), the next 200; `attempts` counts it and the outcome is `synced`.
- `a_retry_after_past_the_limit_fails_with_retry_after_s`: 503 `Retry-After: 60`; failure `index`/`catalog_unavailable`, `retry_after_s = 60`, and no wait (the test finishes in well under 60 s).
- `catalog_failed_fails_at_once`: wiremock expects exactly one index request.
- `version_changed_restarts_once_then_fails`.
- `disabled_and_404_are_skipped`: reasons `disabled` and `server_unsupported`; nothing written.
- `the_deadline_names_the_step_in_progress`: a body delayed past a 300 ms deadline; failure `download`/`deadline` with the category and `deadline_s`.
- `cancel_before_commit_leaves_no_temps_and_no_change`: cancel from the progress callback at `Writing`; outcome `cancelled`, the cache unchanged, no temps.
- `progress_reports_totals_then_committing`.

Each fails if its branch is removed; the report names the revert each one guards. Checks: M1, M2, M3 `cimmeria-launcher-core cooked_presync`; D2e. Docs rows: none (LP-25).

### LP-22 Engine: presync module, `ContentSync`, gate and events

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-22-engine`, at LP-26's reviewed head · **Worktree:** `lcp-22`
- **Commit:** `feat(launcher): engine presync step with progress, failure detail and events (LP-22)`
- **Rules:** R1-R9. Contract: § Engine and shell (`ContentSync`, the gate), the failure table's engine rows, and the launcher telemetry rows.

This packet adds the module and its tests only; LP-27 calls it from the worker.

Files and changes:

1. New `crates/launcher/desktop/engine/src/storage/launch/cooked_presync.rs`, declared in `launch/mod.rs` (`pub mod cooked_presync;` so the shell can name `ContentSync`):
   - `ContentSync` and `ContentSyncPhase` (`Serialize`, `Clone`, `Debug`, `PartialEq`; `#[serde(rename_all = "snake_case")]`) per the contract JSON. `outcome` is `Option<Outcome>` and `failure` is `Option<Failure>` from `run.rs`; `reason` is `Option<&'static str>`; `server` is `Option<String>`. No `skip_serializing_if`: every key is always present.
   - `pub(crate) enum Profile { Firesky(PathBuf), Unavailable, NotThisPlatform }` and `pub(crate) fn profile() -> Profile`: under `#[cfg(all(windows, not(any(test, feature = "test-support"))))]` it is `client_paths::firesky_root()` mapped to `Firesky` or `Unavailable`; everywhere else `NotThisPlatform`. No test can reach the real Documents folder (R6).
   - `pub(crate) async fn presync_at(profile: Profile, content_root: &Path, servers: &[LoginServer], timeouts: Timeouts, deadline: Duration, cancel: &CancellationToken, updates: &watch::Sender<Option<ContentSync>>) -> Report`:
     1. `NotThisPlatform` gives `Report::skipped("platform_unsupported")`.
     2. `Unavailable` gives `Report::failed` at `prepare`/`profile_unavailable`.
     3. No first server gives `prepare`/`no_login_server`.
     4. `Fetcher::with_timeouts(first.url, urls, timeouts)` failing gives `prepare`/`endpoint_refused`.
     5. Otherwise `run(..)` with `Instant::now() + deadline`, the source dir `install_layout::sgwgame_dir(content_root).join("SourceCache.en-us")`, and a progress callback that `send_replace`s a `ContentSync` with the phase and counts.

     Before returning, every path sends the final `ContentSync`: phase `done`, the outcome, reason and failure, and `server = server_label(first.url)` (or None). The final send happens even for the early exits.
   - `pub(crate) async fn presync(profile: Profile, content_root: &Path, servers: &[LoginServer], cancel: &CancellationToken, updates: &watch::Sender<Option<ContentSync>>) -> Report` calls `presync_at(profile, .., Timeouts::DEFAULT, run::DEADLINE, ..)`. The worker passes `profile()`; its tests pass a temp `Firesky`.
   - `pub(crate) enum Gate { Continue, Blocked, Cancelled }` and `pub(crate) fn gate(report: &Report) -> Gate` per the contract. `pub(crate) fn timer_outcome(report) -> phase_timing::Outcome`: `up_to_date`/`synced` give `Succeeded`, `skipped` `Unavailable`, `failed` `Failed`, `cancelled` `Cancelled`.
   - Events, target `launcher.launch`, `operation = operation_numbers::number(id)` as in `launch/events.rs` (don't grow `events.rs`): one `cooked_presync_category` per report category, then `cooked_presync_finished` with the contract's 14 keys, `tracing::warn!` when `failed`, else `info!`. Pass `Option` values straight to the macro so an absent value records nothing. No host, path, URL or error text in any field (F19).
2. `phase_timing.rs`: add `PlayCookedPresync` after `PlayTelemetryMint`, code `"play_cooked_presync"`. Update every list of phase codes: re-grep `play_telemetry_mint` (at least `events_tests.rs` l.83 and the `phase_timing` tests).
3. No new dependency: the engine already uses `tokio-util`'s `CancellationToken` (`worker.rs` l.6) and `cimmeria-launcher-core`. If a dev-dep such as `wiremock` is missing from the engine, add it and update the desktop lock as LP-20 did.

Tests, in `cooked_presync.rs` (or a sibling `cooked_presync_tests.rs` through `#[path]`; current_thread runtime; `crate::test_capture::capture`):

- `not_this_platform_is_skipped_and_sends_done`.
- `an_unavailable_profile_fails_at_prepare`: final `ContentSync` has outcome `failed`, `failure.step` `prepare`, reason `profile_unavailable`.
- `no_login_server_fails_at_prepare`.
- `a_500_index_fails_and_the_final_update_names_it`: `Profile::Firesky(temp)`, wiremock 500; the final update has `failure.http_status == 500`, `server == "127.0.0.1:<port>"`, and the gate is `Blocked`.
- `a_silent_server_hits_the_deadline`: a 127.0.0.1 `TcpListener` that accepts and never answers, a 300 ms deadline and long timeouts; reason `deadline`, step `index`.
- `cancel_gives_cancelled_and_the_cancelled_gate`.
- `finished_event_carries_the_failure_codes_and_no_host`: WARN level; `step`, `reason`, `http_status` present; no field value contains `127.0.0.1`.
- `gate_continues_only_on_up_to_date_synced_and_skipped`: a table over the five outcomes. It fails if `failed` ever maps to `Continue`.

Checks: D1, D2e, D3e `cooked_presync`, D3e `phase`, D3e `events`. Docs rows: 57 is owed by LP-25.

### LP-27 Engine: the Play phase and the gate in the worker

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-27-worker`, at LP-22's reviewed head · **Worktree:** `lcp-27`
- **Commit:** `feat(launcher): block Play when the cooked-data presync fails (LP-27)`
- **Rules:** R1-R9. Contract: the gate. Keep the `worker.rs` hunk to about 25 lines (F16: client-tls-oidc also edits this file).

Files and changes:

1. `crates/launcher/desktop/engine/src/storage/launch/worker.rs`:
   - `Worker` gains `pub content_sync: watch::Receiver<Option<ContentSync>>`; `dispatch` (l.73-80) makes `watch::channel(None)` and passes the sender to `execute`, together with `cooked_presync::profile()`.
   - `execute` (l.147) takes `profile: Profile` and `content: &watch::Sender<Option<ContentSync>>`. After `telemetry_session` (l.186) and before `let prepared = plan.clone();` (l.187):

     ```rust
     let presync_timer = context.start(crate::phase_timing::Phase::PlayCookedPresync);
     let report = cooked_presync::presync(profile, &plan.installation.content_root(), &servers, &cancel, content).await;
     presync_timer.finish(cooked_presync::timer_outcome(&report));
     match cooked_presync::gate(&report) {
         cooked_presync::Gate::Continue => {}
         cooked_presync::Gate::Blocked => {
             events::launch_not_started(plan.id, "cooked_presync_failed");
             return Ok(Observation::NotStarted);
         }
         cooked_presync::Gate::Cancelled => return Ok(Observation::Cancelled),
     }
     ```

     The content root comes from `plan.installation`, never from `root` (that is the state root). The presync never returns an error to `execute`, and `Observation` gains nothing (F18).
2. `worker_tests.rs`: pass `Profile::NotThisPlatform` and a sender to the existing `execute` calls. Add:
   - `a_failed_presync_never_reaches_client_setup` (the revert proof for D-LP18): login servers set to a wiremock URL answering 500 on `/api/cooked/v1/index`, `Profile::Firesky(<temp dir>)`. Expect `Ok(Observation::NotStarted)`; a `launch_not_started` event with reason `cooked_presync_failed`; `play_cooked_presync` finished `failed`; **no** `play_client_setup` phase event; the sender's last value has outcome `failed` and `failure.http_status == 500`. It fails if the gate is removed or `Blocked` is treated as `Continue`.
   - `a_skipped_presync_continues_to_client_setup`: `Profile::NotThisPlatform`; `play_cooked_presync` finished `unavailable` before `play_client_setup` starts.
   - `a_cancelled_presync_returns_cancelled`.

   Build the wiremock server on the test's current_thread runtime (`rt.block_on(MockServer::start())`) so the mock outlives `execute`.
3. The shell's `launch_worker` field type is unchanged; the shell reads `content_sync` in LP-23.

Checks: D1, D2e, D3e `worker`, D3e `cooked_presync`, D3e `launch`. Docs rows: 57 is owed by LP-25.

## Wave 4 (into the same `lcp-w3`, after the wave 3 loop; one PR for waves 3 and 4, merged only after LP-06b passes or the ledger records that the user declined the lab)

### LP-23 Shell: `LaunchStatus.content_sync`

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-23-shell`, at the `lcp-w3` head after the wave 3 loop · **Worktree:** `lcp-23`
- **Commit:** `feat(launcher): report content sync progress and failure in the launch status (LP-23)`
- **Rules:** R1-R9. Contract: `LaunchStatus.content_sync` JSON.

Files and changes:

1. `crates/launcher/desktop/shell/src/host/launch/mod.rs`:
   - `LaunchStatus` (l.37-50) gains `content_sync: Option<launch::cooked_presync::ContentSync>`, always serialized (no `skip_serializing_if`, so it is `null`).
   - `assemble` (l.220-240) takes it as a last argument.
   - In `launch_command_timed`, read it once from `self.launch_worker` (`lock`, then `as_ref().and_then(|w| w.content_sync.borrow().clone())`), before the `store.lock()` at l.134 so the two locks are never held together.
2. `host/launch/tests.rs`: `launch_status_serializes_content_sync`: `null` with no value; with a failed value, the JSON has `"outcome":"failed"`, `"server"` and `"failure":{"step":"download",...,"http_status":500,...}` with every failure key present. Update the existing `assemble` calls. It fails if the field is dropped, renamed or skipped when null.

Checks: D1, F0, D2s, D3s `launch`. Docs rows: none (LP-25).

### LP-24 Frontend: schema, progress and failure text, the Retry label

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-24-frontend`, at `main` · **Worktree:** `lcp-24`
- **Commit:** `feat(launcher): show game data sync progress, the exact failure and Retry at Play (LP-24)`
- **Rules:** R1-R9, plus: every button press gets visible feedback on the first press (Play and Retry show "Starting…" at once; keep that).

Files and changes:

1. `frontend/src/launch-workflow.ts`:
   - `Bytes = Schema.Int.check(Schema.isBetween({minimum: 0, maximum: Number.MAX_SAFE_INTEGER}))`.
   - `Failure = Schema.Struct({step: Schema.String, reason: Schema.String, category_id, http_status, os_code, retry_after_s, source_version, served_version, deadline_s: Schema.NullOr(Count), pak, io_kind: Schema.NullOr(Schema.String)})`. `step` and `reason` are plain strings so a newer engine code decodes; the text table has a fallback. `os_code` is `Schema.NullOr(Schema.Int)` (OS codes can be negative).
   - `ContentSync = Schema.Struct({phase: Literal('checking','downloading','writing','committing','done'), categories_done: Count, categories_total: Count, bytes_done: Bytes, bytes_total: Bytes, outcome: NullOr(Literal('up_to_date','synced','skipped','failed','cancelled')), reason: NullOr(String), server: NullOr(String), failure: NullOr(Failure)})`.
   - `LaunchStatus` gains `content_sync: Schema.optionalKey(Schema.NullOr(ContentSync))`, the pattern in `adoption-workflow.ts` l.16-18 (F11).
   - `export function presyncFailed(s: LaunchStatus | null): boolean`: observation phase `not_started` and `content_sync?.outcome === 'failed'`.
2. New `frontend/src/content-sync-text.ts` (keep `launch-view.ts` small): `export function contentSyncText(s: LaunchState): string | null`, and the failure table below. MB is `bytes / 1_000_000` with one decimal place.

   | State | Text |
   |---|---|
   | checking | "Checking game data with {server}…" |
   | downloading, writing | "Updating game data: {done} of {total} categories ({x} of {y} MB)…" |
   | committing | "Saving updated game data…" |
   | done, up_to_date or synced | "Game data is up to date." / "Game data updated ({categories_total} categories)." while the observation is `preparing`, `host_started` or `process_started`; hidden after that |
   | done, skipped `disabled` | "The server turned off the game data check. Character select may take about a minute." |
   | done, skipped `server_unsupported` | "This server doesn't offer game data updates. Character select may take about a minute." |
   | done, skipped `platform_unsupported`, cancelled, null, or the key absent | hidden, empty text |
   | done, failed | the failure text below |

   Failure text: `"Couldn't {action}: {why} {remedy}"` then, on a second line, `"Details: {step} / {reason}"` plus `, category {id}`, `, HTTP {status}`, `, Windows error {os_code}` for the fields that are present. `{action}` by step: prepare "get ready to check game data"; index "get the game data list from {server}"; plan "compare your game data with {server}"; download "download {pak} from {server}"; stage "build the updated {pak}"; commit "save the updated {pak}". `{io}` is the `io_kind` in words (`sharing_violation` "the file is in use", `lock_violation` "the file is locked", `storage_full` "the disk is full", `permission_denied` "access was denied", `not_found` "it was not found", `connection_refused` "the connection was refused", `connection_reset` "the connection was reset", `timed_out` "it timed out", `host_unreachable` / `network_unreachable` "the server can't be reached from this network", else "an error").

   | Reason | Why, then remedy |
   |---|---|
   | `profile_unavailable` | "Windows didn't give the launcher your Documents folder." "Check that Documents exists, then press Retry." |
   | `no_login_server` | "No login server is set." "Add one under Login servers." |
   | `endpoint_refused` | "The login server address isn't one the launcher may download from." "Check it under Login servers." |
   | `unsafe_path` | "Your game data folder is a link to another place, so the launcher won't write into it." "Replace the link with a normal folder or use Reset client cache, then press Retry." |
   | `connect_failed` | "The server didn't accept the connection ({io})." "Check that the server is online and your network works, then press Retry." |
   | `timeout` | "The server stopped answering." "Press Retry." |
   | `catalog_unavailable` | "The server is still preparing its game data." plus " It asked to wait {retry_after_s} s." when present. "Press Retry in a minute." |
   | `catalog_failed` | "The server couldn't prepare its game data." "Tell the server's operator; Retry won't help until they fix it." |
   | `rate_limited` | "The server is limiting requests from your network." "Press Retry in a minute." |
   | `http_error` | "The server answered with an error (HTTP {status})." "Press Retry. If it keeps happening, tell the server's operator." |
   | `too_large`, `bad_index`, `bad_body` | "The server's answer was damaged or not understood." "Press Retry. If it keeps happening, tell the server's operator." |
   | `unknown_category`, `version_changed` | "The server's game data changed during the update." "Press Retry." |
   | `source_newer` | "Your game install has newer {pak} data (version {source_version}) than the server (version {served_version})." "Tell the server's operator, or use Repair if you changed the game files yourself." |
   | `source_missing` | "Your game install is missing {pak}." "Use Repair, then press Retry." |
   | `source_unreadable` | "The launcher couldn't read {pak} from your game install: {io}." "Close Stargate Worlds, then press Retry, or use Repair." |
   | `base_missing`, `digest_mismatch` | "Your game install's {pak} doesn't match what the server's data builds on, so nothing was saved." "Use Repair, then press Retry." |
   | `pak_in_use` | "{pak} is open in another program: {io}." "Close Stargate Worlds and any tool that has the file open, then press Retry." |
   | `write_failed` | "Windows refused to save {pak}: {io}." "Free disk space or check the folder's permissions, then press Retry." |
   | `deadline` | "It took longer than {deadline_s} seconds." "Press Retry; a slow connection may need another try." |
   | anything else | "An unexpected error ({reason})." "Press Retry. If it keeps happening, use Upload debug logs." |

3. `frontend/src/launch-view.ts`:
   - `launchText`: before the existing `not_started` line, `if (presyncFailed(status)) return 'The game was not started because the game data check failed. Press Retry.'`.
   - `render`: the button text is `'Starting…'` while playing or pending, else `'Retry'` when `presyncFailed(current.status)`, else `'Play'`. Render `contentSyncText(current)` into `#launch-content-sync` (`hidden` when null) on every render, also while playing.
4. `ui/index.html`: after `#launch-status` (re-grep; the footer is one long line at l.46), add `<p id="launch-content-sync" class="caption" role="status" aria-live="polite" hidden></p>`. Use `white-space: pre-line` on it (inline style or the existing stylesheet) so the details line breaks. Don't touch `#privacy-identity` here (LP-28).
5. Tests in the existing files (the `test` script lists its files; don't add test files):
   - `launch-workflow.test.ts`: a status with no `content_sync` key, one with `null`, one with progress and one failed all decode; `phase: 'paused'` fails; an unknown `reason` string decodes.
   - `launch-view.test.ts`: `contentSyncText` for every row of both tables (at least one per reason), the details line with and without `http_status` and `os_code`; the button reads `Retry` after a failed presync and `Play` after a plain `not_started`; the element is hidden for `platform_unsupported`.

Checks: F1. Docs rows: none (LP-25; LP-28 does the privacy text).

### LP-28 Frontend: privacy line and the logic UAT

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-28-frontend-uat`, at LP-24's reviewed head · **Worktree:** `lcp-28`
- **Commit:** `test(launcher): content sync privacy line and logic UAT (LP-28)`
- **Rules:** R1-R9, plus the first-press feedback rule. AGENTS.md requires a REPL-style logic UAT for a meaningful frontend change; this is it.

Files and changes:

1. `ui/index.html`, in `<details id="privacy-identity">` (l.80): one `<p>`: "When you press Play, the launcher asks the first login server which game data it serves (`/api/cooked/v1`), over plain HTTP, and downloads only the entries that server changed. It sends no account, install or machine ID; the server sees your connection address and limits requests per address. Game data is written only to Documents\My Games\FireSky\SGWGame\Cache.en-US. If the check fails, the game is not started and the launcher shows why."
2. `privacy-disclosure.test.ts`: `#privacy-identity` contains `/api/cooked/v1`, `Cache.en-US` and "not started".
3. New `frontend/launch-content-sync-uat.mjs` and the `package.json` script `"uat:launch-content-sync": "esbuild src/launch-view.ts --bundle --platform=node --format=esm --packages=external --outfile=.test-build/launch-view.mjs && node launch-content-sync-uat.mjs"`. Mount `mountLaunch` on `ui/index.html` with `linkedom`, as `launch-uat.mjs` does, with a scripted `invoke` (no native bridge). Steps, each asserting the button text, its `disabled` state and both status lines:
   1. Press Play once: "Starting…" shows on the first press.
   2. Step the polled status through checking, downloading 1/3, writing 3/3, committing, done/synced with observation `preparing`.
   3. A second Play: done/failed at `download`, `http_error`, HTTP 500, observation `not_started`. The button reads **Retry** and is enabled; the text names the PAK, the server and HTTP 500, and the game status says it was not started.
   4. Press Retry once: "Starting…" at once; the scripted `invoke` records a fresh `operation_id`; the run ends done/up_to_date.
   5. done/failed `pak_in_use` with `io_kind` `sharing_violation` and `os_code` 32: the text says the file is in use and to close Stargate Worlds.
   6. done/skipped `disabled`: the note shows and the button reads Play.
   7. done/skipped `platform_unsupported`: the element is hidden.

   Print `Content sync logic UAT passed: ...` and an `Excluded:` line (the real network, the real PAKs, the game, a launcher restart).

Checks: F1, F2. Docs rows: none (LP-25).

### LP-25 Launcher docs

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `launcher-cache-presync/LP-25-launcher-docs`, at the `lcp-w3` head after LP-27 and LP-24 · **Worktree:** `lcp-25`
- **Commit:** `docs(launcher): cooked-data presync at Play blocks on failure, with Retry (LP-25)`
- **Rules:** R1, R3, R4, R5, R7, R8.

Files (doc-update map row 57):

1. `crates/launcher/desktop/docs/launch.md`:
   - the new phase `play_cooked_presync` between the telemetry mint and client setup; its outcomes and the gate (D-LP18);
   - a section "Play doesn't start the game when the presync fails" (D-LP7): the failure steps and reasons (link the contract table, don't copy it), the Retry label, the skip cases (D-LP19, D-LP9), the two-phase write (D-LP22), the time limits (D-LP25), cancel (D-LP23), and that the detail lives in memory only (D-LP26: after a launcher restart the generic `not_started` text shows);
   - amend "Play downloads nothing" (D-LP7), and add `cooked_presync_failed` where `not_started` reasons are listed (l.138).
2. `crates/launcher/desktop/docs/launcher-telemetry.md`: the `play_cooked_presync` phase row (beside l.58-59), the two events with their keys, and a SigNoz recipe: filter `event = cooked_presync_finished AND outcome = failed`, group by `step`, `reason`; join `launch_not_started` on `operation`. Say plainly that these rows reach SigNoz only with the game-diagnostics opt-in or Upload debug logs (F19, D-LP27).
3. `crates/launcher/desktop/docs/privacy-inventory.md`: a network row for `/api/cooked/v1` (data sent, destination, write location) and a local-log row for the two events.
4. `docs/client/launcher-guide.md`: Play (the progress text), the layout (Documents `Cache.en-US`), and troubleshooting: the failure message's parts, Retry, the common remedies (close the game for `pak_in_use`, Repair for `source_missing` or `digest_mismatch`, tell the operator for `catalog_failed`).
5. `docs/guides/macos.md`: one note that macOS doesn't presync yet (LP-F1) and plays as before.
6. `crates/launcher/desktop/README.md` and the `launcher-core` row in `crates/README.md`: one line each.
7. Skip the over-cap implementation ledger (D-LP16) and say so in the report, so the PR body lists it as skipped.

Checks: the R5 check on every edited doc.

## Wave 5

### LP-30 Close-out: status docs, unified UAT, retire

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `launcher-cache-presync/LP-30-close-out`, at `main` after waves 2 to 4 and the campaign review fixes · **Worktree:** `lcp-30`
- **Commit:** `docs: launcher-cache-presync close-out (LP-30)`
- **Rules:** R1, R3, R4, R5, R7, R8.

Steps:

1. `docs/project-status.md` and the matching `docs/gap-analysis*` row: once, naming the presync and that a failed presync blocks Play with Retry (no fallback to the resync).
2. `docs/guides/unified-uat.md`: rows LP-U1 to LP-U4 from the ledger.
3. Ledger README: every packet status, and the follow-ups LP-F1..F4 (LP-F4 stays BlockedDecision on D-LP28 unless the owner has answered).
4. The orchestrator then has `ops` retire every `lcp-*` worktree: `rm-worktree.ps1 --merged-into <PR> <names>` for packet worktrees merged only into an integration branch. It empties `.claude/state/active-campaign`.
