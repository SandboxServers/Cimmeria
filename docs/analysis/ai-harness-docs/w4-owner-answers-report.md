# ai-harness-docs: the owner's answers of 2026-10-18, applied

Architect report. Every change is under `docs/analysis/ai-harness-docs/`, written in the main checkout, uncommitted; AH-W40 ships it. New decisions: D-AH50 to D-AH56; new findings: F48 to F55 (README.md). Nothing was dispatched, compiled or pushed.

## Answer 1: the README holds the newcomer explanation (D-AH50)

**Decided:** yes. The explanation lives in the root README's `## How AI development works here`, as long as it needs; `docs/agents/ai-sdlc.md` stays as the deeper reference (guards in detail, the honest numbers with their dates, platforms, the review loop detail). No paragraph appears in both. D-AH46 is superseded, and D-AH13's 80-line limit with it.

**Re-slice of W4b:**

| Packet | What | Size |
|---|---|---|
| AH-23a | README only: two splices from w4b-drafts.md (`readme-ai-a` from the section heading up to the `harness-hooks` gen block, `readme-ai-b` from `**Everything else:**` up to `**Platform:**`). The generated hook table stays where it is, untouched. Carries the S2 Mermaid diagram. | about 20k |
| AH-23b | Same worktree `ah-23`, after AH-23a: extract `ai-sdlc.md`, then the pointers (docs/readme.md row, AGENTS.md, development-workflow, ai-harness, doc-update-map, the rules-and-gotchas "No hook refuses raw git" bullet) and a scoped regen. | about 25k |

One PR (w4b-pr-body.md, rewritten). CRLF (R4) and the gen-block rule (R5) apply; AH-23a's checks prove the gen block is byte-identical.

## Answer 2: S1 stays BlockedDecision; who it guards

`shell_guard.py` runs on every shell call a hook sees.

- **Guarded, Claude Code:** the `.claude/settings.json` PreToolUse matcher is `Bash|PowerShell`, so it covers every session and subagent with a shell: hands-on, architect, ops, packet-coder, packet-reviewer, documentation-writer, rust-gameserver-dev, every domain advisor, and the built-in Explore, Plan and general-purpose agents. Subagents inherit the project hooks.
- **Guarded, Codex:** `.codex/hooks.json` matches `Bash` only (F52).
- **Not guarded:** the orchestrator (no shell at all), the lab-driver (MCP tools only), a human's own terminal, and any git a repo script runs inside itself. `rm-worktree --prune` runs `git worktree prune` internally (F50); `ops` never passes `--prune`.
- **Bypass:** `# guard-ok` at the end of a command, by design.

So S1 is a second net, under the tool lists, for a prune or stash typed straight into a shell call. The packet is unchanged and ready (work-packets-w4b.md § S1, about 30k, after W4b).

## Answer 3: S2, S3, S5, S6 approved; S4 stays BlockedDecision (D-AH52, D-AH54)

- **S2:** folded into AH-23a (the diagram is part of the README splice). No packet or PR of its own.
- **S3:** documentation-writer, about 35k, after W4b; links from ai-sdlc § Further reading, the README's Everything-else list and docs/readme.md.
- **S5, expanded (D-AH54):** `--merged-into <PR>` with explicit names. Usage errors exit 2. A preflight after the fetch (gh present, the PR `MERGED`, `pull/<PR>/head` fetched) exits 1 on failure. A worktree in state `NO_PR` or `UNMERGED` whose branch is an ancestor of the PR head becomes `MERGED_INTO`, which the skip check and the branch delete treat like `MERGED`. Every other skip still runs first, and it never implies `--force`.
  - **S5a:** `rm-worktree.ps1`, plus a new `test-rm-worktree.ps1` (gh.cmd and psql.cmd stubs, seven cases, revert proof), about 35k.
  - **S5b:** `rm-worktree.sh`, plus a new `test_rm_worktree_sh.py`, about 35k. It runs under Git Bash locally and on Linux in CI's existing discover step. If the base cases fail under Git Bash, it returns `blocked`.
  - **S5c:** the ops allowed-list line, campaign-packet § 6, the ship-pr § 7 line, development-workflow, sync and regen, about 15k. Owner-visible.
  - New test files, because test_ship.py is at 699/700 (F50).
