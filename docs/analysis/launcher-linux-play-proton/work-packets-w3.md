# Work packets: launcher-linux-play-proton (waves 3-4)

Ledger: [README.md](README.md). Rules R1-R9, the contract C-1..C-6 and
§ Dispatch: [work-packets.md](work-packets.md). Read them first; they apply
here unchanged. `D/`, `E/`, `S/` as defined there. **Host: linux** throughout.

## Wave 3

Released when wave 2 is merged into the integration branch. PP-10, PP-11 and
PP-17 can run in parallel (PP-17 also needs `worknotes/PP-S2-ruling.md`);
PP-12 needs PP-10; PP-13 goes last. Merge order: PP-10, PP-11, PP-17, PP-12, PP-13.

### PP-10: Proton preference and the `proton_command` IPC

**Agent:** packet-coder. **Depends:** PP-09. **Commit:** `feat(launcher): choose which Proton plays the game (PP-10)`.

Files: the engine's launcher preferences store (find it with
`grep -rn "game_telemetry_opt_in\|struct .*Preferences" crates/launcher/desktop/engine/src/storage`
and name it in the report), new `S/host/proton_command.rs` and
`S/host/proton_command_tests.rs`, `S/host.rs` (module line), `S/main.rs`
(register the command beside the existing launch commands only),
`S/host/launch/resources.rs` (pass the saved preference instead of `None`).

1. Preference: `proton_preference: Option<String>` (a C-1 id), serde
   `default, skip_serializing_if = "Option::is_none"` so old files still load.
2. `#[tauri::command] proton_command(request: ProtonCommand) -> Result<ProtonView, String>`:
   ```rust
   #[derive(Deserialize)] #[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
   pub enum ProtonCommand { Inspect, Select { id: Option<String> } }
   #[derive(Serialize)]
   pub struct ProtonView { pub available: Vec<ProtonChoice>, pub selected: Option<String>, pub preferred: Option<String> }
   #[derive(Serialize)]
   pub struct ProtonChoice { pub id: String, pub name: String, pub kind: ProtonKind }
   ```
   No path, root or version string crosses IPC (R9). `Select` with an id not
   in `discover()` is refused with the string `unknown_proton`; `None` clears
   the preference. `selected` = `choose(found, preferred)`'s id.
   On a non-Linux build the command returns an empty view (no error), so the
   frontend can call it everywhere.
3. A selection change is seen by the next status refresh (PP-09 rediscovers
   on every status call; no cache).

Tests: `Inspect` on a fake Steam tree lists ids in rank order with no path
in the serialized JSON (assert the JSON has no `/`); `Select` of an unknown
id is refused and the preference is unchanged; `Select{None}` clears it; an
old preferences file without the field loads.

Lane (R1): engine and shell fmt, clippy, `test --lib proton`.

### PP-11: Linux telemetry tags and case-safe paths

**Agent:** packet-coder. **Depends:** PP-07. **Commit:** `feat(launcher): Proton game sessions report as linux/proton (PP-11)`.
The display part moved to PP-17 (PP-S1 ruling).

Files: `E/storage/launch/telemetry.rs`,
`S/host/game_telemetry.rs` (lines 44-45 only), their test files.

1. Tags (D-PP11): a Proton launch tags its game session
   `["desktop-launcher","linux","proton"]`. Leave the launcher session's
   tags (`E/storage/game_telemetry/session.rs:296`) alone.
2. Marker and log tail (D-PP12): the telemetry marker written beside
   `SGW.exe` and the `cimmeria-client-telemetry.log` tail read after exit use
   the binaries folder that `resolve_existing_case` returned, never a literal
   `Working/Binaries` join.

Tests: tags for a Proton plan; the marker lands in a lower-case
`Working/binaries` folder that exists on disk (fails when reverted to a literal
join).

Lane (R1): engine fmt, clippy, `test --lib telemetry`; shell fmt, clippy.

### PP-17: Linux display profile in the Proton prefix

**Agent:** packet-coder. **Host:** linux. **Depends:** PP-07 merged and
`worknotes/PP-S2-ruling.md` (step 3 below). **Branch:** `pp-17`.
**Commit:** `feat(launcher): Proton Play writes the windowed client profile into the prefix (PP-17)`.

