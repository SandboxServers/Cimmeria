# launcher-win-arm64 work packets

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-24
> **Companions:** [Ledger](README.md), [resume card](RESUME.md)

Six packets in one wave, plus a close-out. The design, findings and decisions (D-WA1..D-WA11) are in the [ledger](README.md). Line numbers are from `main` `2019922fe`; re-grep every anchor before editing.

## Dispatch rules

- **Wave 1 ships as one PR** from an integration worktree. Ask `ops` for `pwsh tools/build-lane/mk-worktree.ps1 launcher-win-arm64/integration lwa-integration` at main, and for one worktree per packet: `pwsh tools/build-lane/mk-worktree.ps1 launcher-win-arm64/WA-0N-<slug> lwa-0N`. The worktrees are `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\lwa-0N`.
- Dispatch WA-01, WA-03, WA-04, WA-05 and WA-06 at once, in the background. **WA-02 waits** until WA-01's packet review returns `ship`; then `ops` creates `lwa-02` with its branch at WA-01's reviewed head SHA.
- Agents: WA-01..WA-05 go to `packet-coder`, WA-06 and WA-07 to `documentation-writer`. Each packet gets a packet-review loop, then the wave loop runs on `lwa-integration`.
- Merge order into the integration branch (`ops`, `merge-noff.ps1`): **WA-01, WA-02, WA-03, WA-04, WA-05, WA-06**. WA-02 and WA-03 both edit `tools/launcher-release/test-release-scripts.sh`, in hunks about 45 lines apart (WA-03 at l.94-177, WA-02 after l.224), so git merges them cleanly. No other file is touched by two packets.
- **Campaign merge gate.** The dry-run jobs don't gate the merge queue (D-WA7), so the orchestrator itself holds the wave PR until these checks are green: `desktop release build dry run (Windows arm64)`, `desktop release build dry run (updater feed)`, `desktop release build dry run (Windows)` and `release scripts (shellcheck and stubbed tests)`. That run is the first real arm64 compile (F9). If it fails, dispatch a fix packet-coder in `lwa-integration` with the failed job's log excerpt. A toolchain gap on the runner (clang, Python, rustup) is fixed in `launcher-desktop-windows-arm64.yml` only.
- Report path: `<worktree>\.claude\reports\launcher-win-arm64\<packet>-<role>.md`. Return line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- Commit attribution lines, at the end of every commit message:

```text
Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01WRynanEMGjFbBzPn7s7aWe
```

## Rules every packet carries

- **R1 Shell.** `Host: windows`: PowerShell only, never the Bash tool. Start every command with `Set-Location <your worktree>;`, and use absolute paths for every file write. The `.sh` and `.yml` files are edited, not run: CI runs them.
- **R2 Lane.** Every compiling cargo call goes through `pwsh tools/build-lane/lane.ps1 cargo ...`, in the foreground. Never call cargo directly, and never pass `--target-dir`. A lane failure: read its log under `logs/` before rerunning; at most 5 fix rounds per check.
- **R3 Scope.** Edit only the files your packet names. No `git stash`, no `git worktree prune`, no rebase, no push. Once every check passes, commit at once: `git add <explicit paths>`, then commit with the packet's subject and the attribution lines.
- **R4 File caps.** Soft cap 500 lines, hard cap 700 per file. Stop and report `blocked` rather than pass 700.
- **R5 Line endings.** Keep each file's line endings. `docs/**/*.md`, `crates/launcher/desktop/docs/*.md`, `tools/**/README.md` and the root `README.md` are CRLF. `*.sh` and `.github/workflows/*.yml` are LF (`.gitattributes`). After editing a CRLF doc, check `([IO.File]::ReadAllText('<abs path>') -match "[^\r]\n")` is `False`.
- **R6 Tests.** Every behaviour change has a test that fails when the change is reverted. Say in the report which test and why. Rust tests are unit tests in the module's `#[cfg(test)] mod tests` (TESTING.md); temp dirs are canonicalized (`tempfile::tempdir()?.path().canonicalize()`), because the engine tests also run on macOS, where `/var` is a link.
- **R7 Names.** Don't rename any x64 name: `StargateWorlds-Launcher-windows-x64.zip`, `windows-x86_64`, the job names `desktop-windows`, `sign` and `publish`, and the artifact names `launcher-zip-<tag>` and `launcher-feed-<tag>`. Old launchers and the bootstrap installer depend on them.
- **R8 Secrets.** No IPs, credentials, account names or real keys in code, tests or reports. Only the `sign` step may see `LAUNCHER_UPDATER_MINISIGN_KEY` (`test_workflow_policy.py` asserts it).
- **R9 Generated blocks.** Never hand-edit a `<!-- gen:NAME -->` block.

### Checks

| ID | Command (prefix every one with `Set-Location <worktree>;`) |
|---|---|
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2e | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --all-targets -- -D warnings` |
| D2s | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-desktop --all-targets -- -D warnings` |
| D3e `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine <f>` (cargo test, not nextest: the desktop workspace's `harness = false` tests can't be listed) |
| D3s `<f>` | the same with `-p cimmeria-launcher-desktop` |
| F0 | before D2s or D3s, if the shell's build asks for frontend assets: `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend; npm run build --prefix crates/launcher/desktop/frontend` |
| P1 | `python -m unittest discover -s tools/launcher-release -p "test_*.py"` (19 tests at baseline; needs PyYAML) |

The bash tests (`test-release-scripts.sh`) and shellcheck run only in CI, in the `release-scripts` job. Read your shell edits twice for quoting, `set -u` safety and shellcheck warnings (SC2086 unquoted variables, SC2155 `local x="$(...)"`).

## Contract

The engine API WA-01 adds and WA-02 calls. It lives in `crates/launcher/desktop/engine/src/storage/updater/release_check.rs`, and is re-exported from `updater/mod.rs`.

