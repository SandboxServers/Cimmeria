# ai-harness-docs: work packets, wave 4a (review loops replace packet-review-fixer)

Ledger: [README.md](README.md) (F38-F56, D-AH42 to D-AH58). The verbatim text blocks these packets copy are in [w4-drafts.md](w4-drafts.md); the W4b blocks are in [w4b-drafts.md](w4b-drafts.md). Wave 4b (the README AI section and the deeper reference, AH-23a and AH-23b), wave 5 (the campaign review, AH-24), the approved spicy packets (S3, S5a-c, S6) and the vetoed S1 are in [work-packets-w4b.md](work-packets-w4b.md); S4 (S4a-d, contract C19) is in [work-packets-s4.md](work-packets-s4.md). Wave 4c (the platform-keyed shell for every agent, contract C17, AH-25 to AH-27) is in [work-packets-w4c.md](work-packets-w4c.md). The owner's answers of 2026-10-18 behind those changes: [w4-owner-answers-report.md](w4-owner-answers-report.md) and [w4-s1s4-report.md](w4-s1s4-report.md). Earlier waves: [work-packets.md](work-packets.md), [work-packets-w3.md](work-packets-w3.md), [work-packets-w3b.md](work-packets-w3b.md), [work-packets-w3d.md](work-packets-w3d.md).

Each packet below is self-contained. A packet-coder skips CLAUDE.md, so it reads **§ W4 rules** and the contract items its packet names, then its own section, and nothing else.

## Dispatch rules (for the orchestrator)

0. **Nothing merges while `main` is red** (D-AH55). Before every `ship.ps1 merge` in this campaign, AH-W40 included, check that no issue labelled `main-red` is open (`gh issue list --label main-red --state open`) and that the latest `main` runs of the container build and smoke (`pr-container`) and of `build-and-test` are green. If either fails, hold the merge and tell the owner; do not fix `main` from this campaign. On 2026-10-18 the red was #1844 (the container build and smoke: `seed-accounts-stage.sql` TRUNCATE against the `sgw_player` foreign key), fixed by someone else in #1849; #1844 was closed and `pr-container` was green on dcf736016 at 22:48Z. Recheck anyway: the rule is the state at merge time, not this note.
1. **AH-W40 first.** It ships this ledger (docs only). Every later worktree is cut from `origin/main` after it merges, so the packets, the drafts file and the amended work-packets-w3.md line 216 are in every worktree. Until then, nothing in this file is dispatched.
2. **The new loop is in force now (D-AH43).** From the moment AH-W40 merges, every review in this campaign, W3c included (AH-11g, AH-11h, AH-11p), runs the C12 loop: a read-only `packet-reviewer`, then a fresh `packet-coder` with the fix notes. Do not dispatch `packet-review-fixer` again. Until W4a merges, your persona text and the campaign-packet skill still describe the old agent: **follow C12 in this file, not them.** The same applies to any other campaign planning "one review-fixer per wave" (F39: persistent-player-data waves 2 and 3); tell its coordinator, or edit its RESUME.md, before W4a merges, because the agent file is deleted then.
3. **W3c merges before W4a starts** (F42). AH-11h edits rules-and-gotchas.md line 102 and development-workflow.md; AH-20 edits the lines next to them, and git conflicts on adjacent-line edits. Cut the W4a worktrees only after AH-11g, AH-11h and AH-11p have merged.
4. **W4a worktrees** (one `ops` dispatch each, all in one message): `pwsh tools/build-lane/mk-worktree.ps1 <branch> <name>` for the five rows of C15 that have a branch. AH-17 to AH-21 are file-disjoint (C15) and run in parallel, in the background.
5. **Packet loops run as each coder returns** (C12). Don't wait for the wave: dispatch a packet's review pass 1 when its coder returns `done`.
6. **Integration** (D-AH45): once AH-17 to AH-21 all have `ship`, follow § W4a ship procedure. W4a ships as **one PR**, so `main` never holds a half-retired agent (a skill that names a deleted agent, or a persona that dispatches one).
7. **Briefs** (each under 25 lines) use the C14 templates. The report path is `<worktree>\.claude\reports\ai-harness-docs\<file>` (C13 names the files). Every brief adds: "PowerShell only; run every check in the foreground with `Set-Location <worktree>;` at the start of the call; commit as soon as the checks pass."
8. **Quoting.** Exact text to find or insert is in a fenced `text` block, copied byte for byte with no edits. A short text inside a code span follows work-packets.md rule 8: `\`` is a plain backtick and `\|` a plain `|`.
9. **After W4a merges**, tell the user in one line: relaunch Claude Code, because the orchestrator persona changed (the old persona stays loaded in a running session). Then rewrite RESUME.md.

## W4 rules (every W4 packet)

- **R1.** Work only in your worktree, with absolute paths. Start every PowerShell call with `Set-Location <worktree>;` and run every check in the foreground. `shell_guard` refuses a shell command that only reads (every stage `Select-String`, `Get-Content`, `cat` or `grep`), so run each `Select-String -Path <f> -Pattern <p> -SimpleMatch` check with the Grep tool instead (the same literal pattern and file; count the matching lines), or end the PowerShell command with `# guard-ok`.
- **R2.** PowerShell only, never bash. No `cargo` at all (no W4 packet compiles Rust). Never run `git stash`, `git push`, `git worktree prune`, `git reset`, `git checkout`, `git restore` (except where a step names it) or `gh pr`.
- **R3.** Edit only the files your packet's **Files** line names, plus the files `python tools/agent-skills/sync.py` regenerates from them. If sync changes any other file, stop and return `failed`.
- **R4.** Every text file is CRLF in the working tree, except `.github/workflows/*.yml` and `*.sh` (LF). The Edit tool keeps a file's line endings. After a `[IO.File]` write, check a `.md` with `$t=[IO.File]::ReadAllText('<abs>'); $t -match "(?<!\r)\n"`: it must print `False`. To fix: `$f='<abs>'; $t=[IO.File]::ReadAllText($f); [IO.File]::WriteAllText($f, ($t -replace "\r?\n","`r`n"))`.
- **R5.** Never hand-edit a generated block (the lines between `<!-- gen:NAME -->` and `<!-- /gen:NAME -->`). Only a packet whose steps say so runs `regen.py`, and it keeps only the changes its steps name.
- **R6.** Every find text must match **exactly once** in its file. If it matches zero times or more than once, don't guess: stop and return `blocked` with the step number.
- **R7.** After editing anything under `.claude/agents/` or `.claude/skills/`, run `Set-Location <wt>; python tools/agent-skills/sync.py`, then `python tools/agent-skills/sync.py --check` (exit 0, prints nothing stale).
- **R8.** File caps: soft 500 lines, hard 700. No IPs, credentials or account names.
- **R9.** No whole-file rewrites except where a step says so: `git -C <wt> diff --stat` must show small counts on every file you only edited.
- **R10.** Commit once at the end with the brief's message: `git -C <wt> add -- <each file>`, then `git -C <wt> commit -F <message file>`. In a review fix pass, the subject is `<packet> review fixes, pass N`.
- **R11. Retired-name check**, for the files you own: `Select-String -Path <abs file> -Pattern 'review-fixer','recheck' -SimpleMatch` returns no line, except where your packet says otherwise.

