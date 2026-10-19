# ai-harness-docs: work packets, wave 4b, wave 5 and the spicy packets

Continued from [work-packets-w4.md](work-packets-w4.md): its dispatch rules (rule 0 is the main-red gate), **§ W4 rules (R1 to R11)** and contract **C12 to C16** apply to every packet here. A packet-coder reads § W4 rules, the contract items its packet names, and its own section below. The verbatim W4b blocks are in [w4b-drafts.md](w4b-drafts.md). Ledger: [README.md](README.md). Rewritten 2026-10-18 for the owner's answers ([w4-owner-answers-report.md](w4-owner-answers-report.md), D-AH50 to D-AH56).

## Order (D-AH56)

Each step waits for the merges it names. No merge happens while `main` is red (dispatch rule 0, D-AH55).

1. **AH-W40**: the ledger, before any worktree below is cut.
2. **W3c**: AH-11g and AH-11h (AH-11p merged as #1850). Then **AH-11i** (owner: close #1797).
3. **W4a** (AH-17 to AH-22), one PR from `ah-w4a`.
4. In parallel after W4a, each its own PR, all file-disjoint:
   - **W4b**: AH-23a then AH-23b, one PR from `ah-23`.
   - **S6**: after W4a (it edits AH-22's README line).
   - **S5a** and **S5b**: after W4a and AH-11g; the two in parallel.
   - **S4** (S4a to S4d, sequential in one worktree `ah-s4`, one PR; [work-packets-s4.md](work-packets-s4.md)): after W4a (AH-17 puts the fix-pass brief text in packet-coder.md). It touches only `tools/token-profile/` and `docs/guides/token-profiling.md`.
5. **W4c** (AH-25 to AH-27, in [work-packets-w4c.md](work-packets-w4c.md)): after W3c, W4a **and W4b** have merged, because AH-27 edits `docs/agents/ai-sdlc.md`, which AH-23b creates. W4c may run beside S3, S5 and S6: they touch other sections of the same files (S3 ai-sdlc's Further reading, S5c campaign-packet § 6 and the development-workflow code block, S6 the agent-skills README line that AH-25 step 8 finds either way), so whichever merges second rebases with no overlap.
6. **S5c**: after S5a, S5b and AH-11g have merged (it names both scripts' new option in ops.md).
7. **S3**: after W4b (it links from ai-sdlc.md).
8. **W5**: AH-24 after W4b, W4c, AH-11i, S3, S5c, S6 and S4 have merged; then AH-15 (owner) and AH-16 (close-out).

**Dropped:** S1 (owner veto, 2026-10-18, D-AH57). **Approved:** S4 (D-AH58). S2 is folded into AH-23a. No spicy packet is BlockedDecision any more.

## Shared contract for this file

### C16b. The W4b splice and extraction

C16 of work-packets-w4.md, character for character, with one change: the drafts file is `<wt>\docs\analysis\ai-harness-docs\w4b-drafts.md`, not `w4-drafts.md`. In both one-liners, replace `w4-drafts.md` with `w4b-drafts.md`; change nothing else. Anything other than `status=ok` is a stop: return `blocked` with the error text.

---

## Wave 4b: the README AI section and the deeper reference (D-AH50)

The owner's answer: the README may be as long as it needs (the 500-line soft cap does not bind README.md), and the newcomer explanation lives in the README. `docs/agents/ai-sdlc.md` becomes the deeper reference behind it. Two packets, one worktree, one PR, so a wave loop runs over both (C12).

### AH-23a: the README section

Agent: packet-coder. Branch `ai-harness-docs/ah-23-ai-sdlc`, worktree `ah-23` (`ops` makes it from `origin/main` after W4a merged). Size: about 20k working tokens. Commit subject: `docs: README explains how AI development works here, for newcomers (AH-23a)`.

**Rules:** R1 to R10, and R11 on README.md. README.md is CRLF. It holds the `harness-hooks` generated block and the crate-graph block; the splices stop at the first and never reach the second (R5). No regen in this packet. README.md has **no line cap** (D-AH50); every other file keeps R8.

**Files:** `README.md` only.

**Steps:**

1. Splice A (C16b): target `<wt>\README.md`, block `readme-ai-a`, from `## How AI development works here`, and for `<to>` write `$to='<!-- gen'+':harness-hooks -->'` in place of the quoted value (the split keeps a whole marker out of the packet text).
2. Splice B (C16b): target `<wt>\README.md`, block `readme-ai-b`, from `**Everything else:**`, to `**Platform:**`.

The `readme-ai-a` block already holds the loop diagram (a Mermaid `flowchart` under `### Review loops`): that is S2, folded in (D-AH52).

**Test type:** none (docs). AH-22's `test_agent_refs.py` scans README.md.

**Checks:**

- R4 on README.md.
- `git -C <wt> diff -- README.md` shows no line changed between `<!-- gen:harness-hooks -->` and its closing marker, and none from `## Quick Start` down.
- `Select-String -Path <wt>\README.md -Pattern '## How AI development works here','**Everything else:**','**Platform:**' -SimpleMatch`: one line each, in that order.
- `Select-String -Path <wt>\README.md -Pattern 'writes no code or fix' -SimpleMatch`: one line. `-Pattern '```mermaid' -SimpleMatch`: one line.
- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK.

Its links to `docs/agents/ai-sdlc.md` resolve only after AH-23b; AH-23b checks them.

### AH-23b: the deeper reference and its links

Agent: packet-coder, in the same worktree `ah-23`, after AH-23a has committed (sequential: AH-23b's checks read AH-23a's README). Size: about 25k. Commit subject: `docs(agents): How we build with AI, the reference behind the README's AI section (AH-23b)`.

**Rules:** R1 to R10, and R11 on every file you edit. All targets are CRLF.

**Files:** new `docs/agents/ai-sdlc.md`; `docs/readme.md`; `AGENTS.md`; `docs/agents/development-workflow.md`; `docs/agents/ai-harness.md` (line 7 only); `docs/agents/doc-update-map.md`; `docs/agents/rules-and-gotchas.md`; `README.md` only as regen rewrites its docs count.

**Steps:**

1. Extraction (C16b): target `<wt>\docs\agents\ai-sdlc.md`, block `ai-sdlc`. Then Edit that file: replace `> **Last updated**: YYYY-MM-DD` with `> **Last updated**: ` followed by today's date (`Get-Date -Format yyyy-MM-dd`).
2. `docs/readme.md`: find the table row that starts `| [development-workflow.md](agents/development-workflow.md) |` and insert this row directly above it:
   ```text
   | [ai-sdlc.md](agents/ai-sdlc.md) | How we build with AI: the reference behind the README's AI section (campaign files, packets, tools and models, review-loop detail, guards in detail, shipping, honest numbers) | Complete |
   ```
3. `AGENTS.md` line 5. Find `[README § How AI development works here](README.md#how-ai-development-works-here) and [docs/agents/ai-harness.md](docs/agents/ai-harness.md).` and replace it with `[README § How AI development works here](README.md#how-ai-development-works-here), its deeper reference [docs/agents/ai-sdlc.md](docs/agents/ai-sdlc.md), and [docs/agents/ai-harness.md](docs/agents/ai-harness.md).`
4. `docs/agents/development-workflow.md` line 7. Find `or by hand.` (the end of the paragraph that starts `How a change moves from ticket to merged PR`) and replace it with `or by hand. New to all of it? Start with the [README's AI section](../../README.md#how-ai-development-works-here), then [How we build with AI](ai-sdlc.md).` If `or by hand.` matches more than once, stop (R6).
5. `docs/agents/ai-harness.md` line 7. Find `The workflow itself is in [development-workflow.md](development-workflow.md).` and replace it with `The workflow itself is in [development-workflow.md](development-workflow.md), and [How we build with AI](ai-sdlc.md) is the reference behind the README's AI section.`
6. `docs/agents/doc-update-map.md`, the "AI-harness wiring" row. Find `Update the prose in [ai-harness.md](ai-harness.md), ` and replace it with `Update the prose in [ai-harness.md](ai-harness.md), the reference in [ai-sdlc.md](ai-sdlc.md), `.
7. `docs/agents/rules-and-gotchas.md`: find the line that starts `- **Run agents as defined.**` and insert this line directly above it:
   ```text
   - **No hook refuses raw `git`.** Git is kept safe by tool lists (the orchestrator has no shell, the reviewer has no Edit), by `ops` running only its listed scripts, and by `ship.ps1` refusing an unregistered worktree. A worker brief still says never `git stash`, `git push`, `git worktree prune` or `gh pr`; don't assume a guard will stop one.
   ```
   Then set its line 3 date to today.
8. Generated counts: `Set-Location <wt>; python tools/docs-gen/regen.py --skip-crate-graph`. Keep what it changed in `docs/readme.md` and `README.md` (the docs count moves by one; regen owns those blocks). `git -C <wt> restore -- <file>` every other file it changed that is not in **Files**.

**Test type:** none new (docs). The AH-22 test scans `docs/agents/*.md`, so it covers the new doc.

**Checks:**

- R4 on all seven edited files and the new one.
- `[IO.File]::ReadAllLines('<wt>\docs\agents\ai-sdlc.md').Length` is at most 500.
- Every relative link in `ai-sdlc.md` resolves: for each `](<path>)` without `://`, `Test-Path` the path joined to `<wt>\docs\agents\` (strip any `#anchor`) prints `True`. Do the same for every `](docs/...)` link in README.md's `## How AI development works here` section, joined to `<wt>\`.
- Anchors: `Select-String` finds the heading behind each: `## How AI development works here` in README.md (for `#how-ai-development-works-here`), `## 4. Review loops, then ship` in the campaign-packet skill (for `#4-review-loops-then-ship`), and `## Guards in detail`, `## Honest numbers` and `## Platforms` in ai-sdlc.md.
- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK.
- `Set-Location <wt>; python tools/docs-gen/regen.py --check --skip-crate-graph` does not list README.md or docs/readme.md.

**Docs owed (both packets, one PR):** doc-update-map rows "AI-harness configuration" (a new doc in docs/agents and its row in docs/readme.md; a project rule) and "AI-harness wiring" (the README section); list both.

### W4b wave loop and ship

After both packet loops say `ship`, one wave pass (C12, wave row) runs over `origin/main..HEAD` in `ah-23`: does the README explain and ai-sdlc.md hold the detail, with no paragraph in both (w4b-drafts.md line 10)? Fix notes go to one fresh packet-coder in `ah-23`. Then `ops` ships it with `ship.ps1 pr -C <wt> --body-file <wt>\docs\analysis\ai-harness-docs\w4b-pr-body.md` and, after rule 0, `ship.ps1 merge <PR> --retire ah-23`. Docs-only.

---

## Wave 5: the campaign review, acceptance and close-out

### AH-24: the campaign review loop

Agents: `packet-reviewer` (campaign scope), then fresh `packet-coder`s for the fix passes (C12, campaign row). Worktree `ah-campaign-review`, branch `ai-harness-docs/campaign-review` (`ops` makes it from `origin/main` once W4b, W4c, AH-11i, S3, S5c, S6 and S4 have merged). Size: one review of about 60k (Sonnet, read-only), each fix pass about 30k.

**Reviewer brief additions** (on top of C14):

- Scope `campaign`. The range is not one commit range: list the campaign's squash merges with `git -C <wt> log --format="%h %s" origin/main --grep="AH-" --grep="ai-harness-docs" --since=2026-10-01` and review each one's diff (`git show --stat`, then the files).
- Spec: the ledger README (findings and decisions), work-packets-w4.md C12 to C14, work-packets-w4c.md C17, and readme-notes.md (the owner's binding notes).
- What the campaign review is for, long-reach issues only, which no packet or wave review could see:
  1. Two files that describe the same thing differently: the loop, the verdict words, the pass limits, the ruling rule ("writes no code or fix"), the return lines, the report paths, the platform rule and status, the guard numbers. The files: README.md, AGENTS.md, CLAUDE.md, docs/agents/ai-sdlc.md, ai-harness.md, development-workflow.md, rules-and-gotchas.md, the campaign-packet, ship-pr and lane-build skills, `.claude/agents/*.md`.
  2. A number in the docs that does not match its source (hook-timing.md, docs/analysis/campaign-metrics/findings.md, docs/analysis/token-profile*), or an unmeasured saving stated as measured.
  3. A doc that a reader of readme-notes.md would say does not do what the owner asked.
  4. A claim that something is fixed, tested or validated where nothing tests it. The Linux and WSL paths are "not validated" until AH-15's Linux leg runs.
- Report `campaign-review-<N>.md`.

**Fix passes:** a fresh packet-coder in `ah-campaign-review`, brief per C14 "fix pass", with the R-rules of work-packets-w4.md. A fix note that would touch a generated block becomes a `## Not fixed` item for AH-16's regen.

**Ship:** after `ship`, `ops` runs `ship.ps1 pr -C <wt> -m "AH-24: campaign review fixes"` and, after rule 0, `ship.ps1 merge <PR> --retire ah-campaign-review`. If pass 1 says `ship` with no fix notes, there is nothing to ship: `ops` retires the worktree (`rm-worktree.ps1 ah-campaign-review`; a branch with no commits is ON_MAIN).

### AH-15 addenda (owner acceptance; add to work-packets-w3.md § AH-15)

Three more Windows steps, after step 5 and before recording:

- W8. Open the README on GitHub in a private window, and read only § How AI development works here.
  - **Expect** to be able to answer, without other docs: what a campaign, a wave and a packet are; who reviews a packet and who fixes it; how many review passes there are and what happens after the last (the architect rules and writes no code or fix); which commands are refused and why; where a human has to decide. Check that the Mermaid diagram renders. Record any question the section left open, and whether ai-sdlc.md answered it.
- W9. In the step 2 session, run `/agents`.
  - **Expect** `packet-reviewer` and `packet-coder`, and no `packet-review-fixer`.
- W10. Ask the orchestrator: "How does review work here?"
  - **Expect** the read-only packet-reviewer loop, a fresh packet-coder applying the fix notes, at most 3 passes, an architect ruling with no code after that, and the wave and campaign loops. Any mention of a review-fixer is a fail (an old persona is loaded: relaunch, then retry once).

The Linux leg (L1 to L6) now also proves C17: L6 uses a `Host: linux` packet (work-packets-w3.md § AH-15, amended 2026-10-18). Whether that leg is required stays D-AH33.

### AH-16 addenda (add to work-packets-w3.md § AH-16)

- Step 2's project-status line becomes: `AI harness: mode banner, py.sh hook launcher, worktree-aware orchestrator guard, generated hook and agent tables, a Bash path for ops and packet workers on Linux and WSL (Windows stays PowerShell), read-only review loops per packet, wave and campaign, and a README that explains how AI development works here (ai-harness-docs, 2026-10).`
- Step 1's unified-uat entry includes W8 to W10.
- The retire list: `ops` retires the `ah-*` worktrees with `rm-worktree.ps1 --merged`. That skips the packet worktrees merged only into an integration branch (F43): `ah-17` to `ah-21` (W4a) and `ah-25` to `ah-27` (W4c). `ops` retires those with `rm-worktree.ps1 --merged-into <W4a PR> ah-17 ah-18 ah-19 ah-20 ah-21` and `rm-worktree.ps1 --merged-into <W4c PR> ah-25 ah-26 ah-27` (S5, merged before W5). Any it still skips go to the owner, with the skip line.
- Step 4 also sets S1 to "Not done: the owner vetoed it, 2026-10-18 (D-AH57)", S2 to Done with "folded into AH-23a", and S4 to Done with its PR number (or "Not done" with the reason, if it has not merged).
- AH-24's review also lists S4's "Not done" items (work-packets-s4.md § Not done) as follow-ups.

---

## Spicy packets (D-AH52, D-AH57, D-AH58)

Approved 2026-10-18: S2 (folded into AH-23a), S3, S5 (expanded below into S5a, S5b and S5c), S6 and, later that day, S4 (expanded into S4a to S4d in [work-packets-s4.md](work-packets-s4.md)). **Vetoed:** S1. Each approved packet is its own PR and changes nothing in the base packets.

### S1: shell_guard refuses `git worktree prune` and `git stash` (vetoed)

Dropped: the owner vetoed it on 2026-10-18 (D-AH57). No packet, worktree or PR. The briefs and the tool lists stay the only stop for those two commands, and AH-23b's rules bullet "No hook refuses raw git" stays as written. The packet text is in git history (this file before the veto), in case the owner reopens it.

### S2: the loop diagram (approved; folded into AH-23a)

The Mermaid diagram is part of the `readme-ai-a` block, under `### Review loops`, so AH-23a lands it. No separate packet, worktree or PR. The owner checks the rendering at AH-15 W8 (no local check).

### S3: "a day in the life" of one real packet (approved)

Agent: **documentation-writer** (Sonnet), not packet-coder: it chooses what to quote. Branch `ai-harness-docs/s3-day-in-the-life`, worktree `ah-s3` (`ops` makes it from `origin/main` after W4b merged). Size: about 35k.

**Brief:** write `docs/agents/ai-sdlc-example.md` (at most 250 lines, CRLF, Explanation type, the same header shape as ai-sdlc.md): one packet followed from design to merge, with AH-11m (PR #1842) as the worked example. Sources only: this ledger (README.md rows and decisions for AH-11m, work-packets-w3d.md § AH-11m), `gh pr view 1842 --json title,body,commits,mergedAt`, and the PR's diff. Show the real brief, the real one-line return, the review verdict and what changed, and the merge. Mark anything not in those sources as not recorded; invent nothing (no costs unless the ledger states them). No account names. Link it with one bullet each from:

- ai-sdlc.md § Further reading;
- the README's `**Everything else:**` list (one bullet, outside every generated block);
- docs/readme.md, a row directly under the ai-sdlc row.

Then `python tools/docs-gen/regen.py --skip-crate-graph`, keeping only the docs-count changes in README.md and docs/readme.md.

**Checks:** R4; every relative link resolves; the AH-22 test passes; `regen.py --check --skip-crate-graph` does not list README.md or docs/readme.md. **Docs owed:** doc-update-map row "AI-harness configuration" (a new doc and its docs/readme.md row). Own PR; a wave of one, so no wave loop.

### S4: token-profile counts fix passes as `fixer` (approved; D-AH58)

The owner approved S4 on 2026-10-18. The open question (does the profile store the brief text?) is answered: no, but the ingest sees each subagent's first prompt, so it derives a fixed tag there and stores only the tag (F56). The design (contract C19) and the four prescriptive packets, S4a to S4d, are in [work-packets-s4.md](work-packets-s4.md): one worktree `ah-s4`, sequential, one PR, after W4a. The options (a) to (d) of the earlier draft are retired: the brief tag is option (d) without stored text.

### S5: `rm-worktree --merged-into <PR>` (approved; D-AH54)

**Why:** `rm-worktree --merged` skips a packet worktree whose branch merged only into an integration branch that then squash-merged into `main` (F43): `gh` finds no PR for the branch (`NO_PR`) and the branch is not on `origin/main`. Today the owner retires those with `--force`, because `ops` never passes `--force`. `--merged-into <PR>` lets `ops` retire them safely: only when the named PR has merged, and only when the branch's tip is in that PR's head.

**The design (both twins, identical behaviour):**

- New option `--merged-into <PR>`, with explicit names only: `rm-worktree.ps1 [--dry-run] --merged-into <PR> <name> [<name>...]`.
- Usage errors, exit 2: no value after it (`--merged-into needs a PR number`), a value that is not all digits (`--merged-into needs a PR number, got <v>`), combined with `--merged` (`--merged-into names worktrees; it does not combine with --merged`), no names (the existing usage line, which gains `[--merged-into <PR>]`).
- After the existing `git fetch -q origin`, when the option is set: `gh` must be on PATH, else `--merged-into needs gh`, exit 1. `gh pr view <PR> --json state -q .state` must print `MERGED`, else `PR <PR> is <state>, not merged; nothing retired`, exit 1. `git fetch -q origin pull/<PR>/head` must succeed, and `git rev-parse --verify -q FETCH_HEAD^{commit}` gives the PR's head; else `could not fetch PR <PR>'s head`, exit 1.
- Per worktree, after the existing merge-state lookup: when the option is set, the state is `NO_PR` or `UNMERGED`, the worktree is on a branch, and `git merge-base --is-ancestor <branch> <PR head>` succeeds, the state becomes `MERGED_INTO`. `MERGED_INTO` then counts like `MERGED` in the two places the scripts test for `MERGED` or `ON_MAIN`: the skip check and the branch delete.
- Unchanged: every other skip (not registered, building, locked, uncommitted changes) runs first, exactly as today, and `--merged-into` never implies `--force`. A branch with a commit past the PR head is skipped as `NO_PR` or `UNMERGED`. A branch that has its own PR keeps that PR's state.

S5a and S5b are file-disjoint and run in parallel; S5c follows both.

#### S5a: `rm-worktree.ps1 --merged-into`, with a Windows test

Agent: packet-coder. Branch `ai-harness-docs/s5a-merged-into-ps1`, worktree `ah-s5a` (from `origin/main` after W4a and AH-11g merged). Size: about 35k. Own PR. Commit subject: `feat(build-lane): rm-worktree.ps1 --merged-into retires packets merged into an integration branch (ai-harness-docs S5a)`.

**Rules:**

- R1, R2 (no cargo; this packet runs pwsh and git only), R4, R6, R8, R9, R10 of work-packets-w4.md.
- `tools/build-lane/*.ps1` are CRLF; the Edit tool keeps that.
- Every test repo, worktree and environment variable is temporary, under `[IO.Path]::GetTempPath()`. Never run the script against this checkout, never against a real PR, never against a real database. Restore every `$env:` value you set (`try`/`finally`).
- File caps: the test file at most 300 lines; rm-worktree.ps1 stays under 500.

**Files:** `tools/build-lane/rm-worktree.ps1`; new `tools/build-lane/test-rm-worktree.ps1`.

**Steps:**

1. Header. After line 6 (`#   pwsh tools/build-lane/rm-worktree.ps1 [--dry-run] [--prune] --merged`) insert `#   pwsh tools/build-lane/rm-worktree.ps1 [--dry-run] --merged-into <PR> <name> [<name>...]`. After the line that ends `then deletes Dev Drive target dirs no registered worktree owns.` insert the two lines `# --merged-into <PR> also retires a named worktree whose branch has no PR of its own but` and `# is in merged PR <PR>'s head (a packet merged only into an integration branch).`. Change `-TotalCount 23` to `-TotalCount 26`.
2. Arguments. Add `$MergedInto = ''` to the line that starts `$Dry = $false;`. Replace `foreach ($a in $args) {` with `for ($i = 0; $i -lt $args.Count; $i++) {` and, as its first statement, `$a = $args[$i]`. Add a case before the `-h` case: `'--merged-into' { $i++; if ($i -ge $args.Count) { [Console]::Error.WriteLine('--merged-into needs a PR number'); exit 2 }; $MergedInto = $args[$i]; continue }`.
3. Validation, directly after the arg loop and before the existing usage check: a non-empty `$MergedInto` that does not match `^[0-9]+$` prints `--merged-into needs a PR number, got <v>` and exits 2; `$MergedInto` with `$Merged` prints `--merged-into names worktrees; it does not combine with --merged` and exits 2. In the existing usage line, replace `[--prune] <name>... | --merged` with `[--prune] [--merged-into <PR>] <name>... | --merged`.
4. After the `git fetch -q origin` block (the one that exits 1 on failure), add the `--merged-into` preflight of § S5 design: the gh check, `$st = gh pr view $MergedInto --json state -q .state 2>$null`, the MERGED check, `git fetch -q origin "pull/$MergedInto/head"`, and `$IntoHead = git rev-parse --verify -q 'FETCH_HEAD^{commit}'`. Each failure prints its message from § S5 design to stderr and exits 1. Set `$IntoHead = ''` before it so StrictMode allows the later read.
5. In `Invoke-RetireMarked`, directly after the line `$state = if ($br) { Get-MergeState $br } else { 'DETACHED' }`, add: `if ($MergedInto -and $br -and $state -in 'NO_PR', 'UNMERGED') { git merge-base --is-ancestor $br $IntoHead 2>$null; if ($LASTEXITCODE -eq 0) { $state = 'MERGED_INTO' } }`. Then in the two lines that test `$state -notin 'MERGED', 'ON_MAIN'` and `$state -in 'MERGED', 'ON_MAIN'`, add `'MERGED_INTO'` to each list.
6. `test-rm-worktree.ps1`, in the shape of `tools/build-lane/test-branch-at.ps1` (a `Check` function, `$script:failed`, exit 1 when anything failed, isolated `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`). Setup, in a fresh temp root:
   - a bare `origin.git` (`git init -q --bare -b main`), a clone `main` with `user.name`, `user.email` and `core.autocrlf=false`, a `.gitignore` with `.claude/worktrees/`, one commit, pushed;
   - three worktrees, `git -C main worktree add -q -b zz-s5/a .claude/worktrees/zz-s5-a origin/main` (and `b`, `c`), each with one commit of its own file;
   - an integration branch `zz-s5/int` from `origin/main` that merges `zz-s5/a` and `zz-s5/c` with `--no-ff`; then `git -C origin.git update-ref refs/pull/7/head <int sha>` (the PR head the script fetches); `zz-s5/b` gets one more commit than anything merged (it is not in the head);
   - a stub dir prepended to `$env:PATH` holding `gh.cmd` (`@echo off`; `if "%1 %2"=="pr list" (echo null& exit /b 0)`; `if "%1 %2"=="pr view" (echo %GH_STUB_STATE%& exit /b 0)`; `exit /b 2`), and `psql.cmd` (`@exit /b 0`) with `$env:PSQL` set to its path;
   - `$env:LANE_ROOT` a temp dir, `$env:CIMMERIA_TARGET_ROOT` empty, `$env:GH_STUB_STATE='MERGED'`.
   Run the script as a child, `pwsh -NoProfile -File <wt>\tools\build-lane\rm-worktree.ps1 <args>`, with the current location in `main`, capturing stdout, stderr and the exit code.
7. The cases (each a `Check` with a clear name):
   - `--merged-into 7 zz-s5-a`: exit 0, stdout has `retire zz-s5-a (zz-s5/a, MERGED_INTO)`, the folder is gone, `git -C main branch --list zz-s5/a` prints nothing.
   - Without the option, `zz-s5-c` (before any case retires it): stdout has `skip zz-s5-c: branch zz-s5/c is NO_PR`, the folder stays.
   - `--merged-into 7 zz-s5-b`: skipped (`is NO_PR`), the folder stays.
   - `zz-s5-c` with an untracked file, `--merged-into 7 zz-s5-c`: `skip zz-s5-c: uncommitted changes`; then delete the file and run it again: retired.
   - `GH_STUB_STATE=OPEN`, `--merged-into 7 zz-s5-b`: exit 1, stderr has `not merged`.
   - Usage: `--merged-into` alone, `--merged-into x zz`, `--merged-into 7 --merged`, `--merged-into 7` with no name: each exit 2.
   - `--dry-run --merged-into 7 zz-s5-b`: exit 0, nothing removed (a dry run of a skip).
   Clean up the temp root in `finally` (`Remove-Item -Recurse -Force`; worktree folders first via `git worktree remove --force` inside the temp repo only).

**Test type:** integration of the script against a temp repository (no real gh, no database). **Revert proof:** before step 1, write the test and run it against the unchanged script: `pwsh -NoProfile -File <wt>\tools\build-lane\test-rm-worktree.ps1`. Record in the report that the `MERGED_INTO` and usage cases fail (an unknown option exits 2 with `unknown option`). Then make steps 1 to 5 and record that every case passes.

**Checks:** `Set-Location <wt>; pwsh -NoProfile -File tools\build-lane\test-rm-worktree.ps1` exits 0; every existing `tools\build-lane\test-*.ps1` still exits 0 (run each); `Set-Location <wt>; python -m unittest discover -s tools/build-lane -p "test_ship.py"` OK (its `RetireTests` drive rm-worktree.ps1 on Windows); R4 on both files. CI runs every `test-*.ps1` on Windows (build-lane-scripts.yml).

**Docs owed:** none in this PR; S5c carries the docs (doc-update-map row "Build lane" scripts, listed there). Say so in the PR body.

#### S5b: `rm-worktree.sh --merged-into`, with a Linux test

Agent: packet-coder. Branch `ai-harness-docs/s5b-merged-into-sh`, worktree `ah-s5b` (from `origin/main` after W4a and AH-11g merged). Size: about 35k. Own PR. Commit subject: `feat(build-lane): rm-worktree.sh --merged-into, the bash twin (ai-harness-docs S5b)`.

**Rules:**

- R1, R2 (the agent's own shell stays PowerShell on this Windows host; the test runs bash as a subprocess, as every `tools/build-lane/test_*_sh.py` does), R6, R8, R9, R10.
- `*.sh` files are **LF**. The Edit tool keeps LF; never write a `.sh` with `[IO.File]::WriteAllText` and CRLF. Check: `$t=[IO.File]::ReadAllText('<abs .sh>'); $t.Contains("`r")` prints `False`. The `.py` test is CRLF in the working tree, like its neighbours (R4); its shims are written LF from Python with `newline="\n"`.
- The `.ps1` is the spec (D-AH36): the same option, messages, states and exit codes as S5a's design in § S5. Do not wait for S5a; follow § S5 design.
- Temp repositories only, as in S5a. File caps: the test at most 250 lines.

**Files:** `tools/build-lane/rm-worktree.sh`; new `tools/build-lane/test_rm_worktree_sh.py`.

**Steps:**

1. Header. After line 6 (`#   tools/build-lane/rm-worktree.sh [--dry-run] [--prune] --merged`) insert `#   tools/build-lane/rm-worktree.sh [--dry-run] --merged-into <PR> <name> [<name>...]`. After the line `# registered worktree owns any more.` insert `# --merged-into <PR> also retires a named worktree whose branch has no PR of its own but` and `# is in merged PR <PR>'s head (a packet merged only into an integration branch).`. Change `sed -n '2,33p'` to `sed -n '2,36p'`.
2. Arguments. Add `MERGED_INTO=""` to the line `DRY=0; FORCE=0; MERGED=0; PRUNE=0; NAMES=()`. Replace the `for a in "$@"; do` loop with `while [ $# -gt 0 ]; do a="$1"; case "$a" in ... esac; shift; done`, keeping every existing case, and add `--merged-into) shift; [ $# -gt 0 ] || { echo "--merged-into needs a PR number" >&2; exit 2; }; MERGED_INTO="$1" ;;` before the `-h|--help` case.
3. Validation, after the loop and before the usage check, with the messages of § S5 design: `case "$MERGED_INTO" in ''|*[!0-9]*)` for the digits check (only when it is non-empty), and the `--merged` combination. The usage line gains `[--merged-into <PR>]` as in S5a step 3.
4. After the `git fetch -q origin` block, the preflight of § S5 design (`command -v gh`, `gh pr view "$MERGED_INTO" --json state -q .state`, `git fetch -q origin "pull/$MERGED_INTO/head"`, `INTO_HEAD="$(git rev-parse --verify -q 'FETCH_HEAD^{commit}')"`), each failure to stderr with exit 1. Set `INTO_HEAD=""` at the top so `set -u` allows the later read.
5. In `retire_marked`, directly after the line that sets `state` (`if [ -n "$br" ]; then state="$(merge_state "$br")"; else state="DETACHED"; fi`), add: `if [ -n "$MERGED_INTO" ] && [ -n "$br" ] && { [ "$state" = NO_PR ] || [ "$state" = UNMERGED ]; } && git merge-base --is-ancestor "$br" "$INTO_HEAD" 2>/dev/null; then state=MERGED_INTO; fi`. Change both `MERGED|ON_MAIN)` patterns to `MERGED|ON_MAIN|MERGED_INTO)`.
6. `test_rm_worktree_sh.py`, stdlib `unittest`, in the style of `tools/build-lane/test_py_wrappers_sh.py`: `from test_lane_stats import find_bash`, `from test_rebase_pr import Repo, sh` (Repo makes `origin.git` and `main`), `@unittest.skipUnless(find_bash(), "no bash")`. In `setUp`: the repo, a `.gitignore` with `.claude/worktrees/`, three worktrees under `main/.claude/worktrees/zz-s5-a|b|c` on branches `zz-s5/a|b|c`, the integration merge and `refs/pull/7/head` in `origin.git` exactly as S5a step 6; a `bin` dir with two `#!/bin/sh` shims written with `newline="\n"` and `chmod 0o755`: `gh` (`case "$1 $2" in "pr list") echo null ;; "pr view") echo "$GH_STUB_STATE" ;; *) exit 2 ;; esac`) and `psql` (`exit 0`). The environment: `PATH` with `bin` first (use `Path.as_posix()` for every path), `PSQL` the shim, `LANE_ROOT` a temp dir, `CIMMERIA_TARGET_ROOT` empty, `GH_STUB_STATE=MERGED`. Run `subprocess.run([find_bash(), str(HERE / "rm-worktree.sh"), *args], cwd=main, env=env, capture_output=True, text=True)`. `tearDown` removes the temp dir (`shutil.rmtree(..., ignore_errors=True)`).
7. The same seven cases as S5a step 7, one test method each, with the same expected lines.

**Test type:** integration of the script against a temp repository. It runs on Linux CI (`build-lane-scripts.yml` and `test.yml` both run `python3 -m unittest discover -s tools/build-lane -p "test_*.py"`) and locally under Git Bash. **Revert proof:** write the test first and run it against the unchanged script with `Set-Location <wt>; python -m unittest discover -s tools/build-lane -p "test_rm_worktree_sh.py"`; record that the `MERGED_INTO` and usage cases fail. Then make steps 1 to 5 and record that all pass. If the cases that need no change (the plain skip, the uncommitted skip) fail on this Windows host under Git Bash, stop and return `blocked` with their output: do not add a Windows skip on your own.

**Checks:** that discover run OK; `Set-Location <wt>; python -m unittest discover -s tools/build-lane -p "test_*.py"` OK (or the same skips and failures as on `origin/main`, listed in the report); no local `bash -n` or shellcheck (CI runs `bash -n` and `shellcheck -S error` on every `.sh`); the LF check on the `.sh`.

**Not done here (a follow-up for AH-24 to list):** adding `rm-worktree.sh` to the `shellcheck -S warning` list in build-lane-scripts.yml. No shellcheck exists on this host to prove the script clean at warning level first.

**Docs owed:** none in this PR; S5c carries them.

#### S5c: the ops line, the skill lines and the docs

Agent: packet-coder. Branch `ai-harness-docs/s5c-merged-into-docs`, worktree `ah-s5c` (from `origin/main` after S5a, S5b and AH-11g merged; W4a has merged by then). Size: about 15k. Own PR. Commit subject: `docs(agents): ops may retire integration-merged packets with --merged-into (ai-harness-docs S5c)`.

**Owner-visible:** this widens what `ops`, an agent that runs git, may do. The PR body opens with that sentence and with: "`ops` still never passes `--force` or `--prune`; `--merged-into` retires a worktree only when the named PR has merged and the branch's tip is in that PR's head."

**Rules:** R1 to R10 of work-packets-w4.md, and R11 on every file you edit. R7: after the agent and skill edits, run sync, then `sync.py --check`.

**Files:** `.claude/agents/ops.md`; `.claude/skills/campaign-packet/SKILL.md`; `.claude/skills/ship-pr/SKILL.md`; `docs/agents/development-workflow.md`; the mirrors sync regenerates (`.codex/agents/ops.toml`, `.agents/skills/**`); `docs/agents/ai-harness.md` only as regen rewrites it.

**Steps:**

1. `ops.md`: find the line that starts ``- `pwsh tools/build-lane/rm-worktree.ps1 <name>|--merged` `` (AH-11g appended its bracketed `.sh` twin) and insert directly below it: ``- `pwsh tools/build-lane/rm-worktree.ps1 --merged-into <PR> <name>...` [`bash tools/build-lane/rm-worktree.sh --merged-into <PR> <name>...`]: packet worktrees whose branch merged only into an integration branch, once that branch's PR `<PR>` has merged. Never with `--force`.`` Match the bracket style AH-11g used on the line above (with or without backticks inside the brackets); `test_ops_twins.py` reads the `.sh` names, so keep the `bash tools/build-lane/rm-worktree.sh` text.
2. `campaign-packet/SKILL.md` § 6 (close-out): find `which skips packet worktrees merged only into an integration branch: list those for the owner)` (AH-18 step 10's text) and replace it with `which skips packet worktrees merged only into an integration branch; retire those with \`rm-worktree.ps1 --merged-into <PR> <name>...\`)`, without the backslashes. If AH-18's text is absent, stop (R6).
3. `ship-pr/SKILL.md` § 7: after the line `pwsh tools/build-lane/rm-worktree.ps1 --merged        # every merged, idle (30 min) one`, insert `pwsh tools/build-lane/rm-worktree.ps1 --merged-into <PR> <name>...  # packets merged only into PR <PR>'s branch`.
4. `development-workflow.md` § Retire it when its PR merges:
   - in the code block, after `pwsh tools/build-lane/rm-worktree.ps1 --merged           # the same from PowerShell, no bash`, insert `bash tools/build-lane/rm-worktree.sh --merged-into <PR> <name>...  # packets merged only into an integration branch`;
   - in the bullet that starts ``- `gh` finds no PR for its branch (`NO_PR`)``, replace `Name it with \`--force\` to retire it; the branch is kept.` with `Name it with \`--force\` to retire it; the branch is kept. A packet branch that was merged into an integration branch whose PR has merged is retired with \`--merged-into <PR>\` instead, which deletes the branch and still applies every other check.`, without the backslashes;
   - set the file's `Last updated` line to today.
5. `python tools/agent-skills/sync.py`, then `python tools/docs-gen/regen.py --skip-crate-graph`: keep a change to `docs/agents/ai-harness.md` only if it is ops's row; restore every other file regen changed.

**Test type:** the existing contract test. `tools/agent-skills/test_ops_twins.py` already requires `bash tools/build-lane/rm-worktree.sh` in ops.md and the `.sh` file; no new test (a docs and persona change; the behaviour is tested in S5a and S5b).

**Checks:** `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"` OK; `python tools/agent-skills/sync.py --check` exit 0; `Select-String -Path <wt>\.claude\agents\ops.md -Pattern '--merged-into' -SimpleMatch` returns one line; R4 on the edited `.md` files; `regen.py --check --skip-crate-graph` lists none of the files above.

**Docs owed:** doc-update-map rows "Build lane" (a script option: development-workflow, the ship-pr skill) and "AI-harness wiring" (an agent's allowed list); list both. After it merges, the orchestrator updates work-packets-w4.md's W4a ship step 7 note in the ledger at the next ledger commit (it already says to use `--merged-into` once S5 merged).

### S6: a ledger lint for resume cards (approved)

Agent: packet-coder. Branch `ai-harness-docs/s6-resume-lint`, worktree `ah-s6` (from `origin/main` after W4a merged). Size: about 15k. Own PR. Commit subject: `test(agent-skills): every campaign RESUME.md fits the 40-line resume card (ai-harness-docs S6)`.

**Rules:** R1 to R10. `.github/workflows/*.yml` are LF. **Files:** new `tools/agent-skills/test_ledgers.py`; `tools/agent-skills/README.md`; `.github/workflows/agent-skills.yml`.

**Steps:**

1. `test_ledgers.py` (stdlib, about 30 lines, CRLF like its neighbours, R4): for every `docs/analysis/*/RESUME.md` (repo root from `Path(__file__).resolve().parents[2]`), with `subTest(file=...)`, assert the line count is at most 40, with the message `"<file> has <n> lines; resume_card.py injects only the first 40 (campaign-packet skill § 5)"`. A second test asserts at least 5 files were found.
2. `tools/agent-skills/README.md`: find `` , and `test_agent_refs.py` checks that the harness docs name only agents that exist. `` (AH-22's text) and replace it with `` , `test_agent_refs.py` checks that the harness docs name only agents that exist, and `test_ledgers.py` checks that every campaign `RESUME.md` fits the 40-line resume card. ``. If AH-22's text is absent, stop (R6).
3. `.github/workflows/agent-skills.yml`: add `- "docs/analysis/**/RESUME.md"` to both `paths:` lists, directly after the `- "tools/agent-skills/**"` line, with the one-liner of work-packets-w4.md AH-22 step 3, its list of four new lines replaced by this one line (`$new=$old+'      - "docs/analysis/**/RESUME.md"'+$nl`). It must print `status=ok`; then `git -C <wt> diff --numstat -- .github/workflows/agent-skills.yml` prints `2	0`.

**Test type:** unit. First run it once; if any existing RESUME.md is over 40 lines, **stop and return `blocked`** with the list (the owner decides whether to trim them or loosen the rule). Otherwise, prove it fails: write a 41-line temp `RESUME.md` under `<wt>\docs\analysis\zz-s6-probe\`, run the test (it must fail), and delete the folder (`Remove-Item -Recurse -Force <abs>`; `git -C <wt> status --short` then shows nothing for it).

**Checks:** the full `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"` OK; `python tools/agent-skills/sync.py --check` exit 0.

**Docs owed:** doc-update-map row "AI-harness wiring" (a check on the campaign cycle).