Why (PP-S1 ruling, D-PP22): with the game's own profile (`windowedMode`
false) the window stays black until a focus change; with the launcher's
windowed profile it draws at once. Today Linux takes the Windows branch and
looks in the real Documents folder.

Files: `E/storage/launch/display.rs` (232; the `native_profile` cfg pair at
136-145 and a new `proton_profile` beside `wine_profile` 150-169),
`E/storage/display_options.rs` (290; `allowed_modes` 73-79 and `effective`
116-128 only), `E/storage/launch/proton.rs` (one call),
`E/storage/launch/events.rs` (line 251 only), new
`E/storage/launch/display_proton_tests.rs` (wired from `display.rs` with
`#[cfg(test)] #[path = "display_proton_tests.rs"] mod proton_tests;`), and
`display_options`' test file.

1. `display.rs`: Linux joins the `native_profile() -> None` arm (change the
   cfg at 136 to `all(not(target_os = "macos"), not(target_os = "linux"), not(test))`
   and at 142 to `any(target_os = "macos", target_os = "linux", test)`). Add
   `#[cfg(any(target_os = "linux", test))] pub(super) fn proton_profile(prefix: &Prefix) -> PathBuf`
   = `prefix.profile_root().join("Documents/My Games/Firesky/SGWGame")`.
2. `display_options.rs` (D-PP13 as amended): Linux is **windowed only**, as
   macOS: change the two `cfg!(target_os = "macos")` tests at 73-79 and
   116-128 to `cfg!(any(target_os = "macos", target_os = "linux"))`. The
   macOS result is unchanged. The frame cap list stays as it is.
3. First Play (PP-S2 ruling, D-PP27). The game writes **no**
   `SavedSystemOptions.xml` on its first run, so the window stays black
   until a touch. A seeded file with only `windowedMode` true draws, and the
   game leaves it byte-identical. In `display.rs`:
   - `FileOutcome` gains `Seeded`, with code `"seeded"`.
   - Add the exact seed (441 bytes, one CRLF after the declaration, no final
     newline):
     ```rust
     /// The first Linux Play's saved options (PP-S2 B2): the game writes none itself.
     const SEED_SAVED_OPTIONS: &str = concat!(
         "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n",
         "<savedOptions xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
         "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
         "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
         "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" ",
         "xmlns:SGWSystemOptions=\"http://www.cheyenneme.com/sgw/options\">",
         "<options valueBool=\"true\" version=\"1\" optionName=\"windowedMode\" groupName=\"video\"></options>",
         "</savedOptions>",
     );
     ```
   - Add `#[cfg(any(target_os = "linux", test))] pub(crate) fn apply_proton(operation: Uuid, pfx: &Path, profile: &Path, display: DisplayOptions)`.
     It sets `display = display.effective()`; `fullscreen` and `size` as in
     `apply`. Then:
     - If `profile.join(SAVED_OPTIONS)` exists (`symlink_metadata` is `Ok`),
       `saved = update(..)` exactly as `apply` does.
     - If it does not exist (`NotFound`), seed it. Every existing ancestor of
       `profile` up to and including `pfx` must not be a symlink
       (`symlink_metadata().file_type().is_symlink()`); if one is, `saved = Failed`
       and nothing is written. Then `create_dir_all(profile)`, and the text
       is `saved_options(SEED_SAVED_OPTIONS, !fullscreen)`. Write it with
       `tempfile::NamedTempFile::new_in(profile)`, `write_all`, `sync_all`,
       then `persist_noclobber` to `SAVED_OPTIONS`, so a file the game made
       meanwhile is never overwritten. Success is `Seeded`, and any I/O
       error is `Failed`.
     - Any other `symlink_metadata` error is `Unreadable`.
     - `ini = update(&profile.join(ENGINE_INI), ..)` as `apply` does. Never
       create `SGWEngine.ini`. The game regenerates it at 1024x768, so the
       first Play runs at that size and the chosen size applies from the
       second Play (FU-10).
     - End with the same five lines as `apply` (188-192): the mode code,
       then `events::display_settings`. Copy them; do not refactor `apply`.
   - `events.rs` line 251: `matches!(code, "written" | "unchanged" | "seeded")`.

   In `proton.rs` `prepare`, after DirectX and PhysX, call
   `display::apply_proton(operation, &prefix.pfx, &proton_profile(&prefix), display)`
   and pass `display::arguments(display)` as the game's `args`. `wine.rs:56-60`
   is the model. The display step never fails a launch.

