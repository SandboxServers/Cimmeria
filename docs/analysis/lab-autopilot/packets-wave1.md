# lab-autopilot wave 1: honest results

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 1
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md)

Wave 1 builds the honesty layer in the lab crate: the `proof` module (C1, C2), the evidence gate (C3), the runner's reading of verdicts, server corroboration, the stop report (C8) and `resume_from` (C10). It also splits the three files that block later waves (`server/mod.rs`, `uat/runner/mod.rs`, `supervisor/world/sim.rs`). Nothing here touches the client DLL.

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries); read that section first. Paths below are under `crates/lab/src/` unless they say otherwise.

**Dispatch:** AX-10 and AX-15 at once from main. When both are reviewed `ship` and merged into `lax-w1`, dispatch AX-11, AX-12, AX-13 and AX-14 at once, each at the `lax-w1` head. AX-16 waits for AX-12 merged. **Merge order:** AX-10, AX-15, AX-11, AX-12, AX-13, AX-14, AX-16. One PR from `lab-autopilot/wave1`.

### AX-10 The proof module: Evidence, Why, judge, and Steps::judged

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-10-proof` · **Worktree:** `lax-10`
- **Commit:** `feat(lab): evidence types and judge for honest tool results (AX-10)`
- **Rules:** R1-R4, R6, R8. Contract: [C1](contract.md#c1-evidence), [C2](contract.md#c2-why-codes).

Files and changes:

1. New `proof/mod.rs`: the C1 types, constructors and `judge` exactly as the contract gives them. First line after the module doc: `#![cfg_attr(not(test), allow(dead_code))]` with the comment `// Callers land in AX-11..AX-44; AX-44 removes this.` Declare `pub mod why; pub mod claims; pub mod gate; pub mod fresh; pub mod stop; pub mod corroborate;` and `pub use why::Why;`, and `#[cfg(test)] #[path = "tests.rs"] mod tests;`.
2. New `proof/why.rs`: `Why` with the 18 C2 variants, serde `snake_case`, `as_str`, `hint` (exact texts), and `pub const ALL: [Why; 18]` in table order.
3. New stub files, each holding one module doc line and nothing else, so later packets add no `mod` lines: `proof/claims.rs` (`//! Which tools claim an effect (C3). Filled by AX-11.`), `proof/gate.rs` (`//! The evidence gate (C3). Filled by AX-11.`), `proof/fresh.rs` (`//! Frame freshness of post-reads (C4). Filled by AX-22.`), `proof/stop.rs` (`//! Stop reports (C8). Filled by AX-14.`), `proof/corroborate.rs` (`//! Server corroboration. Filled by AX-13.`).
4. `main.rs`: add `mod proof;` between `mod otlp;` and `mod server;`.
5. `supervisor/world/mod.rs` (369 lines; this packet adds at most 25):
   - `Steps` (l.171-177) gains `evidence: Option<crate::proof::Evidence>`, `None` in `Steps::new`.
   - New `pub fn judged(&mut self, ev: crate::proof::Evidence)` storing it.
   - `finish` (l.239-246): when `evidence` is `Some(ev)`, set `out["evidence"] = ev.to_value()` and `counts_as_native_pass = self.level.counts_as_native() && ev.counts()`. When `None`, the output is byte-for-byte what it is today.
6. New `proof/tests.rs`.

