# ai-harness-docs: work packets, waves 3 and 4

Continued from [work-packets.md](work-packets.md). The dispatch rules, the quoting convention (rule 8) and the contract (C1-C9) are there; packets below cite them by number.

## Wave 3: docs

### AH-11: platform reconciliation (after #1797)

Agent: packet-coder. Branch `ai-harness-docs/ah-11-platform`, worktree name `ah-11`. Size: about 30k working tokens. Commit subject: `docs: one platform statement: Windows with PowerShell 7 supported, others not validated (AH-11)`.

**Gate (the orchestrator checks this before dispatch):** #1797's state, through `ops` (`gh pr view 1797 --json state`).

- **Path A (merged):** cut the worktree from `origin/main`, which contains #1797's "WSL and Linux hosts" section.
- **Path B (closed without merging):** cut from `origin/main`. Step 0 applies `docs/analysis/ai-harness-docs/pr-1797.diff` with `git -C <wt> apply --3way <abs diff path>`, and the commit message gets an extra trailer, `Co-authored-by: aurablacklight <89038013+aurablacklight@users.noreply.github.com>`.
- **Still open** when W3 starts: dispatch the other W3 packets and hold AH-11. If it is still open after AH-14 merges, ask the owner whether to wait or take path B.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start each PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- Use the Edit tool. Every `.md` file is CRLF in the working tree; keep it CRLF. Check with `-match "(?<!\r)\n"` → `False`.
- **CLAUDE.md stays at or under 4,500 bytes.**
- Never hand-edit a `<!-- gen: -->` block.
- Do not edit `docs/agents/rules-and-gotchas.md` (AH-12 owns it), and leave the agent files alone (W2 did those).
- After editing `.claude/skills/lane-build/SKILL.md`, run `python tools/agent-skills/sync.py` and commit what it regenerates.
- Commit once at the end.

**Steps:** use the C7 wording. For each site, Grep for the quoted old text and replace only that sentence.