## Contract

### C12. The review loop (D-AH43; replaces C6)

Three loops, one shape. A **pass** is one fresh, read-only `packet-reviewer` session over one scope. At most **3 passes per loop**.

| Loop | Starts | Scope | Fix notes go to |
|---|---|---|---|
| Packet | that packet's coder returns `done` | the packet's commits, `<base>..<head>` | a fresh `packet-coder` in that packet's worktree |
| Wave | every packet of the wave has `ship` | the whole wave | a fresh `packet-coder` per packet named; in a wave that ships from an integration worktree, one fresh `packet-coder` there |
| Campaign | the last wave has merged, before close-out | everything the campaign merged, on `origin/main` | a fresh `packet-coder` in the `ops` worktree `<campaign>-review` |

A wave of one packet has no wave loop.

On each verdict:

1. `ship`: the loop is done.
2. `fix` on pass 1 or 2: a fresh `packet-coder` with "review fix, pass N of 3", the worktree and the report path. It applies only `## Fix notes`, rejects a note that does not match the code (with its number and reason), reruns every check the packet lists, and commits `<packet> review fixes, pass N`. Then a fresh reviewer runs pass N+1 with the previous report's path.
3. `fix` on pass 3, or `decision` on any pass: `architect` (or the Opus advisor for protocol and RE) decides only what to fix and how, in a ruling note in `worknotes/<packet>.md` (`worknotes/wave<W>.md`, `worknotes/campaign-review.md`): re-spec, re-slice, accept with a follow-up packet, or BlockedDecision. The architect writes no code or fix (D-AH48, decided 2026-10-18; D-AH51). One Sonnet implementer (`documentation-writer` for docs, `rust-gameserver-dev` for code and tools) applies it, then one fresh reviewer runs a **ruling check** outside the 3-pass count. A ruling check that is not `ship`, or an owner call, is BlockedDecision.

Counting words (D-AH6, amended): a **fix round** is one rerun of a check inside a session (at most 5); a **session** is one coder run (at most 3 per packet, and 3 per fix pass); a **pass** is one packet-reviewer session (at most 3 per loop).

### C13. The review report (D-AH44)

Report files, in the reviewed worktree: `<packet>-review-<N>.md`, `wave<W>-review-<N>.md` (here `wave4a-review-<N>.md`), `campaign-review-<N>.md`. Sections, in order:

- `## Verdict`: one line, `ship`, `fix` or `decision`.
- `## Fix notes`: one line each, most severe first: `N<k>. <path>:<line> — <what is wrong> — <the exact change> — verify: <check>`. At wave scope: `N<k> [<packet>]. ...`.
- `## Not fixed`: the `decision` items and anything out of scope, each with its reason.

### C14. Briefs and return lines

