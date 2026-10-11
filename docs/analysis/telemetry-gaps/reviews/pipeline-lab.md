# Telemetry pipeline and lab telemetry review

> Reviewer: general-purpose (TG-PIPE). Data: colo ClickHouse, last 7 days, aggregates only. Code: `tg-ledger` worktree at launch.

**Summary.** The upload path is careful about refusals, but it has one serious leak: the bundle replay exports session key dumps and `current-session.json` token lines to SigNoz (TG-PIPE-01). The client side cannot explain how any process ended. The DLL never flushes at exit, lab `session_id`s span up to 29 SGW.exe launches, and labd logs nothing when it launches, stops or overwrites a client. S1 as written cannot pass: no server path emits `player session ended disconnect_reason=logOff` for a lab logout.

## 1. Inventory

| Surface | Target(s) | Reaches SigNoz | Fired in 7 days |
|---|---|---|---|
| DLL events | `client.native` replay (`client_target` = DLL name) | `cimmeria-client`, every level | 2.39M rows, 36 sessions (16 lab, 21 player), 139 DLL attaches |
| DLL self-report | `client.dll.attached`, `client.hooks.*`, `client.telemetry.health`, `client.telemetry.rollup` | yes | health 1,864 (all INFO, no drops), rollup 79k |
| Launcher live tail | `launcher.client_log` / `launcher.debug_log` (chunk path) | yes | **0**: every `launcher.client_log` row came from bundles; `launcher.debug_log` never fired |
| Launcher bundle | `launcher.client_log` (source=bundle), `launcher.bundle` | yes | 812k INFO lines, 12 bundles, **player sessions only (no lab bundles)** |
| Session meta | `launcher.session_meta` | yes | 14 (21 player sessions) |
| Ingest | `launcher.ingest` | `launcher=debug` | 53k DEBUG `upload-chunk accepted`; 14 WARN `busy` |
| Ingest naming | `launcher.ingest event=entity_labels_unavailable` | DEBUG | 0 |
| Mint | `cimmeria_admin_api::routes::dev_session*` | INFO via default | 36 mints |
| lab-server MCP | `lab.tool_call` | INFO via default `info` | 433 (11 `outcome=error`) |
| labd | `lab.lease`, `lab.instance`, module-path watchdog/lifecycle rows | **no** (local `labd.log` only; D-TG2 pending) | n/a |
| Claude Code | `service.name=claude-code` | yes | 141k; 4,663 `tool_result` rows report the tool only as `mcp_tool` (552 failed) |

The `OTEL_FILTER` default is `info`, so INFO rows from `cimmeria_lab_mcp` and `cimmeria_admin_api` arrive. Their DEBUG rows do not, but none are load-bearing today.

## 2. Positive gaps

- **No per-process identity on client rows.** Lab grants are cached and reused across relaunches (`supervisor/telemetry_session.rs:300`), so one lab `session_id` averages about 8 and peaks at 29 `client.dll.attached` rows. No client row carries a pid or launch id (`session.rs:245` `identity_fields`), so "which process wrote this" can only be read off row order between attaches. Player sessions are 1:1.
- **No session end from the DLL.** `boot.rs:342` runs `run_uploader(consumer, cfg, || false)`: `should_stop` is never true, so `governor_finish` (`uploader.rs:159-167`) never runs in production. Every exit loses the open rollup window, the unsent batch and the final health row. This blocks T5's `client.session.end`.
- **labd launch, stop and pid change are silent.** `launch_client` (`supervisor/mod.rs:382-477`) logs nothing on success, and `st.pid = Some(pid)` (`:469`) overwrites a live previous pid without a row. That is exactly the #1342 orphan. `stop()` (`lifecycle.rs:57`) logs only when the wait fails, never the kill itself.
- **labd tool calls have no row.** `server/handler.rs:90-129` opens only a `lab_call` span. A refusal from `gate_call(...)?` (`:116`), a tool error and the call's duration all go unrecorded. lab-mcp has an audit row for this; labd has none.
- **Launcher game exit is local only.** `launcher/src/telemetry/runner.rs:96` logs `exit_code` to the launcher's own tracing; `SessionMetaKind` (`events.rs:93`) has no `Ended`.
- **Char-select logout has no `session.end` row.** `base/dispatch/session.rs:206` logs `logOff: returning to character select` (27 times), with no `event=` and no `disconnect_reason`. It clears `player_entity_id` (`:219`), so the later teardown has no player and `session.end` never fires for that character (`base-session/.../session_teardown.rs:254`).