| # | File | Old text (find it) | New text |
|---|---|---|---|
| 1 | `docs/agents/development-workflow.md`, § WSL and Linux hosts | the paragraph starting `**Status: not supported for interactive sessions.` | the C7 long form, followed by: `The documented build path is still Windows-native ([building.md](../building.md#prerequisites)); this section records what you hit elsewhere, so you don't rediscover it.` |
| 2 | same section, bullet `**No documented path.**` | that bullet's first two sentences | `- **Not validated.** [building.md](../building.md#prerequisites) and the \`lane-build\` skill name Windows with PowerShell 7 as the validated platform. Every \`tools/build-lane/*.ps1\` has a \`.sh\` twin ([README](../../tools/build-lane/README.md)), but nobody runs them routinely on a developer host.` |
| 3 | same section | `That is an observation, not a supported configuration.` | `That is an observation, not a validated configuration.` |
| 4 | same section, end of `### Headless and CI launches...` | `Interactively, \`claude --agent hands-on\` is the documented escape hatch.` | append ` The session banner (\`.claude/hooks/mode_banner.sh\`) names the persona at the start of every session.` |
| 5 | `docs/agents/development-workflow.md` § Builds | `Development builds run natively on Windows, from PowerShell or Git Bash.` | `Development builds are validated on Windows with PowerShell 7; Git Bash, WSL and Linux are not validated, but not unsupported ([WSL and Linux hosts](#wsl-and-linux-hosts)).` |
| 6 | `CLAUDE.md` | `Windows-native, on the toolchain` | `Windows with PowerShell 7 is the validated platform, on the toolchain` |
| 7 | `CLAUDE.md` | `WSL/Linux hosts are not supported for interactive sessions; what breaks, and how headless launches differ:` | `WSL, Linux and Git Bash are not validated, but not unsupported; what breaks, and how headless launches differ:` |
| 8 | `CONTRIBUTING.md` | `(Windows-native builds on a pinned toolchain, and the build lane agents use)` | `(Windows with PowerShell 7 as the validated platform, a pinned toolchain, and the build lane agents use)` |
| 9 | `docs/architecture/build-system.md` | heading `### 11. No WSL builds` | `### 11. Platforms, and the retired WSL cross-compile`. Then Grep the repo for `#11-no-wsl-builds`, and change each hit to `#11-platforms-and-the-retired-wsl-cross-compile`. |
| 10 | same file | `Development builds run natively on Windows (PowerShell or Git Bash, driven by Claude Code).` | `Development builds are validated natively on Windows with PowerShell 7. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported ([WSL and Linux hosts](../agents/development-workflow.md#wsl-and-linux-hosts)).` |
| 11 | `docs/building.md` | `Development builds run natively on Windows (PowerShell or Git Bash). CI builds on Linux, but there is no WSL or cross-compile path for development.` | `Development builds are validated on Windows with PowerShell 7. CI builds on Linux. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported ([WSL and Linux hosts](agents/development-workflow.md#wsl-and-linux-hosts)); the old WSL cross-compile path is retired.` |
| 12 | the development-workflow section from row 1 | any quote of building.md's old text (`no WSL or cross-compile path for development`) | `the old WSL cross-compile path is retired` |
| 13 | `docs/guides/getting-started.md` line 37 | `There is no WSL build path.` | `Windows with PowerShell 7 is the validated setup; WSL and Linux are not validated, but not unsupported ([WSL and Linux hosts](../agents/development-workflow.md#wsl-and-linux-hosts)).` |
| 14 | `.github/copilot-instructions.md` line 40 | `development builds run natively on Windows;` | `development builds are validated on Windows with PowerShell 7 (WSL, Linux and Git Bash are not validated, but not unsupported);` |
| 15 | `.claude/skills/lane-build/SKILL.md` line 57 | `- **Builds are Windows-native.** The WSL cross-compile is retired; CI builds on Linux runners.` | `- **Windows with PowerShell 7 is the validated platform.** WSL, Linux and Git Bash are not validated, but not unsupported; the WSL cross-compile is retired, and CI builds on Linux runners.` |
| 16 | `docs/guides/autonomous-agent-kickoff.md`, the note #1797 added | `and WSL is not a supported interactive host` / `Which of the two wins on Linux is an open maintainer question.` | `and WSL is not a validated host` / `Until this guide is revised, CLAUDE.md's lane rule wins.` |

Then:

- Grep the repo (`*.md`, `*.yml`, `*.py`, `*.ps1`) for `nopjmp` and `Linux pipeline`. Expect no hits; record the result. If there is a hit, delete that sentence.
- Leave `ship.py`, `lane-slots.ps1` and the `pre-pr-checks.md` "macOS / Linux / WSL" labels unchanged: they state facts, not a support policy.
- If a site's old text is not found (for example, #1797 changed it), apply the C7 meaning to whatever the sentence now says, and list it in the report as a deviation.

**Test type:** none (docs only).

**Checks:** the CRLF check on every edited `.md`, the CLAUDE.md size check, and `python tools/agent-skills/sync.py --check`.

**Docs owed:** doc-update-map row 18 (CLAUDE.md) and row 17 (build commands and invariants: copilot-instructions); list them.

### AH-12: rules-and-gotchas promotions

