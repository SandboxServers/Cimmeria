# harness-tuning: work packets, wave 2

> **Type:** Reference · **Audience:** The orchestrator, packet-coders and packet-reviewers of this campaign · **Updated:** 2026-11-01
> **Companions:** [Ledger](README.md), [dispatch rules, C1-C2 and wave 1](work-packets.md), [waves 3-5](packets-wave3.md), [resume card](RESUME.md)

The dispatch rules, the wave table, the R-PY and R-AGENT rule blocks and contracts C1 and C2 are in [work-packets.md](work-packets.md). Where a packet below says "R-PY", "R-AGENT" or "C1" without "(above)", read it there. This file holds contracts C3-C7 and the wave-2 packets HT-06..HT-11b.

## Contract (continued): C3-C7

### C3. `report_contract.py` (HT-06)

This is a SubagentStop hook. It reads these input keys: `stop_hook_active`, `agent_type`, `agent_id`, `agent_transcript_path`, `transcript_path`, `last_assistant_message` and `cwd`. The project root is `CLAUDE_PROJECT_DIR`, or else `cwd`.

1. If `stop_hook_active` is true, exit 0 with no output.
2. If `.claude/agents/<agent_type>.md` doesn't exist or doesn't contain `## Reporting`, exit 0. That covers ops, lab-driver, hands-on and the built-in agents.
3. **Mode.** `line` when the frontmatter `model:` value contains `haiku` (case-insensitive; today only packet-coder), else `report`.
4. Find the transcript: `agent_transcript_path`, or else `<transcript_path without .jsonl>/subagents/agent-<agent_id>.jsonl`. `final` is `last_assistant_message`, or else the last assistant text in the transcript. If there is neither a transcript nor `final`, exit 0.
5. **The cap.** If the transcript holds a cap deny, exit 0. The user-level cap has denied a tool, and the hook never asks for more (D-HT3). A cap deny is a `tool_result` block in the content list of a `type: "user"` record whose text (its `content` string, or its `text` blocks joined) matches `CAP_RE = ^(?:PreToolUse:\S+ hook error: )?Context cap reached` after `lstrip()`. That is the shape the user-level hook's deny takes in a transcript: `PreToolUse:Edit hook error: Context cap reached: 90,515 tokens > 90,000. STOP NOW. ...` with `is_error: true`. The marker anywhere else is not a cap: in the brief, in a Read or Grep result that quotes it, or in assistant text. `is_error` is not required, so a real deny is never missed (ruling HT-06-1).
6. **Line mode.** If a line of `final` matches LINE (strip one pair of backticks first), exit 0. Otherwise block with reason L. Line mode never reads the brief and never checks a file.
7. **Report mode.**
   - Take the brief (the first user text) and `paths = RP.findall(brief)`. If there is no transcript or no path, exit 0.
   - Pick the **candidate**: the last brief path whose file is missing, empty, or older than the run; if no path is like that, the last brief path. Resolve a relative path against `cwd`, then against the root. "Older than the run" means an mtime earlier than the transcript's first `timestamp` minus 5 s.
   - Check, in order, and block on the first failure:
     - **A**: no line of `final` matches LINE;
     - **B**: the line's report has a `report_suffix` that matches no brief path (a `-` matches none);
     - **C**: the matched brief path's file is missing, empty or older than the run.
8. Otherwise exit 0.

To block, print `{"decision": "block", "reason": <text>}` to stdout and exit 0. Any exception exits 0 with no output (fail open). The reasons, with `<path>` the candidate as the brief wrote it:

- L: `report_contract: end your reply with exactly one line: <id> <done|blocked|failed> <commit-sha|-> - (add reason=<word> when blocked or failed). Write no file and call no tool.`
- A: `report_contract: your final message has no return line. Write your report to <path> if you have not, then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. Call no other tool after that.`
- B: `report_contract: your return line names <got>, but the brief's report path is <path>. Write the report there and end with exactly one line naming it. Call no other tool after that.`
- C: `report_contract: <path> is missing, empty or older than this run. Write your full report there, then end with exactly one line: <id> <done|blocked|failed> <commit-sha|-> <path>. Call no other tool after that.`

`LINE` and `RP` are copied verbatim from C1. HT-09's test checks that the two copies match.

### C4. The Haiku coder contract (HT-07; D-HT3, D-HT14)

- **The line.** `<packet> <done|blocked|failed> <commit-sha|-> -`, then optional suffixes, each once, in this order:
  - `reason=<context|checks|spec|host|platform|tool>` on `blocked` or `failed`;
  - `at=<k>`, the packet check or step number where it stopped;
  - `not_run=<k>,...`, the packet checks (by number) it did not run. When it's absent, every check ran.

  The SHA is the branch's last commit, or `-` when the session committed nothing.
