# Launcher adopt UX: wave 2 work packets (launcher telemetry)

Ledger: [README.md](README.md) (D-AU3, D-AU13, D-AU14, D-AU16..D-AU28, § Security review, OQ-8). Resume card: [RESUME.md](RESUME.md). Wave 1 and the standard checks: [work-packets.md](work-packets.md). Designed 2026-10-20 at `0d71b3b30`. **Redesigned 2026-10-20** after the owner decided OQ-4, OQ-5 and OQ-6 (opt-in, off by default; a `launcher` session kind, so adoption events leave at once without Play). The network-security-auth review ([worknotes/security-review-launcher-session.md](worknotes/security-review-launcher-session.md), "acceptable with changes") is folded in. Its required items are cited as `R<n>` and its recommendations as `Rec<n>`.

## Dispatch rules

- The rules in [work-packets.md § Dispatch rules](work-packets.md#dispatch-rules) apply, with the differences below.
- **Wave 2 ships as two PRs** (D-AU23):
  - **2a, server.** Packets AU-11, AU-12, AU-14, AU-15 and AU-16, from the integration worktree `au-srv-integration` (branch `launcher-adopt-ux/launcher-session`). **The PR is opened and held. It merges only after a maintainer approves the new session kind (OQ-8).**
  - **2b, launcher.** Packets AU-07, AU-10, AU-13, AU-08 and AU-09, from `au-tel-integration` (branch `launcher-adopt-ux/telemetry`). It merges after 2a.
  - `ops` creates both worktrees with `mk-worktree.ps1` from `origin/main`. It merges each packet branch in with `merge-noff.ps1 -C <integration worktree> -Branch <packet branch>`, in the order given in § Waves.
- Worktrees: `au-07`, `au-08`, `au-09`, `au-10`, `au-11`, `au-12`, `au-13`, `au-14`, `au-15` and `au-16`. When each PR merges, `ops` retires that PR's packet worktrees with `rm-worktree.ps1 --merged-into <PR> <names> <integration worktree>`.
- Report path: `<worktree>\.claude\reports\launcher-adopt-ux\<packet>-<role>.md`. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit messages end with the two trailer lines given in work-packets.md.
- No packet uses the lab, a live client, a live server or SigNoz. The SigNoz check is an owner step listed in the 2b PR body (§ Waves).
- No packet changes the client wire, a protocol opcode or `db/scripts`. The only server change is the `launcher` kind on the existing routes `/api/auth/dev-session`, `/api/auth/dev-session/refresh` (refused), `/api/telemetry/upload-chunk` and `/api/telemetry/upload-bundle` (refused).

### Extra checks (on top of D1, D2e, D2s, D3s, F1 and F2 in work-packets.md)

Every command starts with `Set-Location <worktree>;` in the same PowerShell call, runs in the foreground, and is waited for.

| ID | Command |
|---|---|
| L0 | Lock update, run once and **only in AU-08**: `pwsh tools/build-lane/lane.ps1 cargo check --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop` (no `--locked`). Then run `git diff --stat crates/launcher/desktop/Cargo.lock`. Only added packages are allowed; if an existing package changed version, report `blocked` and include the diff. |
| D3g | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine game_telemetry` |
| D3t | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop telemetry` |
| G2 | The game-diagnostics logic UAT, in two calls. (1) Build as in F2 step 1, and note the `Executable unittests src/main.rs (<path>)` path it prints. (2) `Set-Location <wt>; $env:GAME_TELEMETRY_UAT_BINARY='<path>'; npm run uat:game-telemetry --prefix crates/launcher/desktop/frontend`. It must print `Game diagnostics logic UAT passed`. |
| S1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-admin-api -- --check` |
| S2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-admin-api --all-targets -- -D warnings` |
| S3d | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api --lib routes::dev_session` |
| S3t | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api --lib routes::telemetry` |
| S4 | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api` (integration worktree only) |

---

## Contract (wave 2)

Launcher paths are under `crates/launcher/desktop/` and server paths under `crates/admin-api/src/routes/`. Build against these names exactly.

### C8. Engine: scrub and build launcher events (AU-07), new `engine/src/storage/game_telemetry/launcher_event.rs`

```rust
/// Target of every forwarded launcher log line (SigNoz `client_target`).
pub const LAUNCHER_EVENT_TARGET: &str = "client.launcher.adoption";
/// A string field is cut to this many bytes (on a char boundary) and "..." is appended.
pub const LAUNCHER_FIELD_MAX: usize = 240;

/// One forwarded launcher log line, before it is sent.
#[derive(Debug, Clone, PartialEq)]
pub struct LauncherRecord {
    pub ts_ms: i64,
    /// "info", "warn" or "error".
    pub level: String,
    pub fields: serde_json::Map<String, serde_json::Value>,
}

/// What to take out of every string before it leaves the process or reaches the log file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Redaction {
    pub home: Option<String>,
    pub user: Option<String>,
}

impl Redaction {
    /// home: USERPROFILE, else HOME. user: USERNAME, else USER. Trimmed; empty or
    /// shorter than 2 characters is None.
    pub fn from_env() -> Self;
    /// Home folder -> `~`, then user-name path segments -> `<user>`.
    pub fn replace_home(&self, value: &str) -> String;
    /// `replace_home`, then `"<path removed>"` if an absolute path is still left.
    pub fn scrub(&self, value: &str) -> String;
}

/// One `TelemetryEvent::ClientNative` per record, scrubbed. Pure: no I/O, no consent check.
pub fn launcher_events(records: &[LauncherRecord], redaction: &Redaction) -> Vec<TelemetryEvent>;
```

- **Home.** `replace_home` replaces every copy of `home` with `~`. It looks for `home` in three forms, longest first: with each `\` doubled (Debug output), as given, and with `\` turned into `/`.
- **User name.** `replace_home` then splits the value into runs, separated by `/`, `\`, `"`, `'`, `:`, `;` or whitespace. A run equal to `user` (ignoring ASCII case) becomes `<user>`. The separators are kept as they are.
- **Absolute path (used by `scrub`).** A value holds an absolute path if (a) it contains an ASCII letter followed by `:\` or `:/`, or (b) after removing one leading `"`, it starts with `\\` or `/`.
- **The event.** `TelemetryEvent::ClientNative(ClientNativeEvent { ts_ms, seq, target: LAUNCHER_EVENT_TARGET.into(), level, fields })`.
  - `level` is lower-cased.
  - Each string field goes through `scrub`. Then, if it is longer than `LAUNCHER_FIELD_MAX` bytes, it is cut there (on a char boundary) and `...` is appended. Other JSON values pass through unchanged.
  - Two fields are added: `"launcher_version": super::session::mint_launcher_version()` and `"os": std::env::consts::OS`.
  - `seq` comes from one process-wide `static SEQ: AtomicU64` (`fetch_add(1, Relaxed)`).
- The server scrubs again (C14). The client scrub is defence in depth (Rec6).

### C11. Engine: the launcher session sender (AU-13), new `engine/src/storage/game_telemetry/launcher_session.rs`

```rust
/// The `session_kind` a launcher session is minted with (server C12).
pub const LAUNCHER_SESSION_KIND: &str = "launcher";
/// First hold after a failed mint; it doubles per failure in a row, up to MAX_HOLD.
pub const MINT_HOLD: std::time::Duration = std::time::Duration::from_secs(60);
/// The shortest hold after a 429 or 503 (R16).
pub const MIN_RETRY_HOLD: std::time::Duration = std::time::Duration::from_secs(60);
/// The longest hold of any kind.
pub const MAX_HOLD: std::time::Duration = std::time::Duration::from_secs(3_600);
/// Rows per chunk; the server keeps at most this many (C14).
pub const LAUNCHER_MAX_ROWS: usize = 200;
/// Re-mint when the token has less than this left.
pub const REMINT_MARGIN_MS: i64 = 60_000;

#[derive(Debug, Clone)]
pub struct LauncherTarget {
    pub identity: Identity,
    pub servers: Vec<LoginServer>,
}

impl DesktopState {
    /// `Some` only while the player is opted in to game diagnostics and has an
    /// identity. `servers` is the saved login-server preference, or the defaults
    /// when it is empty, validated. Never mints, never sends.
    pub fn launcher_target(&self) -> Option<LauncherTarget>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendOutcome { Empty, Sent, Held, MintFailed, Dropped }

pub struct LauncherSender { /* http: reqwest::Client, session: Mutex<Option<Session>>,
                               hold_until: Mutex<Option<Instant>>, mint_failures: Mutex<u32> */ }

impl LauncherSender {
    pub fn new() -> Self;
    /// Send `events` now, in chunks. Never panics, never writes to disk, never
    /// logs a field value, never logs at target `adoption`.
    pub async fn send(&self, target: &LauncherTarget, events: &[TelemetryEvent]) -> SendOutcome;
    #[cfg(test)] pub fn hold_remaining(&self) -> Option<std::time::Duration>;
    #[cfg(test)] pub fn clear_hold(&self);
}
```

`session.rs` gains `pub(super) async fn start_kind(identity, login_servers, tags, session_kind: Option<&str>) -> Result<Session, Outcome>`, and `start` becomes `start_kind(.., None)`. `MintRequest` gains `#[serde(skip_serializing_if = "Option::is_none")] session_kind: Option<&'a str>`, so a player mint body stays byte-identical to today's.

**`send`, in order.**
1. If `events` is empty, return `Empty`.
2. If `hold_until` is in the future, return `Held`. The events are dropped.
3. Mint when there is no session, or when `expires_at_ms - now_ms < REMINT_MARGIN_MS`. Call `session::start_kind(&target.identity, &target.servers, &["desktop-launcher", std::env::consts::OS, "launcher"], Some(LAUNCHER_SESSION_KIND))`.
   - On `Err`: add 1 to `mint_failures`, hold for `min(MINT_HOLD * 2^(mint_failures-1), MAX_HOLD)`, and return `MintFailed`.
   - On `Ok`: set `mint_failures` to 0.
4. Split the events with `cimmeria_launcher_core::telemetry::chunk::split_batches(events, chunk_max_bytes, LAUNCHER_MAX_ROWS)`. Post each batch with `cimmeria_launcher_core::telemetry::chunk::post_chunk(&http, &upload_endpoint, &token, batch)`.
5. On `Err(ChunkError::TokenRejected)` (401) **only**: forget the session, mint once more (step 3), and post that batch once more. If that fails too, return `Dropped`.
6. On `KillSwitch` or `RateLimited` (503 or 429): hold for `max(MIN_RETRY_HOLD, retry_after_secs() or 60 s)`, capped at `MAX_HOLD`, and return `Dropped`.
7. On any other error (400, 403, 413, other status): return `Dropped`. Do not re-mint, do not retry, and do not hold.
8. When every batch has posted, return `Sent`.

The `Mutex`es are `std::sync::Mutex`, and none is held across an `.await`: clone the `Session` out, mint or post, then store the result.

### C9. Shell: log file, forward layer, forwarder (AU-08), new `shell/src/telemetry.rs`

```rust
pub(crate) const FORWARD_TARGET: &str = "adoption";
pub(crate) const LOG_FILE_PREFIX: &str = "launcher";   // launcher.<yyyy-mm-dd>.log
pub(crate) const LOG_FILES_KEPT: usize = 7;
pub(crate) const FORWARD_CAPACITY: usize = 512;       // sync_channel bound
pub(crate) const FORWARD_BATCH: usize = 64;

pub(crate) struct Telemetry {
    pub guard: Option<tracing_appender::non_blocking::WorkerGuard>,
    pub records: std::sync::mpsc::Receiver<LauncherRecord>,
}
pub(crate) fn init(log_dir: &std::path::Path, redaction: Redaction) -> Telemetry;
pub(crate) fn file_layer<S>(log_dir: &std::path::Path, redaction: Redaction)
    -> Option<(impl tracing_subscriber::Layer<S>, tracing_appender::non_blocking::WorkerGuard)>
where S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>;
pub(crate) struct ForwardLayer { /* tx: SyncSender<LauncherRecord> */ }
impl ForwardLayer { pub(crate) fn new(tx: std::sync::mpsc::SyncSender<LauncherRecord>) -> Self; }
pub(crate) struct Redacting<W> { pub inner: W, pub redaction: Redaction } // io::Write
pub(crate) fn spawn_forwarder(host: std::sync::Arc<crate::host::NativeHost>,
    records: std::sync::mpsc::Receiver<LauncherRecord>, redaction: Redaction);
```