Coder brief: the worktree, `docs/analysis/ai-harness-docs/work-packets-w4.md § <packet heading>`, the report path `<packet>-coder.md`, the commit message with its attribution lines, and the return line `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.

Reviewer brief: the scope (`packet`, `wave` or `campaign`), the worktree, the range (`<base>..<head>`, with `<base>` the worktree's `origin/main` merge-base), the spec path (the packet section; for a wave, this file and the ledger README), the pass number, from pass 2 the previous report's path, and the report path. Return: `<packet-or-scope> done <reviewed-head-sha> <report-path> verdict=<ship|fix|decision>`; at wave scope `... verdicts=<packet>:<ship|fix|decision>,...`; or `<packet-or-scope> blocked - <report-path>`.

Fix-pass brief: the coder brief, plus "review fix, pass N of 3", the review report's path, the packet's own section (for its checks), the report path `<packet>-fix-<N>.md` and the subject `<packet> review fixes, pass N`.

Until AH-17 merges into the integration branch, the `packet-reviewer` on disk still has its old text: put "Use the report format and return line in work-packets-w4.md C13 and C14" in every reviewer brief of W3c and W4a.

### C15. W4a file ownership and merge order

| Packet | Branch | Worktree | Files (plus what sync regenerates) |
|---|---|---|---|
| AH-17 | `ai-harness-docs/ah-17-reviewer` | `ah-17` | `.claude/agents/packet-reviewer.md`, `.claude/agents/packet-coder.md`, `.claude/agents/architect.md`, `.claude/agents/packet-review-fixer.md` (deleted) |
| AH-18 | `ai-harness-docs/ah-18-skill-loops` | `ah-18` | `.claude/skills/campaign-packet/SKILL.md`, `.claude/skills/ship-pr/SKILL.md` |
| AH-19 | `ai-harness-docs/ah-19-orchestrator` | `ah-19` | `.claude/agents/orchestrator.md`, `AGENTS.md` |
| AH-20 | `ai-harness-docs/ah-20-workflow-docs` | `ah-20` | `docs/agents/development-workflow.md`, `docs/agents/rules-and-gotchas.md` |
| AH-21 | `ai-harness-docs/ah-21-harness-readme` | `ah-21` | `docs/agents/ai-harness.md` (outside its generated blocks), `README.md` |
| AH-22 | none: it commits on `ai-harness-docs/w4a` | `ah-w4a` | new `tools/agent-skills/test_agent_refs.py`, `tools/agent-skills/README.md`, `.github/workflows/agent-skills.yml`, `docs/agents/doc-update-map.md`, and the `harness-agents` block of `docs/agents/ai-harness.md` (by regen) |

Merge order into `ai-harness-docs/w4a`: AH-17, AH-18, AH-19, AH-20, AH-21 (any order works: the files are disjoint), then AH-22 is coded there.

### C16. The splice and extraction commands

A block is the text between `<!-- BEGIN <name> -->` and `<!-- END <name> -->` in `<wt>\docs\analysis\ai-harness-docs\w4-drafts.md`. **Splice** replaces the span of a target file from the first character of `<from>` up to (not including) `<to>`, with the block, then one blank line. It refuses when the block does not start with `<from>`, or `<from>` is not unique, or `<to>` is missing. One line; fill in the four values:

```text
Set-Location <wt>; $f='<abs target>'; $n='<block>'; $from='<from>'; $to='<to>'; $d=[IO.File]::ReadAllText('<wt>\docs\analysis\ai-harness-docs\w4-drafts.md'); $m=[regex]::Match($d,"(?s)<!-- BEGIN $n -->\r?\n(.*?)\r?\n<!-- END $n -->"); if(-not $m.Success){throw "no block $n"}; $t=[IO.File]::ReadAllText($f); $nl= if($t.Contains("`r`n")){"`r`n"}else{"`n"}; $b=$m.Groups[1].Value -replace "\r?\n",$nl; if(-not $b.StartsWith($from)){throw 'block'}; $i=$t.IndexOf($from); if($i -lt 0 -or $t.IndexOf($from,$i+1) -ge 0){throw 'from'}; $j=$t.IndexOf($to,$i+$from.Length); if($j -lt 0){throw 'to'}; [IO.File]::WriteAllText($f,$t.Substring(0,$i)+$b+$nl+$nl+$t.Substring($j)); 'status=ok'
```

**Extraction** writes a whole file from a block, CRLF:

```text
Set-Location <wt>; $f='<abs target>'; $n='<block>'; $d=[IO.File]::ReadAllText('<wt>\docs\analysis\ai-harness-docs\w4-drafts.md'); $m=[regex]::Match($d,"(?s)<!-- BEGIN $n -->\r?\n(.*?)\r?\n<!-- END $n -->"); if(-not $m.Success){throw "no block $n"}; $nl="`r`n"; [IO.File]::WriteAllText($f, ($m.Groups[1].Value -replace "\r?\n",$nl) + $nl); 'status=ok'
```

Anything other than `status=ok` is a stop: return `blocked` with the error text.

---

## AH-W40: ship the wave 4 ledger

Agent: packet-coder. Worktree `ah-w4-ledger`, branch `ai-harness-docs/w4-ledger` (`ops` makes it from `origin/main`). Size: about 8k working tokens. Commit subject: `docs(ai-harness-docs): wave 4 ledger: review loops replace packet-review-fixer, AI SDLC docs (AH-W40)`.

**Rules:** R1, R2, R4, R8, R10.

**Steps:**

1. Guard: `git -C C:\Users\Steve\source\projects\Cimmeria log -1 --format=%H origin/main -- docs/analysis/ai-harness-docs/` must print `3b2a99d6ee5d87c0ab834ef886c400967d22e84c`. If it prints anything else, the ledger changed on `main` after this design: stop and return `blocked` with the reason `ledger moved`.
2. Copy each of these files from `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\ai-harness-docs\` to `<wt>\docs\analysis\ai-harness-docs\`, overwriting, with `Copy-Item -LiteralPath <src> -Destination <dst> -Force`: `README.md`, `RESUME.md`, `readme-notes.md`, `w4-drafts.md`, `w4b-drafts.md`, `work-packets.md`, `work-packets-w3.md`, `work-packets-w3b.md`, `work-packets-w4.md`, `work-packets-w4b.md`, `work-packets-w4c.md`, `w4a-pr-body.md`, `w4b-pr-body.md`, `w4c-pr-body.md`, `w4-architect-report.md`, `w4-owner-answers-report.md`, `work-packets-s4.md`, `w4-s1s4-report.md`.
3. Run the R4 check on each of the 18 copies; fix any that prints `True`.

**Checks:** `git -C <wt> status --porcelain` lists exactly those 18 paths (`M` or `??`) and nothing else. `git -C <wt> diff --stat` shows no file with more than 200 changed lines except `README.md` and the thirteen new files (`readme-notes.md`, `w4-drafts.md`, `w4b-drafts.md`, `work-packets-w4.md`, `work-packets-w4b.md`, `work-packets-w4c.md`, `work-packets-s4.md`, `w4a-pr-body.md`, `w4b-pr-body.md`, `w4c-pr-body.md`, `w4-architect-report.md`, `w4-owner-answers-report.md`, `w4-s1s4-report.md`).

**Then:** `ops` runs `ship.ps1 pr -C <wt> -m <the subject>` (docs-only) and `ship.ps1 merge <PR> --retire ah-w4-ledger`. No review loop: the architect wrote these files and the owner approved the notes they implement.

**Docs owed:** none (a campaign ledger).

---

## AH-17: read-only packet-reviewer with fix notes; delete packet-review-fixer

Agent: packet-coder. C15 row AH-17. Size: about 20k working tokens. Commit subject: `chore(agents): packet-reviewer writes fix notes for a fresh packet-coder; delete packet-review-fixer (AH-17)`.

**Rules:** R1 to R11. Read C12 and C13 first.

**Steps:**

1. Replace the whole of `.claude/agents/packet-reviewer.md` with the `reviewer-agent` block: the C16 extraction, target `<wt>\.claude\agents\packet-reviewer.md`, block `reviewer-agent`.
2. `.claude/agents/packet-coder.md` line 3. Find:
   ```text
   description: "Implements one prescriptive work packet in a Cimmeria worktree: edits only its named files, runs its lane checks, commits. Not for design or PRs."
   ```
   Replace with:
   ```text
   description: "Implements one prescriptive work packet, or applies one review's fix notes, in a Cimmeria worktree: edits only the named files, runs the lane checks, commits. Not for design or PRs."
   ```
3. Same file, line 24. Find:
   ```text
   - If the brief says "continuation N of 3", read the worknote first and start at its next step; never redo committed work. Review findings never come to you: the packet-review-fixer fixes them.
   ```
   Replace with these two lines:
   ```text
   - If the brief says "continuation N of 3", read the worknote first and start at its next step; never redo committed work.
   - If the brief says "review fix, pass N of 3", read the review report it names and apply only its `## Fix notes`, exactly as written, in the files the notes name; never any other file. If a note does not match the code, don't apply it: give its number and the reason in your report. Then rerun every check the packet lists and commit with the brief's message (`<packet> review fixes, pass N`).
   ```
4. `.claude/agents/architect.md` line 16. Find:
   ```text
   - Group packets into **waves** that one packet-review-fixer can review in one context: about 6 small packets per wave.
   ```
   Replace with:
   ```text
   - Group packets into **waves** of about 6 small packets, so one packet-reviewer can hold the whole wave at the wave-boundary review. Say whether the wave ships as one PR (an integration branch) or one PR per packet, and give the merge order.
   ```
5. Delete the agent: `Set-Location <wt>; git -C <wt> rm -q -- .claude/agents/packet-review-fixer.md`.
6. R7: run sync. It rewrites `.codex/agents/packet-reviewer.toml`, `.codex/agents/packet-coder.toml` and `.codex/agents/architect.toml`, and deletes the orphan `.codex/agents/packet-review-fixer.toml`. Stage that deletion with `git -C <wt> add -- .codex/agents/packet-review-fixer.toml`.

**Test type:** none in this packet (agent text). AH-22's `test_agent_refs.py` is the regression guard for the deletion.

**Checks:**

- `Test-Path <wt>\.claude\agents\packet-review-fixer.md` and `Test-Path <wt>\.codex\agents\packet-review-fixer.toml` both print `False`.
- `(Get-ChildItem <wt>\.claude\agents -Filter *.md).Count` prints `23`.
- R11 on the three `.md` files and their three `.toml` files.
- `Select-String -Path <wt>\.claude\agents\packet-reviewer.md -Pattern 'verdict=<ship|fix|decision>' -SimpleMatch` returns one line.
- R7's `--check`; R9.

**Docs owed:** doc-update-map row "AI-harness configuration" (the agent roster) and row "AI-harness wiring" (an agent's frontmatter); AH-20 and AH-21 write the prose. List both in the report.

---

## AH-18: campaign-packet review loops; ship-pr review note

Agent: packet-coder. C15 row AH-18. Size: about 25k working tokens. Commit subject: `docs(skills): campaign-packet review loops per packet, wave and campaign; ship-pr review note (AH-18)`.

**Rules:** R1 to R11. Read C12 first. Edit only under `.claude/skills/`; sync writes `.agents/skills/`.

**Steps** (file `.claude/skills/campaign-packet/SKILL.md` unless named):

1. Line 3, the description. Find `one packet-coder per packet and one packet-review-fixer per wave,` and replace it with `one packet-coder per packet, and read-only packet-reviewer loops per packet, wave and campaign whose fix notes go to a fresh packet-coder,`.
2. Line 17. Find:
   ```text
   | Review and fix a whole wave | `packet-review-fixer` | fix commits and a report file |
   ```
   Replace with these two lines:
   ```text
   | Review one packet, a wave or the campaign (read-only) | `packet-reviewer` | a report with numbered fix notes |
   | Apply a review's fix notes | `packet-coder` (fresh) | a fix commit and a report file |
   ```
3. Line 76. Find `Group packets into **waves** that one packet-review-fixer can review in one` and replace it with `Group packets into **waves** of about six small packets, so one packet-reviewer can hold`.
4. Line 77. Find `context: about six small packets per wave. Design-heavy, RE or docs packets` and replace it with `the whole wave at the wave review (§ 4). Design-heavy, RE or docs packets`.
5. § 3 item 4, line 103. Find `4. On a one-line return, update the ledger row and stop. Open the report file` and replace it with `4. On a one-line return, update the ledger row; a coder's \`done\` starts that packet's review loop (§ 4). Open the report file`.
6. Line 104. Find `   only when the return says \`blocked\` or \`failed\`. A \`blocked\` return with` and replace it with `   only when the return says \`blocked\`, \`failed\` or \`verdict=decision\`. A \`blocked\` return with`.
7. § No teammates, line 109. Find `with a running one. Resume a finished agent only for a fix it can make` and replace it with `with a running one. Never resume a finished agent: review fixes go to a fresh`. Line 110: find `without re-reading; otherwise start a fresh one with the report as its` and replace it with `packet-coder (§ 4), and any other follow-up is a fresh agent with the report as its`.
8. Splice § 4 (C16): target `<wt>\.claude\skills\campaign-packet\SKILL.md`, block `skill-s4`, from `## 4. Review`, to `## 5. State on disk at every wave boundary`. This replaces the old § 4 and its "Sessions, passes and escalation" subsection.
9. § 6. Find the line `- \`docs/gap-analysis.md\`, \`docs/gap-analysis/\` and \`docs/project-status.md\`` and insert this line directly above it:
   ```text
   - Before close-out, run the campaign review loop (§ 4). Its fixes ship as one PR before the close-out packet.
   ```
