# ai-harness-docs: work packets, wave 4c (a Bash path on Linux and WSL for every agent with a shell)

Continued from [work-packets-w4.md](work-packets-w4.md): its dispatch rules (rule 0 is the main-red gate), **§ W4 rules (R1 to R11)** and C12 to C14 apply to every packet here, with R2 as amended below. Ledger: [README.md](README.md), decision D-AH53. Why: the owner's answer 5 of 2026-10-18 ([w4-owner-answers-report.md](w4-owner-answers-report.md)): D-AH29's Bash path is yes, but only on WSL and Linux; on Windows the default stays PowerShell.

**When:** after W3c (AH-11g puts C10 in ops.md, and AH-11h writes the section AH-27 edits), W4a (AH-17 rewrites packet-reviewer.md and AH-18 the campaign-packet skill) and W4b (AH-23b creates ai-sdlc.md, which AH-27 edits) have merged. Three packets, file-disjoint, in parallel; one PR from the integration worktree `ah-w4c`.

**Host:** every packet in this file is `Host: windows`. They edit the Linux rules; they do not run on Linux.

**R2 for W4c:** PowerShell only, never Bash; this wave's workers run on Windows. No `cargo` at all. Never run `git stash`, `git push`, `git worktree prune`, `git reset`, `git checkout`, `git restore` (except where a step names it) or `gh pr`.

---

## Contract

### C17. The shell, by platform, for every agent with a shell (D-AH53)

An agent picks its shell from the `Platform:` line of its environment block (Claude Code writes it at session start: `win32`, `linux` or `darwin`; WSL reports `linux`). The mode banner is not the key: it names the persona and warns when Python is missing, and says nothing about the platform.

| Platform line | Tool | Commands |
|---|---|---|
| `win32` | PowerShell | `pwsh ... .ps1` and `pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 cargo ...`; never Bash there, not even Git Bash |
| `linux` (WSL reports `linux`) | Bash | `bash tools/build-lane/<script>.sh` with the same arguments, and `bash tools/build-lane/lane.sh cargo ...`; in Bash, start each call with `cd <wt> &&` where PowerShell would use `Set-Location <wt>;` |
| no `Platform:` line | as `win32` | as `win32` |
| any other (`darwin`, ...) | none | `blocked`, reason `platform <p> is not supported (Windows, Linux and WSL only)` |
| the needed tool is missing | none | `blocked`, reason `no PowerShell tool on win32` or `no Bash tool on linux` |

The `ops` paragraph of AH-11g (work-packets-w3.md step 2) is this table in prose; keep the two identical.

**A packet's host.** Every packet section carries a `Host:` line: `windows`, `linux` or `both`. A packet without one is `windows`. A `both` packet gives each command twice, as `pwsh:` and `bash:`.

- A coder whose platform does not match the host returns `blocked: packet host <h> does not match platform <p>`. `windows` matches `win32`, `linux` matches `linux`, `both` matches either.
- A coder never translates a command from one shell to the other, with one exception: the lane. `bash tools/build-lane/lane.sh <args>` and `pwsh -NoProfile -File <wt>\tools\build-lane\lane.ps1 <args>` are the same command (the twins take the same arguments), so a coder uses its platform's form of a lane command.
- The architect writes `Host: windows` on every packet unless the campaign runs on Linux.

**What else holds on Linux:**

- `shell_guard` covers the Bash tool too (the hook matcher is `Bash|PowerShell`), and it accepts `bash tools/build-lane/lane.sh cargo ...` as the lane (F48).
- One host per checkout. On Linux the lane's slots live under `~/.local/share/cimmeria-build` (`$LANE_ROOT`); on Windows under `%LOCALAPPDATA%\cimmeria-build`, which Git Bash's `lane.sh` shares. A Linux lane and a Windows lane never see each other's slots, so never run a Linux session on a `/mnt/c/...` checkout that Windows sessions also build in (F24).
- The live-DB scripts (`live-db-test.sh`, `reload-db.sh`) have not been run on Linux: not validated.
- The packet workers' Bash path is not validated until AH-15's Linux leg (L6) runs it.

### C7c. Platform wording, after W4c

AH-27 puts this in place of C7b (work-packets-w3b.md) in development-workflow.md § WSL and Linux hosts:

```text
**Status: Windows with PowerShell 7 is fully supported. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported.** The build lane, the ship scripts and every agent rule are validated on Windows with PowerShell 7. Every build-lane `.ps1` has a `.sh` twin, and CI runs shellcheck and the twins' tests on Linux. Agents pick their shell from the `Platform:` line of their environment: PowerShell on `win32` (never Git Bash), Bash and the `.sh` twins on `linux`, which is what WSL reports. A packet runs only on the host its `Host:` line names, Windows unless it says otherwise. "Not validated" means that no full session (orchestrator, packet workers, review, ship) has been run end to end on Linux or WSL, and that the live-DB scripts have not been run there. Expect the gaps below, and report new ones.
```

The short form (the README's `**Platform:**` line) gains one sentence; AH-27 step 3 has the text.

### C18. W4c file ownership and merge order

| Packet | Branch | Worktree | Files (plus what sync regenerates) |
|---|---|---|---|
| AH-25 | `ai-harness-docs/ah-25-agent-shells` | `ah-25` | `.claude/agents/packet-coder.md`, `.claude/agents/packet-reviewer.md`, `.claude/agents/architect.md`, `.claude/agents/game-archaeology-specialist.md`, `.claude/agents/social-systems-engineer.md`, new `tools/agent-skills/test_platform_rule.py`, `tools/agent-skills/README.md` |
| AH-26 | `ai-harness-docs/ah-26-skill-shells` | `ah-26` | `.claude/skills/lane-build/SKILL.md`, `.claude/skills/campaign-packet/SKILL.md`, `AGENTS.md` |
| AH-27 | `ai-harness-docs/ah-27-platform-docs` | `ah-27` | `docs/agents/development-workflow.md`, `docs/agents/rules-and-gotchas.md`, `docs/agents/ai-sdlc.md`, `README.md` (the `**Platform:**` line only) |

Merge order into `ai-harness-docs/w4c`: AH-25, AH-26, AH-27 (any order works: the files are disjoint). CLAUDE.md is not edited: it is about 4,423 bytes against its 4,500-byte cap, and it already points at the lane-build skill.

---

## AH-25: the agents' shell rule

Agent: packet-coder. C18 row AH-25. Size: about 25k working tokens. Host: windows. Commit subject: `feat(agents): packet workers and the architect pick their shell from the Platform line (AH-25)`.

**Rules:** R1, R2 for W4c, R3 to R10, and R11 on every agent file you edit. Read C17 first.

**Steps:**

1. `packet-coder.md` line 8: `tools: Read, Edit, Write, Grep, Glob, PowerShell` → `tools: Read, Edit, Write, Grep, Glob, PowerShell, Bash`.
2. `packet-coder.md`, the line that starts `- Shell: PowerShell only, never \`bash\`` (line 16; the whole line). Replace it with these two lines (remove the backslashes):
   ```text
   - Shell: read the `Platform:` line of your environment. On `win32`, use the PowerShell tool only, never Bash (not WSL, not Git Bash), and compile through `pwsh -NoProfile -File <worktree>\tools\build-lane\lane.ps1 cargo ...`, run from the worktree. On `linux` (which is what WSL reports), use the Bash tool, start each call with `cd <worktree> &&`, run the `.sh` twin of any repo script with the same arguments, and compile through `bash tools/build-lane/lane.sh cargo ...`. If your environment has no `Platform:` line, follow the `win32` rule. On any other platform return `blocked: platform <p> is not supported (Windows, Linux and WSL only)`; without the tool you need, return `blocked: no PowerShell tool on win32` or `blocked: no Bash tool on linux`. Never call `cargo` directly: every compiling command goes through the lane. Never run `git worktree prune`, `git stash`, `git push`, `gh pr` or anything that deletes a worktree.
   - Host: the packet's `Host:` line says where it runs: `windows` (also when there is no `Host:` line) needs `win32`, `linux` needs `linux`, `both` runs on either with the `pwsh:` or `bash:` form of each command. On a mismatch, return `blocked: packet host <h> does not match platform <p>`. Never translate a command between shells yourself, except the lane: `lane.sh <args>` and `lane.ps1 <args>` are the same command, so use your platform's form.
   ```
3. `packet-reviewer.md` frontmatter: `tools: Read, Grep, Glob, PowerShell` → `tools: Read, Grep, Glob, PowerShell, Bash`. Then find `` and `git -C <worktree> log` in PowerShell, with Read and Grep. `` and replace it with `` and `git -C <worktree> log`, with Read and Grep. Pick the shell from the `Platform:` line of your environment: PowerShell on `win32` (never Bash there), Bash on `linux` (which WSL reports), as `win32` when there is no `Platform:` line; on any other platform, return `blocked` with the reason `platform <p> is not supported (Windows, Linux and WSL only)`. ``
4. `architect.md` frontmatter line 6: insert `, Bash` after `PowerShell` (`tools: Read, Grep, Glob, Write, Edit, PowerShell, Bash, Agent, Skill, ToolSearch`). Then line 18, find `- Use PowerShell only, with absolute paths. Never run cargo directly; any compile check goes through \`tools/build-lane/lane.ps1\`.` (whole line, without the backslashes) and replace it with these two lines:
   ```text
   - Pick your shell from the `Platform:` line of your environment: PowerShell on `win32` (never Bash there), Bash on `linux` (which WSL reports), as `win32` when there is no `Platform:` line; on any other platform, stop and report `platform <p> is not supported (Windows, Linux and WSL only)`. Use absolute paths. Never run cargo directly; any compile check goes through `tools/build-lane/lane.ps1` on `win32` or `tools/build-lane/lane.sh` on `linux`.
   - Write `Host: windows` on every packet unless the campaign runs on Linux (`Host: linux`), and give a `Host: both` packet each command as `pwsh:` and `bash:` (the lane-build skill § Linux and WSL hosts).
   ```
5. `game-archaeology-specialist.md` line 89: find `(\`bash tools/build-lane/lane.sh cargo ...\`, see the \`lane-build\` skill)` and replace it with `(\`pwsh tools/build-lane/lane.ps1 cargo ...\` on Windows, \`bash tools/build-lane/lane.sh cargo ...\` on Linux and WSL; see the \`lane-build\` skill)`. Remove the backslashes.
6. `social-systems-engineer.md` line 80: find `the build lane: \`bash tools/build-lane/lane.sh cargo ...\`, with` and replace it with `the build lane: \`pwsh tools/build-lane/lane.ps1 cargo ...\` on Windows, \`bash tools/build-lane/lane.sh cargo ...\` on Linux and WSL, with`. Remove the backslashes.
7. New `tools/agent-skills/test_platform_rule.py` (stdlib `unittest`, CRLF like its neighbours, about 60 lines; repo root from `Path(__file__).resolve().parents[2]`). A helper reads an agent file and splits its frontmatter `tools:` line from the body. Tests, each with `subTest(agent=...)`:
   - `test_shell_agents_grant_both_shells`: for `packet-coder`, `packet-reviewer`, `architect` and `ops`, the `tools:` line lists both `PowerShell` and `Bash`, and the body contains `Platform:`, `win32`, `linux` and `platform <p> is not supported (Windows, Linux and WSL only)`.
   - `test_packet_coder_names_both_lanes_and_the_host_rule`: packet-coder's body contains `tools\build-lane\lane.ps1`, `bash tools/build-lane/lane.sh`, `Host:` and `does not match platform`.
   - `test_domain_agents_name_both_lanes`: `game-archaeology-specialist` and `social-systems-engineer` contain both `tools/build-lane/lane.ps1` and `tools/build-lane/lane.sh`.
   **Reverted** (the agent files as on `origin/main`), every test fails: packet-coder, packet-reviewer and architect have no `Bash`, and the domain agents name only `lane.sh`.
8. `tools/agent-skills/README.md`, the sentence that lists the tests (AH-22 wrote it; S6 may have extended it). Exactly one of these two finds matches; use it and say which in the report. If neither matches, return `blocked` with the current sentence (R6).
   - Before S6: find `` and `test_agent_refs.py` checks that the harness docs name only agents that exist. `` and replace it with `` `test_agent_refs.py` checks that the harness docs name only agents that exist, and `test_platform_rule.py` checks that every agent with a shell picks it from the `Platform:` line. ``
   - After S6: find `` and `test_ledgers.py` checks that every campaign `RESUME.md` fits the 40-line resume card. `` and replace it with `` `test_ledgers.py` checks that every campaign `RESUME.md` fits the 40-line resume card, and `test_platform_rule.py` checks that every agent with a shell picks it from the `Platform:` line. ``
9. R7: sync (it rewrites `.codex/agents/` for the five agents), then `python tools/docs-gen/regen.py --skip-crate-graph`: keep a change to `docs/agents/ai-harness.md` only if it is a row of these agents (the tools column); restore every other file it changed.

**Test type:** unit, an agent-contract drift guard (per TESTING.md). **Revert proof:** write step 7 first and run `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_platform_rule.py"` against the unchanged agents; record that all three tests fail. Then make steps 1 to 6 and record that they pass.

**Checks:**

- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK (with `test_ops_twins.py` from AH-11g and `test_agent_refs.py` from AH-22).
- `python tools/agent-skills/sync.py --check`: exit 0.
- R4 on every edited `.md`; R11 on the five agent files.
- `[IO.File]::ReadAllLines('<wt>\.claude\agents\packet-coder.md').Length` is the base count plus 1 (step 2 turns one line into two); architect.md likewise plus 1.

**Docs owed:** doc-update-map row "AI-harness wiring" (agents' frontmatter and rules); list it.

---

## AH-26: the skills and AGENTS.md

Agent: packet-coder. C18 row AH-26. Size: about 25k. Host: windows. Commit subject: `docs(skills): lane-build and campaign-packet carry the Linux and WSL shell rule (AH-26)`.

**Rules:** R1, R2 for W4c, R3 to R10, R11 on every file you edit. The skills are CRLF in the working tree; the Edit tool keeps that. R7 after the skill edits.

**Steps:**

1. `lane-build/SKILL.md` lines 8-9: find `` `lane.sh`; both share the slots) `` and replace it with `` `lane.sh`, which shares its slots on the same Windows host) ``.
2. `lane-build/SKILL.md`: find the line `## Live-DB tests` and insert directly above it this section, then one blank line:
   ````text
   ## Linux and WSL hosts

   Read the `Platform:` line of your environment. On `win32`, everything above holds: PowerShell and `lane.ps1`, never Bash, not even Git Bash. On `linux`, which is what WSL reports, use the Bash tool and the twin with the same arguments:

   ```bash
   cd <worktree> && bash tools/build-lane/lane.sh cargo check -p cimmeria-cell
   cd <worktree> && bash tools/build-lane/lane.sh --exclusive cargo clippy --workspace <EXCL> --all-targets -- -D warnings
   ```

   No `Platform:` line: follow `win32`. Any other platform: return `blocked: platform <p> is not supported (Windows, Linux and WSL only)`.

   - **A packet runs on the host its `Host:` line names**: `windows` (the default), `linux`, or `both` with each command given as `pwsh:` and `bash:`. A mismatch returns `blocked`; never translate a command, except the lane, whose twins take the same arguments.
   - **One host per checkout.** On Linux the slots live under `~/.local/share/cimmeria-build`, so a Linux lane and a Windows lane never see each other. Never run a Linux session on a `/mnt/c/...` checkout that Windows sessions build in.
   - **Not validated on Linux:** the live-DB scripts below (`live-db-test.sh`, `reload-db.sh`), and a full agent session. CI runs the twins' tests and shellcheck on Linux.
   ````
