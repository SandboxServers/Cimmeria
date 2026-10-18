# Castle continuation: wave R3 packets (rest of the campaign, part 1)

> **Type:** Reference. **Audience:** Castle orchestrator, ops, packet workers and reviewers.
> **Last updated:** 2026-10-17.
> **Companions:** [part 2: authored, hub, closeout, briefs, graph](r3-packets-2.md), [R2 packets](r2-packets.md), [overlay ledger](README.md), [resume card](RESUME.md), [R3 architect report](architect-report-r3.md), [Castle packet menu](../four-zone-planning/castle-work-packets.md), [execution contract](../four-zone-planning/execution-contract.md).

Main-checkout overlay, never committed to `main` (D-CA2). CS-INT3a copies both R3 files to the branch as `docs/analysis/castle-continuation/r3-packets.md` and `r3-packets-2.md`.

R3 covers every Castle packet that R2 held: CS-03d, CS-04a/b/c, CS-05, CS-06, CS-07, CS-08a2 and CS-08b. Wherever the evidence is missing, the packet is a **seal first, build second** pair. The build's gate names the seal's verdict, and no build guesses a mechanic or an ID. This part holds the rules, the contract, the R3a seals and the restoration builds. [Part 2](r3-packets-2.md) holds the authored-quest template, the hub builds, the closeout, the dispatch briefs and the R2+R3 dependency graph.

## Dispatch rules (R3)

- **No lab** (D-CA11), as in R2. No R3 packet starts a game client, takes a lab lease, calls a `lab_*`, `client_*` or `server_*` MCP tool, or starts a Castle server. A claim only the client can prove is written as `UATPending (end UAT, D-CA11)` in the branch file `docs/analysis/castle-continuation/uat/end-uat.md`. CS-07c is the only exception: the user runs it.
- One packet, one worker, one worktree (`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\castle-<packet>`) and one test DB (`sgw_castle_<packet>`, for example `sgw_castle_cs_08b1`). Ops makes each worktree: `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>`, one ops dispatch per worktree.
- Sizes:
  - a packet-coder packet takes about 40k working tokens;
  - a specialist seal takes about 50k. If it runs over, it commits what is sealed and returns `blocked`, naming the remainder.
- Review follows D-CA6 for each wave:
  1. one read-only `packet-reviewer` for the whole wave;
  2. fixes by a fresh `packet-coder` (for code and seeds) or a fresh domain agent (for seals), in the owning worktree;
  3. a fresh re-review, at most 3 rounds.
- Integration is by cherry-pick onto `content/castle-campaign` in `zone-castle` (CS-INT3a, CS-INT3, CS-INT4). The owner retires integrated worktrees with `pwsh tools/build-lane/rm-worktree.ps1 <name> --force` (D-CA14).
- **A gate is checked when the packet is dispatched, not when this file was written.** Before dispatching a packet, the orchestrator reads the gate's verdict line (in a seal report, or a branch file) or asks ops for the named `git log` check. A packet whose gate is unmet keeps its Blocked status.
- Cellblock is live. Never touch any of these:
  - `zone-cellblock` or any `cellblock-*` worktree;
  - `sgw_zone_cellblock`, any `sgw_cellblock_*` DB, or `sgw_castle_cs_01b` (it holds `Cellutb`);
  - Cellblock's server;
  - `docs/analysis/cellblock-continuation/`;
  - `.claude/state/active-campaign`.

  Reading `content/cellblock-campaign` and `content/harset-campaign` with `git show` or `git grep` is allowed.

### Repo rules every R3 packet carries

1. PowerShell only, never bash. Use absolute paths, and start every shell call with `Set-Location <worktree>;`.
2. Never run `cargo` directly. Run `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>` in the foreground, and read the lane's failures file instead of rerunning. `cargo fmt --all -- --check` does not compile and may run directly.
3. Live-DB tests: `pwsh tools/build-lane/live-db-test.ps1 <filter> -p <crate>`. It reloads this worktree's own DB from the seeds.
4. Git:
   - no `git stash`, no `git worktree prune`, no push, no PR;
   - stage by explicit path, never `git add -A`;
   - commit once with the packet's subject and the attribution lines your brief gives (default `Co-Authored-By: Claude <noreply@anthropic.com>`).
