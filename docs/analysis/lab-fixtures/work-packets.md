# Lab row fixtures and rewind: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for FX-02, FX-03 and FX-04, and `packet-reviewer`
> reviewers (Sonnet). This file holds the dispatch header, the contract and
> FX-01 to FX-05; FX-06 to FX-13 are in
> [work-packets-runner-rewind.md](work-packets-runner-rewind.md). Ledger,
> findings (F1 to F18) and decisions (D-FX1 to D-FX9): [README.md](README.md).
>
> PowerShell only. Every compiling command goes through the lane, in this
> order, from the worktree root, with `-p` the packet's crate:
>
> ```powershell
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo fmt -p <crate>
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo clippy -p <crate> --all-targets -- -D warnings
> pwsh -NoProfile -File tools/build-lane/lane.ps1 cargo nextest run -p <crate>
> ```
>
> Live-DB tests run with
> `pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 <test-name substring>`.
> Read the lane's summary and its failures file; do not rerun a build to see
> the output.
>
> Rust rules for every packet: no `unwrap()` outside tests; comments say why,
> not what; `#[cfg(test)] mod tests` last in a file, or a `*_tests.rs`
> sibling as the crate already does; no new file over 500 lines; every log
> event pairs an id with its name (Rule 6). Test names say what they prove.
> Every test that needs a database has `live_db` in its fn or module name and
> uses `require_db_or_skip!`.

## Contents