Tests (`proof/tests.rs`, plus one in a new `supervisor/world/steps_tests.rs` wired from `world/mod.rs` with `#[cfg(test)] #[path = "steps_tests.rs"] mod steps_tests;`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `a_required_false_check_denies_with_its_why` | checks `[optional false, required true(why no_hover) , required false(why out_of_range)]` give `Denied`, why `OutOfRange` | rule 1 picks the first required false check |
| `a_required_unmeasured_check_is_inconclusive` | `[required true, required None]` gives `Inconclusive`, why `NoPostRead` | rule 2 |
| `no_required_checks_is_no_evidence` | `[optional true]` gives `Inconclusive`, `NoEvidence` | rule 3: optional checks never confirm |
| `all_required_true_confirms` | `Confirmed`, `Confirmed`, hint is `Why::Confirmed.hint()` | rule 4 |
| `evidence_round_trips_without_ok` | `to_value` of a check with `ok: None` has no `ok` key; `from_value` gives it back equal | compaction drops nulls (C1) |
| `why_serde_matches_as_str` | for every `Why::ALL`, `serde_json::to_value(w) == as_str()` and `hint()` is not empty | a drifting name would break readers |
| `finish_with_denied_evidence_does_not_count` (steps_tests) | `Steps` with `used(NativeLevel::RealInput)` and `judged(denied)`: `counts_as_native_pass` false, `evidence.verdict == "denied"`; without `judged`, no `evidence` key and `counts_as_native_pass` true | step 5 |

Checks: L1, L2. Report the `world/mod.rs` line count.

### AX-15 Mechanical splits: server/mod.rs input tools, runner row execution

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-15-splits` · **Worktree:** `lax-15`
- **Commit:** `refactor(lab): move input tools and row execution to their own files (AX-15)`
- **Rules:** R1-R4, R6. No behaviour change: the diff moves code and adds the router line and the `mod` lines; nothing else.

Files and changes:

1. `server/mod.rs` (732 lines): move the `#[tool]` methods `client_input_key`, `client_input_mouse`, `client_ui_click`, `client_cursor_move`, `client_type_text`, `client_input_focus`, `client_input_status`, `client_input_release` (l.~556-655, with their doc comments and any arg structs used only by them) into a new `server/input_tools.rs` with `#[tool_router(router = input_router, vis = "pub(super)")] impl LabServer { ... }`, copying the imports pattern of `server/world.rs` (l.1-26). In `server/mod.rs` add `mod input_tools;` after `mod instances;` and `+ Self::input_router()` after `+ Self::lease_router()` (l.331). Arg structs used elsewhere stay where they are. Target: `server/mod.rs` under 650 lines.
2. `uat/runner/mod.rs` (652 lines): move `run_row` (l.~480-585, the per-row driver that ends by building `RowEvidence`) into a new `uat/runner/row.rs` as `impl Runner { pub(super) async fn run_row(...) }`, with the imports it needs. Add `mod row;` next to the other `mod` lines. Target: `mod.rs` under 560 lines.
3. Nothing else changes; visibility widens only where the compiler requires it (`pub(super)`), never to `pub`.

Tests: none new. The whole `cimmeria-lab` suite must pass unchanged, including `every_routed_tool_is_classified` (the moved tools are still routed) and every `uat::runner` test.

Checks: L1, L2. Report: the line counts of the four files, and the routed tool count before and after (from the `every_routed_tool_is_classified` set, or `tool_router.list_all().len()` printed once in a scratch test you do not commit).

### AX-11 The evidence gate

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-11-gate` · **Worktree:** `lax-11` (at `lax-w1` with AX-10 and AX-15)
- **Commit:** `feat(lab): evidence gate on every tool result (AX-11)`
- **Rules:** R1-R4, R6, R8, R9. Contract: [C3](contract.md#c3-the-evidence-gate), C1.

Files and changes:

1. `proof/claims.rs`: replace the stub with `Claim`, `JUDGED` (empty: `pub const JUDGED: &[&str] = &[];`), `NO_CLAIM` and `claim()`. `claim(tool)` returns `NoClaim` when `crate::lease::policy::OPEN` contains the tool or `NO_CLAIM` does; `Judged` when `JUDGED` does; else `Unjudged`. `NO_CLAIM` lists exactly: `client_lua_eval`, `client_wait_for`, `client_mem_write`, `client_call_native`, `client_console`, `client_hook_install`, `client_hook_remove`, `client_events_read`, `client_entity_find`, `client_inventory`, `client_chat_log`, `client_combat_log`, `client_wait_event`, `client_batch`, `client_hover_probe`, `lab_client_start`, `lab_client_stop`, `lab_client_restart`, `lab_uat_run`, `lab_uat_calibrate`, `lab_uat_attest`, `lab_record_start`, `lab_route_record`, `lab_route_stop`. Drop any name that `lease::policy::OPEN` already holds or that is not routed (the test below finds them).
2. `proof/gate.rs`: replace the stub with `pub fn stamp(tool: &str, v: &mut Value)` implementing C3 rules (a)-(f), and `pub fn verdict_of(v: &Value) -> Option<&str>` returning the mirrored `verdict`.
3. New `server/evidence_gate.rs`: `pub(super) fn stamp_response(tool: &str, out: &mut Result<CallToolResponse, McpError>) -> Option<String>`. For `Ok(CallToolResponse::Complete(r))`: for each text content block (match the arms `shape_response` uses, `server/handler.rs` l.66 onward), parse the text; when it is a JSON object, `gate::stamp` it and write it back with `serde_json::to_string`; do the same for `r.structured_content` when it is an object. Return the first block's mirrored verdict. Any other case returns `None` and changes nothing. Add `mod evidence_gate;` to `server/mod.rs` next to `mod handler;`.
4. `server/handler.rs` `call_tool` (l.~179-200): after the `lease::permit::scope(...)` call returns `out` (l.~198) and before `log_call` (l.~199), `let verdict = evidence_gate::stamp_response(&tool_name, &mut out);` (make `out` `mut`). `log_call` gains a last parameter `verdict: Option<&str>` and logs it as the field `verdict` in the `lab.call` event; update its other call sites (grep `log_call(`) to pass `None`.
5. `server/uat.rs` `RouterInvoker::call` (l.202-233): after `normalize_result` builds the outcome, when `outcome.ok`, call `crate::proof::gate::stamp(name, &mut outcome.json)`. At most 3 lines added (the file is 641).
6. New `proof/gate_tests.rs` (wired from `gate.rs`), `proof/claims_tests.rs` (wired from `claims.rs`), `server/evidence_gate_tests.rs` (wired from `evidence_gate.rs`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `errors_pass_untouched` | `{"error":"x","effect_ok":true}` unchanged for any tool | rule (a) |
| `no_claim_tools_pass_untouched` | `client_lua_eval` output unchanged | rule (b) |
| `judged_without_evidence_is_no_evidence` | with a test-only judged name (call the internal `stamp_with(claim, v)` helper the gate uses, so `JUDGED` can stay empty): `evidence.why == "no_evidence"`, `verdict == "inconclusive"` | rule (c) |
| `unjudged_gets_unjudged_and_keeps_effect_ok` | `stamp_with(Claim::Unjudged, v)` (not a tool name: `client_world_click` turns judged in AX-31) with `{"effect_ok":true,"counts_as_native_pass":true}`: evidence `unjudged`, `effect_ok` still true, `counts_as_native_pass` false, `claimed.counts_as_native_pass` true. AX-44 flips the `effect_ok` half when it sets `STRICT` | rules (d), (e) |
| `denied_flips_effect_ok` | carried evidence `denied` + `effect_ok: true`: `effect_ok` false, `claimed.effect_ok` true | rule (e) |
| `inconclusive_removes_effect_ok` | carried `inconclusive` + `effect_ok: true`: no `effect_ok`, `claimed.effect_ok` true | rule (e) |
| `ui_native_pass_is_downgraded_too` | `stamp_with(Claim::Unjudged, v)` with `{"native_pass":true,"native_level":"real_input"}` and no evidence: `native_pass` false, `claimed.native_pass` true, `native_level` unchanged | rule (e), UI spelling |
| `confirmed_keeps_counts` | carried `confirmed` + `counts_as_native_pass: true`: unchanged plus `verdict: "confirmed"` | rule (e) touches only non-confirmed |
| `claim_lists_name_routed_tools` (claims_tests) | every `JUDGED` and `NO_CLAIM` name is in `test_server().tool_router.list_all()` (reuse the helper `every_routed_tool_is_classified` uses in `lease/lease_tests.rs` l.335-357), no name is in both, and no `NO_CLAIM` name is in `policy::OPEN` | a typo or a stale name |
| `stamp_response_rewrites_json_text_only` (evidence_gate_tests) | a `CallToolResult` with one JSON text block and one plain-text block: the JSON block gains `evidence`, the plain one is unchanged, the return is `Some("unjudged")` for the unrouted name `x_unrouted_tool` (a routed tool may turn judged in a later wave) | step 3 |

Checks: L1, L2. Report the wiring lines in `handler.rs` and `uat.rs` with their line numbers (the reviewer checks them by hand: no test drives a live tool through the handler).

### AX-12 The runner reads verdicts

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-12-runner-verdict` · **Worktree:** `lax-12` (at `lax-w1` with AX-10 and AX-15)
- **Commit:** `feat(lab): UAT runner fails denied actions and holds inconclusive rows (AX-12)`
- **Rules:** R1-R4, R6, R8. Contract: C1, C3. Decision: D-AX1 (INCONCLUSIVE stays action-level and maps to row UNVERIFIED).

The rule: a **denied** action fails like `effect_ok: false` does today. An **inconclusive** setup or step action does not stop the row; it is noted, and at grading time the row becomes UNVERIFIED unless at least one required clause of the row passed (an independent read proved the outcome). **Unjudged** changes nothing until AX-44. Teardown actions are ignored.

Files and changes:

1. New `uat/runner/verdict.rs`:
   - `pub(super) enum ActionVerdict { Proceed, Denied(String), Inconclusive(String) }`.
   - `pub(super) fn of_outcome(tool: &str, json: &Value) -> ActionVerdict`: `Evidence::from_value(json.get("evidence"))`; `Denied` gives `"{tool} denied: {why}: {hint}"`; `Inconclusive` gives `"{tool}: inconclusive ({why})"`; anything else, or no evidence, `Proceed`.
   - `pub(super) fn settle(g: &mut <the type grade() returns>, inconclusive: &[String], clauses: &[ClauseResult])`: when `inconclusive` is not empty, push one flag `inconclusive: <note>` per note; then, when no clause has `required && verdict == Verdict::Pass`, call `mark_unverified(g, inconclusive)`.
   - Add `mod verdict;` to `uat/runner/mod.rs`.
2. `uat/runner/mod.rs` `RowCtx` (l.191-219): add `pub inconclusive: Vec<String>` with the doc `/// Actions whose result could not be judged (AX-12): flags, and UNVERIFIED without a passing required clause.`; add `inconclusive: vec![]` at every `RowCtx {` literal (grep the crate).
3. `uat/runner/actions.rs` after `record_outcome(rec, &out);` (l.356) and before the `effect_ok` check (l.360): `match verdict::of_outcome(&name, &out.json)`; `Denied(m)` when `out.ok` returns `fail(rec, m)`; `Inconclusive(m)` when `role != Role::Teardown` pushes `m` to `ctx.inconclusive`. Keep the `effect_ok: false` check after it. At most 12 lines added.
4. `uat/runner/row.rs` (from AX-15): directly after `mark_unverified(&mut g, &ctx.unverified);` call `verdict::settle(&mut g, &ctx.inconclusive, &clauses);`.
5. New `uat/runner/verdict_tests.rs`, wired from `verdict.rs`. Use `super::tests::{Fake, run_one}` (or whatever the sibling `*_tests.rs` files import from `tests.rs`; grep `use super::tests` in `uat/runner/`), and `Fake::answer(tool, replies)` to script replies. Never add to `uat/runner/tests.rs`.

Tests (`verdict_tests.rs`, rows modelled on `a_drag_that_moved_nothing_fails_its_action`, `tests.rs` l.831-860):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `a_denied_step_fails_its_action` | `client_world_click` answers `{"evidence":{"verdict":"denied","why":"no_hover",...}}`: the action is not ok, its error contains `denied: no_hover`, row FAIL | step 3 |
| `an_inconclusive_step_without_a_passing_clause_is_unverified` | the step answers `inconclusive`, the row's only required clause fails or is pending: row UNVERIFIED, flag `inconclusive: client_world_click: inconclusive (no_post_read)` | steps 3-4 |
| `an_inconclusive_step_with_a_passing_required_clause_keeps_pass` | same step, a required `wait` clause with `lua_condition = "true"` passes: row PASS and the flag is present | the rule only bites without independent proof |
| `an_unjudged_step_changes_nothing` | the step answers `{"evidence":{"verdict":"unjudged",...}}`: row PASS, no flag | D-AX13 |
| `teardown_inconclusive_is_ignored` | a teardown answering `inconclusive`: no flag | teardown is outside the claim |

Checks: L1, L2.

### AX-13 Server corroboration, and the Sim split

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-13-corroborate` · **Worktree:** `lax-13` (at `lax-w1` with AX-10 and AX-15)
- **Commit:** `feat(lab): server corroboration checks for world tools (AX-13)`
- **Rules:** R1-R4, R6, R7, R8. Contract: C1, C2. Decision: D-AX11.

Files and changes:

1. `supervisor/world/sim.rs` (670 lines): split first. Move the inherent `impl Sim { ... }` block (l.156-358) into a new `supervisor/world/sim_model.rs`, declared from `sim.rs` as `#[path = "sim_model.rs"] mod model;` so it stays a child module and keeps access to `Sim`'s private fields. No other change in that commit's hunk. Target: `sim.rs` under 480 lines.
2. `proof/corroborate.rs`: replace the stub with

```rust
pub enum Expect {
    EntityPosNear { entity_id: u32, at: [f64; 3], within_m: f64 }, // server metres
    EntityField { entity_id: u32, pointer: String, equals: Value },
    LogLine { contains: String, since_ms: i64 },
}
pub async fn check(server: Option<&dyn crate::uat::invoke::ServerInvoker>, name: &str, e: &Expect, required: bool) -> Check;
pub fn default_server() -> Option<std::sync::Arc<crate::uat::invoke::ServerTools>>;
```

   - `default_server()` builds `ServerTools::from_env()` once and caches it in a `std::sync::OnceLock` (the pattern of `daemon/workbench/mission_api.rs` l.23 `default_server()`; reuse that function if it is visible from here instead of writing a second one).
   - `EntityPosNear`: `server_entity_get {"entity_id"}`; the position is at the JSON pointer `/entity/position` (an array of 3; `crates/lab-mcp/src/tools/entities.rs` returns `{"entity": <snapshot|null>}`). `ok = Some(distance <= within_m)`, `before = at`, `after = position`; `ok: false` carries why `server_disagrees`.
   - `EntityField`: same call, `ok = Some(json.pointer(pointer) == Some(equals))`, why `server_disagrees` when false.
   - `LogLine`: `server_log_tail {"contains", "since_ms", "limit": 50, "include_client": false}`; `ok = Some(entries non-empty)` (the reply's `entries` array, `crates/lab-mcp/src/tools/logs.rs` l.85); why `server_disagrees` when empty.
   - `server` `None`, a failed call, or `entity: null`: `ok: None`, why `server_unreachable` (null entity: why `server_disagrees` with `ok: Some(false)` instead, the entity is gone).
   - Never call `server_packet_tap_read` (it drains the ring the UAT runner reads).
   - Every check has `source: Server`.
3. `supervisor/world/io.rs` `WorldIo` (l.41-101): add a default method `async fn server_position(&mut self, entity_id: u32) -> Result<Option<Vec3>, String> { let _ = entity_id; Ok(None) }` with a doc line, and in `LiveWorld` implement it with `corroborate::default_server()` and the `/entity/position` read, converted with nothing (it is already server metres). At most 25 lines added to `io.rs` (571).
4. `supervisor/world/sim.rs` / `sim_model.rs`: `Sim` gains `pub server_pos: Option<Vec3>` (default `None`) and overrides `server_position` to return it (4 lines in the `impl WorldIo for Sim` block).
5. New `proof/corroborate_tests.rs` using a fake `ServerInvoker` (model on `uat/runner/test_server.rs` `FakeServer`).

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `pos_near_within_radius_confirms` | fake `server_entity_get` answers position 1 m away, `within_m` 2: `ok == Some(true)`, source server | step 2 |
| `pos_far_is_server_disagrees` | 5 m away: `ok == Some(false)`, why `server_disagrees` | step 2 |
| `no_server_is_unreachable_not_false` | `server = None`: `ok == None`, why `server_unreachable` | honesty: an absent server never denies |
| `failed_call_is_unreachable` | fake returns `ok: false`: same as above | step 2 |
| `log_line_found_and_missing` | `entries` with one row gives `Some(true)`; empty gives `Some(false)` | step 2 |
| `never_reads_the_packet_tap` | the fake records tool names: after all four expectations, no `server_packet_tap_read` | step 2 rule |

Checks: L1, L2. Report `sim.rs`, `sim_model.rs` and `io.rs` line counts.

### AX-14 Stop reports

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-14-stop` · **Worktree:** `lax-14` (at `lax-w1` with AX-10 and AX-15)
- **Commit:** `feat(lab): structured stop reports on world tool failures (AX-14)`
- **Rules:** R1-R4, R6, R8. Contract: [C8](contract.md#c8-stop-report).

Files and changes:

1. `proof/stop.rs`: replace the stub with C8's `StopKind`, `StopReport`, `StopPlayer`, plus `impl StopPlayer { pub fn new(at: [f64;3], now: [f64;3]) -> Self }` computing `moved_m` as the 3D distance.
2. `supervisor/world/mod.rs` `WorldFailure`: add `pub stop: Option<crate::proof::stop::StopReport>`; `Steps::fail` sets `None`; `to_json` (l.155-166) adds `"stop": serde_json::to_value(stop)` only when `Some`; `message()` (l.~135-153) appends `; stop: <kind>` when `Some`. New `Steps::fail_stop(&self, now_ms, step, message, stop) -> WorldError`. At most 30 lines added (keep `world/mod.rs` under 430).
3. No tool emits a stop yet; AX-32, AX-34 and AX-56 do.

Tests (`supervisor/world/steps_tests.rs`, created by AX-10):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `fail_stop_serialises_the_report` | `to_json()["stop"]["kind"] == "stuck"`, `suggest` and `player.moved_m` present (3-4-0 triangle gives 5.0) | step 2 |
| `plain_fail_has_no_stop_key` | `Steps::fail` output has no `stop` key | legacy shape unchanged |
| `message_names_the_stop_kind` | `message()` ends with `; stop: stuck` | step 2 |

Checks: L1, L2.

### AX-16 resume_from for lab_uat_run

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `lab-autopilot/AX-16-resume` · **Worktree:** `lax-16` (at `lax-w1` with AX-12 merged)
- **Commit:** `feat(lab): resume a UAT run from a row (AX-16)`
- **Rules:** R1-R4, R6, R8. Contract: [C10](contract.md#c10-resume_from).

Files and changes:

1. `server/uat.rs` `UatRunArgs` (l.33-~60): add `/// Resume an appended run (`run_dir`) at "<section>/<row>", or "auto" for the first row with no manifest entry.` `#[serde(default)] pub resume_from: Option<String>,` and pass it into `RunRequest` where the args are converted (grep `RunRequest {` in `server/uat.rs`). At most 6 lines added.
2. `uat/runner/mod.rs` `RunRequest` (l.64-~100): add `pub resume_from: Option<String>`. Fix every `RunRequest {` literal in `crates/lab` and `crates/lab-cli` that does not use `..Default::default()`.
3. New `uat/runner/resume.rs`:
   - `pub(super) fn validate(req: &RunRequest) -> Result<(), String>` with the three C10 error strings.
   - `pub(super) fn start_index(order: &[String], done: &[String], resume: &str) -> Result<usize, String>`: `order` is the selected rows as `"<section>/<row>"` in spec order; `done` is the manifest's row keys. `"auto"` gives the first index not in `done` (or `order.len()`); otherwise the index of that key, or the C10 unknown-row error.
   - Add `mod resume;` to `uat/runner/mod.rs`.
4. `uat/runner/mod.rs` `run_all` (l.~369-439): call `resume::validate` before anything runs (return its error the way other request errors return; grep how `run_all` reports a bad request). Build `order` with the same section and row filters the loop uses, then skip rows whose position is below `start_index`. Skipped rows are neither run nor re-graded and keep their manifest entries. At most 25 lines added; if `mod.rs` would pass 600, put the order-building in `resume.rs`.

Tests (`uat/runner/resume_tests.rs`, wired from `resume.rs`; the run-level ones with `super::tests::{Fake, run_one}` or the multi-row helper the sibling tests use):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `auto_starts_at_first_missing_row` | order `[a/1,a/2,a/3]`, done `[a/1]`: 1 | step 3 |
| `auto_with_all_done_runs_nothing` | gives 3 | C10 |
| `explicit_row_and_unknown_row` | `a/2` gives 1; `a/9` errors `resume_from: no row a/9 in the selected sections` | C10 |
| `resume_needs_run_dir_and_excludes_rows` | the two validate errors | C10 |
| `a_resumed_run_skips_earlier_rows` | a two-row section run once with row 2 failing (scripted), then again with the same `run_dir` and `resume_from: "auto"`... use `a/2` explicitly: only row 2 runs (the fake's call log shows no row-1 tool), row 1's manifest entry is unchanged | step 4 |

Checks: L1, L2. Report the `uat/runner/mod.rs` and `server/uat.rs` line counts.