10. § 6. Find `(\`rm-worktree.ps1 --merged\`), empties` and replace it with `(\`rm-worktree.ps1 --merged\`, which skips packet worktrees merged only into an integration branch: list those for the owner), empties`.
11. `.claude/skills/ship-pr/SKILL.md` line 92. Find:
    ```text
    For a campaign packet, the wave's `packet-review-fixer` pass is this review: it did not write the code and reviews adversarially, so don't run a second reviewer.
    ```
    Replace with:
    ```text
    For a campaign packet, the packet's `packet-reviewer` loop is this review ([campaign-packet skill](../campaign-packet/SKILL.md) § 4): its final `ship` came from an agent that did not write the code, so don't run a second reviewer.
    ```
12. R7: run sync. It rewrites `.agents/skills/campaign-packet/SKILL.md` and `.agents/skills/ship-pr/SKILL.md` only.

**Test type:** none (skill text). AH-22 guards the agent names and the link to `packet-reviewer.md`.

**Checks:**

- R11 on both skills and both mirrors. The one allowed exception: none (the legacy sentence says "review-and-fix", which does not match).
- `Select-String` the skill for `## 4. Review loops, then ship`, `### Sessions, passes and escalation` and `## 5. State on disk`: one line each, in that order.
- `[IO.File]::ReadAllLines('<wt>\.claude\skills\campaign-packet\SKILL.md').Length` is at most 260.
- R7's `--check`; R9 (the § 4 splice is the one large hunk).