New `shell/src/host/launcher_telemetry.rs`:

```rust
impl NativeHost {
    /// The target and events for `records`, or None. Returns None unless this build
    /// bundles the telemetry DLL (`launch_resources…client_telemetry.is_some()`, read
    /// as `host/game_telemetry.rs` line 35 does) and `launcher_target()` is Some.
    /// Holds the state lock only to read. Never sends.
    pub(crate) fn launcher_batch(&self, records: &[LauncherRecord], redaction: &Redaction)
        -> Option<(LauncherTarget, Vec<TelemetryEvent>)>;
}
```

- **File filter** (`tracing_subscriber::filter::Targets`). The default level is WARN. INFO applies to `cimmeria_launcher_engine`, `cimmeria_launcher_core`, the shell's own crate name (its package name in `shell/Cargo.toml`, with `-` written as `_`) and `adoption`.
- **Forward rule.** An event is forwarded only if `metadata.target() == FORWARD_TARGET` and `*metadata.level() <= Level::INFO` (INFO, WARN or ERROR).
- **Record fields.**
  - `level` is lowercase, and `ts_ms` is `SystemTime` now in milliseconds.
  - The fields map takes `record_str` as a string, `record_i64`, `record_u64`, `record_bool` and `record_f64` as JSON numbers or bools, and `record_debug` as `format!("{value:?}")`. The `message` field is included.
- **Sending.**
  - The layer uses `try_send`. A full channel drops the record, so the layer never blocks.
  - The forwarder thread owns one `LauncherSender`. For each batch it calls `host.launcher_batch(..)`, then `tauri::async_runtime::block_on(sender.send(&target, &events))`, and it ignores the outcome.

### C10. Adoption step events (AU-09), new `shell/src/host/adoption/events.rs`

Every event is `target: "adoption"` with the message `"adoption step"`, and every one carries `step` and `decision_outcome`. Every value is a fixed code or a relative path, never free text (Rec7).

| Function | Level | `step` | `decision_outcome` | Other fields | Called from |
|---|---|---|---|---|---|
| `mode_chosen(in_place: bool)` | INFO | `mode_chosen` | `in_place` or `copy` | none | `main.rs` `choose_existing_game_folder`, after the `adoption_mode_allowed` check passes |
| `folder_chosen(in_place: bool, selected: bool)` | INFO | `folder_chosen` | `selected` or `picker_cancelled` | `mode` | the same function: before each `return cancelled(host).await;` (`false`), and just before `host.begin_folder_adoption(...)` (`true`) |
| `review_ready()` | INFO | `review_ready` | `review_shown` | none | `status.rs` `settle_adoption`, preview `Ok` arm |
| `refused(error: AdoptionError, entries: Option<&adoption::UnsafeReport>)` | WARN | `refused` | `refused` | `error` (snake_case code); `unsafe_total` (u64, 0 when `None`); `unsafe_entries` (the first 5 entries as `reason:path` joined by `; `, with `(location)` for a `None` path; `""` when `None`) | `settle_adoption`, preview `Err` arm, after `adoption.fail(error)` |
| `confirmed(in_place: bool)` | INFO | `confirmed` | `confirm_sent` | `mode` | `mod.rs` `adoption_command`, Confirm arm, after `self.confirm_adoption(...)?` returns Ok |
| `finished(in_place: bool, error: Option<AdoptionError>)` | INFO, or WARN on error | `finished` | `adopted` or `failed` | `mode`; `error` only when failed | `settle_adoption`, the confirm-finished branch, after `fail`/`clear_failure` |

`mode` is `"in_place"` or `"copy"`. The codes come from serde: `serde_json::to_value(error)` is a string in `AdoptionError`'s snake_case, and `reason` comes from `UnsafeReason`'s snake_case in the same way.

### C12. Server: the `launcher` session kind on the mint and refresh routes (AU-11)

`dev_session/token.rs`:
- `pub const SESSION_KIND_LAUNCHER: &str = "launcher";`
- `pub const SCOPE_LAUNCHER_WRITE: &str = "telemetry.launcher.write";`
- `pub fn is_launcher(&self) -> bool`, true when `self.session_kind() == SESSION_KIND_LAUNCHER`.
- `pub fn kind_matches_scope(&self) -> bool`. It is true when the token is a launcher token whose `scope` is exactly `[SCOPE_LAUNCHER_WRITE]`, or a token of any other kind whose `scope` does not contain `SCOPE_LAUNCHER_WRITE`. Any other combination is false (R1).

New `dev_session/launcher_kind.rs`:

```rust
/// A launcher session lives 15 minutes and is never refreshed (R3, R4).
pub const LAUNCHER_TOKEN_TTL_SECONDS: i64 = 15 * 60;
/// The `chunk_max_bytes` a launcher mint replies with (NDJSON before gzip).
pub const LAUNCHER_CHUNK_MAX_BYTES: u64 = 65_536;
/// Launcher mints per address per quota window (1 h), on top of the shared per-IP table (R6).
pub const DEFAULT_LAUNCHER_MINT_PER_IP: u32 = 30;
pub const LAUNCHER_MINT_IP_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_IP";
/// Launcher mints per install_id per quota window, on their own table.
pub const DEFAULT_LAUNCHER_MINT_PER_INSTALL: u32 = 12;
pub const LAUNCHER_MINT_INSTALL_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_INSTALL";
/// "1" or "true" turns off the launcher kind only (mint and upload answer 503). Rec4.
pub const LAUNCHER_DISABLED_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_DISABLED";
pub fn launcher_disabled_from_env() -> bool;
/// Launcher mint metadata (branch, git_sha, launcher_version, tags) must be printable
/// ASCII (' '..='~'); anything else is BadField 400. Rec8.
pub(super) fn validate_launcher_metadata(req: &DevSessionRequest) -> Result<(), AuthError>;
```

| Mint request | Result |
|---|---|
| `session_kind` absent or `"player"` | Unchanged: no `kind` claim, 8 h, 1 MiB, scope `["telemetry.write"]`, stream recorded. |
| `"lab"` | Unchanged. |
| `"launcher"` | Claims are `iss`, `sub` = install_id, `sid`, `iat`, `exp = iat + 900`, `scope = ["telemetry.launcher.write"]` and `kind = "launcher"`. There is **no `acct` and no `mid`**. `machine_id` is ignored entirely: it is not validated, not signed, and not logged at any level, including the debug line at about `handlers.rs` 307. `record_stream` is **not** called. The reply has `chunk_max_bytes = 65_536`. |
| `"launcher"` with a non-blank `account_name` | 400 `BadField { field: "account_name", reason: "not allowed for a launcher session" }` |
| `"launcher"` while `launcher_disabled` | 503 `KillSwitchActive`, before any quota is charged. |
| any other value | 400 with the reason `must be "player", "lab" or "launcher"`. |

- **Order in `mint_inner`.**
  1. The master kill switch.
  2. For a launcher request: `launcher_disabled`.
  3. The shared per-IP quota (`mint_ip`).
  4. For a launcher request: the launcher per-IP quota (`Tables::mint_launcher_ip`, `QuotaPolicy::mint_launcher_per_ip`, scope `"mint/launcher_ip"`).
  5. Validation. A launcher request drops `machine_id` first and also runs `validate_launcher_metadata`.
  6. `kind_claim` and `account_claim`.
  7. The per-install quota. A launcher request charges `Tables::mint_launcher_install` against `QuotaPolicy::mint_launcher_per_install` (scope `"mint/launcher_install"`), never the player's `mint_install`.
- **`refresh_inner`.** A token that verifies and is a launcher token (`is_launcher()`, or a scope that contains `SCOPE_LAUNCHER_WRITE`) is refused with 400 `BadField { field: "session_kind", reason: "a launcher session is not refreshed; mint a new one" }`. The refusal happens before `refresh_ip` is charged (about line 354) and before any new claims are built (R3).

### C13. Server: launcher tokens at the upload gate (AU-12)

**Bearer check.** The new `verify_bearer_for(headers, route: Route) -> Result<TokenClaims, IngestError>` works as follows:
- It decodes and checks the token exactly as `verify_bearer` does today.
- If `!claims.kind_matches_scope()`, it returns `MissingScope { wanted: "telemetry.write" }` (401).
- `Route::Chunk` accepts the scope `telemetry.write` or `telemetry.launcher.write`.
- `Route::Bundle` with a launcher token returns the new `IngestError::Forbidden("a launcher session cannot upload a bundle")` (403, R2). Any other token must hold `telemetry.write`, as today.
- `verify_bearer(headers)` stays, as `verify_bearer_for(headers, Route::Bundle)`.

`admit` calls `verify_bearer_for(&headers, route)`. Because that call comes before every quota and slot charge, a refused bundle charges nothing.

`IngestError::Forbidden(&'static str)` (in `dto.rs`) maps to `StatusCode::FORBIDDEN` and counts as a credential refusal (`is_credential_refusal` returns `true`). Add an arm wherever `IngestError` is matched without a wildcard.

**Kind switch.** `UploadPolicy::launcher_disabled` (read with `launcher_disabled_from_env`) makes a launcher token return `KillSwitchActive`, right after the bearer check.

**Quotas** (per `CHUNK_QUOTA_WINDOW`, 60 s; a launcher chunk is charged on these tables **instead of** `chunk_session` and `chunk_ip`):

| `UploadPolicy` field | Env | Default | Table on `UploadState` (key) | Scope string |
|---|---|---|---|---|
| `launcher_chunk_per_session` | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_SESSION` | 10 | `launcher_chunk_session` (`install_key(&claims.sid)`) | `upload/launcher_session` |
| `launcher_chunk_per_install` | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_INSTALL` | 20 | `launcher_chunk_install` (`install_key(&claims.sub)`) | `upload/launcher_install` |
| `launcher_chunk_per_ip` | `CIMMERIA_TELEMETRY_LAUNCHER_CHUNK_QUOTA_PER_IP` | 60 | `launcher_chunk_ip` (`ip_key(who.peer)`) | `upload/launcher_ip` |

**Slots** (Rec5). `UploadLimits::launcher_chunk_slots` (constant `LAUNCHER_CHUNK_SLOTS = 2` in `telemetry/mod.rs`, next to `CHUNK_SLOTS`) backs `UploadState::launcher_chunk_slots: Arc<Semaphore>`. A launcher chunk takes its slot from that pool, never from `chunk_slots`. The per-peer slot limit still applies.

**Per-kind limits** (R7). `impl UploadLimits { pub(super) fn launcher(&self) -> UploadLimits }` returns a copy in which each of these fields is `.min()` of its own value and the launcher cap:

| Field | Launcher cap |
|---|---|
| `chunk_body_bytes` | 65 536 |
| `chunk_decompressed_bytes` | 262 144 |
| `chunk_row_bytes` | 4 096 |
| `chunk_rows` | 200 |

Player and lab tokens keep their tables, slots and limits. The kill switch stays the first check for every kind.

### C14. Server: launcher rows (AU-14 pure filter, AU-15 wiring and budget)

**AU-14: new `telemetry/launcher_rows.rs` (pure, no I/O):**