```rust
/// The feed's platform key for each Windows zip (D-WA2).
pub const WINDOWS_X86_64_PLATFORM: &str = "windows-x86_64";
pub const WINDOWS_AARCH64_PLATFORM: &str = "windows-aarch64";
/// Every Windows key a release feed carries, x64 first.
pub const WINDOWS_PLATFORMS: [&str; 2] = [WINDOWS_X86_64_PLATFORM, WINDOWS_AARCH64_PLATFORM];
/// This build's own key: the compile target picks it (D-WA3).
#[cfg(target_arch = "aarch64")]
pub const WINDOWS_PLATFORM: &str = WINDOWS_AARCH64_PLATFORM;
#[cfg(not(target_arch = "aarch64"))]
pub const WINDOWS_PLATFORM: &str = WINDOWS_X86_64_PLATFORM;

/// One zip's entry in the feed. `signature` is the Minisign signature file's text.
pub struct FeedEntry<'a> { pub platform: &'a str, pub signature: &'a str, pub url: &'a str }

/// The Tauri v2 static feed: `{version, notes, pub_date, platforms: {<platform>: {signature, url}}}`,
/// each signature stored as STANDARD base64 of the signature file's text.
/// Err when `entries` is empty, a platform is not in WINDOWS_PLATFORMS, or a platform repeats.
pub fn feed_document(version: &str, notes: &str, pub_date: &str, entries: &[FeedEntry<'_>])
    -> Result<serde_json::Value, String>;

/// One zip a release is about to publish. `name` is its file name; `signature_file` the `.zip.sig` bytes.
pub struct WindowsPackage<'a> { pub platform: &'a str, pub name: &'a str, pub zip: &'a [u8], pub signature_file: &'a [u8] }

/// What every Windows launcher does with the release, per platform. `scratch` is an existing,
/// canonical directory; the stages are made under it. The error text names the platform.
pub fn check_windows_feed(public_key: &str, feed: &[u8], version: &str,
    packages: &[WindowsPackage<'_>], scratch: &Path) -> Result<(), String>;
```

`check_windows_feed` does this, in order:

1. Refuse empty `packages`, a repeated platform, or an empty zip.
2. Parse `feed` as JSON. The key set of `platforms` must equal the set of package platforms; on a mismatch, name both sets.
3. For each package `i`:
   - `release_config(public_key.trim(), "0.0.0", platform)`;
   - `verify_windows_release(&config, feed, zip, &scratch.join(format!("stage-{i}")))` must return `version`;
   - the last `/` segment of `platforms[platform].url` must equal `name`;
   - `STANDARD.encode(signature_file)` must equal `platforms[platform].signature`;
   - a copy with its last byte XOR `0x01`, checked into `scratch.join(format!("tampered-{i}"))`, must give `Err(Error::Signature)`.

The `release_feed` CLI (WA-02), which `build-desktop.sh` calls:

```text
release_feed keygen <dir>
release_feed sign <version> <notes> <out-dir> (<platform> <zip> <url>)+
release_feed verify <latest.json> <public-key> <version> (<platform> <zip> <zip.sig>)+
```

The `build-desktop.sh` stages (WA-02), which the workflows call (WA-04, WA-05):

```text
WINDOWS_ARCH=x64 (default) | arm64      picks ZIP and the PE machine verify requires
sign <version> <release-download-base-url> <notes>   signs every zip in FEED_ARCHES (x64 arm64)
verify-feed <version>                                 checks all of them
feed-dry-run <version>                                both, with a throwaway key
```

The arm64 names (D-WA2): feed key `windows-aarch64`, zip `StargateWorlds-Launcher-windows-arm64.zip`, PE machine `0xaa64`. The release artifacts: `launcher-zip-arm64-<tag>`, holding the arm64 `.zip` and `.zip.sha256`; `launcher-feed-<tag>`, now holding both `.zip.sig` files and `latest.json`. The dry-run artifacts: `desktop-launcher-windows-arm64-dry-run` and `desktop-launcher-feed-dry-run`.

## Wave 1

### WA-01 Engine: two Windows platforms, feed builder and checker

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-win-arm64/WA-01-engine-platforms` · **Worktree:** `lwa-01`
- **Commit:** `feat(launcher): Windows arm64 updater platform and two-zip feed checks (WA-01)`
- **Rules:** R1-R9.

Files and changes:

1. `crates/launcher/desktop/engine/src/storage/updater/release_check.rs`:
   - Replace the `WINDOWS_PLATFORM` const (l.18-19) with the five constants from the Contract, with their doc comments.
   - Add `FeedEntry`, `feed_document`, `WindowsPackage` and `check_windows_feed` after `verify_windows_release`.
   - Add `use base64::{engine::general_purpose::STANDARD, Engine};` at the top. `base64`, `serde_json` and `tempfile` are normal dependencies already. Remove the test module's own base64 `use` if clippy flags it as redundant.
   - In the tests, change `WINDOWS_PLATFORM` (l.68, l.74) to `WINDOWS_X86_64_PLATFORM`.
   - If the file passes 500 lines, move the whole test module to a sibling `release_check_tests.rs` (`#[cfg(test)] #[path = "release_check_tests.rs"] mod tests;`).
2. `updater/mod.rs` l.13-15: add `check_windows_feed, feed_document, FeedEntry, WindowsPackage, WINDOWS_AARCH64_PLATFORM, WINDOWS_PLATFORMS, WINDOWS_X86_64_PLATFORM` to the `pub use release_check::{...}`.
3. `updater/policy.rs` l.39-42: the allow-list becomes `"darwin-aarch64" | "darwin-x86_64" | "windows-x86_64" | "windows-aarch64"`.
4. `updater/apply.rs` l.197: `if !WINDOWS_PLATFORMS.contains(&config.platform.as_str()) {` (import it through `super::release_check::WINDOWS_PLATFORMS` or the `mod.rs` re-export).
5. `updater/apply/tests/windows_zip.rs`:
   - Rename `fn signed(bytes)` (l.16) to `fn signed_for(bytes: &[u8], platform: &str)`, passing `platform` where l.30 has `"windows-x86_64"`, and add `fn signed(bytes: &[u8]) -> (Config, Offer) { signed_for(bytes, "windows-x86_64") }`.
   - Add the test below.
6. `crates/launcher/desktop/shell/src/host/updater/mod.rs`:
   - The doc comment at l.53-54 says: "Windows only: the feed carries a `windows-x86_64` and a `windows-aarch64` zip, and the compile target picks this build's key (`updater::WINDOWS_PLATFORM`). macOS stays Disabled."
   - After l.303 add `assert!(updater_config_from(Some(&key), Some("windows-aarch64")).is_some());`.
   - l.59 stays as it is.

The `windows_zip.rs` test:

```rust
#[test]
fn an_arm64_launcher_applies_its_own_windows_zip() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut f = windows_fixture(&root);
    let (config, offer) = signed_for(&f.bytes, "windows-aarch64");
    f.config = config;
    f.offer = offer;
    let mut state = DesktopState::open(&root.join("state")).unwrap();
    let ready = ready(&mut state, &f.config, f.offer.clone(), &f.bytes);
    let status = apply(&mut state, &f, &ready, |_, _| Ok(())).unwrap();
    assert_eq!(status.phase, Phase::RestartRequired);
}
```

The tests to add in `release_check.rs` are unit tests. Write a helper `two_platform_release(version) -> (String /*key*/, Vec<u8> /*feed*/, [(&'static str, &'static str, Vec<u8>, String); 2])`:

- One unencrypted key pair.
- Two distinct zips: `zip_fixture(&[("cimmeria-launcher-desktop.exe", b"x64 exe")])` and the same with `b"arm64 exe"`.
- Each signed with the trusted comment `timestamp:1\tfile:{name}\tversion:{version}`.
- URLs `https://github.com/SandboxServers/Cimmeria/releases/download/launcher-20261024-abcdef1/{name}`.
- The feed built with `feed_document(version, "n", "2026-10-24T00:00:00Z", &entries)`.

The tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `each_windows_platform_is_offered_its_own_zip` | For both platforms: `release_config(&key, "2026.10.10", platform)` then `check(..)` gives `Ok(version)`. The x64 config with the arm64 zip gives `Err(Error::Signature)`. | Without the aarch64 policy entry, `release_config` is `Err(Policy)`. |
| `a_two_zip_release_passes_the_feed_check` | `check_windows_feed` is `Ok(())` for both packages, with `.sig` bytes = the signature text. | It is new API; the revert removes it. |
| `the_feed_check_refuses_a_mismatched_release` | `Err` for each of: only the x64 package (an extra feed key); a feed built from the x64 entry alone checked against both packages (a missing key); the two `signature_file`s swapped; a package `name` that differs from its URL's last segment. Each `Err` text contains the platform it names. | Each refusal is a branch of the new checker. |
| `the_feed_builder_refuses_unknown_repeated_or_no_platforms` | `feed_document` is `Err` for `[]`, for platform `"windows-arm64"`, and for the x64 entry twice. | New API. |
| `an_unknown_windows_key_is_refused_as_policy` | `release_config(&key, "1.0.0", "windows-arm64")` gives `Err(Error::Policy)`. | It guards the allow-list spelling. |
| `the_build_target_picks_the_windows_key` | `assert_eq!(WINDOWS_PLATFORM == WINDOWS_AARCH64_PLATFORM, cfg!(target_arch = "aarch64"))`. | A revert to the single constant fails on arm64 CI. |

Checks, in order:

1. D1.
2. D2e.
3. `D3e updater::`.
4. F0, then D2s.
5. `D3s updater`.

The report lists the test counts. Docs: none; WA-06 owns them.

