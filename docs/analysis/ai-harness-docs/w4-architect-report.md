# ai-harness-docs: wave 4 architect report (2026-10-18)

Brief: design the next wave, which rewrites the README AI section and the rules docs for a newcomer, retires `packet-review-fixer`, and slices the work into Haiku packets, with spicy ideas kept as optional packets the owner can veto. Base: origin/main d1bba0400. I wrote files only in this ledger folder, and I changed no source or docs outside it.

## Design in one paragraph

From now on, review is a loop (D-AH43, contract C12). A fresh, read-only `packet-reviewer` writes numbered fix notes (C13). A fresh `packet-coder` applies them ("review fix, pass N of 3"). Then a fresh reviewer runs the next pass, with at most 3 passes per loop. There are three loops:

- a packet loop, which runs as soon as the packet's coder is done;
- a wave loop, which runs once every packet in the wave has `ship` and looks only for findings that span packets;
- a campaign loop, which runs before close-out and looks only for long-reach findings.

If pass 3 still says `fix`, or a reviewer returns `decision`, the architect writes a ruling note, one Sonnet implementer applies it, and one fresh reviewer checks it. The work ships in three steps:

- **AH-W40** lands this ledger and puts C12 in force before any code changes.
- **W4a** (AH-17 to AH-22) retires the agent everywhere in one PR from the integration worktree `ah-w4a`, and adds the drift test.
- **W4b** (AH-23) adds `docs/agents/ai-sdlc.md`, "How we build with AI", which covers every topic in readme-notes.md. The README section becomes its one-screen summary.

**W5** follows: the campaign loop (AH-24), the owner's fresh-clone run (AH-15) and the close-out (AH-16).

## Key decisions

- **D-AH42: the agent file is deleted, not deprecated.** A deprecated agent still appears in `/agents`, in the Agent tool's menu and in Codex's list, and Haiku and Sonnet dispatchers choose agents by name and description. A deprecation line is one more thing a cheap model can misread. Git history keeps the file. `tools/token-profile` keeps the name in its config, tests and coordination report, so past campaigns keep their cost roles. AH-22's test fails CI on any stale mention outside a generated block.
- **D-AH43 to D-AH45:** the loop (C12), the report format (C13), and W4a as one PR, so `main` never holds a half-retired agent.
- **D-AH46:** the newcomer page is a new Explanation doc. The README is at 465 lines against its soft cap of 500, and ai-harness.md stays the reference.
- **D-AH47:** `tools/agent-skills/test_agent_refs.py` (4 tests). `agent-skills.yml` also runs on changes to README.md, AGENTS.md, CLAUDE.md and docs/agents/ (F46).
- **Superseded or amended:** D-AH4 is superseded. D-AH2 and D-AH5 are partly superseded. D-AH6, D-AH29, D-AH34 and D-AH41 are amended. C6 in work-packets.md carries a supersede note. work-packets-w3.md:216 now names packet-coder and packet-reviewer.

## Owner-visible callout

AH-19 edits the default persona (`.claude/agents/orchestrator.md`). The W4a PR body opens with a callout. After merge, every Claude Code session must be relaunched: a session that is still running keeps the old persona and will try to dispatch an agent that no longer exists. Until W4a merges, coordinators follow C12 over the persona text. Dispatch rule 1 and RESUME.md say so.

## BlockedDecision and owner actions

- **D-AH33:** is the Linux or WSL leg of AH-15 required (my recommendation) or best effort? Ask before AH-15.
- **D-AH48 (veto; applies as written unless the owner objects):** after pass 3, an architect ruling instead of a fourth Haiku pass.
- **D-AH29 (default no):** a bash path for packet-coder, packet-reviewer and the build lane.
- **S1 to S6:** a yes or no for each (see below).
- **Owner actions:**
  - AH-11i: post pr-1797-close.md and close #1797.
  - After W4a merges: `rm-worktree.ps1 ah-17 ah-18 ah-19 ah-20 ah-21 --force` (F43).
  - AH-15: the cold read and the fresh-clone run.

## Risks the dispatcher must handle

- **F39:** persistent-player-data's RESUME.md still plans "one review-fixer each" for its waves 2 and 3, and its wave-2 worktrees exist. Before W4a merges, tell its coordinator or edit its card, and confirm that no review-fixer is still running there. This is dispatch rule 2.
- **F42:** AH-11h (W3c) and AH-20 edit adjacent lines in rules-and-gotchas.md and development-workflow.md. W3c must merge before the W4a worktrees are cut.
- **F43:** `rm-worktree --merged` skips packet branches that merged only into an integration branch. That is why the owner runs the `--force` retire. S5 would fix this.
- **F46/F47:** 22 files outside docs/analysis name the retired agent. AH-17 to AH-22 cover every in-scope file, and 5 token-profile files are kept as history.
- **Order:** AH-W40 must ship before any W3c worktree is cut. Its guard requires the last ledger commit to be 3b2a99d6ee5d87c0ab834ef886c400967d22e84c.

## Spicy ideas for owner veto (each an optional packet, own PR)

| ID | Idea | Cost |
|---|---|---|
| S1 | shell_guard refuses `git worktree prune` and `git stash`, with a test-first revert proof | 1 Haiku packet; added hook latency not measured |
| S2 | Mermaid diagram of the loops in ai-sdlc.md | about 10k Haiku |
| S3 | "A day in the life": AH-11m (#1842) followed from brief to merge | 1 documentation-writer run, about 35k |
| S4 | token-profile counts "review fix, pass" briefs as `fixer` (F41) | architect expansion, then 2 packets |
| S5 | `rm-worktree --merged-into <PR>`, plus the matching ops line | architect expansion, then packets |
| S6 | `test_ledgers.py`: every RESUME.md is at most 40 lines (all 6 pass today) | about 15k Haiku |

Considered and not recommended:

- a live cost dashboard in the README, because the numbers go stale;
- a log of refused calls;
- lowering the reviewer's effort.

The reasons are in README § Spicy ideas for owner veto.

## Measured and not measured

**Measured.** The new text cites only these:

- hook latency (hook-timing.md);
- campaign-metrics PR #1665: Sonnet reviews cost $13.58 and Haiku fix rounds $0.72; 17 of the 20 packets needed at least one fix round; $107.43 was spent on Opus coding;
- the token-profile ledgers.

**Not measured, and labelled that way in the packets and PR bodies:**

- the saving per call the guards refuse;
- the cost of a fix pass under the new loop;
- S1's added latency;
- whether a newcomer understands ai-sdlc.md;
- the Mermaid rendering;
- Codex running with the regenerated tomls.

No campaign has yet run the new loop end to end. The first run is this campaign's W3c, then W4b and W5.

## Files written (all CRLF, no generated-block markers)

All are in `C:\Users\Steve\source\projects\Cimmeria\docs\analysis\ai-harness-docs\`:

- **New:** work-packets-w4.md (AH-W40, W4a AH-17 to AH-22, rules R1-R11, contracts C12-C16, ship procedure), work-packets-w4b.md (AH-23, AH-24, AH-15/16 addenda, S1-S6), w4a-pr-body.md, w4b-pr-body.md, and this report.
- **Updated:** README.md (F38-F47, D-AH42-D-AH49, packet table, spicy section), RESUME.md (32 lines), work-packets.md (supersede notes), work-packets-w3.md (line 216).
- **Normalised to CRLF only:** w4-drafts.md, readme-notes.md.

All of them are uncommitted in the main checkout. AH-W40 copies them into its worktree.
