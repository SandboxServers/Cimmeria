# Work packets: launcher-linux-play-proton (rules, contract, waves 0-2)

Ledger: [README.md](README.md). Waves 3-4: [work-packets-w3.md](work-packets-w3.md).
Paths: `D/` = `crates/launcher/desktop/`, `E/` = `D/engine/src/`, `S/` = `D/shell/src/`.

## Dispatch

- **Host: linux** for every packet (the Steam Deck). On another host, return `blocked`.
- **One final PR** (owner rule, D-PP17). `ops` creates the integration branch
  `launcher-linux-play-proton/integration` (worktree `pp-integration`) from
  `origin/main`. Each packet's branch `launcher-linux-play-proton/<pp-NN>` is cut
  from the integration tip after its dependencies have merged into it
  (`branch-at.sh`), worktree name `pp-NN`. `ops` merges a packet with
  `merge-noff.sh` once its packet review says `ship`. Order: README § Packets.
- Waves: 0 = PP-S1 (hands-on, beside wave 1). 1 = PP-01, PP-02, then PP-04
  (after PP-01), PP-03 (after PP-02). 1B = PP-20..PP-25, install quality
  ([work-packets-w1b.md](work-packets-w1b.md)), beside waves 1 and 2.
  2 = PP-05..PP-09 plus PP-15, PP-16 and the spike PP-S2
  ([work-packets-w2b.md](work-packets-w2b.md)); released by
  [worknotes/PP-S1-ruling.md](worknotes/PP-S1-ruling.md) (2026-10-25).
- Report: `<worktree>/.claude/reports/launcher-linux-play-proton/<packet>-coder.md`;
  return one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

## Rules every packet carries

Read this section and § Contract before your packet section.

- R1. Every compiling cargo call goes through the lane, with bash on this host:
  `bash tools/build-lane/lane.sh cargo <verb> --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <crate> ...`.
  Crates: `cimmeria-launcher-engine` (E/), `cimmeria-launcher-desktop` (S/),
  `cimmeria-launcher-core` (`crates/launcher-core/`, waves 1B and 2).
  Never run `cargo` directly. Order: `fmt -- --check`, then
  `clippy --all-targets -- -D warnings`, then `test --lib <filter>`.