Agent: packet-coder. Branch `ai-harness-docs/ah-12-rules`, worktree name `ah-12`. Size: about 15k working tokens. Commit subject: `docs(agents): promote AI-workflow rules and decisions into rules-and-gotchas (AH-12)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start each PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- Use the Edit tool. The file is CRLF; keep it CRLF.
- Edit only `docs/agents/rules-and-gotchas.md`.
- No IPs, credentials or account names.
- Commit once at the end.

**Steps:**

1. Set line 3 to `> **Last updated**: <today's date>`.
2. Line 74: replace the bullet with `- **Windows with PowerShell 7 is the validated platform.** WSL, Linux, and Git Bash on Windows are not validated, but not unsupported ([WSL and Linux hosts](development-workflow.md#wsl-and-linux-hosts)). The WSL cross-compile and its memory rules (the ~47 GB link, \`pkill -f rustc\`, \`CARGO_BUILD_JOBS=2\`) are retired; the 47 GB figure predates \`rust-lld\` on Windows. See [\`docs/architecture/build-system.md\`](../architecture/build-system.md).`
3. In `## Build and CI`, after the build-lane bullet (line 75), add:
   - `- **Never pass \`--target-dir target/desktop\`** (or any relative \`--target-dir\`) in a lane command or a brief. It overrides the lane's per-worktree target on the Dev Drive and builds a multi-GB tree inside the worktree on C:. On 2026-10-12, about 19 packet worktrees filled C: this way. For \`crates/launcher/desktop/Cargo.toml\`, omit it.`
4. Below the intro paragraph of `## AI-assisted work and token cost` (line 99), add these bullets before the existing ones:
   - `- **Agents use PowerShell, the validated path.** An agent on a host without \`pwsh\` returns \`blocked\` and says why. Humans may use the \`.sh\` twins; agents don't.`
   - `- **Model roles have reasons.** Haiku implements prescriptive packets, Sonnet reviews and fixes, Opus designs and rules but writes no code. Per 1,000 changed lines this cut cost 2.5x ($7.44 to $3.03), and oversteps where Opus coded cost $107.43 ([campaign-metrics](../analysis/campaign-metrics/findings.md)).`
   - `- **Run agents as defined.** No model override on a dispatch; the agent file's \`model\` is the decision. Lab driving through \`lab-driver\` is the one documented exception.`
   - `- **A packet is about 40k working tokens, and a coder gets at most 3 sessions.** About 35-40k of a Haiku session is fixed overhead, and 9 of 34 sessions hit the 90k cap before packets were re-sliced (0 of 36 after). The ladder: [campaign-packet skill](../../.claude/skills/campaign-packet/SKILL.md#sessions-passes-and-escalation).`
   - `- **Every PR gets an adversarial review by an agent that didn't write it.** For a campaign packet that is the wave's packet-review-fixer; otherwise a fresh read-only reviewer (the \`ship-pr\` skill § 5). Every Haiku commit is reviewed by Sonnet before it ships.`
   - `- **Workers run lane checks in the foreground**, with \`Set-Location <worktree>;\` at the start of each call, and commit as soon as the checks pass. On 2026-10-11, workers that backgrounded their lane calls sat idle for an hour with uncommitted work, and others ran lanes from the main checkout.`
5. In `## Client assets and RE tooling`, after the first bullet, add: `- **Never rehost or mirror the game client or its seed archive.** Seed bytes come only from the archive.org source in the signed manifest or the player's own copy; a file index of paths, sizes and hashes is fine ([launcher distribution](../client/launcher-distribution-setup.md)).`
6. In `## Where project state lives`, after the agent-memory bullet: `- **Committed files carry no IPs, credentials or account names**, memory files included.`

**Test type:** none (docs only).

**Checks:** the CRLF check (`-match "(?<!\r)\n"` → `False`). Then `Select-String` the file for `target/desktop`, `rehost` and `adversarial`: each must give at least one hit.

**Docs owed:** doc-update-map row 61 (a new project rule); list it.

### AH-13: `docs/agents/ai-harness.md` reference and review bots

Agent: documentation-writer. Branch `ai-harness-docs/ah-13-harness-reference`, worktree name `ah-13`. Size: about 40k working tokens. Commit subject: `docs(agents): ai-harness reference: settings, agents, skills, hooks, board, memory, review bots (AH-13)`.

**Rules for this packet:**

