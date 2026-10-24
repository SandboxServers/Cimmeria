# LT-14 amendment (amendment 6): packet gaps resolved, LT-16 and LT-17 pre-checked

Architect ruling, 2026-10-21. Base `3d10ddf4d` (launcher integration tip after LT-40), read in the unedited `lt-14` worktree. Docs only; no code changed. No `BlockedDecision`.

## What changed

| File | Change |
|---|---|
| [work-packets-install.md](work-packets-install.md) (new, 230 lines) | LT-14a and LT-14b, fully prescriptive |
| [work-packets-repair-launch.md](work-packets-repair-launch.md) (new, ~341 lines) | LT-16a, LT-16b, LT-17a, LT-17b, re-anchored |
| [work-packets-coverage.md](work-packets-coverage.md) | Amendment 6 note; LT-14, LT-16 and LT-17 sections replaced by pointer stubs (file kept under 500 lines). LT-12, LT-13 and LT-15 untouched |
| [work-packets.md](work-packets.md) | Wave-3 merge order (line 69); C6 allowed fields and value sources |
| [README.md](README.md) | Packet rows split into 14a/b, 16a/b, 17a/b; packet-file links |
| [RESUME.md](RESUME.md) | State line and wave-3 row |

## Coder items and decisions

1. **Unnamed call sites.** All named in LT-14b, using `install_worker/mod.rs` line numbers and applied bottom-up:
   - `install_started` goes before :150 in `spawn_worker`.
   - `install_finished` goes between :226 and :227. It is the one terminal event and covers every `Outcome`: ok and cancelled log at info, and every other outcome logs a warn `install_failed` with the outcome's code as `reason`.
   - `install_phase` comes from a `PhaseLog` built after :245 in `watch_phases`, and `phases.enter(phase)` after :255. It logs once per phase, so it fires only on a change.
   - `install_stage_failed` replaces the :387 `Err` arm.
   - :214, :223, :296 and :311 stay as the packet named them.
   - The shell call is at `host/install/mod.rs:301`.
   - `mod.rs` grows by about 25 lines, to about 520, which is under 700.
2. **`operation: u64` source.** LT-14b adds a new engine module, `operation_numbers`:
   - `number(id: Uuid) -> u64` gives each UUID a process-local counter value the first time it sees it.
   - `elapsed_ms(id)` measures from that first sight.
   - It keeps the last 64 entries.
   - Every helper takes the `Uuid` and converts it internally, so the UUID never reaches a field.

   The rejected options:
   - A truncated UUID is still an identifier, so it is too close to R5.
   - The revision is not in scope at `spawn_worker`, and it does not stay the same across the operation.
   - A counter per area would give the shell, worker and supervisor different numbers for the same operation.

   The shared map is what lets launch's supervisor, which only has `request.operation_id`, log the same number as the worker.
3. **Value sources.**
   - `action` is `install` or `resume`, from `input.ownership.is_some()`, logged as the boolean `resumed`.
   - `outcome` comes from the `Outcome` enum.
   - `phase` is the closed list `download` or `extraction`, mapped from `TimedPhase`.
   - `reason` and `http_status` come from the new `code()` and `http_status()` functions:
     - core `install_codes.rs` (LT-14a) covers `InstallError` and `LocalSeedError`;
     - engine `error_codes.rs` (LT-14b) covers `ContractError`, `StorageError` and `IntentError`, matching their serde names.
   - `files` and `bytes` have no source on the install path and are dropped.
   - No `fn code(` existed in any launcher crate at base.
4. **Core test scope.** Accepted. LT-14a's tests are plain unit tests of `code()` and `http_status()`, which need no subscriber. The core event retargets are covered by the engine path test in LT-14b, which captures the whole install, core included, on the engine's `test_capture`.

## Other defects fixed in LT-14

- **`unpack/mod.rs:356` `println!`.** It sits in an ignored manual test, so it is out of scope.
- **`mark_uncertain_failed` with `error_kind`.** A `ContractError` is not an `io::Error`, so the helper now logs `reason = error.code()`.
- **The real-path test.** `a_failed_download_logs_the_install_lifecycle` reuses `telemetry_tests.rs:81-94` (a 404 download) inside capture on a current-thread runtime. It asserts that `install_started`, `install_stage_failed` (with `http_status` `"404"`) and a WARN `install_failed` share one `operation`.
- **Split.** LT-14 is now LT-14a (core) then LT-14b (engine and shell), in the same `lt-14` worktree on the same branch, so it merges once. The split brings each packet down to about 40k working tokens.

## LT-16 pre-check (now LT-16a/b)

