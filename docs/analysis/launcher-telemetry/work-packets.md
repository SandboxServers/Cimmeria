# Launcher telemetry: work packets (rules, contract, waves 0-2)

Ledger: [README.md](README.md). Waves 3, 4 and 7: [work-packets-coverage.md](work-packets-coverage.md). Waves 5 (adoption reasons) and 6 (file index): [work-packets-adoption.md](work-packets-adoption.md). Read **§ Rules** and the **§ Contract** entries your packet names before you edit anything.

**Amendment 2026-10-21 (owner directives, [owner-directives.md](owner-directives.md)).** LT-01, C8, C10, C11, LT-07, LT-08 and LT-11 below were re-specified for directive 1 (default upload URL, no localhost fallback). Where an older ledger line contradicts these sections, these sections win.

**Amendment 2, 2026-10-21 (directives 5 and 6).** Two owner calls. Directive 5: launcher summaries and the default telemetry upload URL are https, with no plain-http path. Directive 6: no IP address in any launcher or telemetry log, field or attribute, and no hashing. Re-specified: C8, C10, C11, C12, LT-01 and LT-02. New: LT-38 and LT-39 (wave 1), LT-40 (wave 3) and LT-41 (the https default, **BlockedDecision D-LT17**), all in [work-packets-coverage.md](work-packets-coverage.md) except LT-38 and LT-39, which are below. Test literals in LT-05, LT-07 and LT-08:
- the login-server example `http://play.cimmeria.app:8081` becomes `https://play.example`, and its derived upload URL becomes `https://play.example/api/telemetry`;
- a plain-http non-loopback login server that advertises a loopback endpoint gives `effective_upload_endpoint(..) == None` until LT-41 sets the built-in default;
- that `None` is `SendOutcome::MintFailed { reason: "no_https_endpoint" }`.

Every new test in these packets uses a TEST-NET peer (`198.51.100.7`) and a DNS-name Host (`play.example`), and asserts that the captured message and every field value contain neither `198.51.100.7` nor any other IP literal. When the change is reverted, that test fails.

**Amendment 3, 2026-10-21 (re-based on `main`).** All of launcher-adopt-ux is on `main`: PR #1953 squash-merged as `0bd5a2aec` with AU wave 1, 2a, 2b and AU-09; #1954 was closed as superseded by it. Every integration branch below is created from `origin/main`, and no packet waits on an AU branch. Where a packet below names a line number, the current `origin/main` anchor (at `5079be48f`) is this table; it wins over the packet text. Re-grep before editing all the same.

| Packet | Old anchor | `origin/main` anchor |
|---|---|---|
| LT-01 | `handlers.rs:30` `DEFAULT_UPLOAD_ENDPOINT` | `:31` |
| LT-01 | `handlers.rs:201` `mint`; `:297` mint INFO; `:317`, `:416` response `upload_endpoint` | `:214`; `:307` ("Minted dev-session telemetry token"); `:319`, `:419` |
| LT-01 | `handlers.rs:512-521` `upload_endpoint_env` / `upload_endpoint_from` | `:515-524` |
| LT-01 | `mod.rs:127-142` `log_boot_status` | `:130-145` |
| LT-01 | `session_kind_tests.rs:151-157` default pin | `:153` |
| LT-38 | `handlers.rs:438-498` `log_refusal`; `.inspect_err` at `:213`, `:229` | `:445-501`; `:226` (mint), `:242` (refresh) |
| LT-38 | `tests.rs:469`, `:507`, `:550` fixed peers | `:472` (`198.51.100.21`), `:510` (`198.51.100.9`), `:553` (`203.0.113.5`) |
| LT-02 | `refusal_log.rs` `peer = %who.peer` at :332, :375, :401; `request_span.rs` `admin_request_row` :66-105, pin :257 | unchanged |
| LT-39 | `launcher_summary/handlers.rs` `ingest` :90, `peer_ip` :161 | unchanged |
| LT-03 | `launcher_rows.rs` "from AU 2a" | on `main`: `pub(super) const LAUNCHER_TARGETS: [&str; 1] = ["client.launcher.adoption"];` at `:17`, `allowed_target` use at `:83`; tests in `launcher_rows_tests.rs` |
| LT-05 | AU 2b files | on `main`: `shell/src/telemetry.rs` (`FORWARD_TARGET = "adoption"` :26, `ForwardLayer` :114, `on_event` :125); `engine/.../game_telemetry/launcher_session.rs` (`LauncherSender` :69); `launcher_event.rs` ~145 lines |

No LT wave-1 packet edits `otel.rs`; client rows reach SigNoz through `routes/telemetry/replay.rs` unchanged (D-LT1). `launcher_targets.txt` is not on `main` yet; LT-03 and LT-05 still both create it.

**Amendment 4, 2026-10-21 (directive 7: the colo https origin exists).** The board's Caddy proxy serves `https://play.cimmeria.app` and forwards only the five telemetry routes to loopback :8081, so D-LT17 is resolved and LT-41 is unblocked once its status-code probe passes (work-packets-coverage § LT-41). New: **LT-42** (wave 1, PR S, below, contract C18): the server keys its per-IP quotas and per-peer upload slots on `X-Forwarded-For`, trusted from a loopback peer only, and never logs or stores it. LT-01, LT-02, LT-03, LT-38 and LT-39 are **unchanged** in scope and files; LT-42 branches after them. LT-04 now also documents the Caddy route and LT-42.

**Amendment 5, 2026-10-21 (LT-09 re-spec).** The LT-09 coder returned blocked: the packet named five events for fourteen sites and had no source for `attempt` or `http_status`. Under the owner directive "wire all launcher telemetry, nothing silent" LT-09 is now **LT-09a** (transport and queue, the existing `lt-09` worktree) then **LT-09b** (session loop, `install_event.rs`, `patch_events.rs`; a new `lt-09b` worktree after LT-09a merges). `attempt` is the transport's count of consecutive failed flushes (core has no per-attempt retry), `http_status` comes from the raw `TelemetryError` at the call site (`TransportError` is not widened), `queue_dropped` fires at the bundle, `chunk_upload_gave_up` is dropped, and `bound_s` becomes `duration_ms`. C6 is unchanged. No other packet changes scope.

## Dispatch rules (orchestrator)