3. `campaign-packet/SKILL.md`, the "Each packet names:" list: find the line `- the branch, worktree name and commit subject.` and insert directly below it: `- its host: \`Host: windows\` (the default), \`linux\`, or \`both\` with each command given as \`pwsh:\` and \`bash:\` (the \`lane-build\` skill § Linux and WSL hosts).` without the backslashes.
4. `campaign-packet/SKILL.md` § 3, item 3. AH-11g appended a sentence to it, so replace the whole item with this one-liner (it must print `status=ok`):
   ```text
   Set-Location <wt>; $f='<wt>\.claude\skills\campaign-packet\SKILL.md'; $t=[IO.File]::ReadAllText($f); $nl= if($t.Contains("`r`n")){"`r`n"}else{"`n"}; $m=[regex]::Matches($t,'(?s)3\. Workers use PowerShell and the lane only:.*?returns `blocked`\.'); if($m.Count -ne 1){throw "count $($m.Count)"}; $new=@('3. Workers use the shell their platform names (the `lane-build` skill','   § Linux and WSL hosts): on `win32`, PowerShell and `lane.ps1`; on `linux`,','   which WSL reports, Bash and the `.sh` twins with the same arguments. Never','   Bash on Windows. A packet runs only on the host its `Host:` line names','   (Windows unless it says otherwise); a mismatch returns `blocked`. Always:','   the lane only, no direct `cargo`, no `git worktree prune`, no `git stash`,','   no push. They commit locally and stop.') -join $nl; [IO.File]::WriteAllText($f,$t.Remove($m[0].Index,$m[0].Length).Insert($m[0].Index,$new)); 'status=ok'
   ```
   If it throws `count 0`, AH-11g's sentence is not as expected: return `blocked` with the current item 3 text.
5. `AGENTS.md`, three finds (each exactly once):
   - line 33: `` `pwsh tools/build-lane/mk-worktree.ps1 <campaign>/<packet>-<slug> <name>`) `` → `` `pwsh tools/build-lane/mk-worktree.ps1 <campaign>/<packet>-<slug> <name>`; on Linux and WSL, `bash tools/build-lane/mk-worktree.sh` with the same arguments) ``;
   - line 35: `` and `pwsh tools/build-lane/rm-worktree.ps1`. `` → `` and `pwsh tools/build-lane/rm-worktree.ps1` (on Linux and WSL, their `.sh` twins with the same arguments). ``;
   - line 37: `` (`pwsh tools/build-lane/lane.ps1 cargo ...`) `` → `` (`pwsh tools/build-lane/lane.ps1 cargo ...` on Windows, `bash tools/build-lane/lane.sh cargo ...` on Linux and WSL) ``.
6. R7: sync (the `.agents/skills/` mirrors).

**Test type:** none new (skill and doc text); AH-25's `test_platform_rule.py` covers the agents, and `sync.py --check` the mirrors.

**Checks:**

- `python tools/agent-skills/sync.py --check`: exit 0.
- `Select-String -Path <wt>\.claude\skills\lane-build\SKILL.md -Pattern '## Linux and WSL hosts' -SimpleMatch`: one line, above `## Live-DB tests`.
- `Select-String -Path <wt>\.claude\skills\campaign-packet\SKILL.md -Pattern 'Workers use PowerShell' -SimpleMatch`: no line. `-Pattern 'Host: windows' -SimpleMatch`: one line.
- `[IO.File]::ReadAllLines('<wt>\.claude\skills\campaign-packet\SKILL.md').Length` is at most 270; lane-build at most 140.
- R4 on the three files; R11 on all three.
- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK.

