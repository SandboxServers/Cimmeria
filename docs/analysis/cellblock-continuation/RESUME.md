# Cellblock continuation resume

> **Type**: Reference. **Audience**: The Cellblock orchestrator.
> **Last updated**: 2026-10-16 (wave 1b designed).
> **Companions**: [Ledger](README.md), [work packets](work-packets.md), [W1b coder packets](w1b-packets.md).

## State

- Runs under the adopting orchestrator (D-CC1 = B). Feature SHA `e10fd1f0a` (W1a integrated). Not pushed; one PR at CB-N08.
- Integrated: CB-N00, CB-N01, CB-N02s, CB-N02a. CB-N05a merged to main (#1734). UATPending: CB-U03 for 1026/1027.
- Review rule (D-CC6): read-only `packet-reviewer`, fixes by `packet-coder`, fresh re-review; at most 3 rounds.
- Old worktrees `cellblock-cb-n02s` and `cellblock-cb-n02a` are cherry-picked (no PR); the owner retires them.

## Wave 1b (no packet in flight yet)

Five packets in parallel, one worktree each, on `e10fd1f0a`:

| Packet | Agent | Worktree |
|---|---|---|
| CB-N02b (chain 1028, door button) | packet-coder | cellblock-cb-n02b |
| CB-N02c (chain 1036, Ambernol vial) | packet-coder | cellblock-cb-n02c |
| CB-N02d (chain 1037, drone re-arm) | packet-coder | cellblock-cb-n02d |
| CB-N02s2 (restore bit check) | mission-systems-advisor | cellblock-cb-n02s2 |
| CB-N03s (flank/cover seal) | npc-ai-spawn-advisor | cellblock-cb-n03s |

## Next actions

1. Ops: five `mk-worktree` calls (branches in work-packets.md § W1b-R0), then the `rebase --onto` step. If ops refuses, dispatch W1b-R0 (packet-coder).
2. Dispatch the five packets in one message, in the background (sections: w1b-packets.md for N02b/c/d, work-packets.md for N02s2/N03s).
3. On return: reviewer A (CB-N02b/c/d) and reviewer B (CB-N02s2, CB-N03s), both `packet-reviewer`. Fix verdicts go to a `packet-coder` per worktree, then a fresh reviewer.
4. All `ship`: dispatch W1b-INT in zone-cellblock (cherry-picks plus the header and id-allocation bookkeeping).
5. If CB-N02s2 or CB-N03s seals children: architect pass for W1c. Otherwise W2 (CB-N04).
6. Lab rows CB-U03 (relog at 2114/2115/2116/2145/2144) and CB-U05: ask the user first, then run once W1b is integrated.

## Open questions (none block W1b)

- Who owns the cell-door presentation after a relog (sequence 1749): CB-N02s2 answers, likely asset audit CA-03C/D.