- Work only in your worktree, with absolute paths. Start each PowerShell call with `Set-Location <worktree>;`, and run every check in the foreground.
- PowerShell only. No `cargo`. Never run `git stash`, `git push`, `git worktree prune` or `gh pr`.
- `docs/**/*.md` are CRLF. Write the file, then convert it with `[IO.File]` and absolute paths.
- **Never hand-edit a generated block.** Add empty markers, then run `python tools/docs-gen/regen.py --skip-crate-graph`. Commit regenerated output only for the markers this packet adds (`docs/agents/ai-harness.md`, and the docs/readme.md count lines if they change). Run `git -C <wt> restore -- <file>` on every other file regen touched, such as `tools/README.md`. The rule is in tools/docs-gen/README.md lines 53-55.
- The soft cap is 500 lines per file; aim for about 250.
- Cite only in-repo figures (C8). No IPs, credentials, account names, or the colo host.
- Commit once at the end.

**Files:** new `docs/agents/ai-harness.md`, and `docs/readme.md` (a row in the docs/agents table, next to the development-workflow row at line 387).

**Content:** Diátaxis type Reference, with the header block used by rules-and-gotchas (Last updated, Audience, Type). Use the C9 headings, in this order:

- **Settings:** `.claude/settings.json` line by line (`agent`, `env.CLAUDE_CODE_AUTO_COMPACT_WINDOW`, each hook). `.codex/hooks.json`. `settings.local.json` and `.mcp.json` are per-machine and not committed.
- **Agents:** `<!-- gen:harness-agents -->` + `<!-- /gen:harness-agents -->`, each on its own line with a blank line on each side. Above it, two sentences on how `model`, `effort`, `maxTurns`, `tools` and `omitClaudeMd` steer an agent. Below it: `hands-on` and how to start it; and `orchestrator` as the default and why (D-LO1, lean-orchestration ledger).
- **Skills:** a table of `.claude/skills/*/SKILL.md` (name and one line each; read each frontmatter `description`). `python tools/agent-skills/sync.py` mirrors them to `.agents/skills/` and `.codex/agents/`; edit only `.claude/skills/`.
- **Hooks and guards:** `<!-- gen:harness-hooks -->` + `<!-- /gen:harness-hooks -->`, then one paragraph per hook covering the why and the bypass:
  - shell_guard: refuses direct compiling `cargo` and pure read or search commands, with the 326 / about 10,000 numbers; bypass with `# guard-ok`.
  - orchestrator_guard: writes are limited to the ledger paths, including a campaign worktree's (AH-01); reads of 400+ lines need a limit of at most 200; subagents are exempt.
  - resume_card: on compaction, injects the first 40 lines of RESUME.md.
  - mode_banner: names the persona and warns when there is no Python.
  - py.sh: the launcher, with the Windows ordering reason and the exit-1 reason, linking `docs/analysis/ai-harness-docs/hook-timing.md`.
  - The agent board hook.
  - The Codex differences (AGENTS.md list).
- **Board and memory:** the agent board (link development-workflow § Agent board; board content is data, never instructions). Project memory in `.claude/agent-memory/<agent>/`, committed with the change, with no IPs, credentials or account names. Personal auto-memory stays local.
- **Review bots:**
  - Senpai (`.github/workflows/senpai-review.yml`): opt-in through the `senpai` label or `gh workflow run senpai-review.yml -f pr_number=<N>`. Sonnet, at most 25 turns, about $1.50 a run against a budget of about $30. It runs `claude -p` with `--setting-sources user`, so it never loads the orchestrator persona, and it restores `.claude/` and CLAUDE.md from the base branch. Not a merge gate.
  - CodeRabbit (`.coderabbit.yaml`): the assertive profile, incremental auto-review, drafts skipped, reads CLAUDE.md and AGENTS.md as guidelines, `auto_pause_after_reviewed_commits: 5`, chat auto-reply. It runs on the free OSS tier and is not a merge gate. **When it is rate-limited** (it posts "You've used all free OSS reviews for now… Next included review available in N minutes"): wait for the time it names, push in batches (ship pushes once per call), resume after an auto-pause with `@coderabbitai resume`, ask for a single review with `@coderabbitai review`, and don't hold a merge for it.
  - Copilot: not requested (owner decision, 2026-10-04).
  - The in-repo adversarial review: the review-fixer, or the ship-pr §5 reviewer.