**Docs owed:** doc-update-map rows "AI-harness wiring" (skills and AGENTS.md) and "The pre-PR checklist, build commands, or repo invariants" (lane-build's Linux section); list both. That row also names `.github/copilot-instructions.md`: leave it unedited (it is not an agent of this harness) and say so in the report.

---

## AH-27: the platform docs

Agent: packet-coder. C18 row AH-27. Size: about 20k. Host: windows. Commit subject: `docs(agents): agents use Bash and the .sh twins on Linux and WSL; platform status (AH-27)`.

**Rules:** R1, R2 for W4c, R3 to R10, R11 on every file you edit. All four files are CRLF. README.md: edit only the `**Platform:**` line; it is outside every generated block (R5).

**Steps:**

1. `development-workflow.md` § WSL and Linux hosts, the status paragraph. Find the span from `The build lane, the ship scripts and every agent rule are validated` through the first `Expect the gaps below, and report new ones.` after it (AH-11h wrote it from C7b; it must match once). Replace that span with everything in C7c after its bold first sentence. Keep the bold sentence.
2. Same section: find the bullet that starts `` - **`ops` runs the `.sh` twins.** `` (AH-11h's, the whole line) and replace it with these two lines:
   ```text
   - **Agents run the `.sh` twins on Linux and WSL.** On a host whose `Platform:` line says `linux` (WSL says `linux`), [`ops`](../../.claude/agents/ops.md), `packet-coder`, `packet-reviewer` and `architect` use the Bash tool and run `bash tools/build-lane/<script>.sh` and `bash tools/build-lane/lane.sh` with the same arguments as the `.ps1` commands. On `win32` they never use Bash. A packet runs only on the host its `Host:` line names, and a Windows packet on Linux returns `blocked`. Any other platform returns `blocked: platform <p> is not supported (Windows, Linux and WSL only)`.
   - **One host per checkout.** On Linux the lane keeps its slots under `~/.local/share/cimmeria-build`, so it never sees a Windows lane's slots. Never run a Linux or WSL session on a `/mnt/c/...` checkout that Windows sessions build in. The live-DB scripts have not been run on Linux.
   ```
   Then set the file's `Last updated` line to today.
3. `README.md`: find the line that starts `**Platform:**` (the whole line) and replace it with:
   ```text
   **Platform:** Windows with PowerShell 7 is fully supported; WSL, Linux, and Git Bash on Windows are not validated, but not unsupported. On Linux and WSL, agents use Bash and the `.sh` twins ([details](docs/agents/development-workflow.md#wsl-and-linux-hosts)).
   ```
4. `rules-and-gotchas.md`: find the line that starts `- **Agents use PowerShell, the validated path` (AH-11h's text, the whole line) and replace it with:
   ```text
   - **Agents pick their shell from the `Platform:` line.** PowerShell on `win32`, the validated path, and never Bash there; Bash and the `.sh` twins on `linux`, which is what WSL reports (CI tests the twins on Linux; no full agent session has run there). A packet runs only on the host its `Host:` line names. Any other platform returns `blocked` and says why.
   ```
   Then set line 3 to `> **Last updated**: <today's date>`.
5. `ai-sdlc.md` § Platforms: find the line that starts `**Status: Windows with PowerShell 7 is fully supported.` (the whole line) and replace it with the C7c text, its last sentence (`Expect the gaps below, and report new ones.`) replaced by `The known gaps: [development-workflow.md § WSL and Linux hosts](development-workflow.md#wsl-and-linux-hosts).`

**Test type:** none (docs only). AH-25 carries the test.

**Checks:**

- R4 on the four files; R11 on all four.
- `Select-String` development-workflow.md, rules-and-gotchas.md and ai-sdlc.md for `still need PowerShell`, `still use PowerShell only` and `alone also has a bash path`: no line.
- `Select-String -Path <wt>\README.md -Pattern '**Platform:**' -SimpleMatch`: one line; `git -C <wt> diff --numstat -- README.md` prints `1	1`.
- The anchor `#wsl-and-linux-hosts` resolves: `Select-String` development-workflow.md for `## WSL and Linux hosts`: one line.
- `Set-Location <wt>; python -m unittest discover -s tools/agent-skills -p "test_*.py"`: OK (AH-22's test scans these docs).

**Docs owed:** doc-update-map rows "AI-harness configuration" (a project rule) and "AI-harness wiring" (the README's platform line and the workflow prose); list both.

---

## W4c ship procedure

1. **Worktrees.** `ops` makes `ah-25`, `ah-26` and `ah-27` (C18), one dispatch each, in one message, after W3c, W4a and W4b have merged.
2. **Coders and packet loops.** Dispatch the three coders in one message. As each returns `done`, run its packet loop (C12).
3. **Integration worktree.** `ops`: `pwsh tools/build-lane/mk-worktree.ps1 ai-harness-docs/w4c ah-w4c`. Then three `ops` dispatches, one after another: `pwsh -NoProfile -File tools/build-lane/merge-noff.ps1 -C <ah-w4c abs path> -Branch ai-harness-docs/ah-<NN>-<slug> -ExpectBranch ai-harness-docs/w4c`, in the C18 order. A conflict means the files were not disjoint: stop, and dispatch `architect`.
4. **Wave loop.** A `packet-reviewer` at wave scope over `ah-w4c`, range `<merge-base with origin/main>..HEAD`, spec this file and the ledger README, report `wave4c-review-<N>.md`. The scope note: "the platform rule must read the same in ops.md, packet-coder.md, packet-reviewer.md, architect.md, the lane-build and campaign-packet skills, AGENTS.md, development-workflow.md, rules-and-gotchas.md, ai-sdlc.md and the README: the same platforms, the same refusal reasons, `linux` for WSL, no Bash on `win32`, and the `Host:` rule". On `fix`, one fresh `packet-coder` in `ah-w4c` applies all the notes.
5. **Ship.** After rule 0: `ops` runs `pwsh tools/build-lane/ship.ps1 pr -C <ah-w4c abs path> -m "AH-W4c: a Bash path on Linux and WSL for every agent with a shell" --title "AI harness: agents pick their shell from the Platform line; Bash on Linux and WSL (ai-harness-docs W4c)" --body-file <ah-w4c abs path>\docs\analysis\ai-harness-docs\w4c-pr-body.md`. A code PR (a `.py` test), so the merge waits for CI. Then `ship.ps1 merge <PR> --retire ah-w4c`.
6. **Retire the packet worktrees.** `ah-25` to `ah-27` merged only into `ai-harness-docs/w4c`. If S5 has merged, `ops` runs `pwsh tools/build-lane/rm-worktree.ps1 --merged-into <W4c PR> ah-25 ah-26 ah-27`; otherwise list them for the owner's `--force`.
7. **Ledger.** Set AH-25 to AH-27 to Integrated (the W4c PR number), rewrite RESUME.md, and tell the user in one line: relaunch Claude Code, because packet-coder, packet-reviewer and architect changed (a running session keeps the old agent text).
