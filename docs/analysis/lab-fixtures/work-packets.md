# Lab row fixtures and rewind: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku),
> `rust-gameserver-dev` for FX-02, FX-03 and FX-04, and `packet-reviewer`
> reviewers (Sonnet). This file holds the dispatch header, the contract and
> FX-01 to FX-05; FX-06 to FX-13 are in
> [work-packets-runner-rewind.md](work-packets-runner-rewind.md). Ledger,
> findings (F1 to F24) and decisions (D-FX1 to D-FX10): [README.md](README.md).
> Revised 2026-10-10 for the staged-fixture design (D-FX1, D-FX2, D-FX5).
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
- [FX-02 Base fixture store and apply at Play](#fx-02-base-fixture-store-and-apply-at-play)
- [FX-03 Base exact item counts](#fx-03-base-exact-item-counts)
- [FX-04 Lab endpoint tool server_stage_fixture](#fx-04-lab-endpoint-tool-server_stage_fixture)
- [FX-05 Runner plan](#fx-05-runner-plan)
- FX-06 to FX-13 (runner sessions and verify, the spec, docs, snapshots
  and parity, rewind, live UAT, close-out):
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
    /// 1 for `main`/unset, 2 for `mission` (`INV_MISSION`), Err otherwise.
    pub fn container_id(&self) -> Result<i32, String>;
}
impl FixtureSpec {
    /// True when the fixture writes server state (missions, items or
    /// tutorials), so the runner stages it before the session's one world
    /// entry (D-FX5).
    pub fn writes_state(&self) -> bool;
}
```

### The staged-fixture store (`crates/base-session/src/base/world_entry/lab_fixture.rs`, FX-02)

Process-wide, the same shape as `gm_only_worlds.rs:146-189` (a
`OnceLock<Mutex<HashMap<i32, (Instant, StagedFixture)>>>`). Re-exported
from the `world_entry` module so `cimmeria_services::base` reaches it.

```rust
/// A staged fixture expires this long after staging (D-FX2).
pub const STAGED_FIXTURE_TTL: Duration = Duration::from_secs(600);
/// D-FX7.
pub const FIXTURE_MAX_ITEM_COUNT: u32 = 10;
/// Containers the fixture counts as carried: main through crafting.
/// 16 to 20 (buyback, bank, auction, team and command banks) are never
/// touched (crates/entity/src/inventory.rs:13-32).
pub const FIXTURE_CARRIED_CONTAINERS: std::ops::RangeInclusive<i32> = 1..=15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagedMissionState { Active { step_id: i32 }, NotActive, Completed }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedMission { pub mission_id: i32, pub state: StagedMissionState }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedItem { pub type_id: i32, pub count: u32, pub container_id: i32 }

#[derive(Debug, Clone, PartialEq)]
pub struct StagedFixture {
    pub player_id: i32,
    /// The account that owns `player_id` (`sgw_player.account_id`), filled
    /// by the stager. Only a session of this account takes the fixture.
    pub account_id: i32,
    /// For logs (Rule 6).
    pub player_name: String,
    /// A `sgw_player.world_location` value (a world name).
    pub world: Option<String>,
    pub position: Option<[f32; 3]>,
    pub missions: Vec<StagedMission>,
    pub items: Vec<StagedItem>,
    pub tutorials_unseen: Vec<i32>,
}

/// Stage `f` for its player, replacing any earlier one. Returns true when
/// one was replaced. For the lab endpoint only (FX-04 owns the GM and
/// offline gates, and fills `account_id`).
pub fn stage_fixture(f: StagedFixture) -> bool;
/// Remove and return the player's fixture when it was staged for
/// `account_id` (the playing session's account); None when none is staged,
/// it is older than `STAGED_FIXTURE_TTL` (dropped with a WARN), or it is
/// another account's (left staged, WARN `reason = "account_mismatch"`).
/// FX-02 review: replaced `take_fixture(player_id)`, which let any account
/// burn another character's fixture through `playCharacter(victim_id)`.
pub fn take_fixture_for(player_id: i32, account_id: i32) -> Option<StagedFixture>;
/// True while one is staged and not expired (the tool's reply, tests).
pub fn fixture_pending(player_id: i32) -> bool;
```

### The apply (`crates/base-world-entry/src/base/world_entry/lab_fixture/`, FX-02 and FX-03)

```rust
// mod.rs
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FixtureApplied { pub missions: usize, pub items_removed: u64, pub items_granted: u64, pub tutorials_forgotten: u64, pub moved: bool }
/// Write `f` for `player_id` (owned by `account_id`). Called from
/// `handle_play_character` before `query_world_entry`.
/// (FX-02 review: the error is `ApplyError::{Precheck, Write}(String)`, so
/// the "not applied" ERROR carries `reason` precheck_refused / write_failed.)
pub(crate) async fn apply_staged_fixture(pool: &PgPool, account_id: i32, f: &StagedFixture) -> Result<FixtureApplied, ApplyError>;
// missions.rs (FX-02; checks in the precheck, writes after it)
pub(crate) async fn plan_missions(pool: &PgPool, missions: &[StagedMission]) -> Result<Vec<MissionRow>, String>;
pub(crate) async fn write_missions(pool: &PgPool, player_id: i32, rows: &[MissionRow]) -> Result<usize, String>;
// items.rs (FX-03; split like the missions: the checks and a rolled-back dry
// run of the write in the precheck, then one transaction; worknotes/FX-03.md)
pub(crate) async fn plan_items(pool: &PgPool, player_id: i32, items: &[StagedItem]) -> Result<Vec<ItemPlan>, String>;
pub(crate) async fn write_items(pool: &PgPool, player_id: i32, plans: &[ItemPlan]) -> Result<(u64, u64), String>;
// tutorials.rs and position.rs (FX-02)
pub(crate) async fn forget_tutorials(pool: &PgPool, player_id: i32, ids: &[i32]) -> Result<u64, String>;
pub(crate) async fn move_player(pool: &PgPool, player_id: i32, world: Option<&str>, position: Option<[f32; 3]>) -> Result<bool, String>;
```

### The endpoint tool (`crates/lab-mcp/src/tools/fixture.rs`, FX-04)

`server_stage_fixture`. Its arguments are the JSON of `FixtureSpec` with
`character` set to the character's name (exact, case-insensitive, matched
against `sgw_player.player_name` or its last space-separated word; exactly
one character must match):

```json
{ "character": "Fxpraxisdefa", "world": "Castle_CellBlock",
  "position": [-319.0, 73.6, -212.5],
  "missions": [{ "id": 622, "step": 80623 }, { "id": 1360, "state": "not_active" }],
  "items": [{ "id": 55, "count": 0 }, { "id": 3730, "count": 0, "container": "mission" }],
  "tutorials_unseen": [5882] }
```

Success: `{ "staged": true, "player_id": 123, "player_name": "Uat Fxpraxisdefa", "replaced": false }`.
Every refusal is an `invalid_params` error whose message starts
`stage refused: ` and names the field (`stage refused: character Fxpraxisdefa is online`).

### Runner (`crates/lab/src/uat/fixture.rs`, FX-05; `runner/fixture.rs`, FX-06 and FX-07)

```rust
// crates/lab/src/uat/fixture.rs (pure)
pub const FIXTURE_CHARACTER_VAR: &str = "fixture_character";
/// `Fx` + the profile's ASCII letters (lowercase, at most 8) + the instance
/// label's letters (digits 0-9 become a-j, other characters dropped, at
/// most 4), first letter upper case. ("praxis", "default") -> "Fxpraxisdefa";
/// ("praxis", "p2") -> "Fxpraxispc".
pub fn fixture_character_name(profile: &str, instance: &str) -> String;
/// The `server_stage_fixture` arguments: `f` as JSON, `character` = `name`.
pub fn stage_args(f: &FixtureSpec, name: &str) -> serde_json::Value;
/// Mirrors `FIXTURE_CARRIED_CONTAINERS` in base-session.
pub const FIXTURE_CARRIED_CONTAINERS: &[i32] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// One read-only check: a label for the BLOCKED reason, the SQL, a JSON
/// pointer into the `server_db_query` result and the value it must equal.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureCheck { pub what: String, pub sql: String, pub pointer: String, pub want: serde_json::Value }
/// `name` must be ASCII letters only (it is formatted into SQL).
pub fn verify_checks(f: &FixtureSpec, name: &str) -> Result<Vec<FixtureCheck>, String>;

// runner/mod.rs: RunRequest gains
/// The lab instance label this run drives (`default`, `p2`, ...).
pub instance: Option<String>,
/// When a row's fixture is applied (D-FX10). Default `Auto`.
pub fixtures: FixtureMode,

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMode { #[default] Auto, Always, Never }

// runner/fixture.rs
impl<I: ToolInvoker> Runner<'_, I> {
    pub(crate) async fn end_session(&mut self, ctx: &mut RowCtx) -> Result<(), String>;                                               // FX-06
    pub(crate) async fn establish_fixture(&mut self, spec: &SectionSpec, f: &FixtureSpec, ctx: &mut RowCtx) -> Result<(), String>;  // FX-06
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

## FX-02 Base fixture store and apply at Play

**Implementer:** rust-gameserver-dev. **Size:** L. **Wave:** 1. **Depends on:** D-FX1, D-FX2.
**Branch:** `lab-fixtures/fx02-base-apply`. **Worktree:** `fx02`.
**Subject:** `feat(base): FX-02 apply a staged lab fixture when the character enters the world`
**Reviewers:** packet-reviewer, mission-systems-advisor, server-authority-enforcer.

Why: README F2, F7, F10, F16, F19 to F22; D-FX1, D-FX2, D-FX4, D-FX5.

Files:

1. New `crates/base-session/src/base/world_entry/lab_fixture.rs`: the store
   contract. `take_fixture` checks the age against `STAGED_FIXTURE_TTL`.
   Register it in that directory's `mod.rs` and re-export the public names
   the way `gm_only_worlds` is.
2. New directory `crates/base-world-entry/src/base/world_entry/lab_fixture/`
   (five files, foresight rule), `mod lab_fixture;` in the parent `mod.rs`:
   - `mod.rs`: `FixtureApplied`, `apply_staged_fixture`. Order: missions,
     items, tutorials, position. On the first Err stop and return it (each
     part is its own transaction, so earlier parts stay written; the
     runner's verify step reports what is missing).
   - `missions.rs`: per mission, all through `handle_mission_update`
     (`crates/base-methods/src/base/world_entry/methods/missions/mod.rs:105`):
     - `Active { step_id }`: check
       `SELECT mission_id FROM resources.mission_steps WHERE step_id = $1`
       equals the mission (Err naming the owner otherwise, nothing
       written for this fixture: run every check before the first write).
       Write `status = MISSION_ACTIVE`, `current_step_id = step_id`,
       `repeats = 0`, `completed_step_ids` = the mission's steps ordered
       before `step_id` and `completed_objective_ids` = those steps'
       objectives (order by the same column the cell's mission cache
       orders steps by: `crates/cell-catalog/src/cell/spawner/missions.rs:47`,
       `:121`, `:137`), and no active objective ids (hydration rebuilds the
       roster, `mission_restore.rs:58-61`).
     - `Completed`: `status = MISSION_COMPLETED`, no current step,
       `completed_step_ids` = every step, `repeats` = 1 (what
       `complete_mission_direct` persists for a first completion; confirm
       against `crates/cell-content/src/cell/missions/` and say so in the
       commit body).
     - `NotActive`: `status = MISSION_NOT_ACTIVE`, `repeats = 0`, so the
       row is deleted (F16). A not-active row with repeats is not a state a
       fixture asks for.
   - `items.rs`: `apply_items` returning
     `Err("item fixtures land in FX-03")` for a non-empty list, Ok((0, 0))
     for an empty one.
   - `tutorials.rs`: `forget_tutorials`:
     `DELETE FROM sgw_player_tutorials WHERE player_id = $1 AND tutorial_id = ANY($2)`.
   - `position.rs`: `move_player`: one `UPDATE sgw_player SET` of
     `world_location` and/or `pos_x, pos_y, pos_z` (the statement shape of
     `persist_position`, `crates/base-world-entry/src/base/world_entry/cell_dispatch/position.rs:76-90`;
     copy it, do not widen that function's visibility). Ok(false) when both
     are None.
3. `crates/base-world-entry/src/base/world_entry/play_character.rs`: after
   the latch block (about `:83`) and before `query_world_entry`:

   ```rust
   if let Some(f) = lab_fixture_store::take_fixture(player_id) {
       // Lab-only (D-FX2): staged through the lab endpoint for a GM
       // account's character; applied here so every world-entry read
       // below sees it (README F19).
       if access_level < GM_ACCESS_LEVEL {
           tracing::warn!(player_id, player_name = %f.player_name, access_level, "lab fixture discarded: the session is not a GM");
       } else if let Some(pool) = db_pool {
           match lab_fixture::apply_staged_fixture(pool, account_id, &f).await {
               Ok(a) => tracing::info!(player_id, player_name = %f.player_name, ?a, "lab fixture applied"),
               Err(e) => tracing::error!(player_id, player_name = %f.player_name, error = %e, "lab fixture not applied"),
           }
       }
   }
   ```

   (Adapt names to the function's locals.) `apply_staged_fixture` also
   refuses when `sgw_player.account_id` for `f.player_id` is not
   `account_id`.

Tests:

- Store (unit, in `lab_fixture.rs`): `a_fixture_is_taken_once`;
  `staging_again_replaces_and_reports_it`; `an_expired_fixture_is_not_returned`
  (make the TTL check a function of `(staged_at, now)` so the test needs no
  sleep). Fails if `take_fixture` stops removing.
- Live-DB (TESTING.md type 3), module `lab_fixture_live_db_tests` in the
  new directory (a `tests.rs` sibling, `#[cfg(test)]`), sentinel player and
  account, a mission with at least three steps from the seed (622 with
  2113, 80623, 80622 orders them; read the order from
  `resources.mission_steps` in the test rather than hard-coding):
  - `live_db_fixture_moves_a_completed_mission_back_to_a_middle_step`:
    seed a completed 622 row with `repeats = 1`; apply
    `Active { step_id: second }`; assert `status = 1`,
    `current_step_id = second`, `repeats = 0`, and `completed_step_ids`
    holds exactly the first step. Fails if the completed-steps fill or the
    repeats reset is removed.
  - `live_db_fixture_not_active_deletes_the_row_even_with_repeats`.
  - `live_db_fixture_step_from_another_mission_writes_nothing`: one valid
    mission and one bad step in the same fixture; assert Err and that the
    valid mission's row is unchanged.
  - `live_db_fixture_forgets_only_the_named_tutorials`, then
    `record_tutorial_shown` returns `Inserted` for the forgotten one (F10).
  - `live_db_fixture_moves_world_and_position`.
  - `live_db_fixture_refuses_another_accounts_character`: nothing changes.
- Wiring (live-DB or the crate's existing `handle_play_character` test
  harness, whichever exists; say which in the commit body):
  `live_db_play_character_applies_a_staged_fixture_before_world_entry`:
  stage a position and a mission, run `handle_play_character` for a GM
  session, assert the `WorldEntryInfo` / `CreateEntity` position is the
  fixture's and the fixture is no longer pending; a non-GM session leaves
  the rows unchanged and the fixture consumed. Fails if the hook is moved
  after `query_world_entry`.

Checks: the three lane commands with `-p cimmeria-base-session`, then
`-p cimmeria-base-world-entry`; then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 live_db_fixture`
and `... live_db_play_character_applies`.

Docs owed: none here (FX-09). Write the mission row shape you settled on
(Completed repeats, step ordering column) in `worknotes/FX-02.md` for FX-05
and FX-10.

Reviewer focus: nothing reachable from a client message stages or applies
a fixture; every check runs before the first write; the hook sits before
every world-entry read (F19); a missing pool or non-GM session writes
nothing.

## FX-03 Base exact item counts

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 2. **Depends on:** FX-02.
**Branch:** `lab-fixtures/fx03-items`. **Worktree:** `fx03`.
**Subject:** `feat(base): FX-03 exact item counts in a staged lab fixture`
**Reviewers:** packet-reviewer, items-systems-advisor, server-authority-enforcer.

Why: README F23, D-FX7.

Files:

1. `crates/base-world-entry/src/base/world_entry/lab_fixture/items.rs`:
   replace FX-02's placeholder. One transaction for the whole list:
   1. Refuse any `count > FIXTURE_MAX_ITEM_COUNT` or `container_id` other
      than 1 or 2, and any `type_id` with no row in the item resources
      table, before writing.
   2. Per item: `DELETE FROM sgw_inventory WHERE character_id = $1 AND type_id = $2 AND container_id BETWEEN 1 AND 15`
      (bind the bounds from `FIXTURE_CARRIED_CONTAINERS`); count removed.
   3. Per item with `count > 0`: take slots with
      `reserve_free_inventory_slots` (`crates/base-methods/.../vendor/serializers.rs:159`,
      in the same transaction with its advisory lock) and insert with the
      column list of the grant INSERT (`inventory/grant/persist.rs:387-395`),
      splitting by the design's max stack (one row per stack). Not enough
      free slots: Err, transaction rolled back.
   4. Return `(removed, granted)`.
2. `lab_fixture/mod.rs`: nothing to change if FX-02 already calls
   `apply_items`; otherwise one line.

Tests (live-DB, TESTING.md type 3, in the directory's tests module):

- `live_db_fixture_item_count_replaces_every_carried_instance`: a sentinel
  player with two instances of a design id in the backpack, one in the
  bandolier container, one in the bank (16 or 17); apply `count = 1,
  container_id = 1`; assert exactly one carried instance, in container 1,
  and the bank instance untouched. Fails if step 2 deletes only one row or
  widens the container range.
- `live_db_fixture_item_count_zero_removes_all_carried`.
- `live_db_fixture_item_into_the_mission_inventory`.
- `live_db_fixture_items_are_all_or_nothing`: a full backpack makes the
  grant fail; assert the deletes rolled back.

Checks: the three lane commands with `-p cimmeria-base-world-entry`; then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 live_db_fixture_item`.

Reviewer focus: the bank range is never touched; an equipped item removed
by the fixture leaves no dangling bandolier reference on the next load
(the bandolier is read from inventory at `onClientReady`, F19; say how in
the commit body).

## FX-04 Lab endpoint tool server_stage_fixture

**Implementer:** rust-gameserver-dev. **Size:** M. **Wave:** 2. **Depends on:** FX-02.
**Branch:** `lab-fixtures/fx04-stage-tool`. **Worktree:** `fx04`.
**Subject:** `feat(lab-mcp): FX-04 server_stage_fixture stages a lab fixture for an offline GM character`
**Reviewers:** packet-reviewer, server-authority-enforcer.

Why: README F5, F6, F22; D-FX1, D-FX2.

Files:

1. New `crates/lab-mcp/src/tools/fixture.rs`: the argument struct (the
   contract's JSON, `deny_unknown_fields`) and
   `pub async fn stage_fixture(state: &LabState, args: StageFixtureArgs) -> Result<Value, String>`:
   1. No DB pool: `stage refused: no database`.
   2. Resolve the character: one `sgw_player` row whose `player_name`
      equals `character` case-insensitively, or whose last space-separated
      word does; zero or several rows is a refusal naming the count.
   3. `account.accesslevel >= GM_ACCESS_LEVEL` for its account, else
      `stage refused: character <name> is not on a GM account`.
   4. Not online (`state.online_players`, as `sessions.rs` reads it), else
      `stage refused: character <name> is online`.
   5. `world`, when set, names a world the server knows (the same lookup
      `.gotolocation` uses; cite it); missions exist; a `step` belongs to
      its mission (`resources.mission_steps`); items: `count <= 10`,
      `container` `main` or `mission`, design id exists; tutorial ids > 0;
      `position` finite. Refuse on the first failure, naming the field.
   6. Build the `StagedFixture` (`account_id` = the resolved row's
      `sgw_player.account_id`) and `stage_fixture` it; reply per the
      contract. Log one `info` `"lab fixture staged"` with
      `player_id`, `player_name` and the counts.
2. `crates/lab-mcp/src/tools/mod.rs`: register the tool next to
   `server_db_query`, with `audit::emit` on OK and ERROR like the others,
   and a description that says lab-only, offline GM character, applied at
   that character's next Play, taken once, 10-minute expiry.

Tests (in a `fixture_tests.rs` sibling, the crate's style):

- Live-DB (type 3): `live_db_stage_fixture_refuses_a_player_account`
  (fails if gate 3 is removed); `live_db_stage_fixture_refuses_a_step_from_another_mission`;
  `live_db_stage_fixture_stages_for_an_offline_gm_character` (then
  `fixture_pending` is true).
- Unit: an online character is refused (build `LabState` with the
  character in `online_players`, as `sessions_tests.rs` does).

Checks: the three lane commands with `-p cimmeria-lab-mcp`; then
`pwsh -NoProfile -File tools/build-lane/live-db-test.ps1 live_db_stage_fixture`.

Docs owed: none here (FX-09).

Reviewer focus: no path stages for a non-GM account or an online
character; the tool writes nothing to the database itself.

## FX-05 Runner plan

**Implementer:** packet-coder. **Size:** M. **Wave:** 2. **Depends on:** FX-01.
**Branch:** `lab-fixtures/fx05-runner-plan`. **Worktree:** `fx05`.
**Subject:** `feat(lab): FX-05 fixture character names, stage arguments and verify checks`

Why: README F13, F14. Everything the session step needs that has no I/O.

Files:

1. New `crates/lab/src/uat/fixture.rs` with the pure contract items;
   `crates/lab/src/uat/mod.rs`: `pub mod fixture;`.
   - `fixture_character_name`: per the contract doc comment.
   - `stage_args`: `serde_json::to_value(f)`, then set `character` to
     `name`; drop `null` fields.
   - `verify_checks(f, name)`: Err unless `name` is ASCII letters only.
     Lower-case `name` as `n`. Per mission: SQL
     `SELECT count(*) AS n_rows, max(m.status) AS status, max(m.current_step_id) AS step FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id WHERE p.player_name ILIKE '%{n}%' AND m.mission_id = {id}`;
     Active wants pointer `/rows/0/step` == step (and a second check
     `/rows/0/status` == 1); Completed `/rows/0/status` == 2; NotActive
     `/rows/0/n_rows` == 0. Per item:
     `SELECT coalesce(sum(i.stack_size), 0) AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id WHERE p.player_name ILIKE '%{n}%' AND i.type_id = {id} AND i.container_id IN ({FIXTURE_CARRIED_CONTAINERS})`
     wants `/rows/0/n` == count. Per tutorial: count of
     `sgw_player_tutorials` rows for the player and id wants 0.
2. `crates/lab/src/uat/runner/mod.rs`: add `instance` and `fixtures`
   (`FixtureMode`) to `RunRequest` (contract); in `base_vars` insert
   `"instance"` when set.
3. `crates/lab/src/server/uat.rs:385-399`: set
   `instance: Some(self.supervisor.label().to_string())`; `UatRunArgs`
   gains `#[serde(default)] pub fixtures: FixtureMode`, passed through.
4. Fix every other `RunRequest { .. }` literal the compiler names.

Tests (unit, TESTING.md type 1, in a `#[cfg(test)] mod tests` at the end of
`fixture.rs`):

- `fixture_names_are_letters_and_distinct_per_instance`: the two contract
  examples, plus `("praxis", "p3") != ("praxis", "p2")`, and every output
  matches `^[A-Z][a-z]+$`.
- `stage_args_match_the_tool_contract`: the README example fixture gives
  exactly the contract JSON (with `character` replaced) and no nulls.
- `verify_checks_cover_each_mission_state_item_and_tutorial`.
- `verify_checks_refuse_a_name_that_is_not_letters`: `"Fx'; drop"` is an
  Err. Fails if the letters guard is removed (SQL injection through a
  profile name is the bug shape).
- `base_vars_carry_the_instance`.
- `fixture_mode_defaults_to_auto_and_parses_snake_case`.

Checks: the three lane commands, `-p cimmeria-lab`.

Docs owed: none (FX-09).
