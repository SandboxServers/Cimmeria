# launcher-cache-presync work packets

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26 (re-specced for the owner's D-LP4 reversal: a failed presync blocks Play)
> **Companions:** [Ledger](README.md), [resume card](RESUME.md)

Nineteen packets in five waves, plus the LP-06 lab proof. Wave 1 and the lab proof are here; wave 2 is in [packets-wave2.md](packets-wave2.md) and waves 3 to 5 in [packets-wave3.md](packets-wave3.md). The design, findings and decisions (D-LP1..D-LP29) are in the [ledger](README.md). Line numbers are from `main` `2019922fe`; re-grep every anchor before editing.

## Dispatch rules

- **Before any wave:** check that nobody else is running this campaign (campaign-packet § 0). Before waves 2 and 3, ask `ops` whether the client-tls-oidc campaign PR (branch `client-tls-oidc/campaign`) has merged. It edits `worker.rs`, `login_port.rs` and `crates/server/src/main.rs` (F16). If it has merged, branch from the new `main`. If it hasn't, branch from `main` anyway and keep the hunks in those three files small; whichever PR lands second resolves the conflict.
- **Worktrees:** `pwsh tools/build-lane/mk-worktree.ps1 launcher-cache-presync/LP-NN-<slug> lcp-NN`, at `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lcp-NN`. Integration worktrees: `launcher-cache-presync/wave<W>` as `lcp-w<W>`, at `main`.
- **Wave 1** (one PR from `lcp-w1`). Dispatch LP-01 and LP-07 at once. LP-02, LP-03 and LP-04 form a chain: each starts when the previous one's packet review returns `ship`, and `ops` creates its worktree with the branch at that reviewed head SHA. Once LP-04 is merged into `lcp-w1`, `ops` creates `lcp-05` at the `lcp-w1` head, and LP-05 runs. Merge order into `lcp-w1` (`merge-noff.ps1`): **LP-01, LP-02, LP-03, LP-04, LP-07, LP-05**. Because LP-02..04 stack, each merge after LP-01 is a fast-forward of new commits. LP-07 touches no file the others touch. Then the wave loop, then one PR.
- **LP-06** (lab) runs after the wave 1 PR merges. The orchestrator asks the user before any lab use (the lab-uat skill) and waits until the lab is free. Its worktree `lcp-06` is at `main`. Its worknote `worknotes/LP-06.md` is committed in `lcp-06` and merged into `lcp-w3`.
- **Wave 2** (one PR from `lcp-w2`) opens when wave 1 has merged and LP-05's worknote says **compose works** (every `base_missing` and `base_differs` is 0, and no category is `source_newer`). Otherwise an architect ruling comes first (D-LP3). D-LP1 and D-LP2 are Decided. Dispatch LP-10 and LP-11 at once; LP-12 runs after both are merged into `lcp-w2`. Merge order: **LP-10, LP-11, LP-12**.
- **Wave 3** (into `lcp-w3`, no PR yet) opens when wave 1 has merged and LP-06 passed. Merge the LP-06 worknote branch into `lcp-w3` first. Dispatch LP-20, and at the same time wave 4's LP-24 (it works against the contract, at `main`). LP-21, LP-26, LP-22 and LP-27 chain as in wave 1, each at the previous one's reviewed head. Merge order into `lcp-w3`: **LP-06 worknote, LP-20, LP-21, LP-26, LP-22, LP-27**. Then the wave 3 loop on `lcp-w3`.
- **Wave 4** (into the same `lcp-w3`, then one PR for waves 3 and 4). LP-23 starts at the `lcp-w3` head once the wave 3 loop returns `ship`. LP-28 starts at LP-24's reviewed head. LP-25 runs after LP-27 and LP-24 are merged into `lcp-w3`. Merge order: **LP-23, LP-24, LP-28, LP-25**. Then the wave 4 loop, then one PR. Never ship wave 3 alone: from LP-27 on, a failed presync blocks Play, and only wave 4 tells the player why (D-LP18, D-LP29). Waves 2 and 3 can run at the same time, because they share no file.
- **Agents:** LP-05 goes to `rust-gameserver-dev`, LP-06 to `rust-gameserver-dev` with the lab-uat skill, and LP-12, LP-25 and LP-30 go to `documentation-writer`. Everything else goes to `packet-coder`. Each packet gets a packet review loop, and each wave a wave loop on its integration worktree. Wave 2 and the wave 3/4 PR also get one read-only `network-security-auth` pass on the integration head (new anonymous routes, a new download path, a new write into Documents).
- **Report path:** `<worktree>\.claude\reports\launcher-cache-presync\<packet>-<role>.md`. **Return line:** `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Commit attribution lines,** at the end of every commit message:

```text
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_0185QXpFqxKXAzEs5jzVq82J
```

PR bodies end with `🤖 Generated with [Claude Code](https://claude.com/claude-code)` and list the doc-update map rows touched.

## Rules every packet carries

- **R1 Shell.** `Host: windows`: PowerShell only, never the Bash tool. Start every command with `Set-Location <your worktree>;`, and use absolute paths for every file write.
- **R2 Lane.** Every compiling cargo call (`check`, `build`, `test`, `nextest`, `clippy`, `run`, `hakari`) goes through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground. Never call cargo directly, and never pass `--target-dir`. A lane failure: read its log under `logs/` before rerunning; at most 5 fix rounds per check.
- **R3 Scope.** Edit only the files your packet names. No `git stash`, no `git worktree prune`, no rebase, no push. Once every check passes, commit at once: `git add <explicit paths>`, then commit with the packet's subject and the attribution lines.
- **R4 File caps.** Soft cap 500 lines, hard cap 700 per file. Split along a natural seam (a sibling test file through `#[path]`) before passing 500. Report `blocked` rather than pass 700.
- **R5 Line endings.** `docs/**/*.md`, `crates/**/docs/*.md`, `crates/README.md` and every `README.md` are CRLF. After editing one, check `([IO.File]::ReadAllText('<abs path>') -match "[^\r]\n")` is `False`. Keep every other file's line endings.
- **R6 Tests.** Every behaviour change has a test that fails when the change is reverted; say in the report which test, and why. Rust tests are unit tests in the module's `#[cfg(test)] mod tests` (TESTING.md). Temp dirs come from `tempfile::tempdir()`. No test touches the real Documents folder, the network beyond `127.0.0.1`, or a live database.
- **R7 Secrets and paths.** No IPs (except `127.0.0.1`), credentials, account names or local user paths in code, tests, docs, worknotes or reports. Call the player's client copy `<client-copy>`.
- **R8 Generated blocks.** Never hand-edit a `<!-- gen:NAME -->` block, and don't commit a regenerated crate graph or tools catalog: `main` regenerates them.
- **R9 Data.** No CME bytes in the repo: tests build their PAKs in memory from made-up entries. No `db/scripts` migrations. Don't change `cooked_sync` behaviour (`crates/base-session/src/base/cooked_sync/`) except where LP-07 adds a counter.

### Checks

| ID | Command (prefix every one with `Set-Location <worktree>;`) |
|---|---|
| M1 `<c>` | `pwsh tools/build-lane/lane.ps1 cargo fmt -p <c>` |
| M2 `<c>` | `pwsh tools/build-lane/lane.ps1 cargo clippy -p <c> --all-targets -- -D warnings` |
| M3 `<c> [f]` | `pwsh tools/build-lane/lane.ps1 cargo nextest run -p <c> [f]` |
| H1 | `pwsh tools/build-lane/lane.ps1 cargo hakari generate --diff` and `pwsh tools/build-lane/lane.ps1 cargo hakari manage-deps --dry-run`; both must report no change. If the lane refuses `hakari`, say so in the report and leave it to CI. |
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e / D2s | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` (D2s: `-p cimmeria-launcher-desktop`) |
| D3e / D3s `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` (D3s: `-p cimmeria-launcher-desktop`). Use cargo test, not nextest: the desktop workspace's `harness = false` tests can't be listed. |
| F0 | Before D2s or D3s if the shell build asks for frontend assets: `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend; npm run build --prefix crates/launcher/desktop/frontend` |
| F1 | `npm test --prefix crates/launcher/desktop/frontend` |
| F2 | `npm run uat:launch-content-sync --prefix crates/launcher/desktop/frontend` |

## Contract

### Crate `cimmeria-cooked-pak` (`crates/cooked-pak`, LP-01 and LP-02)

```rust
// pak.rs (LP-01)
pub const METADATA_ENTRY: &str = "MetaData";
pub struct Pak { pub version: Option<u32>, pub entries: BTreeMap<u32, Vec<u8>> }
#[derive(Debug, thiserror::Error)] pub enum PakError { #[error("io: {0}")] Io(#[from] std::io::Error), #[error("zip: {0}")] Zip(#[from] zip::result::ZipError) }
pub fn entry_name(key: u32) -> String;                    // "_{key}"
pub fn parse_entry_name(name: &str) -> Option<u32>;       // "_<u32>" only; mirrors ResourceCache::load_pak
pub fn read_version(path: &Path) -> Result<Option<u32>, PakError>;  // MetaData's first 4 bytes LE; absent or short gives None
pub fn read_pak(path: &Path) -> Result<Pak, PakError>;    // other entry names are ignored
pub fn write_pak<W: Write + Seek>(out: W, version: u32, entries: &BTreeMap<u32, Vec<u8>>) -> Result<(), PakError>;
// digest.rs (LP-01)
pub fn category_digest<'a>(category_id: u32, version: u32, entries: impl IntoIterator<Item = (u32, &'a [u8])>) -> [u8; 32];
pub fn digest_hex(digest: &[u8; 32]) -> String;           // 64 lower-case hex
pub fn parse_digest_hex(text: &str) -> Option<[u8; 32]>;
```

`write_pak` writes the `_<key>` entries in ascending key order, deflate, each with `zip::DateTime::default()`, then `MetaData` last, stored, exactly 4 bytes LE. `category_digest` is SHA-256 over `b"CPD1" ‖ category_id LE ‖ version LE ‖ count u32 LE ‖ (key u32 LE ‖ len u32 LE ‖ bytes)*`, keys ascending (the caller passes them ascending; a `BTreeMap` iterator does).

```rust
// catalog.rs (LP-02)
#[derive(Serialize, Deserialize)] pub struct Index { pub schema: u32, pub categories: Vec<IndexEntry> }   // schema == 1
#[derive(Serialize, Deserialize, Clone)] pub struct IndexEntry { pub category_id: u32, pub pak: String, pub version: u32,
    pub entries: u32, pub changed: u32, pub body_bytes: u64, pub digest: String }
