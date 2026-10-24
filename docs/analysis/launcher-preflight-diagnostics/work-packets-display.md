# Launcher preflight diagnostics: display packets (PD-22 to PD-30)

Ledger: [README.md](README.md). The rules R1-R10, the checks table (D1, D2e, D2s, D2p, D3e, D3s, D3p, F1, MG, WG, RG, WR) and C-PD1 are in [work-packets.md](work-packets.md); **every packet here follows them, so read § Rules and § Checks there first.** Owner rulings: [OWNER-DECISIONS.md § D2, § D3](OWNER-DECISIONS.md). The D4 extras (PD-31 to PD-40) are in [work-packets-mac-extras.md](work-packets-mac-extras.md). Evidence: [research-display-wine.md](research-display-wine.md), [research-display-client.md](research-display-client.md), [observation-single-display.md](observation-single-display.md).

Paths: `E` = `crates/launcher/desktop/engine/src/`, `S` = `crates/launcher/desktop/shell/src/`, `FE` = `crates/launcher/desktop/frontend/`, `RP` = `crates/launcher/desktop/runtime-probe/src/`, `LD` = `crates/launcher/desktop/docs/`. Anchors were taken at `main` `2019922fe`; re-grep them and return `blocked` when one no longer holds.

## Dispatch

| Wave | Packets and order | Ships as |
|---|---|---|
| 1 | PD-22, in parallel with PD-01 and PD-02 (see work-packets.md § Dispatch). | Its own PR, as soon as its packet loop says `ship` (black-screen hotfix). |
| D | PD-23 first. Then PD-24 and PD-26 in parallel from the PD-23 tip. Then PD-25 from the PD-24 tip. Merge order PD-23, PD-24, PD-25, PD-26. Runs at the same time as wave 2. | One PR per packet, stacked (`--base`). |
| M | PD-28 moved to wave M (work-packets-mac-extras.md § Dispatch); its spec stays below. | Merged into the wave M integration branch. |
| 4 | PD-29 and PD-30 when wave D has merged; PD-27 only after UAT-PD8 and PD-31. | One PR per packet. |

## Contract

### C-PD6: display preference (PD-23 builds it)

