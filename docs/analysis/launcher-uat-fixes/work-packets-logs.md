# Launcher UAT fixes: work packets (log ingest, waves 3-5, close-out)

Ledger: [README.md](README.md). Rules R1-R10, the check commands, the path shorthands (`E`, `A`, `FE`, `T`) and the dispatch table are in [work-packets.md](work-packets.md); every packet here uses them. Anchors were taken at `origin/main` `3c10f3624`; re-grep them.

The user's direction (2026-10-22): "We should be sending these logs to signoz on upload. Not the whole file but ingesting it in." The design keeps the launcher-session security model (R1-R16 of `security-review-launcher-session.md` (launcher-adopt-ux worknotes, not committed)) and separates log traffic by a token claim (D-LF7).

## Contract

### C-LF4: the `logs` purpose claim (server LF-10, engine LF-14)

- The dev-session mint request for `kind = "launcher"` takes an optional `purpose`. Accepted values: absent (today's event session) or `"logs"`. `"logs"` with any other kind, or any other value, is a 400 with a closed `reason` (`purpose_kind`, `purpose_unknown`).
- A `logs` mint is charged to the same per-IP and per-install mint tables (R6) and gets the same 900-second, no-refresh token (R3, R4).
- The token carries `purpose: "logs"`. The claim is absent for event sessions, so tokens minted before the change still verify as event sessions.
- The claim picks the limits before the body is read (R7). An event-session token refuses log targets, and a log-session token refuses launcher event targets (`launcher_targets.txt`), after decode and before any budget charge. Response: 403, `reason = "purpose_target"`. Any other unknown target in a log chunk is dropped and counted, not refused.

### C-LF5: the log row

A `ClientNativeEvent` (`launcher-core/src/telemetry/events.rs:63-80`): `ts_ms` (the line's timestamp, else the read time), `seq`, `target`, `level`, `fields`.

- `target`: one of the new canonical list `crates/launcher-core/src/telemetry/launcher_log_targets.txt`. It holds exactly two lines: `client.launcher.log` (the launcher's own log) and `client.launcher.game_log` (the game's `sgwdebuglog*` and `sessions/**/*.log`, unfiltered per D-LU3). The server accepts both from wave 3; the launcher sends `game_log` from LF-19.
- `level`: overwritten to `info` on the server (R11). The line's own level travels in `log_level`.

| Field | Type and rule (server-enforced; a row that breaks one is dropped and counted) |
|---|---|
| `message` | string, required. Scrubbed per word (D-LF9), then cut to 2,048 bytes on a char boundary with `...[truncated]` appended |
| `log_level` | one of `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` |
| `log_target` | ≤ 64 bytes of `[A-Za-z0-9_:.-]` |
| `file` | ≤ 64 bytes of `[A-Za-z0-9_.-]` (a file name, never a path) |
| `line_no` | integer ≥ 1 |
| `upload_id` | exactly 16 lowercase hex digits, required |
| `truncated` | bool, optional (the launcher cut the line) |
| `launcher_version` | ≤ 32 bytes of `[A-Za-z0-9_.+-]` |
| `os` | ≤ 16 bytes of `[a-z0-9_]` |

No other key. `DENIED_KEYS` still apply. The per-word scrub replaces each whitespace-separated word that `scrub_value` (`T/launcher_rows.rs:165-193`) flags with `<removed>` and keeps the rest.

### C-LF6: log budgets and limits (server LF-12)

| Limit | Value | Env override | On excess |
|---|---|---|---|
| Rows per chunk | 1,000 | — | 413 |
| Decompressed / compressed chunk | 512 KiB / 64 KiB | — | 413 |
| Row bytes | 4,096 | — | row dropped |
| Chunks per log session per minute | 20 | — | 429 + `Retry-After` |
| Rows per log session | 40,000 | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_SESSION` | 429 |
| Rows per install per UTC day | 80,000 | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_INSTALL_DAY` | 429 + `Retry-After` to the next UTC midnight |
| Rows per address per UTC day | 400,000 | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_IP_DAY` | 429 + `Retry-After` to the next UTC midnight (`log_ip_budget`) |
| Log rows per minute, all installs | 40,000 | `CIMMERIA_TELEMETRY_LOG_ROWS_PER_MINUTE` | 503 + `Retry-After` |

A log chunk is admitted whole or refused whole: the budget is checked for the chunk's row count before any row is written. Event sessions keep their limits exactly (R10). One upload sends at most 20,000 launcher lines plus 20,000 game lines (LF-19), so the session budget is 40,000. The per-minute and per-address ceilings count rows received; the session and install ceilings count rows kept. The per-install chunk window (20 per minute, `DEFAULT_LAUNCHER_CHUNK_PER_INSTALL`) is shared with the install's event session, so the launcher paces log chunks (LF-14, wave 3 ruling D1) and no limit here changes for it. Per D-LU4, a security note may lower the per-minute and per-day numbers but not below one full upload per install per day.

Replay: a log row's body is `message`; its other fields are attributes, as today. Event rows keep the target name as the body.

### C-LF7: the launcher side (LF-13 to LF-16)

```rust
// E/log_upload/lines.rs (LF-13)
pub(crate) struct LogLine { pub file: String, pub line_no: u64, pub end: u64, pub ts_ms: Option<i64>,
                            pub level: Option<String>, pub target: Option<String>, pub message: String, pub truncated: bool }
pub(crate) struct Collected { pub lines: Vec<LogLine>, pub lines_total: u64 } // lines: oldest first, at most the newest MAX_LINES
pub(crate) const MAX_LINES: usize = 20_000;
pub(crate) fn collect(log_dir: &Path, offsets: &Offsets, cancel: &CancellationToken) -> Result<Collected, LogUploadError>;

// E/log_upload/offsets.rs (LF-13): `uploaded-log-offsets.json` beside `uploaded-logs.json`
#[derive(Serialize, Deserialize, Default)] #[serde(deny_unknown_fields)]
pub(crate) struct Offsets { pub version: u32, pub files: BTreeMap<String, u64> } // file name -> byte offset already sent
impl Offsets { pub(crate) fn load(dir: &Path) -> Self; pub(crate) fn save(&self, dir: &Path) -> Result<(), LogUploadError>; // atomic
               pub(crate) fn advance(&mut self, line: &LogLine); }

// E/log_upload/ingest.rs (LF-14)
pub(crate) fn rows(lines: &[LogLine], upload_id: &str, launcher_version: &str, os: &str) -> Vec<TelemetryEvent>;
pub(crate) enum IngestOutcome { Sent { lines_sent: u64 }, Partial { lines_sent: u64, retry_after_secs: u64 }, Unreachable, Failed }
pub(crate) const MIN_CHUNK_INTERVAL_MS: u64 = 3_500; // wave 3 ruling D1: at most 18 log chunks per minute
pub(crate) trait Pacer { fn now_ms(&self) -> u64; fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send; }
```

DTO additions (`LogUploadState`, LF-15; Rust and TS in one commit): `Sent { lines_sent, lines_total, upload_id }`, `Partial { lines_sent, lines_total, upload_id, retry_after_secs }`, `Unreachable`. `NothingNew` and `NoLogs` stay. `Disabled`, `Uploaded` and `UploadedNewest` stay until LF-18.

## Wave 3: server (PR U3)

All three: `Host: windows`, **Rules:** R1-R10, **Checks:** S1, S2, then the S3 filters named, then S3 `telemetry` once at the end. After the wave review, `network-security-auth` reviews the wave diff against R1-R16 and C-LF4..C-LF6 and writes `worknotes/wave3-security.md`, with notes numbered `A<k>` and a verdict `ship` or `fix`.

### LF-10: `purpose = "logs"` for launcher sessions

- **Agent:** packet-coder. **Worktree:** `lf-10`, branch `launcher-uat-fixes/lf-10-log-purpose`, from the `lf-w3` tip. **Contract:** C-LF4.
- **Files:** `T/dev_session/launcher_kind.rs` (235 lines), the dev-session request and claims types (find the mint request struct and the token claims struct the launcher kind uses; name them in the report), and their test files. At most 5 files; more → `blocked`.
- **Do:** add `purpose: Option<String>` to the mint request (`#[serde(default)]`) and to the claims (`#[serde(default, skip_serializing_if = "Option::is_none")]`). Validate as C-LF4. Add `fn is_log_session(&self) -> bool` on the claims. Do not change any limit here.
- **Tests (unit):** `logs` + launcher → 200 with the claim; `logs` + another kind → 400 `purpose_kind`; `"x"` → 400 `purpose_unknown`; a claims JSON without `purpose` still decodes as an event session; a `logs` mint counts toward the per-install mint table (the 13th mint in the hour is refused). Revert proof: `logs` + another kind is accepted.
- **Commit:** `feat(admin-api): launcher sessions can be minted for log upload (LF-10)`

### LF-11: log row rules, per-word scrub, log-targets file

- **Agent:** packet-coder. **Worktree:** `lf-11`, branch `launcher-uat-fixes/lf-11-log-rows`, from the `lf-w3` tip. **Contract:** C-LF5.
- **Files:** `T/launcher_rows.rs` (262 lines: make `scrub_value` `pub(super)`, nothing else), new `T/launcher_log_rows.rs`, new `T/launcher_log_rows_tests.rs`, `T/mod.rs` (one `mod` line), new `crates/launcher-core/src/telemetry/launcher_log_targets.txt` (two lines, `client.launcher.log` then `client.launcher.game_log`, LF line endings, like `launcher_targets.txt`).
- **Do:**
  1. `pub(super) const LAUNCHER_LOG_TARGETS: [&str; 2] = ["client.launcher.log", "client.launcher.game_log"];`
  2. `pub(super) fn scrub_words(message: &str) -> String`: split on whitespace, replace each word that `scrub_value` flags with `<removed>`, join with one space, then cut to 2,048 bytes on a char boundary and append `...[truncated]` when cut.
  3. `pub(super) fn admit_log_rows(rows: Vec<ClientNativeEvent>) -> (Vec<ClientNativeEvent>, u64 /*dropped*/)` mirroring `admit_launcher_rows` (`:75`): exact target match, level forced to `info`, fields per C-LF5 (unknown key or bad value → drop the row), `message` through `scrub_words`. Copy the shape of `clean_fields` (`:112`); do not change it.
  4. A golden test that `LAUNCHER_LOG_TARGETS` equals the lines of `launcher_log_targets.txt` (read it with `include_str!` by relative path, as the existing target golden at `launcher_rows_tests.rs:204` does). Leave the existing three target goldens untouched.
- **Tests (unit):** `C:\Users\someone\x.txt failed` → `<removed> failed`; `connect 10.0.0.1:443 refused` → the address word removed; a 3,000-byte message → 2,048 + suffix; a `client.launcher.game_log` row with a chat line (`[Say] Bob: hello`) is kept unchanged, valid UTF-8; missing `upload_id` → dropped; `upload_id` with uppercase → dropped; an event target (`client.launcher.adoption` or any line of `launcher_targets.txt`) → dropped; `log_level = "ERROR"` kept and `level` forced to `info`; a `file` with `/` → dropped. Revert proof: the module is new; the scrub test pins per-word (not whole-value) replacement.
- **Checks:** S1, S2, S3 `launcher_log_rows`, S3 `launcher_rows`.
- **Commit:** `feat(admin-api): row rules and per-word scrub for launcher log rows (LF-11)`

### LF-12: log-session limits, budgets and replay body

- **Agent:** rust-gameserver-dev. **Worktree:** `lf-12`, branch `launcher-uat-fixes/lf-12-log-budgets`, from the `lf-w3` tip after LF-10 and LF-11 merged in. **Contract:** C-LF4, C-LF6.
- **Files:** `T/chunk.rs` (405, at `:118-123`, `:145-149`, `:184-195`), `T/launcher_gate.rs` (151; add `UploadLimits::launcher_logs()` beside `:140-150`), `T/launcher_budget.rs` (217) or a new `T/launcher_log_budget.rs` if it would pass 400 lines, `T/upload_gate.rs` (460: call lines only), `T/replay_native.rs` (382, `:272`, `:336-337`), and their test files.
- **Do:**
  1. In `chunk.rs`, take the claims' `is_log_session()` **before** reading the body: a log session uses `UploadLimits::launcher_logs()` and `admit_log_rows`, an event session today's path. A target from the other list → 403 `purpose_target` (C-LF4).
  2. Budgets per C-LF6, with the env overrides read the way `CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_SESSION` is. Whole-chunk admission: check the chunk's row count against the session, install-day and global budgets, then charge all three, or refuse with 429/503 and `Retry-After`. The per-install-day ledger keys by the install id the gate already uses and resets at UTC midnight; it holds at most 4,096 installs (evict the oldest day).
  3. Log sessions get 20 chunks per session per minute in the gate; event sessions keep 10.
  4. `replay_native.rs`: when the row's target is in `LAUNCHER_LOG_TARGETS`, the body is the `message` field and `message` is not repeated in the attributes JSON. Everything else unchanged.
  5. Every refusal logs the existing telemetry-route event with a closed `reason` (`log_session_budget`, `log_install_budget`, `log_ip_budget`, `log_global_budget`, `purpose_target`). No install id, IP or token in a field (R5).
- **Tests (unit):** a 1,000-row log chunk is admitted and a 1,001-row one is a 413; the 21st chunk of 1,000 in one session → 429 and no row of it is written; install-day refusal carries `Retry-After` up to midnight; the env override changes the session budget; an event token posting `client.launcher.log` → 403; a log token posting `client.launcher.adoption` → 403; an event session's 300-row budget is unchanged; the replayed log row's body is its message. Revert proof: the log chunk hits the 200-row event limit.
- **Checks:** S1, S2, S3 `chunk`, S3 `launcher`, S3 `replay_native`, S3 `telemetry`.
- **Docs:** none (LF-17 writes the operations rows).
- **Commit:** `feat(admin-api): log-session limits, budgets and replay body (LF-12)`

**After PR U3 merges:** the owner deploys the colo (ask first). Wave 4 can be integrated before that, but PR U4 is not merged before the deploy.

## Wave 4: launcher (PR U4, stacked on U3)

### LF-13: log line reader and offsets

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-13`, branch `launcher-uat-fixes/lf-13-log-lines`, from the `lf-w4` tip. **Rules:** R1-R10. **Contract:** C-LF7 (`lines.rs`, `offsets.rs`).
- **Files:** new `E/log_upload/lines.rs`, new `E/log_upload/offsets.rs`, new `E/log_upload/lines_tests.rs`, `E/log_upload/mod.rs` (287 lines; `mod` lines only).
- **Do:**
  1. Read the shell's file appender format (`crates/launcher/desktop/shell/src/telemetry.rs:28-29`, `:66-72`) and the existing `launcher_logs.rs` (which files, the 4 MiB cap, `scrub_ips(redaction.scrub(line))`). `collect` reads the same `launcher.*.log` files, oldest file first. Do not delete or change `launcher_logs.rs`.
  2. Per file: start at `offsets.files[name]`, or 0 when absent or when the file is now shorter than the offset. Read only up to the last `\n`. A line that does not start with the appender's timestamp is folded into the previous line (joined with `" | "`). Parse the timestamp, level and target when the line matches the appender format; otherwise keep the whole line as `message`. Run the same scrub `launcher_logs.rs` runs. Cut a message over 2,048 bytes on a char boundary and set `truncated`. `end` is the byte offset just past the line's `\n`.
  3. Keep at most the newest `MAX_LINES` lines across all files; `lines_total` counts every complete new line.
  4. `Offsets`: JSON, atomic write (temp + rename, as `store.rs`'s ledger does), `version = 1`; a file that fails to parse loads as empty. `advance` sets the line's file to `max(current, line.end)`.
- **Tests (unit, temp folders):** a second `collect` after `advance` + `save` returns only lines appended since; a half-written last line is not returned until its `\n` arrives; a file shorter than its offset restarts at 0; a continuation line folds into its parent; 25,000 lines → 20,000 newest returned and `lines_total = 25_000`; a home-folder path in a line is scrubbed. Revert proof: the module is new; the tests pin the offset and fold rules.
- **Checks:** D1, D2e, D3e `log_upload`.
- **Commit:** `feat(launcher): incremental launcher-log line reader (LF-13)`

### LF-14: log rows and the log sender

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `lf-14`, branch `launcher-uat-fixes/lf-14-log-sender`, from the `lf-w4` tip. **Rules:** R1-R10. **Contract:** C-LF4, C-LF5, C-LF7 (`ingest.rs`).
- **Files:** new `E/log_upload/ingest.rs`, new `E/log_upload/ingest_tests.rs`, `E/game_telemetry/launcher_session.rs` (294), `E/game_telemetry/session.rs` (502: call-line changes only; `start_kind` at `:153`), the launcher mint request type it builds, and `E/log_upload/mod.rs` (`mod` line).
- **Do:**
  1. `rows`: one `ClientNativeEvent` per line, C-LF5 fields, `seq` from 0, target `client.launcher.log`. Add `pub(crate) const LOG_UPLOAD_TARGETS: [&str; 2]` (both lines; LF-19 is the first to send `game_log`) and a golden test. `rows` takes the target as an argument against `launcher_log_targets.txt` (as `launcher_event.rs:233` does for the event list).
  2. Mint with `purpose = "logs"`: give `start_kind` (or the launcher mint it calls) a `purpose: Option<&str>` argument; every existing caller passes `None`. The log sender holds its own session; it never reuses the event session's token.
  3. `send_logs(target, rows, pacer: &mut impl Pacer, on_chunk_sent: impl FnMut(usize /*rows*/))`: `split_batches(rows, 384 * 1024 /* uncompressed NDJSON bytes, under the server's 512 KiB cap */, 1_000)`, send in order, call `on_chunk_sent` after each 2xx, stop at the first 429/503 → `Partial { retry_after_secs = max(60, Retry-After) }` (R16). After encoding, a batch whose gzip is over 64 KiB is halved and re-encoded until it fits (a one-row batch that still does not fit is dropped and counted). 401 re-mints once. Any other failure → `Failed`. No backoff loop inside one upload.
  4. **Pacing (wave 3 ruling D1; the server's 20 chunks per minute stay).** `pub(crate) const MIN_CHUNK_INTERVAL_MS: u64 = 3_500;` Before every chunk after the first, sleep `MIN_CHUNK_INTERVAL_MS` minus the time since the previous chunk's send started (no sleep when that is already past). That gives at most 18 chunks in any 60 s window, leaving the install's event session headroom in the shared per-install window. One `Pacer` per upload: LF-19's game rows reuse the same pacer, so the interval also holds across the launcher-to-game switch. The seam: `pub(crate) trait Pacer { fn now_ms(&self) -> u64; fn sleep_ms(&mut self, ms: u64) -> impl Future<Output = ()> + Send; }`; production `TokioPacer` uses `tokio::time::Instant` and `tokio::time::sleep`; tests use a `FakePacer` that records each requested sleep and advances its own clock by it (and by a per-send delay the test sets). No real sleep in any test.
  5. Log one INFO event at the start (`event = "log_upload_started"`, `lines`) and one at the end (`event = "log_upload_finished"`, `outcome`, `lines_sent`, `chunks`), on the existing log-upload target. Neither carries the upload id (it is a correlation key for the rows only).
- **Tests (unit, the existing launcher-session HTTP test double):** the mint body carries `purpose: "logs"`; 2,500 rows go as 3 chunks with `on_chunk_sent` called 3 times; a 429 on chunk 2 → `Partial { lines_sent: 1000 }` and chunk 3 is not sent; a 1,000-row batch of 600-byte messages is split below 512 KiB uncompressed, and every gzip body is at most 64 KiB; a batch of 1,000 rows whose messages are pseudo-random hex (about 300 bytes each, so a 384 KiB batch gzips to well over 64 KiB) is halved until every gzip body is at most 64 KiB, and all 1,000 rows are still sent; a single 4,096-byte incompressible row is dropped and counted; event-session sends are unchanged (their existing tests pass untouched). Pacing, with `FakePacer` only: 40 chunks → no sleep before chunk 1, consecutive send starts at least 3,500 ms apart on the fake clock, no 60,000 ms window holds more than 18 sends, total fake time at least 39 × 3,500 ms; a send that took 5,000 ms (per-send delay) makes the next requested sleep 0; a 429 on chunk 21 of 40 → `Partial` with 20 chunks' lines and no sleep requested after it. Revert proofs: the mint body lacks `purpose`; with the pacing sleep removed the window test fails; with the halving removed the incompressible-batch test fails.
- **Checks:** D1, D2e, D3e `log_upload`, D3e `launcher_session`, D3e `game_telemetry`.
- **Then:** `network-security-auth` reviews the LF-14 diff against R1-R16 (`worknotes/LF-14-security.md`, notes `A<k>`). Its remit is D-LU4: harden, never remove data.
- **Commit:** `feat(launcher): send launcher log lines through a log session (LF-14)`

### LF-15: upload state machine, shell command and TS schema

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `lf-15`, branch `launcher-uat-fixes/lf-15-upload-flow`, from the `lf-w4` tip after LF-13 and LF-14 merged in. **Rules:** R1-R10. **Contract:** C-LF7; G3, G4.
- **D-LU1 = yes (owner, 2026-10-22):** the press consents for that one upload, so availability ignores `game_telemetry().opted_in`. Test: with diagnostics off, an upload still sends.
- **Files:** `E/log_upload/mod.rs` (`begin_log_upload` `:173`, `finish_log_upload` `:210`), `E/log_upload/events.rs`, `E/log_upload/tests.rs` (501 lines: put new tests in a new `E/log_upload/ingest_flow_tests.rs`), `E/mod.rs` or wherever `launcher_target()` lives (`:41-51`), `shell/src/host/log_upload.rs`, `shell/src/main.rs` (`:266-295`, call lines), `FE/src/contract.ts` or the log-upload schema file.
- **Do:**
  1. `Upload` no longer calls `ensure_no_active_operation`. A second upload while one runs → the existing `busy` state.
  2. Availability: `Inspect` returns `Unreachable` only when no launcher telemetry target can be built for this install; the diagnostics opt-in does not matter (D-LU1). `Disabled` is no longer returned; the blob store is not called.
  3. `Upload`: `upload_id` = 16 random lowercase hex. `collect` → empty → `NothingNew` (or `NoLogs` when there are no log files) → `rows` → `send_logs`, advancing and saving `Offsets` after each chunk sent. Map the outcome to `Sent` / `Partial` / `Failed` (an existing `LogUploadError::Network` or `Rejected`).
  4. Shell and TS: the three new states with their fields (R8).
  5. A paced upload runs for minutes (LF-14 pacing: about 3.5 s per chunk). Pass a `TokioPacer`; put no timeout under 15 minutes around `Upload` in the engine, the shell command or the TS call, and keep the `busy` state for the whole send.
- **Tests (unit):** an upload during `ReconciliationRequired` sends (fails on revert: `busy`); with no telemetry target → `Unreachable`, never `Disabled`; `Partial` keeps the unsent lines for the next press (offsets stop at the last chunk sent); a second press after `Sent` → `NothingNew`; the TS decoder accepts each new state (F1).
- **Checks:** D1, D2e, D2s, D3e `log_upload`, D3s `log_upload`, F1.
- **Commit:** `feat(launcher): Upload debug logs ingests lines and works during recovery (LF-15)`

### LF-16: debug-logs view copy, first press, tests and UAT

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-16`, branch `launcher-uat-fixes/lf-16-log-view`, from the `lf-w4` tip after LF-15 merged in. **Rules:** R1-R4, R8, R9. **Check:** F1.
- **Files:** `FE/src/log-upload-view.ts` (75), `FE/src/log-upload-view.test.ts`, `FE/log-upload-uat.mjs`.
- **Do:** the button shows `Sending debug logs… this can take a few minutes.` and disables itself on the first press (LF-14 paces chunks at about 3.5 s each, so a full backlog takes 2-4 minutes). The texts:
  - `Sent`: `Sent {lines_sent} log lines. Reference {upload_id}.`
  - `Partial`: `Sent {lines_sent} of {lines_total} lines. The server asked us to wait; press again in about {ceil(retry_after_secs/60)} min.`
  - `NothingNew`: `No new log lines since the last upload.`
  - `NoLogs`: keep today's text.
  - `Unreachable`: the button stays visible but disabled, with `Debug logs can't be sent: this install has no telemetry server.`.
  - No state leaves the "Debug logs" heading with an empty line under it (G4).
  - One line under the button says what is sent: `Sends the launcher's and the game's recent log lines (file paths and addresses removed; game logs can include chat) to the project's diagnostics server. Pressing it sends logs even when Send game diagnostics is off.`
- **Tests:** each state's text; the first press shows the busy text before the promise settles; `Unreachable` keeps the button visible. Update `log-upload-uat.mjs` to walk Inspect → Upload → Sent → second press → NothingNew, and paste its output in the report.
- **Commit:** `feat(launcher): debug-logs panel reports lines sent and a reference (LF-16)`

### LF-17: docs for log ingest and the adoption fixes

- **Host:** windows. **Agent:** documentation-writer. **Worktree:** `lf-17`, branch `launcher-uat-fixes/lf-17-docs`, from the `lf-w4` tip after LF-19 merged in. **Rules:** R2, R4, R9.
- **Files:** `docs/operations/telemetry.md` (a "Launcher log ingest" section: both targets (`client.launcher.log`, `client.launcher.game_log`, unfiltered per D-LU3), C-LF5 fields, C-LF6 budgets and env vars (all five, including `CIMMERIA_TELEMETRY_LOG_ROWS_PER_IP_DAY`), the refusal reasons `log_session_budget`, `log_install_budget`, `log_ip_budget`, `log_global_budget`, `purpose_target`; a log token refuses launcher event targets with 403 `purpose_target` and drops any other unknown target; the launcher paces log chunks at one per 3.5 s; the SigNoz filter `upload_id = <ref>`), `docs/client/launcher-guide.md` (`:956-967`: the new Debug logs behaviour; the adoption Recover/Abandon after an interrupted light adoption, LF-02/LF-03; non-ASCII stock names, LF-04), `crates/launcher/desktop/README.md` (`:110-128`: the blob path is legacy until LF-18), `docs/client/sgw-launcher.md` (`:484-516`: one note at the top of the section that it describes the legacy launcher, G7), `docs/client/launcher-distribution-setup.md` (`:26-30`, `:338-392`: `LAUNCHER_LOG_SAS_URL` is unused by the desktop launcher's upload and is removed by LF-18). Walk `docs/agents/doc-update-map.md` and list the rows in the report. CRLF.
- **Commit:** `docs(launcher): log ingest, light-adoption recovery and stock names (LF-17)`

### LF-19: game-log reader and `client.launcher.game_log` rows (D-LU3: unfiltered)

- **Host:** windows. **Agent:** rust-gameserver-dev. **Worktree:** `lf-19`, branch `launcher-uat-fixes/lf-19-game-logs`, from the `lf-w4` tip after LF-15 merged in. **Rules:** R1-R10. **Contract:** C-LF5, C-LF7. **D-LU3 (owner, 2026-10-22):** no content filter. Chat and character names are acceptable; only the server's per-word scrub applies.
- **Files:** new `E/log_upload/game_lines.rs`, new `E/log_upload/game_lines_tests.rs`, `E/log_upload/offsets.rs` (LF-13), `E/log_upload/mod.rs` (the `Upload` arm LF-15 wrote; `mod` line), `crates/launcher-core/src/logs.rs` (make the file discovery `collect_log_inputs` `pub`, nothing else).
- **Do:**
  1. Discover files through `launcher-core` `collect_log_inputs(<game>)`: `sgwdebuglog*` in the binaries folder and `sessions/**/*.log`. Session key dumps and `current-session.json` stay excluded, as today.
  2. Decode: a UTF-16LE BOM (`FF FE`) or a file whose first 512 bytes are more than 30% NUL at odd offsets is UTF-16LE; otherwise UTF-8, lossy. Track offsets in **bytes of the file**, so a UTF-16 line's `end` is on a 2-byte boundary. Before writing, confirm the encoding on one real `sgwdebuglog` from the QA client tree (`..\SGW\Stargate Worlds-QA\Working\Binaries`, read only). Paste its first 64 bytes in hex in the report.
  3. Lines: split on `\n`, strip `\r`, and drop empty lines. Read only up to the last complete line. There is no timestamp parse unless the line starts with one the reader can prove from the sample: `ts_ms` is then the read time, and `level`/`target` stay `None`. Cut at 2,048 bytes with `truncated`. Run the same local scrub as `launcher_logs.rs`. No other filtering.
  4. Offsets: key game files as `game/<path relative to the binaries folder, '/'-separated>`. Bump `Offsets` to `version = 2` with `heads: BTreeMap<String, String>`, the `h12` of the file's first 256 bytes. When a file's head changes (the client rewrote `sgwdebuglog` at its next launch), restart that file at 0. A version-1 file loads with empty `heads`.
  5. Budget: up to `MAX_LINES` (20,000) newest game lines **in addition to** the 20,000 launcher lines. `rows(..)` gets target `client.launcher.game_log` and `file` = the bare file name (C-LF5 regex; replace other characters with `_`).
  6. In `Upload`: send launcher rows first, then game rows, in one log session with one `upload_id` and the same `Pacer` (LF-14 item 4; the 3.5 s interval holds across the switch). A `Partial` on the launcher rows sends no game rows. Offsets advance per chunk as in LF-15. `lines_total` and `lines_sent` count both.
- **Tests (unit, temp folders):**
  - a UTF-16LE fixture with BOM decodes and resumes at the right byte offset;
  - a rewritten file with a different head restarts at 0;
  - `sessions/x/a-keys.txt` and `current-session.json` are never read;
  - a chat line `[Say] Bob: hello` is sent unchanged;
  - one upload with both kinds carries one `upload_id` and both targets;
  - a version-1 offsets file still loads.
  - Revert proof: without the head check, the rewritten-file test sends nothing.
- **Checks:** D1, D2e, D3e `log_upload`, C1, C2, C3 `logs`.
- **Then:** `network-security-auth` reviews the LF-19 diff (`worknotes/LF-19-security.md`, notes `A<k>`). Its remit is D-LU4: harden, never remove data.
- **Commit:** `feat(launcher): ingest the game's debug and session logs (LF-19)`
## Wave 5: retire the blob path

### LF-18: retire the blob upload path (D-LU2 = retire, decided)

- **Host:** windows. **Agent:** packet-coder. **Worktree:** `lf-18`, branch `launcher-uat-fixes/lf-18-retire-blob`, from `origin/main` after PR U4 merges. **Rules:** R1-R10. **Checks:** D1, D2e, D2s, D3e `log_upload`, D3s `log_upload`, C1, C2, C3 `logs`, F1.
- **Files:** `E/log_upload/store.rs` (delete), `E/log_upload/mod.rs`, `E/log_upload/tests.rs`, the zip builder and `uploaded-logs.json` ledger (`launcher-core` `state.rs`, `UploadedLedger`; delete only those items), the `Disabled`/`Uploaded`/`UploadedNewest` variants in Rust and TS, `.github/workflows/launcher-desktop-macos.yml` (`:67-68`, `:148`), `.github/workflows/launcher-release.yml` (`:155-156`, `:544`), `tools/launcher-release/build-desktop-macos.sh` (`:144`), `crates/launcher/desktop/docs/release.md` (`:60`, `:62`, `:249`), and the doc lines LF-17 marked legacy. Keep the game-log file discovery (`collect_log_inputs` / `select_log_inputs` in `launcher-core/src/logs.rs`): LF-19 reads through it. Delete only the zip, digest, PUT and SAS code around it.
- **Do:** remove the blob path, the build-time `LAUNCHER_LOG_SAS_URL` and its ledger. `uploaded-logs.json` on disk is left alone (never read again). The GitHub secret is deleted by the owner, not by this packet; list it in the report.
- **Tests:** existing tests updated; `rg LAUNCHER_LOG_SAS_URL` returns only historical `docs/analysis/` lines.
- **Commit:** `refactor(launcher): retire the blob log upload (LF-18)`


## Close-out

### LF-20: UAT rows, status docs, retire

- **Host:** windows. **Agent:** documentation-writer. **Worktree:** `lf-20`, branch `launcher-uat-fixes/close-out`, from `origin/main` after U1-U4 (and U5 if decided) merge. **Rules:** R2, R4, R9.
- **Do:** add the README's owner-UAT rows U1-U6 to `docs/guides/unified-uat.md`. Update `docs/project-status.md` and the launcher row of `docs/gap-analysis*` once. Point `docs/analysis/launcher-telemetry/RESUME.md` at this campaign for F1-F6 and G1-G4. Set every packet row to its final status, and record the merged PR numbers in the README.
- **Commit:** `docs: launcher-uat-fixes close-out (LF-20)`