pub struct CategoryBody { pub category_id: u32, pub version: u32, pub keys: Vec<u32>, pub changed: BTreeMap<u32, Vec<u8>>, pub digest: [u8; 32] }
pub struct Limits { pub max_keys: u32, pub max_entry_bytes: u32, pub max_total_bytes: u64 }
impl Limits { pub const DEFAULT: Limits = Limits { max_keys: 1_000_000, max_entry_bytes: 4 << 20, max_total_bytes: 32 << 20 }; }
#[derive(Debug, PartialEq, thiserror::Error)] pub enum BodyError { BadMagic, Truncated, NotAscending, ChangedKeyNotListed, TooLarge, Trailing }
pub fn encode_body(body: &CategoryBody) -> Vec<u8>;
pub fn decode_body(bytes: &[u8], limits: &Limits) -> Result<CategoryBody, BodyError>;
// compose.rs (LP-02)
#[derive(Debug, PartialEq, thiserror::Error)] pub enum ComposeError { BaseMissing { missing: u32, first: u32 }, DigestMismatch }
pub fn compose(body: &CategoryBody, base: &BTreeMap<u32, Vec<u8>>) -> Result<BTreeMap<u32, Vec<u8>>, ComposeError>;
// decide.rs (LP-02)
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Action { UpToDate, Write, SourceNewer }
pub fn effective_version(cache: Option<u32>, source: Option<u32>) -> Option<u32>;
pub fn decide(cache: Option<u32>, source: Option<u32>, served: u32) -> Action;
```

- **Index JSON** (serde ignores unknown fields): `{"schema":1,"categories":[{"category_id":4,"pak":"CookedDataItems.pak","version":123,"entries":5000,"changed":12,"body_bytes":40960,"digest":"<64 hex>"}]}`.
- **Body `CPS1`**, all integers u32 LE: `"CPS1" | category_id | version | key_count | key_count keys, strictly ascending | changed_count | changed_count × (key | len | bytes), strictly ascending, each key also in the key list | digest (32 bytes)`, with nothing after it. `decode_body` checks every count and length against `Limits` before it allocates.
- **compose:** for each key in `body.keys`, take `changed[key]`, else `base[key]`. If some keys are in neither, `BaseMissing` names how many and the first. Then the digest over the result must equal `body.digest`, else `DigestMismatch`. Base keys not in `body.keys` are dropped.
- **decide:** `effective_version`: if `cache` is None, `source`; if `source > cache`, `source`; else `cache`. This mirrors the client's startup copy (F5). `decide`: if effective is `Some(served)`, `UpToDate`; if `source > Some(served)`, `SourceNewer`; else `Write`. The comparison is unsigned (D-LP15).

### Server (`cimmeria-resources`, `cimmeria-admin-api`; LP-03, LP-10, LP-11)

```rust
// crates/resources/src/base/served_catalog/mod.rs (LP-03)
pub struct ServedCategory { pub entry: IndexEntry, pub body: Arc<[u8]> }
pub struct ServedCatalog { /* categories: BTreeMap<u32, ServedCategory>, index_json: Arc<[u8]> */ }
impl ServedCatalog {
    pub fn build(cache: &ResourceCache, data_dir: &str) -> Result<Self, String>;
    pub fn index_json(&self) -> Arc<[u8]>;
    pub fn category(&self, category_id: u32) -> Option<&ServedCategory>;
    #[cfg(any(test, feature = "test-support"))] pub fn from_bodies(bodies: Vec<(String, CategoryBody)>) -> Self;
}
#[derive(Clone)] pub enum Published { Ready(Arc<ServedCatalog>), Failed(&'static str) }  // Failed carries a closed reason code
pub fn publish(state: Published) -> bool;                 // OnceLock; a second call logs WARN and returns false
pub fn current() -> Option<Published>;                    // None while the build is still running
```

`ServedCatalog::build` fails as a whole (`Err`) if any category the cache holds can't be served: its raw PAK is unreadable (`raw_pak_unreadable`) or its body passes `Limits::DEFAULT.max_total_bytes` (`category_too_large`). The error string names the category id and the code. A category the cache doesn't hold at all is left out of the index (the server can't resync it either). Why: with Play blocked on failure (D-LP18), a silently missing category would send the player to the resync the owner ruled out; a failed catalog instead gives every launcher a clear `catalog_failed`, and the operator has the kill switch (D-LP19).

Routes, nested at `/api/cooked` in `login_port_telemetry_router` (LP-10):

| Route | Success | Refusals (JSON `{"error":"<code>"}`) |
|---|---|---|
| `GET /api/cooked/v1/index` | 200, `application/json`, `Cache-Control: no-cache`, the index JSON | 503 `catalog_unavailable` + `Retry-After: 5` (still building); 503 `catalog_failed`, no `Retry-After` (the build failed); 503 `disabled`; 429 `rate_limited` + `Retry-After` |
| `GET /api/cooked/v1/category/{id}?version=<v>` | 200, `application/octet-stream`, the `CPS1` body | the same, plus 404 `unknown_category`; 409 `version_changed` with `"version":<served>`; 400 `bad_request` (missing or bad `version`, bad `id`) |

- Check order per request: quota, then the kill switch, then the catalog state, then the route's own checks.
- `CIMMERIA_COOKED_PRESYNC_DISABLED` (`1` or `true`) is read per request.
- `CIMMERIA_COOKED_PRESYNC_QUOTA_PER_IP` (default 60 a minute) has the launcher-summary quota's semantics, keyed by `dev_session::forwarded_for::rate_limit_ip(peer, &headers)` through `dev_session::quota`.

### Telemetry names (D-LP11, D-LP26)

| Where | Name | Level / kind | Fields or labels |
|---|---|---|---|
| server | `cooked_presync.catalog_ready` | INFO | categories, body_bytes, duration_ms |
| server | `cooked_presync.catalog_unavailable` | WARN | reason, category_id (when one category failed the build) |
| server | `cooked_presync.served` | INFO | category_id, category_name, version, bytes, changed (no address) |
| server | `cooked_presync.index_served` | DEBUG | categories |
| server | `cooked_presync.refused` | INFO | route, reason, status, category_id (0 on the index route) |
| server | `cooked_presync_requests_total` | counter | route = index / category, outcome = served / the refusal code |
| server | `cooked_data_version_replies_total` | counter | outcome, reason, category (`VersionReply::outcome()`, `reason()`, the category name or `unknown`) |
| launcher | `cooked_presync_category` | INFO, target `launcher.launch` | event, operation, category_id, pak, action, cache_version, source_version, served_version, changed, bytes |
| launcher | `cooked_presync_finished` | INFO, or WARN when `failed`; target `launcher.launch` | event, operation, outcome, reason, step, category_id, http_status, io_kind, os_code, attempts, categories_checked, categories_written, bytes_downloaded, duration_ms (14 keys; the sender allows 16) |
| launcher | `launch_not_started` (existing) | WARN | reason `cooked_presync_failed` |

Launcher rows carry closed codes and numbers only: no host, path, URL or error text (F19). An absent value is omitted, not written as 0.

### Launcher (`cimmeria-launcher-core` `cooked_presync`; LP-20, LP-21, LP-26)

```rust
// io_detail.rs (LP-20)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] pub struct IoDetail { pub kind: &'static str, pub os_code: Option<i32> }
pub fn io_detail_of(error: &std::io::Error) -> IoDetail;
pub fn io_detail_in(error: &(dyn std::error::Error + 'static)) -> Option<IoDetail>;  // walks source() to the first io::Error
// fetch.rs (LP-20)
pub const INDEX_CAP: usize = 256 * 1024;
pub const BODY_CAP: usize = 32 * 1024 * 1024;
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const INDEX_TIMEOUT: Duration = Duration::from_secs(15);   // whole index request
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(15);    // per body chunk; no total body limit (D-LP25)
pub fn base_url(login_url: &str) -> String;               // login_url without trailing '/' + "/api/cooked/v1"
pub fn server_label(login_url: &str) -> Option<String>;   // "host:port" (known default port when absent); UI only, never telemetry
pub struct Timeouts { pub connect: Duration, pub index: Duration, pub idle: Duration }   // Timeouts::DEFAULT = the three constants
pub struct Fetcher { /* reqwest::Client: connect timeout, no redirects, no total timeout */ }
impl Fetcher {
    pub fn new(login_url: &str, login_servers: &[String]) -> Result<Self, FetchError>;   // with_timeouts(.., Timeouts::DEFAULT)
    pub fn with_timeouts(login_url: &str, login_servers: &[String], timeouts: Timeouts) -> Result<Self, FetchError>;  // EndpointPolicy::from_login_servers(..).check(..)
    pub async fn index(&self) -> Result<Index, FetchError>;
    pub async fn body(&self, category_id: u32, version: u32) -> Result<(CategoryBody, u64), FetchError>;
}
#[derive(Debug, Clone, PartialEq)] pub enum FetchError { EndpointRefused, ConnectFailed(Option<IoDetail>), Timeout, ServerUnsupported, Disabled,
    CatalogUnavailable { retry_after_s: Option<u32> }, CatalogFailed, RateLimited { retry_after_s: Option<u32> },
    VersionChanged, UnknownCategory, HttpError(u16), TooLarge, BadIndex, BadBody }
impl FetchError {
    pub fn reason(&self) -> &'static str;     // snake_case of the variant; HttpError gives "http_error"
    pub fn http_status(&self) -> Option<u16>; // 404/503/429/409 for the mapped variants, the code for HttpError, else None
    pub fn retry_after_s(&self) -> Option<u32>;
    pub fn io(&self) -> Option<IoDetail>;
}
// apply.rs (LP-21)
pub const TEMP_SUFFIX: &str = ".cimmeria-presync.tmp";
pub fn cache_dir(firesky: &Path) -> PathBuf;              // firesky/SGWGame/Cache.en-US
pub fn ensure_not_linked(firesky: &Path) -> Result<(), ApplyError>;   // no symlink or junction from firesky down to cache_dir
pub fn sweep_temps(cache_dir: &Path) -> u32;              // deletes *TEMP_SUFFIX files directly in cache_dir, best effort
pub struct Plan { pub action: Action, pub cache_version: Option<u32>, pub source_version: Option<u32> }
pub fn plan(entry: &IndexEntry, cache_dir: &Path, source_dir: &Path) -> Plan;
pub struct Staged { pub category_id: u32, pub pak: String, pub bytes: u64, /* temp: PathBuf, committed: bool */ }  // Drop deletes the temp unless committed
pub fn stage(category_id: u32, body: &CategoryBody, pak: &str, cache_dir: &Path, source_dir: &Path) -> Result<Staged, ApplyError>;
pub struct CommitError { pub committed: u32, pub category_id: u32, pub pak: String, pub error: ApplyError }
pub fn commit(staged: Vec<Staged>, cache_dir: &Path) -> Result<u32, CommitError>;   // pre-check every destination, then rename ascending
#[derive(Debug, Clone, PartialEq)] pub enum ApplyError { SourceMissing, SourceUnreadable(IoDetail), BaseMissing { missing: u32, first: u32 },
    DigestMismatch, PakInUse(IoDetail), WriteFailed(IoDetail), UnsafePath }
