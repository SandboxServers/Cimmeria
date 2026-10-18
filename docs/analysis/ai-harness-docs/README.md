# ai-harness-docs: explain and harden the AI harness

Ledger for the ai-harness-docs campaign. Packets: [work-packets.md](work-packets.md) (dispatch rules, contract, waves 1-2) and [work-packets-w3.md](work-packets-w3.md) (waves 3-4). Resume card: [RESUME.md](RESUME.md). Survey and the owner's binding decisions: [survey-and-decisions.md](survey-and-decisions.md). Draft reply to Aura's #1797: [pr-1797-reply.md](pr-1797-reply.md).

## Purpose

Contributors who start Claude Code here the usual way get the `orchestrator` persona: it has no shell, and nothing at the top level says so or explains the workflow behind it. The hooks call bare `python`, so on a host with only `python3` they exit 127 and the guards turn off without a word. The campaign:

- makes a fresh session explain its own mode (a banner and a CLAUDE.md block);
- makes the hooks find an interpreter, and say so when there is none;
- documents the whole AI cycle for humans in the README, with the numbers behind it;
- aligns the cycle docs, the skill and the agent files with the owner's cycle;
- reconciles the platform statements and answers Aura's #1797;
- adds a drift guard, so the hook and agent tables can't go stale.

Acceptance is a fresh-clone smoke test (AH-15).

## Findings

Verified against origin/main da1703d01 on 2026-10-18 unless marked otherwise.

| # | Finding | Evidence | Packet |
|---|---|---|---|
| F1 | `.claude/settings.json` sets `"agent": "orchestrator"` for every launch, including headless `claude -p`. Nothing at session start tells the user. | settings.json line 2; no banner hook | AH-03, AH-06 |
| F2 | All four hook commands run bare `python`. Where only `python3` exists they exit 127, which is non-blocking, so `shell_guard` and `orchestrator_guard` turn off silently. | settings.json (2 entries), orchestrator.md frontmatter, .codex/hooks.json | AH-02, AH-03 |
| F3 | python3-first ordering would break the common Windows case: the Microsoft Store `python3.exe` alias stub exists where python.org installs ship no `python3.exe`. Ordering has to depend on the OS. | Windows install layout | AH-02 |
| F4 | `orchestrator_guard.py` refuses the orchestrator's writes to a worktree's `docs/analysis/**` when its cwd is the main checkout: `under_dirs()` matches only prefixes relative to the root, and a worktree target starts `.claude/worktrees/<name>/...`. That is why the survey had to live in the main checkout. | orchestrator_guard.py:88-123 | AH-01 |
| F5 | There is no packet-writer agent; the architect writes `work-packets.md`. | .claude/agents (24 files) | AH-07, AH-08 |
| F6 | The cycle docs conflict with the owner's cycle. "Don't send the implementing worker back for round two" (development-workflow.md:112); one review-fixer per wave with no re-check (SKILL.md §4); packet-coder stops at about 90k with no continuation rule (packet-coder.md:23). The 3-session Haiku cap and the Sonnet escalation exist only in personal memory. | files cited | AH-07, AH-08, AH-09 |
| F7 | The platform statements disagree with each other and with owner decision 4. Some say "Windows-native" or "No WSL builds", others "PowerShell or Git Bash". | 13 sites, listed in AH-11 | AH-11 |
| F8 | #1797 calls WSL "not supported for interactive sessions", which conflicts with decision 4. Its content is otherwise accurate and useful, including the ops `blocked` reason. | `gh pr diff 1797` | AH-11, pr-1797-reply.md |
| F9 | #1799 changes only `senpai-review.yml` (the `@login-kun` fix) and doesn't touch anything this campaign changes. | `gh pr view 1799` | none |
| F10 | No "nopjmp" or "Linux pipeline" text exists in the repo, so there is nothing to drop. AH-11 re-greps to confirm. | repo grep 2026-10-18 | AH-11 |
| F11 | The README is 406 lines, and lines 105-295 are the generated crate graph. A section placed after it is effectively invisible. | README.md | AH-14 |
| F12 | CLAUDE.md is 3,893 bytes against a cap of 4.5 KB (doc-update-map row 18). #1797 adds about 200 bytes. | file size | AH-06, AH-11 |
| F13 | CodeRabbit runs on the free OSS tier: when the quota is spent it posts "You've used all free OSS reviews for now… Next included review available in N minutes". It auto-pauses after 5 reviewed commits. Senpai is opt-in only, at about $1.50 a run against a budget of about $30. Neither bot is a merge gate. | .coderabbit.yaml:10,43,232; senpai-review.yml; a PR comment | AH-13 |
| F14 | Several memory-only rules a contributor elsewhere needs are missing from rules-and-gotchas: the Haiku session cap, adversarial review of every PR, no `--target-dir target/desktop`, foreground lanes with `Set-Location <wt>;`, running agents as defined, never rehosting the client archive, and the reasons behind the model roles. | memory audit, survey table | AH-12 |
| F15 | Generated blocks follow the `GENERATORS` dict pattern in `tools/docs-gen/regen.py`, and a sibling module loads lazily (as `tools_catalog.py` does). regen.py skips `.claude/` when scanning Markdown but can read files there. | regen.py:275-299 | AH-05 |
| F16 | The numbers in the README must cite repo sources: `docs/analysis/campaign-metrics/findings.md` and the token-usage section of development-workflow. The memory-only #1312 figure isn't cited. | findings.md lines 11-44, 96 | AH-14 |