**Docs owed:** doc-update-map row "AI-harness wiring" (the campaign cycle); list it.

---

## AH-19: the orchestrator persona and AGENTS.md dispatch the review loops

Agent: packet-coder. C15 row AH-19. Size: about 15k working tokens. Commit subject: `docs(agents): orchestrator persona and AGENTS.md run the review loops (AH-19)`.

**This packet edits the default persona.** The PR body calls it out for the owner (w4a-pr-body.md), and running sessions keep the old persona until they are relaunched.

**Rules:** R1 to R11. Read C12 and C14 first.

**Steps** (file `.claude/agents/orchestrator.md` unless named):

1. Line 29. Find `- \`packet-coder\` (Haiku): prescriptive packets, one per worktree.` and replace it with `- \`packet-coder\` (Haiku): prescriptive packets, one per worktree, and each review fix pass.`
2. Line 30. Find:
   ```text
   - `packet-review-fixer` (Sonnet): one per wave, reviews every packet in it.
   ```
   Replace with:
   ```text
   - `packet-reviewer` (Sonnet): read-only review of one packet, one wave or the campaign; writes fix notes. A fresh `packet-coder` applies them.
   ```
3. Line 31. Find `If that fails, \`architect\` re-slices. Review findings never go back to a coder.` and replace it with `If that fails, \`architect\` re-slices. Fix notes never go back into the session that wrote the code: each fix pass is a fresh \`packet-coder\`.`
4. Line 32: replace the whole line. Find:
   ```text
   - Review passes (at most 3 per packet): on a `recheck` verdict, dispatch a fresh `packet-review-fixer` for that packet alone. After pass 3 without `ship`, or on `decision`, dispatch `architect` for a ruling note in `worknotes/<packet>.md`, then a fresh `packet-review-fixer` to apply it. An owner call is BlockedDecision.
   ```
   Replace with:
   ```text
   - Review loops (campaign-packet skill § 4; at most 3 passes each): dispatch a fresh `packet-reviewer` for a packet as soon as its coder returns `done`, for a wave once every packet in it has `ship`, and for the campaign after its last merge. On `fix` at pass 1 or 2, dispatch a fresh `packet-coder` with "review fix, pass N of 3" and the report path, then a fresh reviewer for the next pass. On `fix` at pass 3, or on `decision`, dispatch `architect` for a ruling note, then one Sonnet implementer to apply it, then one fresh reviewer to check it. The architect decides only what to fix and how; it writes no code or fix. An owner call is BlockedDecision.
   ```
5. Line 43. Find `Keep briefs under about 25 lines.` and replace it with `A reviewer's brief also carries the scope (packet, wave or campaign), the commit range, the pass number and, from pass 2, the previous report's path. Keep briefs under about 25 lines.`
6. Line 49: replace the whole line. Find:
   ```text
   - A review-fixer return ends with `verdicts=<packet>:<ship|recheck|decision>,...`. Act on `ship` and `recheck` from the return line; open the report only for `decision`.
   ```
   Replace with:
   ```text
   - A `packet-reviewer` return ends with `verdict=<ship|fix|decision>` (a wave review: `verdicts=<packet>:<ship|fix|decision>,...`). Act on `ship` and `fix` from the return line, passing the report path to the next coder unread; open the report only for `decision` or `blocked`.
   ```
7. Line 53. Find `Never use SendMessage to keep an agent alive. Resume a finished agent only for a fix it can make without re-reading.` and replace it with `Never use SendMessage to keep an agent alive, and never resume a finished agent: review fixes go to a fresh \`packet-coder\`.`
8. `AGENTS.md` line 34: replace the whole line. Find:
   ```text
   4. Review each wave with one `packet-review-fixer` run that covers every packet in the wave. It returns `ship`, `recheck` or `decision` per packet; a `recheck` gets a fresh review-fixer for that packet, up to 3 passes. A coder that stops on context gets a fresh coder that continues from its worknote, up to 3 sessions, then one Sonnet implementer. The full ladder is in the campaign-packet skill.
   ```
   Replace with:
   ```text
   4. Review each packet with a fresh, read-only `packet-reviewer` as soon as its coder is done. It returns `ship`, `fix` or `decision`; on `fix`, a fresh `packet-coder` applies its numbered fix notes and a fresh reviewer checks again, up to 3 passes. The same loop runs once over each finished wave and once over the finished campaign. A coder that stops on context gets a fresh coder that continues from its worknote, up to 3 sessions, then one Sonnet implementer. The full ladder is in the campaign-packet skill.
   ```
9. R7: run sync. It rewrites `.codex/agents/orchestrator.toml` only.

**Test type:** none (persona text).