impl ApplyError { pub fn reason(&self) -> &'static str; pub fn io(&self) -> Option<IoDetail> }
// run.rs (LP-26)
pub const DEADLINE: Duration = Duration::from_secs(120);
pub const MAX_WAITS: u32 = 3;  pub const MAX_WAIT: Duration = Duration::from_secs(30);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Stage { Checking, Downloading, Writing, Committing }
pub struct Progress { pub stage: Stage, pub categories_done: u32, pub categories_total: u32, pub bytes_done: u64, pub bytes_total: u64 }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Outcome { UpToDate, Synced, Skipped, Failed, Cancelled }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)] #[serde(rename_all = "snake_case")]
pub enum Step { Prepare, Index, Plan, Download, Stage, Commit }
#[derive(Debug, Clone, PartialEq, Serialize)] pub struct Failure { pub step: Step, pub reason: &'static str, pub category_id: Option<u32>,
    pub pak: Option<String>, pub http_status: Option<u16>, pub io_kind: Option<&'static str>, pub os_code: Option<i32>,
    pub retry_after_s: Option<u32>, pub source_version: Option<u32>, pub served_version: Option<u32>, pub deadline_s: Option<u32> }
pub enum CategoryAction { UpToDate, Written, Failed }
pub struct CategoryReport { pub category_id: u32, pub pak: String, pub action: CategoryAction,
    pub cache_version: Option<u32>, pub source_version: Option<u32>, pub served_version: u32, pub changed: u32, pub bytes: u64 }