## 3. Negative gaps

- `uploader.rs:213` `Err(_) =>`: the POST error (network, 401, 503, 413) is discarded. Only half-batch drops are counted (`:222-226`), so an hour of 503s at a small batch size shows nothing in SigNoz. `uploader.rs:165,191` discard the final POST result with `let _`.
- `uploader.rs:262-267` claims the server is "idempotent on `(install_id, session_id, seq)`". It is not: `replay.rs` has no dedup. A retry after a lost response duplicates rows.
- `boot.rs:344` `.spawn(...).ok()`: if the uploader thread fails to start, nothing is logged and telemetry is dead. `boot.rs:331` `let _ = producer.try_emit(dll.attached)`.
- `boot.rs:362,379,434` (lab bridge failed to start, input not hooked, install lock unavailable) reach only the local DLL log. The lab then sees tool timeouts with no cause in SigNoz.
- `upload_gate.rs:332-333` `take_slot(...).ok_or(IngestError::Busy)`: the row does not say whether the per-address share or the route pool refused. All 14 busy refusals were from **one address, 4 sessions** on 2026-10-10 (the parallel-lab day), which points to the per-peer share. The refusal row also has no `session_kind` (`refusal_log.rs:147`).
- `entity_labels.rs:384-391` `entity_labels_unavailable` has no `session_id`.
- `lab-mcp/src/audit.rs:28` logs `outcome=error` with no reason and no `duration_ms` on any call.
- `admin-api/.../bundle_unzip.rs:189`: every line replays at INFO with no level, so client ERROR lines are indistinguishable from DEBUG chatter (all 812k rows are INFO).
- Exporter self-health: `opentelemetry=off` (`filters.rs:381`) is correct to stop loops, but a dropped OTLP batch leaves no trace in SigNoz. Not packeted; see the note at the end.

## 4. Noise

- **Leak (not noise, high):** `bundle_unzip.rs:182-198` replays every UTF-8 entry of the bundle. `launcher/src/logs.rs:115` zips the whole `Binaries/sessions` tree. In 7 days SigNoz received:
  - 541 `*-keys.txt` lines that match long base64 runs (2026-09-29 to 10-06);
  - 242 `current-session.json` lines, 14 of them with a `token` key;
  - `lab-instance.json` lines;
  - re-uploads of `OLD/` and `C++/` history.

  That defeats the `launcher.key_dump=off` pin (`filters.rs:377,400`). 708 `.pcap`/binary entries were skipped as non-UTF-8 at DEBUG.
- `client.hooks.iat.slot_mismatch` WARN in 121 of 139 attaches (T11). New data for T11: lab 109/118, player 12/21. `client.mercury.socket_recv` fired in **0 lab sessions** and 3 player sessions, so raw receive telemetry is blind in every lab run. `actual` ends in `…15d0` across ASLR bases, which means one module at a fixed RVA owns the slot. There are two emitters (`iat_hooks/mod.rs:215`, `sinks/install.rs:141`).
- `launcher.ingest` `upload-chunk accepted` DEBUG at 53k/week is fine at DEBUG.

## 5. Seams (two hops)

