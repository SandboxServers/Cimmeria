# README "how AI development works here": owner notes

Source: the user, 2026-10-18. These are binding input for the README docs packet (AH-12/AH-13 follow-up). The packet writer expands them into prose with numbers from PR #1665 and `hook-timing.md`.

## Corrections to the current text

1. **The review loop is packet-reviewer, then back to a packet-coder.** `packet-review-fixer` is not part of the process. Cycle per packet: packet-coder, then the read-only `packet-reviewer`, then, if it has findings, a packet-coder gets the reviewer's notes and fixes them, then the reviewer re-checks. The loop limit is three passes.
2. **Waves of packets are not explained.** Packets are grouped in waves. Independent packets of a wave run in parallel, each in its own worktree.
3. **Wave-boundary review.** At the end of each wave a review loop (same loop limit) looks for cross-packet findings that no single-packet review can see (contract drift between packets, duplicated helpers, merge-order hazards).
4. **Campaign-boundary review.** One more review loop at the end of the campaign looks for long-reach issues (docs vs. behaviour, status docs, things only visible once everything has merged).

## Additions requested

5. **Why we block PowerShell and bash commands, and what that saves.** Explain the hooks that refuse direct cargo calls, raw git, and similar commands, and send them through the build lane and the `ops` scripts. State the cost saved (tokens, rework, CI minutes, wedged worktrees) with numbers; source the hook timing from `hook-timing.md` and PR #1665.
6. **Campaigns: what they are, why we use them, what purpose they serve.** A campaign is the unit for work too big for one PR: a ledger folder, a goal, decisions, waves, packets, a RESUME card that survives compaction.
7. **Anything else a person off the street needs.** Open list; see below.

## Open: candidate topics for "someone off the street"

Proposed by the orchestrator, for the owner to accept or strike:

- **Who does what:** orchestrator (dispatches, keeps the ledger, no shell, never reads source), architect (design, packets), packet-coder (Haiku, prescriptive), packet-reviewer, domain advisors, ops (one git/worktree operation), documentation-writer, lab-driver. Why each has a narrow tool set.
- **Why prescriptive packets for a small model:** cost, and the packet is the contract.
- **Why one worktree and one test database per agent.**
- **Reports on disk and one-line returns:** why the orchestrator never reads reports unless blocked.
- **Continuations:** the ~90k-token stop and "continuation N of 3".
- **The ship path:** `ship pr`, merge queue, green CI required, retire worktrees on merge.
- **When a human decides:** owner decisions, lab use, anything destructive, client patches, new opcodes.
- **Lab and UAT:** why live runs are owner-gated and driven by Haiku.
- **Project memory, the board, and the docs map:** how agents find facts and why status docs change once per campaign.
- **Honest numbers:** what it costs, what it saves, what has not been measured.
- **Platform support:** Windows with PowerShell fully supported; WSL/Linux and bash "not validated, but not unsupported" (decision 4).

## Follow-ups this implies (not yet done)

- The agent definitions and the orchestrator persona text still name `packet-review-fixer` and "one review-fixer per wave". They must change to the reviewer, coder, three-pass loop, plus the wave and campaign reviews. Needs an owner go-ahead before the persona is edited.
- `work-packets*.md` and `campaign-packet` skill wording about the fixer should be reconciled in the same change.
- The README packet text in `work-packets-w3.md` must be re-keyed to these notes before dispatch.
- Note for the packet writer: docs are CRLF; this file was written by the orchestrator and needs normalising.
