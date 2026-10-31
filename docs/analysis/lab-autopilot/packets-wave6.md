# lab-autopilot wave 6: agents, docs, live UAT, close-out

> **Type:** Reference
> **Audience:** Packet-coders, documentation-writers and packet-reviewers of wave 6; the orchestrator for AX-64 and AX-65
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md) (rules R1-R10, checks), [contract](contract.md), [ledger](README.md), [wave 5](packets-wave5.md)

Wave 6 teaches the agents and the docs what waves 1-5 built, proves it on the live client, and closes the campaign:

- a blind `frame-describer` agent that says what a screenshot shows without knowing what the caller hoped for (AX-60);
- the `lab-driver` agent and the `lab-uat` skill report verdicts instead of retelling success (AX-61);
- the guide, the ADR and the client-telemetry docs (AX-62, AX-63);
- one live UAT that tries to make the tools lie (AX-64, needs the user's OK);
- the close-out (AX-65).

Every packet carries R1-R10 of [work-packets.md § Rules](work-packets.md#rules-every-packet-carries). Nothing here changes Rust code.

**Dispatch:**

1. AX-60, AX-61, AX-62 and AX-63 at once, each from `lax-w6` (created by `ops` from main once the wave 5 PR merged).
2. AX-64 after the wave 6 PR merged, after the O1 lab rebuild from that `main` ([work-packets.md § Ops chores](work-packets.md#ops-chores-not-packets)), **and** after the user said yes to the run (D-AX4). The orchestrator asks; no agent does.
3. AX-65 after AX-64 wrote its ledger (or after the user declined the run: AX-65 then records AX-64 as UATPending).

**Merge order:** AX-60, AX-61, AX-62, AX-63 into `lax-w6`, one PR from `lab-autopilot/wave6`. AX-64 ships its UAT ledger as its own docs PR. AX-65 is its own PR.

**What every doc packet here needs:**

- Docs are CRLF (R5); check D1 on every file written. Never hand-edit a `<!-- gen:NAME -->` block (none of the target files has one today; re-check).
- `docs/guides/live-research-lab.md` is 1849 lines. Do not grow it by more than the lines a packet allows; new material goes in a new sibling page.
- The tool names, JSON keys and why codes come from [contract.md](contract.md). Copy them exactly; never paraphrase a key.
- No IPs, account or character names, tokens or real machine paths (R7).
- Autopilot (ramenvr/autopilot, Apache 2.0) contributed **ideas only**: no code, text or test was copied from it. Docs name it as the inspiration where a concept came from it (the evidence gate, the stop report, the watch, the detectors, the blind describer), with one attribution line per page, never more.

### AX-60 The frame-describer agent

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-60-describer` · **Worktree:** `lax-60` (at `lax-w6`)
- **Commit:** `feat(agents): a blind frame-describer for lab screenshots (AX-60)`
- **Rules:** R1, R3, R5, R7. Decision D-AX3 (its model; default `haiku`, written below; the owner may change one line later).

Why: a coordinator that asks "did the vendor window open?" while looking at the screenshot itself sees what it expects. The describer gets only the image paths and neutral questions, never the request, and answers in a fixed shape.

Files and changes:

1. New `.claude/agents/frame-describer.md`, exactly this (LF, like the other agent files):

```markdown
---
name: frame-describer
description: "Describes lab screenshots blind: given image paths and neutral questions, reports what is visibly on screen in a fixed shape. Never told what the caller expects. Read-only."
model: haiku
omitClaudeMd: true
effort: low
maxTurns: 8
tools: Read
---

You describe screenshots of the Stargate Worlds client. You are given one to four image file paths, each with a line such as `client window 1280x720, frames 81230..81233`, and a numbered list of questions. You are never told what the caller hopes to see, and you must not guess it.

Rules:
- Read each image with the Read tool. Read nothing else: no code, no docs, no other files.
- Describe only what is visible. Never infer what happened before or after the frame, why something is on screen, or whether an action worked.
- Answer every question with `yes`, `no` or `cannot tell`, then one short sentence of what you see that decides it. `cannot tell` is the right answer when the window is cut off, covered, blurred, black or too small to read; never stretch.
- Quote on-screen text exactly as written, in double quotes, including capitalisation. Text you cannot read in full is `"<partly legible: ...>"` with the part you can read.
- Name windows by their title text or their visible role ("a window titled \"Vendor\"", "a chat box"), never by an internal name you have not been given.
- A black, white or single-colour image: say so and answer every question `cannot tell`.

Report, nothing else:

frames: <the frames line of each image, in order>
1. <yes|no|cannot tell> - <what you see>
2. ...
visible text: <up to 10 exact quotes, most prominent first>
other windows: <titles of other open windows, or none>
```

2. `docs/agents/development-workflow.md`: one roster row after the `lab-driver` row: `| Describing lab screenshots blind, from image paths and neutral questions (Haiku) | \`frame-describer\` |`. No row in `.github/skills/code-review/SKILL.md`: that table routes diff reviewers, and `lab-driver` has none either.
3. Run A1 (`python tools/agent-skills/sync.py`); commit the regenerated `.codex/agents/frame-describer.toml` with the rest. Do not edit the `.codex` file by hand.

Tests: none (no code). Checks: A1, D1 on `development-workflow.md`. The reviewer checks the frontmatter keys match the other agents' keys (`name`, `description`, `model`, `omitClaudeMd`, `effort`, `maxTurns`, `tools`) and that `tools` is `Read` only.

### AX-61 lab-driver and lab-uat read verdicts

- **Host:** windows · **Agent:** packet-coder · **Branch:** `lab-autopilot/AX-61-driver` · **Worktree:** `lax-61` (at `lax-w6`)
- **Status:** Step 1 Ready. Step 2 BlockedDecision D-AX10 (the driver's tool list); ship step 1 alone if D-AX10 is not answered when the wave PR opens, and record step 2 as a follow-up in the report.
- **Commit:** `feat(agents): the lab driver reports verdicts and the lab-uat skill grades them (AX-61)`
- **Rules:** R1, R3, R5, R7. Contract: C1 (`evidence.verdict`), C2 (why codes), C3 (gate), C8 (stop report), C9 (cooldown).

The keys the agent and skill must name, copied from the contract: every judged tool's result carries `evidence: { verdict: "confirmed" | "denied" | "inconclusive", why?, checks: [...] }`; the why codes the driver must recognise are `already_true`, `stale_read`, `cooling_down` and `posted_only` (all of C2 is in [contract.md § C2](contract.md#c2-why-codes)); a stop carries `stop: { kind, ... }` (C8). Re-read C1-C3 and C8-C9 and use their exact spelling.

Files and changes:

1. **Step 1** `.claude/agents/lab-driver.md`: add a block `Verdicts:` after the `Rules:` list (at most 10 lines), saying:
   - Report every result's `evidence.verdict` and `evidence.why` verbatim on the step's line, before any other value. A step whose tool returned no `evidence` is reported `unjudged`.
   - `confirmed` is the only verdict that means the step did what it said. Never report `inconclusive` or `denied` as done.
   - `why: cooling_down`: do not retry the step; report it and go on (the daemon refused a repeat of a failing call).
   - `why: posted_only`: the input was sent but nothing proved its effect; report it as `posted_only`, not as a pass.
   - `why: already_true`: report it; it is not a pass of an action (the state was there before).
   - A `stop` in the result: report `stop.kind` and its message verbatim.
   - Clicks that open something: pass `expect_visible` with the window the brief names (AX-40), so the click judges itself.
   - The "retry it at most once" rule does not apply to `cooling_down`.
2. **Step 1** `.claude/skills/lab-uat/SKILL.md` (97 lines; at most 30 added):
   - § 4 (brief a lab-driver): one bullet that the brief names `expect_visible` for each click that opens a window, and asks for verdicts verbatim.
   - § 5 (verify): a short block on the verdicts: a `confirmed` is still checked against the server log for the rows D-AX11 covers (`move_to`, non-placeholder `minigame_play`); an `inconclusive` row is graded per D-AX1 (`UNVERIFIED` unless a required clause passed); a screenshot that decides a row goes to a `frame-describer` (AX-60) with neutral questions, never the coordinator's own reading. Two short examples of a neutral question ("Is a window titled \"Vendor\" open?") and a leading one to avoid ("Did the vendor open correctly?").
   - § 6 (grade): `lab_uat_run` rows with `strict` (AX-44) carry their verdicts; a row is PASS only on `confirmed`.
   - Known lab tool traps: two lines: `lab_detectors_read` after a run names hitches and stuck input that explain a failure; `client_move_to` stops with `stop.kind: "ledge"` before drops unless `allow_drops`.
3. **Step 2, only when D-AX10 is answered yes:** remove from `disallowedTools`: `mcp__cimmeria-lab__lab_crash_report`, `mcp__cimmeria-lab__lab_timeline`. The new tools `lab_detectors_read`, `client_ui_clickables`, `lab_recover` and `client_watch` are not in the list today, so they are already allowed; with D-AX10 **no**, add those four to `disallowedTools` instead (the same `mcp__cimmeria-lab__` prefix). Either way, add `mcp__cimmeria-lab__client_trace_line` to `disallowedTools` (raw engine calls stay with the coordinator, like `client_call_native`). Keep every other entry.
4. Run A1. Commit the regenerated mirrors (`.agents/skills/lab-uat/`, `.codex/agents/lab-driver.toml`).

Tests: none (no code). Checks: A1, D1. The reviewer checks: every why code named exists in C2; `client_input_key`, `client_ui_click`, `lab_client_start/stop/restart`, `client_mem_read`, `client_call_native` and `client_trace_line` are all in `disallowedTools`; `lab-driver.md` stays under 100 lines.

### AX-62 The guide: evidence, stops, watch, detectors

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `lab-autopilot/AX-62-guide` · **Worktree:** `lax-62` (at `lax-w6`)
- **Commit:** `docs(lab): how lab tools prove what they did (AX-62)`
- **Rules:** R1, R3, R5, R7. Contract: all of C1-C16 (the guide describes them; it never invents a key). Commit attribution: Sonnet.

Files and changes:

1. New `docs/guides/lab-evidence.md` (CRLF, at most 500 lines; header block Type How-to and Reference, Audience lab coordinators and lab-driver brief writers, Last updated, Companions linking the main guide and the ADR). Sections:
   1. **Verdicts.** C1 and C3: `confirmed`, `denied`, `inconclusive`, how `judge` decides, the gate, D-AX1's grading rule, D-AX13 (unjudged tools are legacy until AX-44's strict switch).
   2. **Why codes.** The C2 table, one line each, with what a coordinator does about each.
   3. **Server corroboration.** What it covers (D-AX11: `move_to` and non-placeholder `minigame_play`) and what it does not; say plainly that every other `confirmed` rests on client reads.
   4. **Stop reports and recovery.** C8 kinds (including `ledge`), C9 cooldown, C10 `resume_from`, C11 `lab_recover` recipes.
   5. **Clicks and items.** `expect_visible`, `client_ui_clickables` (C16), what a direct widget click is not (D-AX7).
   6. **Watching a run.** `client_watch` (C15, C6 fields), `lab_detectors_read` (C12 table and thresholds, D-AX6 provisional), what runs without a watch.
   7. **Frames.** C4 `meta`, freshness, screenshot frame brackets, the same-frame `client_batch` (C7), D-AX2 (capture stays PrintWindow).
   8. **Movement.** Key holds with a lease (C5, D-AX8), the coast lead and the settled arrival (AX-54), the ledge guard with its walk-start self-check and `client_trace_line` (C14: only flags `0x2086` and `0x20bf`; a fault latches the trace off until the client restarts), and the key fail-safe (C5: raw keys are DLL holds, a daemon drop or a lease end lets go of them, fallback mode is the gap).
   9. **The blind describer.** When to send a screenshot to `frame-describer`, neutral vs leading questions.
   10. One attribution line at the end: concepts adapted from ramenvr/autopilot (Apache 2.0); no code copied.
2. `docs/guides/live-research-lab.md` (at most 25 lines added in total):
   - In "The rulebook": one rule, `A tool's word is its evidence.verdict; read docs/guides/lab-evidence.md`, with the link.
   - In "Tools at a glance": one row each for `client_watch`, `lab_detectors_read`, `client_ui_clickables`, `lab_recover` and (if merged) `client_trace_line`, in the table's existing format, each linking its section of `lab-evidence.md`.
   - In "World tools": two lines on `allow_drops` and `ledge_guard` (if AX-56 merged) and on the coast lead.
3. `docs/readme.md` (or the docs index that lists guides): one row for `lab-evidence.md` if guides are listed there.
4. Walk `docs/agents/doc-update-map.md` row "Live research lab" and list in the report each target it names, with "updated" or "not affected, because ...".

Tests: none. Checks: D1. The reviewer greps every backticked key and tool name in `lab-evidence.md` against `contract.md` and `crates/lab/src` on the `lax-w6` head and lists any that do not exist.

### AX-63 ADR addendum and client-telemetry docs

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `lab-autopilot/AX-63-adr` · **Worktree:** `lax-63` (at `lax-w6`)
- **Commit:** `docs(lab): ADR addendum for the lab-autopilot bridge methods (AX-63)`
- **Rules:** R1, R3, R5, R7. Contract: C4-C7, C13, C14. Commit attribution: Sonnet.

Files and changes:

1. `docs/architecture/live-research-lab.md` (the ADR, 421 lines; at most 70 added): a new section `## Addendum 2026-11: lab-autopilot` (use the month the PR opens) with:
   - the new bridge methods (`heartbeat` meta, `hold_*`, `watch_*`, `batch`, and `trace_line` if AX-55 merged) and their one-line purpose;
   - the C13 safety proof as the ADR's rule for every future bridge method (all seven items, plus the AX-55 exception to item 6);
   - D-AX2 (frame capture stays supervisor-side; an in-DLL `Present` hook needs its own ADR), D-AX5 (no ProcessEvent hook; an FName allowlist if one is ever added), D-AX12 (no `InputKey` injection);
   - the attribution line (concepts from ramenvr/autopilot, Apache 2.0, no code copied).
2. `docs/architecture/client-telemetry.md`: the section that says the crate is emit-only except under `lab-bridge` gains the list of new lab-only methods and one sentence that none compiles without the feature and none is in the signed manifest or launcher path (directive 4). At most 25 lines.
3. `crates/client-telemetry/README.md`: the `lab-bridge` feature line lists the new methods (one line each, at most 10 lines).
4. `crates/README.md`: the `lab` row gains the new tool families in a few words (one edit to the existing row).
5. Check (and say so in the report): `git grep -n "autopilot" -- crates/` on the `lax-w6` head finds nothing but comments naming the concept, and no file under `crates/` carries an Apache header from autopilot. Any hit that looks copied: stop and report `blocked - copied code: <path>`.

Tests: none. Checks: D1 on every docs file; the step 5 grep output in the report.

### AX-64 Live UAT: try to make the tools lie

- **Host:** windows · **Agent:** a `lab-uat` coordinator (`hands-on`, Sonnet) with Haiku `lab-driver`s for the driving and one `frame-describer` · **Branch:** `lab-autopilot/AX-64-uat` · **Worktree:** `lax-64` (from main after the wave 6 PR merged)
- **Status:** BlockedDecision D-AX4 (the user's OK for this lab run). UATPending until it runs.
- **Commit:** `docs(lab): lab-autopilot live UAT ledger (AX-64)`
- **Rules:** R1, R3, R5, R7. Follow the `lab-uat` skill end to end (§ 0 ask is the orchestrator's; the coordinator confirms the lab is free, takes the lease, `lab_ensure_in_world`). The O1 rebuild (lab daemon and lab DLL from the merged `main`, `tools/lab/install.ps1`) ran before this packet; the first step confirms heartbeat, `input_status` and `module_info` show that build. Never a release client.

Each case has an **honest** half (the action can succeed) and a **lie bait** half (the action cannot succeed, so any `confirmed` is a lie). A case passes only if both halves get the verdict in the table. Run each case 3 times; record raw results.

| # | Case | Honest half: expect | Lie bait: expect |
|---|---|---|---|
| U1 | `client_world_click` on a vendor NPC at 4 m with `expect_visible` its window | `confirmed` | the same click at 12 m: `denied` or `inconclusive`, never `confirmed` |
| U2 | `client_ui_sequence` click on a visible button that closes its window | `confirmed` | click a window name hidden a moment before: not `confirmed`, a why code |
| U3 | `client_move_to` 10 m over open floor, `arrival_m` 0.5 | `confirmed`, server position within 1 m of the goal; after 1 s still within 0.5 m (coast lead) | goal behind a wall: a stop (`stop.kind` `stuck` or `stalled`), never `confirmed` |
| U4 | `client_move_to` toward a drop of more than 2 m, once indoors and once on an outdoor terrain map | with `allow_drops: true`: arrives, `ledge_guard: "off"`; also record what `client_trace_line` straight down on terrain returns (the open question: does terrain answer `0x2086`?) | default: `stop.kind: "ledge"` before the edge, or, where the trace cannot see the floor, `ledge_guard: "unavailable: no floor under pawn"` and no false `ledge` stop |
| U5 | `client_camera` a 30 deg yaw | `confirmed` | yaw while a full-screen window blocks input: not `confirmed` |
| U6 | `client_use_ability` on a hostile target in range | `confirmed` per D-AX14 (the press got an answer) and `result.verdict` a hit or miss | on a target out of range: `confirmed` is right here (D-AX14: the refusal is an answer), but `result.verdict` must name the refusal, never a hit |
| U7 | `client_watch start`, walk 10 s, `read`, then `lab_detectors_read` | samples with rising `i`, `pulled` > 0; no `teleport` | hold W into a wall for 5 s, then `lab_detectors_read`: `stuck_input` fires |
| U8 | `lab_screenshot` text names `frames a..b` with `b >= a`; one screenshot per case above goes to `frame-describer` with neutral questions | the describer's yes/no agrees with the verdict | a black or covered frame: the describer says `cannot tell` |
| U9 | a repeated failing call (U3's wall, 3 times fast) | - | the third answers `why: cooling_down` (C9) |
| U10 | `lab_recover {recipe: "close_dialogs"}` with an NPC dialog open (C11) | the dialog closes, the result is `confirmed` | - (record the result `close_dialogs` gives with no dialog open, for the campaign review) |
| U11 | Key fail-safe (C5): `client_input_key W down`, then restart the daemon (`tools/lab/daemon.ps1 restart`) | the avatar stops within 1 s of the drop (server position stops changing), `input_status` after reconnect shows no holds | release the lease with W down instead: the avatar stops, the daemon log has `lab.input.released_on_lease_end` with `how: released` |
| U12 | `lab_login` with an account file naming a server that is **not** the combo's first row (AX-03) | the result's `login_server` is that server and the server log shows the login there | an account file naming no combo row with two or more rows: `lab_login` fails naming the rows, and never logs in |
| U13 | Screensaver (AX-04): only if the lab host's screensaver is non-secure and the user agrees to start one | `lab_client_start` with a running `*.scr`: the log names it as dismissed, the launch proceeds | - (a secure screensaver: record the refusal text naming the `.scr` and pid; never unlock the session) |

Write `docs/analysis/lab-autopilot/uat-ax64.md` (CRLF): one table per case with the three runs, each verdict, why code and the server or describer corroboration, then a summary line per case `PASS | FAIL | NOT RUN` and **a list of every lie found** (a `confirmed` on a lie-bait half), each with the tool, arguments and the evidence it returned. Record the client and server build; no account or character names.

Checks: D1. Report: the summary lines and the lie list. Any lie found: the orchestrator files one issue per lie and the campaign review decides the fix.

### AX-65 Close-out

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `lab-autopilot/AX-65-closeout` · **Worktree:** `lax-65` (from main after AX-64's PR merged, or after the user declined the run)
- **Commit:** `docs(lab): lab-autopilot close-out (AX-65)`
- **Rules:** R1, R3, R5, R7. Status docs change once per campaign: this is that once. Commit attribution: Sonnet.

Files and changes:

1. `docs/project-status.md`: one dated update line in the existing style (the 2026-10-11 and 2026-10-12 `> **... update (lab)**` lines are the model): what the lab tools now prove, what still rests on client reads only, AX-64's result (or UATPending), and the link to this ledger.
2. `docs/analysis/lab-autopilot/README.md`: set every packet's status, fill "Close-out" with the merged PRs, AX-64's lie list (or "none found"), and the follow-ups (open decisions, any AX-02 or AX-64 question left open, the provisional D-AX6 threshold).
3. `docs/analysis/lab-autopilot/RESUME.md`: state "closed" with the follow-ups (at most 40 lines).
4. Project memory: update `.claude/agent-memory/game-archaeology-specialist/ue3-reflection-trace-anchors.md` only if AX-01, AX-02 or AX-64 found something not yet in it (the terrain-trace answer from U4, the player dead bit, the actor count); otherwise none; and add a one-line pointer to the guide in `.claude/agent-memory/` of whichever agent's `MEMORY.md` already indexes lab tooling (grep `lab_` in `.claude/agent-memory/*/MEMORY.md`). No IPs, accounts or names.
5. Do not touch `docs/gap-analysis*` unless a row there names a lab capability this campaign changed; if one does, update that row and say which.
6. `docs/guides/unified-uat.md`: add the lab-autopilot UAT the owner still has to run (AX-64's cases if it was declined or any case is NOT RUN), in that guide's existing format, linking `uat-ax64.md` or this packet's AX-64 section.

The orchestrator, not this packet, runs the campaign review loop before AX-65 (its fixes ship as one PR first), has `ops` retire every `lax-*` worktree, empties `.claude/state/active-campaign`, and posts the board handoff.

Tests: none. Checks: D1.