```rust
/// The exact adoption event names a launcher token may post (R10).
pub(super) const LAUNCHER_TARGETS: [&str; 1] = ["client.launcher.adoption"];
pub(super) const MAX_FIELDS: usize = 16;
pub(super) const MAX_KEY_BYTES: usize = 32;
pub(super) const MAX_VALUE_BYTES: usize = 256;
/// Keys `replay_native::LiftedFields` lifts into attributes (R13) and identity keys (defence in depth).
pub(super) const DENIED_KEYS: [&str; 29] = ["method_index", "msg_id", "type_id", "entity_id",
    "target_id", "source_id", "pet_id", "ability_id", "item_type_id", "address", "level_name",
    "dll_version", "usable", "rollup_target", "count", "ip", "addr", "peer", "host", "hostname",
    "account", "account_name", "user", "username", "email", "machine_id", "install_id",
    "session_id", "client_pid"];
pub(super) const REMOVED: &str = "<removed>";
/// Drop every row that is not an allowed `ClientNative` row, keep at most `max_rows`,
/// clean the kept rows in place. Returns how many rows were dropped.
pub(super) fn admit_launcher_rows(events: &mut Vec<TelemetryEvent>, max_rows: usize) -> u64;
pub(super) fn allowed_target(target: &str) -> bool;            // exact member of LAUNCHER_TARGETS
pub(super) fn clean_fields(fields: &mut serde_json::Map<String, serde_json::Value>);
pub(super) fn scrub_value(value: &str) -> String;
```

- **Rows** (R9). Every variant other than `ClientNative` (`ClientLog`, `DebugLog`, `KeyDump`, `SessionMeta`) is dropped. So is a `ClientNative` row whose `target` is not `allowed_target`. Rows after the first `max_rows` kept rows are dropped too.
- **Level** (R11). The row's own `level` field is **overwritten**: lower-cased, and anything other than `info`, `warn` or `error` becomes `info`.
- **Fields** (R12, R13). `clean_fields` works in the map's iteration order.
  - A key is removed if it is not `^[a-z][a-z0-9_]{0,31}$` (checked by hand: no regex crate), if it is in `DENIED_KEYS`, or if its value is null, an array or an object. Arrays and objects are dropped, not flattened.
  - Numbers and bools are kept.
  - A string value is cleaned in this order:
    1. Control characters become `' '`.
    2. Any other character outside `' '..='~'` becomes `'?'`.
    3. If `scrub_value` would remove it, it becomes `REMOVED`.
    4. It is cut to `MAX_VALUE_BYTES`.
  - After the first `MAX_FIELDS` keys are kept, the rest are removed.
