# Lab golden runs: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for GD-06, `documentation-writer` for GD-11, and
> `packet-reviewer` reviewers (Sonnet). Ledger, findings (F1 to F20) and
> decisions (D-GD1 to D-GD10): [README.md](README.md). The names packets build against: [contract.md](contract.md); a brief carries it with the packet section.
>
> PowerShell only. Every compiling command goes through the lane, in this
> order, from the worktree root:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p cimmeria-lab
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab
> ```
>
> PowerShell packets run their test script directly:
> `pwsh -NoProfile -File tools/lab/cli/test-<name>.ps1` (exit 0 is a pass).
> Read the lane's summary and its failures file; do not rerun a build to see
> the output. No packet needs a database, and no packet but GD-10 touches the
> lab.
>
> Rust rules for every packet: no `unwrap()` outside tests; comments say why,
> not what; `#[cfg(test)] mod tests` last in a file; no new file over 500
> lines. Test names say what they prove. Code lives under
> `crates/lab/src/uat/golden/` unless the packet says otherwise.

## Contents

- [Contract](contract.md) (separate file: module layout, spec additions, types, attachments, stable API)
- [GD-01 Spec contract](#gd-01-spec-contract)
- [GD-02 Fingerprint types, labels and key arguments](#gd-02-fingerprint-types-labels-and-key-arguments)
- [GD-03 Normaliser and run fingerprint](#gd-03-normaliser-and-run-fingerprint)
- [GD-04 Agreement and diff](#gd-04-agreement-and-diff)
- [GD-05 Golden file](#gd-05-golden-file)
- [GD-06 Runner capture in fingerprint mode](#gd-06-runner-capture-in-fingerprint-mode)
- [GD-07 MCP tools](#gd-07-mcp-tools)
- [GD-08 lab golden record](work-packets-2.md#gd-08-lab-golden-record)
- [GD-09 lab uat -DiffGolden](work-packets-2.md#gd-09-lab-uat--diffgolden)
- [GD-10 Live acceptance](work-packets-2.md#gd-10-live-acceptance)
- [GD-11 Close-out](work-packets-2.md#gd-11-close-out)

## GD-01 Spec contract

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-golden/gd01-spec`. **Worktree:** `gd01`.
**Subject:** `feat(lab): GD-01 golden variance model in the UAT spec schema`

Why: README D-GD5.

Files:

1. `crates/lab/src/uat/spec.rs`: add `GoldenSpec` (contract, with its doc
   comments) after `EvidenceSpec`, and the `golden` field on `SectionMeta`
   (after `fresh`) and `RowSpec` (after `evidence`). Neither struct is built
   literally anywhere (grep `SectionMeta {` and `RowSpec {` to confirm).
2. `crates/lab/src/uat/spec_validate.rs`: `fn check_golden(at: &str, g: &GoldenSpec, row_level: bool, errs: &mut Vec<String>)`,
   called from `validate` for the section (`at = "[section.golden]"`,
   `row_level = false`) and each row (`at = "<row id>/golden"`). Errors, each
   prefixed with `at`:
   - `skip` set at section level: `skip is row-only`.
   - a glob in any list that is empty or holds whitespace: `bad glob "<g>"`.
   - a glob in both `ignore` and `allow_unordered`: `"<g>" is both ignored and unordered`.
   - a `key_args` entry with an empty name or an empty field list.

Tests (unit, TESTING.md type 1, in `spec_tests.rs`):

- `golden_tables_parse_on_the_section_and_the_row`: a two-row section with
  `[section.golden] allow_unordered = ["onEffectResults"]` and a row with
  `[row.golden] skip = true`; both land in the parsed spec. Fails if either
  field is missing or misplaced.
- `an_unknown_golden_key_is_rejected`: `[section.golden] ignored = [...]`
  fails to parse (`deny_unknown_fields`).
- `section_skip_and_overlapping_globs_are_rejected`: each of the four rules
  above returns its message.
- `committed_specs_parse_and_validate` (existing, `uat/mod.rs`) stays green.

Checks: the three lane commands in the header.

Docs owed: a `### Golden variance` subsection in
`docs/guides/automated-uat.md`, after `### Two-player rows`: the seven keys,
the built-in `ignore` defaults (`avatarUpdate*`, `perfStats`,
`requestEntityUpdate`), that the row extends the section, and that the
`lab golden` commands arrive later in this campaign (link the ledger).

Reviewer focus: `deny_unknown_fields` on the new struct; validation runs for
every row; no behaviour change for specs without the tables.

## GD-02 Fingerprint types, labels and key arguments

**Implementer:** packet-coder. **Size:** M. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-golden/gd02-types`. **Worktree:** `gd02`.
**Subject:** `feat(lab): GD-02 golden fingerprint types, entity labels and key-argument tables`

Why: README D-GD2, D-GD3, D-GD4, F5, F6, F8.

Files:

1. `crates/lab/src/uat/mod.rs`: `pub mod golden;` and one line in the module
   doc's list (`[`golden`] — run fingerprints, golden agreement and diffs`).
2. `golden/mod.rs`: module doc (what a fingerprint is, the owner's five-run
   rule, link to the ledger), the dead-code allow (contract), `pub mod` for
   `event`, `labels`, `key_args`, `normalise`, `build`, `agree`, `diff`,
   `file`, and `#[cfg(test)] pub(crate) mod testkit;`. Re-export
   `pub use event::{Dir, Event, RowFingerprint, RunFingerprint, Variance, FINGERPRINT_SCHEMA};`.
3. Stubs `normalise.rs`, `build.rs`, `agree.rs`, `diff.rs`, `file.rs`,
   `testkit.rs`: one `//!` line each naming the packet that fills it.
4. `event.rs`, `labels.rs`, `key_args.rs` (contract). `Event::render`:
   `"{dir} {name}"`, then `"@{target}"` when set, then `" {k}={v}"` per arg.
   `decode_inbound` reads little-endian with bounds checks, no panics. The
   value-rendering rules go in `key_args.rs` as
   `pub fn render_json(v: &serde_json::Value) -> String` and
   `pub fn render_in(a: &InArg, labels: &mut EntityLabels) -> String`.
5. Fixtures in `golden/fixtures/`, made from the Praxis capture with this
   PowerShell (records 4 to 34 are FS-P3: the teleport to Frost through the
   dialog's close):

   ```powershell
   $src = Get-Content -Raw crates/wireclient/tests/fixtures/praxis_start_tap.json | ConvertFrom-Json
   $msgs = @($src.messages[4..34])
   [ordered]@{ entity_id = 8; read_ok = $true; stopped = $true; coverage = 'full'; replaced_existing = $false
       tap = [ordered]@{ entity_id = 8; capacity = 5000; count = $msgs.Count; dropped = 0; messages = $msgs } } |
       ConvertTo-Json -Depth 12 | Set-Content crates/lab/src/uat/golden/fixtures/fs_p3_tap.json -Encoding utf8NoBOM
   ```

   and `fs_p3_entities.json` by hand (contract shape): player 8, one `bind`
   snapshot holding entity 8 (`is_player: true`), 100751
   (`tag: "ArmYourself_FrostBody"`, `template_id: 14`) and 100748
   (`tag: "ArmYourself_GuardBody"`). Leave 100745 out on purpose (it must
   label as `?1`). Note in a `README` line inside `golden/mod.rs`'s doc
   that the entity file is synthetic and the tags are the seed's
   (`db/resources/Worlds/Seed/spawnlist.sql`).

Tests (unit, TESTING.md type 2 wire-format for `decode_inbound`, type 1 for
the rest; each in its own file's `mod tests`):

- `inbound_calls_decode_from_the_praxis_capture`: load the tap fixture with
  `include_str!`; record 15 (`interact`, the fixture's #19) gives
  `[("target", Entity(100751))]`; the first `dialogButtonChoice` gives dialog
  3995, button -1; `gmGotoXYZ` gives `Pos` whose rendering is
  `-325.0,73.6,-212.8`; `triggerClientHintedGenericRegion` gives region 14,
  entering 0. Fails if the 0xBD sub-slot byte is not skipped.
- `inbound_decode_never_panics_on_short_args`: every listed index with 0 to
  8 bytes returns `None`, never a panic.
- `labels_follow_the_precedence`: `self`, `tag:`, `spawn:`, `tpl:`,
  `player`, `none` for 0, and two unknown ids get `?1` then `?2` (and `?1`
  again on a repeat).
- `events_render_in_one_line`: a C2S event without target and an S2C event
  with target and two args render exactly as in the contract examples.

Checks: the three lane commands in the header.

Docs owed: none (GD-11).

Reviewer focus: offsets (4, or 5 for 0xBD); `?n` numbering is per
`EntityLabels` (per row), in call order; no `unwrap` in non-test code.

## GD-03 Normaliser and run fingerprint

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** GD-01, GD-02.
**Branch:** `lab-golden/gd03-normalise`. **Worktree:** `gd03`.
**Subject:** `feat(lab): GD-03 golden normaliser: variance, transitions, windows and run fingerprints`

Why: README D-GD1, D-GD5, D-GD8, F7, F9 to F12.

Files:

1. `golden/normalise.rs`:
   - `pub const DEFAULT_IGNORE: &[&str] = &["avatarUpdate*", "perfStats", "requestEntityUpdate"];`
     and `pub const DEFAULT_LATE_BIND_SETTLE_MS: u64 = 1000;`.
   - `pub fn glob_match(pattern: &str, name: &str) -> bool`: `*` matches any
     run, everything else literally, case-sensitive.
   - `pub fn resolve(section: Option<&GoldenSpec>, row: Option<&GoldenSpec>) -> Variance`:
     each list is defaults (only `ignore` has any), then section, then row,
     de-duplicated in first-seen order; `key_args` is the section's map with
     the row's entries replacing same-named ones; `late_bind_settle_ms` is
     the row's, else the section's, else the default.
   - `pub fn events_from_tap(messages: &[Value], late: bool, labels: &mut EntityLabels, v: &Variance, subst: &[(String, String)]) -> (Vec<Event>, Vec<String>)`.
     Per record, in order: skip it when `late` and its `ts_ms` is earlier
     than the first record's plus `v.late_bind_settle_ms`; skip it when
     its name matches an `ignore` glob; build the `Event` (`dir` from `"in"`
     or `"out"`; name rule from the contract; S2C target from
     `target_entity_id`; args from `v.key_args` if it names the message,
     else `OUTBOUND_KEYS`, read from `decoded` by dotted path, entity fields
     labelled; C2S args from `decode_inbound` on the hex-decoded `args_hex`).
     Then route it to the unordered list (rendered) when it matches an
     `allow_unordered` glob, else to the ordered list. Finally drop an
     ordered event equal to its predecessor when its name matches a
     `collapse` glob, and sort the unordered list. Returns
     `(ordered, unordered)`.
   - `pub fn transitions(events: &[Event]) -> Vec<String>`: for
     `onMissionUpdate`, `onStepUpdate` and `onObjectiveUpdate` events,
     `mission <MissionID> -> <Status>`, `step <StepID> -> <Status>`,
     `objective <ObjectiveID> -> <Status>`. Call it on the ordered events
     and the unordered ones' source events before routing (keep a copy).
   - `pub fn windows(samples: Option<&Value>, v: &Variance) -> (Vec<String>, Vec<String>)`:
     the first sample without `error` is the start; opened = every window in
     a later good sample that is not in the start; closed = every window in
     any good sample that is not in the last good one; both minus
     `ignore_windows`, sorted. No good sample: both empty.
2. `golden/build.rs` (contract):
   - `pub struct RowInput { pub evidence: RowEvidence, pub tap: Option<Value>, pub entities: Option<Value>, pub windows: Option<Value> }`.
   - `pub struct RunMeta { pub run_id: String, pub run_dir: String, pub spec_sha256: Option<String>, pub server_build: Option<String>, pub sgw_sha256: Option<String> }`
     from `run.json`: `run_id`; the folder name; the `specs[]` entry whose
     `section` matches; `server.service_version`; `client.sgw_exe.sha256`.
   - `pub fn build_row(input: &RowInput, v: &Variance) -> RowFingerprint`:
     `result` from `evidence.result.as_str()`; `tap` from the body's
     `coverage`, `none` without a body; `tap_faults` from
     `tap.dropped > 0` (`dropped <n>`), `replaced_existing`, `stopped == false`
     (`not stopped`), `read_ok == false` (`read failed`); `clauses` per the
     contract (verdict as its serde name); labels from
     `EntityLabels::new(body.entity_id, None, entities)`; events and
     transitions from `events_from_tap` (with `late` when coverage starts
     with `late:`); windows from `windows`; `variance = v.clone()`.
   - `pub fn load_row_input(run: &RunDir, section: &str, row: &str) -> Result<RowInput, String>`:
     the row JSON, then each attachment named `packet_tap`, `entities`,
     `windows` read from `run.root.join(path)`; a missing one is `None`.
   - `fingerprint_run` (contract): for every spec row (or the listed ones, in
     spec order) whose `golden.skip` is not set, `resolve` then `build_row`.
     A listed row with no row JSON in the run is an error naming it.
   - `fingerprint_run_with` (contract): the same with the given variance per
     row, no spec.
3. `golden/testkit.rs` (`#[cfg(test)]`): `pub fn row_evidence(row: &str, result: RowResult, clauses: &[(&str, Source, bool, Verdict)]) -> RowEvidence`
   (every other field empty or zero), and
   `pub fn write_run(root: &Path, run_id: &str, section: &str, rows: &[(RowEvidence, Option<Value>, Option<Value>, Option<Value>)]) -> RunDir`
   which writes `run.json` (with a `specs[]` entry, a `server.service_version`
   of `"testbuild"` and a `client.sgw_exe.sha256`), each row JSON, and each
   present attachment, with `attachments[]` filled in.
4. `golden/fixtures/fs_p3_windows.json`: samples `start` (`SelfStatusWin`),
   `frost` (`DialogWin`, `SelfStatusWin`), `close` (`SelfStatusWin`).

Tests (unit, type 1; in `normalise.rs` and `build.rs`):

- `fs_p3_normalises_to_the_expected_events`: the GD-02 fixtures through
  `build_row` with `resolve(None, None)`. Assert `events` equals, exactly,
  the 17 strings the fixture gives, written out by hand in the test,
  starting
  `c2s gmGotoXYZ pos=-325.0,73.6,-212.8`,
  `s2c onPlayerCommunication@self Channel=9 Text=gmGotoXYZ: teleported to (-325, 73.6, -212.8)`,
  `s2c unknown#27@?1`,
  `c2s triggerClientHintedGenericRegion region=14 entering=0`,
  `c2s interact target=tag:ArmYourself_FrostBody`,
  `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0`,
  and ending `c2s dialogButtonChoice dialog=3995 button=-1`. Derive the rest
  by reading the fixture; there is no `avatarUpdateExplicit`, `perfStats` or
  `requestEntityUpdate` in it. Fails if the defaults, labels or key tables
  break.
- `fs_p3_transitions_and_windows`: transitions start `objective 3238 -> 1`
  and include `mission 1360 -> 0` and `step 80623 -> 0`; `windows_opened ==
  ["DialogWin"]`, `windows_closed == ["DialogWin"]`.
- `the_run_id_never_reaches_the_fingerprint`: a tap whose
  `onPlayerCommunication` text holds `Uat Pxabcdef` with `vars.run_id =
  "abcdef"` renders `Uat Px${run_id}`.
- `allow_unordered_and_collapse_and_settle`: two orders of the same two
  `onEffectResults` give the same `unordered`; three identical collapsed
  events give one; a late tap drops what falls in the settle window.
- `faults_and_clauses_are_recorded`: `dropped: 3` gives `dropped 3`; a
  timing clause and an optional clause are left out of `clauses`.
- `resolve_layers_defaults_section_and_row`.

Checks: the three lane commands in the header.

Docs owed: none (GD-11).

Reviewer focus: transitions are taken before `allow_unordered` routing;
`ignore` runs before anything else; the settle cut uses the first record's
server timestamp, never a host clock.

## GD-04 Agreement and diff

**Implementer:** packet-coder. **Size:** M. **Wave:** 3. **Depends on:** GD-03.
**Branch:** `lab-golden/gd04-diff`. **Worktree:** `gd04`.
**Subject:** `feat(lab): GD-04 golden agreement and one-line first divergence`

Why: the owner's five-run rule, the one-line divergence, and the test-only
acceptance (README D-GD6, D-GD8).

Files:

1. `golden/diff.rs`:
   - `pub enum Part { Missing, Tap, TapFault, Result, Clause, Transition, WindowsOpened, WindowsClosed, Event, Unordered }`.
   - `pub struct Divergence { pub row: String, pub part: Part, pub expected: Option<String>, pub got: Option<String>, pub after: Option<String> }`
     (`None` in `expected` or `got` means "the end of the row").
   - `pub fn diff_row(golden: &RowFingerprint, run: &RowFingerprint) -> Option<Divergence>`:
     compare in this order and return the first difference: `tap`;
     `tap_faults` of the run (any fault is a divergence); `result`;
     `clauses`, `transitions`, `events` as sequences (first index where
     they differ; `after` is the golden's previous element); the two window
     sets; `unordered` (first index where the sorted lists differ).
   - `pub struct DiffReport { pub first: Option<Divergence>, pub rows: Vec<(String, Option<Divergence>)>, pub not_in_golden: Vec<String> }`
     and `pub fn diff_run(golden, run) -> DiffReport`: golden rows in order (a
     golden row the run lacks is `Part::Missing`), then the run's extra rows
     in `not_in_golden`.
   - `impl Divergence { pub fn line(&self) -> String }`. Quote a value as
     `` `v` `` (cut to 120 characters with `...`), an end as
     `the end of the row`; the whole line at most 300 characters. Shapes:
     - Event: ``row FS-P3: expected `E` after `A`, got `G` `` (`first`
       instead of `after ...` at index 0).
     - Transition / Clause / Unordered: the same with `transition ` /
       `clause ` / `unordered ` before the first quote.
     - Tap: ``row FS-P2: expected tap `full`, got `late:play` ``.
     - TapFault: ``row FS-P3: tap fault `dropped 3`; the diff is unreliable``.
     - Result: ``row FS-P4: expected result `PASS`, got `FAIL` ``.
     - Windows: ``row FS-P4: expected windows opened `DialogWin, TutorialWin`, got `DialogWin` ``.
     - Missing: `row FS-P5: in the golden, not in this run`.
2. `golden/agree.rs`:
   - `pub const MIN_AGREEING_RUNS: usize = 5;`
   - `pub struct Disagreement { pub run: usize, pub of: usize, pub divergence: Divergence }`
     with `line()`: `run 4 of 5 disagrees with run 1: <divergence line>`.
   - `pub enum AgreeError { TooFew { have: usize }, NotPassing { run: usize, row: String, result: String }, Disagree(Disagreement) }`
     with `Display` (one line each).
   - `pub fn agree(fps: &[RunFingerprint], require_pass: bool) -> Result<RunFingerprint, AgreeError>`:
     fewer than `MIN_AGREEING_RUNS` is `TooFew`; with `require_pass`, any
     row not `PASS` is `NotPassing`; then `diff_run(&fps[0], &fps[k])` for
     each k, the first `first` is `Disagree`; else `fps[0].clone()`.
   - `pub fn check_runs(metas: &[RunMeta]) -> Result<(), String>`: run ids
     distinct; every `spec_sha256`, `server_build` and `sgw_sha256` present
     and equal across runs. Each failure names the field and the two values.

Tests (unit, type 1; `testkit` bundles):

- `a_changed_dialog_answer_is_one_line` (**the campaign's test-only
  acceptance**): fingerprint the FS-P3 fixtures as five runs and `agree`;
  then edit one copy's tap so the first `onDialogDisplay` (subset index 16, the capture's #20) has `decoded.dialog_id` 3996 (a
  changed server answer), fingerprint it, and assert `diff_run(...).first`'s
  `line()` equals exactly

  ```text
  row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0` after `c2s interact target=tag:ArmYourself_FrostBody`, got `s2c onDialogDisplay@self dialog_id=3996 entity_id=tag:ArmYourself_FrostBody mission_id=0`
  ```

  and that the line has no newline. Fails if the normaliser leaks ids or
  the diff compares the wrong part first.
- `five_identical_runs_agree_and_four_do_not`: `TooFew { have: 4 }`.
- `a_disagreeing_fifth_run_names_itself`: `run 5 of 5 disagrees with run 1: ...`.
- `a_failing_row_cannot_be_golden_when_pass_is_required`.
- `ignored_noise_does_not_break_agreement`: one copy with extra
  `avatarUpdateExplicit` records still agrees.
- `every_part_has_its_line`: one case per `Part`, asserting the shapes above.
- `check_runs_rejects_mixed_builds_and_repeated_runs`.

Checks: the three lane commands in the header.

Docs owed: none (GD-11).

Reviewer focus: the comparison order matches the list; `after` is the
golden's element, not the run's; line caps hold for long events.

## GD-05 Golden file

**Implementer:** packet-coder. **Size:** S. **Wave:** 2. **Depends on:** GD-02.
**Branch:** `lab-golden/gd05-file`. **Worktree:** `gd05`.
**Subject:** `feat(lab): GD-05 golden file format with a size cap and a committed-golden guard`

Why: README D-GD10, F15.

Files: `golden/file.rs`:

```rust
pub const GOLDEN_SCHEMA: u32 = 1;
pub const GOLDEN_MAX_BYTES: usize = 128 * 1024;
pub const GOLDEN_MAX_ROW_EVENTS: usize = 1500;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenFile {
    pub schema: u32,
    pub section: String,
    pub rows: Vec<String>,
    pub recorded_utc: String,
    pub spec_sha256: String,
    pub build: GoldenBuild,
    /// The agreeing runs (local evidence folders, not committed).
    pub evidence: Vec<GoldenEvidence>,
    pub fingerprint: RunFingerprint,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenBuild { pub server: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub sgw_sha256: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenEvidence { pub run_id: String, pub run_dir: String }
/// <specs_dir>/golden/<path_safe(section)>.json
pub fn golden_path(specs_dir: &Path, section: &str) -> PathBuf;
/// Pretty JSON plus a trailing newline. Err when a row has more than
/// GOLDEN_MAX_ROW_EVENTS events (naming the row and suggesting an `ignore`
/// family) or the text is over GOLDEN_MAX_BYTES (naming the size).
pub fn to_text(g: &GoldenFile) -> Result<String, String>;
pub fn save(path: &Path, g: &GoldenFile) -> Result<usize, String>;   // creates golden/; returns bytes
pub fn load(path: &Path) -> Result<GoldenFile, String>;             // both schemas must match
```

Tests (unit, type 1):

- `a_golden_round_trips`; `an_oversized_golden_is_refused` (a row of 1 501
  events, and a file over the byte cap); `a_wrong_schema_is_refused`.
- `committed_goldens_parse_and_fit` (the guard, like
  `committed_specs_parse_and_validate`): for every `*.json` in
  `docs/guides/uat-specs/golden/` (none until GD-10; a missing folder
  passes), `load` succeeds, `to_text` succeeds, the section exists in
  `load_sections`, and every golden row id exists in that spec.

Checks: the three lane commands in the header.

Docs owed: none (GD-11).

Reviewer focus: the guard really reads the committed folder
(`env!("CARGO_MANIFEST_DIR")` + `../../docs/guides/uat-specs/golden`); the
size check is on the written text, not on a compact form.

## GD-06 Runner capture in fingerprint mode

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 2. **Depends on:** GD-01.
**Branch:** `lab-golden/gd06-capture`. **Worktree:** `gd06`.
**Subject:** `feat(lab): GD-06 fingerprint mode: tap every row, bind late, snapshot entities and windows`

Why: README F1 to F4, F9, F14, D-GD3, D-GD8. Judgment: async runner flow
and the late bind.

Files:

1. `crates/lab/src/server/uat.rs`: `UatRunArgs.fingerprint: bool`
   (`#[serde(default)]`, doc: "Capture what a golden fingerprint needs: a
   packet tap on every in-world row, bound late when the row enters the
   world, entity snapshots and window samples. lab golden and lab uat
   -DiffGolden set it."), passed into `RunRequest` (`uat.rs:385`).
2. `crates/lab/src/uat/runner/mod.rs`: `RunRequest.fingerprint: bool`;
   `RowCtx.fp: Option<fingerprint::FpCapture>` (initialised `None` at
   `mod.rs:308`); `mod fingerprint;`. In `drive_row` (`mod.rs:389`):
   `let fp = self.fp_wanted(row);`, `tapped` becomes
   `has_packet_clause || fp`, set `ctx.fp` before `tap_start`, call
   `fp_start` after it, `fp_before_tap_read` before `tap_finish`, and
   `fp_write` at the end (also on the setup-failure path). In
   `drive_steps` (`mod.rs:434`), after each setup and each step action is
   pushed: `self.fp_after_action(name, ctx).await` when `ctx.fp` is set,
   `name` per the contract's `<after>`. Keep the additions to these hooks;
   the logic goes in the new file.
3. `crates/lab/src/uat/runner/packet.rs`: `RowTap.replaced_existing: bool`,
   set from `server_packet_tap_start`'s reply; `tap_finish`'s body gains
   `coverage` and `replaced_existing` when `ctx.fp` is set.
4. New `crates/lab/src/uat/runner/fingerprint.rs`:
   - `pub(crate) struct FpCapture { windows: Vec<Value>, snapshots: Vec<Value>, space_id: Option<u32>, bound_after: Option<String>, unavailable: bool }`
     with `fn coverage(&self, bound_at_start: bool) -> String`.
   - `fp_wanted(&self, row) -> bool`: `self.req.fingerprint` and not
     `row.golden.skip`.
   - `fp_start`: one window sample `start`; if the tap bound, the `bind`
     entity snapshot (`server_entity_get` on the player for `space_id`, then
     `server_entity_query { space_id }`, trimmed to the contract's fields);
     `unavailable` when `self.server` is `None`.
   - `fp_after_action(after, ctx)`: one window sample; then, while the tap
     has no entity and a server is configured, try to bind silently:
     `server.call("server_sessions")` and `find_session` without recording,
     and only when a session is found the recorded `server_packet_tap_start`
     (as `tap_start` does), then `bound_after = after` and the `bind`
     snapshot.
   - `fp_before_tap_read`: the `finish` snapshot when bound.
   - `fp_write`: `windows.json` and `entities.json` as attachments (names
     `windows`, `entities`); when no tap bound, `packet-tap.json` with
     `coverage` (`none` or `unavailable`) and the tap error.
   - Window samples call `self.inv.call("client_ui_state", json!({}))` and
     keep only `/windows`; they are not recorded as actions.
5. New `crates/lab/src/uat/runner/fingerprint_tests.rs` (declared at the end
   of `mod.rs` beside `packet_tests`), reusing `packet_tests.rs`'s
   `FakeServer` pattern (extend it with `server_entity_get` and
   `server_entity_query` replies, and a shared flag that makes
   `server_sessions` list the character only after a given client tool ran).

Tests (TESTING.md type 1, async with the fakes):

- `fingerprint_mode_taps_a_row_without_packet_clauses`: one tool-clause row;
  with `fingerprint` a `packet_tap` attachment has `coverage: "full"`;
  without it there is no attachment. Fails if `tapped` ignores the flag.
- `a_row_that_enters_the_world_binds_its_tap_late`: the session appears
  after the step labelled `play`; coverage is `late:play`, one recorded
  `server_packet_tap_start`, and no recorded failed attempts.
- `windows_are_sampled_after_every_action` and
  `entities_are_snapshotted_at_bind_and_finish`.
- `without_a_server_the_coverage_is_unavailable`.
- `fingerprint_mode_does_not_change_the_grade` (regression guard): the same
  passing and failing rows give the same `result`, `reasons` and step
  actions with the flag on and off.
- The existing `packet_tests` stay green unchanged.

Checks: the three lane commands in the header.

Docs owed: `docs/guides/automated-uat.md` § The evidence bundle: the three
attachments and the `fingerprint` argument, two short paragraphs.

Reviewer focus: every path that starts a tap still stops it (including a
late bind on a row that then fails); no new recorded actions except the one
successful start and the snapshots; `mod.rs` grows by hooks only.

## GD-07 MCP tools

**Implementer:** packet-coder. **Size:** M. **Wave:** 4. **Depends on:** GD-04, GD-05, GD-06.
**Branch:** `lab-golden/gd07-tools`. **Worktree:** `gd07`.
**Subject:** `feat(lab): GD-07 lab_golden_record and lab_golden_diff tools`

Why: README D-GD9, F19.

Files:

1. New `crates/lab/src/server/golden.rs`, following `server/uat.rs`'s
   shape (args structs with `schemars::JsonSchema`, the `#[tool]` methods,
   and pure functions the tests call):
   - `GoldenRecordArgs { section: String, rows: Option<Vec<String>>, run_dirs: Vec<String>, specs_dir: Option<String>, rebless: bool }`.
   - `GoldenDiffArgs { run_dir: String, section: String, specs_dir: Option<String> }`.
   - `pub(crate) fn record(args, now_utc: &str) -> Value`: load the section
     (`load_sections`, `specs_dir` or `uat::specs_dir()`); `fingerprint_run`
     each run dir; refuse (code `cannot_record`) a row with `tap ==
     "unavailable"` or any `tap_faults`; `check_runs` (code
     `cannot_record`); `agree(.., true)` (code `disagree` or `not_passing`);
     a golden that exists without `rebless` (code `exists`); build the
     `GoldenFile` (rows, spec SHA, build, evidence from the metas) and
     `file::save` (code `cannot_record` on a size error). Reply
     `{ "ok": true, "path", "rows", "events", "bytes", "runs" }` or
     `{ "ok": false, "code", "error" }`, `error` one line.
   - `pub(crate) fn diff(args) -> Value`: `file::load(golden_path(..))`
     (code `no_golden`), `fingerprint_run_with` using the golden's rows and
     variances, `diff_run`. Reply `{ "ok": true, "match": bool, "first"?:
     line, "rows"?: { "<row>": line } (at most 5, diverged rows only),
     "not_in_golden"?: [...] }`, no nulls.
   - Tool descriptions: one sentence each, naming the five-run rule and
     that neither drives the client.
2. `crates/lab/src/server/mod.rs`: route the two tools as `uat.rs`'s are.
3. `crates/lab/src/lease/policy.rs`: both names in `OPEN`, under the
   `lab_uat_report` line, with the comment `// Golden runs: read bundles, write only the golden file (no client).`
4. `golden/mod.rs`: remove the dead-code allow. For each warning left, the
   item is in the stable API (keep it with
   `#[allow(dead_code)] // stable API for lab-chaos (README D-GD9)`) or
   unused (delete it).

Tests (unit, type 1, in `golden.rs`):

- `record_writes_a_golden_from_five_agreeing_runs` (tempdir specs dir with
  a two-row spec, five `testkit` runs): `ok`, file present, `rows == 2`.
- `record_refuses_an_existing_golden_without_rebless`, then succeeds with it.
- `record_refuses_disagreeing_runs_with_the_line` and
  `record_refuses_an_unavailable_tap`.
- `diff_reports_the_changed_answer_in_one_line`: the GD-04 acceptance
  through the tool; `first` equals the GD-04 line; the JSON text is under
  500 characters.
- The policy test `every_routed_tool_is_classified` passes.

Checks: the three lane commands in the header.

Docs owed: two rows in the tools table of `docs/guides/live-research-lab.md`
(§ Tools at a glance), and the open-tool list line in the lease section.

Reviewer focus: neither tool touches the client or the lease; `record`
never writes outside `<specs_dir>/golden/`; codes map one-to-one to GD-08's
exit codes.