**Checks:** R11 on `orchestrator.md`, `orchestrator.toml` and `AGENTS.md`. `Select-String -Path <wt>\.claude\agents\orchestrator.md -Pattern 'writes no code or fix' -SimpleMatch` returns one line. `[IO.File]::ReadAllLines('<wt>\.claude\agents\orchestrator.md').Length` prints `72`, as before (every edit replaces one line with one line). R7's `--check`; R9.

**Docs owed:** doc-update-map row "AI-harness wiring" (the campaign cycle, AGENTS.md); list it.

---

## AH-20: development-workflow and rules-and-gotchas follow the review loops

Agent: packet-coder. C15 row AH-20. Size: about 25k working tokens. Commit subject: `docs(agents): development-workflow and rules-and-gotchas follow the review loops (AH-20)`.

**Rules:** R1 to R11. Both files are CRLF. Line numbers below are from `origin/main` before W3c; AH-11h may have moved them by a line or two, so always find by text (R6).

**Steps, `docs/agents/development-workflow.md`:**

1. Line 3: replace the date on the `> **Last updated**:` line with today's (`Get-Date -Format yyyy-MM-dd`). Read line 3 first; AH-11h may have changed it.
2. Find `24 subagents` and replace it with `23 subagents`.
3. Find `` refuses an `Edit` or `Write` outside `docs/analysis/**` and `.claude/state/**`, `` and replace it with `` refuses an `Edit` or `Write` outside `docs/analysis/**`, `.claude/state/**` and `.claude/reports/**` (in the main checkout or a campaign worktree), `` (F44).
4. Delete the roster row for the retired agent with this one-liner (it must print `status=ok`):
   ```text
   Set-Location <wt>; $f='<wt>\docs\agents\development-workflow.md'; $t=[IO.File]::ReadAllText($f); $old='| Reviewing a whole wave of packets and fixing every verified finding (Sonnet) | `packet-review-fixer` |'+"`r`n"; if(([regex]::Matches($t,[regex]::Escape($old))).Count -ne 1){throw 'row'}; [IO.File]::WriteAllText($f,$t.Replace($old,'')); 'status=ok'
   ```
5. Find:
   ```text
   | Adversarial read-only review of one finished packet (Sonnet) | `packet-reviewer` |
   ```
   Replace with:
   ```text
   | Adversarial read-only review of one packet, one wave or a whole campaign, with numbered fix notes (Sonnet) | `packet-reviewer` |
   ```
6. Find `one packet-coder per worktree, one review-fixer per wave,` and replace it with `one packet-coder per worktree, read-only review loops per packet, wave and campaign,`.
7. The bullet that starts `- **One phase per worker, and one-shot agents.**`: replace the whole line. Find:
   ```text
   - **One phase per worker, and one-shot agents.** A `packet-coder` implements one packet, commits locally and stops. One `packet-review-fixer` then reviews every packet of the wave and fixes what it verifies, in each packet's worktree; the orchestrator never fixes findings itself, and there is no separate read-only review followed by a fix worker. `ops` ships. Review findings never go back to the implementing worker, and no agent is kept alive as a teammate with `SendMessage`.
   ```
   Replace with:
   ```text
   - **One phase per worker, and one-shot agents.** A `packet-coder` implements one packet, commits locally and stops. A fresh, read-only `packet-reviewer` then reviews it and writes numbered fix notes, and a fresh `packet-coder` applies them; the same loop runs once per wave and once per campaign ([campaign-packet skill § 4](../../.claude/skills/campaign-packet/SKILL.md#4-review-loops-then-ship)). The orchestrator never fixes findings itself, and fix notes never go back into the session that wrote the code. `ops` ships. No agent is kept alive as a teammate with `SendMessage`.
   ```
8. Find `` The review-fixer returns `ship`, `recheck` or `decision` per packet, and a packet gets at most 3 review passes before the architect rules. `` and replace it with `` Each review pass returns `ship`, `fix` or `decision`, and each review loop (packet, wave or campaign) gets at most 3 passes before the architect rules. ``
9. Find `` The orchestrator updates the ledger row from that line and opens the report only on `blocked` or `failed`. `` and replace it with `` The orchestrator updates the ledger row from that line and opens the report only on `blocked`, `failed` or a reviewer's `decision`; a reviewer's line also ends with `verdict=<ship|fix|decision>`. ``
10. Find `so one review-fixer can hold a wave.` and replace it with `so one packet-reviewer can hold a wave at the wave review.`
11. Find `the worker (or the review-fixer) writes` and replace it with `the worker (or the fix session) writes`.
12. Fallback for AH-11h: if the text `` (`packet-coder`, `packet-review-fixer`) `` is in the file, replace it with `` (`packet-coder`, `packet-reviewer`) ``. If it is not there, skip this step and say so.

**Steps, `docs/agents/rules-and-gotchas.md`:**

13. Line 3: today's date, as in step 1.
14. Find `Haiku implements prescriptive packets, Sonnet reviews and fixes, Opus designs and rules but writes no code.` and replace it with `Haiku implements prescriptive packets and applies review fix notes, Sonnet reviews without editing, Opus designs and rules but writes no code.`
15. Find `For a campaign packet that is the wave's packet-review-fixer; otherwise` and replace it with `For a campaign packet that is its \`packet-reviewer\` loop ([campaign-packet skill § 4](../../.claude/skills/campaign-packet/SKILL.md#4-review-loops-then-ship)); otherwise`.

**Test type:** none (docs only).

**Checks:** R4 on both files. R11 on both files. `Select-String` development-workflow.md for `23 subagents` and `.claude/reports/**`: one line each. R9.

**Docs owed:** doc-update-map rows "AI-harness configuration" (the workflow and roster; a project rule) and "AI-harness wiring" (the cycle prose); list both.

---

## AH-21: ai-harness cycle and the README summary follow the review loops

Agent: packet-coder. C15 row AH-21. Size: about 20k working tokens. Commit subject: `docs(agents): ai-harness cycle and the README summary follow the review loops (AH-21)`.

**Rules:** R1 to R11, but R11 allows exactly **one** `review-fixer` line in `docs/agents/ai-harness.md`: the row inside its `harness-agents` generated block, which AH-22's regen removes (R5: don't touch it). Both files are CRLF. README.md holds two generated blocks (`harness-hooks` and the crate graph); edit neither.