| Hand-off | Sender logs | Receiver logs | Can we tell who dropped it? |
|---|---|---|---|
| DLL to ingest (POST chunk) | nothing per failure (§3) | refusal WARN, throttled, with session | Partly. Server refusals yes; network failures, timeouts and lost responses no |
| Ingest to cell naming channel | `entity_labels_unavailable` (no sid) | cell side unknown | No session pivot |
| Session budget (`session_budget.rs:201`) | n/a | WARN `session_over_budget` | Yes. T5 must add `client.session.` to `PRIORITY_PREFIXES` |
| Launcher to bundle route | local `tracing::warn` (`runner.rs:211`) | `launcher.bundle` | Lab never sends bundles; 7 player sessions have no session_meta, and no row says why |
| labd mint to admin-api | labd local log | `Minted dev-session telemetry token` (36) | The reuse path (`telemetry_session.rs:302`) is local only, so SigNoz can't tell a reused grant from no grant |
| labd kill (`process.rs:213`, exit code `0xDEAD`) to server | none (§2) | `inactivity_timeout` or `duplicate_login` teardown | No. A lab kill and a player crash look the same (T5/T6) |
| Runner `lab_logout` to `SGWPlayer.logOff(0)` | runner evidence bundle (local) | `logOff: returning to character select` | Yes, but no `session.end` |
| Char-select Back to `Account.logOff` 0xC2 (`base/login/mod.rs:554`) | no lab flow exists | `Client requests logOff`, teardown `disconnect_reason=logoff` (3 this week) | Server side only |
| Claude Code to lab | `claude_code.tool_result` tool=`mcp_tool` | `lab.tool_call` (server tools only) | No join key: Claude rows lack the MCP tool name; lab rows lack the caller session |
| Server teardown to presence/org | `session.end`, `org` | n/a | Neighbour reviews (auth, social) |

## 6. Adversarial scenarios