- **`scrub_value`** (R14) returns `REMOVED` if the value, compared without regard to ASCII case, matches any of:
  - (a) an IP literal: a token, after splitting on whitespace and `,;()[]"'=%/`, that parses as `std::net::IpAddr` or `std::net::SocketAddr`;
  - (b) an ASCII letter followed by `:\` or `:/`;
  - (c) a whitespace-separated token that, after one leading `"` or `'` is removed, starts with `\` or `/`;
  - (d) any of `://`, `file:`, `@`, `\users\`, `/users/`, `/home/` or `$home`;
  - (e) two `%` characters with only `[A-Za-z0-9_]` between them.

  Otherwise it returns the value unchanged.

**AU-15: new `telemetry/launcher_budget.rs`, plus the wiring in `chunk.rs`:**

```rust
pub(super) const DEFAULT_LAUNCHER_ROWS_PER_SESSION: u32 = 300;   // per session lifetime (R15)
pub(super) const LAUNCHER_ROWS_PER_SESSION_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_SESSION";
pub(super) const DEFAULT_LAUNCHER_ROWS_PER_MINUTE: u32 = 3_000;  // global, all callers (R8)
pub(super) const LAUNCHER_ROWS_PER_MINUTE_ENV: &str = "CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_MINUTE";
pub(super) const LAUNCHER_LEDGER_SESSIONS: usize = 4_096;
pub(super) const LAUNCHER_LEDGER_TTL: std::time::Duration = std::time::Duration::from_secs(20 * 60);
pub(super) struct LauncherBudget { /* per_session, per_minute, Mutex<HashMap<String,(u32, Instant)>>, Mutex<(Instant, u32)> */ }
impl LauncherBudget {
    pub(super) fn new(per_session: u32, per_minute: u32) -> Self;
    pub(super) fn from_env() -> Self;
    /// Charge `rows` rows for `sid` at `now`. Err(QuotaExceeded) if the global minute
    /// would pass `per_minute` (nothing is charged); otherwise Ok(n), where n <= rows
    /// rows still fit the session's lifetime budget (those n are charged globally).
    pub(super) fn charge(&self, sid: &str, rows: usize, now: Instant) -> Result<usize, AuthError>;
}
```

- **The ledger** is separate from the shared `SessionLedger`. On each insert it prunes entries older than `LAUNCHER_LEDGER_TTL`. If it still holds `LAUNCHER_LEDGER_SESSIONS` entries, a new sid gets `Ok(0)`: its rows are suppressed (fail closed).
- **Where it lives.** `LauncherBudget` is held beside the shared `SessionLedger`. Find how `chunk.rs` reaches the ledger and hold the budget the same way.
- **The `chunk.rs` launcher path** (`claims.is_launcher()`):
  1. Use `limits.launcher()` for the body read and for `decode_chunk`.
  2. Right after the decode, `bad_rows += launcher_rows::admit_launcher_rows(&mut events, limits.launcher().chunk_rows)`.
  3. Call `budget.charge(&claims.sid, events.len(), now)`. An `Err` refuses the whole chunk with 429 and replays nothing. `Ok(n)` admits the first `n` rows; the rest count as `suppressed`.
  4. Skip `admit_budgeted` and `name_chunk`, using `EntityLabels::none()` instead (R13).
  5. Pass `client_pid = None` (R15).
- Player and lab chunks take exactly today's path.

**Where the rows land.** A kept row replays like any `client_native` row:
- `service.name = cimmeria-client`, `client_target = client.launcher.adoption` (also the log body), `client_level`;
- `cimmeria.session_kind = launcher`, `lab = false`, `session_id`, `install_id`;
- the cleaned `fields`, as one JSON string attribute.

No lifted attributes and no entity labels are added.

---

## Waves

| Wave | Packets | Ships as | Merge order into the integration branch |
|---|---|---|---|
| 2a (server) | AU-11, AU-14, AU-12, AU-15, AU-16 | one PR from `au-srv-integration`, **held for OQ-8** | AU-11 → AU-14 → AU-12 → AU-15 → AU-16 |
| 2b (launcher) | AU-07, AU-10, AU-13, AU-08, AU-09 | one PR from `au-tel-integration`, merged after 2a | AU-07 → AU-10 → AU-13 → AU-08 → sync → AU-09 |

**PR merge order into `main`:**
1. The wave-1 PR.
2. The 2a PR, after a maintainer approves OQ-8, followed by a colo deploy.
3. The 2b PR.

If the owner refuses OQ-8, 2b does not merge, and the architect re-rules it (README OQ-8).

### Wave 2 dependencies

- **Start at once:** AU-07, AU-10, AU-11 and AU-14. No two of them touch the same file, and none touches a wave-1 file.
- **AU-12** starts once AU-11 is integrated. Its base is `origin/main` with `launcher-adopt-ux/launcher-session` merged in.
- **AU-15** starts once AU-12 and AU-14 are integrated.
- **AU-16** starts once AU-15 is integrated, so it documents the final values.
- **AU-13** starts once AU-07 is integrated, because both add lines to `game_telemetry/mod.rs`.
- **AU-08** starts once AU-13 is integrated.
- **The sync.** Once the wave-1 PR is in `main`, `ops` merges `origin/main` into `au-tel-integration` with `merge-noff.ps1 -C <au-tel-integration> -Branch origin/main`.
- **AU-09** starts after the sync and AU-08. It needs AU-03's `Adoption::fail`, `clear_failure` and `last_unsafe`.

**Reviews.** Every packet first gets its own packet loop. At each sub-wave boundary there is a wave review:
- **2a:** one `packet-reviewer` holds AU-11, AU-12, AU-14, AU-15 and AU-16. In parallel, `network-security-auth` runs a read-only pass against R1-R16. Its notes are numbered `A<k>`. Both must say `ship`.
- **2b:** one `packet-reviewer` holds AU-07, AU-10, AU-13, AU-08 and AU-09.

**Integration checks.**
- 2a, in `au-srv-integration`: S1, S2, S4.
- 2b, in `au-tel-integration`: D1; the full engine and desktop suites without a filter; D2e and D2s; F1, F2 and G2.

**PR body, 2a.**
- Title it "needs maintainer approval".
- Link README § Security review and § Open owner questions (OQ-8).
- State the routes, the claims, the caps and the env overrides (C12-C14). State that the master kill switch and `CIMMERIA_TELEMETRY_LAUNCHER_DISABLED` both stop launcher mints and uploads.
- State that the PR needs a colo deploy before 2b is of any use, and that rollback is `CIMMERIA_TELEMETRY_LAUNCHER_DISABLED=1` or a revert.
- List the doc-update-map rows AU-16 touched, and AU-16's audit of "not lab means player" consumers.

**PR body, 2b: SigNoz verification.** This is an owner step, run after 2a is deployed. Agents do not run it.
1. Build a packaged launcher that bundles the telemetry DLL. In Settings, turn on **Send game diagnostics**.
2. Adopt a folder that holds a hard link, so the adoption is refused. Then fix it and adopt again. **Do not press Play.**
3. Within a minute, run this in SigNoz Logs: `service.name = 'cimmeria-client' AND client_target = 'client.launcher.adoption' AND cimmeria.session_kind = 'launcher'`.
   - Expect rows for `mode_chosen`, `folder_chosen`, `refused` (WARN, `error` `unsafe_file`), `review_ready`, `confirmed` and `finished`, in the `fields` JSON attribute.
   - Add `fields CONTAINS '"step":"refused"'` to narrow it.
   - No `fields` value may hold a drive letter, an IP address or the user name.
4. If the SigNoz MCP is down, use the ClickHouse route in the `telemetry-triage` skill: `signoz_logs.distributed_logs_v2`, with `attributes_string['client_target'] = 'client.launcher.adoption' AND attributes_string['cimmeria.session_kind'] = 'launcher'`.
5. Turn the setting off and adopt again: no new rows appear. Disconnect the network and adopt: the adoption works, no error is shown, and nothing is sent.
6. Check the local log. `%APPDATA%\app.cimmeria.launcher.desktop\logs\launcher.<date>.log` exists, holds the same steps, and shows the home folder as `~`.

Also list the doc-update-map rows AU-10 touched.
- Tested by agents: unit, route and wiring tests, F2 and G2.
- Not tested by agents: a real mint and upload against the colo, and SigNoz.

---

## Wave 2a: server

Rules shared by every 2a packet, inlined into each packet's list:
- PowerShell only, never Bash.
- Run `cargo` only through `pwsh tools/build-lane/lane.ps1`, in the foreground.
- Never `git stash`, `git worktree prune` or push.
- Soft cap 500 lines, hard cap 700.
- Every behavior change has a test that fails when the change is reverted.
- Never log a token, an IP, an account name, a `machine_id` or a path at INFO.

### AU-11: the `launcher` kind on `/api/auth/dev-session` (mint and refresh)

Host: windows · Branch `launcher-adopt-ux/AU-11-launcher-kind` · Worktree `au-11` · Crate `cimmeria-admin-api` · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-11;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. `handlers.rs` is 515 lines, so add at most 30 lines there. The launcher checks and constants go in `launcher_kind.rs`, and its tests in `launcher_kind_tests.rs`.
5. A behavior change needs a test that fails when the change is reverted. These are route-handler tests on `mint_inner` and `refresh_inner`, in the style of `session_kind_tests.rs`.
6. Security:
   - The token is never logged.
   - A launcher mint never reads, logs or signs `machine_id`.
   - No new INFO field may carry an IP, an account name or a path.
   - Player and lab tokens stay byte-identical; `a_player_mint_has_no_kind_claim` and every other existing test must pass unchanged, except the one refusal text in Do step 5.
7. Edit only these files under `crates/admin-api/src/routes/`:
   - `dev_session/token.rs`
   - `dev_session/handlers.rs`
   - `dev_session/mod.rs` (two `mod` lines)
   - `dev_session/tests.rs` (the `policy` helper only)
   - `dev_session/session_kind_tests.rs` (the expected refusal text only)
   - new `dev_session/launcher_kind.rs`
   - new `dev_session/launcher_kind_tests.rs`
   - any other file holding a `QuotaPolicy {` literal (add the new fields only)

**Contract.** C12.

**Do.**
1. `token.rs`:
   - Add `SESSION_KIND_LAUNCHER` and `SCOPE_LAUNCHER_WRITE` after `SESSION_KIND_PLAYER` (line 34).
   - Add `is_launcher` and `kind_matches_scope` after `is_lab` (line 83).
   - Update the `kind` doc comment (lines 48-51) to name `"launcher"`: a 15-minute label for launcher-only events, with its own scope.
   - Grep the file for the `"telemetry.write"` literal. If there is no constant for it, add `pub const SCOPE_WRITE: &str = "telemetry.write";` and use it.
2. New `launcher_kind.rs`, holding the C12 constants and functions, with a 4-line module doc. Cover four points: what a launcher session is; that minting stays unauthenticated; that the kind is a label backed by its own scope and tighter caps, never a privilege; and README D-AU19/D-AU26.
   - In `mod.rs`, add `mod launcher_kind;` (use `pub(crate)` if the telemetry module needs it, as AU-12's `launcher_disabled_from_env` will).
   - Next to `mod session_kind_tests;` (line 61), add `#[cfg(test)] mod launcher_kind_tests;`.
3. `handlers.rs`:
   - In `kind_claim` (lines 110-120), add the launcher arm and change the reason to `must be "player", "lab" or "launcher"`.
   - Add these fields to `QuotaPolicy`: `pub mint_launcher_per_ip: u32`, `pub mint_launcher_per_install: u32` and `pub launcher_disabled: bool`. Read them in `from_env` with the C12 env names, defaults and `launcher_disabled_from_env()`. Add `mint_launcher_ip` and `mint_launcher_install` (both `WindowTable`) to `Tables`, built in `new`.
   - Grep the crate for `QuotaPolicy {` and add the fields to every literal. The `policy` helper in `tests.rs` gets `mint_launcher_per_ip: 30, mint_launcher_per_install: 12, launcher_disabled: false`.
4. `mint_inner`: follow the C12 order. Compute `let launcher = req.session_kind.as_deref().map(str::trim) == Some(SESSION_KIND_LAUNCHER);` from the raw request right after the kill-switch check.
   - Put the launcher-only steps (disabled, the launcher IP quota, dropping `machine_id`, the metadata check, the account refusal, the launcher install quota) into a `launcher_kind.rs` helper. That keeps `handlers.rs` within its 30-line budget.
   - For a launcher mint: `exp` uses `LAUNCHER_TOKEN_TTL_SECONDS`, `scope` is `[SCOPE_LAUNCHER_WRITE]` and `mid` is `None`. Call `record_stream` only when `!launcher`. The reply's `chunk_max_bytes` is `LAUNCHER_CHUNK_MAX_BYTES`.
   - The debug line at about line 307 must not print `machine_id` for a launcher mint.
5. `refresh_inner`: right after the token decodes and verifies, and before `refresh_ip` is charged (about line 354), refuse a launcher token with the C12 `BadField`. In `session_kind_tests.rs` line 136, update the expected text to the new reason.
6. Grep `crates/` and `tools/` for another verifier of these tokens: `decode_token`, `"telemetry.write"`, and the "Functions-side verifier" named in `token.rs` near line 36. List what you find in the report, and change nothing outside rule 7 (Rec9).

**Tests** (`launcher_kind_tests.rs`). Import the way `session_kind_tests.rs` lines 6-10 do. Build requests with `DevSessionRequest { session_kind: Some("launcher".into()), ..request("cimmeria-launcher") }`.
- `a_launcher_mint_is_short_lived_scoped_and_carries_no_account_or_machine`. Send `machine_id: Some("m-1".into())`, then decode the token and check:
  - `kind == Some("launcher")`, `is_launcher()` and `kind_matches_scope()`;
  - `exp - iat == 900` and `scope == ["telemetry.launcher.write"]`;
  - `acct == None` and `mid == None`;
  - `resp.chunk_max_bytes == 65_536`.

  It must fail if the TTL, the scope, the kind or the `mid` rule is reverted.
- `a_launcher_mint_with_an_account_name_is_refused`. `account_name: Some("gerger".into())` gives `BadField { field: "account_name", .. }` and `status(&err) == 400`.
- `a_launcher_token_is_not_refreshed`. Mint, then call `refresh_inner`: it returns the C12 `BadField` (400). Then refresh a player token as many times as `refresh_per_ip` allows: every refresh passes, which shows the launcher refusal charged no `refresh_ip`. The test must fail if the check is removed, because the launcher refresh would then succeed with `exp - iat == 8 h`.
- `launcher_mints_have_their_own_ip_quota`. Use `QuotaPolicy { mint_per_ip: 100, mint_launcher_per_ip: 3, mint_launcher_per_install: 100, ..policy(10, 10, 10) }` (adjust it to the helper's real argument meaning). Three launcher mints from one IP with three installs pass, and the fourth is `QuotaExceeded`. A player mint from the same IP still passes.
- `launcher_mints_have_their_own_install_quota`. With `mint_launcher_per_install: 2`, two launcher mints pass and the third is `QuotaExceeded`. A player mint for the same install still passes.
- `the_kill_switches_refuse_a_launcher_mint`. Turn on the master kill switch the way the `dev_session_kill_switch` test in `tests.rs` (about line 570) does: the launcher mint is `KillSwitchActive`. Then, with `launcher_disabled: true` and the master switch off: the launcher mint is `KillSwitchActive`, and a player mint passes.
- `launcher_metadata_must_be_printable_ascii`. `launcher_version: Some("1.0\u{202E}x".into())` on a launcher mint gives `BadField` (400). The same value on a player mint is not refused by this rule.
- `lab_and_player_mints_are_unchanged`. A lab mint still has `exp - iat == 8 * 3600` and scope `["telemetry.write"]`. A player mint still has `chunk_max_bytes == 1_048_576`.
- `kind_and_scope_must_agree` (a unit test on `TokenClaims`). Launcher kind with `["telemetry.write"]`: false. Player with `["telemetry.launcher.write"]`: false. The two true cases: true.
- If a helper in the `dev_session/` tests reads the recorded streams (grep `record_stream` and `stream`), add `a_launcher_mint_records_no_stream`. Otherwise say so in the report.

**Checks, in order.** S1, S2, S3d. Report the S3d test count, the line counts of `handlers.rs` and `launcher_kind.rs`, and the Do step 6 findings.

**Docs.** None; AU-16 writes them.

**Commit.** `AU-11: launcher session kind on dev-session mint and refresh (launcher-adopt-ux)`

---

### AU-14: launcher row filter, level, field allow-list and scrub (pure)

Host: windows · Branch `launcher-adopt-ux/AU-14-launcher-rows` · Worktree `au-14` · Crate `cimmeria-admin-api` · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-14;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. Keep `launcher_rows.rs` under 250 lines; the tests go in their own file.
5. A behavior change needs a test that fails when the change is reverted.
6. The module is pure: no I/O, no tracing, and no regex crate. Use `std::net` parsing and plain string code. Nothing calls it yet; AU-15 wires it in. If clippy flags dead code, add `#[cfg_attr(not(test), allow(dead_code))]` on the module item in `mod.rs`, with a comment `// wired in by AU-15`.
7. Edit only these files under `crates/admin-api/src/routes/telemetry/`:
   - `mod.rs` (two `mod` lines)
   - new `launcher_rows.rs`
   - new `launcher_rows_tests.rs`

**Contract.** C14, the AU-14 part. `TelemetryEvent` and `ClientNativeEvent` are in `telemetry/dto.rs`. `ClientNativeEvent` has `ts_ms`, `seq`, `target`, `level` and `fields: serde_json::Map<String, Value>`. Before writing `DENIED_KEYS`, check the lifted key list against `LiftedFields::from_fields` in `replay_native.rs`. If that function reads a key that is not in the list, add it, and say so in the report.

**Do.**
1. New `launcher_rows.rs`, per C14, with a 4-line module doc. Say why the rules exist: a launcher session is minted without a game and without a credential, so its rows are held to an exact event list and a scalar field allow-list, scrubbed of IPs, accounts and paths, and kept out of lifted attributes and entity naming. Cite README D-AU20 and the security review R9-R14.
2. In `telemetry/mod.rs`, add `mod launcher_rows;` and `#[cfg(test)] mod launcher_rows_tests;` with the other `mod` lines.

**Tests** (`launcher_rows_tests.rs`, `use super::launcher_rows::*; use super::dto::*;`). Build `TelemetryEvent`s with `serde_json::from_value(json!({...}))`, copying the row shapes from `upload_limits_tests/mod.rs` `small_chunk`/`rows_of`.
- `only_allowed_client_native_rows_are_kept`. The events are: an adoption row; the same row with target `client.native.status`; one `key_dump`, one `client_log` and one `session_meta` row; and a `client.launcher.x1` row. `admit_launcher_rows(&mut v, 200)` returns 5 and leaves one row. The test must fail if the variant or target check is removed.
- `rows_past_the_cap_are_dropped`. 250 adoption rows with `max_rows` 200 return 50, and `v.len() == 200`.
- `the_level_is_overwritten`. `"FATAL"` becomes `"info"`, `"WARN"` becomes `"warn"`, and `"info\n\u001b[31mx"` becomes `"info"` (R11).
- `fields_are_an_allow_list`. Run `clean_fields` on these fields:

  | Key | Value | After `clean_fields` |
  |---|---|---|
  | `ip` | `"1.2.3.4"` | removed |
  | `Account_Name` | `"gerger"` | removed |
  | `entity_id` | `7` | removed |
  | `nested` | `{"a":1}` | removed |
  | `list` | `[1]` | removed |
  | `h0st` | `"db.example"` | kept |
  | `detail` | `"from 10.0.0.5:8081 now"` | `"<removed>"` |
  | `path` | `"C:\\Users\\bob\\x"` | `"<removed>"` |
  | `unc` | `"\\\\srv\\share"` | `"<removed>"` |
  | `rel` | `"Binaries/x.dll"` | unchanged |
  | `step` | `"refused\nFORGED=1"` | `"refused FORGED=1"` |
  | `uni` | `"caf\u{e9}"` | `"caf?"` |
  | `n` | `2` | the number 2 |
  | `long` | 300 `a`s | 256 bytes |

  `h0st` is kept: the allow-list bounds the shape, and the scrub catches its values.

  Then 20 fields `k0`..`k19` keep exactly 16.
- `scrub_value_cases` (a table):
  - Removed: `1.2.3.4:80`, `[::1]:80`, `::ffff:1.2.3.4`, `fe80::1%eth0`, `see http://x/y`, `D:/SGW`, `\\srv\s`, `/etc/x`, `"/etc/x"`, `file:C`, `a@b.c`, `%USERPROFILE%\x`, `x\Users\y`, `/home/y`.
  - Unchanged: `desktop-0.4.1`, `hard_link:later.txt; link:(location)`, `.`, `Binaries/x.dll`, `10.0.26300.1`.

  Each removed case is a revert proof for its pattern.
- `allowed_target_cases`. `client.launcher.adoption` is true. These are false: `client.launcher.install_result`, `client.native`, `Client.launcher.adoption`, `client.launcher.adoption\n`, and `client.launcher.x1`.

**Checks, in order.** S1, S2, S3t. Report the S3t test count and the line count of `launcher_rows.rs`.

**Docs.** None.

**Commit.** `AU-14: launcher row allow-list and scrub (launcher-adopt-ux)`

---

### AU-12: launcher tokens at the upload gate

Host: windows · Branch `launcher-adopt-ux/AU-12-launcher-gate` · Worktree `au-12` · Crate `cimmeria-admin-api` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/launcher-session` merged in (holds AU-11, AU-14)

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-12;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. `upload_gate.rs` is 387 lines and must stay under 500. If it would pass 480, move `verify_bearer_for` and the launcher policy reading into a new `telemetry/launcher_gate.rs`, and say so in the report.
5. A behavior change needs a test that fails when the change is reverted.
6. The kill switch stays the first check in `admit`. Player and lab tokens keep their scope rule, tables, slots and limits. A refused launcher bundle charges no table and takes no slot.
7. Edit only these files under `crates/admin-api/src/routes/telemetry/`:
   - `upload_gate.rs` (or the new `launcher_gate.rs` from rule 4)
   - `dto.rs`
   - `mod.rs` (the constant, and a `mod` line if needed)
   - `refusal_log.rs` (only if it matches `IngestError` without a wildcard)
   - `upload_limits_tests/mod.rs`
   - new `upload_limits_tests/launcher_gate.rs`
   - any other file that holds an `UploadPolicy {` or `UploadLimits {` literal (add the new fields only)

**Contract.** C13, using `TokenClaims::is_launcher`, `kind_matches_scope` and `SCOPE_LAUNCHER_WRITE` from C12, and `launcher_disabled_from_env`.

**Do.**
1. `dto.rs`: add `IngestError::Forbidden(&'static str)`, with a `#[error(...)]` text in the style of the others. Map it to `StatusCode::FORBIDDEN` in the response match (about line 240), and add it to `is_credential_refusal` as `true`. Let the compiler find every other exhaustive match.
2. `upload_gate.rs`:
   - Add `verify_bearer_for`, as C13 describes, and make `verify_bearer` delegate to it with `Route::Bundle`. In `admit`, call `verify_bearer_for(&headers, route)`. Then, if `claims.is_launcher() && policy.launcher_disabled`, return `KillSwitchActive`.
   - Next to `DEFAULT_CHUNK_PER_IP` (line 71), add the three `DEFAULT_LAUNCHER_CHUNK_*` constants (10, 20, 60), one doc line each.
   - Add the four `UploadPolicy` fields (`launcher_chunk_per_session`, `launcher_chunk_per_install`, `launcher_chunk_per_ip`, `launcher_disabled`). Read them in `from_env` the way `chunk_per_session` is read (line 101), and set the defaults in `defaults()` (line 124).
   - Add the three launcher `WindowTable`s and `launcher_chunk_slots: Arc<Semaphore>` to `UploadState`, built where the others are built.
   - Add `launcher_chunk_slots` to `UploadLimits` (default `LAUNCHER_CHUNK_SLOTS`), and add `UploadLimits::launcher()` per C13.
   - For a launcher `Route::Chunk`, charge the launcher session, install and IP tables, in that order, instead of `chunk_session` and `chunk_ip`. Map each to `IngestError::Auth(AuthError::QuotaExceeded(q))`, as the others are, and use the C13 scope strings. Take the slot from `launcher_chunk_slots` with `limits.launcher_chunk_slots`, through the same acquire call as at about line 304.
3. `mod.rs`: add `const LAUNCHER_CHUNK_SLOTS: usize = 2;` next to `CHUNK_SLOTS` (line 188), with a doc line explaining that launcher tokens are free, so they get their own small pool and never hold the slots the game DLL needs.
4. `upload_limits_tests/mod.rs`: add `mod launcher_gate;`. Next to `claims`/`token`/`chunk_request` (lines 86-119), add three helpers:
   - `launcher_claims(sid, install) -> TokenClaims`, with `kind: Some("launcher".into())`, `scope: vec!["telemetry.launcher.write".into()]`, `sub: install.into()` and `exp` 900 s ahead;
   - `launcher_token(sid, install) -> String`;
   - `launcher_chunk_request(sid, body) -> Request`, using the install `"install-launcher-test"`.

**Tests** (`launcher_gate.rs`). Call `admit` directly with a `HeaderMap` holding `Authorization: Bearer <token>` and `Uploader::anonymous(PEER)`. Drop each `Admitted` at once so its slot frees, and hold `Env::install()`.
- `a_launcher_token_cannot_upload_a_bundle`. `Route::Bundle` returns `IngestError::Forbidden(_)`, and its response status is 403. Then, with `bundle_per_ip: 1` and `bundle_per_session: 1`, a player bundle admit from the same peer still passes. That shows the refusal charged nothing. The test must fail if the check is removed.
- `kind_and_scope_must_agree_at_the_gate`. A token with `kind: Some("launcher")` and scope `["telemetry.write"]` is `MissingScope` on both routes. A player-kind token with scope `["telemetry.launcher.write"]` is `MissingScope` on `Route::Bundle`.
- `a_launcher_token_may_upload_a_chunk`: it is admitted.
- `launcher_chunks_have_their_own_session_quota`. With `UploadPolicy { launcher_chunk_per_session: 2, ..UploadPolicy::defaults() }`, two admits pass and the third is `QuotaExceeded`. Then a player token (`token("sess-player")`) from the same peer is admitted.
- `launcher_chunks_are_capped_per_install`. With `launcher_chunk_per_install: 3`, four sessions of one install send one chunk each: the fourth is `QuotaExceeded`.
- `launcher_chunks_are_capped_per_address`. With `launcher_chunk_per_ip: 2`, three sessions of three installs from one peer: the third is refused.
- `launcher_chunks_use_their_own_slots`. With `UploadLimits { launcher_chunk_slots: 1, .. }`, hold one launcher `Admitted`. A second launcher admit gets the busy refusal the slot code gives (copy the assertion from `gate.rs` about line 119). A player admit still passes while the launcher slot is held.
- `the_kill_switches_refuse_a_launcher_chunk`. `kill_switch: true` returns `KillSwitchActive`. Then, with `launcher_disabled: true` alone, a launcher chunk returns `KillSwitchActive` and a player chunk is admitted.
- `launcher_limits_are_capped`. `UploadLimits::defaults().launcher()` has `chunk_body_bytes == 65_536`, `chunk_decompressed_bytes == 262_144`, `chunk_row_bytes == 4_096` and `chunk_rows == 200`. Use the real constructor name; grep `UploadLimits {` in the tests.
- `player_limits_are_unchanged`. With every launcher limit at 1, three player admits from one session pass.

**Checks, in order.** S1, S2, S3t. Report the S3t test count and the line count of `upload_gate.rs`.

**Docs.** None; AU-16 writes them.

**Commit.** `AU-12: launcher session scope, quotas and slots at the upload gate (launcher-adopt-ux)`

---

### AU-15: launcher chunks: per-kind limits, row filter, own budget, no naming

Host: windows · Branch `launcher-adopt-ux/AU-15-launcher-chunk` · Worktree `au-15` · Crate `cimmeria-admin-api` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/launcher-session` merged in (holds AU-11, AU-14, AU-12)

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-15;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. `chunk.rs` (329 lines) gains at most 35 lines. If the launcher path needs more, put it in a function in `launcher_budget.rs`. Keep `launcher_budget.rs` under 200 lines.
5. A behavior change needs a test that fails when the change is reverted.
6. A player or lab chunk must take exactly today's path. Every existing `upload_limits_tests` and `client_pid_tests` test passes unchanged.
7. Edit only these files under `crates/admin-api/src/routes/telemetry/`:
   - `chunk.rs`
   - `mod.rs` (`mod` lines)
   - wherever the shared `SessionLedger` is held (to hold `LauncherBudget` beside it)
   - new `launcher_budget.rs`
   - `upload_limits_tests/mod.rs` (one `mod` line)
   - new `upload_limits_tests/launcher_chunk.rs`
   - remove the `allow(dead_code)` that AU-14 may have added on `launcher_rows`

**Contract.** C14, the AU-15 part, plus C13's `UploadLimits::launcher()`.

**Do.**
1. New `launcher_budget.rs`, per C14. For the `QuotaExceeded` value, build it the way `WindowTable::check_and_record` builds its refusal (grep `QuotaExceeded(` in `dev_session/quota.rs`). Use the scope `"upload/launcher_rows"` and a retry of the rest of the current minute.
2. Hold one `LauncherBudget::from_env()` beside the shared `SessionLedger`, the same way that is held and reached from `chunk.rs`.
3. `chunk.rs`, the launcher path, per C14:
   - The body read (line 96) and `decode_chunk` (line 228) use `let limits = if launcher { limits.launcher() } else { limits.clone() };`, or a borrow if `UploadLimits` is `Copy`.
   - Destructure `DecodedChunk { mut events, truncation, mut bad_rows }` (lines 109-113).
   - Run the filter, then the budget charge.
   - Skip `admit_budgeted` and `name_chunk` (line 122); use `EntityLabels::none()` and `client_pid = None`.
   - Add one sentence to the module doc (lines 3-9) pointing at `launcher_rows` and `launcher_budget`.

**Tests** (`upload_limits_tests/launcher_chunk.rs`). Drive the chunk handler with AU-12's `launcher_chunk_request`, the way `upload_limits_tests/chunk.rs` drives `chunk_request`.
- `a_launcher_chunk_keeps_only_launcher_rows`. Three rows: an adoption row, the same row with target `client.native.status`, and a `key_dump` row. With a launcher token, `accepted == 1` and `bad_rows == 2`. It must fail if the filter call is removed.
- `a_player_chunk_with_the_same_rows_is_not_filtered`. The same body sent with `chunk_request` gives `bad_rows == 0`, and the key dump is accepted as today.
- `a_launcher_chunk_over_64_kib_is_refused`. A gzip body over 65 536 bytes (incompressible bytes, so it stays big) returns `TooLarge` with a launcher token, and does not return `TooLarge` with a player token.
- `a_launcher_gzip_bomb_is_cut_at_256_kib`. About 300 KiB of adoption rows compressed (well under 64 KiB) gets the outcome the player path gives past its decompressed cap (copy the assertion from the existing decompressed-cap test in `upload_limits_tests/chunk.rs`). The same body with a player token is accepted whole.
- `launcher_rows_have_their_own_session_budget`. With `LauncherBudget::new(3, 1000)` injected the way the tests inject the session ledger (if they cannot inject it, set `CIMMERIA_TELEMETRY_LAUNCHER_ROWS_PER_SESSION` under `Env::install()`'s guard), a 5-row chunk gives `accepted == 3` and `suppressed == 2`. A player session's budget is untouched: a player chunk of 5 rows is accepted whole.
- `the_global_launcher_ceiling_refuses_the_next_chunk`. With `LauncherBudget::new(1000, 4)`, two sessions each send a 3-row chunk: the first is accepted, and the second is `QuotaExceeded` (429) and replays nothing.
- `launcher_rows_are_not_named` (best effort). If the chunk tests can pass an `EntityLabelLink` whose receiver can be counted, send a launcher chunk whose row carries `entity_id`, and assert that zero label requests arrive. If they cannot, test `launcher_budget`'s launcher-path function so it is proven to return `EntityLabels::none()`, or say in the report that only code review covers it.
- In `launcher_budget.rs` `mod tests`: `the_ledger_prunes_and_fails_closed`. A `LauncherBudget` built with a test-only capacity of 2: two sids are charged; a third sid gets `Ok(0)`; after `LAUNCHER_LEDGER_TTL` (pass `now + ttl + 1 s`), a new sid gets its rows.

**Checks, in order.** S1, S2, S3t. Report the S3t test count and the line counts of `chunk.rs` and `launcher_budget.rs`.

**Docs.** None; AU-16 writes them.

**Commit.** `AU-15: launcher chunk limits, row filter and budget on upload-chunk (launcher-adopt-ux)`

---

### AU-16: server docs for the launcher kind, and the query audit

Host: windows · Branch `launcher-adopt-ux/AU-16-server-docs` · Worktree `au-16` · Docs only · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/launcher-session` merged in (holds AU-11, AU-14, AU-12, AU-15)

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-16;`.
2. No code change: this packet runs no `cargo`. If an audit finding needs a code fix, write it in the report as a follow-up and do not change the code.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Every file you edit under `docs/` is **CRLF**.
   - After editing, check with `[IO.File]::ReadAllText('<absolute path>') -match "(?<!\r)\n"`; it must be false.
   - Otherwise normalize with `[IO.File]::WriteAllText('<absolute path>', ($text -replace "\r?\n", "`r`n"))`.
   - Use absolute paths only.
5. Never hand-edit a `<!-- gen:NAME -->` block. Take every number and env name from the code in the integration branch, not from this spec. If they differ, the code wins, and you say so in the report.
6. Add about 70 doc lines in total, at most.
7. Edit only:
   - `docs/operations/telemetry.md`
   - `docs/tools/admin-api.md` (only if it lists `session_kind` values or the dev-session request fields)
   - `docs/architecture/launcher-summary-telemetry.md` (only the audit note in Do step 6)

**Contract.** C12-C14.

**Do** (`docs/operations/telemetry.md`).
1. Near line 11: the session kinds are `player`, `lab` and `launcher`. A launcher session is minted by the desktop launcher for its own game-folder steps, only for opted-in players, without a game. It is short, write-only and has its own scope.
2. § Kill switch (line 221): `CIMMERIA_TELEMETRY_LAUNCHER_DISABLED=1` turns off the launcher kind alone (mint and upload answer 503), and the master switch still stops everything.
3. § Mint and refresh quotas (the table at line 324): add `CIMMERIA_TELEMETRY_LAUNCHER_MINT_QUOTA_PER_IP` (30) and `..._PER_INSTALL` (12). Under the table, add one sentence: a launcher token lives 15 minutes, carries no account or machine, is never refreshed (400), and its mints also count against the shared per-IP quota.
4. § Upload size and rate limits (line 418): a "Launcher sessions" paragraph and table covering:
   - the three chunk quotas;
   - the global rows-per-minute ceiling and the per-session row budget, with their env names;
   - the 2 reserved slots;
   - the per-kind body, decompressed, row and row-count caps;
   - that bundles are refused (403);
   - the exact event list;
   - the field allow-list and the scrub;
   - that rows are never named or lifted.
5. § Where the data lives (line 545): `cimmeria.session_kind` is `lab`, `player` or `launcher`, and `launcher` is not the `launcher_summary` row label (line 548). After the `client.launcher.install_result` bullet (line 575), add:
   - The bullet: `service.name = 'cimmeria-client' AND client_target = 'client.launcher.adoption' AND cimmeria.session_kind = 'launcher'`, the desktop launcher's adoption steps. Its `fields` hold `step`, `decision_outcome`, `mode`, `error`, `unsafe_total` and `unsafe_entries`. Opted-in players only, sent as they happen.
   - One sentence after it: a query meaning "a player's game session" should filter `cimmeria.session_kind = 'player'`, not `!= 'lab'`. Launcher rows can be forged by anyone who mints a session, so funnel counts built from them measure what clients claim.
   - § Endpoints (line 784): the mint row names `"session_kind": "launcher"`, the refresh row says a launcher token is refused (400), and the bundle row says a launcher token is refused (403).
6. **Audit** (Rec3). Grep `crates/admin-api`, `crates/server`, `tools/` and `docs/` for consumers that treat "not lab" as "player": `is_lab()`, `session_kind != 'lab'`, `!= "lab"`, `NOT lab`, and the `launcher_summary/rows.rs` readers. List each one in the report as `file:line: what it does: needs a change (yes/no)`. A docs query that would now count launcher sessions as players gets fixed in this packet (in an allowed file). A code consumer is reported as a follow-up.
7. Walk `docs/agents/doc-update-map.md` rows 57 (admin API) and 60 (operations runbooks). In the report, say which rows you touched and which you checked without change.

**Checks.** The CRLF check (rule 4) on each edited file, and `Test-Path` on every relative link you added.

**Commit.** `AU-16: telemetry runbook for the launcher session kind (launcher-adopt-ux)`

---

## Wave 2b: launcher

### AU-07: engine launcher events, scrubbed (no queue)

Host: windows · Branch `launcher-adopt-ux/AU-07-launcher-event` · Worktree `au-07` · Crate `cimmeria-launcher-engine` · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-07;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500 lines, hard cap 700. Keep `launcher_event.rs` under 200 lines; the tests go in their own file.
5. A behavior change needs a test that fails when the change is reverted.
6. The packet is pure: no consent check, no queue, no network, and no logging of field values. Nothing here may write to `telemetry-queue.jsonl` (README D-AU21).
7. Edit only:
   - `engine/src/storage/game_telemetry/mod.rs` (two module lines)
   - new `engine/src/storage/game_telemetry/launcher_event.rs`
   - new `engine/src/storage/game_telemetry/launcher_event_tests.rs`

**Contract.** C8.

**Do.**
1. `engine/src/storage/game_telemetry/mod.rs`: add `pub mod launcher_event;` after line 9 (`pub(crate) mod install_event;`). After line 8 (`mod event_tests;`), add `#[cfg(test)]` and `mod launcher_event_tests;`.
2. `launcher_event.rs`: write C8. Take `ClientNativeEvent` and `TelemetryEvent` from `cimmeria_launcher_core::telemetry::events`; the transport re-exports `TelemetryEvent` from there. Grep `ClientNativeEvent` in `crates/launcher-core/src/telemetry/events.rs` for its fields.
   - Module doc, 3-5 lines: what the event is; that it is built here and sent at once by `launcher_session` (AU-13) under a `launcher` session; that nothing is queued on disk; and that the server scrubs again (README D-AU19, D-AU21).

**Tests** (`launcher_event_tests.rs`, `use super::launcher_event::*; use super::transport::TelemetryEvent;`):
- `events_carry_the_launcher_target_and_build_fields`. One record (`level "INFO"`, fields `{"step":"mode_chosen"}`) gives one `ClientNative` event with:
  - `target == "client.launcher.adoption"` and `level == "info"`;
  - `fields["step"] == "mode_chosen"`;
  - `fields["os"]` and `fields["launcher_version"]` present.

  Two records get two different `seq`.
- `string_fields_are_scrubbed`. Use `Redaction { home: Some(r"C:\Users\alice".into()), user: Some("alice".into()) }`:

  | Field | Value | After |
  |---|---|---|
  | `path` | `later.txt` | unchanged |
  | `detail` | `C:\Users\alice\Games\SGW` | `~\Games\SGW` |
  | `other` | `D:\SGW\Binaries` | `<path removed>` |
  | `unc` | `\\server\share\x` | `<path removed>` |
  | `seg` | `Games/alice/SGW` | `Games/<user>/SGW` |
  | `n` | `2` | the number 2 |
  | `long` | 300 `a`s | 240 `a`s followed by `...` |

  The test must fail if `scrub` or the cut is skipped.
- `replace_home_and_scrub_cases`. A table on the methods, with `user: None` unless the case names one:
  - With home `C:\Users\alice`: `replace_home(r"x C:\Users\alice\a")` gives `x ~\a`; `C:/Users/alice/a` gives `~/a`; `C:\\Users\\alice\\a` (doubled) gives `~\\a`.
  - Home `None` or `Some("")` changes nothing.
  - With user `bob`: `"Bob/x"` gives `<user>/x`, and `"bobcat.upk"` is unchanged.
  - `scrub("hard_link:later.txt; link:(location)")` is unchanged. `scrub("/home/bob/x")` and `scrub("\"/etc/x\"")` give `<path removed>`. `scrub(".")` is unchanged.

**Checks, in order.** D1, D2e, D3g. Report the D3g test count and the line count of `launcher_event.rs`.

**Docs.** None; AU-10 writes them.

**Commit.** `AU-07: scrubbed launcher adoption events (launcher-adopt-ux)`

---

### AU-13: engine launcher session sender

Host: windows · Branch `launcher-adopt-ux/AU-13-launcher-session` · Worktree `au-13` · Crate `cimmeria-launcher-engine` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/telemetry` merged in (holds AU-07)

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-13;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. Keep `launcher_session.rs` under 250 lines; the tests go in their own file. `session.rs` (208 lines) gains at most 20 lines.
5. A behavior change needs a test that fails when the change is reverted. Tests use the `wiremock` `MockServer`, the way `game_telemetry/tests.rs` (about lines 80-135) mounts the mint route. Tests never reach a real server.
6. Fail silently. `send` returns a `SendOutcome`, never an error the UI could see.
   - It never writes to disk, and never touches `telemetry-queue.jsonl` or `Transport` (README D-AU21).
   - It logs reason codes only, at debug, and never at target `adoption`, which would loop back into the forwarder.
   - It re-mints only on 401 (R16).
7. A player mint stays byte-identical: `start` sends no `session_kind` key.
8. Edit only:
   - `engine/src/storage/game_telemetry/session.rs`
   - `engine/src/storage/game_telemetry/mod.rs` (two module lines)
   - new `engine/src/storage/game_telemetry/launcher_session.rs`
   - new `engine/src/storage/game_telemetry/launcher_session_tests.rs`

**Contract.** C11. Background:
- `launch/telemetry.rs` lines 20-24 read an opted-in player's `Identity` with `game_telemetry()?.opted_in.then_some(choice.identity).flatten()`.
- `launch/preparation.rs` lines 21-24 choose the login servers: the preferences, else `login_servers::default_servers()`, then `login_servers::validate`.

**Do.**
1. `session.rs`: add `session_kind` to `MintRequest` (C11). Move the body of `start` into `pub(super) async fn start_kind(.., session_kind: Option<&str>)`, and make `start` call it with `None`. Add `#[cfg(test)] pub(super) fn mint_body_for_test(kind: Option<&str>) -> serde_json::Value`, which serializes a `MintRequest` with fixed values.
2. `mod.rs`: add `pub mod launcher_session;` next to `pub mod launcher_event;`, and `#[cfg(test)] mod launcher_session_tests;`.
3. `launcher_session.rs`: write C11.
   - `LauncherSender::new` builds its `reqwest::Client` the way `session.rs` lines 102-107 do: no redirects, `session::CONNECT` and `session::TOTAL`.
   - `launcher_target` reads `self.game_telemetry()` and returns `None` on any error.

**Tests** (`launcher_session_tests.rs`):
- `a_player_mint_body_has_no_session_kind`. `mint_body_for_test(None)` has no `session_kind` key, and `mint_body_for_test(Some("launcher"))["session_kind"] == "launcher"`.
- `the_target_needs_the_opt_in`. With state in a tempdir, `launcher_target()` is `None`. After `set_game_telemetry(true)` it is `Some`, with a non-empty `servers`.
- `a_launcher_session_is_minted_and_events_are_posted`.
  - Mount a `MockServer` with two routes:
    - the mint route, matching `body_partial_json({"session_kind":"launcher"})`, with `expect(1)`, replying with `upload_endpoint = format!("{}/api/telemetry", server.uri())`;
    - `/api/telemetry/upload-chunk`, answering 200, with `expect(2)`.
  - The target's one server is `server.uri()`.
  - Two `send`s of one event each both return `Sent`, and the mint ran once.

  The test must fail if `session_kind` is not sent, because the mint mock then does not match.
- `offline_fails_silently_then_holds_with_backoff`. Point the target at `http://127.0.0.1:9`.
  - The first `send` is `MintFailed`, with `hold_remaining()` between 50 s and 60 s; the next `send` at once is `Held`.
  - After `clear_hold()`, the next `send` is `MintFailed` with `hold_remaining()` between 110 s and 120 s (the doubling).
- `a_rejected_token_is_reminted_once`. The chunk mock answers 401 once (`up_to_n_times(1)`), then 200. The mint is expected twice, and `send` is `Sent`.
- `a_forbidden_upload_is_not_reminted`. The chunk mock answers 403. The mint is expected once. `send` is `Dropped`, and the next `send` is not `Held`, because 403 sets no hold.
- `a_retry_after_holds_the_sender`.
  - The chunk mock answers 429 with `Retry-After: 600`: `send` is `Dropped`, `hold_remaining()` is between 590 s and 600 s, the next `send` is `Held`, and no second chunk request arrives.
  - Then, with `Retry-After: 5`, `hold_remaining()` is at least 55 s (the 60 s floor).
- `an_empty_batch_sends_nothing`. `send(&target, &[])` is `Empty`, and the mint mock has `expect(0)`.

**Checks, in order.** D1, D2e, D3g. Report the D3g test count and the line counts of `session.rs` and `launcher_session.rs`.

**Docs.** None; AU-10 writes them.

**Commit.** `AU-13: launcher session sender for adoption events (launcher-adopt-ux)`

---

### AU-08: shell tracing subscriber, rolling log file, `adoption` forwarder

Host: windows · Branch `launcher-adopt-ux/AU-08-subscriber` · Worktree `au-08` · Crate `cimmeria-launcher-desktop` · Agent: packet-coder · Base: `origin/main` with `launcher-adopt-ux/telemetry` merged in (holds AU-07, AU-10, AU-13)

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-08;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`. Run L0 once, after editing `Cargo.toml`, and commit `Cargo.lock`; every other check uses `--locked`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700. `shell/src/main.rs` is about 599 lines, so add at most 8 lines there; the logic goes in `telemetry.rs`.
5. Nothing may block the UI.
   - The file writer is `tracing_appender::non_blocking`.
   - The layer sends with `try_send` only.
   - Network work happens only on the forwarder thread.
   - If the log folder cannot be created, the file layer is left out and the user is told nothing.
6. No OpenTelemetry crate and no network code in the shell. Sending is `LauncherSender`'s job (AU-13).
7. Tests never call `init`, because it sets the global subscriber. Use `tracing::subscriber::with_default` with `file_layer` or `ForwardLayer`.
8. Edit only:
   - `shell/Cargo.toml`
   - `crates/launcher/desktop/Cargo.lock`
   - `shell/src/main.rs`
   - `shell/src/host.rs` (one `mod` line)
   - new `shell/src/telemetry.rs`
   - new `shell/src/telemetry_tests.rs`
   - new `shell/src/host/launcher_telemetry.rs`

**Contract.** C9; it uses C8 and C11.

**Do.**
1. `shell/Cargo.toml` `[dependencies]`: add `tracing-subscriber = { version = "0.3", features = ["registry", "fmt", "std"] }`, `tracing-appender = "0.2"` and `serde_json = "1"`. Leave `[dev-dependencies]` as it is. Run L0.
2. `shell/src/telemetry.rs`, per C9:
   - **`file_layer`.**
     - Call `std::fs::create_dir_all(log_dir).ok()?`.
     - Build the appender with `tracing_appender::rolling::Builder::new().rotation(Rotation::DAILY).filename_prefix(LOG_FILE_PREFIX).filename_suffix("log").max_log_files(LOG_FILES_KEPT).build(log_dir).ok()?`.
     - Then `let (writer, guard) = tracing_appender::non_blocking(appender);`.
     - The layer is `tracing_subscriber::fmt::layer().with_ansi(false).with_writer(move || Redacting { inner: writer.clone(), redaction: redaction.clone() }).with_filter(<Targets per C9>)`.
   - **`Redacting<W: io::Write>`.** In `write`, decode `buf` lossily, call `self.redaction.replace_home`, `write_all` the result, and return `Ok(buf.len())`. `flush` delegates.
   - **`ForwardLayer`.** `impl<S: tracing::Subscriber> tracing_subscriber::Layer<S>` with `on_event` only, following C9's forward rule. A private `FieldVisitor` (`tracing::field::Visit`) fills a `serde_json::Map`.
   - **`init`.**
     - `let (tx, rx) = std::sync::mpsc::sync_channel(FORWARD_CAPACITY);`
     - Split `file_layer(log_dir, redaction)` into `(Option<layer>, Option<guard>)`.
     - `let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer).with(ForwardLayer::new(tx)));`
     - Return `Telemetry { guard, records: rx }`.
   - **`spawn_forwarder`.** Spawn one named thread and ignore a spawn error:
     ```rust
     std::thread::Builder::new().name("launcher-telemetry".into()).spawn(move || {
         let sender = LauncherSender::new();
         while let Ok(first) = records.recv() {
             let mut batch = vec![first];
             while batch.len() < FORWARD_BATCH {
                 match records.try_recv() {
                     Ok(r) => batch.push(r),
                     Err(_) => break,
                 }
             }
             if let Some((target, events)) = host.launcher_batch(&batch, &redaction) {
                 let _ = tauri::async_runtime::block_on(sender.send(&target, &events));
             }
         }
     })
     ```
3. New `shell/src/host/launcher_telemetry.rs`, holding `launcher_batch` per C9.
   - Read `available` exactly as `host/game_telemetry.rs` lines 35-38 do.
   - Then call `self.store()`, lock the way `with_state` does (`host.rs` about line 133), call `launcher_target()`, and drop the lock.
   - Build the events with `launcher_events(records, redaction)`. Any error returns `None`.
   - In `shell/src/host.rs`, add `mod launcher_telemetry;` next to `mod game_telemetry;` (line 11).
4. `shell/src/main.rs`:
   - Add `mod telemetry;` and `#[cfg(test)] mod telemetry_tests;` with the other `mod` lines.
   - In `.setup` (about line 554), right after `let app_data = app.path().app_data_dir()?;` (about line 562), add `let redaction = Redaction::from_env(); let telemetry = telemetry::init(&app_data.join("logs"), redaction.clone());`.
   - Replace `app.manage(Arc::new(host));` (about line 573) with `let host = Arc::new(host); telemetry::spawn_forwarder(Arc::clone(&host), telemetry.records, redaction); app.manage(host); app.manage(std::sync::Mutex::new(telemetry.guard));`.
   - If the managed type was not `Arc<NativeHost>`, report `blocked` and quote the line.

**Tests** (`shell/src/telemetry_tests.rs`, `use crate::telemetry::*;`):
- `only_adoption_info_and_above_is_forwarded`.
  - Create `sync_channel(16)` and run `with_default(registry().with(ForwardLayer::new(tx)), || { … })`. Inside, emit four events:
    1. `info!(target: "adoption", step = "mode_chosen", decision_outcome = "copy", "adoption step")`
    2. `debug!(target: "adoption", "x")`
    3. `warn!(target: "cimmeria_launcher_engine", path = "C:/x", "other")`
    4. `warn!(target: "adoption", reason = ?Some(1), path = %"later.txt", total = 2u64, "adoption refused: an entry cannot be adopted safely")`
  - Expect exactly 2 records:
    - `[0]`: `level "info"`, `fields["step"] == "mode_chosen"` and `ts_ms > 0`;
    - `[1]`: `level "warn"`, `fields["path"] == "later.txt"`, `fields["total"] == 2`, and `fields["message"]` holding `adoption refused`.

  The test must fail if the target or level rule is removed.
- `a_full_channel_drops_instead_of_blocking`. With `sync_channel(1)`, three adoption INFO events leave exactly one record. If `try_send` becomes `send`, the test hangs and the lane times out, which counts as the failure.
- `the_log_writer_writes_the_home_folder_as_a_tilde`. Use `Redacting { inner: Vec::new(), redaction: Redaction { home: Some(r"C:\Users\alice".into()), user: None } }` and `write_all` the text `C:\Users\alice\AppData\x and "C:\\Users\\alice\\y"`. The output contains `~\AppData\x` and `~\\y`, and does not contain `alice`.
- `file_layer_writes_a_launcher_log`.
  - Use a fresh temp dir: `tempfile` if `[dev-dependencies]` has it, else `std::env::temp_dir().join(uuid::Uuid::new_v4().to_string())`.
  - Build the layer with `let (layer, guard) = file_layer(&dir, Redaction { home: Some(r"C:\Users\alice".into()), user: None }).unwrap();`.
  - Run `with_default(registry().with(layer), || tracing::warn!(target: "adoption", path = r"C:\Users\alice\x", "adoption refused"))`, then `drop(guard)`.
  - Expect exactly one file named `launcher.*.log`. It holds `adoption refused` and `~\x`, and does not hold `alice`.
- `a_batch_needs_the_module_and_the_opt_in`, in a `#[cfg(test)] mod tests` inside `host/launcher_telemetry.rs`.
  - Build hosts the way the tests in `host/game_telemetry.rs` (about lines 95-125) build one without the module and one with it.
  - Without the module: `None`. With the module, opted out: `None`.
  - With the module, after `game_telemetry_command(Set { opted_in: true })`: `Some`, holding one event whose target is `client.launcher.adoption`.

  The test must fail if either gate is removed.

**Checks, in order.** L0 (once), D1, D2s, D3t, F2 (the shell still starts its tests, and the adoption UAT still passes). Report the D3t test count, the `git diff --stat` of `Cargo.lock`, and the line counts of `main.rs` and `telemetry.rs`.

**Docs.** None; AU-10 writes them.

**Commit.** `AU-08: launcher log file and adoption-event forwarder (launcher-adopt-ux)`

---

### AU-09: shell adoption step events

Host: windows · Branch `launcher-adopt-ux/AU-09-step-events` · Worktree `au-09` · Crate `cimmeria-launcher-desktop` · Agent: packet-coder · Base: `origin/main` **after the wave-1 PR has merged**, with `launcher-adopt-ux/telemetry` merged in (holds AU-07, AU-10, AU-13, AU-08)

This packet is unchanged from the first wave-2 design except for the base. The events now leave at once through AU-08's forwarder; this packet only emits them.

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-09;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Files: soft cap 500, hard cap 700.
   - `shell/src/host/adoption/mod.rs` is about 494 lines, so add at most 5 lines there: the `mod` line and the Confirm-arm call.
   - The tests go inside `events.rs` (`#[cfg(test)] mod tests`) and in `in_place_tests.rs`.
   - `main.rs` gains at most 6 lines.
5. No field may hold an absolute path or free text. Use only the C10 fields. The paths come from `UnsafeReport`, which already holds relative paths.
6. Before editing, confirm that `Adoption::fail`, `Adoption::clear_failure` and the field `last_unsafe` exist (AU-03). If they do not, report `blocked`: the base is missing wave 1.
7. Edit only these files:
   - `shell/src/host/adoption/mod.rs`
   - `shell/src/host/adoption/status.rs`
   - new `shell/src/host/adoption/events.rs`
   - `shell/src/host/adoption/in_place_tests.rs`
   - `shell/src/host.rs` (one `use` line)
   - `shell/src/main.rs`

**Contract.** C10. Capture events in tests with `crate::telemetry::ForwardLayer` (C9).

**Do.**
1. `events.rs`: the six C10 functions, plus `fn mode(in_place: bool) -> &'static str` and `fn code<T: serde::Serialize>(v: T) -> String`. `code` returns `serde_json::to_value(v)` as a string, or `"unknown"` otherwise.
   - Each function is one `tracing::info!` or `tracing::warn!` call with `target: "adoption"`.
   - `finished` uses two macro calls: one with `error` and one without.
2. `mod.rs`: add `pub(crate) mod events;` with the other `mod` lines. In the Confirm arm (about lines 341-367), right after `self.confirm_adoption(...)?;` succeeds, add `if let Some(job) = &adoption.confirm { events::confirmed(job.in_place); }`. Use the binding the arm already has for the locked `Adoption`.
3. `status.rs` `settle_adoption` (the AU-03 version):
   - Preview `Ok` arm: after `adoption.clear_failure();`, add `events::review_ready();`.
   - Preview `Err` arm: `Err(error) => { adoption.fail(error); if let Some(code) = adoption.last_error { events::refused(code, adoption.last_unsafe.as_ref()); } }`.
   - Confirm-finished branch: after the `match outcome.err() { … }`, add `events::finished(<the job's in_place>, adoption.last_error);`. Read `in_place` from the `ConfirmJob` that the branch takes. If the job is gone by then, copy `in_place` out before it is dropped.
4. `host.rs`: after `mod adoption;` (line 8), add `pub(crate) use adoption::events as adoption_events;`.
5. `main.rs` `choose_existing_game_folder` (lines 350-391):
   - Derive `let in_place = …` from the mode value passed to `host.begin_folder_adoption` (line 387), by comparing it with its in-place variant.
   - After the `adoption_mode_allowed` check (lines 358-361) passes, call `crate::host::adoption_events::mode_chosen(in_place);`.
   - Before every `return cancelled(host).await;` in this function, call `folder_chosen(in_place, false)`. That covers the source picker (line 369) and the copy-destination picker (lines 372-377).
   - Just before `host.begin_folder_adoption(...)`, call `folder_chosen(in_place, true)`.

**Tests.**
- `every_step_carries_its_contract_fields`, in `events.rs` `mod tests`.
  - Capture with `with_default(registry().with(ForwardLayer::new(tx)), || { … })`, and call each C10 function once:
    - `refused(AdoptionError::UnsafeFile, Some(&report))`, with a report whose entries are `[HardLink "later.txt", Link None]` and whose total is 2;
    - `finished(true, None)` and `finished(false, Some(AdoptionError::UnsafeFile))`.
  - Assert each record's `step`, `decision_outcome`, `mode` and level against C10.
  - For `refused`: `error == "unsafe_file"`, `unsafe_total == 2` and `unsafe_entries == "hard_link:later.txt; link:(location)"`.
  - The adopted `finished` record has no `error`.
- `adoption_steps_are_logged_for_a_refusal_and_an_adoption`, in `in_place_tests.rs` (`#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`).
  - Start with `let (tx, rx) = std::sync::mpsc::sync_channel(256); let _guard = tracing::subscriber::set_default(tracing_subscriber::registry().with(crate::telemetry::ForwardLayer::new(tx)));`.
  - Then follow AU-03's `a_hard_linked_file_is_named_and_dismissed`:
    1. Plant the link, begin, and wait for the refusal.
    2. Remove the link, dismiss, begin again, and call `reviewed(&f.host).await`.
    3. Confirm the way the file's existing confirm test does, and wait for idle without an error.
  - Collect `rx.try_iter()`. Expect these `step` values, in order:
    - `refused`, with `error == "unsafe_file"` and `unsafe_total == 2`;
    - `review_ready`;
    - `confirmed`;
    - `finished`, with `decision_outcome == "adopted"` and `mode == "in_place"`.

    The test must fail if any call-site line is removed.
  - If no record arrives because `settle_adoption` runs off the test thread, report `blocked` with that evidence. Do not switch to a global subscriber.

**Checks, in order.** D1, D2s, D3s, D3t, F1, F2. Report the D3s test count and the line counts of `mod.rs`, `status.rs`, `events.rs` and `main.rs`.

**Docs.** None; AU-10 documents C10.

**Commit.** `AU-09: adoption step events with decision outcomes (launcher-adopt-ux)`

---

### AU-10: game-diagnostics label and copy, privacy note, UAT and launcher docs

Host: windows · Branch `launcher-adopt-ux/AU-10-copy-docs` · Worktree `au-10` · `frontend/` and docs · Agent: packet-coder · Base: `origin/main`

**Rules (inline).**
1. PowerShell only, never Bash. Begin every shell call with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\au-10;`.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo ...` (G2 needs it) in the foreground. Never pass `--target-dir`.
3. Never `git stash`, `git worktree prune` or push. Commit locally and stop.
4. Every file you edit under `docs/` and `crates/launcher/desktop/docs/` is **CRLF**.
   - After editing, check with `[IO.File]::ReadAllText('<absolute path>') -match "(?<!\r)\n"`; it must be false.
   - Otherwise normalize with `[IO.File]::WriteAllText('<absolute path>', ($text -replace "\r?\n", "`r`n"))`.
   - Use absolute paths only.
5. Copy the UI texts below exactly. Every frontend change needs its test and the REPL logic UAT (G2). Every button press keeps its visible feedback on the first press: the existing `Turning game diagnostics on…` text and its test stay as they are.
6. Never hand-edit a `<!-- gen:NAME -->` block. Add about 60 doc lines in total, at most. Do not edit `docs/operations/telemetry.md`; AU-16 owns it.
7. Before editing, grep the repository (excluding `.claude/worktrees`, `target` and `node_modules`) for the old label `Send game diagnostics while playing` and for the two old texts: the `<small>` sentence and the `'On. The next Play…'` line.
   - Update every hit under `crates/launcher/desktop/frontend/`, `crates/launcher/desktop/docs/` and `docs/`, and list each one in the report.
   - Leave hits under `docs/analysis/` alone; they are ledgers.
   - A hit in Rust code or a test outside `frontend/` means you report `blocked` with the file.
8. Edit only:
   - `frontend/ui/index.html`
   - `frontend/src/game-telemetry-view.ts`
   - `frontend/src/game-telemetry-view.test.ts`
   - `frontend/game-telemetry-native-uat.mjs`
   - `docs/client/launcher-guide.md`
   - `docs/architecture/dev-session-telemetry.md`
   - `crates/launcher/desktop/docs/launch.md`
   - the label hits from rule 7

**The opt-in (README D-AU22, D-AU25).** It is the same checkbox and the same record (`game-telemetry.json`, `opted_in`). The label changes from **Send game diagnostics while playing** to **Send game diagnostics**, because the launcher now sends when the player is not playing (Rec1). Launcher events go only when the checkbox is on and the build bundles the diagnostics module, so the existing texts for a build without the module ("Nothing is sent") stay true.

**Texts.**
- **Label.** Wherever the old label appears as UI or doc text, it becomes `Send game diagnostics`.
- **Small text.** Replace the `<small>` on index.html line 14 with: `Off by default, and separate from setup diagnostics. When on, Play loads a diagnostics module into the game, which sends game events and logs to your login server, and the launcher sends the steps of Use an existing game folder (such as why a folder was refused) as they happen.`
- **Privacy note.** After the `</label>` on line 14, before `<p id="game-telemetry-status"`, insert: `<p id="game-telemetry-privacy" class="caption">Game-folder events name files only by their place inside the game folder, never your user name, home folder or address. The launcher also keeps its own log on this computer for 7 days.</p>`
- **On text.** In `game-telemetry-view.ts` line 18, the On text becomes: `'On. The next Play loads the diagnostics module into the game and sends game events and logs to your login server. Game-folder steps are sent as they happen.'` The Off text (line 12) and every other line stay as they are.

**Tests and UAT.**
- Update `game-telemetry-view.test.ts` wherever it asserts the old On text or the old label.
- Add `the privacy note names what game-folder events carry`. It reads `../ui/index.html` with `readFileSync(new URL('../ui/index.html', import.meta.url), 'utf8')` (adjust to how the test file resolves paths), and asserts that the file holds `id="game-telemetry-privacy"`, the exact note text, and the label `Send game diagnostics<`. It fails if the note is removed.
- `game-telemetry-native-uat.mjs`:
  - After the step where the first press turns diagnostics on and the native save is confirmed, assert that the status text (`text()`) includes `Game-folder steps are sent as they happen`, and that `document.getElementById('game-telemetry-privacy')` exists.
  - Add `launcher steps disclosed on opt-in` to the final `passed` sentence (line 48), keeping its start `Game diagnostics logic UAT passed`.
- The first-press feedback test (`game-telemetry-view.test.ts` line 14) must still pass unchanged.

**Docs** (CRLF).
1. `docs/client/launcher-guide.md`:
   - § Settings, the game-diagnostics row (about line 279). It names the new label and says: when it is on, the launcher also sends its game-folder events (the steps of **Use an existing game folder**, and why a folder was refused) to your login server as they happen, without Play. They name files only by their place inside the game folder, never your user name or home folder. Off by default.
   - § Use an existing game folder, after lines 210-211, one sentence: with game diagnostics on, each step of the adoption is sent as it happens; offline, nothing is sent, and the adoption works the same.
   - § Where to find logs to share (about line 859). The launcher log is at `%APPDATA%\app.cimmeria.launcher.desktop\logs\launcher.<date>.log`; on Linux, `~/.local/share/app.cimmeria.launcher.desktop/logs/`; on macOS, `~/Library/Application Support/app.cimmeria.launcher.desktop/logs/`. There is one file a day, 7 are kept, and the home folder is written as `~`. **Upload debug logs** does not include it yet, so attach it by hand.
2. `docs/architecture/dev-session-telemetry.md`, the table at line 70. After the `install_result.rs` row, add two rows:
   - `crates/launcher/desktop/engine/src/storage/game_telemetry/launcher_event.rs`: builds `client.launcher.adoption` events from the desktop shell's `adoption` log target, scrubbed of the home folder, the user name and absolute paths.
   - `…/game_telemetry/launcher_session.rs`: mints a short `launcher` session, without a game, for an opted-in player, and posts those events at once. Nothing is queued on disk. It re-mints only on 401 and holds after a 429 or 503.

   Add one sentence under the table: the server side of the `launcher` kind is in `docs/operations/telemetry.md` § Upload size and rate limits.
3. `crates/launcher/desktop/docs/launch.md`, after the `client.launcher.install_result` paragraph (about line 441), one paragraph covering:
   - the shell's subscriber (`shell/src/telemetry.rs`): the rolling `logs/launcher.<date>.log`, 7 kept, home written as `~`;
   - the `adoption` forward layer and its bounded, dropping channel;
   - `launcher_session::LauncherSender`: a `launcher` session minted on the first step, lasting 15 minutes and re-minted when it runs out; events posted at once; dropped silently when offline;
   - the six steps (C10 of [work-packets-wave2.md](../../../../docs/analysis/launcher-adopt-ux/work-packets-wave2.md); check the relative link with `Test-Path`);
   - that nothing leaves without the game-diagnostics opt-in, in a build with the module.

   Leave `client.launcher.install_result` as it is: it still waits for Play.
4. Walk `docs/agents/doc-update-map.md` rows 56 (launcher) and 60 (operations runbooks). In the report, say which rows you touched and which you checked without change.

**Checks, in order.** F1, G2, the CRLF check (rule 4) on each edited doc, and `Test-Path` on every relative link you added.

**Commit.** `AU-10: game-diagnostics label, copy, privacy note and launcher telemetry docs (launcher-adopt-ux)`