- **S6:** `test_ledgers.py`, the agent-skills README line, and the agent-skills.yml path for `docs/analysis/**/RESUME.md`, about 15k, after W4a.
- **S4, the information the owner asked for (F53):** the token-profile database (schema v6) stores agent type, model, worktree, time window and tool-call fingerprints, but **no brief text**. So the planned "brief says `review fix, pass`" rule needs a schema change first. If nothing is done, the totals stay right and only the implementer/fixer split is wrong. The options:
  - (a) a `packet-fixer` alias, about 15k;
  - (b) classify by the fix commit subject `<packet> review fixes, pass N`, about 35k, **recommended**;
  - (c) by hand at the next metrics run;
  - (d) store the brief's first line, about 3 packets plus 10k.

  The details are in work-packets-w4b.md § S4.

## Answer 4: D-AH48 decided; the architect writes no code or fix (D-AH51)

After pass 3 is still `fix`, or on `decision`, the architect decides only what to fix and how, in a ruling note in `worknotes/<packet>.md`. **The architect writes no code or fix.** One Sonnet implementer applies the ruling (`rust-gameserver-dev` for code, `documentation-writer` for docs), then one fresh packet-reviewer checks it.

The sentence is now in:

- C12 item 3 (work-packets-w4.md);
- AH-18's campaign-packet skill text, with a check;
- AH-19's orchestrator.md step 4, with a check;
- AH-21's ai-harness.md reviewer bullet, with a check;
- the README section block (w4b-drafts.md, checked in AH-23a);
- RESUME.md's loop rules.

Each check greps the literal `writes no code or fix` and expects one line.

## Answer 5: a Bash path on Linux and WSL only (D-AH53, wave W4c)

**How an agent detects the platform:** by the `Platform:` line of its environment block, which Claude Code writes at session start (`win32`, `linux`, `darwin`; WSL reports `linux`). This is the key AH-11g's C10 already uses for `ops`. The mode banner is not the key (F49): it names the persona and warns when Python is missing, nothing more.

**The rule (C17):**

| Platform | Shell |
|---|---|
| `win32` | PowerShell, never Bash, not Git Bash |
| `linux` | Bash and the `.sh` twins with the same arguments, starting each call with `cd <wt> &&` |
| no line | as `win32` |
| other (`darwin` included) | `blocked: platform <p> is not supported (Windows, Linux and WSL only)` |
| needed tool missing | `blocked: no PowerShell tool on win32` or `no Bash tool on linux` |

Every packet carries `Host: windows|linux|both` (`windows` by default). A mismatch returns `blocked`. A coder never translates a command, except the lane, whose twins take the same arguments. One host per checkout: Linux lane slots live under `~/.local/share` and Windows slots under `%LOCALAPPDATA%` (F24). shell_guard already accepts `lane.sh` (F48).

**Amended:**

- D-AH29, now yes, Linux and WSL only.
- D-AH16, where C7c replaces C7b after W4c.
- C10 and AH-11g's ops paragraph (work-packets-w3.md and w3b.md): `darwin` is dropped from the Bash branch, the unsupported-platform refusal is added, and both stay identical to C17.
- The AH-15 Linux leg (`Host: linux`).

**New packets (work-packets-w4c.md):** one PR from `ah-w4c`, after W3c, W4a and W4b.

| Packet | Files | Size |
|---|---|---|
| AH-25 | packet-coder, packet-reviewer and architect gain Bash and the platform rule; the two domain agents' `lane.sh`-only lines name both lanes (F54); new `tools/agent-skills/test_platform_rule.py` (fails on revert); one agent-skills README clause | about 25k |
| AH-26 | lane-build § Linux and WSL hosts; campaign-packet's `Host:` bullet and the platform-keyed item 3; AGENTS.md's three twin notes | about 25k |
| AH-27 | development-workflow (C7c, the agents bullet, the one-host bullet); rules-and-gotchas line 102; ai-sdlc § Platforms; the README `**Platform:**` line | about 20k |