- [Contract](#contract)
- [FX-01 Spec schema](#fx-01-spec-schema)
- [FX-02 Cell primitive force_mission_state](#fx-02-cell-primitive-force_mission_state)
- [FX-03 Base messages: item count and tutorials](#fx-03-base-messages-item-count-and-tutorials)
- [FX-04 Lab-only .fixture console family](#fx-04-lab-only-fixture-console-family)
- [FX-05 Runner plan and RunRequest.instance](#fx-05-runner-plan-and-runrequestinstance)
- FX-06 to FX-13 (runner establish and verify, the spec, docs, snapshots,
  rewind, live UAT, close-out):
  [work-packets-runner-rewind.md](work-packets-runner-rewind.md)

## Contract

Parallel packets build against these names. A packet that needs to change
one stops and tells the coordinator.

### Spec types (`crates/lab/src/uat/spec.rs`, FX-01)

One block, headed `// ── Row fixtures (lab-fixtures FX-01) ──`, below
`EvidenceSpec`. lab-spec-vocab also edits this file (README, "Contract
collisions").

```rust
// SectionMeta gains (after `fresh`):
/// Reusable fixture characters, by profile name (`[section.profiles.praxis]`).
#[serde(default)]
pub profiles: std::collections::BTreeMap<String, FreshCharacter>,

// RowSpec gains (after `evidence`):
/// The state the runner establishes before setup (lab-fixtures).
#[serde(default)]
pub fixture: Option<FixtureSpec>,

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureSpec {
    /// A `[section.profiles]` key.
    pub character: String,
    #[serde(default)]
    pub world: Option<String>,
    /// Server metres, as `/gmgotoxyz` takes them.
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub missions: Vec<FixtureMission>,
    #[serde(default)]
    pub items: Vec<FixtureItem>,
    #[serde(default)]
    pub tutorials_unseen: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureMission {
    pub id: i32,
    /// Active on this step. Implies `state = "active"`.
    #[serde(default)]
    pub step: Option<i32>,
    #[serde(default)]
    pub state: Option<FixtureMissionState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMissionState { Active, NotActive, Completed }

#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureItem {
    /// Item design id (`sgw_inventory.type_id`).
    pub id: i32,
    /// Exact carried count after the fixture, 0 to 10 (D-FX7).
    pub count: u32,
    /// `main` (default, container 1, `INV_MAIN`) or `mission` (container 2, `INV_MISSION`, crates/entity/src/inventory.rs:14).
    #[serde(default)]
    pub container: Option<String>,
}

impl FixtureMission {
    /// `step` set means Active; no `step` and no `state` is a spec error.
    pub fn resolved_state(&self) -> Result<FixtureMissionState, String>;
}
impl FixtureItem {
    /// 1 for `main`/unset, 0 for `mission`, Err otherwise.
    pub fn container_id(&self) -> Result<i32, String>;
}
impl FixtureSpec {
    /// True when the fixture writes server state (missions, items or
    /// tutorials), so it ends with a relog (D-FX5).
    pub fn writes_state(&self) -> bool;
}
```

### The `.fixture` console family (FX-04)

Lab-only (D-FX2): reached only through `LabConsoleExec`, acting entity a
GameMaster, acting on that entity only. Grammar and reply lines, which the
runner matches exactly:

| Line | Success reply (one feedback line) |
|---|---|
| `.fixture mission <missionId> <stepId>` | `fixture mission <missionId>: active on step <stepId>` |
| `.fixture mission <missionId> not_active` | `fixture mission <missionId>: not_active` |
| `.fixture mission <missionId> completed` | `fixture mission <missionId>: completed` |
| `.fixture item <designId> <count> [main\|mission]` | `fixture item <designId>: set to <count> (sent)` |
| `.fixture tutorial <tutorialId> forget` | `fixture tutorial <tutorialId>: forget (sent)` |

Every refusal starts `.fixture` and a colon: `.fixture: ...` or
`.fixture <verb>: ...`. The runner's error regex is
`^\.fixture\b[^:]*: `. From the chat path, any `.fixture` line from a GM
gets exactly `.fixture: lab-only; run it through the lab endpoint
(server_console_exec)` and changes nothing.

### Cell primitive (`crates/cell-content/src/cell/missions/forced.rs`, FX-02)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedMission { Active { step_id: i32 }, NotActive, Completed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceOutcome {
    Set,
    NoEntity,
    UnknownMission,
    /// The step is not one of the mission's (`SpaceManager::step_missions`).
    StepNotInMission { owner: Option<i32> },
}

/// Put `mission_id` into exactly `want` on `entity_id`, whatever it was,
/// with `repeats = 0`, and persist it. Fires no content events (D-FX4).
pub async fn force_mission_state(
    entity_id: u32,
    mission_id: i32,
    want: ForcedMission,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> ForceOutcome;
```

Re-exported from `crates/cell-content/src/cell/missions/mod.rs` as
`pub use forced::{force_mission_state, ForceOutcome, ForcedMission};`.

### Base messages (`crates/wire/src/cell/messages/cell_to_base.rs`, FX-03)

```rust
/// Lab fixture (lab-fixtures FX-03): leave the player carrying exactly
/// `count` of design `type_id`: every carried instance is removed, then
/// `count` are granted into `container_id`. Never touches a vault or bank.
FixtureSetItemCount { entity_id: u32, player_id: i32, type_id: i32, count: i32, container_id: i32 },
/// Lab fixture (FX-03): delete these `sgw_player_tutorials` rows, so the
/// next `show_tutorial` displays them again.
FixtureForgetTutorials { entity_id: u32, player_id: i32, tutorial_ids: Vec<i32> },
```

### Runner (`crates/lab/src/uat/fixture.rs`, FX-05; `runner/fixture.rs`, FX-06 and FX-07)

```rust
// crates/lab/src/uat/fixture.rs (pure)
pub const FIXTURE_CHARACTER_VAR: &str = "fixture_character";
pub const FIXTURE_ERROR_REPLY: &str = r"^\.fixture\b[^:]*: ";
/// `Fx` + the profile's ASCII letters (lowercase, at most 8) + the instance
/// label's letters (digits 0-9 become a-j, other characters dropped, at
/// most 4), first letter upper case. ("praxis", "default") -> "Fxpraxisdefa";
/// ("praxis", "p2") -> "Fxpraxispc".
pub fn fixture_character_name(profile: &str, instance: &str) -> String;
/// One console line and the regex its success reply must match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureLine { pub line: String, pub reply: String }
/// Missions, then items, then tutorials, in spec order.
pub fn console_lines(f: &FixtureSpec) -> Result<Vec<FixtureLine>, String>;
/// One read-only check: a label for the BLOCKED reason, the SQL, a JSON
/// pointer into the `server_db_query` result and the value it must equal.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureCheck { pub what: String, pub sql: String, pub pointer: String, pub want: serde_json::Value }
/// `name` must be ASCII letters only (it is formatted into SQL).
pub fn verify_checks(f: &FixtureSpec, name: &str) -> Result<Vec<FixtureCheck>, String>;

// runner/mod.rs: RunRequest gains
/// The lab instance label this run drives (`default`, `p2`, ...).
pub instance: Option<String>,

// runner/fixture.rs
impl<I: ToolInvoker> Runner<'_, I> {
    pub(crate) async fn establish_fixture(&mut self, spec: &SectionSpec, f: &FixtureSpec, ctx: &mut RowCtx) -> Result<(), String>; // FX-06
    pub(crate) async fn verify_fixture(&mut self, f: &FixtureSpec, ctx: &mut RowCtx) -> Result<(), String>;                         // FX-07
}
```

## FX-01 Spec schema

**Implementer:** packet-coder. **Size:** S. **Wave:** 1. **Depends on:** none.
**Branch:** `lab-fixtures/fx01-spec-schema`. **Worktree:** `fx01`.
**Subject:** `feat(lab): FX-01 row fixture and profile schema in UAT specs`

Why: the runner and the spec need one shape (README Purpose).

Files:

1. `crates/lab/src/uat/spec.rs`: add the contract block (the types, the two
   fields, the three `impl` methods). `resolved_state`: `step` Some and
   `state` None or Some(Active) is Active; `step` None and `state` Some(s)
   is s; `step` Some with `state` NotActive or Completed is an Err;
   neither is an Err. `container_id`: None or `"main"` is 1, `"mission"` is
   2 (`INV_MISSION`), anything else an Err naming the value.
2. `crates/lab/src/uat/spec_validate.rs`: in `validate`, for every
   `[section.profiles]` entry check `alignment`, `archetype`, `gender` are
   non-empty; then per row with a fixture call a new
   `fn check_fixture(r: &str, f: &FixtureSpec, spec: &SectionSpec, row: &RowSpec, errs: &mut Vec<String>)`:
   - `character` is a key of `spec.section.profiles` (else
     `"{r}/fixture: no [section.profiles.{c}]"`);
   - `row.state` is `"in_world"` or `"any"`;
   - `row.players == 1` (two-player fixtures are out of scope);
   - each mission: `id > 0`, `resolved_state()` is Ok, `step > 0` when set,
     no duplicate ids;
   - each item: `id > 0`, `count <= 10`, `container_id()` is Ok, no
     duplicate ids;
   - each tutorial id `> 0`, no duplicates;
   - `world`, when set, is non-empty and has no whitespace;
   - `position`, when set, has three finite numbers.
3. `crates/lab/src/uat/spec_tests.rs`: the tests below.

Tests (unit, TESTING.md type 1):

- `a_fixture_row_parses_with_missions_items_and_tutorials`: a section with
  `[section.profiles.praxis]` and the README's example fixture parses;
  assert the mission states (`Active` for both), the item container id (1),
  and `writes_state() == true`.
- `a_position_only_fixture_does_not_write_state`.
- `fixture_spec_errors_name_the_row`: one table-driven test over bad
  fixtures (unknown profile, `state = "char_select"`, `step` with
  `state = "completed"`, neither, `count = 11`, `container = "bank"`,
  duplicate mission id, NaN position via `position = [nan, 0, 0]`), each
  asserting `parse` fails with the row id and the field in the message.
  Fails if any check in `check_fixture` is removed.
- The existing tests stay green, and every committed spec still parses
  (`uat::tests::committed_specs_parse_and_validate`, `crates/lab/src/uat/mod.rs:126`).

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09 writes the authoring section).

Reviewer focus: `deny_unknown_fields` on every new struct; the new fields
default so every committed spec parses unchanged; no edits outside the
delimited block in `spec.rs` (lab-spec-vocab collision).

## FX-02 Cell primitive force_mission_state

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 1. **Depends on:** D-FX1, D-FX4.
**Branch:** `lab-fixtures/fx02-force-mission`. **Worktree:** `fx02`.
**Subject:** `feat(cell-content): FX-02 force_mission_state for lab fixtures`
**Reviewers:** packet-reviewer, mission-systems-advisor, server-authority-enforcer.

Why: README F2, F16. Nothing can put a completed mission back on a step.

Files:

1. New `crates/cell-content/src/cell/missions/forced.rs` with the contract
   types and `force_mission_state`, as `#[tracing::instrument(name = "mission.force", level = "info", skip_all, fields(entity_id, mission_id))]`:
   1. `space_mgr.get_entity(entity_id)` is None: `NoEntity`.
   2. `space_mgr.mission_defs.get(&mission_id)` is None: `UnknownMission`.
   3. `Active { step_id }`: `space_mgr.step_missions.get(&step_id)` must be
      `Some(mission_id)`, else `StepNotInMission { owner }` (the same check
      `gm/missions.rs:286-313` makes).
   4. Remove any instance: `entity.missions.remove_mission(mission_id)`
      (any status, README F16), keeping whether it was hidden.
   5. `NotActive`: send `CellToBaseMsg::MissionUpdate` with
      `MISSION_NOT_ACTIVE`, no step, empty lists, `repeats: 0` (the base
      deletes the row only for 0, `lifecycle.rs:204-208`); if something was
      removed and it was not hidden, send the same removal frame
      `abandon_mission` sends (`lifecycle.rs:283-294`). Return `Set`.
   6. `Active` and `Completed`: build the first step's objectives from the
      def exactly as `handle_mission_assign` does (`gm/missions.rs:101-112`),
      then `accept_mission(entity_id, mission_id, def.step_id, objectives, tx, space_mgr)`
      (the guard passes: there is no prior instance, so repeats start at 0).
      If it returns false, log a warn and return `NoEntity`.
   7. `Active` with `step_id != def.step_id`: `advance_step(entity_id, mission_id, step_id, tx, space_mgr)`.
   8. `Completed`: `complete_mission_direct(entity_id, mission_id, tx, space_mgr)`.
   9. Persist: `send_mission_update(entity_id, player_id, mission_id, "lab_fixture", tx, space_mgr)`.
      Never call `fire_mission_accepted`, `fire_step_activation_regions`,
      Discord or rewards (D-FX4).
   10. One `info` event `"mission forced"` with `entity_name`,
       `mission_name` and `step_name` from the NameBook (Rule 6), and the
       previous status.
2. `crates/cell-content/src/cell/missions/mod.rs`: `mod forced;` and the
   re-export.
3. New `crates/cell-content/src/cell/missions/forced_tests.rs`, wired as
   `#[cfg(test)] #[path = "forced_tests.rs"] mod tests;` at the end of
   `forced.rs`. Build the `SpaceManager` and a player entity the way
   `progression_tests.rs` does, with a two-step mission def.

Tests (unit with the cell's in-memory `SpaceManager`, TESTING.md type 1;
drain the `tx` receiver to assert the persisted `MissionUpdate`):

- `forcing_a_completed_mission_to_a_step_makes_it_active_there_and_saves_it`:
  complete the mission first (`complete_mission_direct`), force
  `Active { step_id: <second step> }`; assert the instance is active on the
  second step with `repeats == 0`, and the last `MissionUpdate` drained has
  `status == MISSION_ACTIVE`, `current_step_id == Some(second)`. Fails if
  step 4 is reverted (accept is refused for a completed mission at its cap).
- `forcing_not_active_saves_repeats_zero_so_the_row_is_deleted`: with an
  instance at `repeats = 2`, force `NotActive`; the `MissionUpdate` has
  `status == MISSION_NOT_ACTIVE` and `repeats == 0`, and no instance is
  left.
- `a_step_from_another_mission_is_refused_and_nothing_changes`: assert
  `StepNotInMission { owner: Some(other) }` and that nothing was sent.
- `forcing_sends_only_mission_frames_and_the_save`: force `Active` on a
  mission with a `mission_accepted` chain and a reward in its def; assert
  every drained message is an `EntityMethodCall` with a mission method
  index or a `MissionUpdate`, nothing else. This guards D-FX4 against a
  later refactor that adds an engine argument and fires the accept chain.

Checks: the three lane commands, `-p cimmeria-cell-content`.

Docs owed: none here; FX-09 notes the primitive in
`docs/protocol/cell-method-dispatch-table.md` row 117 ("primitive exists,
native binding not built").

Reviewer focus: hidden missions stay hidden (no frames); the step-ownership
check runs before anything is removed; nothing persists when the result is
not `Set`.

## FX-03 Base messages: item count and tutorials

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 1. **Depends on:** D-FX1, D-FX7.
**Branch:** `lab-fixtures/fx03-base-fixture`. **Worktree:** `fx03`.
**Subject:** `feat(base): FX-03 base handlers to set an item count and forget tutorials for lab fixtures`
**Reviewers:** packet-reviewer, items-systems-advisor, server-authority-enforcer.

Why: README F10 and D-FX7. The cell owns neither inventory nor tutorials.

Files:

1. `crates/wire/src/cell/messages/cell_to_base.rs`: the two contract
   variants, after `RecordTutorialShown`.
2. `crates/base-world-entry/src/base/world_entry/cell_dispatch/mod.rs`:
   route `FixtureSetItemCount` with the inventory arm (line ~256-269) and
   `FixtureForgetTutorials` next to `RecordTutorialShown` (line ~217).
   Fix any exhaustive `match` elsewhere the compiler names.
3. New `crates/base-world-entry/src/base/world_entry/fixture_items.rs`:
   `pub(crate) async fn handle_fixture_set_item_count(...)` with the same
   context arguments `remove_inventory_item_by_type` takes
   (`inventory_dispatch.rs:118-137`). Steps:
   1. Resolve the owning account from the session (`identity_for_entity`,
      as `shown_tutorials.rs` does) and refuse when `player_id` is not that
      account's character (warn, nothing written).
   2. Refuse `count` outside 0..=10 and `container_id` other than 0 or 1.
   3. Remove every instance of `type_id` in the player's carried containers
      through the same removal path `RemoveInventoryItem` uses (so the
      client gets `onRemoveItem` and a bandolier item clears its slot).
      "Carried" excludes the vault (17) and any bank container; name the
      set as `pub(crate) const FIXTURE_CARRIED_CONTAINERS` and write it in
      the worknote for FX-05.
   4. Grant `count` through the `GrantItem` path
      (`progression_dispatch.rs:133`) into `container_id`, `notify_gm: false`.
   5. One `info` event `"lab fixture item count set"` with `player_name`,
      `item_name` (NameBook), removed and granted counts.
4. `crates/base-world-entry/src/base/world_entry/shown_tutorials.rs`: add
   `pub(crate) async fn handle_fixture_forget_tutorials(...)`: one
   `DELETE FROM sgw_player_tutorials t USING sgw_player p WHERE t.player_id = p.player_id AND p.player_id = $1 AND p.account_id = $2 AND t.tutorial_id = ANY($3)`
   with the session's account as the ownership predicate; log the deleted
   count. If the file passes 500 lines, put this in a new
   `fixture_tutorials.rs` instead.
5. `crates/base-world-entry/src/base/world_entry/mod.rs`: `mod fixture_items;`.

Tests:

- Wire round trip, if the crate serialises `CellToBaseMsg` (check for an
  existing serde or debug round-trip test and add both variants; skip if
  the enum is in-process only).
- `live_db_fixture_item_count_replaces_every_carried_instance` (TESTING.md
  type 3): seed a sentinel player with two instances of a design id in the
  backpack and one in the bandolier and one in the vault; run the handler
  with `count = 1, container_id = 1`; assert exactly one carried instance
  in container 1 and the vault instance untouched. Fails if step 3 removes
  only the first instance (what `RemoveInventoryItemByType` does) or
  touches the vault.
- `live_db_fixture_item_count_refuses_another_accounts_character`: nothing
  changes.
- `live_db_fixture_forget_tutorials_deletes_only_the_named_rows`: two
  tutorial rows, forget one; the other remains. Then the existing
  `record_tutorial_shown` returns `Inserted` again for the forgotten one,
  which is the property FS-P4 needs (README F10).

Checks: the three lane commands with `-p cimmeria-wire`, then
`-p cimmeria-base-world-entry`; then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 live_db_fixture`.
Confirm the crate is in `tools/test-live-db.ps1`'s list (it has live-DB
tests already).

Docs owed: none here (FX-09).

Reviewer focus: the vault and bank are never touched; the ownership
predicate is the session's account, not anything the cell sent; removing an
equipped weapon clears its bandolier slot the way a normal removal does.

## FX-04 Lab-only .fixture console family

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 2. **Depends on:** FX-02, FX-03, D-FX2.
**Branch:** `lab-fixtures/fx04-fixture-console`. **Worktree:** `fx04`.
**Subject:** `feat(console): FX-04 lab-only .fixture commands through server_console_exec`
**Reviewers:** packet-reviewer, server-authority-enforcer.

Why: README F4, F5; D-FX1 (a), D-FX2 (a).

Files:

1. New directory `crates/cell-console/src/cell/console/fixture/` (four
   files from day one, CLAUDE.md foresight rule):
   - `mod.rs`: `pub(crate) const FIXTURE_COMMAND: &str = "fixture";`,
     `pub(crate) async fn exec(caller_id: u32, args: &[&str], tx, space_mgr)`
     routing on `args[0]` (`mission`, `item`, `tutorial`), a usage line
     `.fixture: usage .fixture mission|item|tutorial ...` otherwise.
     Logs one `info` `"lab fixture"` event with the GM's identity
     (`space_mgr.player_identity(caller_id)`, as `mission.rs:85-100`) and
     the line.
   - `mission.rs`: parse `<missionId> <stepId|not_active|completed>` with
     `super::super::parse_i32`; call `force_mission_state` on `caller_id`;
     map each `ForceOutcome` to the contract reply or a `.fixture mission:`
     refusal naming the mission and the step owner.
   - `items.rs`: parse `<designId> <count> [main|mission]`; refuse a count
     above 10; resolve `player_id` from the caller entity; send
     `FixtureSetItemCount`; reply `fixture item <id>: set to <n> (sent)`.
   - `tutorials.rs`: `<tutorialId> forget`; send
     `FixtureForgetTutorials { tutorial_ids: vec![id] }`; reply
     `fixture tutorial <id>: forget (sent)`.
   Every handler acts on `caller_id` only; none reads a target or a name.
2. `crates/cell-console/src/cell/console/dispatch.rs`:
   - New `pub async fn handle_lab_console_command(caller_id, text, tx, space_mgr, engine)`:
     if the first word after `.` is `fixture`, call `fixture::exec`;
     otherwise call `handle_console_command` unchanged.
   - In `handle_console_command`, before the registry lookup: a `fixture`
     line gets the contract's chat refusal and returns. (A GM's chat
     `.fixture` reaches only this function.)
3. `crates/cell-console/src/cell/console/mod.rs`: `mod fixture;`,
   `pub use dispatch::handle_lab_console_command;`, and in
   `command_catalog()` append one `CommandInfo` for `fixture` whose `help`
   starts `Lab only (server_console_exec):`. Add `pub lab_only: bool` to
   `CommandInfo` (false for every registry row).
4. `crates/cell/src/cell/service/base_messages/lab_console.rs:63`: call
   `console::handle_lab_console_command` instead of
   `handle_console_command`. The GM gate above it stays as is.
5. `crates/lab-mcp/src/tools/console.rs`: add `"lab_only": c.lab_only` to
   each listed command.

Tests:

- `crates/cell-console/src/cell/console/tests/fx04_fixture.rs` (new,
  registered in `tests/mod.rs`; unit with the in-memory `SpaceManager`,
  TESTING.md type 1):
  - `chat_fixture_line_from_a_gm_is_refused_and_changes_nothing`: a GM
    sends `.fixture mission 622 completed` through `handle_console_command`;
    assert the one refusal line and no `MissionUpdate` sent. Fails if the
    chat-path refusal (file 2, second bullet) is removed: `fixture` is not
    in the registry, so the line would fall to "unknown command" today, and
    a later registry entry would open the chat path.
  - `lab_fixture_mission_forces_a_completed_mission_back_to_a_step` through
    `handle_lab_console_command`; assert the reply text exactly.
  - `lab_fixture_item_sends_the_count_message_for_the_caller_only`: the
    caller has a selected target; the message carries the caller's
    `player_id`, never the target's.
  - `lab_fixture_refuses_a_count_above_ten`.
- `crates/cell/src/cell/service/base_messages/tests/lab_console.rs`: add
  `lab_console_exec_routes_fixture_lines` (the existing non-GM refusal test
  already covers the GM gate; keep it green).
- `crates/cell-console/src/cell/console/registry/commands/mod.rs` tests
  stay green (`fixture` is not in `COMMANDS`).

Checks: the three lane commands with `-p cimmeria-cell-console`, then
`-p cimmeria-cell`, then `-p cimmeria-lab-mcp`.

Docs owed: none here (FX-09).

Reviewer focus: no path from chat to `fixture::exec`; no handler takes a
target; `lab_only` defaults false everywhere else.

## FX-05 Runner plan and RunRequest.instance

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** FX-01, D-FX3.
**Branch:** `lab-fixtures/fx05-runner-plan`. **Worktree:** `fx05`.
**Subject:** `feat(lab): FX-05 fixture character names, console lines and verify checks`

Why: README F13, F14. Everything the establish step needs that has no I/O.

Files:

1. New `crates/lab/src/uat/fixture.rs` with the pure contract items;
   `crates/lab/src/uat/mod.rs`: `pub mod fixture;`.
   - `fixture_character_name`: per the contract doc comment.
   - `console_lines`: per mission `.fixture mission <id> <step>` with reply
     `^fixture mission <id>: active on step <step>$`, or `not_active` /
     `completed`; per item `.fixture item <id> <count> main|mission` with
     reply `^fixture item <id>: set to <count> \(sent\)$`; per tutorial
     `.fixture tutorial <id> forget`. Escape nothing: every value is a
     validated integer.
   - `verify_checks(f, name)`: Err unless `name` is ASCII letters only.
     Lower-case `name` as `n`. Per mission: SQL
     `SELECT count(*) AS n_rows, max(m.status) AS status, max(m.current_step_id) AS step FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name ILIKE '%{n}%' AND m.mission_id = {id}`;
     Active wants pointer `/rows/0/step` == step (and a second check
     `/rows/0/status` == 1); Completed `/rows/0/status` == 2; NotActive
     `/rows/0/n_rows` == 0. Per item:
     `SELECT coalesce(sum(i.stack_size), 0) AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id WHERE p.player_name ILIKE '%{n}%' AND i.type_id = {id} AND i.container_id IN ({FIXTURE_CARRIED_CONTAINERS})`
     wants `/rows/0/n` == count; take the container list from FX-03's
     worknote into `pub const FIXTURE_CARRIED_CONTAINERS: &[i32]` here, with
     a comment naming the base constant it mirrors. Per tutorial: count of
     `sgw_player_tutorials` rows wants 0.
2. `crates/lab/src/uat/runner/mod.rs`: add `instance` to `RunRequest`
   (contract); in `base_vars` insert `"instance"` when set.
3. `crates/lab/src/server/uat.rs:385-399`: set
   `instance: Some(self.supervisor.label().to_string())`.
4. Fix every other `RunRequest { .. }` literal the compiler names (tests
   use `..Default::default()` mostly).

Tests (unit, TESTING.md type 1, in a `#[cfg(test)] mod tests` at the end of
`fixture.rs`):

- `fixture_names_are_letters_and_distinct_per_instance`: the two contract
  examples, plus `("praxis", "p3") != ("praxis", "p2")`, and every output
  matches `^[A-Z][a-z]+$`.
- `console_lines_cover_each_mission_state_item_and_tutorial`: the README
  example fixture gives exactly the expected lines, in order, and each
  reply regex matches the FX-04 contract reply and not the other states'.
- `the_error_regex_matches_refusals_only`: `.fixture: usage ...`,
  `.fixture mission: step 4037 belongs to mission 1360, not 622` match;
  every contract success reply does not.
- `verify_checks_refuse_a_name_that_is_not_letters`: `"Fx'; drop"` is an
  Err. Fails if the letters guard is removed (SQL injection through a
  profile name is the bug shape).
- `base_vars_carry_the_instance`: a runner built with `instance = "p2"`
  puts `"p2"` in the row vars.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).