- **Invented steps.** Repair has no verify step and no file or byte counts: it restages through `install_all`, then checks `content_valid` (preparation :255). The `verify_*` events, `files` and `bytes` are dropped. The new events are:
  - repair: `repair_started`, `repair_claim_failed`, `repair_stage_failed`, `repair_content_invalid`, `repair_finished` (commit :80), `repair_ended` and `repair_failed`;
  - uninstall: `uninstall_started`, `uninstall_finished`, `uninstall_failed` and `uninstall_kept_adopted_folder`.
- **Wrong or missing anchors.** client_changes is at :178, not :177. The anchors at preparation :152, :171, :200, :252, :256 and :349-357, uninstall :170, :181-207 and :353, and shell repair :290 were checked and named.
- **R5 leaks removed.**
  - uninstall :353 logs a directory path;
  - client_maintenance :97-101 and :122-126 log `?request` and `?error`;
  - client_maintenance :209 logs a folder path;
  - client_maintenance :220 logs a path and `%error`;
  - client_maintenance :230 logs a path.
- **Inspect logs nothing.** Inspect runs on every panel open, so it stays silent and only resets log.
- **Sizes.** `uninstall/mod.rs` is 491 lines and gets call lines only, about 15 more, ending near 506. That is over the soft cap but well under the hard cap of 700.
- **Split.** LT-16a covers repair and uninstall, with real-path tests on `finish_failure` (a journal replaced by a directory) and on a confirmed removal. LT-16b covers client maintenance and client changes, with a real-path test through `client_maintenance_fixture`.

## LT-17 pre-check (now LT-17a/b)

- **Wrong anchors.** The original `resources.rs:26` should have been :30 (the closure) and :41-45 (the telemetry DLL filter). Shell `launch/mod.rs:115` is a `cfg!` filter, not an event, so it is dropped.
- **`supervisor.rs`.** Each outcome is mapped to a closed `reason`: `request_invalid`, `request_too_large`, `spawn_failed`, `no_pid`, `observe_failed`, `stdin_write_failed`, `no_first_event`, `helper_not_started`, `helper_unclean_exit`, `unexpected_event`, `no_exit_event` and `pid_mismatch`. `game_started` and `game_exited` carry `exit_code` and `duration_ms`.
- **`worker.rs` (606 lines, call lines only, about 625 after).** Named sites:
  - :63 `launch_started`;
  - :78-81 splits `Ok(Err)` from a panic;
  - :107-113 commit failure;
  - :114 `launch_finished`;
  - :139 `verify_failed`;
  - :151 `prepare_failed`;
  - :226-229 `patch_events_queue_failed`;
  - :248 replaces a warn that logged `?error` and `%plan.id`, which are R5 leaks.
- **`admit_launch` warns.** The six warns at :88, :94, :116, :128, :148 and :188 log `operation = %id`, which is the UUID. They become `launch_refused` with closed reasons.
- **`telemetry.rs` and `helper_supervisor`.** Each silent `.ok()?` site is separated into a normal silent case and a real failure. The `helper_supervisor` stop failure is named, but it is not on the Windows Play path, so only its helper is tested.
- **Split.** LT-17a covers the lifecycle; its real-path test is the replaced-helper dispatch, `verify_failed`. LT-17b covers the telemetry session, helper stop and DLL artifact; its real-path test is an unreadable game-telemetry choice.

## Notes for the orchestrator and the wave-3 review

- **LT-12, LT-13 and LT-15 are running against the old C6.** They may each invent their own `operation` source, or their own `code()` methods on `ContractError`, `StorageError`, `IntentError` or `InstallError`.
  - The wave-3 review should align every `operation` with `crate::operation_numbers::number`.
  - If LT-13 or LT-15 already added a `code()` to one of those types, the LT-14 branch merge will conflict. Resolve by keeping one function, matching the serde names, and re-pointing the callers.
- **The same `error_kind` defect is in LT-13**, at `dispatch.rs:175` and `preparation.rs:172`, where `mark_uncertain` reports `error_kind` on a `ContractError`. This is reported only; LT-13 was not edited.
- **Merge order.** LT-14a → LT-14b is one branch and one merge into `lt-app-integration`. Then `ops` creates `lt-16` (branch `launcher-telemetry/lt-16-repair`) and `lt-17` (branch `launcher-telemetry/lt-17-launch`) from that tip, and they run in parallel: LT-16a → LT-16b and LT-17a → LT-17b, each one merge. Then the wave-3 review.
- **Dispatch.** LT-14a goes to a fresh packet-coder in the unedited `lt-14` worktree. It counts as a first dispatch, because the block was a packet gap.

## Tested vs not

Nothing was compiled. The anchors and types were read at `3d10ddf4d`. Two things are unproven until each packet's lane run, and each has a written `blocked` fallback:

- the exact line numbers;
- two host-dependent test triggers: a directory in place of a JSON file failing to read on Windows, used by LT-16a and LT-17b.