`E/storage/display_options.rs` (new, a child module of `storage`, so it can reach `Preferences`' private fields):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")]
pub enum DisplayMode { Windowed, Fullscreen }   // Borderless is added by PD-27 only (macOS)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(deny_unknown_fields)]
pub struct DisplayOptions { pub mode: DisplayMode, pub width: u32, pub height: u32 }
impl Default for DisplayOptions {    // D-PD14 (owner D3): windowed 1024x768 on every platform
    fn default() -> Self { Self { mode: DisplayMode::Windowed, width: 1024, height: 768 } }
}
pub const WIDTH: std::ops::RangeInclusive<u32> = 640..=7680;
pub const HEIGHT: std::ops::RangeInclusive<u32> = 480..=4320;
pub fn allowed_modes() -> &'static [DisplayMode];   // macOS: [Windowed] (+ Borderless after PD-27); other: [Windowed, Fullscreen] (D-PD15, D-PD21)
impl DisplayOptions {
    pub fn valid(self) -> bool;                    // mode in allowed_modes(), width in WIDTH, height in HEIGHT
    pub fn effective(self) -> DisplayOptions;      // macOS: mode forced to Windowed; size clamped into range
}
```

- There is no "Game setting" mode: the launcher is the source of truth on every launch (owner D3, D-PD21). An existing Windows player whose game was fullscreen gets a window at the first Play after the update; the settings note says so and Fullscreen is one choice away.
- `Preferences` gains `#[serde(default)] display: DisplayOptions` (and `StoredPreferences` the same field). `PREFERENCES_SCHEMA` stays 2, and an old file reads with the default. If `StoredPreferences` denies unknown fields, an older launcher can't read a file this build wrote; launcher downgrades are not supported, so that is accepted. Say in the report which case holds.
- `NativeCommand::SaveDisplayOptions { schema_version: u32, expected_revision: u64, display: DisplayOptions }`, the same admission as `SaveClientOptions` (`storage/mod.rs:399-460`: finalize summaries, updater idle, `Busy` while a non-Launch operation is open, `PersistenceUncertain` on `requires_reopen`, `StaleRevision`). An invalid value (`!valid()`) returns the new `StorageError::InvalidDisplayOptions` (`invalid_display_options`).
- `NativeSnapshot` gains `display_modes: Vec<DisplayMode>` (= `allowed_modes()`), and the snapshot's `preferences` carries `display`.
- Event `display_options_saved`, `launcher.settings`, INFO: `mode`, `width`, `height`.
- TS (`FE/src/contract.ts`, same commit): `DisplayMode = Schema.Literals(['windowed','fullscreen'])` (PD-27 adds `'borderless'`), `DisplayOptions = Schema.Struct({mode, width: Schema.Number, height: Schema.Number})`, `preferences.display: DisplayOptions`, `NativeSnapshot.display_modes: Schema.Array(DisplayMode)`, the `save_display_options` command in the `Command` union, `invalid_display_options` in `StorageCode`.

### C-PD7: what the launcher writes before each Play (PD-24, PD-25; D-PD16, D-PD21)

The client profile is `<Documents>/My Games/Firesky/SGWGame/`. Two files there carry the display mode (research-display-client.md § 1-3):

| `effective().mode` | `SavedSystemOptions.xml` | `Config/SGWEngine.ini` `[WinDrv.WindowsClient]` | Helper arguments (PD-25) |
|---|---|---|---|
| `windowed` | `windowedMode` → `valueBool="true"` | `StartupFullscreen=False`, `StartupResolutionX=<width>`, `StartupResolutionY=<height>` | `-windowed`, `ResX=<width>`, `ResY=<height>` |
| `fullscreen` (Windows only) | `windowedMode` → `valueBool="false"`; `resolution` untouched (it is a D3D mode index; D-PD21 keeps it so until RE confirms the mode-list order) | `StartupFullscreen=True`; size untouched | none |

The `windowedMode` line looks like this in the client's own file (CRLF):

```xml
<options valueBool="false" version="1" optionName="windowedMode" groupName="video"></options>
```

Hard rules (D-PD16):

- Never create either file and never add a missing key or line: a missing file or key is reported as `missing`/`key_missing` and skipped. (The lab seeds a full copy; a minimal file is unproven.)
- Never touch the `[IniVersion]` section, any other section, or the chain files under the game folder (`Working/Engine/Config/*.ini`): a mismatch regenerates the user ini.
- Keep the file's line endings, every other byte, and its encoding. A file that is not UTF-8 (for example a UTF-16 BOM) is skipped as `unreadable`.
- Write atomically (a temp file in the same folder, then rename). Never change permissions. Write only when the content changes.
- Best effort: a display write never fails the launch. It logs, then Play continues.

Event `display_settings`, target `launcher.launch`, INFO when nothing failed, WARN otherwise: `operation`, `mode`, `saved_options` and `engine_ini` (each `written`, `unchanged`, `missing`, `key_missing`, `unreadable` or `failed`), and `failure_code = "display_settings_not_applied"` only on WARN. No path in any field (R5).

## PD-22: macOS never changes the display mode (F20, D-PD15; wave 1)

- **Host:** mac (hands-on). **Mac gate:** yes (all code is `#[cfg(target_os = "macos")]`). **Agent:** packet-coder role (R1). **Worktree:** `pd-22`, branch `launcher-preflight-diagnostics/pd-22-mac-force-windowed`, from `origin/main`.
- **Why:** the pinned `d3d9.dll` is DXVK v1.10.3-async. Its D3D9 exclusive-fullscreen path changes the display mode, which winemac.drv turns into `CGCaptureAllDisplays()`: the shield window seen at layer 2147483630, the black screen on two displays and the Cmd+Tab freeze. `DXVK_FORCE_WINDOWED=1` makes DXVK force `Windowed=TRUE` in its present parameters, so no mode change and no capture happen, whatever the client's saved setting says (research-display-wine.md § 3). It also protects a first launch with no saved options, which PD-24 cannot (it never creates the file).
- **Files:** `E/storage/launch/wine.rs`, `E/storage/launch/wine_tests.rs`, `crates/launcher/desktop/shell/resources/graphics/README.md`, `docs/guides/macos.md`.
- **Do:**
  1. `wine.rs` `game_environment` (`:62-93`): after `DXVK_FRAME_RATE`, insert `DXVK_FORCE_WINDOWED` = `"1"` from a new `const FORCE_WINDOWED: &str = "1";` with a one-line comment: `// D2: exclusive fullscreen captures every display under winemac.drv; DXVK forces windowed.`
  2. `graphics/README.md`: the pinned `d3d9.dll` is DXVK v1.10.3-async (the macOS async fork), not D9VK; fix every "D9VK" mention there (F26). Don't change the binary, its hash or its pin.
  3. `docs/guides/macos.md` (CRLF): one sentence in the troubleshooting or graphics part: the launcher always starts the game in a window on a Mac, so Cmd+Tab works; exclusive fullscreen is not available.
- **Tests:** `wine_tests.rs`: with the existing `fixture()`, `game_environment(..)` contains `DXVK_FORCE_WINDOWED == "1"`. Revert proof: removing the insert fails it. **macOS only: MG.**
- **Checks:** D1, D2e (compiles nothing new on Windows; it must stay green), MG (`bash tools/build-lane/lane.sh cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine wine`).
- **Docs rows:** 57 (`docs/guides/macos.md`; `LD/launch.md` if it lists the Wine environment). UAT-PD7 is the acceptance test.
- **Commit:** `fix(launcher): never let the game change the Mac display mode (PD-22)`

## PD-23: the display preference and its command (C-PD6, D-PD14)

- **Host:** mac (hands-on). **Mac gate:** yes (the macOS default and `allowed_modes`). **Agent:** packet-coder role (R1). **Worktree:** `pd-23`, branch `launcher-preflight-diagnostics/pd-23-display-preference`, from `origin/main`.
- **Files:** `E/storage/display_options.rs` (new), `E/storage/display_options_tests.rs` (new), `E/storage/mod.rs` (543 lines: only the `mod` line, the enum variant line at `:70`, the field lines in `Preferences` and `StoredPreferences`, one line in each conversion and in `Default`, if they exist; anything more goes in `display_options.rs`; if that is impossible, return `blocked`), `E/commands.rs`, `E/storage/settings_events.rs`, `E/error_codes.rs`, `S/host/install/contract.rs` (`:133`, add the new variant next to `InvalidServerList` → `CorruptState`), `S/host/adoption/contract.rs` (`:132`), `E/storage/adoption/rejection.rs` (`:116` and its test row `:233`), `FE/src/contract.ts`, the contract test file that decodes `NativeSnapshot` today.
- **Do:**
  1. `display_options.rs`: C-PD6 types, `allowed_modes`, `valid`, `effective`, and `impl DesktopState { pub fn save_display_options(&mut self, display: DisplayOptions, expected_revision: u64) -> Result<Preferences, IntentError> }` that copies the admission steps of `save_client_options_with` in the same order and writes through the same `atomic::write(root, "preferences.json", ..)` path. Check `valid()` after the busy and revision checks.
  2. `storage/mod.rs`: `StorageError::InvalidDisplayOptions`; the `display` field lines.
  3. `error_codes.rs`: the code and its row in `codes_match_the_serde_names`.
  4. `commands.rs`: the `SaveDisplayOptions` variant (`schema_version != 1` → `UnsupportedSchema`, as the neighbours do), its dispatch arm calling `save_display_options` then `settings_events::display_options_saved`, and `display_modes` in `NativeSnapshot`.
  5. `settings_events.rs`: `display_options_saved(display: DisplayOptions)` per C-PD6, in the style of `client_options_saved`. If a golden test pins the `launcher.settings` event names or fields for forwarding, add the event there too; if that golden lives in admin-api, return `blocked` naming it.
  6. `FE/src/contract.ts`: every TS change in C-PD6, in the same commit (R8).
- **Tests:**
  - `display_options_tests.rs`: an old `preferences.json` without `display` reads as `DisplayOptions::default()`, which is `Windowed 1024x768` on every platform; a stored `"mode":"game"` (never shipped, but guard it) is rejected by `deny_unknown_fields`/the enum and the file falls back to the default the way other corrupt preference values do (say in the report which path it takes); `save_display_options` with `Windowed 1600x900` round-trips through a reopen; `width 320` → `InvalidDisplayOptions`; a stale revision → `StaleRevision`; while an Install operation is open → `Busy`; `effective()` of `Fullscreen` on macOS is `Windowed` (`#[cfg(target_os = "macos")]`) and stays `Fullscreen` elsewhere (`#[cfg(not(target_os = "macos"))]`); `allowed_modes()` per platform the same way. Revert proof: dropping the `valid()` check makes the 320 case save.
  - `commands.rs` tests: a `save_display_options` command emits INFO `launcher.settings` `event=display_options_saved mode=windowed width=1600 height=900`.
  - TS: a snapshot with `display` and `display_modes` decodes; one without `display_modes` is rejected.
- **Checks:** D1, D2e, D2s, D3e `display_options`, D3e `commands`, D3e `error_codes`, D3s `contract`, F1, MG.
- **Docs rows:** 57 (`LD/` settings or contract doc that lists `NativeCommand` variants, if any), 34 (`display_options_saved`).
- **Commit:** `feat(launcher): a display mode preference the launcher owns (PD-23)`

## PD-24: apply the display mode to the client profile before each Play (C-PD7, D-PD16)

- **Host:** mac (hands-on). **Mac gate:** yes (the Wine profile branch). **Agent:** packet-coder role (R1). **Worktree:** `pd-24`, branch `launcher-preflight-diagnostics/pd-24-apply-display`, from the PD-23 branch tip.
- **Files:** `E/storage/launch/display.rs` (new), `E/storage/launch/display_tests.rs` (new), `E/storage/launch/mod.rs` (`mod` line), `E/storage/launch/worker.rs` (call lines only; it is 429 lines and must stay under 450), `E/storage/launch/wine.rs` (call lines), `E/storage/launch/events.rs` (one fn), `LD/launch.md`.
- **Do:**
  1. `display.rs`, pure part:
     - `pub(crate) fn saved_options(text: &str, windowed: bool) -> Option<String>`: find the one line containing both `optionName="windowedMode"` and `groupName="video"`; set its `valueBool` attribute to `"true"`/`"false"`; return the whole text with every other byte unchanged. No such line, or two of them → `None`.
     - `pub(crate) fn engine_ini(text: &str, fullscreen: bool, size: Option<(u32, u32)>) -> Option<String>`: inside the `[WinDrv.WindowsClient]` section only (from that header to the next `[` header), set `StartupFullscreen=True|False` and, when `size` is some, `StartupResolutionX=<w>` and `StartupResolutionY=<h>`. Match keys case-insensitively and keep the key's original spelling; drop any trailing whitespace on the replaced value (the chain file has a trailing tab). A missing section or key → `None`. Keep `\r\n` or `\n` as found.
     - `pub(crate) fn arguments(display: DisplayOptions) -> Vec<String>`: per C-PD7 (PD-25 uses it; build it here so the table lives in one file).
  2. `display.rs`, effectful part: `pub(crate) fn apply(operation: Uuid, profile: Option<&Path>, display: DisplayOptions)`. `display.effective()`. Read, transform and atomically write each file per C-PD7 (`std::fs::read`, `String::from_utf8`, a `tempfile::NamedTempFile::new_in(<same folder>)` then `persist`). Emit `events::display_settings(..)` once (C-PD7). `profile = None` → both `missing`.
  3. Profile folders: Windows (`cfg(not(target_os = "macos"))`): `cimmeria_launcher_core::client_paths::firesky_root().map(|r| r.join("SGWGame"))`. macOS: `pub(crate) fn wine_profile(prefix: &Path) -> Option<PathBuf>`: the only directory in `prefix/drive_c/users` other than `Public`, then `Documents/My Games/Firesky/SGWGame`; canonicalize it and require that it stays inside `prefix` (reuse `preparation::contained`); zero or several user folders, or a path outside → `None`.
  4. Read the display options where `execute` reads the login servers under the state lock (`worker.rs` around `:170-178`) and pass them to `prepare` (`:322-355`). Call `display::apply` in `prepare_native` just before the `HelperCommand` is built, and in `wine::prepare` just before its `Request` is built (with `wine_profile(&resources.prefix)`). If threading the options through `prepare` would push `worker.rs` past 450 lines, put the read in a helper in `display.rs`.
  5. `events.rs`: `display_settings(operation, mode, saved_options, engine_ini)` per C-PD7.
  6. `LD/launch.md`: the display step, the table in C-PD7 and the rule that an in-game display change is always replaced at the next Play (owner D3).
- **Tests (`display_tests.rs`, cross-platform):** `saved_options` on a 4-line CRLF sample flips only the `windowedMode` value and keeps CRLF and every other line byte for byte; a sample without the line → `None`; `engine_ini` on a sample with `[IniVersion]`, `[WinDrv.WindowsClient]` and a second section that also has a `StartupFullscreen` key changes only the WindowsClient keys, keeps `[IniVersion]` byte for byte, and strips the trailing tab from `StartupResolutionX=1024\t`; `arguments` for each mode; `apply` on a temp profile: windowed writes both files, a second `apply` reports `unchanged` and doesn't touch the mtime, a missing ini reports `missing` and creates nothing, the default options (no saved preference) write windowed 1024x768; the event fields are pinned with `test_capture`. macOS-only test: `wine_profile` finds `users/<name>/Documents/...` and ignores `Public`. Revert proof: removing the section bound in `engine_ini` makes the second-section test change the wrong key.
- **Checks:** D1, D2e, D3e `display`, D3e `launch`, MG, WG (the Windows profile path is `cfg(not(macos))`), then UAT-PD9 on a Windows PC.
- **Docs rows:** 57 (`LD/launch.md`), 34 (`display_settings`).
- **Commit:** `feat(launcher): apply the chosen display mode to the game profile before each Play (PD-24)`

## PD-25: the launch helper passes display arguments to SGW.exe (C-PD7)

- **Host:** mac (hands-on) for the engine half; the helper is an i686 Windows binary verified by CI (RG, WR). **Mac gate:** yes (`wine.rs` builds a `Request` too). **Agent:** packet-coder role (R1). **Worktree:** `pd-25`, branch `launcher-preflight-diagnostics/pd-25-helper-args`, from the PD-24 branch tip.
- **Files:** `RP/game_launch/mod.rs`, `RP/game_launch/windows.rs`, the `game_launch` test file next to them (create `RP/game_launch/args_tests.rs` if there is none), `E/storage/launch/worker.rs` (the `Request` literal in `prepare_native`, `:400-418`), `E/storage/launch/wine.rs` (the `Request` literal), `LD/launch.md`.
- **Do:**
  1. `Request` (`RP/game_launch/mod.rs`, `deny_unknown_fields`): add `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub args: Vec<String>`. `schema_version` stays 1: an empty list serializes exactly as today (R10), and a new launcher never sends args to an old helper because the helper ships inside the same app bundle.
  2. `pub fn valid_args(args: &[String]) -> bool` in `mod.rs` (pure, cross-platform): at most 3 entries; each is exactly `-windowed`, or `ResX=` followed by 3-4 ASCII digits, or `ResY=` followed by 3-4 ASCII digits; no entry repeated. Call it where the request is validated today; an invalid list is refused exactly the way an invalid `exe` is (the same error path and event).
  3. `windows.rs:68`: replace `create_process_suspended(&exe, Some(&directory))` with `cimmeria_client_launch::process::create_process_suspended_with_args(&exe, &args_as_os_strings, Some(&directory))` (`crates/client-launch/src/process.rs:333`); check the crate is already a dependency of runtime-probe, else return `blocked` (no new dependency). With an empty list the command line must equal today's; check that `create_process_suspended` is that function with no args, and say so in the report.
  4. Both `Request` literals set `args: display::arguments(display)` (PD-24), using the options PD-24 already passes into `prepare`.
  5. `LD/launch.md`: the helper accepts the three display arguments and refuses anything else.
- **Tests:** `valid_args`: `[]`, `["-windowed","ResX=1280","ResY=720"]` true; `["-windowed","-windowed"]`, `["ResX=12800"]`, `["-log"]`, `["ResX=12a4"]`, four entries false. Serde: a request JSON without `args` decodes with an empty list; a request with `args: []` serializes without the key. Engine: `prepare_native` for a windowed preference puts the three arguments in the request (use the existing `worker_tests.rs` prepare fixture; if none reaches `prepare_native`, test `display::arguments` only and say so). Revert proof: removing the digit-count check passes `ResX=12800`.
- **Checks:** D1, D2e, D3e `launch`, MG, RG and WR (the helper is i686 Windows code; D2p/D3p run on the Mac only if the crate builds there, else say so).
- **Docs rows:** 57 (`LD/launch.md`).
- **Commit:** `feat(launcher): start the game windowed with launch arguments when the player chose Windowed (PD-25)`

## PD-26: the display mode setting in the gear (requirement: Settings option, D2)

- **Host:** mac (hands-on; frontend only, Node runs F1). **Agent:** packet-coder role (R1). **Worktree:** `pd-26`, branch `launcher-preflight-diagnostics/pd-26-display-setting`, from the PD-23 branch tip.
- **Files:** `FE/src/display-view.ts` (new), `FE/src/display-view.test.ts` (new), `FE/src/composition.ts`, `FE/ui/index.html` (one `<section id="display-panel">` in `#gear`, after `#client-maintenance-panel`), `FE/package.json`.
- **Do:**
  1. Mirror how the login-server setting saves today (find the `save_client_options` caller): same command plumbing, same `expected_revision` handling, same stale-revision recovery.
  2. Controls: `<select id="display-mode">` with one option per `snapshot.display_modes` entry; labels `windowed` → `Windowed`, `fullscreen` → `Fullscreen (uses the resolution chosen in the game)`. `<select id="display-size">` with `1024 × 768`, `1280 × 720`, `1600 × 900`, `1920 × 1080`, `2560 × 1440`, plus the saved size when it is none of these; shown only for `windowed`. (PD-34 later replaces this fixed list with the sizes that fit the chosen display.) `<button id="display-save">Save</button>`.
  3. Copy under the controls: `The launcher applies this every time you press Play, so a display change made inside the game is replaced at the next start.` On Windows only, a second line: `New: the game now starts in a 1024 × 768 window by default. Choose Fullscreen here to go back.` When `display_modes` is only `windowed` (a Mac), the mode select is replaced by the text `On a Mac the game always runs in a window, so Cmd+Tab works.` and only the size select remains.
  4. Save: the first press disables the button and shows `Saving…`; on success `Saved. Applies the next time you press Play.`; on `invalid_display_options` the C-PD1-style text `That size can't be used. Pick one from the list.`; on any other error the mapped `failureText` (PD-04).
- **Tests (`display-view.test.ts`, linkedom):** a Windows snapshot (`display_modes` of two) renders two options and the Windows note with the saved mode selected; choosing `fullscreen` hides the size select; a Mac snapshot renders no mode select and the Mac line; pressing Save sends one `save_display_options` with the chosen mode and size and shows `Saving…` at once; a stale-revision reply follows the login-server recovery path. Revert proof: dropping the Mac branch fails the Mac test.
- **Checks:** F1.
- **Docs rows:** none here (PD-18 documents the setting in the player guides).
- **Commit:** `feat(launcher): a display mode setting in Settings (PD-26)`

## PD-27: Borderless on macOS (D-PD18; wave 4)

- **Status:** BlockedDependency on UAT-PD8. Write the packet spec after the probe, as a re-spec in `worknotes/PD-27.md`. Expected shape (research-display-wine.md § 1, § 4): `DisplayMode::Borderless` on macOS only; the client set to its "fullscreen" path (`windowedMode=false`) with `DXVK_FORCE_WINDOWED=1` still set, and the frame sized to the main display's size in points (`CGDisplayBounds`), which winemac.drv shows as a frameless window above the menu bar, with no capture. The open question UAT-PD8 must answer: which size the client gives that window when its fullscreen size is a D3D mode index (research-display-client.md § 3), and whether `ResX=`/`ResY=` change it.
- **Host:** mac (hands-on), MG; the probe needs a Mac. The frame size comes from PD-31's `DisplayInfo` for the chosen monitor (points), not a fresh `CGDisplayBounds` call.

## PD-28: per-launch Wine and DXVK logs, kept locally (F24, D-PD17, owner D4 (4); wave M)

- **Host:** mac (hands-on). **Mac gate:** yes. **Agent:** packet-coder role (R1). **Worktree:** `pd-28`, branch `launcher-preflight-diagnostics/pd-28-wine-logs`, from the wave M integration tip (`pd-wm`).
- **Brief input:** first check whether the log-upload bundle (`S/host/log_upload.rs` and the engine code it calls) collects every file under `state/logs/` recursively. If it does, return `blocked` (D-PD17 says these logs stay local; uploading them needs an owner decision because they contain local paths).
- **Files:** `E/storage/launch/game_logs.rs` (new), `E/storage/launch/game_logs_tests.rs` (new), `E/storage/launch/mod.rs` (`mod` line), `E/storage/launch/wine.rs`, `E/storage/launch/supervisor.rs`, `E/storage/launch/supervisor_tests.rs` (call sites), `E/helper_supervisor.rs` or wherever `HelperCommand` is defined (one optional field), `LD/launch.md`, `docs/guides/macos.md`.
- **Do:**
  1. `game_logs.rs`: `pub(crate) fn folder(state_root: &Path, operation: Uuid) -> std::io::Result<PathBuf>` creates `state_root/logs/game/<operation>/`, then keeps only the 5 newest operation folders in `logs/game/` (by folder mtime; delete older ones; ignore delete errors). `pub(crate) const STDERR_CAP: u64 = 4 * 1024 * 1024;`.
  2. `HelperCommand` gains `pub stderr_log: Option<PathBuf>` (`None` everywhere except the Wine game launch). In `supervisor.rs:57-59`, when it is some, open the file (create, truncate) and pass it as `stderr`, through a small copy task that stops writing after `STDERR_CAP` bytes (pipe stderr and copy in a `tokio::spawn`; never block the helper on a full pipe: keep reading and discard after the cap). When it is `None`, keep `Stdio::null()` (R10: Windows unchanged).
  3. `wine.rs` `game_environment`: replace `WINEDEBUG` with `err+all,fixme-all,+display` (owner D4 (4); the `display` channel is winemac.drv's display code, research-mac-extras.md § 5); set `DXVK_LOG_LEVEL=info` and `DXVK_LOG_PATH` = the guest path of the folder (`mac_wine::paths::guest`, as `wine.rs` does for `exe`); in `prepare`, set `stderr_log = Some(folder.join("wine.log"))`. If creating the folder fails, launch without logs (both `None`, `WINEDEBUG` back to `-all`) and log nothing new.
  4. Docs: `LD/launch.md` (where the per-launch logs are, that they stay on the computer, the 5-folder limit); `docs/guides/macos.md` troubleshooting (CRLF): where to find `wine.log` and the DXVK logs after a black screen or crash, under the launcher's state folder.
- **Tests:** `game_logs_tests.rs`: with 7 older folders, after `folder()` exactly 5 folders remain: the new one and the 4 newest older ones. `supervisor_tests.rs`: a helper fixture that writes 5 MiB to stderr leaves a log of exactly `STDERR_CAP` bytes and still exits normally (Windows host: the test fixture helper must exist on Windows; if the existing fixture can't write to stderr, add the case to the macOS-only tests and say so). macOS: `game_environment` has `WINEDEBUG=err+all,fixme-all,+display` and a `DXVK_LOG_PATH` starting with `Z:\` or the guest form `guest()` returns. Revert proof: removing the cap makes the size assertion fail.
- **Checks:** D1, D2e, D3e `game_logs`, D3e `supervisor`, MG.
- **Docs rows:** 57 (`LD/launch.md`, `docs/guides/macos.md`). Privacy: these files are local only; `LD/privacy-inventory.md` gets no row unless the upload check above says otherwise (then this packet is blocked anyway).
- **Commit:** `feat(launcher): keep Wine and DXVK logs for each Mac game launch (PD-28)`

## PD-29: a running game with no visible window is reported (D2; Windows probe; wave 4)

- **Host:** mac (hands-on). **Windows gate:** WG, plus UAT-PD13 on a Windows PC (the probe is `cfg(windows)`; a Mac can only run `decide` and the FE tests). **Agent:** packet-coder role. **Worktree:** `pd-29`, branch `launcher-preflight-diagnostics/pd-29-hidden-window`, from `origin/main` after wave 1 and wave D have merged.
- **Files:** `E/storage/launch/window_check.rs` (new), `E/storage/launch/window_check_tests.rs` (new), `E/storage/launch/mod.rs` (`mod` line), `E/storage/launch/failure.rs` (PD-05; add `clear_launch_failure`), `E/storage/launch/worker.rs` (call lines), `E/storage/launch/events.rs` (one fn), `FE/src/launch-view.ts`, `FE/src/launch-view.test.ts`, `LD/launch.md`.
- **Do:**
  1. `window_check.rs`: `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum WindowState { Visible, Hidden, Unknown }`; `pub(crate) fn decide(samples: &[WindowState]) -> Option<bool>`: `Some(true)` (hidden) when the last 3 samples are all `Hidden`; `Some(false)` when the last sample is `Visible`; otherwise `None`. Pure.
  2. Windows probe, `#[cfg(windows)] pub(crate) fn probe(pid: u32) -> WindowState`: `EnumWindows` over top-level windows; a window whose `GetWindowThreadProcessId` is `pid` and `IsWindowVisible` is true, not minimized (`IsIconic` false), with a non-empty `GetWindowRect` → `Visible`; process windows but none visible → `Hidden`; no windows yet → `Hidden`; an API failure → `Unknown`. (`windows-sys` already has `Win32_UI_WindowsAndMessaging`.) Other platforms: `Unknown` (PD-30 adds macOS).
  3. Watch: after the supervisor reports `ProcessStarted { guest_pid }`, sample every 5 s starting 20 s after start, until 120 s after start or the process ends, in a task that the existing cancel token stops. On `Some(true)`: `note_launch_failure(plan, game_window_hidden)` and `events::launch_window_hidden(id)` (WARN, `launcher.launch`, `event=game_window_hidden`, `operation`, `failure_code`, `waited_ms`). On a later `Some(false)`: `clear_launch_failure(plan, game_window_hidden)` (it clears only that code) and `events::launch_window_visible(id)` (INFO). A minimized window counts as Hidden only before the first Visible sample; after one Visible sample, stop watching.
  4. `launch-view.ts`: while the game is running and `status.failure == "game_window_hidden"`, show the failure message and fix (C-PD1) under the running line; once the game exits, show it only if the exit was not code 0.
- **Tests:** `decide` table; `clear_launch_failure` clears `game_window_hidden` but keeps `spawn_failed`; a `#[cfg(windows)]` test that `probe(std::process::id())` on the test process (no windows) is `Hidden`; FE: running + `game_window_hidden` shows the fix; exited 0 + `game_window_hidden` doesn't. Revert proof: making `decide` act on one Hidden sample fails the "two Hidden then Visible" row.
- **Checks:** D1, D2e, D3e `window_check`, D3e `launch`, F1, WG; then UAT-PD13.
- **Docs rows:** 57 (`LD/launch.md`), 34 (the two events).
- **Commit:** `feat(launcher): say so when the game runs with no visible window (PD-29)`

## PD-30: the macOS window probe (D2; wave 4)

- **Host:** mac (hands-on), **Mac gate** and a live Mac check. **Agent:** the hands-on session itself (it needs a live window check). **Worktree:** `pd-30`, branch `launcher-preflight-diagnostics/pd-30-mac-window-probe`, from the PD-29 tip.
- **Files:** `E/storage/launch/window_check_mac.rs` (new), its `_tests.rs`, `E/storage/launch/window_check.rs` (the `cfg` dispatch line).
- **Do:**
  1. Verify on a Mac first, with the game running windowed: which `kCGWindowOwnerName` and `kCGWindowOwnerPID` the SGW window has in `CGWindowListCopyWindowInfo(kCGWindowListOptionAll, kCGNullWindowID)`. The guest pid the helper reports is a Windows pid and won't match. If the window can't be told apart from other Wine windows by owner name `SGW.exe` (or by the pid of the process the launcher's helper started), stop and return `blocked` with what you saw.
  2. `probe(..) -> WindowState`: a matching window with `kCGWindowIsOnscreen` true, layer below `CGShieldingWindowLevel()`, non-empty bounds → `Visible`; a matching window at or above the shield level, or not on screen → `Hidden`; none → `Hidden`; an API failure → `Unknown`. Use raw FFI to CoreGraphics and CoreFoundation (`#[link(name = "CoreGraphics", kind = "framework")]`); no new crate. Free every CF object you create or copy.
  3. The pure part (`classify(entries: &[WindowEntry], shield_level: i32) -> WindowState` with `WindowEntry { owner_matches, onscreen, layer, width, height }`) carries the tests, so they run on any host.
- **Tests:** `classify` table (onscreen layer 0 → Visible; layer 2147483630 → Hidden; offscreen → Hidden; no entries → Hidden). A `#[cfg(target_os = "macos")]` smoke test that `probe` returns without panicking. Revert proof: dropping the shield-level rule makes the 2147483630 row `Visible`.
- **Checks:** MG (D1, D2e, D3e `window_check` through `lane.sh`), WG.
- **Docs rows:** 57 (`LD/launch.md`: the macOS rule).
- **Commit:** `feat(launcher): detect a hidden or captured game window on macOS (PD-30)`
