# Amendment 7: wave-4 pre-check (LT-18..LT-23)

Architect pre-check of the wave-4 packets before dispatch, done the same way as [LT-14-amendment.md](LT-14-amendment.md) was for LT-16 and LT-17. Code read at the `lt-app-integration` tip (worktree `.claude/worktrees/lt-app-integration`, branch `launcher-telemetry/launcher`, LT-05..LT-17 merged). LT-16b's files (client maintenance, client changes) were avoided. The amended packets are in [work-packets-wave4.md](work-packets-wave4.md). No code was changed.

## What changed

| Packet | Defects found | Amended |
|---|---|---|
| LT-18 | Wrong decoder file: the decoder is `storage/launch/supervisor.rs`, not `worker.rs`. `stage: Option<String>` would break `Event: Copy`. `NotStarted`/`Unknown` are unit variants. The panic path is already logged by LT-17a (`worker.rs:79-89`). The real supervisor path is tested only on unix. | A closed `Stage` enum; struct variants with `serde(default)` fields; a per-line mapping in `windows.rs`; two new helpers that keep the event names; three supervisor anchors; decode and capture tests |
| LT-19 | `preferences_unreadable` is invented: `storage/mod.rs:131` and `:263-266` are defaults, and the errors already propagate. `login_servers`/`custom_login_servers` are not C6 fields. The game-consent event was put in the shell when the engine setter is the one place. The source-scan test forbids `launcher_summary_consent` in a new file. `host.rs:211-212` is an automatic save. | Four helpers with `count` and the named boolean `custom_servers`; a before/after compare in `dispatch`; the engine setter call; the host.rs save stays silent; a named dispatch test |
| LT-20 | Eleven sites, of which several are teardown-only or defect-only and cannot be tested. The "interruption only" comment it relies on does not exist. Its test approach was "mock `ui-event.ts`", which the node:test bundle cannot do. | Cut to five testable sites plus a `uiCode` helper; the fake-`__TAURI__` test pattern; one test per file |
| LT-21 | Too large for one Haiku packet. `logs.rs:301` (`upload_blob`) is off the desktop path. `store.rs:78` logs the blob name (R5). The launcher-log folder and the `Redaction` had no named source. | Split into LT-21a (nine events retargeted, blob name dropped) and LT-21b (the `launcher_logs.rs` reader and zip append); the folder comes from `directory.root.parent()/logs`, the redaction from `Redaction::from_env()` |
| LT-22 | "`parse` keeps its https-only rule" is wrong: `parse` accepts loopback http for fixtures, and the summary tests depend on it. The tests are inline in `endpoint.rs`, not under `tests/`. | `parse` unchanged; the https check sits in `for_login_server`; inline tests named; the const is re-exported |
| LT-23 | Too large, and it touched Rust and the frontend at once. The `disabled` conditional resolves to "keep it" (`view.ts:75` enables the box). The DTO is the engine `NativeSnapshot`, which the shell cannot set. `host.rs` needs no change. | Split into LT-23a (the shell `config(first_login_server)`, a debug line, tests) and LT-23b (`summary_endpoint_configured()`, a boolean `NativeSnapshot.setup_diagnostics_endpoint`, the TS optional field, the copy, the first-press text, tests) |

Wave 4 is now eight packets: LT-18, LT-19, LT-20, LT-21a, LT-21b, LT-22, LT-23a and LT-23b.

## Decisions (architect, within the campaign's decided scope)

1. **LT-18 keeps the event names.** Worker failures still log `launch_not_started` and `launch_failed`, now with `reason` from the stage and `win32`. A worker `Unknown` with no stage gives `reason=worker_unknown`. No new event names, so the dashboards stay valid.
2. **LT-19's `custom_servers`** is true when any saved entry's url is not the url of an entry of `default_servers()`. The list length goes in `count`. No server name or url is logged.
3. **LT-21b scrubs per line with `Redaction::scrub` and then `scrub_ips`.** A line that still holds an absolute path after home replacement becomes `<path removed>`. That is the privacy rule `launcher_event.rs:108-116` already applies to forwarded rows. The digest and "nothing new" still cover only the game logs, and nothing is uploaded when there are no game logs. Both are accepted limits, written into the module doc.
4. **LT-23 resolves the endpoint once, at start**, from the list Play uses: the adopted list, else the saved list, else the defaults. A server-list change applies from the next launcher start.
5. **The frontend gets a boolean, not the three-way source.** The copy only needs "sent" or "not sent yet". The source (`login_server`, `default` or `no_https_endpoint`) goes on a local debug line on `launcher.pipeline` as `outcome`.

No BlockedDecision.

## Capture standard (recorded in work-packets-wave4.md § Capture standard)

LT-16a's final commit `44a052bda` (`engine/src/storage/uninstall/tests.rs`) made the uninstall test deterministic with:

1. `tracing::callsite::rebuild_interest_cache();` as the first line inside the `capture` closure;
2. a module-level `static SERIAL: Mutex<()>` with `fn serial()`, which recovers from poisoning with `unwrap_or_else(|e| e.into_inner())`, and `let _serial = serial();` as the first line of each of the 11 tests in that module that drive the same call sites.

Its worknote names the remaining risk: a test in another module can still fire the callsite first with no subscriber. LT-17a (`launch/events.rs` tests :199-205) covers that case by holding `let _second = tracing::Dispatch::new(tracing_subscriber::registry());` for the whole test, so tracing keeps callsite interest per dispatcher.

**The standard for wave 4 and after:** all three pieces in every new capture test (SERIAL, `_second`, and the rebuild first in the closure).

Recommended follow-up, not a packet: move the rebuild and a crate-wide lock into `test_capture::capture` itself (engine and shell, 101 lines each). After that, tests need no boilerplate. The wave-7 test audit should decide.

## Parallel lanes and merge order

Five lanes run at once; no two lanes share a file. Full table: work-packets-wave4.md § Start, parallel lanes and merge order.

| Lane | Worktree | Packets | Needs merged first |
|---|---|---|---|
| 1 | `lt-18` | LT-18 | wave 3 (including LT-16b) and the wave-3 review; `origin/main` merged into `lt-app-integration` |
| 2 | `lt-19` | LT-19 | the same |
| 3 | `lt-20` | LT-20 | the same |
| 4 | `lt-21` | LT-21a → LT-21b (same branch, fresh coder each) | the same; LT-21b needs LT-21a's commit |
| 5 | `lt-22` | LT-22 → LT-23a → LT-23b (same branch, fresh coder each) | the same; each needs the commit before it |

- **Shared file:** `engine/src/commands.rs`. LT-19 edits `dispatch` (:71-106) and LT-23b edits `NativeSnapshot`/`inspect` (:29-68), in different hunks. Merge lane 2 before lane 5.
- **Frontend:** lanes 3 and 5 touch different frontend files. LT-20's tests already exist, so `package.json` should not change; LT-20's rule 6 covers it if one is missing from the test list.
- **Merge into `lt-app-integration`:** LT-18 → LT-19 → LT-20 → LT-21 (a+b) → LT-22/23a/23b → wave-4 review. It ships inside PR L, with no per-packet PRs.
- **Wave review:** one packet-reviewer holds all eight small packets. If it runs short, review lanes 1-3, then lanes 4-5.

## Notes for the orchestrator

- Brief each coder with its packet section **and** § Capture standard (Rust packets), plus its worktree path. Lanes 4 and 5 need a fresh coder per packet, in the same worktree.
- LT-18's check D2p builds `cimmeria-runtime-probe` on Windows, where `windows.rs` compiles. `supervisor_tests.rs` is `cfg(unix)`, so the Windows lane does not run it. A Linux CI run would catch a missed `Event::NotStarted` constructor there, and the packet tells the coder to grep for and update them.
- The README packet table and RESUME now point at work-packets-wave4.md. work-packets-coverage.md keeps pointer stubs and an Amendment 7 note. The wave-4 row of the work-packets.md merge table is updated.
- Follow-ups recorded, not packets:
  - **F-W4-1:** `install-view.ts:136` swallows a typed observe failure silently. Report it when a test can drive it.
  - **F-W4-2:** the C9 capture hardening above.
  - **F-W4-3:** include launcher logs when there are no game logs, and count them in the digest. This needs a store-side decision on the dedup key.

## Tested vs not

- **Read and verified at the tip:**
  - every file:line anchor in the packets;
  - the `Event` enum and its uses;
  - the `InjectError` variants;
  - the supervisor match arms;
  - the `launch/events.rs` helpers and test style;
  - the `commands.rs` arms and test template;
  - the `storage/mod.rs` defaults;
  - the `game_telemetry` setter;
  - the frontend sites and test helpers;
  - the `log_upload` events and the `LogCollection` fields;
  - the `upload_blob` callers;
  - the `zip` dependency;
  - `Redaction`/`scrub_ips`;
  - `endpoint.rs` and its tests;
  - `summary.rs` `start`/`config`/tests;
  - the `NativeSnapshot` struct and `inspect`;
  - the tracker gate;
  - `view.ts` box enabling;
  - `index.html:13`;
  - the admin-api route `/api/telemetry/launcher-summary` on the login port.
- **Not verified:**
  - that `deny_unknown_fields` with internally tagged struct variants decodes the old `{"event":"not_started"}` (LT-18 tells the coder to return `blocked` if it does not);
  - the exact `set_game_telemetry` signature (the coder reads :115);
  - the `logs::LogError` variant names (the coder reads the enum);
  - the exact `UiArea` literals (the coder returns `blocked` if one is missing);
  - whether `physx-view.test.ts` and `ui-event.test.ts` are in the `package.json` test list (the test-list output was cut off; rule 6 covers it);
  - the Effect Schema optional-field spelling in `contract.ts`.
- No build or test was run. This is a docs-only amendment.
