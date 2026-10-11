# Any-Install Adoption Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-10-11. Companions: [ledger and decisions](README.md), [desktop launcher README](../../../crates/launcher/desktop/README.md), [migration contract](../../../crates/launcher/desktop/docs/migration.md), [testing playbook](../../../TESTING.md), [doc-update map](../../agents/doc-update-map.md).

## Dispatch rules

- One worktree per worker, made from the main checkout with `pwsh tools/build-lane/mk-worktree.ps1 any-install-adoption/<packet>-<slug> ai-<nn>`. Packet ids are lower-case in names (`ai-05`).
- **PowerShell only.** No bash, no WSL, no direct `cargo`, no `git worktree prune`, no `git stash`.
- Every compiling cargo call goes through `pwsh tools/build-lane/lane.ps1`. `npm` calls do not compile Rust and run directly.
- The desktop launcher is its own workspace. Its checks always pass `--locked --manifest-path crates/launcher/desktop/Cargo.toml --target-dir target/desktop`. `cimmeria-launcher-core` is in the root workspace **and** a path dependency of the desktop engine, so a core change runs both R1 and D2/D3.
- Workers commit locally and report once, with results. The coordinator runs a `packet-reviewer` (or `packet-review-fixer`) on the diff, plus `server-authority-enforcer` for AI-06, AI-10 and AI-11 (they take paths and bytes from outside the launcher), then ships with `python tools/build-lane/ship.py pr -C <worktree>` and merges with `ship.py merge <PR> --retire <worktree>`.
- Frontend changes owe a REPL-style logic UAT ([AGENTS.md](../../../AGENTS.md)): an `npm run uat:<feature>` script beside the unit tests.
- Every button gets visible feedback on the first press: a busy state while native work runs, then a result line.
- **D-AI5 holds in every packet.** No code, test, fixture or doc may upload, mirror or cache seed bytes or extracted seed files anywhere but the player's own machine. Test fixtures build their own tiny seeds (`write_stored_rar4`, zip fixtures); nothing commits real client files.
- Native Windows behaviour (sharing violations, ACLs, verbatim paths) is proved by tests that run on Windows through the lane; say in the report which tests are `#[cfg(windows)]`.
- Commit messages end with the attribution lines the coordinator puts in the brief.
- Initial state: documentation only, against `main` @ `ee899c2c0`.

### Standard checks

Each packet lists which of these it runs, in this order.

```powershell
# D1 format (no compile)
cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all -- --check
# D2 clippy, the crates the packet touched
pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop --all-targets -- -D warnings
# D3 tests, same crates
pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop
pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop
# F1 frontend
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run check --prefix crates/launcher/desktop/frontend
npm test --prefix crates/launcher/desktop/frontend
# F2 native-backed adoption UAT (Windows runs adoption-in-place-uat.mjs)
pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --target-dir target/desktop --no-run
#   then set ADOPTION_UAT_BINARY to the printed shell test binary and:
npm run uat:adoption --prefix crates/launcher/desktop/frontend
# R1 root workspace, launcher-core
cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-launcher-core --all-targets -- -D warnings
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-launcher-core
```

Test types refer to [TESTING.md](../../../TESTING.md): **unit** (type 1) for engine and core filesystem fixtures, **wire-format** (type 2) for byte-exact serialized records (plan, provenance and index JSON whose bytes feed a digest).

## Contract fixed by this ledger

Wave 1+ packets build against these names. AI-01, AI-02 and AI-05 create them; nobody renames them locally. A worker who needs a change raises it with the coordinator.

### File index (AI-01, launcher-core `src/file_index.rs`)

The index is the launcher's reference reconstruction written down: the files the seed produces after `place_bundled_cooked_data`, then the files each signed patch reads and writes, then the setup rules. Paths are relative to the client root (the folder holding `Working`), `/`-separated, in the stock spelling `client_setup::stock_case` restores, and never include `launcher-installed.json`. Hashes are lower-case hex.

```json
{
  "schema_version": 1,
  "setup_policy_version": 1,
  "seed": {
    "sha256": "<manifest.seed.sha256>",
    "files": [ { "path": "Working/Binaries/SGW.exe", "size": 9273344, "sha256": "…" } ]
  },
  "patches": [
    {
      "state_key": "007-castle-armory-ring",
      "sha256": "<manifest patch sha256>",
      "inputs":  [ { "path": "Working/SGWGame/CookedPC/Maps/…", "size": 1, "sha256": "…" } ],
      "outputs": [ { "path": "Working/SGWGame/CookedPC/Maps/…", "size": 1, "sha256": "…" } ]
    }
  ],
  "setup": [
    { "rule": "aslr_off", "path": "Working/Binaries/SGW.exe", "size": 9273344, "sha256": "…" },
    { "rule": "login_servers", "path": "Working/SGWGame/Content/UI/Startup/Login/LoginInternal.lua" }
  ]
}
```