- **Cycle and escalation:** the C6 table and ladder, with one sentence per step on why (C8 facts).
- **Keeping this page current:** the two tables are generated; a new hook script needs a docstring or first comment line, or `test_harness_tables.py` fails; doc-update-map has a row for harness changes (AH-14 adds it).

**docs/readme.md row:** `| [ai-harness.md](agents/ai-harness.md) | Every customization of the AI harnesses here: settings, agents, skills, hooks and guards, board, memory, review bots, and the campaign cycle with its escalation ladder | Complete |`. Insert it after the development-workflow row.

**Checks:**

```powershell
Set-Location <worktree>; python tools/docs-gen/regen.py --skip-crate-graph
Set-Location <worktree>; git status --porcelain
Set-Location <worktree>; python tools/docs-gen/regen.py --check --skip-crate-graph
```

After the first command, restore the non-owned files as the rules say. The second command must show only your files. In the third, ai-harness.md must not be listed as stale. Finish with the CRLF check.

**Docs owed:** doc-update-map row 61 (the docs/agents table in docs/readme.md); list it.

### AH-14: README section, doc-update-map row, regen

Agent: documentation-writer. **Stacked on `ai-harness-docs/ah-13-harness-reference`.** Branch `ai-harness-docs/ah-14-readme`, worktree name `ah-14`, cut from the AH-13 branch. Size: about 30k working tokens. Commit subject: `docs(readme): how AI development works here, with the numbers (AH-14)`.

**Rules for this packet:** the AH-13 rules apply. Commit regenerated output only for the README's new `harness-hooks` marker; restore other regen changes. README.md is CRLF in the working tree and has a generated crate graph (lines 105-295) that you must not touch. **The README must stay at most 500 lines (the soft cap); it starts at 406.**

**Files:** `README.md` and `docs/agents/doc-update-map.md`.

**Steps:**

1. Insert `## How AI development works here` after the `## Tests & CI` section and before `## Quick Start` (Grep for both headings). About 70 lines, generated table included:
   - **Lead** (3 sentences): Claude Code starts as the `orchestrator` persona, which dispatches agents and has no shell. The banner says so. `claude --agent hands-on` gives a full-tool session.
   - **The cycle**, as a numbered list of 6:
     1. orchestrator (Sonnet) takes the request;
     2. architect (Opus) designs and writes the packets; this is the "packet writer" step (D-AH1);
     3. ops (Haiku) makes worktrees;
     4. packet-coder (Haiku) implements, with at most 3 sessions per packet for the context limit;
     5. packet-review-fixer (Sonnet) reviews and fixes each wave, with at most 3 passes;
     6. ops ships through the merge queue.
   - **Escalation** (2 bullets, from C6): the coder ladder and the reviewer ladder.
   - **Why it works this way:** the C8 table verbatim, plus 4 one-line bullets: the caveat, packet size, the fresh-session cost cliff, and "Opus doesn't code". Cite `docs/analysis/campaign-metrics/findings.md`.
   - **Hooks:** one sentence, then `<!-- gen:harness-hooks -->` / `<!-- /gen:harness-hooks -->`, with a blank line on each side.
   - **Everything else** (2 lines): links to `docs/agents/ai-harness.md` (settings, agents, skills, board, memory, review bots), `docs/agents/development-workflow.md`, the campaign-packet skill and AGENTS.md (Codex).
   - **Platform** (1 line): the C7 short form, with a link.