## Decisions

All rows are decided by the architect except where marked owner. Status: Decided unless noted.

| ID | Decision | Reason |
|---|---|---|
| D-AH1 | "Packet writer" is the architect writing `work-packets.md`; no new agent is added. A big campaign may run the architect twice, once for design and once for packets. | A packet writer needs the design context the architect already holds. An extra agent would add about 35-40k of fixed overhead per run (campaign-metrics) for no gain. |
| D-AH2 | **Coder loop:** at most 3 packet-coder (Haiku) sessions per packet. A session that stops on context writes a worknote. The next session is fresh, continues from the worknote and the commits, and never receives review findings. | Owner decision 6 ("3x loop for the context limit"). A fresh session costs less than a long one: Haiku 5.5 bills 5x per request above a 100k prompt. Findings go to the review-fixer, so development-workflow.md:112 stays true. |
| D-AH3 | **Coder escalation:** if the third session also stops, or a packet fails its lane checks after 5 fix rounds twice, one Sonnet implementer continues from the worknote: `rust-gameserver-dev` for code and tools, `documentation-writer` for docs. If that also fails, the architect re-slices or re-specs the packet. Opus never implements. | The overstep data: $107.43 spent on Opus coding (O1 $76.48 and O3b $27.43, campaign-metrics findings.md:33-39). Escalating one tier at a time is cheaper. |
| D-AH4 | **Reviewer passes:** packet-review-fixer returns `ship`, `recheck` or `decision` per packet. `recheck` means it made a non-trivial fix that needs fresh eyes. The orchestrator then dispatches a fresh review-fixer for that packet alone, with at most 3 passes in total. | Owner decision 6 ("reviewer, up to 3 times"). A reviewer that checked its own fix is not adversarial. This keeps D-LO9 (one Sonnet review-fixer per wave, lean-orchestration ledger); the 2026-10-12 memory variant, where findings went to a fresh Haiku, predates D-LO9 and is retired. |
| D-AH5 | **Reviewer escalation:** after a third pass that still isn't `ship`, or on any `decision` verdict, the architect (Opus) writes a ruling note into `worknotes/<packet>.md` and writes no code. For a protocol or RE question the matching Opus advisor writes the note instead, read-only. A fresh Sonnet review-fixer then applies the ruling. Anything that is the owner's call becomes BlockedDecision. | Keeps the rule that Opus writes no code, while putting the strongest model on the judgment calls. |
| D-AH6 | "Fix round" means a rerun of the lane checks inside one session (at most 5). "Pass" means one review-fixer session (at most 3). "Session" means one coder run (at most 3). The docs use these three words consistently. | The agent files say "five fix rounds" while the owner says "up to 3 times"; they count different things. |
| D-AH7 | **Launcher:** add `.claude/hooks/py.sh`, POSIX and sourced: `set -- <script> <args>; . py.sh`, so it costs no extra process. The search order depends on `$OSTYPE`: msys or cygwin gets `python py python3`, everything else `python3 python`. Lookup uses the `hash` builtin (it finds and caches the path, so `exec` doesn't search PATH again), then `exec`. With no interpreter it prints a one-line stderr warning and exits 1, which is non-blocking but visible. A Store-stub-only Windows host remains a known limitation, and the banner reports it. | Owner decision 3. Sourcing and builtins keep the added cost near zero under Git Bash, where every fork is expensive. An exit code of 2 would block every tool call on a machine with no Python. |
| D-AH8 | **Launcher go/no-go:** keep the launcher only if its median added time is at most 5 ms **and** at most 10% per hook call, measured over 30 runs (`measure_hooks.ps1`). Otherwise AH-02 returns `blocked` with the numbers, AH-03 does not wire it, and the question goes to the owner (conditional BlockedDecision). | Owner decision 3 asks for "no meaningful time" and gives baselines of about 50-80 ms for shell_guard and about 44 ms for bare Python. Measurement noise under Git Bash is a few ms, so 5 ms is the smallest difference worth calling real. |
| D-AH9 | **Banner:** `.claude/hooks/mode_banner.sh` in pure sh, with no Python and no forks. It is the first SessionStart entry, with no matcher, so it also fires after a compaction. It prints `systemMessage` for the user and `additionalContext` for the model. Its wording holds whether or not the payload carries `agent_type`, and it warns when no Python is found. | Owner decision 1. A Python banner would vanish on the very hosts it has to warn. |
| D-AH10 | No Codex banner. AGENTS.md gets the parity text instead. | Codex has no default persona, so there is no hidden mode to announce. Codex's SessionStart context injection is still unproven (#1742 probe). |
| D-AH11 | The CLAUDE.md block is about 300 bytes, placed right after the H1 line. The file must stay at or under 4,500 bytes, counting #1797's pointer once AH-11 trims it. | Owner decision 1, and the cap in doc-update-map row 18. |
| D-AH12 | **orchestrator_guard: allow worktree ledgers.** Edit and Write are allowed under `<root>/.claude/worktrees/<name>/` followed by a writable prefix (`docs/analysis`, `.claude/state`, `.claude/reports`). Read gets the same for `docs/analysis`. `realpath` already resolves `..`, so escapes still fail. | F4. The orchestrator's job includes keeping the ledger in the campaign worktree, and denying it pushes ledgers into the stale main checkout. Worktree code stays denied. |
| D-AH13 | The README section is `## How AI development works here`, between "Tests & CI" and "Quick Start", at most about 80 lines. Full detail goes in a new reference doc, `docs/agents/ai-harness.md`. | F11. The owner asked for a "top-level visible section", and the soft cap of 500 lines applies to the README. |
| D-AH14 | **Drift guard:** `tools/docs-gen/harness_tables.py` with two generators. `harness-hooks` is the README hooks table: every hook in settings.json, agent frontmatter and .codex/hooks.json, with the first docstring or comment line of its script. `harness-agents` is the agent table in ai-harness.md. There is a test that every hook script states its reason, and a doc-update-map row for harness changes. | Owner open item. A table regenerated from the config files cannot drift, and the docstring test makes "every hook with its reason" enforceable. |
| D-AH15 | Only AH-13, AH-14 and AH-16 run `regen.py`; AH-14 stacks on AH-13. Each commits regenerated output only for the markers it adds, and restores every other file regen touched (`git restore`). W1 packets that add tools/ scripts leave the tools catalog stale (main's regen-docs workflow refreshes it every 6 h). | The docs-gen README rule (lines 53-55): a PR commits regen output only for a marker it adds. regen.py has no path filter, and two parallel packets that both regenerate `tools/README.md` conflict on the gen block, which ops can't resolve. |
| D-AH16 | **Platform wording** (exact, contract C7): fully supported is Windows with PowerShell 7. WSL, Linux, and Git Bash on Windows are not validated, but not unsupported. Agents stay on PowerShell, the validated path. An agent with no PowerShell returns `blocked: no PowerShell (pwsh) on this host`. | Owner decision 4. Agent rules choose the validated path; they don't declare the others unsupported. |
| D-AH17 | **#1797:** ask Aura to change his status line to decision 4 and land it first, with credit to him. AH-11 then stacks on main after it. If #1797 closes without merging, AH-11 carries its content with a `Co-authored-by` line for him. Posting the reply is an owner action (draft: pr-1797-reply.md). | His content is accurate and useful. Only the status sentence conflicts. Landing his PR first avoids conflicting edits to the same section. |
| D-AH18 | **ops gets no `.sh` path** (Aura's question 2). | One validated path. ops is Haiku at low effort with 30 turns, and a second shell would double its rule surface. The `.sh` twins already exist for humans. Revisit after the WSL smoke run. |
| D-AH19 | **The orchestrator default stays for all launches** (Aura's question 3). | No setting scopes `agent` by interactivity. Headless and CI runs pass `--agent <name>` or `--setting-sources user`, as Senpai already does. The banner (D-AH9) and the reminder (AH-06) remove the confusion. |
| D-AH20 | #1799 is independent and can land as it is. | F9. The Senpai docs in AH-13 stay true either way. |
| D-AH21 | **Review bots:** both stay (owner decision 7). AH-13 documents what each does, when it runs and what it costs. For the CodeRabbit limit: wait for the reset time it names, push in batches (ship pushes once), resume with `@coderabbitai resume` after its 5-commit auto-pause, and request one review with `@coderabbitai review`. Copilot stays unrequested (the 2026-10-04 decision). | F13. |
| D-AH22 | `docs/guides/autonomous-agent-kickoff.md` gets a status note only. Rewriting or retiring it is a follow-up for its owner. | It describes one operator's WSL host. Rewriting it goes beyond this campaign's evidence. |
| D-AH23 | **Acceptance:** a fresh-clone smoke test (AH-15) on Windows, required, and on WSL, best effort. It is run by the owner or by a hands-on session the owner approves. | Owner open item. It needs a human-launched Claude Code session with no memory, which no packet agent can be. |
| D-AH24 | Credential extraction is out of scope (owner decision 8). | Owner decision 8. |
| D-AH25 | The README cites only figures that are in the repo (F16). It leads with the cost per 1,000 changed lines, a 2.5x saving, and states the "work differs" caveat. | findings.md:13 and :25. The per-PR figures overstate the gain. |

### BlockedDecision (for the orchestrator to ask the owner)

- **None blocks a packet today.**
- **Conditional, D-AH8:** if AH-02 measures more than 5 ms or 10% added per call, ask whether to wire the launcher anyway. The alternative is to document "install `python` on PATH".
- **Conditional, D-AH17:** if #1797 is still open when AH-14 merges, ask whether to keep waiting for Aura or have AH-11 carry his content (path B, with a `Co-authored-by` line).
- **Owner action:** post the #1797 reply (pr-1797-reply.md), or let `ops` post it with the owner's OK.
- **Owner action:** run the AH-15 smoke test, or approve a hands-on session to run it.

## Packet status

| Packet | Wave | Agent | Title | Depends on | Status |
|---|---|---|---|---|---|
| AH-01 | 1 | packet-coder | orchestrator_guard allows worktree ledgers | - | Ready |
| AH-02 | 1 | packet-coder | `py.sh` launcher, launcher tests, timing harness and measurement | - | Ready |
| AH-03 | 1 | packet-coder | Wire the launcher into the 4 hook sites, plus the mode banner hook | AH-02 (stack; go result) | BlockedDependency |
| AH-05 | 1 | packet-coder | `harness_tables.py` generators and tests | - | Ready |
| AH-06 | 2 | packet-coder | Orchestrator hands-on reminder and routing; CLAUDE.md mode block | W1 merged | BlockedDependency |
| AH-07 | 2 | packet-coder | packet-coder continuation and review-fixer verdicts | W1 merged | BlockedDependency |
| AH-08 | 2 | packet-coder | campaign-packet skill: cycle and escalation ladder; ship-pr §5 note | W1 merged | BlockedDependency |
| AH-09 | 2 | packet-coder | development-workflow worker lifetime, plus AGENTS.md parity | W1 merged | BlockedDependency |
| AH-11 | 3 | packet-coder | Platform reconciliation, after #1797 | #1797 merged or closed; W2 merged | BlockedDependency |
| AH-12 | 3 | packet-coder | rules-and-gotchas promotions | W2 merged | BlockedDependency |
| AH-13 | 3 | documentation-writer | `docs/agents/ai-harness.md` reference, review bots and index row | W2 merged | BlockedDependency |
| AH-14 | 3 | documentation-writer | README section, doc-update-map row and regen | AH-13 (stack) | BlockedDependency |
| AH-15 | 4 | owner / hands-on | Fresh-clone smoke test (acceptance) | W3 merged | BlockedDependency |
| AH-16 | 4 | packet-coder | Close-out | AH-15 | BlockedDependency |

AH-04 and AH-10 were folded into AH-03 and AH-06 during slicing; their IDs are not reused.
