# ai-harness-docs: work packets, waves 3 and 4

Continued from [work-packets.md](work-packets.md). The dispatch rules, the quoting convention (rule 8) and the contract (C1-C9) are there; packets below cite them by number. Wave 3b (AH-11a to AH-11f), with the AH-11 rules, C7b and C10, is in [work-packets-w3b.md](work-packets-w3b.md).

## Wave 3: docs (merged)

AH-12 (#1817), AH-13 (#1818) and AH-14 (#1819) are merged; their sections stay as the record. The old AH-11 (paths A and B, waiting on #1797) is replaced by the #1797 takeover: wave 3b in [work-packets-w3b.md](work-packets-w3b.md) and wave 3c below.

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

## Wave 3c: AH-11g to AH-11i

Dispatch after every W3b PR has merged. "The AH-11 rules", C7b and C10 are in [work-packets-w3b.md](work-packets-w3b.md#rules-for-every-ah-11-packet); the decisions and findings are in the [README](README.md#decisions).

### AH-11g: `ops` runs the `.sh` twins off Windows

Agent: packet-coder. Branch `ai-harness-docs/ah-11g-ops-bash`, worktree name `ah-11g`. Size: about 30k working tokens. Wave 3c, after every W3b PR has merged. Commit subject: `feat(agents): ops uses the bash twins on Linux and WSL hosts (AH-11g)`.

**Rules:** the AH-11 rules. Edit only:

- `.claude/agents/ops.md`;
- `.claude/skills/ship-pr/SKILL.md` (lines 8-10);
- `.claude/skills/campaign-packet/SKILL.md` (§ 3, item 3);
- the new `tools/agent-skills/test_ops_twins.py`;
- the mirrors `python tools/agent-skills/sync.py` regenerates (`.codex/agents/ops.toml`, `.agents/skills/**`).

Keep ops's `description` line unchanged: the generated agents table in `docs/agents/ai-harness.md` is built from it.

**Steps:**

1. `ops.md` frontmatter: `tools: PowerShell, Read` → `tools: PowerShell, Bash, Read`.
2. `ops.md` body:
   - After the first paragraph, insert:
   ```text
   **Pick the shell from the host.** Read the `Platform:` line of your environment. On `win32`, use the PowerShell tool and the `pwsh ... .ps1` commands below; never use Bash there. On any other platform (`linux`, `darwin`), use the Bash tool and the `.sh` twin given in brackets, with the same arguments: the twins take the same options and print the same output with the same exit codes. Never run a `.ps1` on Linux. If the tool you need is unavailable, return `blocked` with the reason `no PowerShell tool on win32` or `no Bash tool on <platform>`, never a bare `blocked`.
   ```
   - Append a bracketed twin to each script line:
     - mk-worktree: `[bash tools/build-lane/mk-worktree.sh <branch> <name>]`;
     - both ship lines: `[bash tools/build-lane/ship.sh ...same arguments]`;
     - rm-worktree: `[bash tools/build-lane/rm-worktree.sh <name>|--merged]`.
   - Replace the board line with: ``- Board reads and posts, as the main session (no `--as`): on `win32`, `python "$HOME\.agent-board\board.py" <command>` (call `board.py` directly, because the `board` shim is a `/bin/sh` script); elsewhere, `"$HOME/.agent-board/board" <command>`.``
   - In the "Never run" paragraph:
     - change `a push happens only inside `ship.ps1`` to `a push happens only inside `ship.ps1` or `ship.sh``;
     - delete the sentence `If the PowerShell tool is unavailable (a Linux or WSL host), return `blocked` and state that reason in the one-clause reason; never a bare `blocked`.`, which the new paragraph replaces.
3. `ship-pr/SKILL.md`: replace `Run everything\nfrom PowerShell; do not use bash.` with `Run everything\nfrom PowerShell on Windows; on a Linux or WSL host, \`ops\` runs the \`.sh\` twins with the same arguments.` Keep the line break where it is, and remove the backslashes (quoting rule 8).
4. `campaign-packet/SKILL.md` § 3 item 3: after `They commit locally and\n   stop.` add ` On a Linux or WSL host only \`ops\` has a shell path (the \`.sh\` twins); a packet worker without PowerShell returns \`blocked\`.` Remove the backslashes.
5. Create `tools/agent-skills/test_ops_twins.py`, a **unit test** of the agent contract that the existing agent-skills CI job runs (`python3 -m unittest discover -s tools/agent-skills`):
   - It reads `.claude/agents/ops.md` (path from `Path(__file__).resolve().parents[2]`).
   - `test_every_script_has_its_bash_twin`: for each `tools/build-lane/<name>.ps1` named in ops.md, `tools/build-lane/<name>.sh` exists, and `bash tools/build-lane/<name>.sh` appears in ops.md.
   - `test_frontmatter_grants_bash_and_the_body_keys_on_platform`: the frontmatter `tools:` line lists `Bash`, and the body contains `Platform:` and `win32`.

   **Reverted** (ops.md back to PowerShell only), both tests fail.
6. Run `python tools/agent-skills/sync.py`, then `python tools/docs-gen/regen.py --check --skip-crate-graph`. ai-harness.md must not be listed as stale; if it is, ops's description changed, so undo that.

**Checks:**

- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`;
- `python tools/agent-skills/sync.py --check`;
- the regen `--check` from step 6.

**Docs owed:** the doc-update-map "AI-harness wiring" row (an agent's frontmatter: AH-14 added it); list it. The prose that row asks for is AH-11h's.

### AH-11h: the WSL and Linux section and the agent rule, to the new state

Agent: packet-coder. Branch `ai-harness-docs/ah-11h-linux-docs`, worktree name `ah-11h`. Size: about 25k working tokens. Wave 3c, parallel with AH-11g. Commit subject: `docs(agents): Linux script fixes and the ops bash path in the platform docs (AH-11h)`.

**Rules:** the AH-11 rules. Edit only `docs/agents/development-workflow.md` and `docs/agents/rules-and-gotchas.md`.

**Steps** (find each text exactly as AH-11a left it, and replace only that):

1. § WSL and Linux hosts, the status paragraph. Find the sentence from `The build lane, the ship scripts and every agent rule are validated` through `returns \`blocked: no PowerShell (pwsh) on this host\`.` Replace that span with everything in C7b after its bold first sentence. Keep the bold sentence, and keep the building.md sentence that follows the span.
2. Find the bullet that starts `- **\`ops\` cannot run.**` (the whole line). Replace it with:
   ```text
   - **`ops` runs the `.sh` twins.** On a host whose `Platform` is not `win32`, [`ops`](../../.claude/agents/ops.md) uses the Bash tool and runs `bash tools/build-lane/<script>.sh` with the same arguments as the `.ps1` commands. The packet workers (`packet-coder`, `packet-review-fixer`) still use PowerShell only, and without it they return `blocked: no PowerShell (pwsh) on this host`.
   ```
3. Find `- **Repo scripts that fail on Linux or WSL:**`. Replace it with `- **Repo scripts on Linux or WSL** (fixed in the scripts; CI tests them on Linux):`. Then replace its five sub-bullets, in order, with:
   ```text
     - `mk-worktree.sh` symlinks `external/` on Linux and WSL (a junction under Git Bash), and warns when the main checkout has no `external/` yet; see [Create a worktree that builds](#create-a-worktree-that-builds).
     - `ship.sh` and `rebase-pr.sh` use `$PYTHON`, else the first of `python3`, `python` and `py` that runs, in the same order as `lane.sh`.
     - `ship.py merge` reads the checks from `gh pr view --json statusCheckRollup` when the installed `gh` has no `gh pr checks --json` (2.45 has none).
     - `ship.py` retargets restacked PRs through `gh api -X PATCH` when `gh pr edit --base` fails on an old `gh` (a GraphQL error about classic projects). Editing a PR body by hand hits the same error; use `gh api -X PATCH repos/<owner>/<repo>/pulls/<n> -F body=@file`.
     - `rm-worktree.sh` runs on Linux and WSL, and `ship.sh merge --retire` calls it.
   ```
4. § Builds, worktrees and test databases. Find `The \`.sh\` scripts stay for CI, Linux and macOS, and for Git Bash.` Replace it with `The \`.sh\` scripts stay for CI, Linux and macOS, for Git Bash, and for \`ops\` on a Linux or WSL host.` Remove the backslashes.
5. Same section, § Create a worktree that builds. Find `junctions \`external/\` in` and replace it with `links \`external/\` in (a junction on Windows, a symlink elsewhere)`.
6. Find `- **\`ops\` ships a packet with \`ship.ps1\`.**` and replace it with `- **\`ops\` ships a packet with \`ship.ps1\`** (\`ship.sh\` on a Linux or WSL host).` Remove the backslashes.
7. `docs/agents/rules-and-gotchas.md` line 102. Find `- **Agents use PowerShell, the validated path.** An agent on a host without \`pwsh\` returns \`blocked\` and says why. Humans may use the \`.sh\` twins; agents don't.` Replace it with:
   ```text
   - **Agents use PowerShell, the validated path; `ops` alone also has a bash path.** On a Linux or WSL host, `ops` runs the `.sh` twins, which CI tests on Linux. Every other agent on a host without `pwsh` returns `blocked` and says why. Humans may use the `.sh` twins too.
   ```
   Then set line 3 to `> **Last updated**: <today's date>`.

**Test type:** none (docs only). AH-11b to AH-11e and AH-11g carry the tests.

**Checks:**

- The CRLF check on both files.
- `Select-String` development-workflow.md for `cannot run`, `that fail on Linux` and `not supported`: no hits.
- Every anchor these steps use resolves. Grep the headings `## WSL and Linux hosts` and `### Create a worktree that builds`.

**Docs owed:** the doc-update-map rows for a new project rule (rules-and-gotchas) and for AI-harness wiring (the prose); list both.

### AH-11i: close #1797 with a reply (owner action)

Not a packet-agent task. `ops` has no `gh pr comment` or `gh pr close` (F27), and the reply speaks for the maintainers to a contributor. The text is [pr-1797-close.md](pr-1797-close.md); fill in the PR numbers when you post it. After AH-11g and AH-11h merge, the owner (or a `hands-on` session the owner approves) runs:

```powershell
gh pr comment 1797 --body-file <abs path to a copy of the text below pr-1797-close.md's line, PR numbers filled in>
gh pr close 1797
```

Then the orchestrator sets AH-11i to Done in the README.

---

## Wave 4: acceptance and close-out

### AH-15: fresh-clone smoke test (acceptance)

Agent: **the owner, or a `hands-on` session the owner approves.** No packet agent can run it, because it needs a human-launched Claude Code session with no memory. Status starts at BlockedDependency (W3c merged) and becomes UATPending.

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

**Linux or WSL leg (D-AH33: BlockedDecision, required or best effort; the recommendation is required).** It is the only end-to-end proof of the `ops` bash path, since CI tests the scripts but not the agent. Run it on a WSL Ubuntu or Linux host with `git`, `gh`, `python3` and Claude Code, signed in to `gh`. `pwsh` is not needed.

- L1. Clone into the Linux filesystem (for example `~/src/cimmeria`, not under `/mnt/c`). Skip `setup.ps1`, and run `mkdir external` so that worktrees have something to link.
- L2. Start Claude Code with no `--agent`.
  - **Expect** `Mode: orchestrator...` with no WARNING: `py.sh` finds `python3`.
- L3. Ask: "Have ops create the worktree ah15-probe on branch probe/ah15, then remove it."
  - **Expect** ops to use the Bash tool (not a refusal and not a `.ps1`), and `bash tools/build-lane/mk-worktree.sh probe/ah15 ah15-probe` to print `(external/ symlinked)`.
  - **Expect** `bash tools/build-lane/rm-worktree.sh ah15-probe` to succeed, after which `git worktree list` no longer shows it.
- L4. In a plain terminal, run `bash tools/build-lane/ship.sh pr -C /nonexistent -m x; echo $?`.
  - **Expect** a `status=refused` line and exit 2, not exit 127.
- L5. Record `gh --version`. On a version older than 2.50, `ship.py merge` takes the `statusCheckRollup` fallback (AH-11d). Its first real use is the next PR shipped from this host; record that run when it happens.
- L6. Ask the orchestrator to dispatch a `packet-coder` for any one-line docs fix.
  - **Expect** `blocked: no PowerShell (pwsh) on this host` when `pwsh` is absent (D-AH29 keeps the workers PowerShell-only). If `pwsh` is installed, record what happened instead.
- L7. Record the results in the same worknotes file.

A pass proves the `ops` path. It does not change C7b: Linux and WSL stay "not validated" until a full session (orchestrator, packet workers, review, ship) has run end to end there.

### AH-16: close-out

Agent: packet-coder. Branch `ai-harness-docs/ah-16-close-out`, worktree name `ah-16`. Size: about 15k working tokens. Commit subject: `docs(ai-harness-docs): close-out (AH-16)`.

**Rules for this packet:** the AH-12 rules, plus the AH-13 regen rule.

**Steps:**

1. `docs/guides/unified-uat.md`: add an entry for AH-15 that copies its steps (the Windows steps and the Linux leg), unless AH-15 is already Done, in which case add its result line. Follow the file's existing entry format (Grep for `CR-13` to see one).
2. `docs/project-status.md`: Grep for an AI or agent-tooling section. If one exists, add one line: `AI harness: mode banner, py.sh hook launcher, worktree-aware orchestrator guard, generated hook and agent tables, ops bash path on Linux and WSL (ai-harness-docs, 2026-10).` If none exists, change nothing and say so. `docs/gap-analysis*` doesn't change: this is not game functionality.
3. Run `python tools/docs-gen/regen.py --skip-crate-graph`, and commit only the generated blocks this campaign introduced. Restore the others.
4. Ledger: set every packet row to its final state, and rewrite RESUME.md to "campaign closed".

The orchestrator then:

- empties `.claude/state/active-campaign`;
- has ops retire every `ah-*` worktree (`rm-worktree.ps1 --merged`);
- has ops post the board handoff.

**Checks:** the CRLF check on every edited `.md`.

**Docs owed:** the status docs (once), and unified-uat.