pub struct Report { pub outcome: Outcome, pub reason: &'static str, pub failure: Option<Failure>, pub categories: Vec<CategoryReport>,
    pub bytes_downloaded: u64, pub duration_ms: u64, pub attempts: u32 }
impl Report { pub fn failed(failure: Failure) -> Self; pub fn skipped(reason: &'static str) -> Self; }  // for the engine's own early exits
pub async fn run(fetcher: &Fetcher, firesky: &Path, source_dir: &Path, deadline: Instant, cancel: &CancellationToken,
    progress: &(dyn Fn(Progress) + Send + Sync)) -> Report;
```

`io_kind` codes: `sharing_violation` (OS 32), `lock_violation` (OS 33), `storage_full` (`StorageFull` or OS 112), `permission_denied`, `not_found`, `connection_refused`, `connection_reset`, `timed_out`, `host_unreachable`, `network_unreachable`, `other`.

**Failure table.** `Report.reason` equals `failure.reason` when failed. The `step` and `reason` pairs, and the fields each sets:

| Step | Reasons | Extra fields |
|---|---|---|
| `prepare` (engine, LP-22) | `profile_unavailable`, `no_login_server`, `endpoint_refused` | - |
| `prepare` (run) | `unsafe_path` | - |
| `index` | `connect_failed`, `timeout`, `catalog_unavailable`, `catalog_failed`, `rate_limited`, `http_error`, `too_large`, `bad_index` | `http_status`, `io_kind`, `os_code`, `retry_after_s` when known |
| `plan` | `source_newer` | `category_id`, `pak`, `source_version`, `served_version` |
| `download` | the index reasons, plus `unknown_category`, `version_changed`, `bad_body` | as `index`, plus `category_id`, `pak` |
| `stage` | `source_missing`, `source_unreadable`, `base_missing`, `digest_mismatch`, `write_failed` | `category_id`, `pak`, `io_kind`, `os_code` |
| `commit` | `pak_in_use`, `write_failed` | `category_id`, `pak`, `io_kind`, `os_code` |
| any | `deadline` (the step in progress), `cancelled` (outcome `cancelled`) | `deadline_s`, plus the category in progress |

Non-failure reasons: `versions_match` (`up_to_date`), `synced`, and the skip reasons `disabled`, `server_unsupported` (`run`, D-LP19) and `platform_unsupported` (engine, D-LP9).

**`run` order** (D-LP18, D-LP20, D-LP22, D-LP23, D-LP25). Every network await and every wait races the deadline and the cancel token; a deadline gives `Failed`/`deadline`, a cancel gives `Cancelled`/`cancelled`.

1. `progress(Checking)`. `ensure_not_linked(firesky)`, else `Failed` (`prepare`, `unsafe_path`). `sweep_temps(cache_dir)`.
2. `index()`. `ServerUnsupported` or `Disabled` give `Skipped`. `CatalogUnavailable` and `RateLimited` wait `retry_after_s` (default 5 s) and retry, if it is at most `MAX_WAIT`, fits before the deadline, and fewer than `MAX_WAITS` waits have happened in this run; otherwise, and for every other error, `Failed` at step `index`.
3. `plan` every entry. Any `SourceNewer` gives `Failed` (`plan`, `source_newer`) before any download. No `Write` gives `UpToDate`/`versions_match`.
4. For each `Write` category in ascending id: `progress(Downloading)`, `body()` (the same waits; `VersionChanged` restarts from step 2 once, `attempts` counts index fetches), `progress(Writing)`, `stage` in `spawn_blocking`. The first error gives `Failed` at that step, with the category; earlier `Staged` values drop and delete their temps.
5. A last deadline and cancel check, then `progress(Committing)` and `commit` in `spawn_blocking`, awaited without the race: a commit that has begun finishes (D-LP23). A `CommitError` gives `Failed` (`commit`).
6. `Synced`/`synced`.

`categories_total` counts the `Write` categories, and `bytes_total` sums their `body_bytes`.

### Engine and shell (`ContentSync`; LP-22, LP-23, LP-24)

**`LaunchStatus.content_sync`** (LP-22 defines it, LP-23 serializes it, LP-24 reads it). The shell always writes the key, `null` until the presync starts. Every key inside is always present (null when empty):

```json
"content_sync": {"phase": "done", "categories_done": 1, "categories_total": 3, "bytes_done": 1048576, "bytes_total": 4194304,
  "outcome": "failed", "reason": "http_error", "server": "play.example:8081",
  "failure": {"step": "download", "reason": "http_error", "category_id": 4, "pak": "CookedDataItems.pak", "http_status": 500,
    "io_kind": null, "os_code": null, "retry_after_s": null, "source_version": null, "served_version": null, "deadline_s": null}}
```

- `phase` is one of `checking`, `downloading`, `writing`, `committing`, `done`.
- `outcome` and `reason` are null until `done`; then `outcome` is one of `up_to_date`, `synced`, `skipped`, `failed`, `cancelled`, and `reason` its code.
- `server` is `server_label` of the first login server, or null. `failure` is null unless `outcome` is `failed`.
- Counts are u32 and bytes u64 (≤ 2^53 in practice). The value lives in the worker's watch channel only (D-LP26): it stays readable until the next Play, and is gone after a launcher restart.

**The gate** (LP-22 defines it, LP-27 applies it): `up_to_date`, `synced`, `skipped` continue to client setup; `failed` returns `Ok(Observation::NotStarted)` after `events::launch_not_started(id, "cooked_presync_failed")`; `cancelled` returns `Ok(Observation::Cancelled)`.

## Wave 1

### LP-01 `cimmeria-cooked-pak`: PAK read/write, digest, workspace wiring

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-01-cooked-pak` · **Worktree:** `lcp-01`
- **Commit:** `feat(cooked-pak): shared cooked PAK reader, writer and digest (LP-01)`
- **Rules:** R1-R9. Contract: the LP-01 part of § Crate `cimmeria-cooked-pak`.

Files and changes:

1. New `crates/cooked-pak/Cargo.toml`: package `cimmeria-cooked-pak`, edition and lints like `crates/patchset/Cargo.toml`. Deps: `zip`, `sha2`, `serde` (derive), `serde_json`, `thiserror`, declared the way patchset declares its deps, plus `cimmeria-workspace-hack = { version = "0.1", path = "../workspace-hack" }`. Dev-dep `tempfile`.
2. New `src/lib.rs` (a `//!` summary of the crate and the no-CME-bytes rule; `pub mod pak; pub mod digest;`), `src/pak.rs` and `src/digest.rs` per the Contract. Copy the write options from `crates/patchset` (`SimpleFileOptions::default().compression_method(..).last_modified_time(zip::DateTime::default())`).
3. Root `Cargo.toml`: add `"crates/cooked-pak"` to `members` (l.9..) and `default-members` (l.72..), alphabetically, and `cimmeria-cooked-pak = { path = "crates/cooked-pak" }` beside `cimmeria-resources` (l.267).
4. `crates/server/src/logging/target_scan_tests.rs` l.28: add `"cooked-pak"` to `IN_PROCESS_CRATES`, alphabetically. `crates/server/src/logging/parity_tests/crate_rows.rs` l.37: add `("cooked-pak", NEVER_EXPORTED_BELOW_INFO)` to `NO_OWN_ROW`.
5. `tools/crate-graph/groups.toml`: add `cimmeria-cooked-pak` to the group that holds `cimmeria-resources` (l.39).
6. `crates/README.md`: one row for `cooked-pak`, in the shape of its neighbours (CRLF).
7. Hakari: run `pwsh tools/build-lane/lane.ps1 cargo hakari generate` and `... cargo hakari manage-deps --yes`, then commit whatever they change in `crates/workspace-hack` and `crates/cooked-pak/Cargo.toml`.

Tests, in `pak.rs` and `digest.rs`:

- `write_then_read_round_trips_entries_and_version`: write three entries and version 7 to a temp file; `read_pak` gives the same map and `Some(7)`. It fails if the writer drops or renames entries.
- `metadata_is_last_stored_and_four_bytes`: open the written zip with `zip::ZipArchive`. The last entry is `MetaData`, with `CompressionMethod::Stored` and size 4, and the `_<key>` names are ascending. It fails if the order or storage changes. LP-06 depends on this layout.
- `a_pak_without_metadata_reads_version_none` and `unknown_entry_names_are_ignored`: a hand-built zip with `foo` and `_x` entries.
- `parse_entry_name_accepts_only_underscore_u32`: `_12` gives 12; `12`, `_`, `_-1` and `_4294967296` give None.
- `digest_changes_with_any_byte_key_version_or_category`, and `digest_hex_round_trips`.

Checks: M1, M2, M3 `cimmeria-cooked-pak`; M3 `cimmeria-server target_scan` and M3 `cimmeria-server crate_rows`; H1.

Docs rows: 27 (new crate: crates/README, groups.toml, the scan lists). The crate graph regenerates on `main` (R8).

### LP-02 `cimmeria-cooked-pak`: index, body codec, compose, decide

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-02-catalog-codec`, at LP-01's reviewed head · **Worktree:** `lcp-02`
- **Commit:** `feat(cooked-pak): presync index, CPS1 body codec, compose and decide (LP-02)`
- **Rules:** R1-R9. Contract: the LP-02 part of § Crate `cimmeria-cooked-pak`, the Index JSON, the `CPS1` body, compose and decide.

Files and changes:

1. New `crates/cooked-pak/src/catalog.rs`, `compose.rs` and `decide.rs`, declared in `lib.rs`, exactly per the Contract. `BodyError` and `ComposeError` get `#[error("...")]` messages.
2. `decode_body` reads with a cursor. Before allocating, it checks `key_count <= max_keys`, `changed_count <= key_count`, each `len <= max_entry_bytes`, and the running total `<= max_total_bytes` (else `TooLarge`). Strictly ascending keys, or `NotAscending`. A changed key absent from the key list gives `ChangedKeyNotListed`. Bytes after the digest give `Trailing`; a short read gives `Truncated`.
3. If any file passes 400 lines, put its tests in a sibling `<name>_tests.rs` through `#[cfg(test)] #[path = "<name>_tests.rs"] mod tests;`.

Tests:

- `catalog`:
  - `body_round_trips`;
  - one test per `BodyError` variant, each a hand-mangled encoding;
  - `limits_are_checked_before_allocation`: `key_count = u32::MAX` with a 20-byte buffer gives `TooLarge`, not a panic or OOM;
  - `index_json_ignores_unknown_fields`.
- `compose`:
  - `compose_takes_changed_over_base_and_drops_extra_base_keys`;
  - `compose_reports_missing_base_keys` (count and first);
  - `compose_rejects_a_wrong_digest`.

  Build the digest with `category_digest` over the expected result. Each test fails if compose stops preferring `changed`, stops checking, or keeps extra keys.
- `decide`, as a table test over (cache, source, served):
  - (None, None, 5) gives Write;
  - (Some(5), None, 5) gives UpToDate;
  - (Some(4), Some(5), 5) gives UpToDate, because the client copies the source;
  - (Some(5), Some(6), 5) gives SourceNewer;
  - (Some(6), Some(4), 5) gives Write;
  - (Some(!5), Some(4), 5) gives Write (an interrupted push, F1);
  - (Some(u32::MAX), None, 1) gives Write.

Checks: M1, M2, M3 `cimmeria-cooked-pak`.

Docs rows: none (LP-12 documents the format).

### LP-03 `cimmeria-resources`: served catalog

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-03-served-catalog`, at LP-02's reviewed head · **Worktree:** `lcp-03`
- **Commit:** `feat(resources): served cooked-data catalog for the launcher presync (LP-03)`
- **Rules:** R1-R9. Contract: § Server, `ServedCatalog`, the Index JSON and the `CPS1` body (excerpts above).

Files and changes:

1. `crates/resources/Cargo.toml`: add `cimmeria-cooked-pak = { workspace = true }`. Add `tempfile` as a dev-dep if it is not one already.
2. `crates/resources/src/base/resources/mod.rs`: make `fn load_pak` (l.308) `pub(crate)`. Change nothing else there.
3. `crates/resources/src/lib.rs`: add `pub mod served_catalog;` inside `pub mod base { ... }`.
4. New `crates/resources/src/base/served_catalog/mod.rs`, per the Contract. `build`:
   - For each `(id, pak)` in `CATEGORY_PAKS`, take the category the cache serves and its version. Use the same accessor and version that `handle_version_info_request` uses: re-grep `server_version` and the cache accessor in `crates/base-session/src/base/cooked_data.rs` and `cooked_sync/`.
   - A category the cache doesn't hold at all is left out (no log).
   - Re-load the raw PAK with `load_pak(&format!("{data_dir}/{pak}"))`. If that fails, `build` returns `Err(format!("category {id}: raw_pak_unreadable"))`. It doesn't skip the category (D-LP18: the launcher blocks Play on failure, so a hole in the catalog must be loud).
   - `changed` is every served key whose bytes differ from the raw entry, or that the raw PAK lacks. `keys` is every served key, ascending.
   - The digest is `category_digest(id, version, served entries)`. Encode with `encode_body`. If the body exceeds `Limits::DEFAULT.max_total_bytes`, `build` returns `Err(format!("category {id}: category_too_large ({n} bytes)"))`.
   - `IndexEntry.pak` is the `CATEGORY_PAKS` name (with `.pak`). `index_json` is `serde_json::to_vec(&Index { schema: 1, .. })`, categories ascending.
5. `Published`, `publish` and `current` use a `static CATALOG: OnceLock<Published>` (Contract). `Published::Failed` carries a closed code: `raw_pak_unreadable` or `category_too_large`. Add `pub fn failure_code(error: &str) -> &'static str` that maps the `build` error string to the code; anything else maps to `build_failed`.

Tests (unit, in `served_catalog/tests.rs` through `#[path]`):

- `every_overridden_key_whose_bytes_differ_is_changed`:
  - write a temp `data_dir` holding an in-memory items PAK (category 4) built with `cimmeria_cooked_pak::pak::write_pak`, and build a `ResourceCache` from it the way existing tests do (re-grep `load_raw_mission_category_for_test` and the `test-support` helpers);
  - change one entry the way an override would (or use `from_bodies` if a cache can't be built without real data);
  - assert `changed` holds exactly that key, and the digest equals `category_digest` over the served map.

  It fails if `changed` is computed from the wrong side or left empty.
- `an_unchanged_category_has_no_changed_entries_and_lists_every_key`.
- `an_oversize_body_fails_the_build_naming_the_category`: pass a tiny `Limits` to a private helper; the `Err` names the id and `category_too_large`, and `failure_code` maps it.
- `an_unreadable_raw_pak_fails_the_build`: the cache holds category 4 but the raw file is removed; `Err` names `raw_pak_unreadable`. It fails if the category is skipped instead.
- `publish_succeeds_once`: the first `publish(Published::Ready(..))` returns true and `current()` is `Some(Ready)`; the second returns false. Use a private `OnceLock` in a helper so the test doesn't fight the process global.

Checks: M1, M2, M3 `cimmeria-resources`; H1.

Docs rows: none (LP-12).

### LP-04 `cooked-pak-tool` binary

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-04-tool`, at LP-03's reviewed head · **Worktree:** `lcp-04`
- **Commit:** `feat(resources): cooked-pak-tool to build, compose and check presync data offline (LP-04)`
- **Rules:** R1-R9.

Files and changes:

1. `crates/resources/Cargo.toml`: `[[bin]] name = "cooked-pak-tool"`, `path = "src/bin/cooked-pak-tool.rs"`, `test = false`.
2. New `src/bin/cooked-pak-tool.rs` with a `//!` summary (one paragraph: what it does and its subcommands; doc-map row 20), plain `std::env::args` parsing, and these subcommands:
   - `build --data-dir <dir>`: loads `ResourceCache::load_all`, builds `ServedCatalog`, and prints the index JSON.
   - `compose --data-dir <dir> --source <SourceCache dir> --out <dir>`: for every category whose `decide` is `Write`, composes it from `<source>/<pak>` and writes `<out>/<pak>` with `write_pak`. It prints one line per category: `<id> <pak> <action> <reason>`.
   - `check --data-dir <dir> --source <dir>`: per category it prints `category_id, pak, served_version, source_version, keys, changed, base_missing, base_differs, action`. `base_differs` counts unchanged served keys whose source bytes differ from the served bytes. It exits 1 if any `base_missing` or `base_differs` is non-zero.
   - `layout --pak <file>`: lists the zip entries in order, with name, compression method, compressed size and size.
3. Put the per-category logic the subcommands share in `crates/resources/src/base/served_catalog/check.rs` as `pub fn check_category(...) -> CheckRow`, so it can be unit-tested.

Tests: in `check.rs`:

- `check_counts_missing_and_differing_base_entries`;
- `a_matching_base_reports_zero_and_action_write`.

Both use in-memory PAKs in temp dirs. They fail if the counts are swapped or the source is ignored.

Checks: M1, M2, M3 `cimmeria-resources`; then a smoke run: `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-resources --bin cooked-pak-tool -- layout --pak <a PAK written by the test fixtures or the build>`. If the run needs real data, report the smoke as not run.

Docs rows: 20 (the `//!` summary; the tools catalog regenerates on `main`).

### LP-07 Server counter `cooked_data_version_replies_total`

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-07-reply-counter`, at `main` · **Worktree:** `lcp-07`
- **Commit:** `feat(base-session): count cooked-data version replies by outcome (LP-07)`
- **Rules:** R1-R9.

Files and changes:

1. `crates/base-session/src/base/cooked_data.rs`: right after the `tracing::info!` that ends at l.83, add `cimmeria_observability::counter!("cooked_data_version_replies_total", "outcome" => reply.outcome(), "reason" => reply.reason(), "category" => category_name(category_id).unwrap_or("unknown"))`. Re-grep `category_name`'s import in that file. Change nothing else.
2. `crates/base-session/src/base/cooked_sync/tests/telemetry.rs`: add the test below next to `an_up_to_date_reply_names_its_branch`.

Test: `version_replies_are_counted_by_outcome`:

- `cimmeria_observability::testing::install();`
- take `counter_total("cooked_data_version_replies_total", &[("outcome","up_to_date"),("reason","versions_match"),("category","<stargates name>")])`, and the same for `full_resync`/`version_mismatch` on WORLD_INFO;
- send one matching request for STARGATES (13) and one mismatched one for WORLD_INFO (12) through the file's `rig`;
- assert each total rose by exactly 1.

Copy the counter pattern from `crates/organization/src/.../creation/tests/seams.rs` l.102-113. Get the category labels from `category_name`. It fails if the counter line is removed.

Checks: M1, M2, M3 `cimmeria-base-session cooked_sync`.

Docs rows: 36 (observability catalog) is owed by LP-12; say so in the report.

### LP-05 Offline check against the player's client copy

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `launcher-cache-presync/LP-05-offline-check`, at the `lcp-w1` head after LP-04 · **Worktree:** `lcp-05`
- **Commit:** `docs(analysis): launcher-cache-presync LP-05 offline check (LP-05)`
- **Rules:** R1-R9. Read-only on `<client-copy>` and on the Documents cache; write nothing outside the worktree.

The orchestrator supplies `<client-copy>`, the player's own client install, in the brief. Steps:

1. `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-resources --bin cooked-pak-tool -- check --data-dir data/cache --source "<client-copy>\Working\SGWGame\SourceCache.en-us"`. If that folder doesn't exist, use the stock `Cache.en-US` the install shipped with, and say which.
2. `compose` into a scratch folder outside the repo (your session scratchpad), never committed. Run `layout` on one composed PAK and on the same stock PAK, and compare the entry order, methods and `MetaData`.
3. List, read-only, the PAK file names in the Documents `Cache.en-US` (from `client_paths::firesky_root`'s rule). Compare them, case-sensitively, with the `CATEGORY_PAKS` names. Record names only, never the path.
4. Write `docs/analysis/launcher-cache-presync/worknotes/LP-05.md` (CRLF) with:
   - a table per category: id, pak, served version, source version, keys, changed, `base_missing`, `base_differs`, action;
   - the body size per category, and the largest one;
   - the layout comparison;
   - the name check;
   - a verdict: **compose works** (every `base_missing` and `base_differs` is 0, and no category's action is `SourceNewer`) or **ruling needed** (D-LP3). With Play blocked on failure (D-LP18), a `SourceNewer` category would block every player with that install, so it needs a ruling too.
   - If the largest body is over 16 MB, say so: at the D-LP25 idle timeout it still downloads, but the 120 s deadline needs a ruling on slow links.

Return `done` with the verdict in the report's first line.

## Lab proof

### LP-06 Lab proof: a composed cache gives `up_to_date` at character select

- **Host:** windows · **Agent:** rust-gameserver-dev, with the lab-uat skill · **Branch:** `launcher-cache-presync/LP-06-lab-proof`, at `main` after wave 1 · **Worktree:** `lcp-06` · **Status:** UATPending (the orchestrator asks the user first)
- **Commit:** `docs(analysis): launcher-cache-presync LP-06 lab proof (LP-06)`
- **Rules:** R1-R9, plus the lab-uat skill's rules: the lease, one lab-driver, verify the driver's claims against the server log.

Steps:

1. Build and start a local server from `lcp-06` with its own test database (`sgw_lcp-06`), as the lab-uat skill says.
2. **Baseline:** wipe the lab instance profile's Documents `Cache.en-US` (back it up first). Log in to character select and time it. Expect `cooked_data.version_reply` `full_resync` and `cooked_data.sync_start`.
3. **Proof:**
   - quit the client;
   - restore the backup or wipe again;
   - run `cooked-pak-tool compose --data-dir data/cache --source <that install's SourceCache.en-us> --out <the profile's Cache.en-US>`;
   - log in again.

   Expect:
   - every `cooked_data.version_reply` is `up_to_date`;
   - no `cooked_data.sync_start`;
   - `client.cooked.versions_held` shows `resync_pending = 0`;
   - no `client.cooked_cache.write` events;
   - `cooked_data_misses_total` / `miss_served` unchanged;
   - at least one overridden item or mission shows its server value in game.
4. Record the D-LP15 check if any served version is 2^31 or more.
5. Restore the profile's cache afterwards.
6. Write `docs/analysis/launcher-cache-presync/worknotes/LP-06.md` (CRLF) with:
   - the two timings;
   - the event counts per run;
   - pass or fail per expectation.

   No paths, IPs or account names.

## Wave 2

Wave 2 (LP-10, LP-11, LP-12) is in [packets-wave2.md](packets-wave2.md).

## Waves 3 to 5

Wave 3 (LP-20, LP-21, LP-26, LP-22, LP-27), wave 4 (LP-23, LP-24, LP-28, LP-25) and the close-out (LP-30, wave 5) are in [packets-wave3.md](packets-wave3.md). The rules, checks and contract above apply to them.
