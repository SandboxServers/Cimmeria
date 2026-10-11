# Lab golden runs: contract

> Type: contract. Audience: every GD- packet worker and reviewer, and the
> lab-chaos and lab-record campaigns (the stable API at the end). Ledger:
> [README.md](README.md). Packets: [work-packets.md](work-packets.md).

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator.

## Module layout

```text
crates/lab/src/uat/
  mod.rs              + pub mod golden;                                      (GD-02)
  spec.rs             GoldenSpec; SectionMeta.golden, RowSpec.golden          (GD-01)
  spec_validate.rs    check_golden                                           (GD-01)
  golden/
    mod.rs            module doc, pub use, the dead_code allow (GD-02; GD-07 removes the allow)
    event.rs          Dir, Event, Variance, RowFingerprint, RunFingerprint   (GD-02)
    labels.rs         EntityLabels                                           (GD-02)
    key_args.rs       OUTBOUND_KEYS, ENTITY_FIELDS, InArg, decode_inbound    (GD-02)
    normalise.rs      DEFAULT_IGNORE, resolve, glob_match, events_from_tap,
                      transitions, windows                                   (GD-03)
    build.rs          RowInput, RunMeta, build_row, load_row_input,
                      fingerprint_run, fingerprint_run_with                  (GD-03)
    testkit.rs        #[cfg(test)] bundle builders                           (GD-03)
    agree.rs          agree, check_runs, AgreeError, Disagreement            (GD-04)
    diff.rs           Part, Divergence, DiffReport, diff_row, diff_run       (GD-04)
    file.rs           GoldenFile, golden_path, to_text, save, load           (GD-05)
    fixtures/         fs_p3_tap.json, fs_p3_entities.json                    (GD-02)
                      fs_p3_windows.json                                     (GD-03)
  runner/fingerprint.rs  FpCapture and the capture hooks                     (GD-06)
crates/lab/src/server/golden.rs  lab_golden_record, lab_golden_diff          (GD-07)
tools/lab/cli/golden.ps1, golden-lib.ps1, test-golden.ps1                    (GD-08)
docs/guides/uat-specs/golden/<section>.json                                  (GD-10)
```

GD-02 creates every file under `golden/` (the later ones as a one-line `//!`
stub) and declares them all in `golden/mod.rs`, so later packets never edit
`mod.rs` except GD-07. `golden/mod.rs` starts with
`#![cfg_attr(not(test), allow(dead_code))]` and a comment: `cimmeria-lab` is a
binary crate, so its items are dead until GD-07 wires the tools (README F13).

## Spec additions (GD-01)

The only `spec.rs` changes this campaign makes. lab-spec-vocab and
lab-fixtures edit neighbouring lines; merge on top, no design conflict.

```rust
/// What may differ between runs that still agree (README D-GD5).
/// `[section.golden]` applies to every row; `[row.golden]` adds to it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoldenSpec {
    /// Message-name globs (`*` only) dropped before comparing.
    #[serde(default)] pub ignore: Vec<String>,
    /// Globs moved out of the ordered stream into a per-row sorted multiset.
    #[serde(default)] pub allow_unordered: Vec<String>,
    /// Globs whose consecutive identical events count once.
    #[serde(default)] pub collapse: Vec<String>,
    /// Window-name globs left out of the opened and closed sets.
    #[serde(default)] pub ignore_windows: Vec<String>,
    /// Message name -> decoded fields to keep; replaces the built-in entry.
    #[serde(default)] pub key_args: std::collections::BTreeMap<String, Vec<String>>,
    /// A tap bound after the row entered the world drops this many ms of
    /// messages after its first one (default 1000).
    #[serde(default)] pub late_bind_settle_ms: Option<u64>,
    /// Row only: leave the row out of every fingerprint (cleanup rows).
    #[serde(default)] pub skip: bool,
}
// SectionMeta gains:  #[serde(default)] pub golden: Option<GoldenSpec>,
// RowSpec gains:      #[serde(default)] pub golden: Option<GoldenSpec>,
```

In TOML:

```toml
[section.golden]
allow_unordered = ["onEffectResults"]

[[row]]
id = "FS-99"
# ...
[row.golden]
skip = true
```

## Events and fingerprints (GD-02)

```rust
// event.rs
pub const FINGERPRINT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir { C2s, S2c }               // renders "c2s" / "s2c"

/// One normalised message. Rendered:
/// `c2s interact target=tag:ArmYourself_FrostBody`
/// `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub dir: Dir,
    /// The tap's msg_name; `unknown#<method_index>` when the tap says `unknown`.
    pub name: String,
    /// S2C only: the label of `target_entity_id`.
    pub target: Option<String>,
    /// Key arguments in table order, values already rendered.
    pub args: Vec<(String, String)>,
}
impl Event { pub fn render(&self) -> String; }