Tests (unit, TempDir; the new ones go in `display_proton_tests.rs`):
- On Linux, `allowed_modes` is `[Windowed]` and `effective` forces windowed.
- `proton_profile` is under `pfx/drive_c/users/steamuser`.
- In a pfx with only `drive_c/users/steamuser/Documents`, `apply_proton`
  creates `…/Firesky/SGWGame/SavedSystemOptions.xml`, exactly 441 bytes and
  byte-equal to `SEED_SAVED_OPTIONS`. The event has
  `saved_options="seeded"` and `engine_ini="missing"`. This fails if the
  seed branch is removed.
- An existing file with `windowedMode` false is rewritten to true and not
  replaced by the seed. This fails if the seed overwrites.
- `#[cfg(unix)]`: when `Documents` is a symlink to another TempDir, nothing
  is written in either folder and `saved_options="failed"`. This fails if
  the symlink check is removed.
- `saved_options(SEED_SAVED_OPTIONS, true)` returns the seed unchanged, so
  later Plays can edit it.
- `prepare` with an existing profile in a TempDir prefix rewrites
  `windowedMode` to true. This fails if the `prepare` call is removed.

Name the reverting line for each.

Lane (R1): engine fmt, clippy, `test --lib display`, `test --lib launch`.

### PP-12: Frontend: Linux Play copy, Proton picker and REPL UAT

**Agent:** packet-coder. **Depends:** PP-10. **Commit:** `feat(launcher): Linux Play copy and a Proton picker in Settings (PP-12)`.

Files: `D/frontend/src/launch-view.ts` (lines 37, 49 and 51 copy only),
`D/frontend/src/launch-workflow.ts` (line 9: add `'proton'` to the
`prerequisites_missing` literals only), the Settings view file that hosts the display options (find with
`grep -rln "display" D/frontend/src/*settings*`), new
`D/frontend/src/proton-picker.ts`, its test file beside the existing
frontend tests, new `D/frontend/proton-uat.mjs`, `D/frontend/package.json`
(add `"uat:proton": "node proton-uat.mjs"`).

1. Copy: when `prerequisites_missing` holds `"proton"`, the Play panel says
   "Install Proton to play. In Steam, install Proton Experimental, or install
   GE-Proton with ProtonUp-Qt. Then reopen the launcher." Remove the line-51
   text that says Play is not available on Linux. Line 49 names `proton` as
   "Proton", never "PhysX". Line 37 (the Play-refusal text) names only the
   missing pieces, Proton included. A status carrying `"proton"` decodes; it
   is never a `schema` failure.
2. Picker (Settings, Linux only, hidden when `available` is empty): a select
   of `name` values; changing it calls `proton_command {op:"select", id}`;
   "Automatic" sends `id: null`. Show "Saved." on success and the refusal
   text on error, on the first change (project rule: visible feedback).
3. `proton-uat.mjs`: a REPL-style logic UAT modelled on `launch-uat.mjs`
   with a fake IPC: (a) no Proton → Play disabled and the install copy
   shown and the status decodes without a `schema` failure; (b) two Protons → picker lists both in order, select → "Saved.";
   (c) unknown id refused → error text, selection unchanged;
   (d) `proton_missing` and `proton_setup_failed` failures render their C-6
   titles. Print one PASS/FAIL line per step; exit 1 on any FAIL.

Checks (R2): `npm run check`, `npm test`, `npm run uat:launch`,
`npm run uat:proton`; paste the UAT output in the report.

### PP-13: Docs

**Agent:** documentation-writer. **Depends:** PP-03, PP-09..PP-12, PP-15..PP-17 and wave 1B merged; the
sibling campaign's final PR merged (ask the orchestrator; if not, do every
row except 5 and report `blocked` on that row only). **Commit:** `docs(launcher): Linux Play through Proton (PP-13)`.