1. **The lab watchdog relaunch orphans an SGW.exe (#1342, hit 2026-10-10).**
   - **Today:** labd's local log shows `relaunched after crash new_pid=…`. SigNoz shows two `client.dll.attached` rows under one lab `session_id` with no pid, plus a server `duplicate_login` teardown. Nothing names the orphan.
   - **Should show:** `lab.client.launched {instance, pid, cause=watchdog_relaunch, previous_pid}`, and a WARN `lab.client.pid_overwritten {previous_pid, previous_alive=true}`. Once D-TG2 ships it, both reach SigNoz.
2. **A player's client exits normally or crashes.**
   - **Today:** client rows simply stop, and the last 2 s and the open rollup window are lost. The launcher writes the exit code locally. The server sees `inactivity_timeout` about 15 s later.
   - **Should show:** a final `client.telemetry.health {final:true}` and `client.session.end {reason}` on a hooked exit (TG-PIPE-03), and `launcher.session_meta kind=ended {exit_code}` (TG-PIPE-12).
3. **Five parallel lab clients from one address hit `busy`.**
   - **Today:** one WARN `busy` per session per 10 s; no pool name, no kind.
   - **Should show:** `pool=peer_share`, `limit`, and `session_kind=lab`.
4. **The lab bridge fails to bind.**
   - **Today:** every lab tool times out. SigNoz shows a healthy `client.dll.attached` and the hook rows.
   - **Should show:** a WARN `client.dll.bridge_failed {reason}`.
5. **An S1 run ends.**
   - **Today:** the runner never logs out. If it did, the server would log `logOff: returning to character select` and nothing in `session.end`. The spec's assertion `player session ended disconnect_reason=logOff` would fail on every run.

## S1: how it would be implemented

- **Where the runner lives:** `crates/lab/src/uat/runner/` (`mod.rs` `run_all` :230, `drive_row` :389). The MCP tool is `server/uat.rs`.
- **Session-level setup and teardown:** none. `SectionSpec`/`SectionMeta` (`uat/spec.rs:29-58`) are `deny_unknown_fields` and carry only the character and account. `setup`/`teardown` are **per row** (`spec.rs:121-125`). Teardown is skipped when setup failed (`runner/mod.rs:424`) and when the lease is revoked (the `select!` at `:334` drops `drive_row`). `run_all` has no end hook. No spec in `docs/guides/uat-specs/` (12 files) has a teardown that logs out; specs reach `char_select` or `client_stopped` only as a row's starting `state`.
- **What `lab_logout` does today:** it types `/logout` in chat and waits for the character-select window (`supervisor/flows/login.rs:347-374`). The server runs `SGWPlayer.logOff(0)`, path `logoff_character_select`; the session stays and no `session.end` row is written. There is no flow for a full disconnect. `lab_client_stop` calls `TerminateProcess(…, 0xDEAD)` (`process.rs:213`), and the server then ends the session by `inactivity_timeout`.
- **Today's real disconnect labels:** `logoff_character_select`, `logoff_full_exit` (`dispatch/session.rs:45-49`), `logoff` (Account 0xC2), `inactivity_timeout`, `duplicate_login`, and others. There is no `logOff`. **S1's assertion must change** to `disconnect_reason=logoff`, or TG-PIPE-09 must land first.
- **Recommended shape:** runner-level, not spec-level, so it runs once per run whatever the row filter is:
  1. `lab_logout`.
  2. A new `lab_disconnect` flow that clicks Back at character select, which sends Account `logOff` (TG-PIPE-07).
  3. A server clause through `server_log_tail` for `Client requests logOff` and `disconnect_reason=logoff`.
  4. Optionally `lab_client_stop`.

  Record it as a synthetic `_session_end` row in the manifest (TG-PIPE-08).

## Candidate packets

| ID | Title | Sev | Side |
|---|---|---|---|
| TG-PIPE-01 | Bundle replay: allowlist log files, never keys or session JSON | high | Server |
| TG-PIPE-02 | DLL sends its pid with every chunk | high | Client DLL |
| TG-PIPE-03 | DLL final flush at process exit | high | Client DLL, needs-domain-agent |
| TG-PIPE-04 | Ingest stamps `client_pid` on replayed rows | high | Server |
| TG-PIPE-05 | labd launch/stop/pid-overwrite rows | med | Lab |
| TG-PIPE-06 | labd per-tool-call row | med | Lab |
| TG-PIPE-07 | `lab_disconnect` flow | med | Lab |
| TG-PIPE-08 | S1: runner ends every run with logout and disconnect | med | Lab |
| TG-PIPE-09 | `session.end` row for logout to character select | med | Server |
| TG-PIPE-10 | Busy refusal names its pool | med | Server |
| TG-PIPE-11 | lab-mcp audit: error reason and duration | low | Server |
| TG-PIPE-12 | Launcher `session_meta kind=ended {exit_code}` | low | Launcher |
| TG-PIPE-13 | DLL upload-failure accounting on health; fix false idempotency claim | low | Client DLL |
| TG-PIPE-14 | DLL boot failures as events | low | Client DLL |
| TG-PIPE-15 | Bundle lines carry a parsed `level` | low | Server |
| TG-PIPE-16 | `entity_labels_unavailable` carries `session_id` | low | Server |

**TG-PIPE-01. Bundle replay allowlist (high).**
- **Files:** `crates/admin-api/src/routes/telemetry/bundle_unzip.rs` (entry loop around :140-210).
- **Change:** replay an entry only when its path ends `.log` or is `SGWDebugLog.log`. Skip everything else (`*-keys.txt`, `*.json`, `*.pcap`, `*.txt`) before the UTF-8 read. Log a DEBUG `launcher.bundle` `event=bundle_entry_skipped {session_id, reason="not_a_log", ext}`, with no path content beyond the extension.
- **Why server-side:** deployed launchers keep sending these files.
- **Test:** a unit test with a zip holding `a.log` and `x-keys.txt`, under `LogCapture`. It asserts no `launcher.client_log` row has `source_file` ending `keys.txt`, and that one skip row appears. Reverting the change replays the keys line, so the test fails.
- **Follow-ups:** the owner decides whether to purge existing rows. A launcher-side filter in `launcher/src/logs.rs` (`collect_log_inputs`) is a separate low follow-up.

**TG-PIPE-02. DLL sends its pid (high).**
- **Files:** `crates/client-telemetry/src/uploader.rs` (`post_batch`), `boot.rs` (add `pid` to `client.dll.attached`).
- **Change:** add the header `X-Cimmeria-Client-Pid: <GetCurrentProcessId>` on every POST.
- **Test:** extend `uploader_ships_batch_to_local_server` to assert the header. Removing it fails the test.
- **Release:** ships per D-TG4.

**TG-PIPE-03. Final flush at exit (high, needs-domain-agent).**
- **Files:** `boot.rs:342` (stop flag), `uploader.rs` (bounded final POST), and an exit hook (an IAT `ExitProcess` slot or a UE3 shutdown site; needs RE).
- **Change:** set the stop flag, wait at most 1.5 s for `governor_finish`, and mark the final health row `final: true`. This is the carrier for T5's `client.session.end`.
- **Test:** a unit test that drives `run_uploader` with a stop callback and asserts the last POST holds a health row with `final: true`.

**TG-PIPE-04. Ingest stamps `client_pid` (high).**
- **Files:** `routes/telemetry/chunk.rs` (read the header, validate it as u32), `replay.rs` and `replay_native.rs` (add a `client_pid` record attribute).
- **Check:** the 32-field cap on the ID-pair shape (`observability.md` §client index).
- **Test:** a `client_index_tests`-style replay with the header asserts the attribute. Without the header the attribute is absent, never 0.
- **Docs:** add a row to the attribute table in `observability.md`.

**TG-PIPE-05. labd launch and stop rows (med).**
- **Files:** `crates/lab/src/supervisor/mod.rs` (`launch_client` gains `cause: &'static str`), `lifecycle.rs` (start, stop), `watchdog.rs` (relaunch).
- **Events:**
  - INFO `lab.client` `event=launched {instance, pid, cause=start|restart|watchdog_relaunch, telemetry_session_id, grant=minted|reused|unavailable}`
  - WARN `event=pid_overwritten {previous_pid, previous_alive}` when `st.pid` is `Some` and alive at `:469`
  - INFO `event=stopped {pid, exited, cause}`
- **Test:** a `LogCapture` unit test using `fake_bridge`: two launches without a stop give one `pid_overwritten` row. It fails without the row.
- **Pin:** none in the server. D-TG2's labd exporter must keep `lab.client`.

**TG-PIPE-06. labd per-tool-call row (med).**
- **Files:** `crates/lab/src/server/handler.rs` (`call_tool`).
- **Event:** INFO `lab.call` `{tool, instance, lease_owner?, outcome=ok|error|refused, duration_ms, error (capped at 200 chars)}`. No args, since they can carry Lua or chat text.
- **Test:** call through `call_tool` with a lease-gated tool and no lease, then assert `outcome=refused`.

**TG-PIPE-07. `lab_disconnect` flow (med).**
- **Files:** `supervisor/flows/login.rs` (new `disconnect_flow`: at character select, click Back, wait for the login screen), `server/flows.rs` (register the tool), `lease/policy.rs` (guarded tool list), `uat/tools.rs` (driver entry).
- **Test:** a unit test with fake CEGUI calling the widget click; the policy test lists the tool as guarded.

**TG-PIPE-08. S1 runner end-of-run (med; depends on 07, and on the S1 assertion text being corrected).**
- **Files:** `uat/runner/mod.rs` (`run_all`, after the loop), `uat/runner/session.rs` (new `end_session`), `server/uat.rs` (`end_session: bool`, default true; false for `plan_only`).
- **Steps:** skip when the lease is revoked or the client is not running. Otherwise run `lab_logout`, then `lab_disconnect`, then a server clause on `server_log_tail` containing `disconnect_reason=logoff`. Record the result as row `_session_end`.
- **Test:** a runner test with the fake invoker asserts the call order `lab_logout`, `lab_disconnect` after the last row, and none under `plan_only`.

**TG-PIPE-09. `session.end` for logout to character select (med; may duplicate the auth review).**
- **Files:** `crates/base/src/base/dispatch/session.rs` (`handle_log_off`).
- **Event:** INFO `session.end` `"character session ended" {account_id, account_name, player_id, player_name, entity_id, entity_name, disconnect_reason=path, session_secs?}`, emitted only when `ended` is `Some`.
- **Test:** a `LogCapture` unit test on `handle_log_off(0)` with a seeded session.

**TG-PIPE-10. Busy names its pool (med).**
- **Files:** `upload_slots.rs` (`take_slot` returns `Result<_, BusyPool>`), `upload_gate.rs:332`, `dto.rs` (`Busy { pool, limit }`, mapped into `budget()`/`limit()`).
- **Test:** the existing `upload_limits_tests::gate` busy case asserts `budget="peer_share"`.

**TG-PIPE-11. lab-mcp audit (low).**
- **Files:** `crates/lab-mcp/src/audit.rs`, `tools/mod.rs`.
- **Change:** add `error` (capped) and `duration_ms`; WARN level on error.
- **Test:** a `LogCapture` test on `server_db_query` with bad SQL.

**TG-PIPE-12. Launcher `ended` meta (low).**
- **Files:** `crates/launcher/src/telemetry/events.rs` (`Ended`), `runner.rs:96`.
- **Change:** enqueue `kind=ended {exit_code, pid}` before the final flush. The server accepts any `kind` string (`dto.rs:73`). Make the `let _` at `:78` a WARN.
- **Test:** a serde round-trip, plus a runner test with a fake exit.

**TG-PIPE-13. DLL upload accounting (low).**
- **Files:** `uploader.rs:213`, `governor/mod.rs` (`ExternalDrops` gains `upload_failures`, `last_status`), `queue.rs`.
- **Change:** health reports `upload_failures_total` and `last_upload_status`. Correct the doc at `uploader.rs:262`.
- **Test:** the `retains_batch_after_failed_post` case asserts `upload_failures_total=1`.

**TG-PIPE-14. DLL boot failures as events (low).**
- **Files:** `boot.rs:344,362,379,434`.
- **Events:** WARN `client.dll.uploader_spawn_failed`, `client.dll.bridge_failed {reason}`, `client.dll.input_unhooked`, `client.hooks.install_lock_unavailable`.
- **Test:** a unit test on an extracted `report_bridge_result` helper.

**TG-PIPE-15. Bundle line level (low).**
- **Files:** `bundle_unzip.rs:189`.
- **Change:** parse the log4cxx level token (`DEBUG|INFO|WARN|ERROR|FATAL`) after the timestamp into a `level` field. Keep the severity as is.
- **Test:** a `LogCapture` test on one ERROR line.

**TG-PIPE-16. Naming row session (low).**
- **Files:** `entity_labels.rs:384` (pass `sid`).
- **Test:** a `LogCapture` test on `resolve` with a busy permit.

**Not packeted (coordinator notes):**
- T11 now has the per-kind data above. The fix should name the slot owner with `GetModuleHandleExW(FROM_ADDRESS)` in both emitters.
- Claude Code reports MCP tools as `mcp_tool`. Check whether the installed version's tool-detail logging setting adds server and tool names, as an ops change. Without it, Claude Code rows cannot be joined to lab rows.
- OTLP exporter drops are invisible in SigNoz by design. A periodic dropped-records counter would need a domain decision.