/// The resolved variance one row was fingerprinted with (stored in the golden).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variance {
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub ignore: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub allow_unordered: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub collapse: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub ignore_windows: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub key_args: BTreeMap<String, Vec<String>>,
    pub late_bind_settle_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowFingerprint {
    pub row: String,
    /// `RowResult::as_str()`: PASS, FAIL, ...
    pub result: String,
    /// `full`, `late:<after>`, `none` or `unavailable` (the packet-tap
    /// body's `coverage`; `none` when the row has no packet-tap attachment).
    pub tap: String,
    /// `dropped <n>`, `replaced`, `not stopped`, `read failed`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub tap_faults: Vec<String>,
    /// `<clause id>=<verdict>` for required clauses whose source is not
    /// timing, signoz or human, in spec order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub clauses: Vec<String>,
    /// `mission 1360 -> 0`, `step 2113 -> 1`, `objective 3238 -> 1`, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub transitions: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub windows_opened: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub windows_closed: Vec<String>,
    /// Rendered ordered events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub events: Vec<String>,
    /// Rendered `allow_unordered` events, sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub unordered: Vec<String>,
    pub variance: Variance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunFingerprint { pub schema: u32, pub section: String, pub rows: Vec<RowFingerprint> }
```

## Entity labels and key arguments (GD-02)

```rust
// labels.rs: README D-GD3.
pub struct EntityLabels { /* player, p2, known: HashMap<u32, String>, unknown: Vec<u32> */ }
impl EntityLabels {
    /// `entities`: the entities.json attachment (below), or None.
    pub fn new(player: u32, p2: Option<u32>, entities: Option<&serde_json::Value>) -> Self;
    /// 0 -> `none`; player -> `self`; p2 -> `p2`; another player -> `player`;
    /// then `tag:<tag>`, `spawn:<id>`, `tpl:<id>`; else `?<n>` by first call.
    pub fn label(&mut self, id: u32) -> String;
}

// key_args.rs: README D-GD4.
pub const OUTBOUND_KEYS: &[(&str, &[&str])] = &[
    ("onDialogDisplay", &["dialog_id", "entity_id", "mission_id"]),
    ("InteractionType", &["TypeId"]),
    ("onMissionUpdate", &["MissionID", "Status"]),
    ("onStepUpdate", &["StepID", "Status"]),
    ("onObjectiveUpdate", &["ObjectiveID", "Status", "Hidden", "Optional"]),
    ("onKnownAbilitiesUpdate", &["AbilityData.items"]),
    ("onPlayerCommunication", &["Channel", "Text"]),
    ("onSequence", &["kismet_event_set_seq_id", "source_id", "target_id"]),
    ("onEntityProperty", &["type", "value"]),
];
/// Decoded fields holding an entity id, rendered as labels.
pub const ENTITY_FIELDS: &[&str] = &["entity_id", "source_id", "target_id"];

#[derive(Debug, Clone, PartialEq)]
pub enum InArg { Int(i64), Entity(u32), Pos([f32; 3]) }
/// Key arguments of an inbound cell call from its raw args (the tap's
/// args_hex, decoded). Skips the 4-byte entity-id prefix, and the sub-slot
/// byte too when msg_id is 0xBD (README F5). None for a method not listed
/// or args too short.
///   74  interact                          target: Entity (INT32)
///   75  dialogButtonChoice                dialog: Int, button: Int (INT32 each)
///   38  moveItem                          (INT32 item id skipped) bag, slot, qty: Int
///   163 gmGotoXYZ                         pos: Pos (3 x FLOAT)
///   85  triggerClientHintedGenericRegion  region: Int (INT32), entering: Int (UINT8)
pub fn decode_inbound(method_index: i32, msg_id: Option<u8>, args: &[u8]) -> Option<Vec<(&'static str, InArg)>>;
```

Value rendering, everywhere: integers in decimal; floats to one decimal
(`-325.0`); a position as `x,y,z`; arrays as `[579,597]` in the order given;
strings with the run's variable values replaced (below), whitespace runs
collapsed to one space, cut to 80 characters.

## Bundle attachments in fingerprint mode (GD-06 writes, GD-03 reads)

All three sit in `rows/<section>/<row>/` and are listed in the row's
`attachments[]` under these names.

- `packet_tap` (`packet-tap.json`, existing): the body gains
  `"coverage": "full" | "late:<after>" | "none" | "unavailable"` and
  `"replaced_existing": bool`. In fingerprint mode it is written for every
  row, also when no tap bound (`{ "coverage": "none", "error": "..." }`).
- `entities` (`entities.json`):
  `{ "player": 8, "space_id": 65563, "snapshots": [ { "at": "bind" | "finish", "capped": false, "entities": [ { "entity_id": 100751, "is_player": false, "tag": "ArmYourself_FrostBody", "template_id": 14 } ] } ] }`
  (a snapshot that failed has `"error"` and no `entities`; absent keys are
  left out).
- `windows` (`windows.json`):
  `{ "samples": [ { "after": "start", "windows": ["SelfStatusWin"] }, { "after": "frost", "windows": ["DialogWin", "SelfStatusWin"] }, { "after": "step#4", "error": "..." } ] }`.

`<after>` is the action's label, else `setup#<i>` or `step#<i>` (0-based in
that list).

Variable substitution: the row's `vars` values for `run_id`, `character` and
`p2_character` are replaced, case-insensitively and longest first, by
`${run_id}`, `${character}` and `${p2_character}`.

## Stable API for lab-chaos and lab-record

Changing any of these needs a coordinator note in both ledgers.

```rust
pub fn fingerprint_run(run: &RunDir, spec: &SectionSpec, rows: Option<&[String]>)
    -> Result<(RunFingerprint, RunMeta), String>;                                   // build.rs
pub fn fingerprint_run_with(run: &RunDir, section: &str, rows: &[(String, Variance)])
    -> Result<(RunFingerprint, RunMeta), String>;                                   // build.rs
pub fn diff_run(golden: &RunFingerprint, run: &RunFingerprint) -> DiffReport;      // diff.rs
impl Divergence { pub fn line(&self) -> String; }                                  // diff.rs
pub fn load(path: &Path) -> Result<GoldenFile, String>;                            // file.rs
```

The tools `lab_golden_diff` and `lab golden` / `lab uat -DiffGolden` (and
their `-Json` shapes in GD-07 to GD-09) are the same API for scripts.