- **No report.** No report file, worknote or summary, even when a brief names a report path. The commits are the coder's record, and the reviewer's report is the packet's record.
- **Commits.** Commit at the end with the brief's message. A finished part may be committed earlier, with the subject suffix `(part N)`, but only after that part's checks pass; never commit a failing state. A deviation from the spec is a commit-body line `Deviation: <what> - <why>`. On a fix pass, a rejected note is a commit-body line `Rejected <note-id>: <reason>`. A fix pass that rejects every note commits with `--allow-empty`, so the reasons reach the reviewer.
- **The cap.** Past 90k, the user-level hook denies every tool with `Context cap reached`. Its text asks for a "final report". For a packet-coder, that report is exactly the line `<packet> blocked <sha|-> - reason=context`. Uncommitted edits stay in the worktree.
- **A continuation** (`continuation N of 3, base=<sha>` in the brief) runs `git -C <wt> log --oneline <sha>..HEAD`, `git -C <wt> status --porcelain` and `git -C <wt> diff --stat`, compares them with the spec's steps, and starts at the first step not done. It keeps uncommitted edits that match the spec, never redoes committed work, runs every check and commits.
- **The orchestrator** routes a coder's `blocked` or `failed` by `reason=`:
  - `context`, or no usable line at all → a continuation;
  - `checks` → the Sonnet implementer;
  - `spec` → `architect`;
  - `host`, `platform` or `tool` → the packet's host note.

### C5. Per-session resume card (HT-08)