- R2. Frontend checks run in `D/frontend`: `npm run check`, then `npm test`
  (and the packet's `uat:*` script). These are not cargo; no lane.
- R3. Tests are **unit** tests (TESTING.md) on fixture directories built in a
  `tempfile::TempDir`; never touch the real `~/.steam`. Each test must fail when
  the change it guards is reverted; say in the report which line reverts it.
- R4. New Linux-only code is `#[cfg(target_os = "linux")]`; pure logic that the
  tests need is `#[cfg(any(target_os = "linux", test))]`. Do not edit code under
  `#[cfg(target_os = "macos")]` or `#[cfg(windows)]` unless the packet says so:
  this host cannot compile it.
- R5. Files: soft cap 500 lines, hard cap 700. Tests go in a sibling
  `*_tests.rs` file (`#[cfg(test)] #[path = "x_tests.rs"] mod tests;`).
- R6. Files under `docs/` and `D/docs/` are CRLF; keep their line endings.
- R7. Edit only the files your packet names. No `git stash`, no
  `git worktree prune`, no push. Commit locally with the packet's subject and
  the attribution lines from your brief, then stop.
- R8. No path, user name or free error text in a forwarded `launcher.*` event
  field; paths go only to local logs.
- R9. The frontend never supplies a path, executable, hash or environment.
  Native code resolves them.

## Contract

### C-1 Discovery (`E/linux_proton/discovery.rs`, PP-01)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtonKind { Ge, Experimental, Valve, Other }   // declaration order = default rank

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtonInstall {
    pub id: String,               // "<kind>:<directory name>", e.g. "ge:GE-Proton11-11-x86_64"
    pub name: String,             // second token of `version`, else the directory name
    pub kind: ProtonKind,
    pub root: PathBuf,            // canonical directory holding the `proton` script
    pub version: String,          // `version` file, trimmed, e.g. "1792432585 GE-Proton11-11"
    pub runtime: Option<PathBuf>, // <runtime dir>/_v2-entry-point when require_tool_appid is installed;
                                  // logged only: D-PP3 ruled bare (PP-S1), invocation never uses it
}

pub struct SteamRoots { pub client: PathBuf, pub libraries: Vec<PathBuf> }

pub fn steam_roots(home: &Path) -> Option<SteamRoots>;
pub fn discover(roots: &SteamRoots, system_tools: &Path) -> Vec<ProtonInstall>; // ranked
pub fn choose<'a>(found: &'a [ProtonInstall], preferred: Option<&str>) -> Option<&'a ProtonInstall>;
```

### C-2 Prefix (`E/linux_proton/prefix.rs`, PP-04)

```rust
pub struct Prefix { pub compat: PathBuf, pub pfx: PathBuf, _lock: std::fs::File }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stamp {
    pub schema_version: u32,          // 1
    pub proton_version: String,       // ProtonInstall::version used by the last good setup
    #[serde(default)] pub physx: PhysxState,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysxState { #[default] NotInstalled, Installed, Failed }

pub const STAMP_FILE: &str = "cimmeria-setup.json";
pub fn claim(state_root: &Path) -> Result<Prefix, ProtonError>;   // <state_root>/proton/compatdata
impl Prefix {
    pub fn stamp(&self) -> Option<Stamp>;                     // None when missing or unreadable
    pub fn write_stamp(&self, stamp: &Stamp) -> Result<(), ProtonError>; // temp file + rename
    pub fn needs_setup(&self, install: &ProtonInstall) -> bool; // no stamp, or another version
    pub fn profile_root(&self) -> PathBuf;  // pfx/drive_c/users/steamuser
}

#[derive(Debug, thiserror::Error)] // messages: "no usable Proton", "the Proton prefix is in use",
pub enum ProtonError { Missing, Busy, Setup(&'static str), Io(#[from] std::io::Error) } // "Proton setup failed: {0}", transparent
```

### C-3 Invocation (`E/linux_proton/invocation.rs`, PP-05)

```rust
pub struct Invocation { pub program: PathBuf, pub args: Vec<OsString> }
/// Always bare (D-PP3, PP-S1 ruling: the runtime container returns exit 1 and
/// no stdout): program = <root>/proton, args = ["runinprefix", target, extra...].
/// install.runtime is ignored.
pub fn runinprefix(install: &ProtonInstall, target: &str, extra: &[OsString]) -> Invocation;
/// program = <root>/proton, args = ["run", "wineboot.exe"] (D-PP2, PP-S1 step 1).
pub fn setup(install: &ProtonInstall) -> Invocation;
```

### C-4 Environment (`E/linux_proton/environment.rs`, PP-04)

Start empty. Copy only these from the launcher's environment when set:
`HOME USER LOGNAME PATH LANG LC_ALL DISPLAY WAYLAND_DISPLAY XAUTHORITY
XDG_RUNTIME_DIR XDG_SESSION_TYPE XDG_CURRENT_DESKTOP XDG_DATA_DIRS
DBUS_SESSION_BUS_ADDRESS PULSE_SERVER PIPEWIRE_RUNTIME_DIR SteamDeck
GAMESCOPE_WAYLAND_DISPLAY`. Never copy `PROTON_LOG`, `SteamAppId`,
`SteamGameId`, `LD_PRELOAD`, `WINEPREFIX` or `APPIMAGE`/`APPDIR`/`OWD`
(they are simply not on the list). Then set:

| Variable | Value |
|---|---|
| `STEAM_COMPAT_DATA_PATH` | `Prefix::compat` |
| `STEAM_COMPAT_CLIENT_INSTALL_PATH` | `SteamRoots::client` |
| `WINEDLLOVERRIDES` | `winemenubuilder.exe=d;mscoree=d;mshtml=d` (Proton appends its own); the game's `game()` value is prefixed with `d3dx9_40=n;xinput1_3=n;` by PP-16 (C-8) |
| `DXVK_FRAME_RATE` | the frame cap (game only) |
| `DXVK_LOG_LEVEL`, `DXVK_LOG_PATH` | `info` and the `Z:\` log folder, only when a log folder exists |

```rust
pub const PASSTHROUGH: &[&str] = &[/* the list above */];
pub fn base(get: impl Fn(&str) -> Option<OsString>, prefix: &Prefix, steam_client: &Path)
    -> BTreeMap<OsString, OsString>;
pub fn game(base: BTreeMap<OsString, OsString>, frame_rate: u32, log_guest: Option<&str>)
    -> BTreeMap<OsString, OsString>;
```

### C-5 Launch resources (`E/storage/launch/resource.rs`, PP-06)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProtonSelection { pub id: String, pub root: PathBuf, pub version: String }
// new last field of Resources:
#[serde(default, skip_serializing_if = "Option::is_none")]
pub proton: Option<ProtonSelection>,
```

On Linux, `Native` installations are admitted only with `proton: Some(..)`,
`graphics: None` and a still-matching `root/version` file.

### C-6 Failure codes (PP-06)

`proton_missing`, `proton_setup_failed`, added to the Rust `FailureCode`
(find it with `grep -rn "enum FailureCode" crates/launcher*`), to the shared
`failure-codes.json` fixture and to `D/frontend/src/failure-copy.ts`:

| Code | Title | Fix |
|---|---|---|
| `proton_missing` | Proton isn't installed. | Install Proton Experimental from Steam, or GE-Proton with ProtonUp-Qt, then press Play again. |
| `proton_setup_failed` | Proton couldn't set up the game. | Press Play to try again. If it fails again, upload your logs from Settings. |

## Wave 0

### PP-S1: Proton Play spike on the Deck (hands-on)

**Done 2026-10-25** ([worknotes/PP-S1.md](worknotes/PP-S1.md), ruling
[worknotes/PP-S1-ruling.md](worknotes/PP-S1-ruling.md)). The spec below is kept as run.

**Agent:** `claude --agent hands-on` on the Deck, owner present (Desktop Mode).
**Output:** `docs/analysis/launcher-linux-play-proton/worknotes/PP-S1.md` (CRLF),
committed on the integration branch. No code. Record every command and its
result; mark each row **tested** or **untested**.

Inputs:
- The four Windows pieces: from the newest green `launcher runtime probe`
  run on `main`, artifact `windows-runtime-probe` (worker, prerequisite worker,
  client-patches DLL), and the newest green `client-telemetry-build` artifact
  (non-`lab-bridge` DLL): `gh run download <run> -n <artifact> -D ~/pp-spike/bin`.
  Record each SHA-256.
- A launcher-installed game copy (sibling U1); note its path and whether it is
  on the SD card.
- `P` = a Proton root from README F-04; `C=~/.local/share/Steam`;
  `export STEAM_COMPAT_CLIENT_INSTALL_PATH=$C STEAM_COMPAT_DATA_PATH=~/pp-spike/compat-<proton>`;
  `mkdir -p` it. `R` = that Proton's runtime `_v2-entry-point` (README F-05).

Steps (run 1-6 with GE-Proton11-11 and Proton Experimental; 1-4 with Proton 9.0):

1. **Prefix setup.** Try `"$P/proton" run wineboot.exe` (then `cmd.exe /c exit`
   if that hangs). Pass: exit 0 within 120 s, `pfx/system.reg` exists, and
   `pfx/drive_c/windows/syswow64/d3d9.dll` is DXVK (`strings … | grep -ci dxvk` > 0).
2. **Stdio.** Pipe a request for a missing exe into the worker:
   `printf '%s\n' '{"schema_version":1,"operation_id":"6f1c0a52-0000-4000-8000-000000000001","exe":"Z:\\nope\\SGW.exe","directory":"Z:\\tmp","dlls":[]}' | "$P/proton" runinprefix 'Z:\home\deck\pp-spike\bin\cimmeria-launch-worker.exe'`.
   Pass: one stdout line with `"event":"not_started"`. Repeat wrapped as
   `"$R" --verb=waitforexitandrun -- "$P/proton" runinprefix …`.
3. **Game, no DLLs.** Same, with the real `SGW.exe` and its folder as `Z:\` paths,
   `"args":["-windowed","ResX=1280","ResY=720"]`. Pass: `process_started`, the
   login screen renders. Then quit: `process_exited`, the host process exits
   within 10 s. Note whether `wineserver` lingers.
4. **Patches DLL.** `"dlls":["Z:\\…\\cimmeria_client_patches.dll"]`. Pass: the
   login screen renders and the DLL's boot line appears where
   `docs/architecture/client-patches.md` says it logs.
5. **Both DLLs** (patches, then telemetry). Pass: `cimmeria-client-telemetry.log`
   appears beside `SGW.exe` (no session marker is fine: it should say so).
6. **Login.** Sign in on the configured server and enter a world. Record login
   and world entry separately from `process_started`.
7. **PhysX.** In a fresh prefix, run step 3 without PhysX and record whether the
   game starts. Then run the prerequisite worker through `runinprefix` with a
   schema-1 `PrepareRequest` (fields: `D/runtime-probe/src/prerequisite/mod.rs`)
   whose `package` is the game's
   `.cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe`. Pass:
   result `probed`, SDK `initialized_and_released`.
8. **SD card.** If the game is on `/run/media/…`, repeat step 3 in container
   mode; else copy it there and repeat.
9. **Optional, Game Mode.** A one-line script running step 3, added as a
   non-Steam game. Record what renders and whether the controller works.

Rulings the worknote must state (each with its evidence row): container or
bare (D-PP3); the setup target (D-PP2); the default Proton order (D-PP4);
whether PhysX is needed (D-PP7); any extra variable the game needed (for
example `WINE_LARGE_ADDRESS_AWARE`), and whether the display arguments work.
**Stop and report** if no mode passes step 2, or no Proton passes step 4:
the architect then re-slices (a file-based worker channel is the fallback).

## Wave 1

### PP-01: Proton discovery and ranking

**Agent:** packet-coder. **Depends:** none. **Commit:** `feat(launcher): find installed Proton builds on Linux (PP-01)`.

Files (new unless noted): `E/linux_proton/mod.rs`, `E/linux_proton/vdf.rs`,
`E/linux_proton/discovery.rs`, `E/linux_proton/discovery_tests.rs`; `E/lib.rs`
(one line: `#[cfg(any(target_os = "linux", test))] pub mod linux_proton;`).

`mod.rs`: `pub mod discovery; mod vdf;` plus a module doc of 3-5 lines naming D-PP1..D-PP5.

`vdf.rs`: a tiny reader for Valve's text KeyValues, enough for these files.
`pub(crate) fn values<'a>(text: &'a str, key: &str) -> Vec<&'a str>` returns
every quoted value whose quoted key equals `key`, at any depth (lines look like
`\t"path"\t\t"/home/deck/.local/share/Steam"`). Ignore `//` comments. Unescape
`\\` to `\`. No nesting model is needed.

`discovery.rs` (C-1):
1. `steam_roots(home)`: candidates `home/.steam/root`, `home/.local/share/Steam`,
   `home/.steam/steam`. Canonicalize each existing one; the first is `client`;
   drop duplicates. `libraries` = `client` plus every `path` value of
   `client/steamapps/libraryfolders.vdf` that exists, canonicalized, deduplicated,
   in file order. None when no candidate exists.
2. `discover(roots, system_tools)`: candidates are
   `<lib>/steamapps/common/*` whose name starts with `Proton` for each library,
   `<client>/compatibilitytools.d/*` and `system_tools/*` (production passes
   `/usr/share/steam/compatibilitytools.d`). A candidate counts only if it holds
   a regular file `proton`, a `toolmanifest.vdf` and a `version` file. Kind:
   directory name starting `GE-Proton` → `Ge`; `Proton - Experimental` → `Experimental`;
   `Proton <digit>…` under `steamapps/common` → `Valve`; else `Other`.
   `id` = `"{kind}:{dir name}"` with kind in snake_case.
   `runtime`: read `require_tool_appid` from the toolmanifest; for each library,
   read `steamapps/appmanifest_<appid>.acf` → `installdir`; if
   `<lib>/steamapps/common/<installdir>/_v2-entry-point` is a file, use it.
3. Rank: by `kind` (enum order), then newer first. Newer = larger numeric
   `(major, minor)` parsed from the name (`GE-Proton11-11` → (11,11),
   `Proton 9.0 (Beta)` → (9,0)); Experimental and Other tie-break on the first
   token of `version` (a build timestamp), larger first; then `id` ascending.
4. `choose(found, preferred)`: the entry whose `id == preferred`, else `found.first()`.

Tests (`discovery_tests.rs`, build a fake Steam tree in a TempDir that mirrors
README F-04 and F-05): roots through a `.steam/root` symlink and a second
library from `libraryfolders.vdf`; a directory without `proton` is skipped;
kinds and ids; ranking puts `ge:GE-Proton11-11-x86_64` before
`ge:GE-Proton7-38` before `experimental:Proton - Experimental` before
`valve:Proton 9.0 (Beta)` before `valve:Proton 8.0`; `runtime` resolved for
appid 4183110 and `None` when the runtime is not installed; `choose` honours a
present preference and falls back when it is gone; `vdf::values` on a
two-level sample with a comment.

Lane (R1): fmt, clippy, `test --lib linux_proton`. Docs: none (PP-13).

### PP-02: Embed the Windows pieces in the Linux shell

**Agent:** rust-gameserver-dev. **Depends:** none. **Commit:** `feat(launcher): embed the pinned Windows Play pieces in the Linux build (PP-02)`.

Files: `D/shell/build_payload.rs`, `S/payload_startup.rs`, `S/main.rs`,
`S/host/launch/resources.rs`, `S/host.rs` (prerequisite helper path only),
`E/embedded_payload/mod.rs` if it is Windows-gated, and their test files.

1. `build_payload.rs`: embed when `CARGO_CFG_TARGET_OS` is `windows` **or**
   `linux`. Same three pins as Windows, plus on Linux the prerequisite worker
   pinned by the existing `CIMMERIA_PREREQUISITE_HELPER_SHA256`
   (`S/host.rs:171`) at `windows/cimmeria-prerequisite-worker.exe`. Keep the
   rule "all required pins under `CIMMERIA_EMBED_PAYLOAD=1` or a tagged build";
   the telemetry DLL stays optional. A Linux build with no pins and no
   `CIMMERIA_EMBED_PAYLOAD` still builds (developer builds), with no payload.
2. `payload_startup.rs` and the `mod payload_startup;` / call sites in
   `main.rs` (lines 11-12, 550-552, 604-611): widen `#[cfg(windows)]` to
   `#[cfg(any(windows, target_os = "linux"))]` where they concern the payload.
   Do not touch the WebView2 lines. Extracted files on Linux are mode 0644 in
   a 0700 directory; refuse symlinks as Windows refuses reparse points.
3. `resources.rs`: on Linux, `bundled()` reads the extracted directory. Keep
   `graphics` `None` off macOS (D-PP6).
4. Expose the extracted prerequisite worker path to the engine as
   `Option<PathBuf>` on the host (`S/host.rs`), Linux only, for PP-08.

Tests: the existing payload tests run on Linux (remove a Windows-only gate
only where the code is now portable); add one that a Linux payload with a
mismatched pin is refused and nothing is extracted. Name the reverting line.

Lane (R1): fmt, clippy and `test --lib payload` for `cimmeria-launcher-desktop`,
then the engine if you touched it. Docs: none (PP-13 covers launch.md).

### PP-03: Linux release job carries the pinned pieces

**Agent:** packet-coder. **Depends:** PP-02. **Commit:** `build(launcher): the Linux AppImage embeds the pinned Windows Play pieces (PP-03)`.

Files: `tools/launcher-release/build-desktop-linux.sh`,
`.github/workflows/launcher-desktop-linux.yml`,
`.github/workflows/launcher-release.yml` (the `desktop-linux` job's `needs`
and artifact download only), `tools/launcher-release/test-release-scripts.sh`
if it asserts the old header.

1. Copy the macOS job's pattern (`build-desktop-macos.sh` lines 63-67 and
   200-216; the macOS workflow's artifact download): the Linux job downloads
   the same Windows-built artifact, stages the four pieces with
   `D/tools/stage-helper.py` (kinds `launch`, `prerequisite`,
   `client-patches`, `client-telemetry`), exports the four
   `CIMMERIA_*_SHA256` pins and `CIMMERIA_EMBED_PAYLOAD=1`.
2. Keep the loose-file guard (lines 142-145) and change only its comment to
   "Windows pieces are embedded in the launcher binary, never loose (D-PP8)".
3. `verify` stage: run the extracted launcher with `--verify-embedded-payload`
   (as the Windows job does) and fail on a non-zero exit.
4. Replace the header lines 24-28 with what the script now carries.
5. The PR dry run in `launcher-desktop.yml` must still pass without secrets:
   when the artifact is absent, build without the payload and say so.

Tests: `shellcheck` both scripts; `bash tools/launcher-release/test-release-scripts.sh`.
Docs owed: `D/docs/release.md` § What the Linux job builds (CRLF, R6) and line
432-433, in this packet.

### PP-04: Proton prefix, stamp and environment

**Agent:** packet-coder. **Depends:** PP-01 (merged into integration). **Commit:** `feat(launcher): a private Proton prefix and its environment (PP-04)`.

Files: `E/linux_proton/mod.rs` (add `pub mod prefix; pub mod environment; pub mod paths;`),
new `prefix.rs`, `prefix_tests.rs`, `environment.rs`, `environment_tests.rs`, `paths.rs`.

1. `prefix.rs` per C-2. `claim` creates `<state_root>/proton/compatdata`
   (mode 0700) and takes an exclusive lock on `<state_root>/proton/prefix.lock`
   with `File::try_lock()` (std); if the pinned toolchain lacks it, use the
   engine's existing non-macOS lock helper (`grep -rn "try_lock\|flock" E/`)
   and name it in the report. A held lock is `ProtonError::Busy`.
   `pfx = compat.join("pfx")`. Stamp is JSON at `compat/STAMP_FILE`.
2. `environment.rs` per C-4. `get` is injected so tests do not read the real
   environment; production passes `|k| std::env::var_os(k)`.
3. `paths.rs`: copy `guest()` from `E/mac_wine/paths.rs` (absolute host path
   to `Z:\…`, refuse non-UTF-8 and relative paths) with its own error type and
   a first-line comment: "Twin of mac_wine/paths.rs; FU-5 merges them." (D-PP14)

Tests: second `claim` is `Busy`; stamp round-trip; `needs_setup` true with no
stamp and with another version; `base` copies `DISPLAY`, not `PROTON_LOG`,
`SteamAppId`, `LD_PRELOAD`; `game` sets `DXVK_FRAME_RATE`, log variables only
with a log folder; `guest("/home/deck/a b/x.exe")` = `Z:\home\deck\a b\x.exe`.

Lane (R1): engine fmt, clippy, `test --lib linux_proton`.

## Wave 2

Released: `worknotes/PP-S1-ruling.md` exists (2026-10-25) and the packet
text below is re-specced to it. Wave 2 also holds PP-15, PP-16, PP-S2 and
(later) PP-08W from [work-packets-w2b.md](work-packets-w2b.md); merge order
there. Known gap until PP-08W: Linux Play stops at `proton_setup_failed`
(PhysX), because the worker's probe answers `failed/probe` under Proton.

### PP-05: Proton invocation and prefix setup

**Agent:** packet-coder. **Depends:** PP-04, PP-S1 ruling. **Commit:** `feat(launcher): start Windows helpers through Proton (PP-05)`.

Files: `E/linux_proton/mod.rs` (add `pub mod invocation; pub mod setup;`),
new `invocation.rs`, `setup.rs`, `invocation_tests.rs`, `setup_tests.rs`.

1. `invocation.rs` per C-3: **bare only**, `setup()` = `proton run wineboot.exe`.
   No `_v2-entry-point` argv anywhere (PP-S1 ruling, D-PP3).
2. `setup.rs`:
   `pub async fn ensure(install: &ProtonInstall, prefix: &Prefix, env: &BTreeMap<OsString, OsString>, log: Option<&Path>) -> Result<(), ProtonError>`.
   If `!prefix.needs_setup(install)`, return Ok. Else spawn `setup(install)`
   with `env_clear()` + `env`, stdin null, stdout null, stderr appended to
   `log` (or null), `kill_on_drop(true)`, timeout 180 s. Exit 0 and
   `prefix.pfx.join("system.reg")` present → write a stamp with this
   `proton_version`, keeping the old `physx` value; anything else →
   `ProtonError::Setup("prefix")`.

Tests: `invocation` exact argv for `runinprefix` and `setup`, including with
`install.runtime = Some(..)` (still bare; fails if a container shape comes back); `ensure` with a
fake `proton` shell script in a TempDir that creates `pfx/system.reg` (stamp
written), one that exits 1 (Setup error, no stamp), one that sleeps past a
test-only shorter timeout (make the timeout a parameter of an inner fn).

Lane (R1): engine fmt, clippy, `test --lib linux_proton`.

### PP-06: Linux launch admission and failure codes

**Agent:** rust-gameserver-dev. **Depends:** PP-05. **Commit:** `feat(launcher): admit Linux Play with a selected Proton (PP-06)`.

Files: `E/storage/launch/resource.rs`, `E/storage/launch/mod.rs` (gate at
196-209 only), `E/storage/launch/worker.rs` (`Ownership`, `prepare` 371-416
only), the `FailureCode` file, `failure-codes.json`, `D/frontend/src/failure-copy.ts`,
and the matching tests.

1. C-5: add `ProtonSelection` and `Resources.proton`. Fix every `Resources {..}`
   literal in engine tests with `proton: None`.
2. Gate: `Native` passes on Windows as now; on Linux it passes only when
   `resources.proton` is `Some`, `resources.graphics` is `None`, and
   `root/version` (trimmed) equals `version`; a mismatch is refused with
   `proton_missing` (use the existing refusal path that carries a failure code).
3. `worker.rs`: add `#[cfg(target_os = "linux")] Proton { _prefix: linux_proton::prefix::Prefix }`
   to `Ownership`, and in `prepare` a Linux branch that calls
   `super::proton::prepare(...)` with the same arguments `wine::prepare` takes.
   Create `E/storage/launch/proton.rs` with only a stub returning
   `Err(<proton_missing refusal>)` and `#[cfg(target_os = "linux")] mod proton;`
   in `launch/mod.rs`; PP-07 fills it.
4. C-6: add the two codes everywhere the taxonomy test requires (run it to find out).

Tests: admission on Linux with `proton: None` refused; with a selection whose
version file changed refused `proton_missing`; with a matching selection
admitted; an old plan JSON without `proton` still decodes and its digest is
unchanged; the taxonomy fixture test passes with the two codes.

Lane (R1): engine fmt, clippy, `test --lib launch`; then R2 `npm test`.

### PP-07: Proton launch preparation

**Agent:** rust-gameserver-dev. **Depends:** PP-06. **Commit:** `feat(launcher): Play starts the game through Proton (PP-07)`.

Files: `E/storage/launch/proton.rs`, new `E/storage/launch/proton_tests.rs`.
Model it on `E/storage/launch/wine.rs` (read it fully first).

`pub(super) fn prepare(plan, state_root, session, servers, display, placements) -> Result<(HelperCommand, Request, linux_proton::prefix::Prefix), IntentError>`:
1. Rediscover (`steam_roots(home)`, `discover`), find `plan.resources.proton.id`;
   missing or version changed → `proton_missing`.
2. `prefix::claim(state_root)`; `Busy` → the existing busy refusal.
3. `setup::ensure(...)` (blocking on the runtime the worker already uses);
   failure → `proton_setup_failed`. Then, in this order, PP-16 adds
   `directx::ensure` and PP-08 adds PhysX here; leave a
   `// PP-16 directx, PP-08 physx, PP-17 display` comment at the spot.
   Do not write the display profile here (PP-17 does, D-PP22).
4. Binaries folder: `cimmeria_patchset::resolve_existing_case(game_root, Path::new("Working/Binaries"))`
   (D-PP12), never a literal join.
5. `Request` exactly as `wine.rs` builds it, with `paths::guest` for exe,
   directory and DLLs (patches first, then telemetry from `telemetry::injected`).
6. Log folder `logs/game/<operation>/` exactly as `wine.rs` (same pruning);
   stderr file `proton.log`. Environment: `environment::game(environment::base(..), frame_rate, log_guest)`
   plus `game_telemetry::passthrough_environment()` when a session exists.
7. `HelperCommand { executable, arguments }` from `invocation::runinprefix(install, &guest(helper), &[])`,
   `directory` = binaries folder.

Tests: with a fake Steam tree and a fake `proton` script that writes
`pfx/system.reg`: the command's argv (bare `proton runinprefix <Z:\ worker>`), the request's guest
paths with a lower-case `Working/binaries` folder on disk (fails if step 4 is a
literal join), DLL order, no `PROTON_LOG` in the environment, and
`proton_missing` when the selected Proton's version file changes.

Lane (R1): engine fmt, clippy, `test --lib launch`.

### PP-08: PhysX in the Proton prefix

**Agent:** packet-coder. **Depends:** PP-05, PP-07, PP-S1 ruling (skip the packet, as `Done (not needed)`, if the ruling says PhysX is not needed). **Commit:** `feat(launcher): install legacy PhysX into the Proton prefix on first Play (PP-08)`.

Files: `E/linux_proton/mod.rs` (add `pub mod physx;`), new `physx.rs` and
`physx_tests.rs`, `E/storage/launch/proton.rs` (one call after `ensure`).

`pub async fn ensure_physx(install, prefix, env, helper: &Path, game_binaries: &Path, operation: Uuid, log: Option<&Path>) -> Result<(), ProtonError>`:
skip when the stamp says `Installed`. Else build the schema-1 `PrepareRequest`
from `D/runtime-probe/src/prerequisite/mod.rs` (guest paths; `package` =
`<game root>/.cimmeria-prerequisites/PhysX/PhysX_7.11.13_SystemSoftware.exe`
resolved with `resolve_existing_case`; `scratch` = a new
`<state_root>/proton/scratch/<operation>`), and run it with
`prerequisites::supervisor::run` on `invocation::runinprefix(install, &guest(helper), &[])`,
180 s deadline. `Probed` with an initialized SDK → stamp `Installed`; anything
else → stamp `Failed` and `ProtonError::Setup("physx")`. The helper path comes
from PP-02's host field through the launch resources; if it is absent,
`proton_setup_failed`.

PP-S1 ruling (D-PP7): PhysX is needed and this acceptance rule stays strict;
never treat `failed/probe` as installed. Two additions:
- Pass the supervisor's stderr log as `Some(<log folder>/physx.log)` (the
  `logs/game/<operation>/` folder PP-07 makes), not `None` as macOS does.
- Log `proton_physx` on `launcher.proton` with `result` = the closed result
  kind (`probed`, `installer_failed`, `failed`) and `reason` (the `Failure`
  code, or the installer code); no path (R8).
Until PP-08W lands, U3 fails here on the real worker; that is expected.

Tests: a fake runner printing `Probed` (stamp `Installed`, no second run);
one printing `InstallerFailed` (stamp `Failed`, error). Lane (R1): engine
fmt, clippy, `test --lib linux_proton`.

### PP-09: Shell: Linux Play status and resources

**Agent:** packet-coder. **Depends:** PP-02, PP-06. **Commit:** `feat(launcher): the Linux launcher offers Play when a Proton is found (PP-09)`.

Files: `S/host/launch/mod.rs` (lines 104-120 and 209-213 only),
`S/host/launch/resources.rs`, their tests.

1. Playable on Linux: `Native` content with `cfg!(target_os = "linux")`
   counts as playable when a Proton is chosen.
2. On Linux, after `bundled()`, discover (C-1) with the saved preference
   (none until PP-10: pass `None`) and set `Resources.proton`. No Proton →
   `prerequisites_missing` gets `"proton"` and Play stays disabled; the
   status still reports `resources_available` from the payload alone.
3. `setup_complete` on Linux needs a chosen Proton but **not** a prepared
   prefix (the first Play prepares it, D-PP7).

Tests: status with a fake Steam tree (`HOME` injected through the function
that reads it; add a parameter if needed): Proton found → playable and
`prerequisites_missing` empty; none → `["proton"]`.

Lane (R1): shell fmt, clippy, `test --lib launch`.
