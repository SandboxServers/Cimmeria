# Starter-world UAT, Praxis: work packets

> **Type:** runbook. **Audience:** the orchestrator, the brief authors, the lab drivers and the verifiers.
> **Companions:** [README.md](README.md) (scope, state, decisions, status), [RESUME.md](RESUME.md). The [Union](../starter-uat-union/work-packets.md) and [shops](../starter-uat-shops/work-packets.md) campaigns reuse § Shared UAT contract below by link.

## Dispatch rules

1. **Wave 0, then wave 1.** `ops` creates `sp-integration` (`pwsh tools/build-lane/mk-worktree.ps1 starter-uat-praxis/integration sp-integration`) and commits these three campaign files into it first. SP-00 runs alone in it. When SP-00 is reviewed and committed there, `ops` creates one worktree per wave-1 packet **branched at the `sp-integration` head** (branch at SHA, so `rows.md` is present), and the orchestrator dispatches SP-01, SP-02a, SP-02b, SP-03a, SP-03b, SP-04 and SP-05 in one message, in the background. Each is merged back `--no-ff` (`merge-noff.ps1`) after its packet review passes.
2. **Authoring packets** go to `documentation-writer` as defined (no model override). They write Markdown only and run no cargo, except SP-91 and SP-92, which go to `packet-coder`.
3. **Lab waves (2 to 5) are ASK-FIRST** (contract § C1). The orchestrator asks the user before each lab wave and waits for a yes. Without one, the wave stays as it is.
4. **One lab lane, one client at a time.** Drive packets run one at a time, in the waves-table order, with one client process running at a time. Before a part starts on its instance, the orchestrator stops every other instance's client (`lab_client_stop`) and checks with `lab_client_status` (no other instance `running`). Reason: concurrent clients fight over the desktop foreground (12 of 12 rows blocked in this campaign's two-client run; `reference_lab_login_foreground_lanes_2026_10_14`). The only concurrent run is the wave 5 two-client part, SP-DTO2-p1 (the SP-04 spec called that two-player case PH-01a/b): it runs alone, on `default` + `p2`, after every other wave 5 part, with no other client open. "After lease of A" means start once A's driver has released its lease; "after verify of A" means start once A's verifier has returned (used when two parts share an area or spawn, and between the parts of one split packet). Each brief's `AREA:` header line names the area its rows use, so a plan to bring parallel lanes back can check overlap. A split packet takes its parent's slot and its parts run in order. Attempt-4 takeovers run at the end of their wave, on the failed part's instance (rule 5).
5. **Per drive packet** (a packet split into parts, `SP-DCB1-p1`, `SP-DCB1-p2`, and so on, is one drive packet per part, each with its own brief, verify file, result file and attempt count): `lab-driver` as defined (Haiku), prompt = the part's brief file pasted verbatim with `ATTEMPT:` set. On its report, start the verifier: `general-purpose`, no model override, prompt = the part's verify file pasted verbatim with the driver's report under `## DRIVER REPORT`. **Three driver attempts, then one Sonnet takeover.** The `ATTEMPT:` field is the count: `ATTEMPT: 1` is the first driver; `ATTEMPT: 2` and `ATTEMPT: 3` are successors of a driver that ran out of context (the brief's resume probe skips the finished rows), and each of them **spends one of the three attempts**, so a part re-run with `ATTEMPT: 2` has two attempts left, not three. The takeover is `general-purpose`, the takeover paragraph in [hub-pass work-packets § Sonnet takeover](../hub-pass-uat/work-packets.md#sonnet-takeover), `ATTEMPT: 4`.
6. **Results on disk.** The verifier writes `docs/analysis/starter-uat-praxis/results/<packet>.md` (`<packet>` is the part id, for example `SP-DCB1-p1`) in the `sp-integration` worktree (contract § C6) and returns one line. The orchestrator updates the README status row and the grade column of `rows.md`.
7. **Issues.** A verifier never files an issue itself. It writes an issue body file under `results/issues/<packet>-<n>.md`; `ops` files it (`issue create` from a file) and the orchestrator writes the number back into the result file.
8. **Between packets** check `lab_client_status` (`crash_count_10min`, `login_state`). Two crashes in ten minutes: stop the lane and diagnose before another attempt (screensaver, a build on the lane, disk full).
9. **Review loops** (campaign-packet skill § 4) run on the authoring packets (packet loop, then the wave-1 loop) and on SP-91/92. Lab results are reviewed by their verifier, not by `packet-reviewer`.

## Shared UAT contract

Every drive packet in starter-uat-praxis, starter-uat-union, starter-uat-shops and debug-armory follows this contract.

### C1. Ask first

> **LAB: ASK-FIRST.** Before the first packet of a lab wave, the orchestrator asks the user with AskUserQuestion: "May I take the lab now for wave W (packets …, about N minutes, against server S)?" and waits for a yes. No yes means the wave stays as it is. A yes covers that wave only. Never force another owner's lease. Start clients only when the build lane is idle; kill the screensaver (`Ribbons.scr`) first.

### C2. Driver brief template

Copy the structure of [hub-pass briefs/HP-M1.md](../hub-pass-uat/briefs/HP-M1.md) exactly:

- Header lines: `PACKET:`, `ATTEMPT: 1`, `INSTANCE:`, `CHARACTER:`, `SERVER:` (from D-SP2), lease owner `<campaign-prefix>-<packet>` (for example `sp-SP-DCB1`), TTL 900 s, hard stop at 28 tool calls; ESTIMATE ≤ 22, counted per § C2a. `AREA:` names the room, plaza or hub the rows use (dispatch rule 4). A brief whose planned count is over 22 is split into part files `briefs/<packet>-p<n>.md` and `verify/<packet>-p<n>.md`, cut at row boundaries; a sub-row carries a letter (`CB-U03a`).
- `§ 0 Front-loaded gotchas`: copy the 10 hub-pass lessons ([hub-pass README § Lessons](../hub-pass-uat/README.md#lessons-for-the-next-lab-campaign)) and add the zone's own (positions to stand at, 4 to 6 m off targets, dialogs to clear first, known issue ids the row may hit).
- `§ 1 Fixed macros`, each with its cost: L1 (lease), L2 (ensure-in-world), WAIT (two `client_batch` waits of 20 s, never `client_wait_for` on a world load, #1677), CHATX and CHAT1 (filter server replies by channel id `"9"`), READ, SHOT, JOURNAL, DIALOG, PROMPT (`Prompt1_SingleButton`), BUG. Costs and the call-count rule: § C2a.
- `# <packet>`: `ROWS:`, `ESTIMATE:` (tool calls), then `## Steps`: S1 lease and ensure in world, S2 the **resume probe** (what the driver reads to see which rows are already done), then one `### ROW <id>` block per row, each with exact tool calls, exact ids, positions and expected server-visible effect.
- `§ 2 Report format` and `§ 3 Always end like this` (release the lease; raw facts only, no diagnosis).

### C2a. Call-count rule

Set by the SP-02a ruling (review 1, D1). Every drive brief follows it, and so does every later brief that links this contract (Union, shops, debug-armory).

- **A call is one MCP tool invocation**, whatever the tool: `lab_lease_acquire`, every `client_batch`, every `client_ui_sequence`, every read, click and screenshot, `lab_lease_release`, and any tool-schema or tool-search lookup. A `client_batch` with 12 steps is 1 call. Two tools in one step are 2 calls.
- **ESTIMATE = the planned success path**, from L1 to `lab_lease_release` inclusive. A conditional call the brief makes routine counts at its maximum. Failure-path calls do not count (gotcha 10 SHOT and retry, a lease refusal, PROMPT after a click that opened nothing, the JOURNAL key-and-reread fallback): the headroom of 6 is for them.
- **ESTIMATE ≤ 22 per brief file; the hard stop stays at 28.** The cap exists for the Haiku driver's context, not for lab stability, so it is not raised. A split costs about 5 calls (lease, ensure, WAIT, resume probe, release) and one more verifier run; running out of context mid-row costs a whole attempt.
- **Label format.** Every step that makes calls carries `(<N> call[s]: <tool>, <tool>, ...)`, one tool name per call, in order, so N equals the number of names. The ESTIMATE line is the sum of the labels: `ESTIMATE: <sum> tool calls (planned success path; hard stop 28)`.
- **No schema-read steps.** The author writes the exact JSON inline, citing `file:line` in `crates/lab/src/server/`. A Haiku never improvises arguments.
- **Macro costs:** L1 = 1; L2 = 1; WAIT = 1; CHATX = 1, whatever the number of lines; CHAT1 = 3; READ = 1; SHOT = 1; JOURNAL = 1; PROMPT = 1 plus 1 per button clicked; DIALOG = 0 when it is the first `lua` op of a `client_batch` the step already makes, otherwise 1; BUG = 0 inside a CHATX, otherwise 1.
- **Folds.** `client_batch` (at most 64 steps) runs `lua`, `wait`, `player_state`, `window_text`, `mem_read` and `call_native` in one call. `client_ui_sequence` (at most 32 actions) runs CEGUI `click`, `key`, `type`, `drag`, `wait_window` and `wait` in one call. Never folded: `client_world_click`, `client_entity_find`, `client_chat_log`, `client_window_read`, `client_window_click`, `client_ui_clickables`, `client_ui_state`, `lab_screenshot`, `lab_logout`, `lab_ensure_in_world`, `lab_livewire_plan`, `client_minigame_play`.
- **Gotcha 11 (covering window, #1676), as the briefs state it.** Before every world click, the step's batch runs DIALOG as its first op. Run PROMPT only when the preceding L2, WAIT, WAITW or screenshot output shows a window or prompt name; if it shows none, skip PROMPT (no call). On a dead click the order is SHOT, then PROMPT only if the SHOT shows a window or prompt name, then one unchanged retry. If a click still does nothing, read `client_ui_state` once and record the top window.
- **Split parts.** A part starts from a fixture of its own when the start state is expressible (`missions` with `step`, `position`, `items`), so a FAIL in part 1 does not block part 2. A part that needs a chain binding the fixture does not replay (a dialog set added on `mission_accepted`, a glyph or interact binding) runs **carry-over** instead: same character, after the previous part's verifier returns, and a part-1 FAIL makes its rows BLOCKED (§ C5). A brief header says `Start: fixture` or `Start: carry-over from -p<n>`. Three attempts and the takeover are counted per part.
- **Check, per brief file** (absolute path, PowerShell): `$t=[IO.File]::ReadAllText('<brief>'); $l=[regex]::Matches($t,'\((\d+) calls?: ([^)]+)\)'); $bad=@($l|?{[int]$_.Groups[1].Value -ne ($_.Groups[2].Value -split ',').Count}).Count; $sum=($l|%{[int]$_.Groups[1].Value}|Measure-Object -Sum).Sum; $est=[regex]::Match($t,'ESTIMATE: (\d+) tool calls').Groups[1].Value; "sum=$sum estimate=$est bad=$bad"`. It passes when `sum` equals `estimate`, `estimate` ≤ 22 and `bad` = 0.

### C3. Verify brief template

Copy [hub-pass verify/HP-M1.md](../hub-pass-uat/verify/HP-M1.md): rules (grade only on server evidence; a clause seen only in the client UI or a screenshot is PARTIAL or NEEDS_HUMAN), setup, the ClickHouse-over-ssh helper (`reference_signoz_clickhouse_via_ssh`), the Haiku looker step for screenshots, the report format (§ C6), what to do on FAIL (write an issue body file, rule 7), a "do not" list (no file edits outside `results/`, no lab tools, no fixes), one grading section per row naming the exact log event or DB query that proves it, then `## DRIVER REPORT`.

### C4. Fixtures and setup

- A row that starts after Cellblock uses a fixture: `server_stage_fixture` with `world`, `position`, `missions` and `items` (fields in `crates/lab/src/uat/spec_fixture.rs`; at most 10 items). Characters are created as `Uat Fx<key><n>` on the GM account.
- Fixtures have no level, cash or ability fields. Setup uses `.givexp <n>`, `/gmgivecash <n>` and `/gmgiveability <id>` (G tier) and says so in the row.
- A fixture that cannot be set up makes the row BLOCKED, never FAIL.
- A brief never uses `.gotolocation` onto or next to a hostile NPC, unless the row first sends `.aggro off`, reads its `proximity aggro OFF` reply, and sends `.aggro on` at the row's end (`docs/commands.md`, `.aggro`); it never uses `client_move_to` in a hub.

### C5. Grades

PASS, PARTIAL, FAIL, BLOCKED, NOT-RUN, NEEDS_HUMAN, NATIVE_SHORTFALL (server correct, the action had to be sent by stock Lua). PASS needs server evidence (a log row, a DB read or a packet-tap row). A FAIL in an early world that blocks later rows makes those rows BLOCKED, not FAIL.

### C6. Result file

`results/<packet>.md` (the part id when the packet is split), CRLF, one table: `| Row | Grade | Tier (N1/N2/N3/G) | Server evidence (query or log event, time) | Screenshot | Issue |`, then `## Not run` (each clause and why), then `## Resume hint` (state a re-run would start from), then `## Recipe` (exact tool calls that worked, for SP-91/92).

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

Each packet below: **Agent** documentation-writer, **Host** windows, worktree `sp-<packet>` on branch `starter-uat-praxis/<packet>-briefs`, commit `docs(starter-uat-praxis): <packet> briefs`. It writes, for each drive packet it owns, `docs/analysis/starter-uat-praxis/briefs/<drive>.md` (§ C2) and `docs/analysis/starter-uat-praxis/verify/<drive>.md` (§ C3); a packet over the call cap is written as parts `<drive>-p<n>` (§ C2a), using the rows `rows.md` assigns. Rules § C7. Checks: CRLF; each brief ≤ 500 lines; each row in `rows.md` for its drive packets has a `### ROW` block in the brief and a grading section in the verify file (`Select-String` count equal).

### SP-01: Cellblock briefs (SP-DCB1, DCB2, DCB3)

- **Read first:** [castle-cellblock.toml](../../guides/uat-specs/castle-cellblock.toml) and [routes/castle-cellblock.toml](../../guides/uat-specs/routes/castle-cellblock.toml) (recorded routes: reuse `prisoner-to-med-station`), [first-session.toml](../../guides/uat-specs/first-session.toml), the cellblock-continuation checklist.
- **Character:** a fresh Praxis Commando per lane (`char_creation` 13). The cell door opens only after mission 638; the route file says how.
- **Gotchas to front-load:** #1667 (pistol), LMG effect 644 inert (expected, not a FAIL of the row unless the row is about it), dialog 60151 client syntax error (record, screenshot, continue).

### SP-02a: Castle briefs part 1 (SP-DCA1 to DCA4)

- **Read first:** [castle-continuation/uat/end-uat.md](../castle-continuation/uat/end-uat.md), [CS-01-walk.md](../castle-continuation/uat/CS-01-walk.md) (the paused walk: "walking was blocked by lab input"; copy its working parts).
- **Character:** fixture `world = Castle`, position in the packet's START line; never log a fresh client straight into Castle (#756): the fixture lands at character select, then Play.
- **Gotchas:** K20 (friendly NPCs give no kill credit), interior/exterior navmesh not connected (use GM teleport between them, tier G).

### SP-02b: Castle briefs part 2 (SP-DCA5 to DCA7)

Same reads, character rule and gotchas as SP-02a. FZ-U1 (mission 567) is in SP-DCA5: trace its chain from `db/resources/` and the castle ledger.

### SP-03a: Harset briefs part 1 (SP-DHA1, DHA2)

- **Read first:** [harset-continuation README](../harset-continuation/README.md), [placements.md](../harset-continuation/placements.md), [uat.md](../harset-continuation/uat.md), the O14-16/17/19 rows of [owner-decisions-2026-10-14.md](../four-zone-planning/owner-decisions-2026-10-14.md).
- **Character:** fixture `world = Harset` (57) or the interior world the row needs; SP-DHA2 needs item 3730 in the fixture `items` for the hand-in rows and **not** for the refusal row.
- **Gotchas:** Market has no navmesh (teleport, tier G); door pairs unplaced (a door row that cannot be reached is BLOCKED, not FAIL). Each brief's § 0 names the area its rows use (dispatch rule 4).

### SP-03b: Harset briefs part 2 (SP-DHA3, DHA4, DHA5)

- **Read first:** as SP-03a, plus the FZ-U2 row of [unified-uat.md](../../guides/unified-uat.md) and the FZ-U2a to FZ-U2h split in SP-00.
- **Character:** SP-DHA3 is one Praxis character carried through FZ-U2a, b and c in order. SP-DHA4 and SP-DHA5 start each sub-row from the fixture the SP-00 split names (§ C4), so a row never depends on another row's leftovers; trace each fixture's mission step ids (1664, 5226) from `db/resources/` and cite them.
- **Gotchas:** the D-PU6 door-control position is pinned during SP-DHA3: the brief asks the driver to type `.bug pin cmd door control` and record `.location` at the control. The two debrief lines are provisional text (D-FZ15): grade that a line appears, not its wording. "Not offered again" is option P0 (O18-02a); if the owner has since picked P3, a re-offer is the expected result. All three packets use the gate and the Command Center, so they run one after another on one instance (waves table).

### SP-04: Tollana briefs (SP-DTO1 to DTO4)

- **Read first:** [tollana-continuation README](../tollana-continuation/README.md), [T-03a-568-chain-spec.md](../tollana-continuation/evidence/T-03a-568-chain-spec.md), [phasing.md](../tollana-continuation/evidence/phasing.md), [tollana-568-spec.md](../../../.claude/agent-memory/mission-systems-advisor/tollana-568-spec.md) (the accept-on-close wart and one-shot chain gate) and [interact-dialog-routing-traps.md](../../../.claude/agent-memory/mission-systems-advisor/interact-dialog-routing-traps.md).
- **Character:** fixture `world = Tollana` (19). Phasing rows (PH-01a/b, mission 621) need two players: lanes `default` and `p2` together. That part is SP-DTO2-p1, a two-instance part; its header says so. It runs alone, after the other Tollana parts (waves table, rule 4).
- **Gotchas:** the zero-button accept-on-close wart on mission 568 (expected), the Decline button on dialog 2153 (owner decision pending; record what it does).

### SP-05: hand-off leg briefs (SP-DL1, DL2, DL3)

- **Read first:** [ring-transport-cellblock-castle README](../ring-transport-cellblock-castle/README.md) (no ring animation without the client patch: the leg grades the server-side transfer only), O14-16 (chain 1366, Castle→Harset crossing), the Harset and Tollana ledgers for the third leg.
- Each leg starts from a fixture at the end state of the previous world and checks: the transfer happens on the first press, the arrival point is on the navmesh (no fall), mission and item state survive the transfer, and a relog after arrival keeps it.

## Waves 2 to 5: lab drive packets (ASK-FIRST)

Every drive packet: **LAB: ASK-FIRST** (§ C1). Agent `lab-driver` (brief) then verifier (`general-purpose`, verify file). Host: windows (the lab machine). Lease owner `sp-<packet>`. Writes only `results/<packet>.md` and `results/issues/` through the verifier (§ C6). Depends on its brief being integrated (all seven wave-1 packets are merged into `sp-integration`) and on D-SP1 and D-SP2. A split packet's parts are separate drive packets named `<packet>-p<n>`.

| Wave | Order | Part (brief, verify) | Instance | Starts | Calls | Depends on |
|---|---|---|---|---|---|---|
| 2 | 1 | [SP-DCB1-p1](briefs/SP-DCB1-p1.md), [verify](verify/SP-DCB1-p1.md) | default | wave start | 16 | SP-01 |
| 2 | 2 | [SP-DCB1-p2](briefs/SP-DCB1-p2.md), [verify](verify/SP-DCB1-p2.md) | default | after verify of SP-DCB1-p1 | 19 | SP-01 |
| 2 | 3 | [SP-DCB1-p3](briefs/SP-DCB1-p3.md), [verify](verify/SP-DCB1-p3.md) | default | after verify of SP-DCB1-p2 | 20 | SP-01 |
| 2 | 4 | [SP-DCB1-p4](briefs/SP-DCB1-p4.md), [verify](verify/SP-DCB1-p4.md) | default | after verify of SP-DCB1-p3 | 18 | SP-01 |
| 2 | 5 | [SP-DCB1-p5](briefs/SP-DCB1-p5.md), [verify](verify/SP-DCB1-p5.md) | default | after verify of SP-DCB1-p4 | 21 | SP-01 |
| 2 | 6 | [SP-DCB1-p6](briefs/SP-DCB1-p6.md), [verify](verify/SP-DCB1-p6.md) | default | after verify of SP-DCB1-p5 | 22 | SP-01 |
| 2 | 7 | [SP-DCB1-p7](briefs/SP-DCB1-p7.md), [verify](verify/SP-DCB1-p7.md) | default | after verify of SP-DCB1-p6 | 22 | SP-01 |
| 2 | 8 | [SP-DCB1-p8](briefs/SP-DCB1-p8.md), [verify](verify/SP-DCB1-p8.md) | default | after verify of SP-DCB1-p7 | 22 | SP-01 |
| 2 | 9 | [SP-DCB1-p9](briefs/SP-DCB1-p9.md), [verify](verify/SP-DCB1-p9.md) | default | after verify of SP-DCB1-p8 | 19 | SP-01 |
| 2 | 10 | [SP-DCB1-p10](briefs/SP-DCB1-p10.md), [verify](verify/SP-DCB1-p10.md) | default | after verify of SP-DCB1-p9 | 20 | SP-01 |
| 2 | 11 | [SP-DCB1-p11](briefs/SP-DCB1-p11.md), [verify](verify/SP-DCB1-p11.md) | default | after verify of SP-DCB1-p10 | 20 | SP-01 |
| 2 | 12 | [SP-DCB2-p1](briefs/SP-DCB2-p1.md), [verify](verify/SP-DCB2-p1.md) | default | after lease of SP-DCB1-p11 | 29 | SP-01 |
| 2 | 13 | [SP-DCB2-p2](briefs/SP-DCB2-p2.md), [verify](verify/SP-DCB2-p2.md) | default | after verify of SP-DCB2-p1 | 29 | SP-01 |
| 2 | 14 | [SP-DCB2-p3](briefs/SP-DCB2-p3.md), [verify](verify/SP-DCB2-p3.md) | default | after verify of SP-DCB2-p2 | 22 | SP-01 |
| 2 | 15 | [SP-DCB2-p4](briefs/SP-DCB2-p4.md), [verify](verify/SP-DCB2-p4.md) | default | after verify of SP-DCB2-p3 | 19 | SP-01 |
| 2 | 16 | [SP-DCB2-p5](briefs/SP-DCB2-p5.md), [verify](verify/SP-DCB2-p5.md) | default | after verify of SP-DCB2-p4 | 24 | SP-01 |
| 2 | 17 | [SP-DCB2-p6](briefs/SP-DCB2-p6.md), [verify](verify/SP-DCB2-p6.md) | default | after verify of SP-DCB2-p5 | 17 | SP-01 |
| 2 | 18 | [SP-DCB2-p7](briefs/SP-DCB2-p7.md), [verify](verify/SP-DCB2-p7.md) | default | after verify of SP-DCB2-p6 | 17 | SP-01 |
| 2 | 19 | [SP-DCB3-p1](briefs/SP-DCB3-p1.md), [verify](verify/SP-DCB3-p1.md) | default | after lease of SP-DCB2-p7 | 19 | SP-01 |
| 2 | 20 | [SP-DCB3-p2](briefs/SP-DCB3-p2.md), [verify](verify/SP-DCB3-p2.md) | default | after verify of SP-DCB3-p1 | 19 | SP-01 |
| 2 | 21 | [SP-DCB3-p3](briefs/SP-DCB3-p3.md), [verify](verify/SP-DCB3-p3.md) | default | after verify of SP-DCB3-p2 | 21 | SP-01 |
| 2 | 22 | [SP-DCB3-p4](briefs/SP-DCB3-p4.md), [verify](verify/SP-DCB3-p4.md) | default | after verify of SP-DCB3-p3 | 18 | SP-01 |
| 2 | 23 | [SP-DCB3-p5](briefs/SP-DCB3-p5.md), [verify](verify/SP-DCB3-p5.md) | default | after verify of SP-DCB3-p4 | 18 | SP-01 |
| 2 | 24 | [SP-DCB3-p6](briefs/SP-DCB3-p6.md), [verify](verify/SP-DCB3-p6.md) | default | after verify of SP-DCB3-p5 | 21 | SP-01 |
| 2 | 25 | [SP-DCB3-p7](briefs/SP-DCB3-p7.md), [verify](verify/SP-DCB3-p7.md) | default | after verify of SP-DCB3-p6 | 16 | SP-01 |
| 2 | 26 | [SP-DCB3-p8](briefs/SP-DCB3-p8.md), [verify](verify/SP-DCB3-p8.md) | default | after verify of SP-DCB3-p7 | 22 | SP-01 |
| 2 | 27 | [SP-DCB3-p9](briefs/SP-DCB3-p9.md), [verify](verify/SP-DCB3-p9.md) | default | after verify of SP-DCB3-p8 | 14 | SP-01 |
| 2 | 28 | [SP-DCB3-p10](briefs/SP-DCB3-p10.md), [verify](verify/SP-DCB3-p10.md) | default | after verify of SP-DCB3-p9 | 18 | SP-01 |
| 2 | 29 | [SP-DCB3-p11](briefs/SP-DCB3-p11.md), [verify](verify/SP-DCB3-p11.md) | default | after verify of SP-DCB3-p10 | 13 | SP-01 |
| 2 | 30 | [SP-DCB3-p12](briefs/SP-DCB3-p12.md), [verify](verify/SP-DCB3-p12.md) | default | after verify of SP-DCB3-p11 | 22 | SP-01 |
| 2 | 31 | [SP-DCB3-p13](briefs/SP-DCB3-p13.md), [verify](verify/SP-DCB3-p13.md) | default | after verify of SP-DCB3-p12 | 22 | SP-01 |
| 2 | 32 | [SP-DCB4-p1](briefs/SP-DCB4-p1.md), [verify](verify/SP-DCB4-p1.md) | default | after lease of SP-DCB3-p13 | 20 | SP-01 |
| 2 | 33 | [SP-DCB4-p2](briefs/SP-DCB4-p2.md), [verify](verify/SP-DCB4-p2.md) | default | after verify of SP-DCB4-p1 | 22 | SP-01 |
| 2 | 34 | [SP-DCB4-p3](briefs/SP-DCB4-p3.md), [verify](verify/SP-DCB4-p3.md) | default | after verify of SP-DCB4-p2 | 20 | SP-01 |
| 2 | 35 | [SP-DCB4-p4](briefs/SP-DCB4-p4.md), [verify](verify/SP-DCB4-p4.md) | default | after verify of SP-DCB4-p3 | 17 | SP-01 |
| 2 | 36 | [SP-DCB4-p5](briefs/SP-DCB4-p5.md), [verify](verify/SP-DCB4-p5.md) | default | after verify of SP-DCB4-p4 | 16 | SP-01 |
| 2 | 37 | [SP-DCB4-p6](briefs/SP-DCB4-p6.md), [verify](verify/SP-DCB4-p6.md) | default | after verify of SP-DCB4-p5 | 20 | SP-01 |
| 2 | 38 | [SP-DCB4-p7](briefs/SP-DCB4-p7.md), [verify](verify/SP-DCB4-p7.md) | default | after verify of SP-DCB4-p6 | 14 | SP-01 |
| 2 | 39 | [SP-DCB4-p8](briefs/SP-DCB4-p8.md), [verify](verify/SP-DCB4-p8.md) | default | after verify of SP-DCB4-p7 | 13 | SP-01 |
| 2 | 40 | [SP-DCB5-p1](briefs/SP-DCB5-p1.md), [verify](verify/SP-DCB5-p1.md) | default | after lease of SP-DCB4-p8 | 18 | SP-01 |
| 2 | 41 | [SP-DCB5-p2](briefs/SP-DCB5-p2.md), [verify](verify/SP-DCB5-p2.md) | default | after verify of SP-DCB5-p1 | 18 | SP-01 |
| 2 | 42 | [SP-DCB5-p3](briefs/SP-DCB5-p3.md), [verify](verify/SP-DCB5-p3.md) | default | after verify of SP-DCB5-p2 | 14 | SP-01 |
| 2 | 43 | [SP-DCB5-p4](briefs/SP-DCB5-p4.md), [verify](verify/SP-DCB5-p4.md) | default | after verify of SP-DCB5-p3 | 19 | SP-01 |
| 2 | 44 | [SP-DCB5-p5](briefs/SP-DCB5-p5.md), [verify](verify/SP-DCB5-p5.md) | default | after verify of SP-DCB5-p4 | 15 | SP-01 |
| 2 | 45 | [SP-DCB5-p6](briefs/SP-DCB5-p6.md), [verify](verify/SP-DCB5-p6.md) | default | after verify of SP-DCB5-p5 | 15 | SP-01 |
| 2 | 46 | [SP-DCB5-p7](briefs/SP-DCB5-p7.md), [verify](verify/SP-DCB5-p7.md) | default | after verify of SP-DCB5-p6 | 18 | SP-01 |
| 2 | 47 | [SP-DCB5-p8](briefs/SP-DCB5-p8.md), [verify](verify/SP-DCB5-p8.md) | default | after verify of SP-DCB5-p7 | 18 | SP-01 |
| 2 | 48 | [SP-DCB5-p9](briefs/SP-DCB5-p9.md), [verify](verify/SP-DCB5-p9.md) | default | after verify of SP-DCB5-p8 | 18 | SP-01 |
| 2 | 49 | [SP-DCB6-p1](briefs/SP-DCB6-p1.md), [verify](verify/SP-DCB6-p1.md) | default | after lease of SP-DCB5-p9 | 20 | SP-01 |
| 2 | 50 | [SP-DCB6-p2](briefs/SP-DCB6-p2.md), [verify](verify/SP-DCB6-p2.md) | default | after verify of SP-DCB6-p1 | 22 | SP-01 |
| 2 | 51 | [SP-DCB6-p3](briefs/SP-DCB6-p3.md), [verify](verify/SP-DCB6-p3.md) | default | after verify of SP-DCB6-p2 | 15 | SP-01 |
| 2 | 52 | [SP-DCB7-p1](briefs/SP-DCB7-p1.md), [verify](verify/SP-DCB7-p1.md) | default | after lease of SP-DCB6-p3 | 19 | SP-01 |
| 2 | 53 | [SP-DCB7-p2](briefs/SP-DCB7-p2.md), [verify](verify/SP-DCB7-p2.md) | default | after verify of SP-DCB7-p1 | 21 | SP-01 |
| 2 | 54 | [SP-DCB7-p3](briefs/SP-DCB7-p3.md), [verify](verify/SP-DCB7-p3.md) | default | after verify of SP-DCB7-p2 | 20 | SP-01 |
| 2 | 55 | [SP-DCB7-p4](briefs/SP-DCB7-p4.md), [verify](verify/SP-DCB7-p4.md) | default | after verify of SP-DCB7-p3 | 13 | SP-01 |
| 2 | 56 | [SP-DCB7-p5](briefs/SP-DCB7-p5.md), [verify](verify/SP-DCB7-p5.md) | default | after verify of SP-DCB7-p4 | 21 | SP-01 |
| 2 | 57 | [SP-DCB7-p6](briefs/SP-DCB7-p6.md), [verify](verify/SP-DCB7-p6.md) | default | after verify of SP-DCB7-p5 | 21 | SP-01 |
| 2 | 58 | [SP-DCB7-p7](briefs/SP-DCB7-p7.md), [verify](verify/SP-DCB7-p7.md) | default | after verify of SP-DCB7-p6 | 21 | SP-01 |
| 2 | 59 | [SP-DCB7-p8](briefs/SP-DCB7-p8.md), [verify](verify/SP-DCB7-p8.md) | default | after verify of SP-DCB7-p7 | 21 | SP-01 |
| 2 | 60 | [SP-DCB7-p9](briefs/SP-DCB7-p9.md), [verify](verify/SP-DCB7-p9.md) | default | after verify of SP-DCB7-p8 | 21 | SP-01 |
| 2 | 61 | [SP-DL1](briefs/SP-DL1.md), [verify](verify/SP-DL1.md) | default | after verify of SP-DCB1-p11 and after lease of SP-DCB7-p9 | 22 | SP-05 |
| 2 | 62 | [SP-DCA1-p1](briefs/SP-DCA1-p1.md), [verify](verify/SP-DCA1-p1.md) | p2 | after lease of SP-DL1 | 17 | SP-02a |
| 2 | 63 | [SP-DCA1-p2](briefs/SP-DCA1-p2.md), [verify](verify/SP-DCA1-p2.md) | p2 | after verify of SP-DCA1-p1 | 16 | SP-02a |
| 2 | 64 | [SP-DCA2-p1](briefs/SP-DCA2-p1.md), [verify](verify/SP-DCA2-p1.md) | p3 | after lease of SP-DCA1-p2 | 19 | SP-02a |
| 2 | 65 | [SP-DCA2-p2](briefs/SP-DCA2-p2.md), [verify](verify/SP-DCA2-p2.md) | p3 | after verify of SP-DCA2-p1 | 22 | SP-02a |
| 3 | 1 | [SP-DCA3-p1](briefs/SP-DCA3-p1.md), [verify](verify/SP-DCA3-p1.md) | default | wave start | 22 | SP-02a |
| 3 | 2 | [SP-DCA3-p2](briefs/SP-DCA3-p2.md), [verify](verify/SP-DCA3-p2.md) | default | after verify of SP-DCA3-p1 | 10 | SP-02a |
| 3 | 3 | [SP-DCA3-p3](briefs/SP-DCA3-p3.md), [verify](verify/SP-DCA3-p3.md) | default | after verify of SP-DCA3-p2 | 21 | SP-02a |
| 3 | 4 | [SP-DCA4-p1](briefs/SP-DCA4-p1.md), [verify](verify/SP-DCA4-p1.md) | p2 | after lease of SP-DCA3-p3 | 18 | SP-02a |
| 3 | 5 | [SP-DCA4-p2](briefs/SP-DCA4-p2.md), [verify](verify/SP-DCA4-p2.md) | p2 | after verify of SP-DCA4-p1 | 16 | SP-02a |
| 3 | 6 | [SP-DCA4-p3](briefs/SP-DCA4-p3.md), [verify](verify/SP-DCA4-p3.md) | p2 | after verify of SP-DCA4-p2 | 16 | SP-02a |
| 3 | 7 | [SP-DCA5-p1](briefs/SP-DCA5-p1.md), [verify](verify/SP-DCA5-p1.md) | p3 | after verify of SP-DCA4-p3 (same tank 13802 and Infirmary) | 17 | SP-02b |
| 3 | 8 | [SP-DCA5-p2](briefs/SP-DCA5-p2.md), [verify](verify/SP-DCA5-p2.md) | p3 | after verify of SP-DCA5-p1 | 18 | SP-02b |
| 3 | 9 | [SP-DCA5-p3](briefs/SP-DCA5-p3.md), [verify](verify/SP-DCA5-p3.md) | p3 | after verify of SP-DCA5-p2 | 17 | SP-02b |
| 3 | 10 | [SP-DCA5-p4](briefs/SP-DCA5-p4.md), [verify](verify/SP-DCA5-p4.md) | p3 | after verify of SP-DCA5-p3 | 13 | SP-02b |
| 3 | 11 | [SP-DCA6-p1](briefs/SP-DCA6-p1.md), [verify](verify/SP-DCA6-p1.md) | default | after verify of SP-DCA3-p3 and after lease of SP-DCA5-p4 | 17 | SP-02b |
| 3 | 12 | [SP-DCA6-p2](briefs/SP-DCA6-p2.md), [verify](verify/SP-DCA6-p2.md) | default | after verify of SP-DCA6-p1 | 20 | SP-02b |
| 3 | 13 | [SP-DCA7-p1](briefs/SP-DCA7-p1.md), [verify](verify/SP-DCA7-p1.md) | p2 | after verify of SP-DCA5-p4 (same Infirmary) | 20 | SP-02b |
| 3 | 14 | [SP-DCA7-p2](briefs/SP-DCA7-p2.md), [verify](verify/SP-DCA7-p2.md) | p2 | after verify of SP-DCA7-p1 | 12 | SP-02b |
| 3 | 15 | [SP-DCA7-p3](briefs/SP-DCA7-p3.md), [verify](verify/SP-DCA7-p3.md) | p2 | after verify of SP-DCA7-p2 | 14 | SP-02b |
| 3 | 16 | [SP-DL2](briefs/SP-DL2.md), [verify](verify/SP-DL2.md) | p3 | after lease of SP-DCA7-p3 | 19 | SP-05 |
| 4 | 1 | [SP-DHA1-p1](briefs/SP-DHA1-p1.md), [verify](verify/SP-DHA1-p1.md) | default | wave start | 16 | SP-03a |
| 4 | 2 | [SP-DHA1-p2](briefs/SP-DHA1-p2.md), [verify](verify/SP-DHA1-p2.md) | default | after verify of SP-DHA1-p1 | 19 | SP-03a |
| 4 | 3 | [SP-DHA1-p3](briefs/SP-DHA1-p3.md), [verify](verify/SP-DHA1-p3.md) | default | after verify of SP-DHA1-p2 | 20 | SP-03a |
| 4 | 4 | [SP-DHA1-p4](briefs/SP-DHA1-p4.md), [verify](verify/SP-DHA1-p4.md) | default | after verify of SP-DHA1-p3 | 21 | SP-03a |
| 4 | 5 | [SP-DHA1-p5](briefs/SP-DHA1-p5.md), [verify](verify/SP-DHA1-p5.md) | default | after verify of SP-DHA1-p4 | 17 | SP-03a |
| 4 | 6 | [SP-DHA1-p6](briefs/SP-DHA1-p6.md), [verify](verify/SP-DHA1-p6.md) | default | after verify of SP-DHA1-p5 | 18 | SP-03a |
| 4 | 7 | [SP-DHA1-p7](briefs/SP-DHA1-p7.md), [verify](verify/SP-DHA1-p7.md) | default | after verify of SP-DHA1-p6 | 15 | SP-03a |
| 4 | 8 | [SP-DHA1-p8](briefs/SP-DHA1-p8.md), [verify](verify/SP-DHA1-p8.md) | default | after verify of SP-DHA1-p7 | 20 | SP-03a |
| 4 | 9 | [SP-DHA2-p1](briefs/SP-DHA2-p1.md), [verify](verify/SP-DHA2-p1.md) | p2 | after lease of SP-DHA1-p8 | 18 | SP-03a |
| 4 | 10 | [SP-DHA2-p2](briefs/SP-DHA2-p2.md), [verify](verify/SP-DHA2-p2.md) | p2 | after verify of SP-DHA2-p1 | 19 | SP-03a |
| 4 | 11 | [SP-DHA2-p3](briefs/SP-DHA2-p3.md), [verify](verify/SP-DHA2-p3.md) | p2 | after verify of SP-DHA2-p2 | 10 | SP-03a |
| 4 | 12 | [SP-DHA2-p4](briefs/SP-DHA2-p4.md), [verify](verify/SP-DHA2-p4.md) | p2 | after verify of SP-DHA2-p3 | 14 | SP-03a |
| 4 | 13 | [SP-DHA3-p1](briefs/SP-DHA3-p1.md), [verify](verify/SP-DHA3-p1.md) | p3 | after lease of SP-DHA2-p4 | 17 | SP-03b |
| 4 | 14 | [SP-DHA3-p2](briefs/SP-DHA3-p2.md), [verify](verify/SP-DHA3-p2.md) | p3 | after verify of SP-DHA3-p1 | 20 | SP-03b |
| 4 | 15 | [SP-DHA3-p3](briefs/SP-DHA3-p3.md), [verify](verify/SP-DHA3-p3.md) | p3 | after verify of SP-DHA3-p2 | 22 | SP-03b |
| 4 | 16 | [SP-DHA4-p1](briefs/SP-DHA4-p1.md), [verify](verify/SP-DHA4-p1.md) | p3 | after lease of SP-DHA3-p3 | 21 | SP-03b |
| 4 | 17 | [SP-DHA4-p2](briefs/SP-DHA4-p2.md), [verify](verify/SP-DHA4-p2.md) | p3 | after verify of SP-DHA4-p1 | 18 | SP-03b |
| 4 | 18 | [SP-DHA4-p3](briefs/SP-DHA4-p3.md), [verify](verify/SP-DHA4-p3.md) | p3 | after verify of SP-DHA4-p2 | 11 | SP-03b |
| 4 | 19 | [SP-DHA4-p4](briefs/SP-DHA4-p4.md), [verify](verify/SP-DHA4-p4.md) | p3 | after verify of SP-DHA4-p3 | 15 | SP-03b |
| 4 | 20 | [SP-DHA5-p1](briefs/SP-DHA5-p1.md), [verify](verify/SP-DHA5-p1.md) | p3 | after lease of SP-DHA4-p4 | 19 | SP-03b |
| 4 | 21 | [SP-DHA5-p2](briefs/SP-DHA5-p2.md), [verify](verify/SP-DHA5-p2.md) | p3 | after verify of SP-DHA5-p1 | 22 | SP-03b |
| 4 | 22 | [SP-DHA5-p3](briefs/SP-DHA5-p3.md), [verify](verify/SP-DHA5-p3.md) | p3 | after verify of SP-DHA5-p2 | 21 | SP-03b |
| 4 | 23 | [SP-DL3-p1](briefs/SP-DL3-p1.md), [verify](verify/SP-DL3-p1.md) | default | after verify of SP-DHA1-p8 and after lease of SP-DHA5-p3 | 10 | SP-05 |
| 4 | 24 | [SP-DL3-p2](briefs/SP-DL3-p2.md), [verify](verify/SP-DL3-p2.md) | default | after verify of SP-DL3-p1 | 19 | SP-05 |
| 5 | 1 | [SP-DTO1-p1](briefs/SP-DTO1-p1.md), [verify](verify/SP-DTO1-p1.md) | default | wave start | 17 | SP-04 |
| 5 | 2 | [SP-DTO1-p2](briefs/SP-DTO1-p2.md), [verify](verify/SP-DTO1-p2.md) | default | after verify of SP-DTO1-p1 | 21 | SP-04 |
| 5 | 3 | [SP-DTO2-p2](briefs/SP-DTO2-p2.md), [verify](verify/SP-DTO2-p2.md) | default | after verify of SP-DTO1-p2 | 11 | SP-04 |
| 5 | 4 | [SP-DTO3-p1](briefs/SP-DTO3-p1.md), [verify](verify/SP-DTO3-p1.md) | default | after verify of SP-DTO2-p2 | 22 | SP-04 |
| 5 | 5 | [SP-DTO3-p2](briefs/SP-DTO3-p2.md), [verify](verify/SP-DTO3-p2.md) | default | after verify of SP-DTO3-p1 | 19 | SP-04 |
| 5 | 6 | [SP-DTO3-p3](briefs/SP-DTO3-p3.md), [verify](verify/SP-DTO3-p3.md) | default | after verify of SP-DTO3-p2 | 19 | SP-04 |
| 5 | 7 | [SP-DTO3-p4](briefs/SP-DTO3-p4.md), [verify](verify/SP-DTO3-p4.md) | default | after verify of SP-DTO3-p3 | 17 | SP-04 |
| 5 | 8 | [SP-DTO4-p1](briefs/SP-DTO4-p1.md), [verify](verify/SP-DTO4-p1.md) | default | after verify of SP-DTO3-p4 | 14 | SP-04 |
| 5 | 9 | [SP-DTO4-p2](briefs/SP-DTO4-p2.md), [verify](verify/SP-DTO4-p2.md) | default | after verify of SP-DTO4-p1 | 22 | SP-04 |
| 5 | 10 | [SP-DTO4-p3](briefs/SP-DTO4-p3.md), [verify](verify/SP-DTO4-p3.md) | default | after verify of SP-DTO4-p2 | 17 | SP-04 |
| 5 | 11 | [SP-DTO2-p1](briefs/SP-DTO2-p1.md), [verify](verify/SP-DTO2-p1.md) | default + p2 | after verify of SP-DTO4-p3; the only concurrent run (lanes `default` + `p2`), no other client open | 19 | SP-04 |
| 2-5 | last | attempt-4 takeovers | the failed part's instance | after the wave's last part | | the three Haiku attempts |

One sequential lane (rule 4): the table order is the run order, and the "Starts" column is each part's start condition. The table lists the 116 parts that have a brief and a verify file on disk. At about 12 minutes a part (hub-pass figure) plus attempt retries that is wave 2 65 parts, about 780 minutes, wave 3 16 parts, about 192 minutes, wave 4 24 parts, about 288 minutes, wave 5 11 parts, about 132 minutes. Tollana is one small hub, so wave 5 runs one part at a time on `default`, and the two-client part SP-DTO2-p1 runs last with no other client open. Wave 3's tank (spawn 13802) and Infirmary parts wait on each other's verifier.

**No brief yet:** SP-DCA8 to SP-DCA24 (81 end-uat rows, `rows.md` § Split changes) have no brief or verify file, so they are in no wave. That is documentation debt for a follow-up brief packet; until then those rows stay `NOT-RUN`.

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
- **Input:** `results/SP-DHA*.md`, `results/SP-DTO*.md`, `results/SP-DL2.md`, `results/SP-DL3-p*.md` § Recipe, with the same PASS or NATIVE_SHORTFALL filter as SP-91.
- **Write:** new `docs/guides/uat-specs/harset.toml` (section id `harset`) and `docs/guides/uat-specs/tollana.toml` (section id `tollana`), in the SP-91 row shape and id prefix. The FZ-U2 sub-rows keep their letters (`SP-FZ-U2c`).
- **Checks, in this order:** the same two as SP-91, run from `sp-92`: `Set-Location <sp-92 worktree>; pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`, then the `git diff --name-only` check. Fix rounds ≤ 5.
- **Rules:** as SP-91.

### SP-98: issue roll-up and sign-off sheet

- **Agent:** documentation-writer. Worktree `sp-integration`. Commit `docs(starter-uat-praxis): SP-98 results summary`.
- **Write:** a `## Results summary` section at the top of README.md in the shape of [hub-pass README § Results summary](../hub-pass-uat/README.md#results-summary-final-2026-10-13) (grade counts, per-world sign-off table, issues filed, lessons) and `signoff.md` listing every NEEDS_HUMAN row with its screenshot path for the owner.

### SP-99: close-out

- **Agent:** documentation-writer. **Depends on:** SP-98 and the four-zone FZ-99 close-out (D-SP7). Worktree `sp-integration`. Commit `docs(starter-uat-praxis): SP-99 close-out`.
- **Write:** in each zone ledger (cellblock, castle, harset, tollana continuation READMEs) change only the UAT rows this campaign graded, citing `results/<packet>.md`. In `docs/guides/unified-uat.md` add a `## Tollana` section and update the Castle CellBlock, Castle and Harset rows with the grade and date. Then the status docs once: one line per world in `docs/project-status.md` and the matching `docs/gap-analysis*` rows (walk [doc-update-map.md](../../agents/doc-update-map.md); list the rows touched in the PR body). Rewrite RESUME.md to "closed". Rules § C7.