- Input: `transcript_path` from stdin. When it is missing, or the file is, there's no card. The global pointer file's **contents are no longer read**.
- **Tier 1.** Take this transcript's last `Write`, `Edit` or `MultiEdit` whose `norm_path(file_path)` ends with `/.claude/state/active-campaign`. The text is `content` for Write and `new_string` for Edit; strip a BOM and take the first non-empty line. When that text is empty, there is **no card** and tier 2 is skipped. Otherwise apply today's name check (not `""`, `.` or `..`, no `/` or `\`) and require `docs/analysis/<name>/RESUME.md`; the card starts with today's `POINTER`.
- **Tier 2** applies only when the session never wrote the pointer. Take the last 40 `Agent` tool uses. For each, collect the names in `json.dumps(input).lower()` that match `docs[\\/]+analysis[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])` or `\.claude[\\/]+reports[\\/]+([a-z0-9][a-z0-9.-]*[a-z0-9])`, keeping only names that have a `RESUME.md`. That gives a set per prompt. `naming` is the number of prompts with at least one name, and `top` is the name in the most sets, with count `c`. A card is printed when `naming >= 3` and `3*c >= 2*naming`. It starts with `Context was compacted. Inferred campaign (from this session's dispatches): {name}. Read docs/analysis/{name}/RESUME.md and the ledger README before acting; if this is not your campaign, ignore this card.`
- Both tiers append the first 40 lines of RESUME.md, as today.
- **Reading.** Read only lines that contain `active-campaign` (tier 1) or `"Agent"` (tier 2). For a file over 200 MB, seek to its last 200 MB and drop the partial first line.

### C6. Not checked (HT-09)

- **Reviewer report.** A fourth section after `## Not fixed`: `## Not checked`, listing `- NOT ASSESSED — <what> — <reason>` lines, or `- none`. Every check in the coder's `not_run=` list that the reviewer didn't verify itself goes here.
- **Reviewer return line.** Ends with ` not_assessed=<n>` after `verdict=` (and after `verdicts=` at wave scope).
- **Rejected notes.** From pass 2, the reviewer reads the coder's reasons from the fix commit's body (`Rejected <note-id>: ...`), not from a report.
- **Orchestrator.** It passes the coder's last line into the reviewer's brief. On `verdict=ship` with `not_assessed=<n>` and n > 0, it opens the report's `## Not checked` section and copies its items into the packet's ledger row, or a worknote, before shipping.
- **Coders** write no report. Their only skipped-check signal is the `not_run=` suffix (C4), because the owner's rule leaves no other place (D-HT8).

### C7. ID prefixes (HT-10)

```python
DECISION_ROW = re.compile(r"^\|\s*\**(D-[A-Z]{1,4})\d+[a-z]?\b")
PACKET_ROW = re.compile(r"^\|\s*\**([A-Z]{2,4})-\d{2,3}[a-z]?\b")
```

A prefix "belongs" to every folder `docs/analysis/<c>/` whose top-level `*.md` files have a first table cell matching either pattern. A prefix that belongs to more than one folder fails unless it is in `GRANDFATHERED` (prefix → frozenset of folders) **and** its folders are a subset of that entry. `NOT_PACKETS` (a set) lists tokens that aren't packet IDs, such as `UTF`, `SHA` and `AES`. Bare `D1`..`D8` never match.

## Wave 2: harness changes

### HT-06: report-contract SubagentStop hook

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-06-report-contract` · worktree `ht-06`, from `main`
- **Files:**
  - new `.claude/hooks/report_contract.py` (C3; at most 250 lines; its module docstring's first line names the hook, which `test_harness_tables.py` requires);
  - `.claude/settings.json`;
  - new `tools/agent-skills/test_report_contract.py`;
  - `tools/agent-skills/test_hooks.py`, the `WiringTest` class only.
- **Contract:** C3 in full (above), with `LINE` and `RP` from C1.
- **Steps:**
  1. Write the hook to C3. Use the standard library only. `main()` returns 0, and `if __name__ == "__main__": sys.exit(main())`.
  2. In `settings.json`, add a top-level `"SubagentStop"` array under `"hooks"`, after `"PreToolUse"`:
     `[{"hooks": [{"type": "command", "command": "f=\"$CLAUDE_PROJECT_DIR/.claude/hooks/report_contract.py\"; [ -n \"$CLAUDE_PROJECT_DIR\" ] && [ -f \"$f\" ] || exit 0; set -- \"$f\"; . \"$CLAUDE_PROJECT_DIR/.claude/hooks/py.sh\"", "timeout": 10}]}]`.
     Keep the 2-space indent and the CRLF.
  3. `test_hooks.py` `WiringTest`:
     - in `test_every_python_hook_goes_through_the_launcher`, collect commands from **every** event in `settings["hooks"]`, not just the two, and from the frontmatter of **every** `.claude/agents/*.md` (each `command:` line between the first two `---`);
     - add `test_settings_wires_report_contract_on_subagent_stop`.
- **Test** (`test_report_contract.py`, unittest). Run the hook as a subprocess with `CLAUDE_PROJECT_DIR` set to a temp project. The project holds `.claude/agents/packet-reviewer.md` (`model: sonnet`, `## Reporting`) and `.claude/agents/packet-coder.md` (`model: haiku`, `## Reporting`). Write a fixture subagent transcript at `<tmp>/s1/subagents/agent-a1.jsonl` with `transcript_path=<tmp>/s1.jsonl`.

  **Report mode** (packet-reviewer):
  1. no line, so block A and the reason names the brief's path;
  2. a good line with a fresh report: no output;
  3. a good line with the report missing: block C;
  4. a report whose mtime is set to 2020 by `os.utime`: block C;
  5. a line naming another path: block B;
  6. `stop_hook_active: true`: no output;
  7. a brief with no report path: no output;
  8. no `last_assistant_message`, so the hook reads the transcript;
  9. no `agent_transcript_path`, so it derives the path;
  10. a backtick-wrapped line, and a line buried in prose: no output;
  11. a relative brief path resolved against `cwd`;
  12. a pass-2 brief that names an old report plus a new one: a line naming the new one with the file written passes, and with no line the reason names the new one.

  **Line mode** (packet-coder):

  13. prose with no line: block L; the reason contains `Write no file` and not the brief's path;
  14. `HT-01 done abc1234 -`, with a brief that names a stray report path and no file written: no output;
  15. `HT-01 blocked - - reason=context`: no output;
  16. prose, with a transcript that holds a `tool_result` containing `Context cap reached`: no output;
  17. an agent file with `model: claude-haiku-4-5` behaves as line mode.

  **Every agent:**

  18. a reviewer with no line and a `Context cap reached` result: no output;
  19. `agent_type` `Explore` (no agent file): no output;
  20. malformed stdin: exit 0, no output;
  21. a reviewer whose transcript holds a Read `tool_result` quoting the marker mid-text (`    54\tCAP_MARKER = "Context cap reached"`) and prose at the end: block A;
  22. a packet-coder whose brief quotes ``At `Context cap reached`, call no tool`` and prose at the end: block L;
  23. a reviewer whose cap deny `tool_result` content is a list of `text` blocks: no output.

  The cap fixture in cases 16, 18 and 23 uses the real deny text (`PreToolUse:Edit hook error: Context cap reached: ...`, `is_error: true`). **Why 21 and 22 fail on revert:** with the cap test put back to "any transcript line contains the marker", both see no output.

  **Why it fails on revert:** without the hook, cases 1, 3, 4, 5 and 13 see no block output.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_report_contract.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  3. `python tools/agent-skills/sync.py --check`
  4. `python -m unittest discover -s tools/docs-gen -p "test_harness_tables.py"`
- **Docs:** the hook docstring now; the prose in HT-11b (row 63). Don't run `regen.py` (HT-11b does).
- **Commit:** `HT-06: SubagentStop hook enforces the report contract and the coder line`
- **Benefit / re-measure:**
  - Report writers: no report write 5.5% → < 2%, and no usable line 14.6% → < 3%.
  - packet-coder: no usable line with the cap not fired 4.9% → < 1%.
  - It never fires on a capped Haiku (D-HT3).

  Re-measured by `metrics returns --since <wave-2 merge>` (`report_writers`, `coder_lines`). The cost is one extra turn on a dispatch that breaks the contract; F2/F3a say those already cost an investigation or a re-dispatch.
- **Rules:** R-PY, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only.
  - At most 700 lines; docstrings.
  - `settings.json` keeps CRLF and its indent; new files CRLF.
  - No private data in fixtures.
  - One line back, no report or worknote. Commit the hook plus cases 1-12 as part 1 once they pass. At `Context cap reached`, return `HT-06 blocked <sha|-> - reason=context`.

### HT-07: the Haiku coder contract in the agent files

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-07-coder-contract` · worktree `ht-07`, from `main`
- **Files:**
  - `.claude/agents/packet-coder.md`: the bullets that start `- Edit only the files`, `- Commit once`, `- Your context limit`, `- If the brief says "continuation` and `- If the brief says "review fix`, the `Report, compactly:` line, and the `## Reporting` paragraph. **Leave the `- Shell:` and `- Host:` bullets as they are**: `test_platform_rule.py` checks their phrases.
  - `.claude/agents/orchestrator.md`: the `- Coder loop` bullet, the `Every brief carries:` paragraph under `## Brief format`, and the `- On a one-line return` bullet.
  - the regenerated `.codex/agents/packet-coder.toml` and `orchestrator.toml`;
  - new `tools/agent-skills/test_coder_contract.py`.
- **Contract:** C4 in full (above).
- **Steps.** Replace each named text with exactly this:
  1. **packet-coder, `- Edit only the files ...` bullet:** keep its first sentence. Replace its ending `say exactly what you changed and why in your report.` with `add the line \`Deviation: <what> - <why>\` to the commit body.`
  2. **packet-coder, `- Commit once ...` bullet:** `- Commit on the worktree's current branch: \`git add\` only the packet's files, then \`git commit\` with the message from the brief (it includes the attribution lines), adding any \`Deviation:\` or \`Rejected\` lines to the body. You may commit a finished part earlier with the subject suffix \`(part N)\`, but only after its checks pass; never commit a failing state. Do not amend other commits.`
  3. **packet-coder, `- Your context limit ...` bullet:** `- You write code and commits, never a report, worknote or summary: the commits are your record. Your context limit is 95k tokens. Read only the line ranges you need (use Grep to find them). Past 90k a hook denies every tool with \`Context cap reached\` and asks for a final report: your final report is then exactly one line, \`<packet> blocked <commit-sha|-> - reason=context\`. Call no tool. Leave uncommitted edits in the worktree; the next packet-coder picks them up. A packet gets at most 3 coder sessions.`
  4. **packet-coder, `- If the brief says "continuation ...` bullet:** `- If the brief says "continuation N of 3, base=<sha>": run \`git -C <worktree> log --oneline <sha>..HEAD\`, \`git -C <worktree> status --porcelain\` and \`git -C <worktree> diff --stat\`, compare them with the spec's steps, and start at the first step not done. Keep uncommitted edits that match the spec; never redo committed work. Then finish, run every check the packet lists, and commit.`
  5. **packet-coder, `- If the brief says "review fix ...` bullet:** `- If the brief says "review fix, pass N of 3", read each review report it names and apply only their \`## Fix notes\`, exactly as written, in the files the notes name; never any other file. If a note does not match the code, don't apply it: add \`Rejected <note-id>: <reason>\` to the commit body. Then rerun every check the packet lists and commit with the brief's message (\`<packet> review fixes, pass N\`); if you rejected every note, commit with \`--allow-empty\` so the reviewer sees the reasons. A fix pass that stops on context returns the same context line, and its fresh sessions count up to 3 within that pass.`
  6. **packet-coder, the `Report, compactly: ...` line:** `Return only the line in \`## Reporting\`. No advice, no summary.`
  7. **packet-coder, the `## Reporting` paragraph:** `Return exactly one line: \`<packet-or-task> <done|blocked|failed> <commit-sha|-> -\`. On \`blocked\` or \`failed\`, add \` reason=<context|checks|spec|host|platform|tool>\` and, when it applies, \` at=<k>\` (the check or step number where you stopped). Add \` not_run=<k>,...\` with the number of each packet check you did not run. For the platform, host and tool cases above, the line is \`<packet> blocked - - reason=<platform|host|tool>\`, followed by the \`blocked:\` text on the next line. Write no report file, even if the brief names one. Never message other agents or wait for replies.`
  8. **orchestrator, `- Coder loop ...` bullet:** `- Coder loop (3 Haiku sessions, then one Sonnet implementer): a \`packet-coder\` writes code and commits, never a report or worknote (owner ruling, harness-tuning D-HT3). When it returns \`blocked\` with \`reason=context\`, or returns no usable line, dispatch a fresh \`packet-coder\` with "continuation N of 3, base=<sha>", where the base is the packet's start SHA; it resumes from the branch. After the third stop, dispatch one \`rust-gameserver-dev\` (tools packets) or \`documentation-writer\` (docs packets) with the same base and a report path. If that fails, \`architect\` re-slices. Fix notes never go back into the session that wrote the code: each fix pass is a fresh \`packet-coder\`.`
  9. **orchestrator, `Every brief carries: ...` paragraph:** `Every brief carries: the worktree path, the packet section path (not its text; the agent reads it) and the return line. A \`packet-coder\` brief names no report path; its line is \`<packet> <done|blocked|failed> <commit-sha|-> -\` with optional \`reason=\`, \`at=\` and \`not_run=\` (packet-coder.md § Reporting). Every other implementer's brief carries the absolute report path inside its worktree (\`<worktree>\.claude\reports\<campaign>\<packet>-<role>.md\`, never the root checkout) and the line \`<packet> <done|blocked|failed> <commit-sha|-> <report-path>\`. A reviewer's brief does not use that return line: it names the reviewer return line in [packet-reviewer.md](packet-reviewer.md) (the reviewed SHA and \`verdict=<ship|fix|decision>\`). It also carries the scope (packet, wave or campaign), the commit range, the pass number, the coder's last return line and, from pass 2, the previous report's path. The \`<role>\` is \`impl\`, \`review-<N>\`, \`ruling-<N>\` or \`ruling-check\` (a wave review \`wave<W>-review-<N>\`, the campaign \`campaign-review-<N>\`). Keep briefs under about 25 lines.`
  10. **orchestrator, `- On a one-line return ...` bullet:** `- On a one-line return, update the ledger row; an implementer's \`done\` starts that packet's review loop. A \`packet-coder\`'s \`blocked\` or \`failed\` has no report: route it by \`reason=\` (\`context\` → the coder loop; \`checks\` → the Sonnet implementer; \`spec\` → \`architect\`; \`host\`, \`platform\` or \`tool\` → the packet's host note). For any other agent, open its report only when the return says \`blocked\`, \`failed\` or \`verdict=decision\`.`
  11. R-AGENT: run sync, then check.
- **Test** (`test_coder_contract.py`, unittest, reading the two files from the repo root):
  - **packet-coder.md contains** `Context cap reached`, `reason=context`, `base=<sha>`, `Rejected <note-id>:`, `Deviation: <what> - <why>`, `not_run=`, `--allow-empty`, `(part N)`, and in `## Reporting` the text `<commit-sha|-> -`.
  - **packet-coder.md does not contain** `worknotes/`, `worknote first`, `<report-path>`, `in your report`, `write your report` or `Report, compactly`.
  - **orchestrator.md:** its `- Coder loop` bullet contains `base=<sha>` and `no usable line`, and not `worknote path`. Its `Every brief carries:` paragraph contains `names no report path`.

  **Why it fails on revert:** today's packet-coder.md has `worknotes/` and lacks `Context cap reached`; today's orchestrator bullet has `worknote path`.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_coder_contract.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_platform_rule.py"`
  3. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  4. `python tools/agent-skills/sync.py --check`
- **Docs:** the agent files are the doc. The skill and the workflow docs follow in HT-11a.
- **Commit:** `HT-07: packet-coder writes code and commits, returns one line, resumes from the branch`
- **Benefit / re-measure:**
  - The coder's instructions match the owner's rule and the user-level cap. No coder wastes a turn on a write the cap denies (F5: 51 Edits and 12 Writes were the first denied tool).
  - A continuation resumes from the branch rather than from a worknote nobody can write past the cap.
  - The orchestrator routes every coder return without opening a file.

  Re-measured with `metrics returns` (`coder_lines.capped_usable_line / capped`, informational) and with the `no_usable_line_uncapped` target shared with HT-06.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only; edit only the named bullets and paragraphs.
  - At most 700 lines; docstrings.
  - Agent `.md` files keep CRLF.
  - Run `python tools/agent-skills/sync.py`, then `--check`, and add the changed `.codex/agents/*.toml`.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-07 blocked <sha|-> - reason=context`.

### HT-08: per-session resume card

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-08-resume-card` · worktree `ht-08`, from `main`
- **Files:**
  - `.claude/hooks/resume_card.py` (C5; at most 200 lines; update the docstring);
  - `tools/agent-skills/test_hooks.py`, the `ResumeCardTest` class only (lines 40-115 today);
  - `.claude/agents/orchestrator.md`, the `## State on disk` paragraph;
  - the regenerated `.codex/agents/orchestrator.toml`.
- **Contract:** C5 in full (above).
- **Steps:**
  1. In `main()`, parse stdin JSON. A malformed payload means no card, exit 0. Then call `build_card(root, transcript_path)`.
  2. Keep `POINTER`, `CARD_LINES` and the name check. Add `INFERRED`, `tier1(transcript)` and `tier2(transcript, root)`.
  3. Replace the orchestrator's State on disk paragraph with: `Keep \`docs/analysis/<campaign>/RESUME.md\` current at every wave boundary. When you start or resume a campaign in a session, Write its folder name to \`.claude/state/active-campaign\`: the resume card after a compaction comes from this session's own last write of that file (or, failing that, from the campaign most of your recent dispatches name). Write an empty file when the campaign closes. After a compaction, read RESUME.md first.`
  4. R-AGENT: run sync, then check.
- **Test** (rewrite `ResumeCardTest`; a helper writes a fixture main transcript and passes `{"transcript_path": ...}` on stdin). Cases:
  1. **The F7 bug:** the global pointer file says `other` and has a RESUME. The transcript has no pointer write and no Agent uses. There is no card.
  2. The transcript's last Write of `<tmp>\.claude\state\active-campaign` says `demo`, and the global file says `other`. The card names `demo`.
  3. Two writes, `alpha` then `demo`: the card names `demo`.
  4. An Edit whose `new_string` is `demo`: the card names `demo`.
  5. A last write that is empty, plus 5 Agent prompts naming `docs/analysis/demo/`: no card.
  6. No pointer write, and 3 of 4 prompts name `demo`: an inferred card, whose text starts with `Context was compacted. Inferred campaign`.
  7. 2 of 4 prompts: no card. 2 of 2 prompts: no card (`naming < 3`).
  8. A prompt that names a campaign with no RESUME.md doesn't count.
  9. Keep today's truncation at 40 lines, the BOM, the path-name rejection (`../demo` written as the content), malformed JSON, and the `build_card` raising case.
  10. A missing `transcript_path`: no card.

  **Why it fails on revert:** the old hook prints a card for case 1 and names `other` in case 2.
- **Checks:**
  1. `python -m unittest discover -s tools/agent-skills -p "test_hooks.py"`
  2. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  3. `python tools/agent-skills/sync.py --check`
- **Docs:** the hook docstring; the skill and docs prose in HT-11b.
- **Commit:** `HT-08: resume card comes from this session, not a shared pointer`
- **Benefit / re-measure:** wrong-campaign cards go from 44% to ≤ 5%, with tier 1 and tier 2 reported apart. Re-measured with `metrics resume-cards --since <wave-2 merge>`.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, absolute paths, foreground checks.
  - No cargo, stash, push, prune or `gh pr`.
  - Listed files only; edit only `ResumeCardTest` in `test_hooks.py` and only `## State on disk` in orchestrator.md.
  - At most 700 lines.
  - CRLF kept.
  - Sync, then check.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-08 blocked <sha|-> - reason=context`.

### HT-09: Not checked, `not_run=`, and the report-format test

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-09-not-checked` · worktree `ht-09`, starting from `ht-w2` with 10, 08, 06 and 07 merged
- **Files:**
  - `.claude/agents/packet-reviewer.md`: the report-sections paragraph, the `**From pass 2:**` bullet, and `## Reporting`;
  - `.claude/agents/orchestrator.md`: `## Waiting`;
  - the regenerated `.codex/agents/*.toml`;
  - new `tools/agent-skills/test_report_formats.py`.
- **Contract:** C6, and C1's `LINE`.
- **Steps:**
  1. **Reviewer sections.** Change "Write the report with three sections" to four. After the `## Not fixed` bullet, add `- \`## Not checked\`: every check the spec or your scope asked for that you did not run or could not judge, one per line as \`- NOT ASSESSED — <what> — <reason>\`; include each check in the coder's \`not_run=\` list (from the brief) that you did not verify yourself. Write \`- none\` when nothing is left.`
  2. **Reviewer, pass 2.** In the `**From pass 2:**` bullet, replace `Judge a rejected note on its merits, and drop it if the coder's reason holds.` with `A rejected note's reason is a \`Rejected <note-id>:\` line in the fix commit's body (\`git log -1 --format=%B\`); judge it on its merits, and drop the note if the reason holds.`
  3. **Reviewer `## Reporting`.** Append ` not_assessed=<n>` (the number of NOT ASSESSED lines) to both return-line templates, after `verdict=`/`verdicts=`.
  4. **Orchestrator `## Waiting`.** Add the bullet `- A reviewer line with \`verdict=ship\` and \`not_assessed=<n>\` above 0: open only the report's \`## Not checked\` section and copy its items into the packet's ledger row (or worknote) before shipping, so the close-out sees what nobody checked.`
  5. R-AGENT: run sync, then check.
- **Test** (`test_report_formats.py`). Load `.claude/hooks/report_contract.py` with `importlib.util.spec_from_file_location` and use its `LINE`. Cases:
  - **Sample lines.** These match:
    - `HT-01 done abc1234 C:\w\.claude\reports\c\HT-01-review-1.md`
    - `HT-01 done abc1234 -`
    - `HT-01 blocked - - reason=context`
    - `HT-01 done abc1234 - not_run=3,4`
    - `wave2 done abc1234 x.md verdict=fix verdicts=HT-06:ship,HT-07:fix not_assessed=2`
    - `` `HT-01 blocked - x.md reason=context` ``

    These don't: `HT-01 done` and `done abc x.md` (bad SHA).
  - **packet-reviewer.md** contains `## Verdict`, `## Fix notes`, `## Not fixed` and `## Not checked`, in that order, plus `NOT ASSESSED —`, `not_run=`, `Rejected <note-id>:` and `not_assessed=<n>`.
  - **orchestrator.md** contains `not_assessed=`.
  - **Templates.** Fill each backticked return-line template in packet-coder.md and packet-reviewer.md: `<packet-or-task>` → `HT-01`, `<done|blocked|failed>` → `done`, `<commit-sha|->` and `<reviewed-head-sha>` → `abc1234`, `<report-path>` → `x.md`, `<ship|fix|decision>` → `ship`, `<n>` → `0`, `<packet>:<ship|fix|decision>,...` → `HT-01:ship`. Each must match `LINE`.
  - **Metrics copy.** `@unittest.skipUnless((ROOT / "tools/token-profile/metrics/transcripts.py").is_file(), ...)`: the metrics `LINE.pattern` equals the hook's.

  **Why it fails on revert:** removing `## Not checked`, the suffix or the `Rejected` reading fails the reviewer case, and a drifting template fails the template case. This closes AH-24-review-2:25 (no test guarded the verdict words or the return-line format).
- **Checks:** `python -m unittest discover -s tools/agent-skills -p "test_report_formats.py"`, then all `test_*.py` in tools/agent-skills, then `python tools/agent-skills/sync.py --check`.
- **Docs:** HT-11b (row 63: the cycle in the skill).
- **Commit:** `HT-09: reviewer Not checked section, not_run passthrough, report-format test`
- **Benefit / re-measure:** unrun checks become visible on the return line, where today about 6 were shipped silently. 100% of reviews after wave 2 carry the section (`metrics returns` `not_assessed_suffix` / `reviewer_lines`). Coder skips show as `coder_lines.not_run_suffix`.
- **Rules:** R-PY + R-AGENT, inlined:
  - Windows PowerShell, no cargo, listed files only, at most 700 lines, CRLF kept, sync then check.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-09 blocked <sha|-> - reason=context`.

### HT-10: unique decision and packet ID prefixes

- **Agent / host:** packet-coder · `Host: windows` · branch `ht/HT-10-id-prefixes` · worktree `ht-10`, from `main`
- **Files:** `tools/agent-skills/test_ledgers.py` and `.github/workflows/agent-skills.yml` (LF).
- **Contract:** C7 in full (above).
- **Steps:**
  1. Write a scan into the test module: `prefix_owners() -> dict[str, set[str]]` over `docs/analysis/*/*.md` (the top level of each campaign folder only, read as utf-8 with errors replaced), using `DECISION_ROW` and `PACKET_ROW`.
  2. Run it once by hand (`python -c "import sys; sys.path.insert(0, r'<wt>\tools\agent-skills'); import test_ledgers as t; print({k: sorted(v) for k, v in t.prefix_owners().items() if len(v) > 1})"`).
  3. Put every prefix it lists into `GRANDFATHERED` with its exact folders, except tokens that aren't packet IDs (`UTF`, `SHA`, `AES` and similar), which go to `NOT_PACKETS`. Add a one-line comment: "frozen 2026-10-31; never add a new campaign here".
  4. Add `test_prefixes_belong_to_one_campaign`. Its failure message names each prefix and its folders and points at the campaign-packet skill § 1.
  5. In `agent-skills.yml`, add `- "docs/analysis/**/*.md"` to both `pull_request.paths` and `push.paths`, after the `RESUME.md` line. Keep LF.
- **Test:** the new test itself. Prove it: temporarily copy a folder's README into `docs/analysis/zz-probe/README.md`, so its prefixes now belong to two folders. The test must fail. Then delete the probe. **Why it fails on revert:** removing the test removes the guard (it is a guard test). The probe proves it catches a new collision; the reviewer repeats it.
- **Checks:** `python -m unittest discover -s tools/agent-skills -p "test_ledgers.py"`, then all `test_*.py` in tools/agent-skills.
- **Docs:** HT-11b writes the SKILL § 1 note.
- **Commit:** `HT-10: one campaign per decision and packet ID prefix`
- **Benefit / re-measure:** no new cross-campaign ID collisions, enforced in CI, so `metrics packets` joins stay right. Checked by the test in CI.
- **Rules:** R-PY, inlined:
  - Windows PowerShell, no cargo.
  - Listed files only.
  - The `.yml` stays **LF**; `test_ledgers.py` keeps CRLF.
  - Don't leave the probe folder behind.
  - One line back, no report or worknote. At `Context cap reached`, return `HT-10 blocked <sha|-> - reason=context`.

### HT-11a: the coder contract in the skill and the workflow docs

- **Agent / host:** documentation-writer · `Host: windows` · branch `ht/HT-11a-coder-docs` · worktree `ht-11a`, starting from `ht-w2` with 10, 08, 06 and 07 merged (in parallel with HT-09)
- **Report:** `<wt>\.claude\reports\harness-tuning\HT-11a-impl.md`; return line `HT-11a <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Contract:** C4, and the packet-coder and orchestrator text HT-07 merged.
- **Files** (all CRLF; line numbers are from 2026-10-31, so find each by its text):
  - `.claude/skills/campaign-packet/SKILL.md`:
    - § 1, the agent table rows for "One packet, one worktree" and "Apply a review's fix notes": the output becomes "local commits and a one-line return".
    - § 3 step 2: a packet-coder brief names no report path. Its line is C4's. A continuation brief carries `base=<start SHA>`. Other implementers keep the report path.
    - § 3 step 4: route a coder's `blocked` by `reason=`.
    - § 4 fix-pass step 2: rejected notes go in the commit body; all rejected → `--allow-empty`. Drop the `fix-<N>` report role (role list in the long paragraph after the table).
    - § 4 ship step 2: worknotes are the orchestrator's.
    - § Sessions, the table row "Coder session" and escalation step 1: a context stop returns the line; the next session resumes from the branch with `base=`.
  - `docs/agents/development-workflow.md`: the coder-report and worknote lines in § Worker lifetime and notifications (around lines 112-113 and 129-130).
  - `docs/agents/ai-sdlc.md`: the coder report and worknote mentions (around lines 18, 57-63, 85, 104 and 154).
  - `docs/agents/ai-sdlc-example.md`: the coder report in the example (around line 50).
  - `docs/agents/ai-harness.md`: the coder report and worknote mentions (around lines 124 and 131).
  - `AGENTS.md`: the coder return contract (around lines 33-35).
  - whatever `sync.py` regenerates.
- **What to write:**
  - Say once, in the skill's § 3, why: the owner's rule (D-HT3) is that Haiku coders write code, not reports, and the user-level cap (Haiku agents only) stays. Cite it as an owner rule; never call the cap a defect.
  - Sonnet and Opus agents keep their reports and worknotes; change only what a packet-coder is told to do.
  - Leave `docs/agents/rules-and-gotchas.md` (it covers other workers) and every status doc alone.
- **Then run:**
  1. `python tools/agent-skills/sync.py`
  2. `python tools/docs-gen/regen.py --check --skip-crate-graph`
  3. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  4. `python -m unittest discover -s tools/docs-gen -p "test_*.py"`
  5. `python tools/agent-skills/sync.py --check`

  If step 2 fails on a block you didn't touch, note it in the report; HT-11b runs `regen.py`.
- **Test:** docs only. `test_agent_refs.py`, `test_harness_tables.py`, `test_ledgers.py` and HT-07's `test_coder_contract.py` must pass. Before committing, Grep the listed files for `-coder.md`, `worknote` and `coder report` and account for every remaining hit in the report.
- **Docs:** doc-update-map row 62 (agent workflow). List it in the report for the PR body.
- **Commit:** `HT-11a: document the Haiku coder contract - code and commits, one line, resume from the branch`
- **Benefit / re-measure:** none directly. No doc tells an orchestrator to give a coder a report path, which keeps `coder_lines.dash_report_lines` near 100% of usable coder lines after wave 2.
- **Rules:**
  - Windows PowerShell; no cargo, stash, push, prune or `gh pr`.
  - Never hand-edit a `<!-- gen:NAME -->` block; `regen.py` owns it.
  - `docs/**/*.md` stay CRLF.
  - Files under 700 lines; if `development-workflow.md` would pass 700, move detail to the skill, don't grow the doc.
  - Edit the skill only under `.claude/skills/`, then run `sync.py`.
  - One commit.
  - Don't edit agent files (HT-07 and HT-09 own them).
  - Context: if you run short, commit what passes, write `<wt>\docs\analysis\harness-tuning\worknotes\HT-11a.md` (done, left, next file), write your report, and return `blocked` with `reason=context`.

### HT-11b: wave-2 harness docs and regen

- **Agent / host:** documentation-writer · `Host: windows` · branch `ht/HT-11b-docs` · worktree `ht-11b`, starting from `ht-w2` with HT-06..HT-10 and HT-11a merged
- **Report:** `<wt>\.claude\reports\harness-tuning\HT-11b-impl.md`; return line `HT-11b <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Files** (all CRLF):
  - `docs/agents/development-workflow.md`: § Agent skills and the shell guard;
  - `docs/agents/ai-harness.md`, prose only;
  - `docs/agents/ai-sdlc.md`, if it describes hooks or the resume card;
  - `AGENTS.md`, if it describes hooks or the resume card;
  - `.claude/skills/campaign-packet/SKILL.md`;
  - `tools/agent-skills/README.md`;
  - whatever `regen.py` and `sync.py` regenerate.
- **What to write:**
  - **The hooks:**
    - `report_contract.py`: SubagentStop; blocks once; report mode for report writers and line mode for Haiku agents with `## Reporting`; never after `Context cap reached`; the four reasons.
    - The per-session resume card: both tiers; the pointer is written per session.
    - The user-level Haiku cap as the owner's rule (D-HT3): it is not in the repo, the repo never works around it, and a capped coder returns one line.
  - **Return lines.** `reason=`, `at=`, `not_run=` and `not_assessed=<n>`, plus the reviewer's `## Not checked`.
  - **SKILL § 1:**
    - "A decision prefix (`D-XX`) and a packet prefix (`XX-`) belong to one campaign; `test_ledgers.py` enforces it. Don't start a table row with another campaign's ID; cite it in a later column."
    - "A Decisions table may add **Recommended** and **Evidence** columns."
  - **SKILL § 3:** the SubagentStop hook blocks a missing line or report once.
  - **SKILL § 4:** four report sections; the reviewer copies `not_run=` into `## Not checked`; on `ship` with `not_assessed`, copy the items to the ledger.
  - **SKILL § 5:** the card is per session; the pointer is written per session; it's emptied at close-out.
  - **`tools/agent-skills/README.md`:** add `test_report_contract.py`, `test_coder_contract.py` and `test_report_formats.py` to the test list. Split the long list into a bullet list while you're there, so later packets don't conflict on one line.
  - In `ai-harness.md`'s test notes, note that `test_report_formats.py` closes the AH-24-review-2:25 note.
- **Then run:**
  1. `python tools/agent-skills/sync.py`
  2. `python tools/docs-gen/regen.py --skip-crate-graph`
  3. `python tools/docs-gen/regen.py --check --skip-crate-graph`
  4. `python -m unittest discover -s tools/agent-skills -p "test_*.py"`
  5. `python -m unittest discover -s tools/docs-gen -p "test_*.py"`
  6. `python tools/agent-skills/sync.py --check`
- **Test:** docs only. The existing `test_agent_refs.py`, `test_harness_tables.py` and `test_ledgers.py` must pass.
- **Docs:** doc-update-map rows 62 (agent workflow) and 63 (harness wiring). List them in the report for the PR body.
- **Commit:** `HT-11b: document the report-contract hook, Not checked and the per-session resume card`
- **Benefit / re-measure:** none directly. It keeps docs matching behaviour, which the campaign review checks.
- **Rules:** as HT-11a, plus: don't redo HT-11a's coder-contract text; link to it.