2. doc-update-map.md: add a row after row 61: `| AI-harness wiring: \`.claude/settings.json\`, \`.codex/hooks.json\`, a hook script under \`.claude/hooks/\`, an agent's frontmatter, or the campaign cycle (sessions, passes, escalation) | Run \`python tools/docs-gen/regen.py --skip-crate-graph\` (the hook and agent tables in [README.md](../../README.md#how-ai-development-works-here) and [ai-harness.md](ai-harness.md) are generated). Give a new hook script a docstring or first comment line (\`test_harness_tables.py\` fails without one). Update the prose in [ai-harness.md](ai-harness.md), the cycle in the campaign-packet skill, [development-workflow.md](development-workflow.md) and [AGENTS.md](../../AGENTS.md) together. |`
3. Run regen, then restore the files you don't own, as in AH-13.

**Checks:** the AH-13 three commands. Then `(Get-Content <abs>\README.md).Count` must be at most 500, plus the CRLF check on both files.

**Docs owed:** this packet adds the new doc-update-map row; list it.

---

## Wave 4: acceptance and close-out

### AH-15: fresh-clone smoke test (acceptance)

Agent: **the owner, or a `hands-on` session the owner approves.** No packet agent can run it, because it needs a human-launched Claude Code session with no memory. Status starts at BlockedDependency (W3 merged) and becomes UATPending.

**Windows (required):**

1. `git clone` the repo into a new folder outside every worktree, then run `setup.ps1` as documented.
2. Start Claude Code there with no `--agent`.
   - **Expect** the banner `Mode: orchestrator...` in the first screen, with no WARNING.
3. Ask: "Fix a typo in docs/guides/getting-started.md."
   - **Expect** a one-line hands-on mention at most, and then a dispatch (documentation-writer or packet-coder), not a refusal.
4. Quit. Start Claude Code with `--agent hands-on`.
   - **Expect** `Mode: hands-on` (or the orchestrator wording if the payload carries no `agent_type`; record which).
   - Ask it to run `cargo check -p cimmeria-common` directly. **Expect** shell_guard to refuse it and point to the lane.
5. Start a session with Python removed from PATH: `$env:PATH = ($env:PATH -split ';' | ? { $_ -notmatch 'Python' }) -join ';'; claude`.
   - **Expect** the banner WARNING, and a visible hook error, not silence, on the first shell call.
6. Record each result, pass or fail, in `docs/analysis/ai-harness-docs/worknotes/AH-15.md`.

**WSL or Linux (best effort):** repeat steps 2-5 with only `python3` installed. **Expect** the banner without a WARNING, and the guards active.

### AH-16: close-out

Agent: packet-coder. Branch `ai-harness-docs/ah-16-close-out`, worktree name `ah-16`. Size: about 15k working tokens. Commit subject: `docs(ai-harness-docs): close-out (AH-16)`.

**Rules for this packet:** the AH-12 rules, plus the AH-13 regen rule.

**Steps:**

1. `docs/guides/unified-uat.md`: add an entry for AH-15 that copies its steps, unless AH-15 is already Done, in which case add its result line. Follow the file's existing entry format (Grep for `CR-13` to see one).
2. `docs/project-status.md`: Grep for an AI or agent-tooling section. If one exists, add one line: `AI harness: mode banner, py.sh hook launcher, worktree-aware orchestrator guard, generated hook and agent tables (ai-harness-docs, 2026-10).` If none exists, change nothing and say so. `docs/gap-analysis*` doesn't change: this is not game functionality.
3. Run `python tools/docs-gen/regen.py --skip-crate-graph`, and commit only the generated blocks this campaign introduced. Restore the others.
4. Ledger: set every packet row to its final state, and rewrite RESUME.md to "campaign closed".

The orchestrator then:

- empties `.claude/state/active-campaign`;
- has ops retire every `ah-*` worktree (`rm-worktree.ps1 --merged`);
- has ops post the board handoff.

**Checks:** the CRLF check on every edited `.md`.

**Docs owed:** the status docs (once), and unified-uat.
