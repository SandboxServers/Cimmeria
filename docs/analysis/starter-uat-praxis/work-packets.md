# Starter-world UAT, Praxis: work packets

> **Type:** runbook. **Audience:** the orchestrator, the brief authors, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md) (scope, state, decisions, status), [RESUME.md](RESUME.md). The [Union](../starter-uat-union/work-packets.md) and [shops](../starter-uat-shops/work-packets.md) campaigns reuse § Shared UAT contract below by link.

## Dispatch rules

1. **Wave 0, then wave 1.** `ops` creates `sp-integration` (`pwsh tools/build-lane/mk-worktree.ps1 starter-uat-praxis/integration sp-integration`) and commits these three campaign files into it first. SP-00 runs alone in it. When SP-00 is reviewed and committed there, `ops` creates one worktree per wave-1 packet **branched at the `sp-integration` head** (branch at SHA, so `rows.md` is present), and the orchestrator dispatches SP-01, SP-02a, SP-02b, SP-03a, SP-03b, SP-04 and SP-05 in one message, in the background. Each is merged back `--no-ff` (`merge-noff.ps1`) after its packet review passes.
2. **Authoring packets** go to `documentation-writer` as defined (no model override). They write Markdown only and run no cargo, except SP-91 and SP-92, which go to `packet-coder`.
3. **Lab waves (2 to 5) are ASK-FIRST** (contract § C1). The orchestrator asks the user before each lab wave and waits for a yes. Without one, the wave stays as it is.
4. **One lab lane per lab instance.** Instances `default` (account `lab`), `p2` (`lab2`), `p3` (`lab3`). A wave runs at most three drive packets at once, one per instance, and never two lanes on the same plaza or room (hub-pass lesson: lanes must not share a plaza). Each brief's § 0 names the area (room, plaza or hub) its rows use. Two lanes in the same world run at once only when their areas differ; otherwise the later packet waits and runs on the same instance after the earlier one. "A then B" in the waves table means B starts on that instance only after A's verifier has returned.
5. **Per drive packet:** `lab-driver` as defined (Haiku), prompt = the packet's brief file pasted verbatim with `ATTEMPT:` set. On its report, start the verifier: `general-purpose`, no model override, prompt = the packet's verify file pasted verbatim with the driver's report under `## DRIVER REPORT`. Three driver attempts, then one Sonnet takeover (`general-purpose`, the takeover paragraph in [hub-pass work-packets § Sonnet takeover](../hub-pass-uat/work-packets.md#sonnet-takeover), `ATTEMPT: 4`).
6. **Results on disk.** The verifier writes `docs/analysis/starter-uat-praxis/results/<packet>.md` in the `sp-integration` worktree (contract § C6) and returns one line. The orchestrator updates the README status row and the grade column of `rows.md`.
7. **Issues.** A verifier never files an issue itself. It writes an issue body file under `results/issues/<packet>-<n>.md`; `ops` files it (`issue create` from a file) and the orchestrator writes the number back into the result file.
8. **Between packets** check `lab_client_status` (`crash_count_10min`, `login_state`). Two crashes in ten minutes: stop the lane and diagnose before another attempt (screensaver, a build on the lane, disk full).
9. **Review loops** (campaign-packet skill § 4) run on the authoring packets (packet loop, then the wave-1 loop) and on SP-91/92. Lab results are reviewed by their verifier, not by `packet-reviewer`.

## Shared UAT contract

Every drive packet in starter-uat-praxis, starter-uat-union, starter-uat-shops and debug-armory follows this contract.

### C1. Ask first

> **LAB: ASK-FIRST.** Before the first packet of a lab wave, the orchestrator asks the user with AskUserQuestion: "May I take the lab now for wave W (packets …, about N minutes, against server S)?" and waits for a yes. No yes means the wave stays as it is. A yes covers that wave only. Never force another owner's lease. Start clients only when the build lane is idle; kill the screensaver (`Ribbons.scr`) first.

### C2. Driver brief template

Copy the structure of [hub-pass briefs/HP-M1.md](../hub-pass-uat/briefs/HP-M1.md) exactly:

- Header lines: `PACKET:`, `ATTEMPT: 1`, `INSTANCE:`, `CHARACTER:`, `SERVER:` (from D-SP2), lease owner `<campaign-prefix>-<packet>` (for example `sp-SP-DCB1`), TTL 900 s, hard stop at 28 tool calls.
- `§ 0 Front-loaded gotchas`: copy the 10 hub-pass lessons ([hub-pass README § Lessons](../hub-pass-uat/README.md#lessons-for-the-next-lab-campaign)) and add the zone's own (positions to stand at, 4 to 6 m off targets, dialogs to clear first, known issue ids the row may hit).
- `§ 1 Fixed macros`: lease, ensure-in-world, WAIT (two `client_batch` waits of 20 s, never `client_wait_for` on a world load, #1677), CHAT (filter server replies by channel id `"9"`), READ, SHOT, DIALOG, PROMPT (`Prompt1_SingleButton`), BUG.
- `# <packet>`: `ROWS:`, `ESTIMATE:` (tool calls), then `## Steps`: S1 lease and ensure in world, S2 the **resume probe** (what the driver reads to see which rows are already done), then one `### ROW <id>` block per row, each with exact tool calls, exact ids, positions and expected server-visible effect.
- `§ 2 Report format` and `§ 3 Always end like this` (release the lease; raw facts only, no diagnosis).

### C3. Verify brief template

Copy [hub-pass verify/HP-M1.md](../hub-pass-uat/verify/HP-M1.md): rules (grade only on server evidence; a clause seen only in the client UI or a screenshot is PARTIAL or NEEDS_HUMAN), setup, the ClickHouse-over-ssh helper (`reference_signoz_clickhouse_via_ssh`), the Haiku looker step for screenshots, the report format (§ C6), what to do on FAIL (write an issue body file, rule 7), a "do not" list (no file edits outside `results/`, no lab tools, no fixes), one grading section per row naming the exact log event or DB query that proves it, then `## DRIVER REPORT`.

### C4. Fixtures and setup

- A row that starts after Cellblock uses a fixture: `server_stage_fixture` with `world`, `position`, `missions` and `items` (fields in `crates/lab/src/uat/spec_fixture.rs`; at most 10 items). Characters are created as `Uat Fx<key><n>` on the GM account.
- Fixtures have no level, cash or ability fields. Setup uses `.givexp <n>`, `/gmgivecash <n>` and `/gmgiveability <id>` (G tier) and says so in the row.
- A fixture that cannot be set up makes the row BLOCKED, never FAIL.
- A brief never uses `.gotolocation` onto hostile NPCs; it never uses `client_move_to` in a hub.

### C5. Grades

PASS, PARTIAL, FAIL, BLOCKED, NOT-RUN, NEEDS_HUMAN, NATIVE_SHORTFALL (server correct, the action had to be sent by stock Lua). PASS needs server evidence (a log row, a DB read or a packet-tap row). A FAIL in an early world that blocks later rows makes those rows BLOCKED, not FAIL.

### C6. Result file

`results/<packet>.md`, CRLF, one table: `| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |`, then `## Not run` (each clause and why), then `## Resume hint` (state a re-run would start from), then `## Recipe` (exact tool calls that worked, for SP-91/92).

### C7. Repo rules every authoring packet carries

1. Write only the files the packet names. No `git stash`, no push, no `git worktree prune`. Commit locally with the packet's subject and the attribution lines the brief gives, then stop.
2. `.md` files under `docs/` are **CRLF**. After writing, run in PowerShell: `Get-ChildItem <your .md files> | ForEach-Object { $t=[IO.File]::ReadAllText($_.FullName); $t=$t -replace "`r?`n","`r`n"; [IO.File]::WriteAllText($_.FullName,$t) }` with **absolute paths**. Exception: the TOML specs under `docs/guides/uat-specs/` stay **LF** like the files already there; never run the CRLF conversion on a `.toml`.
3. Soft cap 500 lines per file, hard cap 700. A brief over 500 lines is two packets.
4. Every id, coordinate and dialog name in a brief is traced from `db/resources/` or the zone ledger and cited inline (`file:line`). Never invent one; an untraceable row is written `BLOCKED: <what is missing>`.
5. Never hand-edit a `<!-- gen:NAME -->` block.
6. Check `docs/` and `.claude/agent-memory/` before guessing client behaviour (`lab_driving`, `movement-teleport-advisor`, `mission-systems-advisor` notes).
7. A ticket or ledger row is a claim, not evidence: the brief tests what the row says, and the verifier grades what the server shows.

### C8. Spec runs (`lab_uat_run`)

Any packet that runs a committed TOML spec instead of a step brief (starter-uat-shops SS-D1 to SS-D4, debug-armory DR-L1 and the DR-S packets) makes its calls in this order, as in [automated-uat.md § Hand a run to a cheap agent](../../guides/automated-uat.md):

1. **Plan:** `lab_uat_run` with `plan_only: true` and the packet's `sections` and `rows`. Ready rows come back SKIPPED; the rest come back BLOCKED with the missing tool named. A row BLOCKED in the plan is left out of the execute call, and the verifier grades it BLOCKED with the plan's reason. If no row is ready, the packet stops here.
2. **Execute:** one `lab_uat_run` call with the same `sections` and only the ready `rows`, no `plan_only` and no `lease_id` (the run takes and releases its own lease). The driver notes the `run_dir` it returns.
3. **Resume after a stop:** if the call times out, or the run passes 90 minutes and is stopped, the driver runs the rows not yet in that run in one more call **with the same `run_dir`** ("rerun the remaining rows with the same `run_dir`", automated-uat.md). When the packet names split spec files instead of one spec, each split file gets its own plan call and execute call, in the order the packet lists them, and its own `run_dir`.
4. **Record:** the driver reports every call's arguments and the `run_dir` it returned. The verifier copies them into a `## Runs` table at the top of `results/<packet>.md`: `| Call | Spec or split file | plan_only | run_dir | Outcome |`. A row graded from a run with no recorded `run_dir` is NEEDS_HUMAN.

## Wave 0

### SP-00: row inventory and lane plan

- **Agent:** documentation-writer. **Host:** windows. **Worktree:** `sp-integration` (branch `starter-uat-praxis/integration`). **Commit:** `docs(starter-uat-praxis): SP-00 row inventory`.
- **Read:** [cellblock-continuation README](../cellblock-continuation/README.md) and [owner-lab-checklist.md](../cellblock-continuation/owner-lab-checklist.md); [castle-continuation/uat/end-uat.md](../castle-continuation/uat/end-uat.md) and its [README](../castle-continuation/README.md) (FZ-U1); [harset-continuation README](../harset-continuation/README.md) and RESUME (FZ-U2); [tollana-continuation README](../tollana-continuation/README.md) (P1-P10, U1-U7, T18); [unified-uat.md](../../guides/unified-uat.md) Castle CellBlock section (T01-T25).
- **Write** `docs/analysis/starter-uat-praxis/rows.md`: one table `| Row id | World | Source (file#anchor) | What it checks (one line) | Fixture or setup | Drive packet | Grade |`. Grade is empty. Assign each row to a drive packet using the provisional split below; keep at most 5 rows and 25 estimated calls per packet; if a world needs more packets than listed, add `SP-D<WW><n+1>` and note it in a `## Split changes` section (the orchestrator adds the README rows).

  | Packet | Provisional rows |
  |---|---|
  | SP-DCB1 | CB-U01 to CB-U04 |
  | SP-DCB2 | CB-U05 to CB-U08 |
  | SP-DCB3 | CB-U09 to CB-U12, plus the T01-T25 visual halves that need a screenshot |
  | SP-DCA1 | Castle A1-A5 |
  | SP-DCA2 | Castle D1-D4 |
  | SP-DCA3 | Castle D5-D7 |
  | SP-DCA4 | Castle Mission 1521 (Symbiote Tank) S1-S5 |
  | SP-DCA5 | Castle Mission 1521 S6-S8, FZ-U1 (mission 567) |
  | SP-DCA6 | Castle hub quartermaster S1-S5 |
  | SP-DCA7 | HB1, HB2, RQ-R2 and any end-uat row not listed above |
  | SP-DHA1 | H20, H31, H40, H41 |
  | SP-DHA2 | HA-1360a, HA-1360b (letter hand-in, refusal without the letter, one-shot after completion, O14-17/O14-19) |
  | SP-DHA3 | FZ-U2a, FZ-U2b, FZ-U2c: one Praxis character through mission 1664 in the order Kaerlia, door control (D-PU6 pinned here), Marsh then Moh'katan |
  | SP-DHA4 | FZ-U2d (the other report order), FZ-U2h (Human with 1361 not active watches Marsh) |
  | SP-DHA5 | FZ-U2e, FZ-U2f, FZ-U2g (abandon 1664 at each step) |
  | SP-DTO1 | Tollana P1-P5 |
  | SP-DTO2 | Tollana P6-P10 |
  | SP-DTO3 | Tollana U1-U4 |
  | SP-DTO4 | Tollana U5-U7, T18 |
  | SP-DL1 | leg: Cellblock exit ring (chain 1109) to Castle arrival, character state carried |
  | SP-DL2 | leg: Castle→Harset stargate crossing advances mission 1360 step 4037→4038 with item 3730 kept (O14-16) |
  | SP-DL3 | leg: Harset to Tollana hand-off (find the hand-off mission in the Harset and Tollana ledgers; if none is playable, the packet is one BLOCKED row naming what is missing) |

  FZ-U2 ([unified-uat.md](../../guides/unified-uat.md), the FZ-U2 row in the Harset section) is one long row. `rows.md` splits it into these sub-rows, each graded on its own; FZ-18 to FZ-27 are the four-zone build packets that made it, not UAT rows:

  | Sub-row | What it checks (from FZ-U2's expected column) | Start state |
  |---|---|---|
  | FZ-U2a | Kaerlia has a name plate, sensible facing and a cue; her click opens her dialog and the journal reads "Make your way to the Command Center"; a relog keeps it | Castle 708 done, 1664 granted, at the Harset gate |
  | FZ-U2b | The door control shows a use cursor; its click takes you into the Command Center with the journal on "Report for debriefing"; `.bug pin cmd door control` and `.location` at the panel (D-PU6); a relog keeps it | after FZ-U2a |
  | FZ-U2c | Marsh, then Moh'katan: each shows a cue, each click puts that NPC's line in chat and ticks one objective; re-clicking Marsh before the second report gives nothing new; a relog mid-5226 keeps the cue only on Moh'katan; the second report completes 1664 | after FZ-U2b |
  | FZ-U2d | The other order (Moh'katan, then Marsh) on a second character, with the same checks as FZ-U2c | fixture: 1664 at step 5226 in the Command Center |
  | FZ-U2e | Abandon 1664 at the Kaerlia step: every 1664 cue gone, Kaerlia says "Keep moving...", 1664 not offered again (option P0, O18-02a) | fixture: 1664 at its first step, at the gate |
  | FZ-U2f | Abandon at the door-control step, with the same checks as FZ-U2e | fixture: 1664 at the door step |
  | FZ-U2g | Abandon at 5226 after reporting to only one NPC, debrief cues included, with the same checks as FZ-U2e | fixture: 1664 at 5226, one report made |
  | FZ-U2h | A Human with 1361 not active: Marsh's "?" offer before and after 1664 ends, and after an abandon at 5226 made in the Command Center versus elsewhere (it returns on the next Command Center entry) | fixture per the harset ledger; BLOCKED if a Human cannot hold 1664 |

- **Also:** in [tollana-continuation/RESUME.md](../tollana-continuation/RESUME.md), replace the "local only, no PR" status line with one line saying the work merged in #1948. Change nothing else there.
- **Checks:** CRLF (§ C7 rule 2); every row id in `rows.md` appears in its source file (`Select-String` each; for a split sub-row such as FZ-U2c, search for the base id FZ-U2); no drive packet holds more than 5 rows.
- **Rules:** § C7.

## Wave 1: briefs (docs only, one PR from `sp-integration`)

Each packet below: **Agent** documentation-writer, **Host** windows, worktree `sp-<packet>` on branch `starter-uat-praxis/<packet>-briefs`, commit `docs(starter-uat-praxis): <packet> briefs`. It writes, for each drive packet it owns, `docs/analysis/starter-uat-praxis/briefs/<drive>.md` (§ C2) and `docs/analysis/starter-uat-praxis/verify/<drive>.md` (§ C3), using the rows `rows.md` assigns. Rules § C7. Checks: CRLF; each brief ≤ 500 lines; each row in `rows.md` for its drive packets has a `### ROW` block in the brief and a grading section in the verify file (`Select-String` count equal).

### SP-01: Cellblock briefs (SP-DCB1, DCB2, DCB3)

- **Read first:** [castle-cellblock.toml](../../guides/uat-specs/castle-cellblock.toml) and [routes/castle-cellblock.toml](../../guides/uat-specs/routes/castle-cellblock.toml) (recorded routes: reuse `prisoner-to-med-station`), [first-session.toml](../../guides/uat-specs/first-session.toml), the cellblock-continuation checklist.
- **Character:** a fresh Praxis Commando per lane (`char_creation` 13). The cell door opens only after mission 638; the route file says how.
- **Gotchas to front-load:** #1667 (pistol), LMG effect 644 inert (expected, not a FAIL of the row unless the row is about it), dialog 60151 client syntax error (record, screenshot, continue).

### SP-02a: Castle briefs part 1 (SP-DCA1 to DCA4)

- **Read first:** [castle-continuation/uat/end-uat.md](../castle-continuation/uat/end-uat.md), [castle-continuation/placements.md](../castle-continuation/placements.md), [CS-01-walk.md](../castle-continuation/uat/CS-01-walk.md) (the paused walk: "walking was blocked by lab input"; copy its working parts).
- **Character:** fixture `world = Castle`, position from placements.md; never log a fresh client straight into Castle (#756): the fixture lands at character select, then Play.
- **Gotchas:** K20 (friendly NPCs give no kill credit), interior/exterior navmesh not connected (use GM teleport between them, tier G).

### SP-02b: Castle briefs part 2 (SP-DCA5 to DCA7)

Same reads, character rule and gotchas as SP-02a. FZ-U1 (mission 567) is in SP-DCA5: trace its chain from `db/resources/` and the castle ledger.

### SP-03a: Harset briefs part 1 (SP-DHA1, DHA2)

- **Read first:** [harset-continuation README](../harset-continuation/README.md), [route-baseline.md](../harset-continuation/route-baseline.md), [owner-lab-checklist.md](../harset-continuation/owner-lab-checklist.md), [evidence-matrix.md](../harset-continuation/evidence-matrix.md), the O14-16/17/19 rows of [owner-decisions-2026-10-14.md](../four-zone-planning/owner-decisions-2026-10-14.md).
- **Character:** fixture `world = Harset` (57) or the interior world the row needs; SP-DHA2 needs item 3730 in the fixture `items` for the hand-in rows and **not** for the refusal row.
- **Gotchas:** Market has no navmesh (teleport, tier G); door pairs unplaced (a door row that cannot be reached is BLOCKED, not FAIL). Each brief's § 0 names the area its rows use (dispatch rule 4).

### SP-03b: Harset briefs part 2 (SP-DHA3, DHA4, DHA5)

- **Read first:** as SP-03a, plus the FZ-U2 row of [unified-uat.md](../../guides/unified-uat.md) and the FZ-U2a to FZ-U2h split in SP-00.
- **Character:** SP-DHA3 is one Praxis character carried through FZ-U2a, b and c in order. SP-DHA4 and SP-DHA5 start each sub-row from the fixture the SP-00 split names (§ C4), so a row never depends on another row's leftovers; trace each fixture's mission step ids (1664, 5226) from `db/resources/` and cite them.
- **Gotchas:** the D-PU6 door-control position is pinned during SP-DHA3: the brief asks the driver to type `.bug pin cmd door control` and record `.location` at the control. The two debrief lines are provisional text (D-FZ15): grade that a line appears, not its wording. "Not offered again" is option P0 (O18-02a); if the owner has since picked P3, a re-offer is the expected result. All three packets use the gate and the Command Center, so they run one after another on one instance (waves table).

### SP-04: Tollana briefs (SP-DTO1 to DTO4)

- **Read first:** [tollana-continuation README](../tollana-continuation/README.md), [T-03a-568-chain-spec.md](../tollana-continuation/evidence/T-03a-568-chain-spec.md), [phasing.md](../tollana-continuation/evidence/phasing.md), [tollana-568-spec.md](../../../.claude/agent-memory/mission-systems-advisor/tollana-568-spec.md) (the accept-on-close wart and one-shot chain gate) and [interact-dialog-routing-traps.md](../../../.claude/agent-memory/mission-systems-advisor/interact-dialog-routing-traps.md).
- **Character:** fixture `world = Tollana` (19). Phasing rows (PH-01a/b, mission 621) need two players: lanes `default` and `p2` together, so SP-DTO3 or DTO4 (whichever holds them) is a two-instance packet; say so in its header. It runs alone, after the other Tollana packets (waves table).
- **Gotchas:** the zero-button accept-on-close wart on mission 568 (expected), the Decline button on dialog 2153 (owner decision pending; record what it does).

### SP-05: hand-off leg briefs (SP-DL1, DL2, DL3)

- **Read first:** [ring-transport-cellblock-castle README](../ring-transport-cellblock-castle/README.md) (no ring animation without the client patch: the leg grades the server-side transfer only), O14-16 (chain 1366, Castle→Harset crossing), the Harset and Tollana ledgers for the third leg.
- Each leg starts from a fixture at the end state of the previous world and checks: the transfer happens on the first press, the arrival point is on the navmesh (no fall), mission and item state survive the transfer, and a relog after arrival keeps it.

## Waves 2 to 5: lab drive packets (ASK-FIRST)

Every drive packet: **LAB: ASK-FIRST** (§ C1). Agent `lab-driver` (brief) then verifier (`general-purpose`, verify file). Host: windows (the lab machine). Lease owner `sp-<packet>`. Writes only `results/<packet>.md` and `results/issues/` through the verifier (§ C6). Depends on its brief being integrated and on D-SP1 and D-SP2.

| Wave | Packet | Brief / verify | Instance | Depends on |
|---|---|---|---|---|
| 2 | SP-DCB1 | briefs/SP-DCB1.md, verify/SP-DCB1.md | default | SP-01 |
| 2 | SP-DCB2 | SP-DCB2 | p2 | SP-01 |
| 2 | SP-DCB3 | SP-DCB3 | p3 | SP-01 |
| 2 | SP-DL1 | SP-DL1 | default (after DCB1) | SP-05 |
| 2 | SP-DCA1 | SP-DCA1 | p2 (after DCB2) | SP-02a |
| 2 | SP-DCA2 | SP-DCA2 | p3 (after DCB3) | SP-02a |
| 3 | SP-DCA3 then SP-DCA6 | | default | SP-02a, SP-02b |
| 3 | SP-DCA4 then SP-DCA7 | | p2 | SP-02a, SP-02b |
| 3 | SP-DCA5 then SP-DL2 | | p3 | SP-02b, SP-05 |
| 4 | SP-DHA1 then SP-DL3 | | default | SP-03a, SP-05 |
| 4 | SP-DHA2 | | p2 | SP-03a |
| 4 | SP-DHA3 then SP-DHA4 then SP-DHA5 | | p3 | SP-03b |
| 5 | SP-DTO1 then SP-DTO2 then the one-instance packet of SP-DTO3/DTO4 | | default | SP-04 |
| 5 | the two-instance packet of SP-DTO3/DTO4 (PH-01a/b) | | default + p2, alone, after the default chain | SP-04 |
| 2-5 | attempt-4 takeovers | | the failed packet's instance, after that instance's chain in the same wave | the three Haiku attempts |

Wave 3 and wave 4 lanes share a world (Castle, Harset); dispatch rule 4 still applies, so if two concurrent briefs name the same area in § 0, the orchestrator moves the later one to the end of the other's chain. Tollana is one small hub, so wave 5 runs its packets one at a time on `default`, and the two-player packet runs last with no other lane open.

A wave's results ship as one docs PR from `sp-integration` (README rows, `rows.md` grades, `results/`).

## Wave 6: specs, roll-up, close-out

### SP-91: TOML specs for passed Cellblock and Castle rows

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `sp-91`, branch `starter-uat-praxis/SP-91-specs`. **Commit:** `docs(uat-specs): Praxis Cellblock and Castle rows from starter-uat-praxis`.
- **Input:** `results/SP-DCB*.md`, `results/SP-DCA*.md`, `results/SP-DL1.md` § Recipe. Only rows graded PASS or NATIVE_SHORTFALL with a recipe.
- **Write:** add rows to `docs/guides/uat-specs/castle-cellblock.toml` (Cellblock) and a new `docs/guides/uat-specs/castle.toml` (section id `castle`, `account = "gm"`, `character = "fresh"`, fixtures per § C4). Copy the row shape of [abilities.toml](../../guides/uat-specs/abilities.toml) (`[[row]]`, `setup`, `step`, `teardown`, `[[row.expect]]`). Row ids are the ledger ids prefixed `SP-` (for example `SP-CB-U03`). One header comment per file citing this campaign.
- **Checks, in this order** (the only cargo; no Rust is edited, so no `cargo fmt` or `cargo clippy`):
  1. `Set-Location <worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded` (both guards in `crates/lab/src/uat/mod.rs`: every spec parses and validates, and every tag a spec names is seeded).
  2. `git -C <worktree> diff --name-only` lists only the TOML files the packet names.

  Both must pass. Fix rounds ≤ 5; on the sixth failure, stop and report `blocked` with the lane log path.
- **Rules:** § C7 rules 1, 2 (the TOML exception: never convert a `.toml` to CRLF), 3, 4; run the lane in the foreground, from the worktree.

### SP-92: TOML specs for passed Harset, Tollana and leg rows

- **Agent:** packet-coder. **Host:** windows. **Worktree:** `sp-92`, branch `starter-uat-praxis/SP-92-specs`. **Commit:** `docs(uat-specs): Praxis Harset and Tollana rows from starter-uat-praxis`.
- **Input:** `results/SP-DHA*.md`, `results/SP-DTO*.md`, `results/SP-DL2.md`, `results/SP-DL3.md` § Recipe, with the same PASS or NATIVE_SHORTFALL filter as SP-91.
- **Write:** new `docs/guides/uat-specs/harset.toml` (section id `harset`) and `docs/guides/uat-specs/tollana.toml` (section id `tollana`), in the SP-91 row shape and id prefix. The FZ-U2 sub-rows keep their letters (`SP-FZ-U2c`).
- **Checks, in this order:** the same two as SP-91, run from `sp-92`: `Set-Location <sp-92 worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`, then the `git diff --name-only` check. Fix rounds ≤ 5.
- **Rules:** as SP-91.

### SP-98: issue roll-up and sign-off sheet

- **Agent:** documentation-writer. Worktree `sp-integration`. Commit `docs(starter-uat-praxis): SP-98 results summary`.
- **Write:** a `## Results summary` section at the top of README.md in the shape of [hub-pass README § Results summary](../hub-pass-uat/README.md#results-summary-final-2026-10-13) (grade counts, per-world sign-off table, issues filed, lessons) and `signoff.md` listing every NEEDS_HUMAN row with its screenshot path for the owner.

### SP-99: close-out

- **Agent:** documentation-writer. **Depends on:** SP-98 and the four-zone FZ-99 close-out (D-SP7). Worktree `sp-integration`. Commit `docs(starter-uat-praxis): SP-99 close-out`.
- **Write:** in each zone ledger (cellblock, castle, harset, tollana continuation READMEs) change only the UAT rows this campaign graded, citing `results/<packet>.md`. In `docs/guides/unified-uat.md` add a `## Tollana` section and update the Castle CellBlock, Castle and Harset rows with the grade and date. Then the status docs once: one line per world in `docs/project-status.md` and the matching `docs/gap-analysis*` rows (walk [doc-update-map.md](../../agents/doc-update-map.md); list the rows touched in the PR body). Rewrite RESUME.md to "closed". Rules § C7.