All files CRLF (R6). Walk `docs/agents/doc-update-map.md` and list the rows in
the report.

1. `D/docs/launch.md`: a "Linux (Proton)" section: discovery and ranking
   (D-PP4), the private prefix and its stamp (D-PP5), the invocation (bare
   `proton runinprefix`, setup `proton run wineboot.exe`: PP-S1 ruling),
   the native `d3dx9_40`/`xinput1_3` from the game's cabs with their pins
   and overrides (D-PP20, D-PP21, C-8), PhysX (D-PP7), the windowed-only
   prefix profile and its first-Play seed (D-PP13, D-PP22, D-PP27; the first
   Play runs at 1024x768, FU-10; `display_settings` gains
   `saved_options="seeded"` in `D/docs/launch.md` § Display settings), the environment allow-list (C-4), the two
   failure codes (C-6), the tags (D-PP11); add Linux rows to the human
   checklist and to the Desktop Play control pins table.
   Also in `D/docs/launch.md` § Display settings (about 396-433): the Linux
   profile root.
2. `D/README.md` lines 63-89: replace "Not on Linux yet" with what Linux
   Play needs (an installed Proton) and what it still lacks (Repair, Update,
   self-update: D-PP18).
3. `docs/client/launcher-guide.md` lines 133-151: a Steam Deck "Play" step
   list; delete "Proton comes later" (line 151).
4. `D/docs/release.md`: check PP-03's section reads right; fix wording only.
5. Sibling ledger `docs/analysis/launcher-linux-install-telemetry/README.md`
   line 139 (row U1): reconcile with what main now does (Linux installs and,
   after this campaign, plays) and link this ledger.
6. `docs/architecture/` or `docs/agents/` pages that say the Linux launcher
   cannot play (`grep -rln "platform_unavailable\|not on Linux" docs D/docs`):
   fix each hit, list it.
7. Wave 1B: the install phases and their copy (C-7, PP-20/PP-21), the
   archive flow's game-folder line (PP-23), the new `launcher.install` and
   `log_upload_post` lines (PP-22, PP-24) wherever the launcher's log events
   are listed (`grep -rln "seed_archive_started\|log_upload_finished" docs D/docs`).
   PP-25 documents its own README sentence.

Check: `python tools/docs-gen/regen.py --check` if the touched pages carry
generated blocks; never hand-edit a `gen:` block.

## Wave 4

### PP-U: Deck UAT (owner + hands-on)

**Agent:** `claude --agent hands-on` with the owner, on a launcher built from
the integration branch (`ops` runs the Linux AppImage job on it). Rows: README
§ UAT (U1-U8). Record each as PASS/FAIL with evidence (screenshot path, the
`logs/game/<op>/` file, the SigNoz query for the telemetry row) in
`worknotes/PP-U.md` (CRLF). U6 (Game Mode) gates the merge only if the owner
rules so on OD-2. A FAIL goes back to the architect for a ruling.
The Play rows also cover PP-09's Linux playable arm and the per-call Proton
selection, which have no unit test (PP-09-ruling-1).

### PP-14: Close-out

**Agent:** packet-coder (docs only). **Depends:** PP-U passed and the campaign
review `ship`. **Commit:** `docs: close out launcher-linux-play-proton (PP-14)`.

1. `docs/project-status.md`: one update for the whole campaign (project rule:
   status docs change once per campaign). Linux launcher: Play through Proton.
2. `docs/gap-analysis*`: only the launcher row, if one exists.
3. Ledger: every packet row `Done` with its merge commit; RESUME.md to its
   final state; follow-ups FU-1..FU-9 left as open items for the owner.
4. Architect memory: `.claude/agent-memory/architect/` note "Proton
   `runinprefix` keeps stdio; `run` goes through steam.exe; PROTON_LOG
   redirects stdout" with the PP-S1 evidence row (no paths with user names).

Then `ops` opens the one final PR from the integration branch (D-PP17) after
`full-ci` (Windows and macOS jobs) passes on it; the PR body lists the
doc-update-map rows from PP-13.
