---
name: campaign-packet
description: Run a multi-PR effort as a coordinated campaign - a ledger under docs/analysis/<campaign>/, prescriptive work packets, one worktree and one implementing agent per packet, an adversarial review of each diff, then ship, merge and retire. Use when the user says "design this as a campaign", "split this into packets", "dispatch workers", "resume the <name> campaign", or when a change is too big for one PR. Also use when writing or reviewing a work-packets.md, worknote or ledger.
---

# Campaign and work packets

The coordinator plans, writes packets, dispatches, reviews, fixes and merges.
Workers each implement one packet and stop. Canonical rules:
[docs/agents/development-workflow.md](../../../docs/agents/development-workflow.md)
(worker lifetime, notifications, shared docs, definition of done).

## 0. Make sure nobody else is running it

A ledger saying "nothing built yet" doesn't mean nobody is working on it.
Before creating worktrees, writing rules files or dispatching:

- Check `git worktree list`, campaign branch creation times and
  `git log --since="1 hour ago"` on campaign branches.
- Check the agent board for the campaign's subcategory and recent handoffs.
- If a peer session is live on it, coordinate with it and stand down. If the
  peer has been idle for more than about an hour, don't message it (its cache
  has expired, so a wake re-bills its whole context). Tell the user it's ready
  to resume instead.
- List a shared folder before writing into it. Running workers read those
  files.

## 1. Ledger

Create `docs/analysis/<campaign>/` (example: `docs/analysis/bank-vault/`):

- `README.md`: purpose, findings with a packet column, a **Decisions** table
  (`D-XX1`... with status and reason), and a packet status table using
  **Ready / BlockedDependency / BlockedDecision / Writing / Review /
  Integrated / UATPending / Done**.
- `work-packets.md`: dispatch rules, then the **contract** parallel packets
  build against (exact type names, function signatures, schema), then one
  section per packet.
- `worknotes/` and `handoffs/` for per-packet and per-session handoffs.

Create the board subcategory once: `board campaign create "<name>"`. Get
owner decisions marked BlockedDecision answered before dispatching packets
that depend on them.

## 2. Write packets prescriptively

A small model (or a fresh agent with no history) should finish a packet in
under about 100k tokens. Each packet names:

- the exact files and functions to change, with code shapes for anything
  non-obvious;
- the test to add, its type per [TESTING.md](../../../TESTING.md), and why it
  fails when the change is reverted;
- its lane checks in order (`fmt`, then `clippy`, then tests, each `-p` the
  crate; live-DB through `live-db-test.ps1`);
- the docs rows it owes ([doc-update map](../../../docs/agents/doc-update-map.md));
- the branch, worktree name and commit subject.

Design-heavy, RE or docs packets go to the defined domain agents
(`rust-gameserver-dev`, `game-archaeology-specialist`,
`documentation-writer`) rather than a small model.

## 3. Dispatch

Per packet, from the main checkout:

```powershell
pwsh tools/build-lane/mk-worktree.ps1 <campaign>/<packet>-<slug> <worktree-name>
```

- One worktree and one test database (`sgw_<worktree>`) per worker. Never
  run two implementers in one checkout.
- Implementer: the [`packet-coder`](../../agents/packet-coder.md) agent (a
  Haiku-tier coder that edits only the named files, compiles through the lane
  and commits locally). A packet that needs judgment goes to
  `rust-gameserver-dev` with the same brief instead.
  The brief carries the worktree path, the packet section, and the commit
  message with attribution lines.
- **Workers use PowerShell and the lane only.** No bash, no direct `cargo`,
  no `git worktree prune`, no `git stash`.
- A worker reports once, with results not progress, and doesn't wait around
  (parked workers re-bill their cache). Review fixes go to a fresh worker or
  to the coordinator, never back to the original implementer.

## 4. Review, fix, ship

1. Run an adversarial read-only reviewer on the packet's diff: the
   [`packet-reviewer`](../../agents/packet-reviewer.md) agent (Sonnet tier),
   plus the matching domain advisor or `server-authority-enforcer` when the
   packet touches their area. Give it the worktree, the commit range and
   the packet spec.
2. The coordinator verifies each finding, fixes what's real in the worktree,
   and reruns the packet's lane checks.
3. Ship and merge with the `ship-pr` skill (`ship.ps1 pr -C <worktree>`, then
   `ship.ps1 merge <PR> --retire <worktree>`). Merge on the minimum CI.
4. Update the packet's row in the ledger and write
   `worknotes/<packet>.md` if anything is left over.

## 5. Status docs and close-out

- `docs/gap-analysis.md`, `docs/gap-analysis/` and `docs/project-status.md`
  change **once**, in the close-out packet, never per packet.
- Close-out also updates `docs/guides/unified-uat.md` with the UAT steps the
  owner still has to run, retires every worker worktree
  (`pwsh tools/build-lane/rm-worktree.ps1 --merged`), and posts a handoff to
  the board.
- Coordinators compact at wave boundaries once the ledger and resume note
  are current.