### WA-02 `release_feed` CLI, example tests and `build-desktop.sh` arch rows

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-win-arm64/WA-02-feed-cli` · **Worktree:** `lwa-02`, branched at WA-01's reviewed head
- **Commit:** `feat(launcher-release): sign and verify both Windows zips in one feed (WA-02)`
- **Rules:** R1-R9. The Contract above is the API you call; it already exists in your worktree.

Files and changes:

1. `crates/launcher/desktop/engine/examples/release_feed.rs`:
   - Update the module doc's usage block (l.7-11) to the Contract's CLI.
   - The match in `main`: `["keygen", dir]`, then `["sign", version, notes, out, rest @ ..] => sign(version, notes, Path::new(out), &triples(rest)?)`, then `["verify", feed, public_key, version, rest @ ..] => verify(Path::new(feed), public_key, version, &triples(rest)?)`. Anything else is `Err` with the new usage line.
   - Add `fn triples<'a>(rest: &[&'a str]) -> Result<Vec<[&'a str; 3]>>`: `Err` when `rest` is empty or `rest.len() % 3 != 0`, else `rest.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()`.
   - `sign` now reads:
     - `stable_release` first, then the secret key exactly as today, then one `timestamp`.
     - For each `[platform, zip, url]`: the file name, the trusted comment `timestamp:{ts}\tfile:{name}\tversion:{version}`, and `minisign::sign(...)`, collecting `(platform, name, url, signature_text)`. Refuse two zips with the same file name.
     - Build `Vec<updater::FeedEntry>`, then `updater::feed_document(version, notes, &pub_date, &entries)?`. This validates before anything is written.
     - Then `create_dir_all(out)`, each `{name}.sig`, `latest.json` (`to_vec_pretty`), and one `println!` per zip.
   - `verify` now reads:
     - Read the feed. For each `[platform, zip, sig]`, read the zip and sig bytes and take the zip's file name.
     - `let scratch = tempfile::tempdir()?; let root = scratch.path().canonicalize()?;`.
     - Build `Vec<updater::WindowsPackage>` from the owned data, then `updater::check_windows_feed(public_key, &feed, version, &packages, &root)?`.
     - Print `verified: every Windows launcher (<platforms joined by ", ">) is offered <version> and accepts its zip`.
   - Remove the now-unused imports.
   - Tests: keep the existing one, and add `triples_needs_whole_groups`: `[]` and two arguments are `Err`, six arguments give two groups in order.
2. `crates/launcher/desktop/engine/Cargo.toml`: append the block below (D-WA9; F5: without it `cargo test` never runs the example's tests). Other examples stay auto-discovered.

```toml
[[example]]
name = "release_feed"
test = true
```

3. `tools/launcher-release/build-desktop.sh` (LF):
   - Replace l.90-95 with the arch rows below, keeping `ZIP` and `FEED`.
   - `pe_machine`'s comment adds `0xaa64 = Arm64`.
   - `stage_verify` l.215: `[ "$(pe_machine "$LAUNCHER")" = "$LAUNCHER_MACHINE" ] || die "$LAUNCHER is not a Windows $WINDOWS_ARCH executable (PE machine $LAUNCHER_MACHINE)"`.
   - l.274's echo: `verified: self-contained Windows $WINDOWS_ARCH launcher with pinned 32-bit worker and DLLs`.
   - Keep `--verify-embedded-payload`, `corrupt cache fixture` and the `stage_package` `<<'PY'` block byte-identical: `test_workflow_policy.py` reads them.
   - Replace `stage_sign`, `stage_verify_feed` and the URL in `stage_feed_dry_run` with the shapes below.
   - `paths` (l.463) also prints `"WINDOWS_ARCH=$WINDOWS_ARCH" "LAUNCHER_MACHINE=$LAUNCHER_MACHINE"`.
   - In the header:
     - `launcher`: "the launcher for WINDOWS_ARCH";
     - `package`: `artifacts/StargateWorlds-Launcher-windows-<x64|arm64>.zip`;
     - `sign <version> <base-url> <notes>`: "sign every Windows zip (FEED_ARCHES) ... feed offering each from <base-url>/<zip name>";
     - `verify-feed`: "all of them";
     - a new line: "WINDOWS_ARCH=x64 (default) or arm64 picks the zip name and the PE machine verify requires".
   - Fix the dispatcher's `sed -n '2,60p'` to the header's new last line.

The arch rows (they replace l.90-95):

```bash
# The release names (D-WA2). Players and docs link to each zip by this exact
# name on launcher-current; the updater offers whatever latest.json names.
WINDOWS_ARCH="${WINDOWS_ARCH:-x64}"
# <feed platform> <zip name> <PE machine of the launcher>
arch_row() {
  case "$1" in
    x64) echo "windows-x86_64 StargateWorlds-Launcher-windows-x64.zip 0x8664" ;;
    arm64) echo "windows-aarch64 StargateWorlds-Launcher-windows-arm64.zip 0xaa64" ;;
    *) echo "WINDOWS_ARCH must be x64 or arm64, got $1" >&2; return 2 ;;
  esac
}
ROW="$(arch_row "$WINDOWS_ARCH")" || exit 2
read -r _ ZIP_NAME LAUNCHER_MACHINE <<<"$ROW"
# Every zip one release feed signs and offers (D-WA4).
FEED_ARCHES=(x64 arm64)
ZIP="artifacts/$ZIP_NAME"
FEED="artifacts/latest.json"
```

The new `stage_sign` and `stage_verify_feed`:

```bash
stage_sign() {
  local version="${1:?sign needs the version}" base="${2:?sign needs the release download base URL}" notes="${3:-}"
  local arch platform name args=()
  for arch in "${FEED_ARCHES[@]}"; do
    read -r platform name _ <<<"$(arch_row "$arch")"
    need "artifacts/$name"
    args+=("$platform" "$(winpath "artifacts/$name")" "${base%/}/$name")
  done
  [ -n "${LAUNCHER_UPDATER_MINISIGN_KEY:-}" ] || die "sign needs LAUNCHER_UPDATER_MINISIGN_KEY"
  release_feed sign "$version" "$notes" "$(winpath artifacts)" "${args[@]}"
  for arch in "${FEED_ARCHES[@]}"; do
    read -r platform name _ <<<"$(arch_row "$arch")"
    need "artifacts/$name.sig"
  done
  need "$FEED"
}

stage_verify_feed() {
  local version="${1:?verify-feed needs the version}"
  [ -n "${LAUNCHER_UPDATER_PUBKEY:-}" ] || die "verify-feed needs LAUNCHER_UPDATER_PUBKEY"
  need "$FEED"
  local arch platform name args=()
  for arch in "${FEED_ARCHES[@]}"; do
    read -r platform name _ <<<"$(arch_row "$arch")"
    need "artifacts/$name"; need "artifacts/$name.sig"
    args+=("$platform" "$(winpath "artifacts/$name")" "$(winpath "artifacts/$name.sig")")
  done
  release_feed verify "$(winpath "$FEED")" "$LAUNCHER_UPDATER_PUBKEY" "$version" "${args[@]}"
}
```

In `stage_feed_dry_run`, the `stage_sign` call passes the base `https://github.com/SandboxServers/Cimmeria/releases/download/launcher-dry-run`.

4. `tools/launcher-release/test-release-scripts.sh` (LF): add **one block only**, between the `done` of the release-tools loop (l.224) and `exit "$FAILED"`. Leave l.94-177 alone; WA-03 owns them. The block:
   - Run `mkdir -p "$WORK/feed/artifacts" && cd "$WORK/feed"`, then `"$HERE/build-desktop.sh" feed-tool >/dev/null`. The stub cargo builds the stub tool, which logs its arguments one per line to `$STUB_TOOL_LOG`.
   - Create a fixture for both zip names: `artifacts/<name>`, `artifacts/<name>.sig` (the stub writes nothing) and `artifacts/latest.json`.
   - **sign**: `: > "$STUB_TOOL_LOG"`, then `LAUNCHER_UPDATER_MINISIGN_KEY=stub "$HERE/build-desktop.sh" sign 2026.10.2401 https://example.invalid/rel "notes"`. Pass when the log has the exact lines `sign`, `windows-x86_64`, `windows-aarch64`, `https://example.invalid/rel/StargateWorlds-Launcher-windows-x64.zip` and `https://example.invalid/rel/StargateWorlds-Launcher-windows-arm64.zip` (`grep -qxF`).
   - **verify-feed**: `: > "$STUB_TOOL_LOG"`, then `LAUNCHER_UPDATER_PUBKEY=stub "$HERE/build-desktop.sh" verify-feed 2026.10.2401`. Pass when the log has `verify` and `$PWD/artifacts/StargateWorlds-Launcher-windows-arm64.zip.sig` (no cygpath on Linux, so `winpath` gives `$PWD/...`).
   - **missing zip**: move the arm64 zip aside, `: > "$STUB_TOOL_LOG"`. Pass when `sign` exits non-zero and the log has no `sign` line. Move the zip back.
   - **arch rows**: pass when `WINDOWS_ARCH=arm64 "$HERE/build-desktop.sh" paths` prints `ZIP=artifacts/StargateWorlds-Launcher-windows-arm64.zip` and `LAUNCHER_MACHINE=0xaa64`, the default prints `ZIP=artifacts/StargateWorlds-Launcher-windows-x64.zip` and `LAUNCHER_MACHINE=0x8664`, and `WINDOWS_ARCH=arm` exits non-zero.
   - Use the file's `pass` and `fail` helpers. Every name must survive `set -u`.

Checks, in order:

1. D1.
2. D2e.
3. `D3e --example release_feed`; the report gives its test count, which must be 2.
4. `D3e updater::`.
5. P1.
6. Read the `.sh` diff for shellcheck issues.

Revert reasoning: without the `[[example]]` block the example tests don't run (count 0). The bash block fails if `sign` still signs only one zip. Docs: none; WA-06 owns them.

### WA-03 `publish.sh`: both Windows zips

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-win-arm64/WA-03-publish` · **Worktree:** `lwa-03`
- **Commit:** `feat(launcher-release): publish the Windows arm64 zip (WA-03)`
- **Rules:** R1-R9. Both files are LF bash; CI runs them (`release-scripts` job), so there is no local check besides P1 and a careful read.

Changes to `tools/launcher-release/publish.sh`:

1. After `MAC_ZIP` (l.32) add `ARM_ZIP_NAME=StargateWorlds-Launcher-windows-arm64.zip` and `ARM_ZIP="artifacts/$ARM_ZIP_NAME"`.
2. In `dated`'s body, add a paragraph right after the `**Windows:**` paragraph (l.108). It must not contain the text `macOS`: the tests grep for it.

   ```text
   **Windows on Arm:** on an Arm PC, download \`${ARM_ZIP_NAME}\` instead. The launcher runs natively; the game itself runs through Windows' built-in x86 emulation. The x64 zip also works on Arm, emulated, and keeps updating to x64. The newest release is always at ${STABLE}/${ARM_ZIP_NAME}.
   ```

3. In `dated`, l.113-115 become:

   ```bash
   local assets=("$ZIP" "$ZIP.sha256" "$ARM_ZIP" "$ARM_ZIP.sha256")
   [[ -s "$ZIP.sig" && -s "$ARM_ZIP.sig" ]] || die "signed updater artifacts are required for both Windows zips"
   assets+=("$ZIP.sig" "$ARM_ZIP.sig")
   ```

4. In `current`:
   - The guard is `[[ ! -s "$ZIP.sig" || ! -s "$ARM_ZIP.sig" || ! -s artifacts/latest.json ]]`, with the message "both signatures and the verified feed are required".
   - The upload loop is `"$ZIP" "$ZIP.sha256" "$ZIP.sig" "$ARM_ZIP" "$ARM_ZIP.sha256" "$ARM_ZIP.sig" artifacts/latest.json`, with the feed still last.
5. In `current_notes`, after the Windows sentence and before macOS, add `if grep -qxF "$ARM_ZIP_NAME" <<<"$assets"; then notes="$notes Windows on Arm: download ${ARM_ZIP_NAME}."; fi`.
6. In the header (l.16-20), `dated` and `current` name both Windows zips and both `.zip.sig` files. Fix `sed -n '2,26p'` if the header's last line moves.

Changes to `tools/launcher-release/test-release-scripts.sh`, **l.94-177 only** (WA-02 adds a block after l.224):

1. l.98: add `StargateWorlds-Launcher-windows-arm64.zip` to the fixture loop.
2. After l.103: `printf 'stub-signature\n' > artifacts/StargateWorlds-Launcher-windows-arm64.zip.sig`.
3. After l.124: pass when `**Windows on Arm:**` appears exactly once in `$A`.
4. Between l.130 and l.131, so after `scenario e dated current` and before `macos-current`, three checks:
   - "dated release carries both Windows zips and signatures": `$STUB_STORE/$TAG/assets` lists the arm64 `.zip`, `.zip.sha256` and `.zip.sig`.
   - "launcher-current: both Windows sets, the feed last": `$STUB_STORE/launcher-current/assets` lists all six Windows files, and `tail -n 1` of it is `latest.json`.
   - "current notes link the arm64 zip": `launcher-current/body` contains `Windows on Arm: download StargateWorlds-Launcher-windows-arm64.zip`.
5. After l.141, a mirror of l.135-141 for the arm64 `.sig`: "missing arm64 signature refuses dated and current publication".

Revert reasoning: each new check fails against today's `publish.sh`, which uploads and names only the x64 set. Checks: P1, then read both diffs for quoting. Docs: none; WA-06 owns them.

### WA-04 Reusable arm64 workflow and the release jobs

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-win-arm64/WA-04-release-ci` · **Worktree:** `lwa-04`
- **Commit:** `ci(launcher-release): build the Windows arm64 launcher natively (WA-04)`
- **Rules:** R1-R9. YAML is LF. Copy the pinned SHAs exactly:
  - checkout `9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0  # v6`
  - upload-artifact `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a  # v7`
  - download-artifact `9000827ccba6bdab643e8b6fd33ac0654aef8333  # v8`
  - rust-cache `e18b497796c12c097a38f9edb9d0641fb99eee32  # v2`
  - setup-node `949feb2413d6458794dcd2491c4babbbce0c15c1  # v7.1.0`

  Add no new third-party action.

**1. The new file `.github/workflows/launcher-desktop-windows-arm64.yml`** is modelled on `launcher-desktop-macos.yml`. It opens with a header comment: what it builds, both callers, D-WA1/D-WA8, "no signing key, no write token", and the pinning policy.

`on: workflow_call` takes these inputs:

| Input | Type | Default |
|---|---|---|
| `pieces_artifact` | string | required |
| `pieces_sha256` | string | required |
| `artifact_name` | string | required |
| `retention_days` | number | 30 |
| `tag`, `version`, `epoch`, `build_branch`, `updater_pubkey` | string | `''` |
| `cache_key` | string | required |
| `save_cache` | boolean | true |

It also declares the secret `LAUNCHER_MANIFEST_PUBKEY_HEX` (`required: false`) and the output `zip_sha256: ${{ jobs.build.outputs.zip_sha256 }}`. Top-level `permissions: {}`.

Job `build`:

- `name: desktop launcher (Windows arm64) -> zip`, `runs-on: windows-11-arm`, `permissions: contents: read`, `timeout-minutes: 75`, `defaults.run.shell: bash`.
- `outputs.zip_sha256: ${{ steps.digest.outputs.sha256 }}`.
- env: `CARGO_TERM_COLOR: always`, `CARGO_INCREMENTAL: 0`, `WINDOWS_ARCH: arm64`, `ZIP: artifacts/StargateWorlds-Launcher-windows-arm64.zip`, `PIECES_SHA256: ${{ inputs.pieces_sha256 }}`.

Its steps, in order:

1. `require the Play pieces digest`: `[[ "${PIECES_SHA256}" =~ ^[0-9a-f]{64}$ ]] || { echo "::error::no Play pieces digest from the x64 job"; exit 1; }`.
2. The pinned checkout, with `ref: ${{ github.sha }}` and `persist-credentials: false`.
3. `make sure python, clang and rustup are on the runner` (D-WA8):

   ```bash
   set -euo pipefail
   python --version || { echo "::error::the windows-11-arm image has no python"; exit 1; }
   if ! command -v clang >/dev/null; then
     # ring and aws-lc-sys compile C for aarch64-pc-windows-msvc with clang.
     choco install llvm -y --no-progress
     echo 'C:\Program Files\LLVM\bin' >> "$GITHUB_PATH"
   fi
   if ! command -v rustup >/dev/null; then
     curl -sSfL -o "$RUNNER_TEMP/rustup-init.exe" https://static.rust-lang.org/rustup/dist/aarch64-pc-windows-msvc/rustup-init.exe
     "$RUNNER_TEMP/rustup-init.exe" -y --profile minimal --default-toolchain none --default-host aarch64-pc-windows-msvc
     echo "$USERPROFILE\\.cargo\\bin" >> "$GITHUB_PATH"
   fi
   ```

4. `build for the arm64 host`: `clang --version`, then `rustup set default-host aarch64-pc-windows-msvc`.
5. `- uses: ./.github/actions/rust-toolchain`, with no `targets`: this build compiles no i686 code.
6. `check the toolchain is native arm64`: `HOST="$(rustc -vV | tr -d '\r' | sed -n 's/^host: //p')"; echo "rustc host: $HOST"; [ "$HOST" = aarch64-pc-windows-msvc ] || { echo "::error::rustc host is $HOST, not aarch64-pc-windows-msvc"; exit 1; }`.
7. The pinned rust-cache, with `shared-key: ${{ inputs.cache_key }}`, `save-if: ${{ inputs.save_cache }}` and `workspaces: crates/launcher/desktop -> ../../../target/desktop`.
8. The pinned setup-node, `node-version: '24'`.
9. The pinned download-artifact, `name: ${{ inputs.pieces_artifact }}`, `path: play-pieces`.
10. `take in the Play pieces, checked against the x64 job's digest`: `tools/launcher-release/build-desktop.sh play-pieces-in play-pieces "${PIECES_SHA256}"`.
11. `build the arm64 desktop launcher, pinned to them`:
    - env: `LAUNCHER_MANIFEST_PUBKEY_HEX: ${{ secrets.LAUNCHER_MANIFEST_PUBKEY_HEX }}`, `CIMMERIA_LAUNCHER_VERSION: ${{ inputs.version }}`, `CIMMERIA_LAUNCHER_TAG: ${{ inputs.tag }}`, `CIMMERIA_LAUNCHER_BUILD_EPOCH: ${{ inputs.epoch }}`, `CIMMERIA_BUILD_BRANCH: ${{ inputs.build_branch }}`, `CIMMERIA_BUILD_GIT_SHA: ${{ github.sha }}`, `LAUNCHER_UPDATER_PUBKEY: ${{ inputs.updater_pubkey }}`.
    - run: `set -euo pipefail`, then the unset loop below, then `tools/launcher-release/build-desktop.sh launcher`. Comment the loop: an empty input must reach the build as unset, because the build scripts read some stamps with `.ok()` and an empty tag would count as a release.

    ```bash
    for name in CIMMERIA_LAUNCHER_TAG CIMMERIA_LAUNCHER_VERSION CIMMERIA_LAUNCHER_BUILD_EPOCH CIMMERIA_BUILD_BRANCH LAUNCHER_MANIFEST_PUBKEY_HEX LAUNCHER_UPDATER_PUBKEY; do
      if [ -z "${!name:-}" ]; then unset "$name"; fi
    done
    ```

12. `verify the artifacts`: env `CIMMERIA_LAUNCHER_TAG`, `CIMMERIA_LAUNCHER_VERSION` and `LAUNCHER_UPDATER_PUBKEY` from the inputs, the same unset loop over those three, then `tools/launcher-release/build-desktop.sh verify`.
13. `package`: `tools/launcher-release/build-desktop.sh package`.
14. `record the zip digest` (`id: digest`): the same script as `launcher-release.yml` l.199-203.
15. The pinned upload-artifact:
    - `name: ${{ inputs.artifact_name }}`;
    - `path:` the arm64 `.zip` and `.zip.sha256`;
    - `if-no-files-found: error`, `retention-days: ${{ inputs.retention_days }}`, `overwrite: true`.

**2. `.github/workflows/launcher-release.yml`:**

- **Header** (l.17-43): "Seven jobs". Add a bullet: "`desktop-windows-arm64` builds the Windows on Arm launcher natively on `windows-11-arm` from desktop-windows' Play pieces, through the reusable `launcher-desktop-windows-arm64.yml` the PR dry run also calls". The `sign` bullet says it signs both Windows zips into one feed, and `publish` publishes both. Replace "the feed offers `windows-x86_64` only" with "the feed offers `windows-x86_64` and `windows-aarch64`".
- **Top `env`**: add `ARM_ZIP_NAME: StargateWorlds-Launcher-windows-arm64.zip`.
- **`desktop-windows` Play-pieces steps** (l.258-281): remove both `continue-on-error: true` lines. Change `retention-days: 1` to `7`. Rewrite the comment: the arm64 and mac-pieces jobs both take these pieces; a failure now fails the release, and "Re-run failed jobs" recovers it (D-WA5). Drop "Empty when they could not be collected" from the `play_pieces_sha256` output comment.
- **The new job** after `desktop-windows`, with a two-line comment above it:

  ```yaml
  desktop-windows-arm64:
    name: desktop launcher (Windows arm64)
    needs: desktop-windows
    if: github.ref == 'refs/heads/main'
    permissions:
      contents: read
    uses: ./.github/workflows/launcher-desktop-windows-arm64.yml
    with:
      pieces_artifact: launcher-play-pieces-${{ needs.desktop-windows.outputs.tag }}
      pieces_sha256: ${{ needs.desktop-windows.outputs.play_pieces_sha256 }}
      artifact_name: launcher-zip-arm64-${{ needs.desktop-windows.outputs.tag }}
      tag: ${{ needs.desktop-windows.outputs.tag }}
      version: ${{ needs.desktop-windows.outputs.version }}
      epoch: ${{ needs.desktop-windows.outputs.epoch }}
      build_branch: main
      updater_pubkey: ${{ vars.LAUNCHER_UPDATER_PUBKEY }}
      cache_key: launcher-release-desktop-arm64
    secrets:
      LAUNCHER_MANIFEST_PUBKEY_HEX: ${{ secrets.LAUNCHER_MANIFEST_PUBKEY_HEX }}
  ```

- **`sign`**:
  - `needs: [desktop-windows, desktop-windows-arm64]`.
  - Env: add `ARM_ZIP: artifacts/StargateWorlds-Launcher-windows-arm64.zip` and `ARM_ZIP_SHA256: ${{ needs.desktop-windows-arm64.outputs.zip_sha256 }}`.
  - After the x64 download, add a second pinned download with `name: launcher-zip-arm64-${{ needs.desktop-windows.outputs.tag }}`, `path: artifacts` and the same `if`.
  - The digest step checks both pairs (`ZIP`/`ZIP_SHA256` and `ARM_ZIP`/`ARM_ZIP_SHA256`), with the same two assertions each.
  - The sign step's second argument becomes `"https://github.com/${GITHUB_REPOSITORY}/releases/download/${TAG}"`, the base, with no zip name.
  - The upload `path` lists both `.zip.sig` files and `artifacts/latest.json`.
  - Keep the key step's env and its `run` free of cargo, npm and node; `test_workflow_policy.py` checks this.
- **`publish`**:
  - `needs: [desktop-windows, desktop-windows-arm64, sign]`.
  - Its `if` adds `&& needs.desktop-windows-arm64.result == 'success'`.
  - Env: add `ARM_ZIP` and `ARM_ZIP_SHA256`.
  - Add a pinned download of `launcher-zip-arm64-<tag>` into `artifacts`.
  - The check step runs `publish.sh check` for both zips.

**3. The new file `tools/launcher-release/test_workflow_arm64.py`.** Write it in the style of `test_workflow_policy.py`: `from test_workflow_policy import ROOT, workflow`, and one class `WindowsArm64Release`.

Two parsing notes:

- PyYAML parses the `on:` key as `True`, so `workflow(name)[True]["workflow_call"]`.
- Normalize `needs` with `needs = [n] if isinstance(n, str) else n`.

The tests:

| Test | Asserts |
|---|---|
| `test_reusable_build_is_native_arm64_and_pinned` | `permissions == {}`; `workflow_call` is present. The build job has `runs-on == "windows-11-arm"` and env `WINDOWS_ARCH == "arm64"`. Its joined `run` text contains `play-pieces-in`, `build-desktop.sh launcher`, `build-desktop.sh verify`, `build-desktop.sh package` and `aarch64-pc-windows-msvc`, and not `build-desktop.sh helpers`. Every `uses` not starting with `./` matches `@[0-9a-f]{40}$`. |
| `test_reusable_build_never_sees_a_signing_key` | The file text has no `MINISIGN`; the job has no `environment`. |
| `test_release_calls_it_with_the_x64_jobs_pieces` | `desktop-windows-arm64.uses` is the reusable path; it needs `desktop-windows`. `with.pieces_sha256` contains `needs.desktop-windows.outputs.play_pieces_sha256`; `with.pieces_artifact` starts with `launcher-play-pieces-`; `with.updater_pubkey` contains `vars.LAUNCHER_UPDATER_PUBKEY`; `secrets` has `LAUNCHER_MANIFEST_PUBKEY_HEX`. |
| `test_sign_and_publish_wait_for_both_windows_builds` | `sign` and `publish` both need `desktop-windows` and `desktop-windows-arm64`. `publish.if` contains `needs.desktop-windows-arm64.result == 'success'`. Both download a `launcher-zip-arm64-` artifact. `sign`'s upload `path` names both `.zip.sig` files and `latest.json`. |
| `test_play_pieces_are_on_the_release_critical_path` | `desktop-windows` has exactly two steps that touch the Play pieces: the one whose `run` contains `build-desktop.sh play-pieces`, and the upload whose `with.name` starts with `launcher-play-pieces-`. Neither has `continue-on-error`, and the upload's `retention-days >= 7`. |

Revert reasoning: each test reads a key or text the packet adds, so a revert fails it with an assertion error or a `KeyError`.

Checks: P1 (the report gives the new test count). Docs: none; WA-06 owns them.

### WA-05 PR dry run: arm64 build and the two-zip feed job

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-win-arm64/WA-05-dry-run` · **Worktree:** `lwa-05`
- **Commit:** `ci(launcher): dry-run the arm64 build and the two-zip feed on PRs (WA-05)`
- **Rules:** R1-R9. YAML is LF. The reusable workflow `launcher-desktop-windows-arm64.yml` comes from WA-04 and isn't in your worktree. Reference it by path only; the integration branch joins them. Its inputs are in WA-04 § 1.

Changes to `.github/workflows/launcher-desktop.yml`:

1. In both `paths` lists, after `'.github/workflows/launcher-desktop-macos.yml'` (l.15 and l.39), add `'.github/workflows/launcher-desktop-windows-arm64.yml'`.
2. In `release-dry-run`:
   - Delete the step `sign and check the updater feed with a throwaway key` (l.185-194, with its comment).
   - The upload `desktop-launcher-windows-dry-run` keeps only `artifacts/StargateWorlds-Launcher-windows-x64.zip*` and gains `if-no-files-found: error`.
   - Edit the job comment (l.138-146): the feed is now signed by `release-dry-run-feed`, over both zips.
3. Add after `release-dry-run`, each with a short comment:

   ```yaml
   release-dry-run-arm64:
     name: desktop release build dry run (Windows arm64)
     needs: release-dry-run
     permissions:
       contents: read
     uses: ./.github/workflows/launcher-desktop-windows-arm64.yml
     with:
       pieces_artifact: desktop-play-pieces-dry-run
       pieces_sha256: ${{ needs.release-dry-run.outputs.play_pieces_sha256 }}
       artifact_name: desktop-launcher-windows-arm64-dry-run
       retention_days: 7
       cache_key: launcher-release-desktop-arm64-dry-run
       save_cache: ${{ github.ref == 'refs/heads/main' }}

   release-dry-run-feed:
     name: desktop release build dry run (updater feed)
     needs: [release-dry-run, release-dry-run-arm64]
     runs-on: <copy release-dry-run's runs-on expression exactly>
     timeout-minutes: 30
     defaults:
       run:
         shell: bash
     steps:
       - uses: actions/checkout@v7
         with:
           persist-credentials: false
       - uses: ./.github/actions/rust-toolchain
       - uses: Swatinem/rust-cache@v2
         if: runner.environment == 'github-hosted'
         with:
           shared-key: launcher-release-desktop-dry-run
           save-if: false
           workspaces: |
             crates/launcher/desktop -> ../../../target/desktop
       - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8
         with:
           name: desktop-launcher-windows-dry-run
           path: artifacts
       - uses: actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333 # v8
         with:
           name: desktop-launcher-windows-arm64-dry-run
           path: artifacts
       # (move the deleted step's comment here, saying "both Windows zips")
       - name: sign and check the two-zip updater feed with a throwaway key
         run: |
           set -euo pipefail
           VERSION="$(date -u +%Y).$(date -u +%-m).$(( $(date -u +%-d) * 100 + 2 ))"
           tools/launcher-release/build-desktop.sh feed-dry-run "${VERSION}"
       - uses: actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7
         with:
           name: desktop-launcher-feed-dry-run
           path: |
             artifacts/*.zip.sig
             artifacts/latest.json
           if-no-files-found: error
           retention-days: 7
   ```

Add the new file `tools/launcher-release/test_workflow_arm64_dry_run.py` the same way: import `from test_workflow_policy import workflow`, with one class `WindowsArm64DryRun`.

| Test | Asserts |
|---|---|
| `test_arm64_dry_run_is_the_release_build_without_release_inputs` | `release-dry-run-arm64.uses` is `./.github/workflows/launcher-desktop-windows-arm64.yml`, and it needs `release-dry-run`. `with.pieces_artifact == "desktop-play-pieces-dry-run"`; `with.pieces_sha256` contains `needs.release-dry-run.outputs.play_pieces_sha256`; `with.artifact_name == "desktop-launcher-windows-arm64-dry-run"`. None of `tag`, `version`, `epoch`, `updater_pubkey` is in `with`, and there is no `secrets` key. |
| `test_feed_dry_run_signs_both_zips_after_both_builds` | `release-dry-run-feed` needs exactly `{release-dry-run, release-dry-run-arm64}`. Its `run` text contains `build-desktop.sh feed-dry-run`. Its download names are exactly `{desktop-launcher-windows-dry-run, desktop-launcher-windows-arm64-dry-run}`. `release-dry-run`'s `run` text no longer contains `feed-dry-run`. `secrets.` appears nowhere in the feed job (`str(job)`). |
| `test_paths_filters_cover_the_arm64_workflow` | For `pull_request` and `push`, `workflow(...)[True][event]["paths"]` contains the reusable path. |

Leave `native-engine`, `release-dry-run-mac-pieces`, `release-dry-run-macos` and `release-scripts` unchanged; `test_workflow_policy.py` checks `native-engine`. Checks: P1. Docs: none; WA-06 owns them.

### WA-06 Docs

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `launcher-win-arm64/WA-06-docs` · **Worktree:** `lwa-06`
- **Commit:** `docs(launcher): Windows on Arm release and updater (WA-06)`
- **Rules:** R1, R3, R5 (every file below is CRLF), R7, R8, R9. Document the Contract, the names and the decisions in the ledger; the code lands in the same PR. State what is tested (CI dry run) and what isn't yet (no release has carried the arm64 zip; WA-U1..U3 are pending).

| File | Change |
|---|---|
| `README.md` (root) l.37 | The launcher downloads name both Windows zips (x64; arm64 for Windows on Arm). |
| `crates/launcher/desktop/docs/release.md` | l.13-14 and l.39-40: the asset list. "Jobs" (l.81 and nearby): the `desktop-windows-arm64` job, `sign` signing both zips, `publish` needing both, D-WA5's critical path and its rerun recovery. l.161 and l.163: the dry-run jobs `release-dry-run-arm64` and `release-dry-run-feed`. l.232: "the feed offers `windows-x86_64` and `windows-aarch64`". The feed example (l.254-268) gets both platform entries. A new subsection, "Windows on Arm", covers: the native arm64 launcher; the i686 Play pieces shared with x64; the game under x86 emulation; the compile-time key (D-WA3); x64-on-Arm staying x64 with no cross-grade; and the `windows-11-arm` hosted runner, with the tools step (D-WA8). |
| `crates/launcher/desktop/docs/updater.md` l.28 and l.176 | Add `windows-aarch64`; the build target picks the key. |
| `docs/client/launcher-self-update.md` l.33-37 | Both Windows keys in one feed; an arm64 launcher updates to arm64, an x64 one to x64. |
| `docs/client/launcher-guide.md` l.75-76 | Which zip to download on an Arm PC, and that the x64 zip also runs there, emulated. |
| `docs/client/launcher-distribution-setup.md` l.89 | The release assets and the `launcher-feed-<tag>` contents (both `.sig` files). |
| `tools/launcher-release/README.md` l.7 | `build-desktop.sh` builds either arch (`WINDOWS_ARCH`); `sign` covers both zips. |
| `tools/ci-runners/README.md` l.228 | The `launcher-desktop.yml` pool row adds `desktop release build dry run (updater feed)` on the pool. Add a note that the arm64 dry run runs on hosted `windows-11-arm`, not the pool. |

Don't touch the privacy inventory: telemetry already reports `aarch64` (F10). Don't grow `docs/analysis/playtests/2026-10-03-macos-wine/launcher-implementation-ledger.md` (D-WA11, over the cap). In the report, list the doc-update-map row 57 targets you touched and say the ledger was skipped for WA-F2, so the PR body can carry it. Checks: R5's CRLF test on every file, and relative links resolve (open each target).

## Close-out

### WA-07 Close-out

- **Host:** windows · **Agent:** documentation-writer · **Worktree:** `lwa-closeout` (ops, from main after the wave PR and any campaign-review PR have merged)
- **Commit:** `docs(launcher-win-arm64): close out the campaign (WA-07)`

The work:

1. Status docs, once: a launcher line in `docs/project-status.md` and the matching `docs/gap-analysis*` line, if one covers launcher platforms. Say it ships natively for Windows on Arm, with release proof pending.
2. `docs/guides/unified-uat.md` gets WA-U1..WA-U3, copied from the ledger's UAT table.
3. The ledger: packet rows set to Integrated or Done, and WA-F1 and WA-F2 left as follow-ups. Rewrite RESUME.md.
4. Write an empty `.claude/state/active-campaign`.

`ops` then retires `lwa-01`..`lwa-06` and `lwa-integration` with `rm-worktree.ps1 --merged-into <PR> <names>`, and posts the board handoff. The first release after the merge is the orchestrator's call under the standing launcher-release instruction, if that instruction covers this campaign; otherwise it is the owner's.