The three packets are file-disjoint and form one wave (one wave review), with the packet bodies in w4c-pr-body.md. CLAUDE.md is not edited (about 4,423 of 4,500 bytes).

**Why W4c waits for W4b:** AH-27 edits ai-sdlc.md, which AH-23b creates.

**Status wording stays "not validated"** until a full session runs on Linux. AH-15's Linux leg (D-AH33, still the owner's call) is the first run that would test it.

## Answer 6: the gate (D-AH55)

Dispatch rule 0 (work-packets-w4.md): nothing merges while `main` is red. Before each merge:

- check `gh issue list --label main-red --state open`;
- check the latest `main` runs of pr-container and build-and-test.

If either is red, hold the merge and tell the owner. As of this revision, #1844 is closed after #1849, and pr-container was green on dcf736016 at 22:48Z (F55). This was not rechecked after that; rule 0 rechecks at every merge.

## The order (D-AH56; work-packets-w4b.md § Order)

1. AH-W40.
2. W3c: AH-11g and AH-11h (AH-11p is already #1850).
3. AH-11i.
4. W4a.
5. In parallel: W4b (AH-23a, then AH-23b), S6, S5a and S5b.
6. W4c after W4b; S5c after S5a and S5b; S3 after W4b.
7. W5: AH-24 after W4b, W4c, AH-11i, S3, S5c and S6; then AH-15 and AH-16.

Once S5c has merged, the packet worktrees of W4a and W4c are retired with `--merged-into`; until then the owner retires them with `--force`.

## Other fixes made in passing

- **AH-W40** now copies 16 files: it adds `w4b-drafts.md`, `work-packets-w4c.md`, `w4c-pr-body.md` and this report. Its guard hash `3b2a99d6e` was rechecked against origin/main and is unchanged.
- **R1 gains a shell_guard note.** A `Select-String`- or `Get-Content`-only check is refused by the hook, so packets run those checks with the Grep tool or `# guard-ok`. The packets' line-count checks now use `[IO.File]::ReadAllLines(...).Length`, which the guard allows. Without this, a packet-coder would have hit refusals on its own checks.
- **AH-26** names the real doc-update-map row ("The pre-PR checklist, build commands, or repo invariants"). There is no "Build lane" row.
- **AH-25 step 8** has two exact finds, before and after S6's change to the same README sentence.

## Files touched (all under docs/analysis/ai-harness-docs/, CRLF)

- **Edited:** README.md, RESUME.md (33 lines), work-packets-w3.md, work-packets-w3b.md, work-packets-w4.md, w4-drafts.md.
- **Rewritten:** work-packets-w4b.md, w4b-pr-body.md.
- **New:** w4b-drafts.md, work-packets-w4c.md, w4c-pr-body.md, this report.

## Measured versus not measured

**Verified in the repo or on GitHub today:**

- F48 to F55;
- the `Platform:` line in an agent environment block (win32 on this host);
- the shell_guard matchers;
- the token-profile schema v6 columns;
- the CI discover steps;
- the rm-worktree line anchors;
- the AH-W40 guard hash.

**Not verified:**

- whether Codex's environment carries a `Platform:` line;
- any agent session on Linux or WSL;
- the live-DB scripts on Linux;
- S5b's Git Bash run of the base retire cases (it returns `blocked` if they fail);
- the Mermaid rendering (AH-15 W8);
- the packet sizes, which are estimates against the 40k cap.

## BlockedDecision (for the orchestrator to put to the owner)

- **S1:** yes or no, now that the coverage above is known.
- **S4:** option (a), (b), (c) or (d); (b) is recommended.
- **D-AH33:** is AH-15's Linux or WSL leg required (recommended) or best effort? It now also covers the packet workers' Bash path.

## Owner actions, not decisions

- **AH-11i:** post pr-1797-close.md and close #1797.
- **AH-15:** run the smoke test, or approve a hands-on session to run it.
- **Relaunch** after W4a, S5c and W4c.
- **Retire with `--force`** only if S5c has not merged by the time W4a or W4c merges.