- `ops` creates one worktree per packet: `pwsh tools/build-lane/mk-worktree.ps1 launcher-telemetry/<packet>-<slug> <name>`, giving `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\<name>`. Packet worktree names are `lt-01`, `lt-02`, and so on.
- Integration branches: `ops` merges each packet branch with `merge-noff.ps1` in the order below. Wave reviews run in the integration worktree, and one PR ships it. After the PR merges, retire the packet worktrees with `rm-worktree.ps1 --merged-into <PR> <name>...`.
- A packet brief carries: the worktree path, this section path, the report path `<worktree>\.claude\reports\launcher-telemetry\<packet>-coder.md`, the attribution lines, and the return line.
- Parallel read-only passes: **network-security-auth** on LT-01, LT-02, LT-38 and LT-42, with notes numbered `A<k>`, using the packet-reviewer report sections and verdict words.
- Amendment 3: no AU pre-check is left (#1953 carries all of AU; #1954 closed). `ops` records the `origin/main` SHA it creates the integration branches from, and every packet branch of the same dispatch uses that SHA.

### Branches

| Integration | Worktree | Created from | Packets |
|---|---|---|---|
| `launcher-telemetry/server` | `lt-srv-integration` | `origin/main` (amendment 3) | LT-01..LT-04, LT-38, LT-39, LT-42 |
| `launcher-telemetry/launcher` | `lt-app-integration` | `origin/main` (amendment 3) | LT-05..LT-23, LT-25, LT-26, LT-40 |
| (none, own PR H) | `lt-41` | `origin/main` after PR S and PR L | LT-41 |
| `launcher-telemetry/adoption` | `lt-adopt-integration` | `lt-app-integration` at the LT-05 merge; no AU merge (AU wave 1 and AU-09 are on `main`, amendment 3) | LT-28..LT-34 |
| `launcher-telemetry/file-index` | `lt-index-integration` | `origin/main` | LT-35, LT-36 (LT-37 is an owner step, no branch) |

Each packet branch is created from its integration branch's tip at the moment it is dispatched.

- Wave 1 packets LT-01, LT-02, LT-03 and LT-39 branch from the base at once. LT-38 branches after LT-01 has merged, because both edit `dev_session/handlers.rs`. LT-42 (amendment 4) branches after LT-01, LT-02, LT-38 and LT-39 have merged, because it edits `dev_session/handlers.rs` and `mod.rs` (LT-01, LT-38), `launcher_summary/handlers.rs` (LT-39) and the `login_port.rs` doc comment (LT-02 may touch that file). LT-03 shares no file with it. LT-04 branches after all six have merged into `lt-srv-integration`.
- LT-40 branches from `lt-app-integration` after LT-05 has merged, like the rest of wave 3, and merges first in wave 3.
- LT-05 goes first. LT-06..LT-10 and every wave-3 packet branch from `lt-app-integration` after LT-05 has merged into it. Waves 2 and 3 then run side by side.
- LT-09b (amendment 5) branches from `lt-app-integration` after LT-09a has merged, because it calls `transport::events::reason`.
- LT-11 branches after LT-07, LT-08 and LT-10 have merged.
- Wave 4 branches after every wave-2 and wave-3 packet has merged. Before wave 4 starts, ops merges `origin/main` into `lt-app-integration` with `merge-noff.ps1` (amendment 3; was the AU 2b tip).

### Merge order

| Wave | Order |
|---|---|
| 1 (PR S) | LT-01 → LT-02 → LT-03 → LT-39 → LT-38 (branches after LT-01 merges) → LT-42 (branches after LT-01, LT-02, LT-38, LT-39 merge) → wave-1 review → LT-04 → PR S |
| 2 | LT-05 → LT-06 → LT-07 → LT-08 → LT-09a → LT-09b (branches after LT-09a merges) → LT-10 → LT-11 → wave-2 review |
| 3 | LT-40 → LT-12 → LT-13 → LT-14a → LT-14b (one branch, one merge) → LT-15 → LT-16a → LT-16b (one branch) → LT-17a → LT-17b (one branch) → wave-3 review (with LT-40's IP guard live in C9). LT-16 and LT-17 branch from `lt-app-integration` after the LT-14 branch merges; they run in parallel (amendment 6). |
| 4 | LT-18 → LT-19 → LT-20 → LT-21a+b → LT-22, LT-23a, LT-23b → wave-4 review (amendment 7: five parallel lanes, LT-19 before the LT-22 lane; [work-packets-wave4.md](work-packets-wave4.md)) |
| 5 (PR A) | LT-28 → LT-29 → LT-30 → LT-31 → LT-32 → LT-33a → LT-33b → LT-33c → LT-34 → wave-5 review → PR A |
| 6 (PR I) | LT-35 → LT-36 → wave-6 review → PR I. LT-37 (owner publish) runs only after PR I merges. |
| 7 | LT-25 → ops merges `origin/main` into `lt-app-integration` → LT-26 → PR L. LT-24 is PR F; LT-27 is PR X. |
| H | LT-41 (BlockedDecision D-LT17) from `origin/main` after PR S and PR L, before PR X and before the launcher release is cut. It edits both sides (the admin-api C11 constant and the engine C10 and summary constants), so it cannot ride in PR S or PR L alone. |

Wave 5 runs side by side with waves 2-4: its packets branch from `lt-adopt-integration` and touch only the adoption files. Wave 6 needs nothing else in this campaign and can start at once. Wave 7 is the old wave 5 (verification, docs, close-out), renumbered.

## Rules (every packet; packet-coder reads these)

- **R1 Host and shell.** `Host: windows`. Use PowerShell only, never Bash. Start every command with `Set-Location <worktree>;`, run it in the foreground and wait for it. Never call `cargo` directly; only through `pwsh tools/build-lane/lane.ps1`. Run the checks in the listed order, with at most 5 fix rounds per check.
- **R2 Scope and git.** Edit only the files your packet names, plus the new files it names. No `git stash`, no `git worktree prune`, no push, no rebase. Make one local commit with the given subject, ending with:
  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01WCmH1WPLJdizxaciRv4aJM
  ```
- **R3 File caps.** Soft cap 500 lines, hard cap 700. If a named file is already over 450 lines, put the new logic in the new file the packet names and add only call lines to the big file. Report the line count of every file you touched.
- **R4 Line endings.** Files under `docs/` are CRLF; keep them CRLF. Code files keep their current line endings. `git diff --stat` must show only your own lines.
- **R5 Privacy, for any event on a forwarded target (C1).** Never put a path, URL, host name, IP, port, account or user name, token, session id, or the `Display`/`Debug` of an error (`%e`, `?e`, `?code` of an error type) into a field or the message. Use closed codes instead: `reason` (snake_case), `error_kind` (`io::ErrorKind` as snake_case through `events::kind()`, C6), `http_status: u16`, `win32: u32`, counts, `duration_ms: u64`, and booleans. **Amendment 2 (D-LT18):** the same holds on the server for every telemetry, dev-session and telemetry-path `admin.request` row. An IP is dropped, never hashed. From LT-40 on, the C9 capture helpers panic on any captured IP literal.
  - **R5a (adoption only, D-LT14).** On the `adoption` target, the `entry` field may hold a path **relative to the chosen folder** (`/`-separated, `"."` for the folder), as D-AU7 already allows, or a manifest label (`seed`, `patch:<id>`, `file_index`, `manifest`). `expected` and `found` hold only sizes and digests in the C14 `digest()` form. Never an absolute path, a URL or a host.
- **R6 Log discipline.**
  - Levels: `info` for each state transition (one start, one outcome), `warn` for a recoverable failure the player sees, `error` for an unrecoverable one. `debug` stays local and is never forwarded.
  - No per-tick or per-progress events, and at most 20 forwarded events per operation.
  - The message names the system, the expected and actual result, and the consequence.
  - Numbers are logged as numbers. At most 12 fields per event; keys are snake_case ASCII.
- **R7 Tests.** Write unit tests ([TESTING.md](../../../TESTING.md)).
  - Every new test must fail when your change is reverted. Pin the level, the target and the `event` field (and `reason`), not the message text.
  - Use the C9 capture helper; never add a second one.
  - Put tracing calls in the packet's `events.rs` helper functions. Test each helper under capture, and drive at least one real failing path the packet names, so the call site is pinned too.
- **R8 Frontend.** The decoder uses `onExcessProperty:'error'`. A new or changed Rust DTO field and its TS schema land in the same commit. Run F1.
- **R9 Report and return.** Write `<worktree>\.claude\reports\launcher-telemetry\<packet>-coder.md`, with the files and their line counts, each check and its result, and anything you rejected and why. Return exactly `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. When the packet doesn't cover something, return `blocked`; don't invent a design.
- **R10 Lock file.** Every check runs `--locked`. Only LT-05 runs L0.

### Checks (copy the exact command)

| ID | Command |
|---|---|
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | the same with `-p cimmeria-launcher-desktop` |
| D2p | the same with `-p cimmeria-runtime-probe` |
| D3e `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` |
| D3s `<f>` | the same with `-p cimmeria-launcher-desktop <f>` |
| D3p `<f>` | the same with `-p cimmeria-runtime-probe <f>` |
| C1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-launcher-core` |
| C2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-launcher-core --all-targets -- -D warnings` |
| C3 `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-launcher-core <f>` |
| S1 | `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-admin-api` |
| S2 | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked -p cimmeria-admin-api --all-targets -- -D warnings` |
| S3 `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api --lib <f>` |
| S4 | `pwsh tools/build-lane/lane.ps1 cargo test --locked -p cimmeria-admin-api` |
| F1 | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend`, then `npm run check --prefix crates/launcher/desktop/frontend`, then `npm test --prefix crates/launcher/desktop/frontend` |
| L0 | the same as D2e without `--locked`, then `git diff --stat Cargo.lock crates/launcher/desktop/Cargo.lock`. Only dependency-edge lines may change: no new package and no version change. |

## Contract

### C1: launcher targets (D-LT5)

The canonical file is `crates/launcher-core/src/telemetry/launcher_targets.txt`, with LF line endings. LT-03 and LT-05 each create it **byte-identical** to this:

```
# Launcher event wire targets, one per line (D-LT5, docs/analysis/launcher-telemetry).
# Pinned by golden tests in cimmeria-launcher-engine and cimmeria-admin-api; change all three together.
client.launcher.adoption
client.launcher.startup
client.launcher.crash
client.launcher.update
client.launcher.game_update
client.launcher.install
client.launcher.repair
client.launcher.launch
client.launcher.settings
client.launcher.telemetry
client.launcher.ui
client.launcher.login
```

The engine table, in `engine/src/storage/game_telemetry/launcher_event.rs`:

```rust
/// Tracing target -> wire target. The only targets the launcher forwards (D-LT5).
pub const LAUNCHER_EVENT_TARGETS: [(&str, &str); 12] = [
    ("adoption", "client.launcher.adoption"),
    ("launcher.startup", "client.launcher.startup"),
    ("launcher.crash", "client.launcher.crash"),
    ("launcher.update", "client.launcher.update"),
    ("launcher.game_update", "client.launcher.game_update"),
    ("launcher.install", "client.launcher.install"),
    ("launcher.repair", "client.launcher.repair"),
    ("launcher.launch", "client.launcher.launch"),
    ("launcher.settings", "client.launcher.settings"),
    ("launcher.telemetry", "client.launcher.telemetry"),
    ("launcher.ui", "client.launcher.ui"),
    ("launcher.login", "client.launcher.login"),
];

pub fn wire_target(tracing_target: &str) -> Option<&'static str> {
    LAUNCHER_EVENT_TARGETS.iter().find(|(t, _)| *t == tracing_target).map(|(_, w)| *w)
}
```

The admin-api side is `pub(super) const LAUNCHER_TARGETS: [&str; 12]`, with the file's lines in the same order.

### C2: `LauncherRecord`

`LauncherRecord` gains `pub target: &'static str`, holding the **wire** target that the layer resolved through `wire_target`. `launcher_events` writes `record.target` where it used `LAUNCHER_EVENT_TARGET` before. Keep `LAUNCHER_EVENT_TARGET` as a constant only if something outside `launcher_event.rs` still uses it.

### C3: shell forwarding and file filter

In `shell/src/telemetry.rs`:

- The forward rule becomes `let Some(wire) = wire_target(metadata.target()) else { return }; if *metadata.level() > Level::INFO { return }`.
- `FORWARD_TARGET` is removed.
- The file filter (`Targets`) adds `("launcher", Level::INFO)` and keeps every existing entry.

### C4: the file-only target

The sender, the forwarder, the mint paths in `launcher_session.rs`, and the panic detail log only to `launcher.pipeline`, which is never in C1. The `launcher` file-filter prefix keeps it in the local log.

### C5: which subsystem uses which target

| Tracing target | Subsystems | Packets |
|---|---|---|
| `launcher.startup` | startup facts | LT-06 |
| `launcher.crash` | panic, previous-run panic | LT-06 |
| `launcher.update` | launcher self-update (existing apply events stay as they are) | LT-06 (host.rs), LT-12 |
| `launcher.game_update` | game update, release catalog, patch notes | LT-13 |
| `launcher.install` | install, seed archive, prerequisites, PhysX, migration | LT-14, LT-15 |
| `adoption` | adoption (AU, unchanged) | — |
| `launcher.repair` | repair, client maintenance, uninstall, client changes | LT-16 |
| `launcher.launch` | Play, launch worker, helper supervisor | LT-17, LT-18 |
| `launcher.settings` | preferences, both consents, server list, client patches | LT-19 |
| `launcher.telemetry` | game-session mint, game transport, pipeline recovery, log upload | LT-07, LT-08, LT-09a, LT-09b, LT-21 |
| `launcher.ui` | frontend reports | LT-10, LT-20 |
| `launcher.login` | reserved for oidc-login; no emitter in this campaign | — |

### C6: event vocabulary

- `event` is required, snake_case, and at most 40 characters.
- Operations use `<op>_started` (info), `<op>_finished` (info, `outcome` = `"ok"` or a closed code, plus `duration_ms`), and `<op>_failed` (warn, `reason`).
- Allowed fields: `operation: u64`, `action`, `outcome`, `reason`, `error_kind`, `http_status`, `win32`, `bytes`, `files`, `count`, `duration_ms`, `attempt`, `exit_code: i64`, `phase` (closed list `download` or `extraction`), `patch` (a signed-manifest patch id), plus the booleans a packet names (for example `resumed`, `recovering`, `removed`, `telemetry_dll`, `early`).
- Value sources (amendment 6). An `operation` that refers to an engine operation is `crate::operation_numbers::number(id)` of its `Uuid` (a process-local counter, first sight gets the next number; never the UUID or part of it, R5). An operation's `duration_ms` is `crate::operation_numbers::elapsed_ms(id)` unless the packet names a local `Instant`. A `reason` taken from an error comes from that type's `code()` (`ContractError`, `StorageError`, `IntentError` in `engine/src/error_codes.rs`; `InstallError` and `LocalSeedError` in `launcher-core/src/install_codes.rs`; both added by LT-14a/LT-14b) and matches the type's serde name where it has one. A packet that names no source for a field leaves the field out.
- Each area's helper file is `<area dir>/events.rs`, holding small `pub(crate) fn` wrappers around the `tracing::` calls. A helper the shell calls (`mark_uncertain_failed`, `dll_artifact_unavailable`) is `pub` in a `pub mod events`.
- `error_kind` uses a shared helper that LT-05 puts in the engine, `pub fn kind(e: &std::io::Error) -> &'static str` in `engine/src/storage/game_telemetry/launcher_event.rs`. It maps `NotFound`, `PermissionDenied`, `AlreadyExists`, `WouldBlock`, `InvalidInput`, `InvalidData`, `TimedOut`, `WriteZero`, `Interrupted`, `UnexpectedEof`, `OutOfMemory`, `StorageFull` (if the toolchain has it) and `Other` to snake_case, and everything else to `"other"`.

### C7: pipeline health (LT-07)

```rust
// engine/src/storage/game_telemetry/health.rs
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub struct FailureCounts { pub mint_failed: u64, pub held: u64, pub dropped: u64 }

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct TelemetryHealth {
    pub sent: u64,
    pub totals: FailureCounts,
    pub since_success: FailureCounts,
    pub last_outcome: Option<&'static str>, // "sent" | "held" | "mint_failed" | "dropped"
    pub last_reason: Option<&'static str>,  // the MintFailed reason (C8)
    pub last_success_ms: Option<u64>,
    pub first_failure_ms: Option<u64>,      // first failure since the last success
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recovery { pub failures: FailureCounts, pub outage_ms: u64 }

impl TelemetryHealth {
    /// `Empty` changes nothing. `Sent` after one or more failures returns
    /// `Some(Recovery)` and resets `since_success` and `first_failure_ms`.
    pub fn record(&mut self, outcome: &SendOutcome, now_ms: u64) -> Option<Recovery>;
}
pub type SharedHealth = std::sync::Arc<std::sync::Mutex<TelemetryHealth>>;
```

The shell holds one process-wide handle in `shell/src/telemetry.rs`: `pub(crate) fn health() -> SharedHealth`, backed by a `static OnceLock<SharedHealth>`.

### C8: the `SendOutcome` mint reason (LT-07)

`SendOutcome::MintFailed` becomes `MintFailed { reason: &'static str }`. The reason is one of `transport`, `timeout`, `http_status`, `decode`, `endpoint_refused` or (amendment 2) `no_https_endpoint`, used when `effective_upload_endpoint` (C10) returns `None`.

*Amended (directive 1):* there is **no** `endpoint_misconfigured` refusal and **no** `Outcome::EndpointMisconfigured`. A loopback endpoint from a remote login server is replaced, not refused (C10 `effective_upload_endpoint`), and the replacement is reported by one info event `upload_endpoint_overridden` on `launcher.telemetry` (LT-07, LT-08).

### C9: capture helper (LT-05; the only one)

Engine `engine/src/test_capture.rs` and shell `shell/src/test_capture.rs` are identical. Each is declared `#[cfg(test)] mod test_capture;` in `lib.rs` (engine) and `main.rs` (shell).

```rust
//! Test-only tracing capture (launcher-telemetry C9). Thread-local: run async code
//! inside `capture` on a current-thread runtime.
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::Context, prelude::*, Layer, Registry};

#[derive(Debug, Clone)]
pub(crate) struct Captured {
    pub target: String,
    pub level: tracing::Level,
    pub fields: BTreeMap<String, String>,
}

impl Captured {
    pub(crate) fn field(&self, key: &str) -> Option<&str> { self.fields.get(key).map(String::as_str) }
}

struct Cap(Arc<Mutex<Vec<Captured>>>);
struct Visit<'a>(&'a mut BTreeMap<String, String>);

impl tracing::field::Visit for Visit<'_> {
    fn record_str(&mut self, f: &tracing::field::Field, v: &str) { self.0.insert(f.name().into(), v.to_owned()); }
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) { self.0.insert(f.name().into(), format!("{v:?}")); }
}

impl<S: tracing::Subscriber> Layer<S> for Cap {
    fn on_event(&self, e: &tracing::Event<'_>, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        e.record(&mut Visit(&mut fields));
        self.0.lock().unwrap().push(Captured { target: e.metadata().target().into(), level: *e.metadata().level(), fields });
    }
}

pub(crate) fn capture<R>(f: impl FnOnce() -> R) -> (R, Vec<Captured>) {
    let store = Arc::new(Mutex::new(Vec::new()));
    let r = tracing::subscriber::with_default(Registry::default().with(Cap(store.clone())), f);
    let v = store.lock().unwrap().clone();
    (r, v)
}
```

The engine has no `tracing-subscriber` today. LT-05 adds `tracing-subscriber = { version = "0.3", default-features = false, features = ["registry", "std"] }` under the engine's `[dev-dependencies]` and runs L0.

### C10: endpoint sanity (LT-05)

```rust
// engine/src/storage/game_telemetry/endpoint_sanity.rs
/// True when the host of `url` is `localhost`, in 127.0.0.0/8 or `::1`. False when `url` does not parse.
pub fn is_loopback_url(url: &str) -> bool;
/// The server handed out a loopback upload endpoint, although the login
/// server it came from is not loopback, so uploads would go to the player's own machine.
pub fn misconfigured(login_server: &str, upload_endpoint: &str) -> bool {
    is_loopback_url(upload_endpoint) && !is_loopback_url(login_server)
}

/// The path every Cimmeria login port serves telemetry on.
pub const UPLOAD_PATH: &str = "/api/telemetry";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effective { pub url: String, pub overridden: bool }

/// Amendment 2 (directive 5): the built-in https upload URL. `None` until LT-41
/// sets it (D-LT17); no plain-http value is ever allowed here.
pub const DEFAULT_UPLOAD_ENDPOINT: Option<&str> = None;

/// D-LT7 (amended by directives 1 and 5): the upload endpoint the launcher uses.
/// `advertised` wins unless `misconfigured(login_server, advertised)`. Then:
/// 1. an `https` login server gives `https://<host[:port]>` of `login_server` plus
///    UPLOAD_PATH (`overridden = true`), so `https://play.example` gives
///    `https://play.example/api/telemetry`;
/// 2. otherwise `DEFAULT_UPLOAD_ENDPOINT` (`overridden = true`) when it is `Some`;
/// 3. otherwise `None`. A plain-http login server never yields a derived URL.
/// Also `None` when `login_server` does not parse.
pub fn effective_upload_endpoint(login_server: &str, advertised: &str) -> Option<Effective>;
```

Parse with the URL parser `storage/launcher_summary/endpoint.rs` already uses. Add no new dependency. The launcher reads **no** environment override for this (the server's `CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT` stays the only explicit override). A non-loopback advertised URL is kept whatever its scheme: it is the operator's explicit override (D-LT3), and the server WARNs about plain http at boot.

### C11: server upload endpoint (LT-01)

```rust
// crates/admin-api/src/routes/dev_session/upload_endpoint.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EndpointSource { Env, RequestHost, Default }
impl EndpointSource { pub(crate) fn as_str(self) -> &'static str } // "env" | "request_host" | "default"
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedEndpoint { pub url: String, pub source: EndpointSource }
/// Directives 1 and 5: the built-in https Cimmeria URL. `None` until LT-41 sets it
/// (D-LT17: the colo has no https endpoint yet). Never a plain-http value.
pub(crate) const DEFAULT_UPLOAD_ENDPOINT: Option<&str> = None;
/// In order: a non-blank env wins (the explicit override, used as given); else a
/// valid Host with `x-forwarded-proto` exactly `https` gives `https://{host}/api/telemetry`;
/// else a loopback Host from a loopback peer (a one-host dev box) gives
/// `http://{host}/api/telemetry`; else `DEFAULT_UPLOAD_ENDPOINT`; else `None`.
/// No other plain-http URL is ever derived.
pub(crate) fn resolve(env: Option<&str>, headers: &axum::http::HeaderMap, peer_is_loopback: bool)
    -> Option<ResolvedEndpoint>;
/// The Host value is a DNS name or IPv4 (`[A-Za-z0-9.-]`, 1..=253 chars) or a bracketed
/// IPv6 address, plus an optional `:port` of 1..=5 digits. Nothing else.
pub(crate) fn valid_host(host: &str) -> bool;
pub(crate) fn is_loopback_url(url: &str) -> bool;
/// The boot WARN reason for the env value, else None:
/// - `upload_endpoint_loopback` when the env is a loopback URL;
/// - `upload_endpoint_plain_http` when it is a non-loopback `http://` URL;
/// - `upload_endpoint_unset` when the env is unset and DEFAULT_UPLOAD_ENDPOINT is None,
///   so only https-proxied or one-host dev mints can succeed.
/// An unset env with a Some default is normal and is INFO only.
pub(crate) fn boot_warning(env: Option<&str>) -> Option<&'static str>;
/// At most one WARN per 600 s (static AtomicU64 of the last-warn unix second), plus a `suppressed` count.
pub(crate) fn warn_if_unreachable(url: &str, peer_is_loopback: bool);
```

`EndpointSource` gains `LoopbackDev` (`"loopback_dev"`). No log line built from C11 carries the URL when its source is `request_host` or `loopback_dev`, because a Host can be an IP literal. They carry `upload_endpoint_source` and `upload_endpoint_https: bool` instead. Only the boot line may print the env or default URL, and only when its host is a DNS name.

### C12: refusal logs (LT-02, LT-38, LT-39; re-specified in amendment 2)

**Correction.** `origin/main` already has `crates/admin-api/src/routes/telemetry/refusal_log.rs`, a throttled `RefusalLog` for chunk and bundle uploads. It uses `event = upload_refused | upload_auth_refused` on target `launcher.ingest`, has a doubling window and a `suppressed` count, and writes `peer = %who.peer` at :332, :375 and :401. The old C12 (a new ≤120-line `note`) would have collided with it, so it is replaced by three pieces.

1. **Ingest rows (LT-02).** `RefusalLog` keeps its API and throttle. The `peer` field is deleted from all three rows. The peer may stay as the in-memory throttle `subject` when there is no session id, because it is never written.
2. **Dev-session refusals (LT-38).** These move out of `dev_session/handlers.rs:442-498` into a new file:

   ```rust
   // crates/admin-api/src/routes/dev_session/refusal_log.rs
   /// One row per (route, reason) per 60 s at the reason's existing level, plus a
   /// `suppressed: u64` count of the rows held back since the last one. No peer.
   pub(super) fn log_refusal(route: &'static str, err: &AuthError);
   ```

3. **Summary refusals (LT-39).** These go in `launcher_summary/refusal.rs`:

   ```rust
   /// WARN target "launcher.ingest", event "upload_refused", route "launcher_summary",
   /// fields reason, status: u16, suppressed: u64. At most once per reason per 60 s.
   pub(super) fn note(reason: &'static str, status: u16);
   ```

No telemetry or dev-session log row carries a peer address, an IP literal, a token or a body, and nothing is hashed (D-LT18). In-memory quota keys (`ip_key`) and the game-session stream lookup (`StreamRecord.peer_ip`) are functional state, not log fields. They stay, and they must never be written to a row.

### C13: Tauri commands (LT-10, LT-11)

```rust
// shell/src/host/ui_event.rs
pub(crate) const UI_AREAS: [&str; 16] = ["install", "adoption", "update", "game_update", "launch", "repair",
    "settings", "seed_archive", "server_list", "log_upload", "physx", "migration", "patch_notes",
    "client_patches", "client_maintenance", "telemetry"];
#[tauri::command]
pub(crate) fn ui_event(area: String, event: String, code: String, level: String) -> Result<(), String>;

// shell/src/host/telemetry_status.rs
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(crate) struct TelemetryStatus {
    pub last_outcome: Option<&'static str>,
    pub last_reason: Option<&'static str>,
    pub last_success_age_s: Option<u64>,
    pub failures_since_success: u64,
}
#[tauri::command]
pub(crate) fn telemetry_status() -> TelemetryStatus;
```

The serde casing follows whatever the DTOs in `shell/src/host/game_telemetry.rs` use. The TS side lives in `frontend/src/ui-event.ts` (`reportUiEvent`) and `frontend/src/telemetry-status-view.ts`.

### C18: forwarded rate-limit key (LT-42, amendment 4, D-LT19)

C14-C17 are in [work-packets-adoption.md](work-packets-adoption.md).

```rust
// crates/admin-api/src/routes/dev_session/forwarded_for.rs
/// D-LT19 (directive 7a). The address a per-IP quota or a per-peer upload slot counts.
/// When `peer.to_canonical().is_loopback()` (the colo's board-proxy Caddy forwards the
/// https routes to loopback :8081) and the request has an `X-Forwarded-For` header, this
/// is the RIGHTMOST comma-separated entry of the LAST such header, trimmed, if it parses
/// as a bare `IpAddr` (no port, no brackets, valid UTF-8), returned `.to_canonical()`.
/// In every other case it is `peer`. A non-loopback peer's header is ignored entirely.
/// The result goes only into `quota::ip_key(..)`; it is never logged, stored, returned,
/// or put in a field, span, `Uploader`, `RefusalLog` subject or `StreamRecord` (D-LT18).
pub(crate) fn rate_limit_ip(peer: IpAddr, headers: &axum::http::HeaderMap) -> IpAddr;
```

Rightmost, because it is the address the proxy itself saw; anything to its left came from the caller. The socket peer still drives everything else: `StreamRecord.peer_ip`, LT-01's `peer_is_loopback` and `warn_if_unreachable`, the `RefusalLog` throttle subject and the request span (D-LT21).

## Wave 0

### LT-00: SigNoz triage (read-only)

- **Agent:** general-purpose, read-only. No worktree.
- **Writes:** only `docs/analysis/launcher-telemetry/worknotes/LT-00.md`, CRLF. It blocks nothing.
- **Do:**
  - Use the SigNoz MCP. If it is down, use ClickHouse over `ssh cimmeria-colo` (`.claude/agent-memory`, reference "signoz clickhouse via ssh").
  - Over the last 14 days, count:
    1. `cimmeria-server` rows whose body starts with "Minted dev-session telemetry token", by `session_kind`;
    2. the boot line "dev-session telemetry:", recording only whether its `upload_endpoint` is loopback (yes/no);
    3. `cimmeria-client` rows by `client_target`, for those starting `client.launcher.` and for `launcher.summary`;
    4. `cimmeria-server` rows with target `launcher.ingest`.
- **Rules:** commit nothing. Write counts only: no IPs, names, session ids, paths or host names, and never the colo address.
- **Return:** `LT-00 done - <path>`.

## Wave 1: server (PR S)

### LT-01: upload endpoint derivation and loud misconfiguration

- **Host:** windows. **Worktree:** `lt-01`, branch `launcher-telemetry/lt-01-upload-endpoint`, from the `lt-srv-integration` tip. **Rules:** R1-R7, R9, R10. **Contract:** C11.
- **Why:** F3 and owner directives 1, 5 and 6. Today a server without the env var tells every remote launcher to upload to localhost. This packet removes that fallback. Afterwards the endpoint comes from the explicit env override, an https-proxied Host, or a one-host dev box, and otherwise from the built-in https default. That default is `None` until LT-41 (D-LT17), and while it is `None` an unconfigured server refuses the mint loudly rather than handing out a plain-http or loopback URL. This packet replaces the old "set the colo env var" quick win (OQ-LT5 is now only a verification step, LT-27 U1). It also logs no IP and no derived URL (directive 6).
- **Files:**
  - new `crates/admin-api/src/routes/dev_session/upload_endpoint.rs` (≤200 lines), with the code from C11;
  - new `crates/admin-api/src/routes/dev_session/upload_endpoint_tests.rs`;
  - `dev_session/mod.rs`: declare both modules (`#[cfg(test)]` for the tests) and change `log_boot_status` (lines 127-142);
  - `dev_session/handlers.rs` (~518 lines, so add call lines only).
- **Change:**
  1. Move `upload_endpoint_env()` and `upload_endpoint_from()` (handlers.rs:512-521) into `upload_endpoint.rs`, and **delete** `DEFAULT_UPLOAD_ENDPOINT = "http://localhost:8443/api/telemetry"` (handlers.rs:30). The new `pub(crate) const DEFAULT_UPLOAD_ENDPOINT` in `upload_endpoint.rs` is the C11 value. Afterwards `rg -n "localhost:8443" crates/admin-api/src` must find no production line (tests may keep it as an explicit env value). Fix the handlers.rs:22-25 doc comment to say the env var is an optional override. Update `session_kind_tests.rs:151-157` only where it pinned the old default.
  2. In `mint_inner` and `refresh_inner`, replace the endpoint value (response fields near :317 and :416) with `upload_endpoint::resolve(env.as_deref(), &headers, peer.ip().is_loopback())`.
     - If the handlers do not take `headers: HeaderMap` yet, add the extractor to `mint` (:201) and to the refresh handler, and pass it down.
     - On `None`, refuse **before** signing a token: return 503 with the JSON error code `no_upload_endpoint`, in the existing error-body shape (grep `IntoResponse` in `dev_session/`). Log nothing here, because LT-38 owns the refusal rows; add only a `// LT-38 logs this refusal` comment. Never fall back to http or loopback.
  3. In the mint INFO at :297, add `upload_endpoint_source = resolved.source.as_str()` and `upload_endpoint_https = resolved.url.starts_with("https://")`. **Never the URL** (a Host can be an IP literal). Then call `warn_if_unreachable(&resolved.url, peer_is_loopback)`. That WARN is: `reason = "upload_endpoint_unreachable_for_peer"`, the message "dev-session telemetry: advertised a loopback upload endpoint to a remote launcher; its uploads will go to its own machine and be lost", and the fields `upload_endpoint_source` and `suppressed`. **No URL and no peer field.**
  4. In `log_boot_status`:
     - env unset, default `Some`: the INFO says "dev-session telemetry: upload endpoint is the built-in https default (or an https-proxied Host); CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is an optional override", with `upload_endpoint_source = "default"`. No WARN.
     - env unset, default `None` (until LT-41): a WARN with `reason = "upload_endpoint_unset"` and "dev-session telemetry: no https upload endpoint is configured; remote mints are refused (503 no_upload_endpoint) until CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is set or the built-in https default ships".
     - env loopback (`upload_endpoint_loopback`): a WARN, "dev-session telemetry: CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is loopback; expected a public https URL; remote launchers will replace it".
     - env non-loopback http (`upload_endpoint_plain_http`): a WARN, "dev-session telemetry: CIMMERIA_TELEMETRY_UPLOAD_ENDPOINT is plain http; the explicit override is used as given, but uploads cross the network unencrypted; use https".
     - Every boot line prints the env URL only when its host is a DNS name. Otherwise it prints `upload_endpoint_source` alone.
- **Tests:**
  - Unit tests in `upload_endpoint_tests.rs`:
    - the precedence of `resolve`: env wins; a blank env falls through; a valid Host with `x-forwarded-proto: https` gives `https://play.example/api/telemetry`; a valid Host **without** it from a non-loopback peer gives `None` (reverting to the old http Host derivation fails this); `Host: localhost:8443` from a loopback peer gives `http://localhost:8443/api/telemetry` (`loopback_dev`); the same Host from a non-loopback peer gives `None`; a missing or invalid Host (`evil/x`, `a b`, 300 chars) gives `None`;
    - a test that `DEFAULT_UPLOAD_ENDPOINT` is `None` or starts with `https://`, so LT-41 cannot ship an http default;
    - `boot_warning`: `None` env gives `upload_endpoint_unset`, `http://127.0.0.1:8081/api/telemetry` gives `upload_endpoint_loopback`, `http://play.example:8081/api/telemetry` gives `upload_endpoint_plain_http`, and `https://play.example/api/telemetry` gives `None`;
    - `valid_host` cases, including `[::1]:8081`;
    - `is_loopback_url` cases;
    - log tests for the boot WARNs and for the throttled unreachable WARN (two calls, one event).
  - For log capture, copy the layer pattern in `routes/telemetry/replay_tests.rs`; admin-api has no `LogCapture`.
  - Router tests, copied from an existing mint test in `dev_session/` (grep `dev-session` in its tests):
    1. `Host: play.example`, `x-forwarded-proto: https`: the JSON `upload_endpoint` is `https://play.example/api/telemetry`.
    2. The same request with no `x-forwarded-proto`, from peer `198.51.100.7`: 503 `no_upload_endpoint`, and no token is minted. Reverted, it is localhost or http, so the test fails.
  - **No-IP test (directive 6).** Send a mint from peer `198.51.100.7` with `Host: 203.0.113.9:8081` and `x-forwarded-proto: https`. Assert that no captured message or field value contains `198.51.100.7` or `203.0.113.9`. Reverting to a mint INFO that carries `upload_endpoint` fails it.
  - Existing tests that send no Host and pinned `localhost:8443` must now either send a loopback Host from a loopback peer or set the env explicitly. Change only the setup, and list each changed test in the report.
- **Checks:** S1, S2, S3 `routes::dev_session`, then S4.
- **Docs:** none (LT-04).
- **Commit:** `LT-01: https-only telemetry upload endpoint resolution, no localhost fallback`.

### LT-02: no IP in ingest refusal rows or telemetry request rows (re-specified, amendment 2)

- **Host:** windows. **Worktree:** `lt-02`, branch `launcher-telemetry/lt-02-ingest-no-ip`, from the `lt-srv-integration` tip. **Rules:** R1-R7, R9, R10. **Contract:** C12 item 1.
- **Why:** directive 6 and F16. The chunk and bundle refusal logging F4 asked for already exists on `main` (`routes/telemetry/refusal_log.rs`), but every row writes the peer IP. The `admin.request` row writes it too, on every telemetry and dev-session request. The summary half of the old LT-02 is now LT-39.
- **Files:**
  - `crates/admin-api/src/routes/telemetry/refusal_log.rs` (~450 lines, so edit lines only): delete `peer = %who.peer,` at :332, :375 and :401. Change the module doc at :39-42 to say the rows carry the token's ids and never the peer address. Leave `let peer = …` / `subject` (:303-304, :360-361, :389-390) as they are; they are the in-memory throttle key and are never written;
  - its tests in `routes/telemetry/upload_limits_tests/refusal_log.rs` (the peer is `198.51.100.7` at :229);
  - `crates/admin-api/src/request_span.rs` (`admin_request_row`, :66-105) and its tests (the `peer=127.0.0.1` pin at :257).
- **Change:**
  1. The three `launcher.ingest` rows lose `peer`.
  2. `admin_request_row`: when the matched `path` starts with `/api/telemetry` or `/api/auth/dev-session`, `peer` is `None`, so the field is absent. Other admin paths keep their peer; they are operator traffic on a loopback-published port and out of scope. Add `fn is_telemetry_path(path: &str) -> bool` next to `row_level`. Also grep `admin_request_row` in `login_port.rs`: if the login-port router layers it, the same rule already covers it, because that router serves only telemetry routes.
- **Tests:**
  - In `upload_limits_tests/refusal_log.rs`: for each of `report`, `report_truncated` and `report_bad_rows`, assert that the captured row has no `peer` key and that no field value or message contains `198.51.100.7`. Reverted, the field is back and all three fail. Any existing assertion that a `peer` is present is flipped; list each one in the report.
  - In `request_span.rs` tests: a request to `/api/telemetry/upload-chunk` (copy the test near :237) from a connect-info peer gives an `admin.request` row with no `peer`. The existing `peer=127.0.0.1` pin for a non-telemetry admin path stays green.
  - The throttle tests stay unchanged; the subject key still uses the peer.
- **Checks:** S1, S2, S3 `routes::telemetry`, S3 `request_span`, then S4.
- **Docs:** none (LT-04).
- **Commit:** `LT-02: drop the peer address from ingest refusal and telemetry request rows`.

### LT-38: dev-session refusal logs: no peer, throttled with a count (amendment 2)

- **Host:** windows. **Worktree:** `lt-38`, branch `launcher-telemetry/lt-38-dev-session-refusals`, from the `lt-srv-integration` tip **after LT-01 has merged into it** (both edit `dev_session/handlers.rs` and `mod.rs`). **Rules:** R1-R7, R9, R10. **Contract:** C12 item 2.
- **Why:** directive 6 (OQ-LT3 decided: drop, do not hash). `log_refusal` (`dev_session/handlers.rs:442-498`) writes `peer = %peer` on all seven arms and has no throttle, so a retrying launcher writes one row with its IP per request.
- **Files:**
  - new `crates/admin-api/src/routes/dev_session/refusal_log.rs` (≤200 lines, with an inline `#[cfg(test)] mod tests`), declared in `dev_session/mod.rs`;
  - `dev_session/handlers.rs` (over 450 lines, so call lines only): delete `log_refusal` (:438-498) and change the two `.inspect_err(|e| log_refusal("mint", peer.ip(), e))` calls (:213, :229) to `.inspect_err(|e| refusal_log::log_refusal("mint" | "refresh", e))`. Also log LT-01's 503 `no_upload_endpoint` refusal through `refusal_log::log_no_endpoint(route)` (reason `dev_session_no_upload_endpoint`, WARN);
  - the dev-session tests that use `198.51.100.21`, `198.51.100.9` and `203.0.113.5` (`dev_session/tests.rs:469`, `:507`, `:550`), only where they pin a `peer` field.
- **Change:**
  1. Move the match from `handlers.rs:443-497` into `refusal_log.rs` unchanged: the same arms, levels, `reason` values and fields (`scope`, `retry_after_secs`, `elapsed`, `cap`, `exp`, `now`). **Remove `peer` from every arm.** Never hash it.
  2. Throttle each (route, reason) pair with a `static Mutex<HashMap<(&'static str, &'static str), Slot>>`: the first row writes at once, repeats inside 60 s are counted, and the next row carries `suppressed: u64`. The key holds no address. The `debug` arm (`dev_session_bad_request`) is not throttled, because it is never forwarded.
  3. Keep the message `"dev-session request refused: {err}"` only where `err`'s `Display` holds no address. Grep the `AuthError` `Display` impl, and where an arm's `Display` could hold a caller-supplied string, use a fixed message with the reason instead. Say which arms in the report.
- **Tests:**
  - A throttle test: the same (route, reason) twice gives one row, the next row after the window gives `suppressed = 1` (inject `now` the way `routes/telemetry/refusal_log.rs` does), and a different reason gives its own row.
  - **No-IP test (directive 6).** Drive a real refused mint from peer `198.51.100.7` (copy the quota test at `tests.rs:~469`, which already exceeds the quota from a fixed peer) and assert that the WARN has `reason = dev_session_quota_exceeded`, no `peer` key, and no `198.51.100.7` in its message or any field. Reverted, `peer` is back and the test fails.
  - One `no_upload_endpoint` case: LT-01's 503 writes one `dev_session_no_upload_endpoint` WARN with no peer.
- **Checks:** S1, S2, S3 `routes::dev_session`, then S4.
- **Docs:** none (LT-04).
- **Commit:** `LT-38: dev-session refusal logs without the peer address, throttled with a count`.

### LT-39: launcher-summary whole-request refusal rows (the summary half of the old LT-02)

- **Host:** windows. **Worktree:** `lt-39`, branch `launcher-telemetry/lt-39-summary-refusals`, from the `lt-srv-integration` tip. **Rules:** R1-R7, R9, R10. **Contract:** C12 item 3.
- **Why:** F4. `launcher_summary/handlers.rs` has no `tracing::` call, so a refused summary (quota, bad body, ingest paused) is invisible.
- **Files:** new `crates/admin-api/src/routes/telemetry/launcher_summary/refusal.rs` (≤120 lines, with an inline `#[cfg(test)] mod tests`), declared in `launcher_summary/mod.rs`; `launcher_summary/handlers.rs` (`ingest`, :90, call lines only).
- **Change:** where `ingest` turns an `ingest_inner` whole-request refusal into its response, call `refusal::note(code, status)` with the refusal's existing closed code and status (grep its error type and `IntoResponse`). If the type has no closed code accessor, add `fn code(&self) -> &'static str` next to it, one snake_case code per variant named after the variant. Don't invent semantics. The `peer_ip` parameter (:161) stays a quota key and is never passed to `note`.
- **Tests:** `note` throttles (two calls, one row; then another reason gives a second row). Add one router case, copying a quota-exceeded test in `launcher_summary/tests/quota.rs` (peer `203.0.113.78`): exactly one `upload_refused` row with `route = launcher_summary`, the quota code, status 429 or whatever the test already expects, and no `203.0.113.78` anywhere in it. Reverted, there is no row and the test fails.
- **Checks:** S1, S2, S3 `routes::telemetry::launcher_summary`, then S4.
- **Docs:** none (LT-04).
- **Commit:** `LT-39: log refused launcher summaries (throttled, no peer)`.

### LT-03: server accepts the 12 launcher targets

- **Host:** windows. **Worktree:** `lt-03`, branch `launcher-telemetry/lt-03-targets`, from the `lt-srv-integration` tip. `routes/telemetry/launcher_rows.rs` is on `main` (AU 2a, via #1953; amendment 3 anchor table); if `LAUNCHER_TARGETS` is not `[&str; 1]` there, return `blocked`. **Rules:** R1-R7, R9, R10. **Contract:** C1.
- **Files:**
  - new `crates/launcher-core/src/telemetry/launcher_targets.txt`, byte-identical to C1;
  - `crates/admin-api/src/routes/telemetry/launcher_rows.rs`;
  - its test module (grep `allowed_target` in the tests).
- **Change:**
  1. `LAUNCHER_TARGETS: [&str; 12]`, in C1 order.
  2. Any existing test that uses a second `client.launcher.*` target as the "refused" example switches to `client.launcher.bogus`.
- **Tests:**
  - A golden test, `launcher_targets_match_the_canonical_file`. It reads `concat!(env!("CARGO_MANIFEST_DIR"), "/../launcher-core/src/telemetry/launcher_targets.txt")`, drops blank lines and lines starting with `#`, and asserts that the result equals `LAUNCHER_TARGETS`.
  - One accept test for `client.launcher.install` and one refuse test for `client.launcher.bogus`.
- **Checks:** S1, S2, S3 `routes::telemetry`, then S4.
- **Docs:** none (LT-04).
- **Commit:** `LT-03: accept the launcher-telemetry target list on the server`.

### LT-42: per-IP rate-limit key from `X-Forwarded-For`, loopback peers only (amendment 4)

- **Host:** windows. **Worktree:** `lt-42`, branch `launcher-telemetry/lt-42-forwarded-rate-key`, from the `lt-srv-integration` tip **after LT-01, LT-02, LT-38 and LT-39 have merged into it**. It changes nothing those packets own; it only edits lines next to theirs. **Rules:** R1-R7, R9, R10. **Contract:** C18, and C12's no-IP rule.
- **Why:** directive 7(a), F24. `https://play.cimmeria.app` is the board's Caddy proxy forwarding to loopback :8081, so every https caller reaches the server from the proxy's address and shares one bucket: summary 12/min (`launcher_summary/handlers.rs:42`), chunk 600/min and bundle 30/h (`upload_gate.rs:71`, `:77`), mint 120/h (`dev_session/handlers.rs:38`), refresh 480 and bad-refresh 30 (`:44`, `:47`), and the one-upload-per-peer slot (`SLOTS_PER_PEER`, `routes/telemetry/mod.rs:194`). At scale players 429 and 503 each other. This packet makes the forwarded client address the key of those counters, and nothing else.
- **Files:**
  - new `crates/admin-api/src/routes/dev_session/forwarded_for.rs` (≤160 lines, with an inline `#[cfg(test)] mod tests`), the C18 code; declare `pub(crate) mod forwarded_for;` in `dev_session/mod.rs` next to `quota`;
  - `routes/telemetry/upload_gate.rs`, `admit` (:281-349): after the token verifies, `let rate_ip = forwarded_for::rate_limit_ip(who.peer, headers);`, then `ip_key(rate_ip)` at :329 (the ip quota) and at :342 (`take_slot`). `who.peer` and `Uploader` stay unchanged;
  - `routes/telemetry/launcher_summary/handlers.rs`, `ingest_inner` (:158-176): `ip_key(forwarded_for::rate_limit_ip(peer_ip, request.headers()))` at :171, before `into_parts`. Nothing else in the file;
  - `routes/dev_session/handlers.rs` (over 450 lines, call lines only): the `ip_key(peer_ip)` calls in `mint_inner` (:250) and `refresh_inner` (:345, :355). Grep the inner signatures after LT-01. If they take `&HeaderMap`, compute `rate_ip` inside them. Otherwise add `rate_ip: IpAddr` right after `peer_ip`, compute it in the `mint` and refresh handlers from `peer.ip()` and the headers LT-01 extracts, and pass `rate_ip = peer_ip` from every test caller (list them in the report). `record_stream(.., peer_ip)` (:293, :403) and LT-01's loopback arguments keep the socket peer;
  - `crates/admin-api/src/login_port.rs`, the doc comment at :46-60 only: one sentence that the mint, refresh, summary and upload quotas key on the forwarded address when the peer is a loopback proxy (C18);
  - new tests in `launcher_summary/tests/quota.rs`, `upload_limits_tests/gate.rs` and `dev_session/tests.rs`. Existing tests stay unchanged.
- **Never (D-LT19, directive 6):** the header value and `rate_ip` go only into `ip_key(..)`. Not into a tracing field or message, `Uploader`, `RefusalLog`, `StreamRecord`, `upload_endpoint::resolve`, a response body, a row or a span. Add no log line in this packet.
- **Tests (unit, TESTING.md; TEST-NET addresses only):**
  1. `forwarded_for` table. Loopback peer `127.0.0.1` with `198.51.100.7` gives `198.51.100.7`; `::1` with `2001:db8::7` gives `2001:db8::7`; peer `::ffff:127.0.0.1` counts as loopback; `203.0.113.9, 198.51.100.7` gives the rightmost; two headers (`203.0.113.9`, then `198.51.100.7`) give the last; `::ffff:198.51.100.7` gives `198.51.100.7`. Loopback with no header, `""`, `unknown`, `198.51.100.7:443`, `[2001:db8::7]` or non-UTF-8 bytes gives `127.0.0.1`. **Spoof:** non-loopback peer `203.0.113.9` with `198.51.100.7` gives `203.0.113.9`.
  2. Summary, own allowance: `per_ip = 1`, loopback peer, XFF `198.51.100.7` then `198.51.100.8`, both served; `198.51.100.7` again is 429. Copy the `post(peer, n)` harness at `quota.rs:~277`. Reverted, the second request is 429 and the test fails.
  3. Summary spoof: `per_ip = 1`, peer `203.0.113.9`, XFF `198.51.100.7` then `198.51.100.8`: the second is 429.
  4. Upload gate: copy the `chunk_per_ip: 2` setup at `upload_limits_tests/gate.rs:~236`. From a loopback peer, three `admit` calls with three different XFF values all pass the ip quota and each take a peer slot while the earlier `Admitted` values are still held. Reverted, the second gets `Busy` (or 429) and the test fails. Run it once for `Route::Bundle` too. Spoof case: a non-loopback peer that rotates XFF is refused at the third call.
  5. Mint: copy `mint_refuses_one_address_past_the_per_ip_quota` (`tests.rs:136`). From a loopback peer, distinct XFF values mint past the per-ip quota; one repeated XFF is refused at quota + 1; a non-loopback peer that rotates XFF is refused at quota + 1.
  6. **IP-leak regression (directive 6).** Under log capture (reuse the helper LT-38 or LT-39 added; grep), drive a refused summary (429), a refused mint (quota) and an admitted chunk, each from a loopback peer with XFF `198.51.100.7`. Assert that no captured message or field value contains `198.51.100.7`. Prove that it can fail: add `client_ip = %rate_ip` to one row, watch the test fail, remove the line, and say so in the report.
- **Checks:** S1, S2, S3 `routes::dev_session`, S3 `routes::telemetry`, then S4.
- **Docs:** none (LT-04).
- **Parallel read-only pass:** network-security-auth (trust boundary), notes `A<k>`.
- **Commit:** `LT-42: per-IP rate-limit key from X-Forwarded-For behind a loopback proxy`.

### LT-04: server docs for wave 1

- **Host:** windows. **Worktree:** `lt-04`, from `lt-srv-integration` after LT-01..LT-03, LT-38, LT-39 and LT-42. **Rules:** R2, R4, R9. Docs only, so no cargo.
- **Files:** `docs/operations/telemetry.md` (the endpoint policy at lines ~96-153, the colo setup at ~155-219, the mint-quota proxy caveat at ~382-401, the summary proxy caveat at ~703-711, the endpoints at ~780-800) and `docs/architecture/observability.md` (the client index, ~196-257).
- **Write:**
  - The resolution order: env, then an https-proxied Host, then a one-host dev box, then the built-in https default (`None` until LT-41), then 503 `no_upload_endpoint`. Plain http is never derived, and an http env override WARNs.
  - The four boot WARN reasons (`upload_endpoint_unset`, `_loopback`, `_plain_http`, `_unreachable_for_peer`), the `upload_refused` and `upload_auth_refused` reasons, the summary refusals (LT-39) and the dev-session refusals with `suppressed` (LT-38), with one SigNoz/ClickHouse filter each.
  - The privacy rule (D-LT18): no telemetry, dev-session or `admin.request` row on a telemetry path carries a peer address or any IP, and nothing is hashed. Correct `docs/operations/telemetry.md` wherever it says the rows carry the peer (grep `peer`).
  - Replace the "colo is plain http :8081" guidance (~135-202) with the https requirement. **Amendment 4:** the Cloudflare Tunnel in "Pointing the launcher…" (~145) and in "Enable client telemetry on the colo" steps 2-3 (~165-202) was never deployed; delete it. Describe the real route instead: `https://play.cimmeria.app` is served by the board's Caddy reverse proxy (`board-proxy`), which forwards only five routes (POST `/api/auth/dev-session`, `/api/auth/dev-session/refresh`, `/api/telemetry/upload-chunk`, `/api/telemetry/upload-bundle`, `/api/telemetry/launcher-summary`) to the server's login port on loopback 8081, and answers 404 for everything else; plain http :8081 still serves the same routes. The Caddyfile is host config, mirrored in the private agent-board repo (README § Owner and ops tasks, OPS-LT1). Rollback: restore the newest `Caddyfile.bak-*` and run `docker restart board-proxy`. LT-41 makes that origin the built-in default; until it ships the default is `None`. No node names and no addresses.
  - **Amendment 4, the proxy quota caveats (~382-401 and ~703-711):** the server now trusts `X-Forwarded-For` only from a loopback peer, takes the rightmost entry, and uses it only as the per-IP quota and upload-slot key (LT-42). It never logs or stores it. Delete any sentence saying the server never reads the header. Keep the "raise the quota" advice for every other shared address: a NAT, any non-loopback proxy, and a container that sees the proxy as its bridge gateway rather than loopback (D-LT20; say this is unverified on the colo).
  - The 12 accepted targets in the client index table.
  - A "No launcher rows in SigNoz" checklist: the boot WARN, then mint INFO `upload_endpoint_source`, then `upload_refused`, then client rows.
  - No addresses. CRLF.
  - Walk `docs/agents/doc-update-map.md` and list the rows you touched in the report.
- **Commit:** `LT-04: document the upload endpoint derivation and refusal logs`.

## Wave 2: launcher pipeline (into `lt-app-integration`)

### LT-05: forward every launcher target, capture helpers, endpoint sanity

- **Host:** windows. **Worktree:** `lt-05`, branch `launcher-telemetry/lt-05-forward`, from the `lt-app-integration` tip. **Rules:** R1-R10 (this packet runs L0). **Contract:** C1, C2, C3, C6 (`kind`), C9, C10.
- **Files:**
  - `engine/src/storage/game_telemetry/launcher_event.rs` (~145 lines): add the table, `wire_target`, `kind`, and `LauncherRecord.target`;
  - new `engine/src/storage/game_telemetry/endpoint_sanity.rs`, declared in `game_telemetry/mod.rs`;
  - new `engine/src/test_capture.rs`, declared in the engine `lib.rs`;
  - `engine/Cargo.toml` (`[dev-dependencies]`);
  - `shell/src/telemetry.rs` (~185 lines): C3, and fill `LauncherRecord.target` in the layer;
  - new `shell/src/test_capture.rs`, declared in `shell/src/main.rs` (one line);
  - new `crates/launcher-core/src/telemetry/launcher_targets.txt`, byte-identical to C1.
  - First grep the engine and shell `src/` for `fn on_event`. If a test capture layer already exists, use it as C9 and say so in the report.
- **Tests:**
  - `wire_target` for each row plus an unknown target.
  - A golden test that the table's wire column equals the file. Read it with `concat!(env!("CARGO_MANIFEST_DIR"), "/../../../launcher-core/src/telemetry/launcher_targets.txt")`; the engine sits at `crates/launcher/desktop/engine`.
  - `launcher_events` writes `record.target`.
  - `endpoint_sanity` cases: `http://localhost:8443/api/telemetry` from `http://play.example:8081` is misconfigured; loopback from loopback is not; https remote is not.
  - `effective_upload_endpoint` (amendment 2):
    - (`https://play.example`, `http://localhost:8443/api/telemetry`) gives `https://play.example/api/telemetry`, `overridden=true`;
    - (`http://play.example:8081`, `http://localhost:8443/api/telemetry`) gives `None` while `DEFAULT_UPLOAD_ENDPOINT` is `None`. Reverting to the http derivation fails this;
    - a non-loopback advertised URL is kept with `overridden=false`;
    - (`http://127.0.0.1:8081`, loopback) is kept;
    - a login server that does not parse gives `None`;
    - `DEFAULT_UPLOAD_ENDPOINT` is `None` or starts with `https://`.
  - A shell test that an event on `launcher.install` is forwarded with wire `client.launcher.install`, while `launcher.pipeline` and a `debug` on `launcher.install` are not. Use the existing forward-layer tests (`ForwardLayer::new(SyncSender)`) as the model.
  - Reverted, `client.launcher.install` is not forwarded, so the test fails.
- **Checks:** L0, D1, D2e, D2s, D3e `game_telemetry`, D3s `telemetry`.
- **Docs:** none (LT-26).
- **Commit:** `LT-05: forward the launcher-telemetry target table and add capture helpers`.

### LT-06: startup event, panic hook, crash marker

- **Host:** windows. **Worktree:** `lt-06`, from `lt-app-integration` after LT-05. **Rules:** R1-R9. **Contract:** C4, C5, C6, C9.
- **Files:**
  - new `shell/src/startup_event.rs` (≤200 lines) and new `shell/src/panic_hook.rs` (≤200 lines), both with inline tests;
  - `shell/src/main.rs`: module lines, the `panic_hook::install(...)` call right after the subscriber is set up (grep `telemetry::` in `main`), and replace the `eprintln!` at ~:536 with a `tracing::warn!` on `launcher.startup` that has a `reason`;
  - `shell/src/host.rs`: the `emit_startup` call in `host::start` after state is loaded, and the `let _ =` at ~:160 (`reconcile_launcher_update`) and ~:180 (`transport::recover_now`). Each becomes a warn on `launcher.update`, with `event=update_reconcile_failed` / `update_recovery_failed` and a `reason`.
- **Change:**
  - `StartupFacts`: `{ version: &'static str (CARGO_PKG_VERSION), os, arch (std::env::consts), release_build: bool (!cfg!(debug_assertions)), dll_bundled: bool, game_diagnostics: bool, setup_diagnostics: bool, login_servers: u64, log_file_ok: bool, previous_run_panicked: bool }`.
    - `dll_bundled` comes from the same `option_env!("CIMMERIA_CLIENT_TELEMETRY_SHA256")` that `host/launch/resources.rs:18` reads; add a tiny `pub(crate) fn dll_bundled() -> bool` there.
    - The consent values and the login-server count come from the state `host::start` already loads.
  - `emit_startup(&StartupFacts)` emits an info on `launcher.startup`, `event=launcher_started`, with those fields.
  - The panic hook:
    - `install(state_dir: PathBuf)` chains `std::panic::take_hook()`.
    - The hook calls `record_panic(location_basename: Option<(String, u32)>, thread: Option<&str>, state_dir)`, which emits an error on `launcher.crash` with `event=panic`, `location="<file basename>:<line>"` and `thread`. It writes the message only to `launcher.pipeline`, never to a forwarded target.
    - It writes `<state_dir>/last-panic.json` as `{ "ts_ms": u64, "location": String }`.
  - At startup, `take_previous_panic(state_dir) -> Option<String>` reads and deletes the marker. If one exists, emit a warn on `launcher.crash` with `event=previous_run_panicked` and `location`, and set `previous_run_panicked`.
  - The state dir is the `<app_data>/state` directory the game-telemetry files use.
- **Tests:**
  - `emit_startup` under capture: pin the fields, and assert that no field value contains `\` or `/`.
  - `record_panic` writes the marker and emits the error.
  - `take_previous_panic` round-trips and deletes the marker.
  - Never set the global hook in a test.
- **Checks:** D1, D2s, D3s `startup_event`, D3s `panic_hook`.
- **Docs:** none (LT-26).
- **Commit:** `LT-06: launcher startup event, panic hook and crash marker`.

### LT-07: pipeline health counters and the recovery row

- **Host:** windows. **Worktree:** `lt-07`, from `lt-app-integration` after LT-05. **Rules:** R1-R9. **Contract:** C4, C7, C8, C10.
- **Files:**
  - new `engine/src/storage/game_telemetry/health.rs` (≤220 lines, with inline tests), declared in `mod.rs`;
  - `engine/src/storage/game_telemetry/launcher_session.rs` (~203 lines);
  - `shell/src/telemetry.rs`: the forwarder loop in `spawn_forwarder` and the `health()` global.
- **Change:**
  1. Implement C7.
  2. `SendOutcome::MintFailed { reason }` (C8), with reasons set at each mint failure point in `launcher_session.rs`.
  3. After a successful mint, post to `endpoint_sanity::effective_upload_endpoint(login_server, &upload_endpoint)` (C10), not to the raw advertised URL. When `overridden` is true, emit once per launcher session (the 15-minute token) an info on `launcher.telemetry`: `event=upload_endpoint_overridden`, `kind="launcher"`, message "launcher telemetry: the server advertised a loopback upload address; expected its public address; using the login server's /api/telemetry instead". If `effective_upload_endpoint` returns `None`, return `MintFailed { reason: "decode" }`.
  4. Any existing log in `launcher_session.rs` moves to target `launcher.pipeline` (C4).
  5. In the forwarder, take `let outcome = sender.send(...)` and call `health().lock().record(&outcome, now_ms)`. On `Some(r)`, emit an info on `launcher.telemetry`: `event=pipeline_recovered`, `mint_failed`, `held`, `dropped`, `outage_ms`, `last_reason`, with the message "launcher telemetry: sends resumed; expected every batch delivered, some were lost while the server was unreachable".
  6. Log nothing else to a forwarded target from the forwarder.
- **Tests:**
  - `record` state machine: Empty is a no-op; Sent alone gives no recovery; MintFailed, then Held, then Sent gives a recovery with counts 1/1/0 and a reset; totals keep counting.
  - The `launcher_session` override case: the mint fake returns `http://localhost:8443/api/telemetry` while the login server is the fake's non-loopback-looking base (if the fake only binds 127.0.0.1, test the pure helper the sender calls instead and say so); the post goes to the derived URL and one `upload_endpoint_overridden` is emitted. Reverted, the post goes to the loopback URL.
  - A shell test that two failures then a success emit exactly one `pipeline_recovered`.
- **Checks:** D1, D2e, D2s, D3e `game_telemetry`, D3s `telemetry`.
- **Docs:** none (LT-26).
- **Commit:** `LT-07: track launcher telemetry health and report recovery`.

### LT-08: game-session mint causes and `endpoint_misconfigured`

- **Host:** windows. **Worktree:** `lt-08`, from `lt-app-integration` after LT-05. **Rules:** R1-R9 (R8 applies). **Contract:** C5, C8, C10.
- **Files:**
  - `engine/src/storage/game_telemetry/session.rs` (~207 lines; `start()` at :78-145);
  - new `engine/src/storage/game_telemetry/session_events.rs` (≤120 lines);
  - `game_telemetry/mod.rs`: the `Outcome` variant and its status string;
  - `shell/src/host/game_telemetry.rs`: Outcome to DTO;
  - the frontend `src/game-telemetry-workflow.ts`, `src/game-telemetry-view.ts`, their schema, and their `*.test.ts`.
- **Change:**
  1. At each failure return in `start()` (:87, :101, :107, :114, :116, :122, :125, :130, :132, :136), call `session_events::mint_failed(reason, http_status: Option<u16>, latency_ms)`. That is a warn on `launcher.telemetry`, with `event=game_session_mint_failed`, using the C8 reason codes. The message: "game telemetry: expected a dev-session token from the login server, got <reason>; this Play session sends no diagnostics".
  2. After a successful mint, the game session uploads to `endpoint_sanity::effective_upload_endpoint(first_login_server, &upload_endpoint)` (C10). When `overridden`, call `session_events::endpoint_overridden()` (info on `launcher.telemetry`, `event=upload_endpoint_overridden`, `kind="game"`). This is what the injected DLL and the transport are handed; grep `upload_endpoint` in `session.rs` and `transport/` and replace the value at its single source.
  3. No new `Outcome` and no new UI text (C8 amended). The existing statuses stay.
- **Tests:**
  - `session_events` under capture.
  - If the engine tests have a dev-session fake (grep `dev-session`), add one case that returns `http://localhost:8443/api/telemetry` and asserts the session's upload URL is the login server's `/api/telemetry` and one `upload_endpoint_overridden` was emitted. Otherwise test the single source function directly.
  - No frontend change; drop F1 from the checks unless a TS file changed.
- **Checks:** D1, D2e, D2s, D3e `game_telemetry`, D3s `game_telemetry`, F1.
- **Docs:** none (LT-26).
- **Commit:** `LT-08: log game-session mint failures and replace a loopback upload endpoint`.

### LT-09a: game-telemetry transport events (re-specified, amendment 5)

- **Host:** windows. **Worktree:** `lt-09` (already exists: branch `launcher-telemetry/lt-09-transport-events`, base `53acbc2dd`, no edits yet). **Rules:** R1-R9 (R8 does not apply). **Contract:** C5, C6 (unchanged).
- **Why:** F8 and the owner directive "wire all launcher telemetry, nothing silent". The transport logs free strings with `?code` on the module target, which is never forwarded.
- **Paths.** Every path below is under `crates/launcher/desktop/engine/src/storage/game_telemetry/`. Line numbers are at `53acbc2dd`; re-grep before editing.
- **Files (only these):**
  - new `transport/events.rs` (≤220 lines, no tests inside);
  - new `transport/events_tests.rs` (≤260 lines);
  - `transport/mod.rs` (471 lines, over 450, so call lines only, plus the lines listed in step 3);
  - `transport/queue.rs` (3 call lines);
  - `transport/tests.rs` (already over the hard cap: only the 6 one-line edits in step 6, nothing else).
- **Do not touch:** `minted()`, `client()`, or `remint()` apart from its line :441 (LT-08 may edit near `minted()`); `test_capture.rs` (LT-40 edits it); `session_loop/` (LT-09b); C6.
- **Field rules for this packet.** Target `"launcher.telemetry"`, written as a string literal in every macro (`tracing::warn!(target: "launcher.telemetry", ...)`). Fields only from C6: `event`, `reason`, `action`, `http_status`, `attempt`, `count`, plus the boolean `held`. Never `?code`, `%e`, a URL, a body or a token. `http_status` is `Option<u16>`: tracing records an `Option` only when it is `Some`, so the key is absent for `None`. If the compiler rejects an `Option` field value, write two macro calls in the helper (`if let Some(s) = status { ...http_status = s... } else { ... }`) and say so in the report.

**1. `transport/events.rs`.** Module doc: `//! Game-telemetry transport events (launcher-telemetry LT-09a; C5, C6).` Contents, in this order:

```rust
use super::TransportError;
use cimmeria_launcher_core::telemetry::auth::AuthError;
use cimmeria_launcher_core::telemetry::bundle::BundleError;
use cimmeria_launcher_core::telemetry::chunk::ChunkError;
use cimmeria_launcher_core::telemetry::TelemetryError;
use std::sync::atomic::{AtomicU64, Ordering};

/// The closed code for a transport error (C6 `reason`). No wildcard arm.
pub(crate) fn reason(code: TransportError) -> &'static str {
    match code {
        TransportError::NotOptedIn => "not_opted_in",
        TransportError::Choice => "choice_unreadable",
        TransportError::SessionUnavailable => "session_unavailable",
        TransportError::EndpointRefused => "endpoint_refused",
        TransportError::Queue => "queue",
        TransportError::TokenRejected => "token_rejected",
        TransportError::BackingOff => "backing_off",
        TransportError::Upload => "upload",
    }
}

/// The HTTP status behind a raw core error, if the server answered.
pub(crate) fn http_status(error: &TelemetryError) -> Option<u16> {
    match error {
        TelemetryError::Auth(AuthError::Status { status, .. })
        | TelemetryError::Chunk(ChunkError::Status { status, .. })
        | TelemetryError::Chunk(ChunkError::Rejected { status, .. })
        | TelemetryError::Bundle(BundleError::Status { status, .. }) => Some(*status),
        TelemetryError::Chunk(ChunkError::TokenRejected)
        | TelemetryError::Bundle(BundleError::TokenRejected) => Some(401),
        TelemetryError::Chunk(ChunkError::RateLimited { .. }) => Some(429),
        TelemetryError::Auth(AuthError::KillSwitch { .. })
        | TelemetryError::Chunk(ChunkError::KillSwitch { .. })
        | TelemetryError::Bundle(BundleError::KillSwitch { .. }) => Some(503),
        _ => None,
    }
}

/// Consecutive failures of one kind. The first is forwarded at warn; repeats
/// stay local at debug, so an outage writes one forwarded row, not one per flush.
#[derive(Debug, Default)]
pub(super) struct Streak(AtomicU64);

impl Streak {
    /// Count one more failure; returns the new count (1 for the first).
    pub(super) fn fail(&self) -> u64 { self.0.fetch_add(1, Ordering::SeqCst) + 1 }
    /// Clear the count; returns what it was.
    pub(super) fn reset(&self) -> u64 { self.0.swap(0, Ordering::SeqCst) }
}

#[derive(Debug, Default)]
pub(super) struct Streaks {
    pub(super) upload: Streak,
    pub(super) refresh: Streak,
}
```

Then these `pub(super) fn` helpers. Each is one `tracing::` call (two for the streak ones: warn when the count is 1, else the same fields at debug). Messages are exact:

| Helper | Level | `event` | Fields | Message |
|---|---|---|---|---|
| `chunk_upload_failed(streak: &Streak, error: &TelemetryError)` | warn when `streak.fail() == 1`, else debug | `chunk_upload_failed` | `reason = reason(TransportError::of(error))`, `http_status = http_status(error)`, `attempt` = the count | "game telemetry: telemetry flush failed; expected the server to take the queued chunks; they stay queued and the next flush retries" |
| `chunk_upload_resumed(streak: &Streak)` | info, only when `streak.reset() > 0`; otherwise no event | `chunk_upload_resumed` | `count` = the reset count | "game telemetry: chunk uploads resumed after failed flushes; the events they held are being sent" |
| `token_renew_started()` | info | `token_renew_started` | `reason = "token_rejected"` | "game telemetry: the upload endpoint refused the token; renewing it" |
| `token_refresh_failed(action: &'static str, code: TransportError, status: Option<u16>)` | warn | `token_refresh_failed` | `action`, `reason = reason(code)`, `http_status = status`, `held = true` | "game telemetry: telemetry token not renewed; expected a working token from the login server; flushes hold off and retry later" |
| `proactive_refresh_failed(streak: &Streak, code: TransportError, status: Option<u16>)` | warn when `streak.fail() == 1`, else debug | `token_refresh_failed` | `action = "refresh_proactive"`, `reason = reason(code)`, `http_status = status`, `held = false`, `attempt` | "game telemetry: proactive token refresh failed; expected a fresh token before this one expires; the current token stays in use" |
| `session_reminted()` | info | `session_reminted` | `reason = "token_rejected"` | "game telemetry: minted a new telemetry session after the server refused the old token; uploads continue on it" |
| `bundle_upload_failed(error: &TelemetryError)` | warn | `bundle_upload_failed` | `reason = reason(TransportError::of(error))`, `http_status = http_status(error)` | "game telemetry: session log bundle not sent; expected the server to take it; this session's logs are not uploaded" |
| `queue_dropped(count: u64)` | warn | `queue_dropped` | `count` | "game telemetry: the disk queue dropped lines since the last bundle (queue cap or chunks the server refused); those events were never sent" |
| `dropped_counter_reset_failed()` | warn | `dropped_counter_reset_failed` | `reason = "queue"` | "game telemetry: dropped-lines counter not reset after the bundle; expected it cleared; the next bundle reports these lines again" |
| `queue_discarded()` | info | `queue_discarded` | none beyond `event` | "game telemetry: queue deleted unsent after the game-telemetry choice changed" |
| `queue_discard_failed(action: &'static str)` | warn | `queue_discard_failed` | `action`, `reason = "queue"` | "game telemetry: queue not deleted; expected it gone after the choice changed; it is retried at the next change or startup" |

`TransportError::of` is private to `transport/mod.rs`; `events` is a child module, so `TransportError::of(error)` compiles there. Do not make `of` public.

**2. `transport/mod.rs` declarations** (next to `mod queue;`): `pub(super) mod events;` and `#[cfg(test)] mod events_tests;`. `pub(super)` lets LT-09b call `events::reason` from `session_loop`.

**3. `transport/mod.rs` changes.** Add the field `streaks: events::Streaks,` to `struct Transport` (after `auth_hold`) and `streaks: events::Streaks::default(),` in `Transport::open`. Then:

| Site (at `53acbc2dd`) | Replace with |
|---|---|
| :272 `tracing::info!("telemetry token refused by the upload endpoint; renewing")` | `events::token_renew_started();` |
| :292-294 `if sent > 0 { self.release_auth_hold(); }` | inside the same `if`, after `release_auth_hold()`: `events::chunk_upload_resumed(&self.streaks.upload);` (only `sent > 0` resets, as for the auth hold; an `Ok(0)` proves nothing) |
| :297-300 `Err(e) => { let code = ...; tracing::warn!(?code, ...); Err(code) }` | `Err(e) => { events::chunk_upload_failed(&self.streaks.upload, &e); Err(TransportError::of(&e)) }` |
| :341-346, the `Ok(outcome)` arm of `upload_bundle` | after `self.release_auth_hold();` add `if dropped > 0 { events::queue_dropped(dropped); }`; replace the :344 warn with `events::dropped_counter_reset_failed();` |
| :349 `Err(e) => Err(TransportError::of(&e)),` (no log today) | `Err(e) => { events::bundle_upload_failed(&e); Err(TransportError::of(&e)) }` |
| :366-390 `refresh_if_due` | the block below |
| :397-409 `renew` | the block below |
| :413 the warn in `hold_after_refused_renewal` | `events::token_refresh_failed("upload_after_renew", TransportError::TokenRejected, Some(401));` |
| :441 `tracing::info!("telemetry session re-minted ...")` | `events::session_reminted();` |

`queue_dropped` is emitted only here, at the bundle, where `dropped` is already read (once per session). A bundle that fails or is empty leaves the counter, and the next session's bundle reports it.

```rust
    async fn refresh_if_due(&self) -> Result<(), TransportError> {
        let (code, status) = match self.telemetry.refresh_if_due(&self.auth).await {
            Ok(_) => {
                self.streaks.refresh.reset();
                return Ok(());
            }
            Err(e) => (TransportError::of(&e), events::http_status(&e)),
        };
        match code {
            TransportError::TokenRejected => match self.remint().await {
                Ok(()) => Ok(()),
                Err(code) => {
                    events::token_refresh_failed("remint", code, None);
                    self.hold_auth();
                    Err(code)
                }
            },
            TransportError::BackingOff => {
                events::token_refresh_failed("refresh", code, status);
                self.hold_auth();
                Err(code)
            }
            code => {
                events::proactive_refresh_failed(&self.streaks.refresh, code, status);
                Ok(())
            }
        }
    }

    async fn renew(&self) -> Result<(), TransportError> {
        let mut status = None;
        let result = match self.telemetry.refresh_now(&self.auth).await {
            Ok(()) => Ok(()),
            Err(e) => match TransportError::of(&e) {
                TransportError::TokenRejected => self.remint().await,
                code => {
                    status = events::http_status(&e);
                    Err(code)
                }
            },
        };
        result.inspect_err(|code| {
            events::token_refresh_failed("renew", *code, status);
            self.hold_auth();
        })
    }
```

Keep each function's existing doc comment. `remint` failures inside `renew` give `token_refresh_failed` with `action = "renew"`; LT-08 logs the mint's own cause (`game_session_mint_failed`), so no `http_status` is needed here.

**4. `transport/queue.rs`.** :93 becomes `super::events::queue_discard_failed("withdrawn_write");`, :163 `super::events::queue_discarded();`, :168 `super::events::queue_discard_failed("choice_changed");`.

**5. Retries and `attempt` (decided).** Core has no per-attempt chunk retry: `launcher-core/src/telemetry/flush.rs` posts each chunk once and stops at the first error, and `chunk.rs` has no loop. So `attempt` is the transport's own count of consecutive failed flushes (`Streak`), and no core file changes. There is no `chunk_upload_gave_up`: events are never given up, they stay queued (LT-09b logs `final_flush_failed` at game exit).

**6. `transport/tests.rs` (one-line edits only).** The existing `transport_errors_and_logs_carry_no_server_text_or_token` test's `Capture::enabled` (:793) keeps only `cimmeria*` targets, so it would see nothing after this packet. Change that line to `metadata.target().starts_with("cimmeria") || metadata.target().starts_with("launcher.")`. Keep the messages above: that test still looks for the substrings "telemetry flush failed" and "not renewed". Then make five helpers visible to `events_tests.rs` by adding `pub(super)` in front of: `const CHUNK` (:9), `async fn mount_mint` (:29), `fn opted_in` (:38), `async fn open` (:45), `fn event` (:53).

**7. Tests, in `transport/events_tests.rs`** (unit tests, TESTING.md):
- Header: `use super::events::*; use super::tests::{event, mount_mint, open, opted_in, CHUNK}; use super::*; use crate::test_capture::capture;` plus the wiremock imports `tests.rs` uses.
- **Callsite interest.** In every test, before calling `capture`, hold `let _second = tracing::Dispatch::new(tracing_subscriber::registry());`, with a comment pointing to `transport/tests.rs:711-716`. Other tests hit these callsites with no subscriber, and with one live dispatcher tracing would cache them as disabled.
- **Async under capture.** `capture` is thread-local, so these are plain `#[test]` functions (never `#[tokio::test]`) that run `tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async { ... })` inside the `capture` closure.
- Tests:
  1. `reason` maps all eight variants to the eight strings in step 1.
  2. `http_status` table: Chunk `Status{500}` gives 500, Chunk `Rejected{413}` 413, Chunk `TokenRejected` 401, Chunk `RateLimited` 429, Chunk `KillSwitch` 503, Auth `Status{500}` 500, Auth `KillSwitch` 503, Auth `ReplyTooLarge` `None`, Bundle `Status{502}` 502, Bundle `TokenRejected` 401, Bundle `KillSwitch` 503, Bundle `Empty` `None`. Use `body: "MARKER".into()` in the `Status` and `Rejected` cases.
  3. Streak: two `chunk_upload_failed` calls give a WARN with `attempt=1` and then a DEBUG with `attempt=2`; then `chunk_upload_resumed` gives one INFO with `count=2`; a second `chunk_upload_resumed` gives no event.
  4. Each other helper once under capture: pin target `launcher.telemetry`, level, `event`, and `reason`/`action`/`held` where it has them. `token_refresh_failed(.., None)` has no `http_status` key.
  5. **Real path (the revert proof).** On a `MockServer`: `mount_mint`, then the chunk route answers 500 (body "MARKER") `up_to_n_times(2)`, then 200 (copy the success mock a flush test in `tests.rs` uses). Enqueue one event, flush three times. Expect exactly one WARN `chunk_upload_failed` (`http_status=500`, `reason=upload`, `attempt=1`), one DEBUG with `attempt=2`, and one INFO `chunk_upload_resumed` (`count=2`). No captured field value contains `MARKER`. Reverted, there is no `launcher.telemetry` event and the test fails.
  6. **Real path, bundle.** Copy a bundle test in `tests.rs` that writes a session log and posts a bundle (grep `upload_bundle`); make the bundle route answer 500. Expect one WARN `bundle_upload_failed` with `http_status=500`.
- No IP literal in any test value (LT-40 makes capture panic on one).
- **Checks:** D1, D2e, D3e `game_telemetry`.
- **Report:** the line counts of every touched file, and whether the `Option` field values compiled.
- **Docs:** none (LT-26).
- **Commit:** `LT-09a: structured game-telemetry transport events`.

### LT-09b: game-telemetry session-loop and queue-write events (amendment 5)

- **Host:** windows. **Worktree:** `lt-09b`, branch `launcher-telemetry/lt-09b-session-events`, from the `lt-app-integration` tip **after LT-09a has merged into it** (it calls `transport::events::reason`). **Rules:** R1-R9 (R8 does not apply). **Contract:** C5, C6 (unchanged).
- **Why:** the same directive. `session_loop/mod.rs` has 7 free-string sites (README F8), and `install_event.rs` and `patch_events.rs` log queue-write failures on the module target. No other packet owns them.
- **Paths.** Under `crates/launcher/desktop/engine/src/storage/game_telemetry/`. Lines at `53acbc2dd`; re-grep.
- **Files (only these):**
  - new `session_loop/events.rs` (≤260 lines, with an inline `#[cfg(test)] mod tests`);
  - `session_loop/mod.rs` (360 lines): `pub(super) mod events;` next to the `use` lines, and the sites below;
  - `session_loop/tests.rs`: only add `not_queued: 0,` to the `SessionEnd { .. }` literal at :189;
  - `install_event.rs` (3 sites, plus a `Run::as_str`);
  - `patch_events.rs` (1 site).
- **Do not touch:** `transport/`, `session.rs`, `session_events.rs`, `game_telemetry/mod.rs` (LT-07 and LT-08 edit it), `test_capture.rs`. If `SessionEnd { .. }` is built as a literal anywhere else (grep `crates/launcher/desktop`), return `blocked`.
- **Field rules.** As LT-09a: target literal `"launcher.telemetry"`; C6 fields only: `event`, `reason`, `action`, `outcome`, `count`, `duration_ms`, `exit_code: i64`, plus the booleans `bundle_sent` and `exit_observed`. No pid, no path, no `?code`. `bound_s` is gone: the settle bound is logged as `duration_ms` (C6 is unchanged).

**1. `session_loop/events.rs` helpers** (`pub(crate) fn`; reasons via `crate::storage::game_telemetry::transport::events::reason`, or `super::super::transport::events::reason`, whichever path compiles):

| Helper | Level | `event` | Fields | Message |
|---|---|---|---|---|
| `previous_session_stopped(bound: Duration)` | warn | `previous_session_stopped` | `duration_ms = u64::try_from(bound.as_millis()).unwrap_or(u64::MAX)` | "game telemetry: the previous session was still sending past its bound; stopped it; what it had queued goes with this session" |
| `bundle_not_sent(code: TransportError)` | debug (local only; LT-09a's `bundle_upload_failed` and `token_refresh_failed` are the forwarded rows) | `bundle_not_sent` | `reason` | "game telemetry: session log bundle not sent" |
| `final_flush_failed(reason: &'static str)` | warn | `final_flush_failed` | `reason` | "game telemetry: the final flush at game exit did not finish; expected the whole queue sent; the rest goes with the next session" |
| `game_session_finished(end: &SessionEnd, duration_ms: u64)` | info | `game_session_finished` | `count = end.events`, `bundle_sent`, `exit_observed = end.exit.is_some()`, `exit_code = end.exit.map(\|e\| i64::from(e.code))` (absent when `None`), `duration_ms` | "game telemetry: session finished" |
| `session_events_lost(count: u64)` | warn | `session_events_lost` | `count` | "game telemetry: events could not be queued this session; expected every event on disk; those events were lost" |
| `game_session_withdrawn(count: u64)` | info | `game_session_withdrawn` | `count` | "game telemetry: turned off during the session; stopped sending and left no queue" |
| `client_patches_read_failed()` | warn | `client_patches_read_failed` | `reason = "task_failed"` | "game telemetry: client-patches log read failed; expected its events; they are not sent" |
| `event_queue_failed(code: TransportError, n: u64)` | warn when `n == 1`, else debug | `event_queue_failed` | `reason`, `count = n` | "game telemetry: event not queued; expected it written to the disk queue; it is lost" |
| `install_result_queued(action: &'static str, outcome: &'static str, count: u64)` | info | `install_result_queued` | `action`, `outcome`, `count` | "game telemetry: install result queued for the next telemetry session" |
| `install_result_not_queued(action: &'static str, reason: &'static str)` | warn | `install_result_not_queued` | `action`, `reason` | "game telemetry: install result not queued; expected it on disk for the next session; it is not sent" |
| `client_patch_events_not_queued(count: u64)` | warn | `client_patch_events_not_queued` | `count` | "game telemetry: client-patches events not queued; expected them on disk for the next session; they are not sent" |

**2. `session_loop/mod.rs` changes.**
- `SessionEnd` gains `pub not_queued: u64,` with the doc `/// Events that could not be queued (lost).`
- `run()`: first line `let started = std::time::Instant::now();`.
- :179-182 (`bound_s` warn) becomes `events::previous_session_stopped(bound);`.
- :266 becomes `Err(code) => events::bundle_not_sent(code),`.
- :268-271 become:
  ```rust
  match tokio::time::timeout(SHUTDOWN_BOUND, transport.shutdown()).await {
      Ok(Ok(_)) => {}
      Ok(Err(code)) => events::final_flush_failed(transport_reason(code)),
      Err(_) => events::final_flush_failed("timeout"),
  }
  ```
  (`transport_reason` is the `reason` import, renamed with `as` so it does not clash.)
- :272-278 become `if end.not_queued > 0 { events::session_events_lost(end.not_queued); }` then `events::game_session_finished(&end, u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX));`.
- :284-287 (in `withdrawn`) become `events::game_session_withdrawn(end.events);`.
- :337 becomes `events::client_patches_read_failed();`.
- :358-361 become `Err(code) => { end.not_queued += 1; events::event_queue_failed(code, end.not_queued); Ok(()) }`.
- Leave :228 (`transport.flush().await == Err(TransportError::NotOptedIn)`) unchanged. Update the doc comment on `SessionEnd` only if it lists fields.

**3. `install_event.rs`.** Add to `Run`: `fn as_str(self) -> &'static str` giving `"install"`, `"repair"`, `"update"`. :64-67 becomes `super::session_loop::events::install_result_not_queued(run.as_str(), "choice_unreadable");`, :75-80 becomes `super::session_loop::events::install_result_queued(run.as_str(), report.result.as_str(), report.patches.len() as u64);`, and :84-88 becomes `super::session_loop::events::install_result_not_queued(run.as_str(), "queue_write_failed");`.

**4. `patch_events.rs`.** Count instead of logging per event: before the loop `let mut failed = 0u64;`; the `Err(_)` arm (:107-110) becomes `Err(_) => failed += 1,`; after the loop `if failed > 0 { super::session_loop::events::client_patch_events_not_queued(failed); }`.

**5. Tests** (inline in `session_loop/events.rs`; unit; the same `_second` dispatcher and current-thread `block_on` inside `capture` as LT-09a):
- Each helper under capture: pin target, level, `event`, `reason`/`action`/`count`. `game_session_finished` with an exit has `exit_code` and `exit_observed=true`; without one it has no `exit_code` key and `exit_observed=false`. `event_queue_failed(.., 1)` is WARN and `(.., 2)` is DEBUG.
- **Real path (the revert proof).** Inside `block_on`: `let pending = tokio::spawn(std::future::pending::<()>());`, insert `pending.abort_handle()` into `super::previous_sessions()` under a `tempfile::tempdir()` path, then `super::settle_previous(dir.path(), Duration::from_millis(20)).await`. Expect exactly one WARN `previous_session_stopped` with `duration_ms=20` on `launcher.telemetry`, and no `bound_s` key. Reverted, the event is on the module target with `bound_s`, and the test fails.
- **Checks:** D1, D2e, D3e `game_telemetry`.
- **Report:** line counts, and any other `SessionEnd` literal you found.
- **Docs:** none (LT-26).
- **Commit:** `LT-09b: structured game-telemetry session-loop and queue-write events`.

### LT-10: `ui_event` bridge

- **Host:** windows. **Worktree:** `lt-10`, from `lt-app-integration` after LT-05. **Rules:** R1-R9 (R8 applies). **Contract:** C13 (`ui_event`), D-LT10.
- **Files:**
  - new `shell/src/host/ui_event.rs` (≤200 lines, with inline tests), declared in `host.rs` or the host mod file (one line);
  - `shell/src/main.rs`: add `ui_event` to the `generate_handler!` list (~:576-596);
  - new `frontend/src/ui-event.ts` and `frontend/src/ui-event.test.ts`.
- **Change:**
  - Validate the input: `area` is in `UI_AREAS`; `event` and `code` match `^[a-z][a-z0-9_]{0,39}$`, checked by hand with no regex crate; `level` is `info` or `warn`. Invalid input returns `Err("invalid ui event")` plus a `debug`.
  - Throttle: at most 60 per rolling 60 s, using an `AtomicU64` window start and a count. Excess input is counted and reported as `suppressed` on the next accepted event.
  - Emit `info` or `warn` on `launcher.ui` with `event`, `area`, `code` and `suppressed`, and the message "launcher UI: <area> reported <code>".
  - TS: `export type UiArea` is the 16 literals, and `export function reportUiEvent(area: UiArea, event: string, code: string, level: 'info' | 'warn' = 'warn'): void`. It invokes `ui_event` with the same invoke import `log-upload-view.ts` uses, and ignores rejection with a commented `.catch(() => undefined)`. This is the only allowed empty catch.
- **Tests:**
  - Rust: valid input gives one event; a bad area, event or level gives `Err` and no event; the 61st call in a window is suppressed, and the next window carries `suppressed=1`.
  - TS: `reportUiEvent` calls invoke with the arguments, using the existing invoke mock pattern from a view test.
- **Checks:** D1, D2s, D3s `ui_event`, F1.
- **Docs:** none (LT-26).
- **Commit:** `LT-10: ui_event bridge from the frontend to launcher telemetry`.

### LT-11: `telemetry_status`, the status line, and stale copy

- **Host:** windows. **Worktree:** `lt-11`, from `lt-app-integration` after LT-07, LT-08 and LT-10. **Rules:** R1-R9 (R8 applies). **Contract:** C7, C13 (`telemetry_status`).
- **Files:**
  - new `shell/src/host/telemetry_status.rs` (≤150 lines), declared in the host mod (one line);
  - `shell/src/main.rs`: the handler list;
  - new `frontend/src/telemetry-status-view.ts` and its test;
  - `frontend/ui/index.html`: a `#telemetry-status` line under `#game-telemetry`, and the `:45` copy;
  - the mount call in `frontend/src/view.ts` or `composition.ts`, whichever mounts `game-telemetry-view`.
- **Change:**
  - `telemetry_status()` snapshots `telemetry::health()`, computing the age from `last_success_ms`.
  - The view refreshes on mount, every 15 s, and right after the `#game-telemetry` checkbox changes, so a toggle gets feedback on the first press. It shows:
    - "Diagnostics are off." when opted out;
    - "Diagnostics on. Nothing sent yet this run.";
    - "Diagnostics sent <n> min ago.";
    - "Diagnostics not sent: <text>. The launcher will retry." The reason texts are: `transport`/`timeout` "the server could not be reached"; `http_status`/`endpoint_refused` "the server refused the request"; `decode` "the server's answer was unreadable"; `dropped` "the server's limit was reached".
  - **Do not change `index.html:45`** (amended, D-LT13): the `#adoption-telemetry` row is shown only when the old launcher was opted in **and** this build cannot send (`requires_telemetry_acceptance`, engine `adoption/mod.rs:127-131`), so its text is true whenever it is visible. Leave `:13` and `view.ts:133` to LT-23.
- **Tests:**
  - Rust: the DTO from a seeded health.
  - TS: each text branch, and a checkbox change triggers a refresh.
- **Checks:** D1, D2s, D3s `telemetry_status`, F1.
- **Docs:** none (LT-26).
- **Commit:** `LT-11: show launcher telemetry status in the UI`.
