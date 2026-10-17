# Cellblock continuation resume

> **Type**: Reference. **Audience**: The Cellblock orchestrator.
> **Last updated**: 2026-10-17 (wave 2 designed).
> **Companions**: [Ledger](README.md), [work packets](work-packets.md), [W2 packets](w2-packets.md).

## State

- Runs under the adopting orchestrator (D-CC1 = B). W1a and W1b integrated; feature SHA `0fca3114f`. Not pushed; one PR at CB-N08.
- UATPending: CB-U03 (relog at 2114/2115/2116/2144/2145; chains 1026-1028, 1036, 1037).
- W1c not triggered: CB-N02s2 and CB-N03s sealed no children. CB-N03 is BlockedEvidence on CB-U05.
- Review rule (D-CC6): read-only `packet-reviewer`, fixes by `packet-coder`, fresh re-review; at most 3 rounds. Haiku agents under 100k tokens.
- Owner retires the cherry-picked W1 worktrees (`rm-worktree.ps1 <name> --force`): cellblock-cb-n02s, -n02a, -n02b, -n02c, -n02d, -n02s2, -n03s.

## Wave 2 (no packet in flight yet)

Sequential, cluster 1 of CB-N04 only ([w2-packets.md](w2-packets.md)):

| Step | Packet | Agent | Worktree |
|---|---|---|---|
| 1 | CB-N04s (cast seal) | npc-ai-spawn-advisor | cellblock-cb-n04s on the W2 architect commit |
| 2 | CB-N04m (mission sign-off) | mission-systems-advisor | cellblock-cb-n04s |
| 3 | CB-N04a (spawns, holder dialog, barks, tests) | packet-coder | cellblock-cb-n04a on the CB-N04s tip |
| 4 | Review, fixes, re-review | packet-reviewer / packet-coder | per packet |
| 5 | W2-INT | packet-coder | zone-cellblock |

Cast rows are `is_stationary` and outside Region1 (D-CC12). Claimed IDs: spawns 520-522, dialog 60160, screens 200160-200162, chains 11010-11012 (id-allocation.md, CB-N04 claims).

## Next actions

1. Ops: `mk-worktree.ps1 cellblock/cb-n04s-cast-seal cellblock-cb-n04s`, then `rebase --onto <W2 architect commit> HEAD cellblock/cb-n04s-cast-seal`.
2. Dispatch CB-N04s. If the verdict is `no safe pocket`, CB-N04 becomes BlockedEvidence on CB-U08; ask the user for a lab run.
3. Dispatch CB-N04m in the same worktree. On `approve` or `change`: ops makes `cellblock-cb-n04a` on the CB-N04s tip (branch `cellblock/cb-n04-<room>`); dispatch CB-N04a.
4. One reviewer for all three; then W2-INT.
5. Lab rows CB-U03, CB-U05, CB-U08 and CB-U09: ask the user first (D-CC3).

## Open decisions

- D-CC11 (owner): CB-N07's acceptable death, difficulty and time impact. Ask after CB-U05 and CB-U09 have run.