**Steps:**

1. `docs/agents/ai-harness.md` line 3: today's date on the `> **Last updated**:` line.
2. Find `- **The in-repo adversarial review** is the packet-review-fixer, or the ship-pr §5 reviewer.` and replace it with `- **The in-repo adversarial review** is the \`packet-reviewer\` loop for a campaign packet, or the ship-pr §5 reviewer for anything else.`
3. Splice (C16): target `<wt>\docs\agents\ai-harness.md`, block `ah-cycle`, from `## Cycle and escalation`, to `## Keeping this page current`.
4. `README.md` line 64. Find `` 5. `packet-review-fixer` (Sonnet) reviews and fixes each wave, with at most 3 passes. `` and replace it with:
   ```text
   5. `packet-reviewer` (Sonnet) reviews each packet, then each wave, then the campaign, without editing; a fresh `packet-coder` applies its fix notes, with at most 3 passes per loop.
   ```
5. `README.md` line 70. Find:
   ```text
   - **Reviewer:** the verdict is `ship`, `recheck` or `decision`, with at most 3 passes. Then `architect` writes a ruling note with no code, and a fresh Sonnet review-fixer applies it. An owner call is BlockedDecision.
   ```
   Replace with:
   ```text
   - **Reviewer:** the verdict is `ship`, `fix` or `decision`, with at most 3 passes per loop. Then `architect` decides only what to fix and how, in a ruling note, and writes no code or fix; a Sonnet implementer applies it, and a fresh reviewer checks it. An owner call is BlockedDecision.
   ```

**Test type:** none (docs only).

**Checks:**

- R4 on both files.
- `(Select-String -Path <wt>\docs\agents\ai-harness.md -Pattern 'review-fixer' -SimpleMatch).Count` prints `1`, and that line starts `| packet-review-fixer |`. `recheck`: 0 lines in both files. `review-fixer`: 0 lines in README.md.
- `git -C <wt> diff -- README.md` touches only lines 64 and 70. `git -C <wt> diff -- docs/agents/ai-harness.md` touches nothing between `<!-- gen:` and `<!-- /gen:` lines.
- `Select-String -Path <wt>\docs\agents\ai-harness.md -Pattern 'writes no code or fix' -SimpleMatch` returns one line (the `ah-cycle` block). (They resolve once AH-18 is merged; the campaign-metrics anchor resolves now.)

**Docs owed:** doc-update-map row "AI-harness wiring" (the prose in ai-harness.md); list it.

---

## AH-22: drift test for agent names in the harness docs

Agent: packet-coder, in the integration worktree `ah-w4a` (branch `ai-harness-docs/w4a`) after AH-17 to AH-21 are merged into it. Size: about 30k working tokens. Commit subject: `test(agent-skills): harness docs name only agents that exist (AH-22)`.

**Rules:** R1 to R10. This packet alone runs `regen.py` (R5), and only to refresh one block.

**Files:** new `tools/agent-skills/test_agent_refs.py`; `tools/agent-skills/README.md`; `.github/workflows/agent-skills.yml` (LF); `docs/agents/doc-update-map.md`; `docs/agents/ai-harness.md` (only what regen writes in its `harness-agents` block).

**Steps:**

1. Write `tools/agent-skills/test_agent_refs.py` (stdlib only, about 60 lines) with this shape:
   ```text
   """Agent names in the harness docs match the agent files (ai-harness-docs D-AH42, D-AH47)."""
   import re
   import unittest
   from pathlib import Path

   ROOT = Path(__file__).resolve().parents[2]
   AGENTS = ROOT / ".claude" / "agents"
   NAME = re.compile(r"`(packet-[a-z-]+)`")
   LINK = re.compile(r"\]\(([^)\s#]+\.md)(?:#[^)]*)?\)")


   def scanned():
       files = sorted(AGENTS.glob("*.md")) + sorted((ROOT / ".claude" / "skills").glob("*/SKILL.md"))
       files += [ROOT / n for n in ("AGENTS.md", "README.md", "CLAUDE.md")]
       files += sorted((ROOT / "docs" / "agents").glob("*.md"))
       return [f for f in files if f.is_file()]
   ```
   and one `unittest.TestCase` with four tests:
   - `test_scans_enough_files`: `len(scanned()) >= 20`.
   - `test_backticked_agent_names_exist`: for every file and every `NAME` match, with `self.subTest(file=..., name=...)`, assert `(AGENTS / f"{name}.md").is_file()`.
   - `test_agent_links_exist`: for every `LINK` match that does not contain `://`, resolve it against the file's folder; when the resolved parent is `AGENTS`, assert the file exists.
   - `test_retired_review_fixer_is_gone`: `"review-fixer"` is not in each file's lowercased text, except inside generated blocks: first remove every span matching `(?s)<!-- gen:[a-z-]+ -->.*?<!-- /gen:[a-z-]+ -->`. The failure message: `"<file> names the retired packet-review-fixer (ai-harness-docs D-AH42); describe the packet-reviewer loop instead"`.
   Read files with `encoding="utf-8"`.