5. Edit only the packet's allowlist. If a file outside it is needed, stop and return `blocked`.
6. Files under `docs/` are CRLF. After writing one, run ``$t=[IO.File]::ReadAllText($p) -replace "(?<!`r)`n","`r`n"; [IO.File]::WriteAllText($p,$t)`` with an absolute `$p`, then `git diff --check`.
7. File caps: 500 lines soft, 700 hard. Split a new file before it reaches 500. Do not grow a file that is already over 500: `mission_round_trip_tests.rs` (638), `mission_1360.rs` (555) and `resources/tests/mission_additions.rs` (610) take only the edits a packet names.
8. Seeds are the source of truth. Edit `db/resources/`, never `db/scripts/`. A build packet copies a seal's frozen SQL **verbatim**; if a row is ambiguous, it returns `blocked`.
9. Never write an IP address other than loopback, a password or a token into any file.
10. Report and return: write the full report at the path in your brief, then return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

## Contract excerpt (R3)

- **Branch.** The feature branch is `content/castle-campaign` in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\zone-castle`. `FEATURE_SHA` is whatever RESUME.md records at dispatch: CS-INT2a's SHA for R3a, and the CS-INT3a SHA (taken after CS-INT2) for R3b. Packet branches are named `castle/<packet-lower>-<slug>`.
- **Statuses:** Ready, BlockedDependency, BlockedDecision, BlockedEvidence, Writing, Review, Integrated, UATPending, Done. A seal's verdict line is one of `SEALED`, `BlockedEvidence (<gate>)`, `BlockedDecision (<question>)` or `BlockedDependency (<what>)`.
- **Provenance labels**, one per claim: `SHIPPED_DATA`, `RECOVERED_SCRIPT`, `CLIENT_MAP`, `RECONSTRUCTION` (bounded and delegated under O14-03), `ORAL_HISTORY` or `CIMMERIA_AUTHORING`. A claim without proof is `UNRESOLVED (<gate>)`.
- **Restoration chain IDs** (O14-22: the coordinator allocates within 1367-1400 after a collision check):

  | Block | Owner | R3 use |
  |---|---|---|
  | 1366 | CS-08a1 | seeded (Frost's Letter exit route) |
  | 1367-1393 | R2 (1520, 1521, CS-02S, CS-03S extras) | CS-03dS may take only an **unused** ID from 1386-1393 that CS-03S did not freeze |
  | 1394-1397 | CS-00R2 / CS-08b | the G-567 Castle-leg graph; CS-08bM re-verifies it |
  | 1398 | CS-06 | the 1664 exit grant (assigned here) |
  | 1399 | CS-06 | the 1664 catch-up or retry chain, only if CS-06S proves one is needed |
  | 1400 | reserve | fixes; only the architect assigns it |

- **Authored IDs.** Chains come from the Castle block 10000-10999 (`id-allocation.md` § Reserved blocks). Missions, steps, objectives, tasks, dialogs, screens and texts come from the **sub-blocks CS-00Be proposes** in branch `evidence/mission-catalog.md`. No R3 packet invents an authored ID outside those sub-blocks. A mission ID must be at most 65535 (the `MissionAddition` root key) and absent from stock `CookedDataMissions.pak`.
- **No-collision check.** For every new ID, run `git -C <wt> grep -n -w <id> <ref> -- db/resources` for each of `FEATURE_SHA`, `content/cellblock-campaign`, `content/harset-campaign` and `content/tollana-campaign`, and paste the output. Also check Cellblock's individual claims in `content/cellblock-campaign:docs/analysis/four-zone-planning/id-allocation.md`: mission 62050 with steps, objectives and tasks 62051-62056; dialog 60150, screen 96110; dialog 60160, screens 200160-200162; chains 11000-11005 and 11010.
- **Seed shape** (chain 1366, `db/resources/Content/Seed/castle_carry_in_chains.sql:1-16`):

  ```sql
  INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (<id>, '<mission> - <what> (<LABEL>)', 'mission', <mission>, true, 0);
  INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (<id>, '<event>', '<key>', 'player', false, 0);
  INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES
    (<id>, 'world', 8, NULL, 'eq', NULL, 0), (<id>, 'mission_status', <m>, NULL, 'eq', 'active', 1);
  INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (<id>, 'advance_step', <m>, '<step>', '{}', 0, 0);
  ```

- **Triggers.** `dialog_choice`, `enter_region`, `interact_tag`, `player_loaded`, `mission_accepted`, `entity_dead_tag`, `stargate_crossed`, `stargate_dialed`, and `mission_completed` (the key is the mission id; precedent `chain_replay_tests/class_start_sgc/ordinance_rewards.rs:99`, which calls `event_dispatch::fire_mission_completed`). `Action::CompleteMission` awaits `fire_mission_completed` inside the executor arm, so a `mission_completed` chain runs synchronously in the completing dispatch (`chain_replay_tests/mission_1324.rs:168-175` and `:803-806`). A chain with no trigger gets the synthetic `__direct_invoke_<id>`.
- **Authored catalog API** (#1734, `crates/resources/src/base/mission_additions.rs`):

  ```rust
  pub struct MissionAddition { pub mission_id: u32, pub xml: &'static str }
  pub const MISSION_ADDITIONS: &[MissionAddition] = &[];   // production; empty today
  pub const MAX_MISSION_XML_BYTES: usize = 355_840;
  ```

  The module is declared at `crates/resources/src/lib.rs:34`. `resources/mod.rs:217` loads `MISSION_ADDITIONS`. `resources/tests/mission_additions.rs:125` asserts `MISSION_ADDITIONS.is_empty()`. The test fixture is mission 61000 (`TEST_MISSION_XML`, 61001-61003). The execution contract says: implement the first authored quest as the client-catalog proof, and hold the others until journal, dialog, advance and relog pass in the live client (D-CA16).
- **Castle authored layout** (D-CA17, architect). This keeps Cellblock's CB-N05 edit to a one-line array conflict.
  - Castle XML lives in a new `crates/resources/src/base/mission_additions_castle.rs`: one `pub const CASTLE_<KEY>_XML: &str = r#"..."#;` per quest, each with a doc comment naming its seal.
  - `MISSION_ADDITIONS` lists `MissionAddition { mission_id: <id>, xml: super::mission_additions_castle::CASTLE_<KEY>_XML }`, in mission-ID order.
  - New catalog tests go in a new `crates/resources/src/base/resources/tests/mission_additions_castle.rs`.
  - Authored chains go in `db/resources/Content/Seed/castle_side_stories_chains.sql`; repeatables in `castle_repeatable_chains.sql`.
- **Placement rules** (as R2): on the navmesh with `y` at floor height; no hostile aggro radius plus 3 reaches a mission actor; friendlies sit at least 5 u outside any aggro radius; never on a disconnected component. The nav tool is `pwsh tools/build-lane/lane.ps1 cargo run -p cimmeria-navmesh-extractor --bin nav_inspect -- data\spaces\castle.nav --probe NAME=X,Y,Z` (exit 0 ok, 2 off tolerance, 3 another component). `crates/entity/tests/castle_navmesh.rs` checks every world-8 spawnlist row.
- **Replay harness:** `crates/cell-content/src/cell/content/chain_replay_tests/` (`load_single_chain_for_test`, `require_db_or_skip`). Register each module in `chain_replay_tests/mod.rs` (126 lines). DB round-trip tests live in `crates/services/src/*_round_trip_tests.rs`, each registered in `crates/services/src/lib.rs` as `#[cfg(test)]` plus `mod <name>;` beside `mod mission_round_trip_tests;` (`:91-92`).
- **Cross-zone owners.**
  - HA-01b owns 1664 steps 5224-5226.
  - Harset owns chains 6503/6504 (567 recipient, disabled until G-567) and HA-1360a/b.
  - Cellblock owns CB-N05b (the Castle recipient), held on HA-1360.
  - FZ-99 owns `docs/gap-analysis*`, `docs/project-status.md` and `docs/content/zone-audit.md`. No Castle packet edits them.

### Base check (R3)

Step 0 of every worktree packet. Use `<BASE>`: the `FEATURE_SHA` for a fresh packet, or for a stacked packet the predecessor's reviewed tip that the brief names.

```powershell
Set-Location <wt>; git status --porcelain            # must be empty
git -C <wt> rev-list --count origin/main..HEAD       # must be 0
git -C <wt> rebase --onto <BASE> origin/main <branch>
git -C <wt> rev-parse HEAD                           # must equal <BASE>
```

If any value differs, stop and return `blocked`.

## Wave R3a: seals (docs only, at most seven in parallel)

All of these are based on the `FEATURE_SHA` after CS-INT2a. They can run while R2b/R2c build, because each one writes only new docs files. None compiles or touches seeds. The only lane use allowed is `nav_inspect` and `lane.ps1 cargo nextest run -p cimmeria-entity --test castle_navmesh`. Each seal ends its output with a **verdict line**, a **frozen rows** section (exact SQL or Rust, or "none"), a **test list** naming the consuming packet for each row, and **end-UAT rows**.

### CS-06S: the 1664 exit-grant seal (G-1664)

**Agents:** mission-systems-advisor, then movement-teleport-advisor appends `## Travel failure`, in the same worktree. **Branch / worktree:** `castle/cs-06s-1664-seal` / `castle-cs-06s`. **Status:** Ready once CS-INT2a lands.

**Allowlist:**
- `docs/analysis/castle-continuation/evidence/1664.md` (new);
- the status cell of the G-1664 row in `evidence/reconciliation.md`.

**Inputs** (branch):
- the 1664 candidate in `work-packets.md:19-50`;
- `evidence/castle-contract.csv` (the 1664 row) and the G-1664 row of `evidence/reconciliation.md`;
- `db/resources/Content/Seed/castle_706_708_chains.sql`, the chain that completes 708 on `stargate_crossed`;
- `crates/cell-content/src/cell/content/executor/mission.rs:420-440` and `event_dispatch/mission.rs`;
- the Harset ledger: `git show content/harset-campaign:docs/analysis/harset-continuation/README.md` and `work-packets.md` (HA-01b, HA-D05, Kaerlia).

Answer each point with a provenance label and a file:line:

1. **Firing order.** Does a `mission_completed '708'` chain run while the player's entity is still in world 8, before `perform_gate_travel` moves it? Cite the dispatch path. If the `world` condition would read 68, say so, and freeze the conditions without it.
2. **Chain 1398.** Freeze the grant:
   - trigger `mission_completed` key `'708'`;
   - conditions: `mission_status 708 completed` and `mission_status 1664 not_active`, plus `world 8` only if point 1 proves it;
   - the accept action, using the exact `action_type` and params the executor uses for an accept (cite it);
   - `delay_ms 0`;
   - label `RECONSTRUCTION` (the candidate) unless a client source pins it.
3. **Never twice.** Prove that redialing, re-crossing, relogging and abandoning cannot grant 1664 a second time. Cite how the accept refuses an active or completed mission (`executor/mission.rs:55-56`).
4. **Travel failure** (movement-teleport-advisor). If `perform_gate_travel` fails after the grant, the player keeps 1664 active in Castle. Say whether the player can still reach Harset by redialing, with no chain re-firing. If not, freeze chain 1399 as the retry or catch-up and say exactly what it does. The same chain may cover characters who finished 708 before CS-06 shipped: a `player_loaded` in world 8 with 708 completed and 1664 `not_active`. Freeze it only if the accept is proven idempotent. Otherwise write "1399 not needed" and give the reason.
5. **Receiver.** HA-01b owns steps 5224-5226 and objectives 6063 and 6059-6062.
   - Reconcile 6059 (optional) against 6060 (the required door interaction) with the HA-01b draft.
   - Record HA-D05's status and the Harset SHA you read.
   - Record Kaerlia's placement (name moniker 27315) as Harset's dependency.
   - Name the exact HA-01b commit subject or packet whose integration unblocks CS-06a (D-CA18).
6. **Test lists:**
   - CS-06a replay (`mission_1664.rs`): grant on 708 completion; no grant with 708 not completed; no grant when 1664 is active or completed; the 1399 rows if frozen.
   - CS-06b live-DB (`live_db_castle_1664`): the accept persists and survives relog, and a second completion dispatch grants nothing.
7. **End-UAT rows:** the journal shows 1664 after the Castle exit; the Harset arrival continues it.

**Verdict:** `SEALED`, `BlockedEvidence (G-1664)` or `BlockedDependency (HA-01b)`. **Lane:** none. **Commit:** `docs(castle): seal the 1664 exit grant (CS-06S)`. **Reports:** `...\castle-cs-06s\.claude\reports\castle\CS-06S-mission.md` and `CS-06S-movement.md`.

### CS-08bM: the 567 Castle-leg sign-off (G-567)

**Agents:** items-systems-advisor, then server-authority-enforcer appends `## Authority`. **Branch / worktree:** `castle/cs-08bm-567-signoff` / `castle-cs-08bm`. **Gate:** the CS-00R2 G-567 verdict is `SEALED`. Otherwise the packet stays **BlockedEvidence (G-567)** and is not dispatched.

**Allowlist:**
- `docs/analysis/castle-continuation/worknotes/CS-08bM.md` (new);
- an appended `## R3 sign-off` section in `evidence/g-567-706-1376.md`.

**Inputs:** the CS-00R2 frozen rows (1394-1397) in `evidence/g-567-706-1376.md`; `castle_carry_in_chains.sql`; `chain_replay_tests/mission_567.rs` (234 lines); the item 2698 row; moniker 7547 (`texts.sql:54162`); Harset HA-D04 and chains 6503/6504.

1. Re-verify each frozen row against the seed shape and the executor's action names (cite them).
2. **Item 2698.** Cite its row: unique or stackable, and its bag. What happens when the bag is full (cite `add_item`)? Does the chain advance anyway?
3. **The cabinet (7547).** Which entity the player uses: an existing spawn, or a new template and spawn. If new, give the IDs with the no-collision check and a `nav_inspect` proof.
4. **Grant once per character.** Cover relog, re-interact, already holding 2698, and abandon followed by re-accept. If a guard condition is missing, add it to the frozen rows (from 1394-1397) or return BlockedEvidence.
5. **Authority.** The client can only send the interaction. Cite where the server checks step state and interaction range for `interact_tag`.
6. **Harset interface.** Name the Harset chain that takes 2698 at step 4039. Chains 6503/6504 stay disabled; enabling them is Harset's job (HA-D04).
7. **`mission_567.rs`.** List each assertion, by line, that the Castle leg makes false. Assertions about 6503/6504 stay unchanged.
8. **Test lists:** CS-08b1 (replay) and CS-08b2 (live-DB grant once and persistence). **End-UAT rows.**

**Verdict:** `SEALED` or `BlockedEvidence (G-567: <what>)`. **Commit:** `docs(castle): sign off the 567 Castle-leg rows (CS-08bM)`. **Reports:** `...\castle-cs-08bm\.claude\reports\castle\CS-08bM-items.md` and `CS-08bM-authority.md`.

### CS-03dS: the Option A implant seal (only if D-CA-SYM = A)

**Agent:** items-systems-advisor. **Branch / worktree:** `castle/cs-03ds-implant-seal` / `castle-cs-03ds`. **Gate:** the owner picks Option A (**BlockedDecision, D-CA-SYM**) and the CS-03S verdict is `SEALED`. Option B means neither CS-03dS nor CS-03d runs.

**Allowlist:**
- `docs/analysis/castle-continuation/worknotes/CS-03dS.md` (new);
- an appended `## Option A seal` section in `evidence/g-sym.md`.

1. Take CS-03S's "Option A deltas (not frozen)": `add_item 5169 {"container":2}`, and optionally `apply 1926` on accept. Freeze which chain carries each delta (an existing 1521 chain or one unused ID from 1386-1393), its `sort_order`, and `delay_ms 0`.
2. **Item 5169.** Cite its row, what bag 2 means, and the bag-full outcome. Prove one grant per character across relog, the second tank use, abandon and re-accept. If no guard exists, return BlockedEvidence.
3. **Debuff 1926.** Is `apply` an executor action (cite)? Cite effect 1926's row, its duration and whether it persists across relog. If any part is unproven, freeze "no debuff". Option A allows that.
4. **Test list** for CS-03d (`mission_1521_implant.rs`). **End-UAT rows** (the inventory icon, the debuff icon).

**Verdict** and **Commit:** `docs(castle): seal the 1521 Option A implant deltas (CS-03dS)`. **Report:** `...\castle-cs-03ds\.claude\reports\castle\CS-03dS-items.md`.

### CS-05S, CS-04aD, CS-04bD and CS-04cS

These four R3a seals are in [part 2](r3-packets-2.md#wave-r3a-continued-authored-and-hub-seals).

## CS-INT3a: integrate R3a

**Agent:** packet-coder, in `zone-castle` (DB `sgw_zone_castle`). **Gate:** the R3a review gate (D-CA6), and CS-INT2 integrated. If CS-INT2 is not done, integrate the seals anyway; R3b still waits for CS-INT2.

1. Cherry-pick the reviewed seal commits that returned, each with its fixes, in this order: CS-06S, CS-08bM, CS-03dS, CS-05S, CS-04aD, CS-04bD, CS-04cS. Skip any packet that was not dispatched.
2. Copy both overlay files to the branch as `docs/analysis/castle-continuation/r3-packets.md` and `r3-packets-2.md`.
3. Update the branch README rows from the overlay README: R3 findings, D-CA15..D-CA18, and the R3 status rows with each seal's verdict.
4. Make sure every file is CRLF and that `git diff --check` is clean.

**Commit:** `docs(castle): integrate R3a seals`. **Report:** `...\zone-castle\.claude\reports\castle\CS-INT3a-coder.md`. The new tip becomes `FEATURE_SHA` for R3b.

## Wave R3b: restoration builds (base = CS-INT3a SHA)

### CS-08a2M: split the Frost's Letter round trip out of a 638-line file

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-08a2m-letter-tests` / `castle-cs-08a2m`. **DB:** `sgw_castle_cs_08a2m`. **Status:** Ready at R3b. It is behavior-neutral and needs no seal.

**Why:** `crates/services/src/mission_round_trip_tests.rs` is 638 lines, so it cannot take the recipient round trip (CS-08a2) or Harset's HA-1360b extension. D-CA15 moves the letter tests into their own file first.

**Allowlist:**
- `crates/services/src/mission_round_trip_tests.rs`;
- `crates/services/src/frosts_letter_round_trip_tests.rs` (new);
- `crates/services/src/lib.rs` (one `#[cfg(test)]` plus `mod frosts_letter_round_trip_tests;`, placed after `:92` with a one-line `///` doc in the style of `:89-90`).

**Steps:**
1. Move, verbatim, everything from the `///` doc comment above `async fn fresh_letter_entity` (`:386-388`) to the end of the file into the new file. That covers the helpers `fresh_letter_entity`, `letter_inventory`, `drain_letter_traffic` and `persist_letter_traffic`, and the test `live_db_frosts_letter_seeded_bridge_round_trip`.
2. The moved code still calls `cleanup` (`:30`) and `insert_account_and_player` (`:45`). Make those two `pub(crate)`, and import them in the new file as `use crate::mission_round_trip_tests::{cleanup, insert_account_and_player};`. Copy only the `use` lines the moved code needs.
3. If anything above `:386` uses a moved helper, stop and return `blocked`.
4. Give the new file a `//!` header: "Frost's Letter (1360) cross-cell round trips; CS-08a2 and HA-1360b extend this file."

**Tests:** none new (a pure move). The guard is that the test name and the test count stay the same. Run `lane.ps1 cargo nextest list -p cimmeria-services` before and after, and paste both counts into the report.

**Lane, in order:**
1. `cargo fmt --all -- --check`;
2. `lane.ps1 cargo clippy -p cimmeria-services --all-targets -- -D warnings`;
3. `live-db-test.ps1 'frosts_letter | mission_completion | second_completion' -p cimmeria-services`.

**Commit:** `refactor(services): move the Frost's Letter round trip into its own test module (CS-08a2M)`. **Report:** `...\castle-cs-08a2m\.claude\reports\castle\CS-08a2M-coder.md`.

### CS-08b1: 567 Castle-leg seeds and replay tests

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-08b1-567-castle-leg` / `castle-cs-08b1`. **DB:** `sgw_castle_cs_08b1`. **Gate:** the CS-08bM verdict is `SEALED`. Otherwise **BlockedEvidence (G-567)**.

**Allowlist:**
- `db/resources/Content/Seed/castle_carry_in_chains.sql` (append chains 1394-1397 after the 1366 block; no `db/database.sql` edit, since `:397` already includes it);
- `entity_templates.sql` and `spawnlist.sql`, only if CS-08bM point 3 freezes a new cabinet entity;
- `crates/cell-content/src/cell/content/chain_replay_tests/mission_567_castle.rs` (new);
- `chain_replay_tests/mod.rs` (one `mod` line);
- `chain_replay_tests/mission_567.rs`, only the assertion lines CS-08bM point 7 names.

**Steps:**
1. Copy the frozen SQL from branch `worknotes/CS-08bM.md` and `evidence/g-567-706-1376.md` verbatim.
2. Write the CS-08b1 replay tests that CS-08bM lists, with the 1366-style `mission_1360.rs` test as the template. At least:
   - interaction at step 2000 grants 2698 and moves to 2012;
   - interaction at any other step does nothing;
   - a second interaction grants nothing;
   - the wrong world does nothing.

   Each test asserts the exact actions emitted, and each must fail if its seed row is removed.
3. Edit `mission_567.rs` only where CS-08bM names a line. Change each such assertion to the new state, and leave the 6503/6504 assertions alone.

**Lane, in order:**
1. `cargo fmt --all -- --check`;
2. `lane.ps1 cargo clippy -p cimmeria-cell-content -- -D warnings`;
3. `live-db-test.ps1 mission_567 -p cimmeria-cell-content`;
4. if a spawn was added: `live-db-test.ps1 live_db_castle_population -p cimmeria-cell-catalog` and `lane.ps1 cargo nextest run -p cimmeria-entity --test castle_navmesh`.

**Telemetry:** the existing chain-fired and step-advance events. If CS-08bM names a missing refusal reason, return `blocked`; this packet adds no new logging.

**Commit:** `feat(castle): author the Castle leg of Romney's Files (CS-08b1)`. **Report:** `...\castle-cs-08b1\.claude\reports\castle\CS-08b1-coder.md`.

### CS-06a: 1664 exit-grant seeds and replay tests

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-06a-1664-grant` / `castle-cs-06a`. **DB:** `sgw_castle_cs_06a`.

**Gate:** both of these.
- The CS-06S verdict is `SEALED`.
- **D-CA18:** the HA-01b receiver commit that CS-06S point 5 names is in `git log content/harset-campaign` (an ops check).

Until then the packet is **BlockedDependency (HA-01b)**. Never seed a grant the player cannot finish.

**Allowlist:**
- `db/resources/Content/Seed/castle_1664_chains.sql` (new; chain 1398, plus 1399 only if frozen);
- `db/database.sql` (one `\ir resources/Content/Seed/castle_1664_chains.sql` line after the last Castle include);
- `chain_replay_tests/mission_1664.rs` (new);
- `chain_replay_tests/mod.rs` (one line).

**Steps:**
1. Copy the frozen SQL from branch `evidence/1664.md` verbatim.
2. Write the CS-06a replay rows from CS-06S point 6. Drive them through `fire_mission_completed` (the `ordinance_rewards.rs:99` pattern), and for the end-to-end row through the real `stargate_crossed` dispatch that completes 708. Each test fails if chain 1398 is removed, or if the `mission_status 1664 not_active` condition is removed (the second-grant row).

**Lane, in order:**
1. `cargo fmt --all -- --check`;
2. `lane.ps1 cargo clippy -p cimmeria-cell-content -p cimmeria-content-engine -- -D warnings`;
3. `live-db-test.ps1 mission_1664 -p cimmeria-cell-content`;
4. `live-db-test.ps1 'mission_708 | castle_706_708' -p cimmeria-cell-content` (the 708 exit must not regress; record "0 tests matched" if the filter matches nothing).

**Commit:** `feat(castle): grant 1664 at the Castle exit (CS-06a)`. **Report:** `...\castle-cs-06a\.claude\reports\castle\CS-06a-coder.md`.

### CS-03d: the 1521 implant (Option A only)

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-03d-implant` / `castle-cs-03d`. **DB:** `sgw_castle_cs_03d`. **Gate:** the CS-03dS verdict is `SEALED`, and CS-INT2 has integrated the 1521 seeds. Status: **BlockedDecision (D-CA-SYM)**.

**Allowlist:**
- `db/resources/Content/Seed/castle_1521_chains.sql` (only the rows CS-03dS freezes);
- `chain_replay_tests/mission_1521_implant.rs` (new);
- `chain_replay_tests/mod.rs` (one line).

**Steps:**
1. Apply the frozen deltas verbatim.
2. Tests:
   - the grant happens at the sealed step;
   - no second grant on replay, relog or the second tank use;
   - the debuff rows, only if frozen.

   Each fails if its delta is reverted.

**Lane:** `fmt`; `lane.ps1 cargo clippy -p cimmeria-cell-content -- -D warnings`; `live-db-test.ps1 mission_1521 -p cimmeria-cell-content` (the entry and graph tests must stay green).

**Commit:** `feat(castle): grant the 1521 symbiote implant (CS-03d)`. **Report:** `...\castle-cs-03d\.claude\reports\castle\CS-03d-coder.md`.

R3b also holds CS-05a and CS-04a1s ([part 2](r3-packets-2.md#wave-r3b-continued)). That makes six packets at most, for one reviewer.

## Wave R3c: stacked follow-ons

### CS-08b2: 567 grant-once round trip (base = CS-08b1 reviewed tip)

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-08b2-567-round-trip` / `castle-cs-08b2`. **DB:** `sgw_castle_cs_08b2`.

**Allowlist:**
- `crates/services/src/castle_567_round_trip_tests.rs` (new);
- `crates/services/src/lib.rs` (one `#[cfg(test)]` and `mod` line).

**Test:** a live-DB round trip, `live_db_castle_carry_in_567_grant_once_round_trip`. Copy the Cell-to-Base-to-DB-to-relog shape of `frosts_letter_round_trip_tests.rs` (after CS-08a2M; if CS-08a2M has not integrated, `mission_round_trip_tests.rs:496`).
1. Fixture: a character with 567 active at step 2000, in world 8.
2. Fire the cabinet interaction. Assert that 2698 is in the persisted inventory and that the saved step is 2012.
3. Relog and fire the interaction again. Assert there is still exactly one 2698 and the step is unchanged.

It fails if CS-08bM's grant-once guard row is removed.

**Lane:** `fmt`; `lane.ps1 cargo clippy -p cimmeria-services --all-targets -- -D warnings`; `live-db-test.ps1 live_db_castle_carry_in -p cimmeria-services`.

**Commit:** `test(castle): prove the 567 file grant persists once per character (CS-08b2)`. **Report:** `...\castle-cs-08b2\.claude\reports\castle\CS-08b2-coder.md`.

### CS-06b: 1664 persistence round trip (base = CS-06a reviewed tip)

**Agent:** packet-coder. **Branch / worktree:** `castle/cs-06b-1664-round-trip` / `castle-cs-06b`. **DB:** `sgw_castle_cs_06b`.

**Allowlist:**
- `crates/services/src/castle_1664_round_trip_tests.rs` (new);
- `crates/services/src/lib.rs` (one line pair).

**Test:** `live_db_castle_1664_exit_grant_round_trip`, following the CS-06S point 6 list.
1. Complete 708 through the real dispatch. Assert that the saved missions contain 1664 active.
2. Relog. Assert that 1664 is still active at its first step.
3. A second completion dispatch adds nothing.

It fails if chain 1398 is removed.

**Lane:** `fmt`; `lane.ps1 cargo clippy -p cimmeria-services --all-targets -- -D warnings`; `live-db-test.ps1 live_db_castle_1664 -p cimmeria-services`.

**Commit:** `test(castle): prove the 1664 exit grant persists (CS-06b)`. **Report:** `...\castle-cs-06b\.claude\reports\castle\CS-06b-coder.md`.

### CS-08a2S: record the HA-1360a/b recipient interface

**Agent:** mission-systems-advisor (read-only on Harset). **Branch / worktree:** `castle/cs-08a2s-1360-interface` / `castle-cs-08a2s`. **Gate:** HA-1360a3c and HA-1360b are in `git log content/harset-campaign` (an ops check). Until then: **BlockedDependency (HA-1360a/b)**.

**Allowlist:** `docs/analysis/castle-continuation/worknotes/CS-08a2S.md` (new).

1. Record the Harset SHA, and the chain IDs, triggers and conditions that take 1360 from 4038 to completion, including where the letter is consumed (cite Harset's seed file:line).
2. Record the missing-letter refusal path and its telemetry reason.
3. Write the exact fixture and assertions for regression-contract steps 3 and 4 of branch `worknotes/CS-08a.md` (recipient completion, missing-letter refusal), for `frosts_letter_round_trip_tests.rs`.
4. Write the D-CA15 call: which campaign's PR reaches `main` second, from both ledgers' state. If Harset is second, CS-08a2 becomes **Dropped (Harset owns)**. Say so, and write the Harset-facing note the orchestrator relays.

**Commit:** `docs(castle): record the HA-1360 recipient interface (CS-08a2S)`. **Report:** `...\castle-cs-08a2s\.claude\reports\castle\CS-08a2S-mission.md`.

### CS-08a2: Frost's Letter recipient round trip (final phase only)

**Agent:** packet-coder. **Worktree:** `zone-castle` after CS-07R (a rebased branch that contains Harset's HA-1360a/b from `origin/main`). **Gate:** CS-08a2S `SEALED`, and the D-CA15 call says Castle merges second. **Status:** BlockedDependency (HA-1360a/b on `origin/main`).

**Allowlist:** `crates/services/src/frosts_letter_round_trip_tests.rs` only.

**Steps:** add the two CS-08a2S tests verbatim:
- `live_db_frosts_letter_recipient_completes_1360`;
- `live_db_frosts_letter_missing_letter_refused`.

Each fails if the Harset recipient chain, or its has-item condition, is removed.

**Lane:** `fmt`; `lane.ps1 cargo clippy -p cimmeria-services --all-targets -- -D warnings`; `live-db-test.ps1 frosts_letter -p cimmeria-services`.

**Commit:** `test(castle): prove the Frost's Letter recipient round trip (CS-08a2)`. **Report:** `...\zone-castle\.claude\reports\castle\CS-08a2-coder.md`.

R3c also holds CS-04a1c, CS-05c and CS-DOC3 ([part 2](r3-packets-2.md#wave-r3c-continued)). CS-05b (repeatables) is in R3d unless CS-05S finds a stock mission.