`inputs` are the files a patch reads at their pre-patch bytes (a patch set's op sources); a whole-file overlay has none. `outputs` are every file the patch writes, at their post-patch bytes.

```rust
pub const FILE_INDEX_SCHEMA: u32 = 1;
/// Bumped when client_setup changes what it writes. A launcher ignores an
/// index whose policy it does not implement and builds one locally instead.
pub const SETUP_POLICY_VERSION: u32 = 1;
/// The blob's cap. The real index is about 1 MB (README F-9).
pub const MAX_FILE_INDEX_BYTES: u64 = 16 * 1024 * 1024;

/// In the signed manifest. Optional, so no schema bump (the `min_launcher` precedent).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileIndexRef { pub blob: String, pub size: u64, pub sha256: String }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIndex {
    pub schema_version: u32,
    pub setup_policy_version: u32,
    pub seed: SeedLayer,
    pub patches: Vec<PatchLayer>,
    pub setup: Vec<SetupRule>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedLayer { pub sha256: String, pub files: Vec<IndexedFile> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PatchLayer {
    pub state_key: String,
    pub sha256: String,
    #[serde(default)]
    pub inputs: Vec<IndexedFile>,
    pub outputs: Vec<IndexedFile>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndexedFile { pub path: String, pub size: u64, pub sha256: String }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case", deny_unknown_fields)]
pub enum SetupRule {
    AslrOff { path: String, size: u64, sha256: String },
    LoginServers { path: String },
}
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FileIndexError {
    #[error("file index is larger than its cap")] TooLarge,
    #[error("file index bytes do not match the signed reference")] Digest,
    #[error("unsupported file index schema {0}")] Schema(u32),
    #[error("unsupported setup policy {0}")] SetupPolicy(u32),
    #[error("file index does not describe this release: {0}")] Binding(String),
    #[error("invalid path in file index: {0}")] InvalidPath(String),
    #[error("duplicate path in one layer: {0}")] Duplicate(String),
    #[error("{0}")] Io(String),
}

/// Where one path's bytes come from at one layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin { Seed, Patch(usize) }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State { pub origin: Origin, pub size: u64, pub sha256: [u8; 32] }
/// Every state a path passes through, oldest first. `release()` is the last.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathHistory { pub path: String, pub states: Vec<State> }

impl FileIndex {
    /// Size and SHA-256 against the signed reference first, then parse, then `validate`.
    pub fn parse_verified(bytes: &[u8], reference: &FileIndexRef) -> Result<Self, FileIndexError>;
    /// Schema, setup policy, path rules (relative, no `..`, `.`, empty, `\`, `:`
    /// or leading `/`, ASCII), no duplicate path within one layer (case-blind).
    pub fn validate(&self) -> Result<(), FileIndexError>;
    /// Seed sha256 equal (case-blind) and the patch list equal to the manifest's,
    /// position by position: `state_key()` and `sha256`.
    pub fn bind(&self, manifest: &Manifest) -> Result<(), FileIndexError>;
    /// Keyed by lower-cased path.
    pub fn histories(&self) -> BTreeMap<String, PathHistory>;
}
```

`Manifest` (`crates/launcher-core/src/manifest.rs:158`) gains, after `patches`:

```rust
/// Signed per-file index (any-install D-AI2). Optional: no schema bump.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub file_index: Option<FileIndexRef>,
```

### Local seed archive (AI-05, launcher-core `src/seed_source.rs`)

```rust
/// What the seed step uses. Shared by Install, Repair, Update and adoption.
#[derive(Clone)]
pub struct SeedSourceHandle { /* Arc<Mutex<Option<VerifiedLocalSeed>>>, switch: CancellationToken */ }
impl SeedSourceHandle {
    pub fn download() -> Self;
    pub fn local(seed: VerifiedLocalSeed) -> Self;
    /// Switch a running seed step to `seed`: stops its download at the next
    /// chunk and removes the partial file. Later seed steps use it too.
    pub fn use_local(&self, seed: VerifiedLocalSeed);
    pub fn current(&self) -> Option<VerifiedLocalSeed>;
    pub(crate) fn switched(&self) -> CancellationToken;
}

/// A player's archive whose size and SHA-256 matched the signed seed when it
/// was chosen. Not proof at use: `copy_into` re-hashes what it copies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiedLocalSeed {
    pub path: PathBuf,
    pub size: u64,
    pub sha256: String,
    pub identity: LocalFileIdentity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalFileIdentity { pub volume: u64, pub file_id: u128, pub modified_ns: i128 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum LocalSeedError {
    #[error("not a regular file")] NotAFile,
    #[error("archive size {actual} does not match the signed seed ({expected})")] SizeMismatch { expected: u64, actual: u64 },
    #[error("archive does not match the signed seed")] HashMismatch,
    #[error("archive changed after it was checked")] Changed,
    #[error("cancelled")] Cancelled,
    #[error("I/O error")] Io,
}

/// Size first (instant refusal), then a streamed SHA-256 with progress and cancel.
/// Opens without following links; reads only.
pub fn verify_local_seed(path: &Path, seed: &SeedEntry, cancel: &CancellationToken, progress: &ProgressSink) -> Result<VerifiedLocalSeed, LocalSeedError>;
impl VerifiedLocalSeed {
    /// Opens `path` without following links, checks identity and size are the
    /// verified ones, copies to `<dest>.local-copy` while hashing, fsyncs, and
    /// renames onto `dest` only if the hash matches. Never writes, moves or
    /// deletes `path`.
    pub fn copy_into(&self, dest: &Path, cancel: &CancellationToken, progress: &ProgressSink) -> Result<(), LocalSeedError>;
}
```

`InstallContext` (`crates/launcher-core/src/install.rs:141`) gains `pub seed_source: &'a SeedSourceHandle`. `install_seed::apply` checks `current()` before downloading; `download_to_file` takes the switch token and returns the new `InstallError::SeedSwitched` when it fires.

### Adoption source and records (AI-02, engine `storage/adoption/`)

Serialized records that already exist on players' disks (`adoption-plan-*.json`, `adoption-*.json`, `installed-content.json`) must read back and **digest to the same bytes**. So the in-memory source is an enum, and the serialized form adds only optional fields that are skipped when empty.

```rust
// model.rs
pub enum AdoptionSource {
    Legacy(migration::LegacyImport),
    Folder(FolderSource),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FolderSource {
    /// What the player picked: the client root, its `Working`, or `Working\Binaries`.
    pub selected: PathBuf,
    /// Canonical client root, the folder holding `Working` (AI-10 resolves it).
    pub root: PathBuf,
    /// `digest(&inventory::scan(root))` at review. Replaces `import_digest` as the
    /// "folder changed after review" guard.
    pub scan_digest: [u8; 32],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdoptionMode { InPlace, Copy }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification { Matched, KnownTransform, NeedsPatching, Modified, Missing, Extra }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadEstimate { pub seed: bool, pub patches: Vec<String>, pub bytes: u64 }
/// One file the adopted tree gets from the replacement store (AI-09).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replacement { pub path: String, pub size: u64, pub sha256: [u8; 32], pub reason: Classification }
```

Field changes, each `#[serde(default, skip_serializing_if = "...")]` so an old record's bytes are unchanged:

| Type | Change |
|---|---|
| `Choices` | add `replace_modified_files: bool` (skip if false) |
| `Report` | `imported_identity` and `requested_config` become `Option<_>` (skip if none); add `download: Option<DownloadEstimate>`, `backup: Option<PathBuf>` |
| `Provenance` | `legacy_install_id` and `import_digest` become `Option<_>`; add `folder_scan_digest: Option<[u8; 32]>`, `file_index_sha256: Option<String>` |
| `Plan` | `imported` becomes `Option<LegacyImport>`; add `folder: Option<FolderSource>`, `replacements: Vec<Replacement>` (skip if empty); `Plan::source(&self) -> Result<AdoptionSourceRef<'_>, Error>` requires exactly one of `imported` / `folder` |
| `Plan::digest` | `Digested` mirrors every new field with the same skip attributes |

`Preview` replaces `imported: LegacyImport` with `source: AdoptionSource` and `_legacy_lock: File` with `_legacy_lock: Option<File>`. `PreviewRequest` replaces `import_digest: String` with `source: SourceRequest`:

```rust
pub enum SourceRequest {
    /// The saved import, as today. `destination` keeps its meaning.
    Legacy { import_digest: String },
    /// A natively picked folder; AI-10 resolves it.
    Folder { selected: PathBuf },
}
pub struct PreviewRequest {
    pub source: SourceRequest,
    pub mode: AdoptionMode,
    pub destination: PathBuf,           // == root in place; the new folder for a copy
    pub operation_revision: u64,
    pub preferences_revision: u64,
    pub release: VerifiedRelease,
    pub seed_source: SeedSourceHandle,  // AI-06 fills it; AI-02 passes `download()`
    pub artifacts: Option<Artifacts>,
}
```

New `Error` variants: `FolderNotWritable`, `NotAGameFolder`, `BackupConflict`, `UnsupportedMode`. `Error::OutdatedInstall` stays defined until AI-08 removes its only use.

New engine files AI-02 creates, so wave 2 packets do not collide in `mod.rs`:

| File | Owns | Filled by |
|---|---|---|
| `adoption/source.rs` | `pub(super) fn resolve(state, &PreviewRequest) -> Result<Resolved, Error>`; `Resolved { source: AdoptionSource, root: PathBuf, lock: Option<File>, servers: Vec<LoginServer> }`. AI-02 moves the legacy checks from `copy_preview` and `in_place_preview::preview` here; the `Folder` arm returns `Err(Error::UnsupportedMode)`. | AI-10 |
| `adoption/verification.rs` | `pub(super) fn verify(...) -> Result<Verified, Error>`; `Verified { reference: reference::Reference, files: Vec<Difference>, download: Option<DownloadEstimate>, replacements: Vec<Replacement> }`. AI-02 moves `reference_and_comparison` here unchanged. | AI-08, AI-09 |

### Shell IPC and TypeScript (AI-02, AI-06)

| Item | Shape | Filled by |
|---|---|---|
| Tauri `choose_existing_game_folder(mode: AdoptionMode)` | one folder picker ("Choose your Stargate Worlds game folder"); for `copy` a second picker for where the new `Stargate Worlds` folder goes; then `NativeHost::begin_folder_adoption(selected, mode, copy_location, release)`. AI-02 registers it returning `PlatformUnavailable`. | AI-10 (in place), AI-12 (copy) |
| `AdoptionCommand::Confirm` | add `replace_modified_files: bool` | AI-02 |
| `AdoptionStatus` | add `modes: Vec<AdoptionMode>` (Windows `[in_place, copy]` once AI-12 lands, macOS `[copy]`) | AI-02, AI-12 |
| `Review` | add `source_kind: "legacy" \| "folder"`, `counts.needs_patching`, `requires_replacement: bool`, `backup_directory: Option<PathBuf>`, `download: Option<DownloadEstimate>` | AI-02 |
| `AdoptionError` | add `folder_not_writable`, `not_a_game_folder`, `backup_conflict`, `unsupported_mode` | AI-02 |
| Tauri `seed_archive_command(request: SeedArchiveCommand)` | `#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]`: `Inspect`, `Cancel`, `Forget`, each `{ schema_version: 1 }` | AI-06 |
| Tauri `choose_seed_archive()` | native file picker (`*.rar`), then starts verification on a retained worker | AI-06 |
| `SeedArchiveStatus` | `{ schema_version: 1, native: NativeSnapshot, chosen: Option<{ file_name: String, size: u64 }>, verifying: Option<{ current: u64, total: u64 }>, last_error: Option<SeedArchiveError> }` | AI-06 |
| `SeedArchiveError` | flat snake_case string: `not_a_file`, `size_mismatch`, `hash_mismatch`, `changed`, `cancelled`, `busy`, `io`, `release_unavailable` | AI-06 |

`contract.ts` and `adoption-workflow.ts` gain the TypeScript mirrors in the packet that adds the Rust side; the decode is strict (`onExcessProperty: 'error'`), so Rust and TS change in the same commit.

## Wave 0

### AI-01: `file_index` types and the optional manifest field

- **Agent:** rust-gameserver-dev. **Branch:** `any-install-adoption/ai-01-file-index-types`. **Worktree:** `ai-01`.
- **Do:** create `crates/launcher-core/src/file_index.rs` exactly as in the contract (types, constants, `parse_verified`, `validate`, `bind`, `histories`); `pub mod file_index;` in `lib.rs`. Add `file_index: Option<FileIndexRef>` to `Manifest`. Fix every `Manifest { .. }` literal the compiler finds (tests, `overlay_pack.rs`, engine fixtures) with `file_index: None`.
- **Tests (unit):**
  - a manifest JSON without `file_index` parses and re-serializes byte-for-byte (fails if the field is not skipped when `None`; an old signed manifest's digest would change);
  - a manifest with `file_index` round-trips it (fails if the field is dropped, F-11);
  - `parse_verified` refuses one flipped byte with `Digest`, and a body over the cap with `TooLarge`;
  - `bind` refuses a different seed sha, a missing patch, a reordered patch and a patch with another sha (four cases, one assertion each);
  - `validate` refuses `../x`, `a\b`, `C:/x`, and two paths differing only by case in one layer;
  - `histories` on a seed file overwritten by patch 1 gives two states in order, and a file added by patch 2 gives one `Patch(1)` state.
- **Checks:** R1; then D2, D3 for `cimmeria-launcher-engine` (literals).
- **Docs:** `crates/README.md` `launcher-core` row (new module). The manifest schema comment at the top of `manifest.rs` gains the field.
- **Commit:** `feat(launcher-core): signed per-file index types and the optional manifest file_index field (AI-01)`.

### AI-02: engine, shell and TypeScript adoption contract scaffold

- **Agent:** rust-gameserver-dev. **Depends on:** AI-01. **Branch:** `any-install-adoption/ai-02-contract`. **Worktree:** `ai-02`.
- **Do:** everything under "Adoption source and records" and the AI-02 rows of "Shell IPC and TypeScript", with behaviour unchanged:
  - `Preview::consent` adds: `requires_replacement()` (any `Modified` or `Missing`) needs `choices.replace_modified_files`; telemetry acceptance applies only to `AdoptionSource::Legacy`. In place still refuses through `blocking()` until AI-11.
  - Every reader of `Provenance.import_digest` / `legacy_install_id` (`publication.rs:573-574`, `effective_settings/mod.rs:40-44`) treats `None` as `StorageError::Corrupt` for now; AI-10 gives folder provenance its meaning.
  - Move code into `source.rs` and `verification.rs` as the contract table says. No logic change.
  - Shell: `review.rs`, `status.rs`, `contract.rs`, `main.rs` registration; frontend `adoption-workflow.ts` schema and `codes`. The view renders nothing new yet.
- **Tests:**
  - **wire-format:** before editing anything, on `main`, write a test-only helper that prints the JSON and `Plan::digest()` of the in-place plan the existing fixture in `in_place_tests.rs` admits, and of its `Provenance`. Commit both JSON files as fixtures under `engine/src/storage/adoption/fixtures/` (`plan-v1-in-place.json`, `provenance-v1.json`) and the digest as a hex constant. The test reads them with the new types, re-serializes byte-identically, and recomputes the same digest. Fails if any new field is not skipped when empty or a renamed field changes order: a published adoption would then read as corrupt.
  - unit: `Choices` JSON without `replace_modified_files` reads as `false`; `Plan` with both `imported` and `folder` (and with neither) fails `source()`.
  - the existing `bridge_has_no_native_observation_or_arbitrary_field_command` test gains a `confirm` line with an unknown field and a `seed_archive_command` line once AI-06 adds it.
- **Checks:** D1, D2, D3, F1, F2.
- **Docs:** `crates/launcher/desktop/README.md` command list (new command name, marked "not yet available").
- **Commit:** `refactor(launcher-desktop): adoption source, mode and classification contract for any-install adoption (AI-02)`.

### AI-03: fix the Windows adoption reference for the RAR seed

- **Agent:** rust-gameserver-dev. **Branch:** `any-install-adoption/ai-03-rar-seed`. **Worktree:** `ai-03`.
- **Do:** in `engine/storage/adoption/reference.rs`, the native seed branch (`:52-62`) calls `extract`, which demands a zip (`strict_zip`, `:143-148`). Split it: patches keep `extract` (patches are zips); the seed goes through `authenticated_copy` then `crate::unpack::unpack` (which detects RAR, a CAB set inside it, or zip, and is what a fresh Windows install already uses), then removes its private copy. Keep `strict_zip` for a zip seed. Name-safety for RAR entries is `unpack::rar`'s job; read `unpack/rar.rs` and say in the report which checks it makes (path traversal, links) and whether any `strict_zip` check is missing there. If one is, add it in `unpack/rar.rs` with its own test.
- **Tests (unit):** an in-place fixture whose seed is a stored RAR4 built with `cimmeria_launcher_core::unpack::test_fixtures::write_stored_rar4` (the engine dev-dependency enables `test-fixtures`) adopts in place. Fails on revert with `UnsupportedArchive`. Add it beside the zip fixture in `in_place_tests.rs`; reuse the fixture builder with a `SeedKind::{Zip, Rar}` parameter.
- **Checks:** D1, D2, D3 for the engine; R1 if `unpack/rar.rs` changed.
- **Docs:** `crates/launcher/desktop/docs/migration.md` "In-place adoption on Windows" step 2: the seed may be the RAR. Add a line to `docs/analysis/launcher-consolidation/acceptance.md` that LX-26's adoption rows needed this fix.
- **Commit:** `fix(launcher-desktop): in-place adoption verifies against the RAR seed on Windows (AI-03)`.

## Wave 1

### AI-04: build the index, the `build-file-index` tool, stale indexes in `pack-client-overlay`

- **Agent:** rust-gameserver-dev. **Depends on:** AI-01. **Branch:** `any-install-adoption/ai-04-build-index`. **Worktree:** `ai-04`.
- **Do:**
  - `file_index::build(seed: &Path, patches: &[(PatchEntry, PathBuf)], manifest: &Manifest, work: &Path, sink: &UnpackSink) -> Result<FileIndex, FileIndexError>` in `file_index.rs` (move to `file_index/` with `build.rs` if the file passes 500 lines). It must reproduce `engine/storage/adoption/reference.rs:12-96` state by state:
    1. unpack the seed into `work/tree` with `unpack::unpack`; `install_layout::place_bundled_cooked_data`; restore stock case with `client_setup::stock_case::restore`; scan → `seed.files`;
    2. for each patch in signed order: before applying, read its zip's `cimmeria-patch.json` if present and record each op source (path relative to the client root, mapped through `patch_dest` for `root: sgw_game`) as `inputs` with the bytes currently in the tree; apply with `unpack::unpack` into `patch_dest::patch_dest(tree, patch)`; restore stock case; every path whose bytes changed or appeared is an `output`;
    3. `setup`: copy `SGW.exe`, run `client_setup::aslr::disable` on the copy, record `aslr_off`; record `login_servers` for `login_servers::path`.
    Never record `launcher-installed.json`. Paths use the scan's `/` names relative to the tree root.
  - `crates/launcher-core/src/bin/build-file-index.rs` (summary `//!` line for the tools catalog): `build-file-index --manifest manifest.json --seed <archive> --patch <id>=<zip>... --out file-index.json`. It checks every artifact's size and sha256 against the manifest first, runs `build`, writes the JSON (pretty-printed is fine; the hash covers the bytes as written), and prints the `"file_index": { "blob": "<fill in>", "size": N, "sha256": "…" }` entry to paste. It reads the seed from a local path only; it never downloads or uploads anything (D-AI5). Add a `[[bin]]` row in `crates/launcher-core/Cargo.toml`.
  - `pack-client-overlay --manifest`: when it appends a patch, a `file_index` already in the manifest no longer binds. Set it to `None` and print `file_index removed: rebuild it with build-file-index after adding patches`.
- **Tests (unit, core):**
  - fixture seed zip with three files plus one overlay patch replacing one of them and one patch set (built with `cimmeria_patchset` test helpers) deriving a new file from a seed file: the index has three seed files, the overlay's one output, the patch set's input at seed bytes and its output; `bind` passes against the fixture manifest.
  - **parity:** run `build` and, in the engine (`reference_tests.rs` is macOS-gated, so add `index_parity_tests.rs` gated `cfg(test)` on every OS), `reference::reconstruct` on the same artifacts; every final state in `histories()` equals `reference.raw`'s entry for that path, and the sets of paths are equal. Fails if `build` skips `place_bundled_cooked_data`, stock case or the `sgw_game` root.
  - `pack-client-overlay` merge drops a present `file_index` (fails on revert: the stale index would ship).
- **Checks:** R1; D2, D3 for the engine (parity test).
- **Docs:** `tools/README.md` catalog regenerates from the `//!` line (run `python tools/docs-gen/regen.py` locally to preview, do not commit generated blocks); `docs/client/launcher-distribution-setup.md` gains a short "File index" subsection that AI-14 completes.
- **Commit:** `feat(launcher-core): build the per-file release index and the build-file-index tool (AI-04)`.

### AI-05: local seed archive seam in launcher-core

- **Agent:** rust-gameserver-dev. **Branch:** `any-install-adoption/ai-05-seed-source`. **Worktree:** `ai-05`.
- **Do:** create `crates/launcher-core/src/seed_source.rs` as in the contract.
  - `verify_local_seed`: `symlink_metadata` must be a regular file (no reparse point on Windows: open with `FILE_FLAG_OPEN_REPARSE_POINT` and refuse if the opened handle is a reparse point; on other OSes `O_NOFOLLOW`); size first; then stream SHA-256 in 1 MiB reads through the same handle, reporting `Progress::Downloading { label: "Checking your game archive" }` and honouring `cancel` between reads. Identity on Windows from `GetFileInformationByHandleEx(FileIdInfo)` (volume serial and 128-bit file id) plus last-write time; elsewhere `dev`, `ino`, `mtime`.
  - `copy_into`: same no-follow open, identity and size must equal the verified ones (else `Changed`), copy to `<dest>.local-copy` hashing the bytes read, `sync_all`, compare hash (else remove the copy and `HashMismatch`), rename onto `dest`. On any error remove only `<dest>.local-copy`. The source handle is read-only; nothing ever renames, writes or deletes `path`.
  - `install_seed::apply` (`install_seed.rs:32-95`): if `ctx.seed_source.current()` is `Some`, call `copy_into(&archive, ..)` instead of the cache check and download, then continue unchanged (the existing code verifies again and later deletes **its own copy**). Otherwise download as today, but pass `ctx.seed_source.switched()` into `download_to_file`; when it fires, `download_to_file` returns `InstallError::SeedSwitched`; `apply` removes the partial `.tmp-seed-*.download` and runs the local branch.
  - `download_to_file` (`install.rs:339`): add the switch token parameter and check it beside `cancel` in the read loop. Update every caller (the patch path passes a never-firing token).
  - Add `seed_source` to every `InstallContext` literal (desktop engine included) as `&SeedSourceHandle::download()`; AI-06 wires the real handle.
- **Tests (unit, core):**
  - a wrong-size file is refused with `SizeMismatch` without reading it (assert via a 0-byte file claimed as huge, and time is not the measure: assert the progress sink got no `Downloading` event);
  - a right-size, wrong-content file is refused with `HashMismatch`; the file is still there and unchanged afterwards (fails if any path deletes or rewrites the player's file);
  - after `verify_local_seed`, replacing the file's bytes (same size) makes `copy_into` fail with `Changed` or `HashMismatch`, and `dest` does not exist (fails if the copy trusts the pick-time hash, D-AI15);
  - `install_all` with a local seed (a `write_stored_rar4` fixture) and an HTTP mock that fails every seed request installs successfully and leaves the player's file in place (fails if the download still runs);
  - a mock seed download slowed by a responder delay, switched mid-way with `use_local`: install succeeds, the partial `.tmp-seed-*.download` is gone, the mock saw no request after the switch.
  - `#[cfg(windows)]`: a symlink (or junction, when symlinks need a privilege the runner lacks: then skip with a printed reason) to a valid archive is refused with `NotAFile`.
- **Checks:** R1; D2, D3 for the engine (literals).
- **Docs:** `crates/README.md` `launcher-core` row.
- **Commit:** `feat(launcher-core): a verified local seed archive can replace the seed download (AI-05)`.

## Wave 2

### AI-06: local seed archive in the desktop engine and shell

- **Agent:** rust-gameserver-dev, reviewed by `server-authority-enforcer`. **Depends on:** AI-05. **Branch:** `any-install-adoption/ai-06-seed-archive-desktop`. **Worktree:** `ai-06`.
- **Do:**
  - Engine `storage/seed_archive/mod.rs`: `DesktopState` holds one `SeedSourceHandle` for the process. The chosen archive persists as `seed-archive.json` (the `VerifiedLocalSeed` plus the manifest seed sha it matched) in the state root, written atomically; on open it is restored only when its seed sha equals the current release's (a new seed means "choose again"). A retained worker (mirror `adoption::worker`) runs `verify_local_seed` with progress and cancel, then `handle.use_local(..)` and writes the record. `Forget` clears both.
  - Route every seed fetch through the handle: `install_worker/mod.rs:336`, `install_worker/wine/mod.rs:115`, `repair/preparation/mod.rs:198-227`, `update/preparation/mod.rs:201-232` pass `seed_source: &state.seed_source()`; adoption `artifacts::fetch` (`artifacts.rs:55-119`) takes `&SeedSourceHandle` and, when it has a local seed, uses `copy_into(store.join(name), ..)` instead of downloading the seed blob (patches still download). `PreviewRequest.seed_source` carries the same handle.
  - Shell: `host/seed_archive.rs` with `NativeHost::seed_archive_command`; `main.rs` `seed_archive_command` and `choose_seed_archive` (file dialog with a `rar` filter, title "Choose your downloaded Stargate Worlds archive"; a cancelled dialog returns the current status). Choosing is refused (`busy`) while a verification is already running; it is allowed while an Install, Repair, Update or adoption preparation runs, which is the point (D-AI4). The webview never passes a path.
  - `contract.ts` mirrors `SeedArchiveCommand`, `SeedArchiveStatus` and the error codes.
- **Tests:**
  - engine unit: a record whose seed sha differs from the current release is not restored (fails if a stale archive is offered for a new seed);
  - engine unit: an Install with a chosen local seed and an unroutable seed URL succeeds (reuse the install worker fixtures);
  - engine unit: adoption `artifacts::fetch` with a local seed requests only the patch blobs from the transport (count requests on the test transport);
  - shell unit: `bridge_has_no_native_observation_or_arbitrary_field_command` gains `seed_archive_command` with an unknown field and with a `path` field, both refused (fails if the renderer could name a file).
- **Checks:** D1, D2, D3, F1 (`npm run check` for the TS mirror).
- **Docs:** `crates/launcher/desktop/README.md` command list; new `crates/launcher/desktop/docs/seed-archive.md` contract (record, identity check, copy-not-move, switch mid-download), linked from the README.
- **Commit:** `feat(launcher-desktop): use a downloaded game archive for Install, Repair, Update and adoption (AI-06)`.

### AI-08: index-based verification and classification

- **Agent:** rust-gameserver-dev, reviewed by `testing-validation-engineer`. **Depends on:** AI-02, AI-03, AI-04. **Branch:** `any-install-adoption/ai-08-index-verify`. **Worktree:** `ai-08`.
- **Do:** in `verification.rs`:
  1. **Get the index.** If `manifest.file_index` is `Some`, fetch the blob through the adoption transport (bounded to `MAX_FILE_INDEX_BYTES`, same TLS and redirect policy as `artifacts::download`), `parse_verified`, `bind`. Store it in the state root as `file-index-<sha256>.json` so a second review does not refetch. If the field is absent, the setup policy is unknown, or `bind` fails: log a warning naming the reason and fall back to fetching the seed and patches (`artifacts::fetch`) and `file_index::build` locally (one code path for classification either way).
  2. **Expected prepared bytes.** From `histories()`, the release state of every path, then the setup rules: `SGW.exe` takes the `aslr_off` hash; `LoginInternal.lua` takes the hash of `login_servers::render(servers)`; the synthesized ledger `launcher-installed.json` takes the hash of `reference::ledger(release)` written into a temp folder and scanned. `reference_digest` becomes `inventory::content_digest` of that map (same function, same input shape as today).
  3. **Classify** each path against the folder scan (case-blind names, as `comparison.rs:24,48` does; keep its full / working / flat layout mapping for copy mode only):
     - bytes equal the release state or the expected prepared bytes: `Matched` at the same spelling, `KnownTransform` at another spelling;
     - `LoginInternal.lua` with any other bytes: `KnownTransform` (D-AI13; AI-11 backs it up when it differs from both);
     - bytes equal an earlier state of that path: `NeedsPatching`;
     - absent, and the path's first state is a patch output: `NeedsPatching` (the patch adds it);
     - absent otherwise: `Missing`; other bytes: `Modified`;
     - folder files no history names: `Extra`.
  4. Remove `outdated()` and its call (`in_place_preview.rs:17-35,104-108`) and the `Error::OutdatedInstall` mapping chain (engine, shell, TS code list, view text) (D-AI12).
  5. `Verified.download` stays `None` here; AI-09 fills it.
- **Tests (unit):** fixtures built with AI-04's `build` on tiny artifacts:
  - stock-shaped folder (seed state only, no ledger) classifies the patched files `NeedsPatching` and nothing `Modified` (fails if `outdated` or the old comparison returns);
  - a folder with an index published: the transport sees no seed request (fails if the index is ignored);
  - a manifest whose `file_index` hash does not match the blob: falls back and the transport sees the seed request (fails if a bad index is trusted);
  - `SGW.exe` with ASLR still on, `eula.lua` in the wrong case, and a custom `LoginInternal.lua`: all `KnownTransform`;
  - one byte changed in a seed file: `Modified`; a deleted seed file: `Missing`; a deleted patch-added file: `NeedsPatching`.
- **Checks:** D1, D2, D3, F1, F2.
- **Docs:** `crates/launcher/desktop/docs/migration.md`: replace "Known limits: verification needs the signed seed once" (`:303-305`) with the index path and its fallback.
- **Commit:** `feat(launcher-desktop): verify adoption folders against the signed file index, classify needs-patching files (AI-08)`.

### AI-10: folder source

- **Agent:** rust-gameserver-dev, reviewed by `server-authority-enforcer`. **Depends on:** AI-02. **Branch:** `any-install-adoption/ai-10-folder-source`. **Worktree:** `ai-10`.
- **Do:**
  - `source.rs`, `Folder` arm. Resolve `selected` (canonicalized, refused if not canonical-safe as today `mod.rs:213-220`): if it holds `Working\Binaries\SGW.exe` (case-blind), root = selected; if its name is `Working` (case-blind) and it holds `Binaries\SGW.exe`, root = parent; if its name is `Binaries` and its parent is `Working` and it holds `SGW.exe`, root = grandparent. In copy mode a flat folder (`SGW.exe` beside `SGWGame`) is also accepted with root = selected. Anything else: `NotAGameFolder`, before any download.
  - In place only: `FolderNotWritable` unless creating and deleting `<root>\.cimmeria-write-probe-<uuid>` and renaming a probe inside `Working\Binaries` succeed (D-AI10), checked before any download. The same overlap rules as `in_place_preview.rs:74-87` (launcher state root) apply. `clear_unbegun_admission` and `MANAGED` apply unchanged.
  - Legacy precedence (D-AI12): if `legacy_import()` is `Some` and its `game_directory` equals the resolved root, resolve as `Legacy` (today's checks); otherwise `Folder`, with `servers` from `preferences.login_servers` (or `default_servers()` when empty), no lock, no import checks, no telemetry acceptance.
  - `Provenance` for a folder: `folder_scan_digest = Some(scan digest)`, legacy fields `None`. `publication::verify_provenance` (`:573-574`) checks the field that is present against the plan's source.
  - `effective_settings::launch_binding` returns `Ok(None)` binding for a folder provenance (so `injection_policy` follows the preferences and telemetry opt-in as for a fresh install); `launch/preparation.rs:17-21` uses the adoption's server list only for a legacy provenance.
  - Shell: `begin_folder_adoption` and `choose_existing_game_folder` for `mode = in_place` (Windows and Linux test hosts); `adoption_choice_allowed` stops requiring an import (keep the import's `blocker` check for the legacy case only).
  - Keep `choose_adoption_destination` working until AI-13 switches the view.
- **Tests:**
  - engine unit: picking `Working`, `Working\Binaries` and the root of the same fixture all resolve the same root (three cases); a folder without `SGW.exe` is `NotAGameFolder` and the transport saw nothing;
  - engine unit: a folder adoption with **no** legacy files anywhere publishes, and Play's injection policy follows `preferences.client_patches_enabled` (flip it and assert both values; fails if the folder adoption is bound like a legacy one);
  - engine unit: with a saved import naming the same folder the review is `source_kind: legacy`; naming another folder it is `folder`;
  - `#[cfg(windows)]` engine unit: a root whose ACL denies the current user write (set with `icacls` in the test via `std::process::Command`, restored in a drop guard) is `FolderNotWritable` before any request;
  - shell unit (`in_place_tests.rs`): `choose_existing_game_folder` path through the production host with an injected picker result.
- **Checks:** D1, D2, D3, F1, F2.
- **Docs:** `crates/launcher/desktop/docs/migration.md` "In-place adoption on Windows" step 1 (no import, layout picks, writable check).
- **Commit:** `feat(launcher-desktop): adopt any game folder without a legacy import (AI-10)`.

## Wave 3

### AI-07: local seed archive UI

- **Agent:** packet-coder. **Depends on:** AI-06. **Branch:** `any-install-adoption/ai-07-seed-archive-ui`. **Worktree:** `ai-07`.
- **Files:** new `frontend/src/seed-archive-view.ts`, `seed-archive-view.test.ts`, `seed-archive-workflow.ts`, `frontend/seed-archive-uat.mjs`; `frontend/ui/index.html`; `frontend/src/view.ts` (mount in Settings, as `mount<Name>(document, invoke, context)` with the LX-02 `PanelContext`); `frontend/src/install-view.ts` (a **Use a downloaded game archive…** button beside Install, and visible while the seed download progress shows); `package.json` (`test` list and `uat:seed-archive`).
- **Do:** follow `game-telemetry-view.ts` and `adoption-workflow.ts` (Effect, strict decode, one mutation at a time, a lost reply reported and never retried). Texts:
  - idle: "Already have the game archive (Stargate Worlds (0.8348.1.4046) (2009-06-30) (beta).rar)? Choose it to skip the 4.1 GB download.";
  - verifying: "Checking your archive: X of Y MB." with a Cancel button;
  - chosen: "Using <file name> (verified). The launcher copies it when it needs the game files and never changes or deletes your file." with a Forget button;
  - `size_mismatch`: "That file is not the game archive: it is A bytes, the signed release's archive is B bytes. Nothing was changed.";
  - `hash_mismatch`: "That file is the right size but its contents do not match the signed release, so it was not used. Download it again, or let the launcher download it. Nothing was changed.";
  - `changed`: "Your archive changed after it was checked. Choose it again.".
  The install-area button shows "Switching to your archive…" on press, then the status line.
- **Tests:** frontend unit: each error code renders its text and leaves no "using" line; the Choose button is disabled while verifying and shows busy on the first press. UAT script (mocked bridge, the LX-03 pattern): choose → verifying with progress → chosen; choose a bad file → `hash_mismatch` text; forget → idle.
- **Checks:** F1, `npm run uat:seed-archive --prefix crates/launcher/desktop/frontend`.
- **Commit:** `feat(launcher-desktop): Use a downloaded game archive in Settings and the install area (AI-07)`.

### AI-09: targeted replay and the verified replacement store

- **Agent:** rust-gameserver-dev. **Depends on:** AI-05, AI-08. **Branch:** `any-install-adoption/ai-09-replay`. **Worktree:** `ai-09`.
- **Do:** new `adoption/replay.rs`, called from `verification.rs` after classification when anything is `NeedsPatching`, `Modified` or `Missing` (or `KnownTransform` at another path in copy mode).
  1. **Plan.** `targets` = those paths. For each, the release state's origin. Walk patches newest to oldest: a target whose release state is patch *k* needs patch *k* applied to a scratch tree where every `inputs` path of patch *k* holds its pre-*k* bytes, and every target written by later patches needs those too. A pre-*k* state is available locally when the folder's file for that path has exactly those bytes; otherwise it comes from an earlier patch (recurse) or the seed. Result: `needs_seed: bool`, the ordered list of patches to apply, and per path its local source file.
  2. **Estimate.** `DownloadEstimate { seed: needs_seed, patches, bytes }` from the manifest sizes, minus artifacts already stored or a chosen local seed (then `seed` is `false` and the review says the player's archive is used).
  3. **Fetch** only those blobs through `artifacts::fetch_some(..)` (new, the same store and verification as `fetch`, given a subset), the seed through the `SeedSourceHandle` (D-AI4). Never from any other host than the signed `blob` URL (D-AI5).
  4. **Replay** into `<preparation dir>/scratch/`: copy the local source files in (hash-checked with `inventory::copy`), extract the seed when needed (`unpack::unpack`, or the Wine extractor on macOS as `reference.rs:33-51` does) and keep only the paths the replay needs, apply the planned patches in signed order with `unpack::unpack` into `patch_dest`, run nothing from `client_setup` (setup runs at publication). Then copy each target's release-state bytes into `<preparation dir>/replacements/<path>` with `inventory::copy` against the history's hash. A mismatch is `InvalidArtifact`.
  5. Fill `Verified.download` and `Verified.replacements` (one `Replacement` per target, `reason` its classification).
  The full-reference reconstruction (`reference::reconstruct`) is no longer called when an index is available. Keep the `Reference` type for the fallback and the parity test.
- **Tests (unit):**
  - stock fixture (seed state only) with an index: the transport is asked for exactly the patch blobs and **not** the seed; the replacements equal the release bytes (fails if replay downloads the seed for needs-patching files);
  - a modified file whose release state is an overlay patch output: only that patch is fetched;
  - a modified file whose release state is a patch-set output, with its input still at seed bytes in the folder: no seed fetched; with its input also modified: the seed is fetched (`needs_seed` true) and the estimate says so;
  - a modified seed file with a chosen local seed: no seed request, the local fixture archive is copied (assert its bytes are unchanged afterwards);
  - estimate bytes equal the sum of the fetched blob sizes.
- **Checks:** D1, D2, D3.
- **Docs:** `crates/launcher/desktop/docs/migration.md` "Verification boundaries": what is fetched and from where (D-AI5 wording).
- **Commit:** `feat(launcher-desktop): replay only the patches and seed files an adopted folder needs (AI-09)`.

## Wave 4

### AI-11: in-place publication with replacements and backups

- **Agent:** rust-gameserver-dev, reviewed by `server-authority-enforcer` and `testing-validation-engineer`. **Depends on:** AI-09, AI-10. **Branch:** `any-install-adoption/ai-11-in-place-replace`. **Worktree:** `ai-11`.
- **Do:** `in_place.rs` and `model.rs`.
  - `Report::blocking()` no longer lists `Modified`, `Missing` or `NeedsPatching` when the review has a replacement for them; a `KnownTransform` at another spelling moves from `source_path` to `path` instead of blocking. Consent: `replace_modified_files` for `Modified`/`Missing` (D-AI1), nothing extra for `NeedsPatching` (D-AI7).
  - `Report.backup = Some(<root>/cimmeria-adoption-backup-<work id>)` when any file is `Modified`, or `LoginInternal.lua` differs from both its release and expected prepared bytes (D-AI13). The work id is the one admission uses, so the review shows the exact name: let the review mint the work id, or show `cimmeria-adoption-backup-<date>` and fix the name at admission; pick one and say which in the report. `BackupConflict` if it exists at admission.
  - **Admission** (`admit`, `:147-236`): also copy every replacement from the preparation store into `<root>/.cimmeria-adopt-<work id>/replacements/` with `inventory::copy` (the state root may be on another volume, so never rename across), and record `plan.replacements`. The reviewed-hash file (`adoption-reviewed-<work>.json`) also names each original to back up.
  - **Moves** (`move_files`, `Way::Adopt`): per release file, by its classification: `Matched`/`KnownTransform` rename into `game` as today; `Modified` (and the D-AI13 `LoginInternal.lua`) rename the original into the backup at the same relative path, then copy its replacement into `game`; `NeedsPatching` rename the original (if present) into `.cimmeria-adopt-<work>/originals/`, then copy the replacement; `Missing` copy the replacement. Each step is idempotent: a `game` file already holding the replacement hash is done, an original already in the backup is not moved again. Then setup and `validate` as today, against the new `reference_digest`.
  - **Publication** (`finish`): after `Published`, remove `.cimmeria-adopt-<work>` (best effort, warn on failure; recovery retries). Never remove the backup.
  - **Abandon** (while `Staged`): move originals back from the backup and from `originals/` to their old paths, remove replacement copies from `game`, then today's steps. A file in both places keeps the folder's copy, as today.
  - Log the backup path at `info` and each backed-up file at `debug`.
- **Tests (unit, engine `in_place_tests.rs` / `in_place_recovery_tests.rs`):**
  - a folder with a modified `SGW.exe` and one needs-patching file adopts with consent; the backup holds the modified original byte-for-byte, `originals/` is gone after publication, `game` matches the reference digest (fails on revert: `verification_failed`);
  - the same without `replace_modified_files`: `consent_required`, nothing moved;
  - crash after the backup move and before the replacement copy (the existing `hook` points; add `Point::Replaced`): recovery finishes; abandon puts the modified original back at its old path;
  - uninstall after adoption leaves the backup folder and its files (fails if the backup becomes a managed entry);
  - `BackupConflict` when the backup folder name already exists at admission.
- **Checks:** D1, D2, D3, F1, F2 (update `uat_bridge_in_place.rs` and `adoption-in-place-uat.mjs` with a modified-file case).
- **Docs:** `crates/launcher/desktop/docs/migration.md` steps 2-5 and the recovery paragraph.
- **Commit:** `feat(launcher-desktop): in-place adoption patches stock files and replaces modified ones, keeping a backup (AI-11)`.

### AI-12: verified copy on Windows

- **Agent:** rust-gameserver-dev. **Depends on:** AI-09, AI-10. **Branch:** `any-install-adoption/ai-12-windows-copy`. **Worktree:** `ai-12`.
- **Do:**
  - Engine: `preview_using` (`mod.rs:142-159`) dispatches on `request.mode`, not the OS: `InPlace` → `in_place_preview::preview` (refused with `UnsupportedMode` on macOS), `Copy` → `copy_preview` on every OS. Lift `#[cfg(target_os = "macos")]` from `copy_preview`, from `mod in_place_preview` (now every OS), and from the copy-only tests that need no Wine (`tests.rs`, `publication_tests.rs`, `recovery_tests.rs`, `preparation_tests.rs`); keep Wine-only tests macOS-gated. `Backend::Native` copies on Windows.
  - `copy_preview` takes its source from `source.rs` (legacy or folder) instead of the import.
  - `publication::copy_and_publish` (`:195-237`): a non-reusable file is copied from `<preparation>/replacements/<path>` (AI-09) instead of `reference.prepared`; keep `reference.prepared` only on the no-index fallback.
  - Shell: `COPY_FOLDER` and `begin_adoption_preview` on every OS; `choose_existing_game_folder` with `mode = copy` opens the second picker; `AdoptionStatus.modes` is `[in_place, copy]` on Windows.
- **Tests:**
  - engine unit (Windows and Linux): a stock fixture copies into a new folder, the original folder's scan is unchanged afterwards, the copy matches the reference digest, and the transport saw no seed request with an index (fails if the copy still reads `reference.prepared`);
  - engine unit: a copy destination inside the source folder is refused (existing rule, now on Windows);
  - shell unit: `modes` lists both on Windows (`#[cfg(windows)]`).
- **Checks:** D1, D2, D3, F1, F2.
- **Docs:** `crates/launcher/desktop/docs/migration.md` "Verified-copy adoption in Settings": every OS.
- **Commit:** `feat(launcher-desktop): verified-copy adoption on Windows (AI-12)`.

## Wave 5

### AI-13: Settings UI for any-install adoption

- **Agent:** packet-coder. **Depends on:** AI-07, AI-11, AI-12. **Branch:** `any-install-adoption/ai-13-adoption-ui`. **Worktree:** `ai-13`.
- **Files:** `frontend/src/adoption-view.ts`, `adoption-view.test.ts`, `adoption-workflow.ts`, `frontend/ui/index.html`, `frontend/adoption-in-place-uat.mjs`, `frontend/adoption-native-uat.mjs`, `frontend/src/migration-view.ts` (wording only), `shell/main.rs` (remove `choose_adoption_destination` and its `generate_handler!` line).
- **Do:**
  - The section is titled **Use an existing game folder**. With nothing owned and no operation running, it shows a mode choice when `modes` has two entries ("Adopt it in place: the folder becomes the launcher's installation" / "Make a verified copy: your folder is left as it is"), then a **Use an existing game folder…** button that calls `choose_existing_game_folder` with the mode. No import is needed; the legacy import panel stays above, headed "Optional: bring settings from the old launcher".
  - Review lines: counts with `needs_patching` ("N files will be updated to the current release"); when `requires_replacement`, an **unchecked** box "Replace N modified or missing release files with the signed release's versions. Your originals are moved to <backup_directory>." (copy mode: "…The copy gets the signed versions; your folder is not changed."); the download line ("Downloads: the game archive (4.1 GB) and patches A, B (X MB)" / "Downloads: patches A, B (X MB)" / "Nothing to download" / "Uses your game archive"), with a link-styled button to the seed-archive panel when the seed is needed; for a folder source, "Settings: this launcher's own server list and client-patch setting" instead of imported ones.
  - `consented` requires the replacement box when `requires_replacement`.
  - New failure texts: `folder_not_writable` ("The launcher cannot write to this folder (for example under Program Files), so it cannot adopt it in place. Make a verified copy instead. Nothing was changed."), `not_a_game_folder`, `backup_conflict`, `unsupported_mode`. Remove the `import_required`, `outdated_install` and "Import legacy launcher settings above first" paths.
  - Completed line names the backup folder when there is one.
- **Tests:** frontend unit: the replacement box starts unchecked and Confirm stays disabled until it is ticked (fails if consent defaults on); a folder-source review renders no imported settings; each new code renders its text. Update the in-place and native UAT scripts to drive `choose_existing_game_folder` and a modified-file review.
- **Checks:** F1, F2; D2, D3 for `cimmeria-launcher-desktop` (the `main.rs` line).
- **Commit:** `feat(launcher-desktop): Use an existing game folder in Settings (AI-13)`.

### AI-14: publish the file index for `content-current`

- **Agent:** owner (the offline manifest key holder). **Depends on:** AI-04; D-AI8 answered.
- **Do:** on the machine holding the archive.org seed and the current patch zips: `build-file-index --manifest manifest.json --seed <archive.org RAR> --patch 001-…=… --out file-index.json`; upload `file-index.json` (paths, sizes and hashes only; no seed content, D-AI5) to the release that holds the current patches; paste the printed `file_index` entry with its `blob` URL into `manifest.json`; sign with `cimmeria-patchset sign`; publish to `content-current`. Verify with `cargo run --example catalog_probe` (desktop README, through the lane) that the release loads, and with an adoption preview that no seed request is made.
- **Rule for later releases:** every manifest change that adds or replaces a patch, or changes the seed, rebuilds the index in the same session; `pack-client-overlay` drops a stale one (AI-04), and a stale index that slips through only costs the launcher a fallback download (AI-08), never a wrong verdict.
- **Docs:** `docs/client/launcher-distribution-setup.md` "File index" and `docs/client/launcher-guide.md` manual publish steps (AI-15 writes them; the owner confirms them by doing it).

### AI-15: developer, operator and player docs

- **Agent:** documentation-writer. **Depends on:** AI-07, AI-11, AI-12, AI-13.
- **Do:** `crates/launcher/desktop/docs/migration.md` (re-read whole after AI-08..AI-13; one coherent "Adopting an existing folder" section replacing "In-place adoption on Windows"), `crates/launcher/desktop/docs/seed-archive.md` (from AI-06), `crates/launcher/desktop/README.md`, player `docs/client/launcher-guide.md` (Use an existing game folder, the two modes, the backup folder, Use a downloaded game archive with the exact archive.org file name and size), `docs/client/launcher-distribution-setup.md` (File index publishing), `docs/analysis/launcher-consolidation/acceptance.md` (adoption gates now owned here). Doc-update map row: "Desktop launcher workflows, IPC, packaging or release". List the rows in the PR body.
- **Checks:** `pwsh tools/lint-md.ps1` on the changed files; CRLF.
- **Commit:** `docs(launcher): any-install adoption and local game archives (AI-15)`.

### AI-16: offer a matching archive found in Downloads

- **Agent:** packet-coder. **Depends on:** AI-07.
- **Do:** on Settings mount and when Install would download the seed, the engine looks in the player's Downloads known folder (`SHGetKnownFolderPath(FOLDERID_Downloads)`; `~/Downloads` on macOS), top level only, for a regular file with the seed's exact size whose name is the archive.org file name. If one is found, the panel offers "Found <name> in Downloads. Use it?"; Yes runs the normal verification (size and SHA-256). Nothing is read before the player says yes.
- **Tests:** engine unit with a temp known-folder override: an exact-size, exact-name file is offered; a wrong size is not. Frontend unit and UAT.

## Wave 6

### AI-17: Windows UAT (owner-run)

- **Agent:** owner. **Depends on:** AI-13, AI-15; AI-14 for rows 2 and 3.
- Run on Windows from the released zip, with a clean launcher state folder each row. Record the result, the launcher log lines (`target: adoption`, `seed_archive`) and download sizes in `docs/analysis/any-install-adoption/uat.md`.

| Row | Setup | Expect |
|---|---|---|
| 1 | Run the 2009 installer (`SetupQA.exe` from the seed) into a writable folder. Adopt in place. | Review lists needs-patching files only, or names what the installer wrote differently (record the list: README "facts that bend"); adoption succeeds; Play reaches login. |
| 2 | Same install, index published (AI-14). | Download line shows patches only; no seed request in the log. |
| 3 | Hand-extract the archive.org RAR (7-Zip or WinRAR, then the CABs). Adopt in place, picking `Working\Binaries`. | Root resolved to the client root; needs patching only; succeeds. |
| 4 | Row 3 with a hex-edited `SGW.exe` and one deleted PAK. | Replacement box unchecked, Confirm disabled until ticked; backup folder holds the edited `SGW.exe`; a missing PAK means the seed is needed (archive or download); Play works. |
| 5 | Install under `C:\Program Files (x86)\…` with the 2009 installer. Adopt in place. | `folder_not_writable` before any download; then a verified copy succeeds and the original is untouched. |
| 6 | An egui-managed folder with its old launcher files. Import, then adopt. | Legacy source; reviewed servers and patch toggle kept, as LX-10. |
| 7 | Settings: Use a downloaded game archive with the real RAR. Then a fresh Install. | Verification progress, then "using"; Install makes no seed request; the RAR is unchanged (size and hash after). |
| 8 | Start a fresh Install, then choose the RAR while the seed downloads. | Download stops, partial file removed, install completes from the archive. |
| 9 | Choose a truncated copy and a same-size corrupted copy of the RAR. | `size_mismatch`, then `hash_mismatch`; both files unchanged. |
| 10 | After row 4, Uninstall. | `game` gone, backup folder and other files kept. |

- Interrupt rows: kill the launcher mid-move in row 4; Recover finishes; repeat and Abandon puts the edited `SGW.exe` back.

### AI-18: close-out

- **Agent:** coordinator. **Depends on:** AI-17.
- **Do:** packet table to Done; `docs/gap-analysis.md` (launcher area) and `docs/project-status.md` once; `docs/guides/unified-uat.md` gets any rows still owed; retire every worker worktree (`pwsh tools/build-lane/rm-worktree.ps1 --merged`); board handoff in the campaign subcategory; agent memory for anything a later launcher campaign needs (the index format, the copy-not-read rule for local archives).
- **Commit:** `docs(launcher): close out any-install adoption (AI-18)`.