2. `tools/agent-skills/README.md` line 14. Find `` , and `test_shell_guard.py` tests the `PreToolUse` shell guard in `.claude/hooks/shell_guard.py`. `` and replace it with `` , `test_shell_guard.py` tests the `PreToolUse` shell guard in `.claude/hooks/shell_guard.py`, and `test_agent_refs.py` checks that the harness docs name only agents that exist. ``
3. `.github/workflows/agent-skills.yml`: in **both** `paths:` lists (under `pull_request:` and under `push:`), add the four paths `docs/agents/**`, `README.md`, `AGENTS.md` and `CLAUDE.md` directly after the `- "tools/agent-skills/**"` line, with the same indentation, so a docs-only PR that names an agent runs the test. That line is in the file twice, once per list, by design: this step is the one exception to R6, and edits both with this one-liner (it must print `status=ok`), not with the Edit tool:
   ```text
   Set-Location <wt>; $f='<wt>\.github\workflows\agent-skills.yml'; $t=[IO.File]::ReadAllText($f); $nl= if($t.Contains("`r`n")){"`r`n"}else{"`n"}; $old='      - "tools/agent-skills/**"'+$nl; if(([regex]::Matches($t,[regex]::Escape($old))).Count -ne 2){throw 'count'}; $new=$old+(('      - "docs/agents/**"','      - "README.md"','      - "AGENTS.md"','      - "CLAUDE.md"') -join $nl)+$nl; [IO.File]::WriteAllText($f,$t.Replace($old,$new)); 'status=ok'
   ```
   Then `git -C <wt> diff --numstat -- .github/workflows/agent-skills.yml` prints `8	0` (8 lines added, none removed).
4. `docs/agents/doc-update-map.md`, the "AI-harness wiring" row. Find `` (`test_harness_tables.py` fails without one). `` and replace it with `` (`test_harness_tables.py` fails without one). A doc that names a `packet-*` agent in backticks, or links one, needs its agent file (`test_agent_refs.py`). ``
5. Regenerate one block: `Set-Location <wt>; python tools/docs-gen/regen.py --skip-crate-graph`. Then `git -C <wt> status --porcelain`, and for **every** modified file other than `docs/agents/ai-harness.md` and the files of steps 1-4, run `git -C <wt> restore -- <file>`. Then `git -C <wt> diff -- docs/agents/ai-harness.md` must change only lines inside the `harness-agents` block: the `packet-review-fixer` row gone, and the `packet-coder` and `packet-reviewer` descriptions updated.

**Test type:** unit (`unittest`, per TESTING.md), a documentation drift guard. It fails on revert because the old `architect.md` names `packet-review-fixer`, which has no agent file.

**Checks, in order:**

1. `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_agent_refs.py" -v`: 4 tests, OK.
2. `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK (the CI command).
3. `Set-Location <wt>; python tools/agent-skills/sync.py --check`: exit 0.
4. `Set-Location <wt>; python tools/docs-gen/regen.py --check --skip-crate-graph`: it must not list `docs/agents/ai-harness.md` or `README.md`. (Other files may be listed; main's regen-docs workflow owns them.)
5. **Revert proof** (one call; it must print `reverted-fails` and then `restored`):
   ```text
   Set-Location <wt>; $f='<wt>\.claude\agents\architect.md'; $b=[IO.File]::ReadAllBytes($f); $base=git -C <wt> merge-base HEAD origin/main; git -C <wt> show "${base}:.claude/agents/architect.md" | Set-Content -LiteralPath $f -Encoding utf8NoBOM; python -m unittest discover -s tools/agent-skills -p "test_agent_refs.py" 2>$null; if($LASTEXITCODE -ne 0){'reverted-fails'}else{'reverted-PASSES-bad'}; [IO.File]::WriteAllBytes($f,$b); if(-not (git -C <wt> status --short -- .claude/agents/architect.md)){'restored'}
   ```
6. R4 on doc-update-map.md and ai-harness.md.

**Docs owed:** doc-update-map row "AI-harness wiring" (this packet edits it); list it.

---

## W4a ship procedure

1. **Worktrees.** `ops` makes `ah-17` to `ah-21` (C15), one dispatch each, in one message, after W3c has merged (dispatch rule 3).
2. **Coders and packet loops.** Dispatch AH-17 to AH-21 in one message, in the background. As each returns `done`, run its packet loop (C12) in its own worktree. A packet with `ship` waits for the rest.
3. **Integration worktree.** `ops`: `pwsh tools/build-lane/mk-worktree.ps1 ai-harness-docs/w4a ah-w4a`. Then five `ops` dispatches, **one after another** (each changes the branch the next one merges into): `pwsh -NoProfile -File tools/build-lane/merge-noff.ps1 -C <ah-w4a abs path> -Branch ai-harness-docs/ah-<NN>-<slug> -ExpectBranch ai-harness-docs/w4a`, in the C15 order. A conflict means the files were not disjoint: stop, and dispatch `architect`.
4. **AH-22.** A packet-coder in `ah-w4a`, then its packet loop there (its range is the AH-22 commit alone: `HEAD~1..HEAD` when it is one commit).
5. **Wave loop.** A `packet-reviewer` at wave scope over `ah-w4a`, range `<merge-base with origin/main>..HEAD`, spec this file and the ledger README, report `wave4a-review-<N>.md`. The scope note for the brief: "contract drift between AH-17 to AH-22: the verdict words, the return-line formats, the report paths and the pass limits must read the same in every file; and every place that still names the retired agent". On `fix`, one fresh `packet-coder` in `ah-w4a` applies all the notes (C12).
6. **Ship.** `ops`: `pwsh tools/build-lane/ship.ps1 pr -C <ah-w4a abs path> -m "AH-W4a: review loops replace packet-review-fixer" --title "AI harness: read-only review loops replace packet-review-fixer (ai-harness-docs W4a)" --body-file <ah-w4a abs path>\docs\analysis\ai-harness-docs\w4a-pr-body.md`. It is a code PR (a `.py` and a workflow), so the merge waits for CI. Then `ship.ps1 merge <PR> --retire ah-w4a`.
7. **Retire the packet worktrees.** `ah-17` to `ah-21` were merged only into `ai-harness-docs/w4a`, so `rm-worktree.ps1 --merged` skips them, and `ops` never passes `--force` (F43). If S5 (S5a to S5c) has merged by then, `ops` runs `pwsh tools/build-lane/rm-worktree.ps1 --merged-into <W4a PR> ah-17 ah-18 ah-19 ah-20 ah-21` instead. Otherwise list them for the owner, who runs `pwsh tools/build-lane/rm-worktree.ps1 <name> --force` for each once the W4a PR has merged.
8. **Ledger.** Set AH-17 to AH-22 to Integrated (the W4a PR number), rewrite RESUME.md, and tell the user to relaunch Claude Code (dispatch rule 9).
