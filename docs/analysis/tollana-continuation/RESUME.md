# Tollana campaign resume

> **Title:** Tollana campaign resume
> **Type:** Reference
> **Audience:** The Tollana orchestrator
> **Last updated:** 2026-10-18
> **Companions:** [Ledger](README.md), [W1 packets](w1-packets.md), [W1 frozen literals](w1-frozen.md)

Branch `content/tollana-campaign` in `.claude\worktrees\zone-tollana`, tip `1052e739c`. **Local only: no PR and no push yet.** No agent uses the lab (user directive, 2026-10-17).

## W1 status

- W1a-W1d are all INTEGRATED on the campaign branch.
- Packets done: T-02a, T-03a1-a4, PH-01a, PH-01b. Two review rounds where fixes were needed.
- Checks at W1d-INT: fmt ok, clippy ok, nextest 1234 passed, live-DB mission_568 20 passed.
- All packet worktrees are retired.

## Next actions

1. The user runs manual UAT on zone-tollana's server: P1-P10 placement, U1-U7 568 flow, T18 double-click accept. U-PH1..5 only after PH-04.
2. Owner decisions: O1 568 reward; Decline button on 2153; D-PH1..D-PH7; proposals 618 -> Moh'katan and 621 -> Ba'al.
3. Campaign PR only after T-14 close-out (one remote PR per execution contract).

## Held

T-03b/c, T-04..T-11, T-05a/b, T-08e/T-09, T-11s, T-12/T-13, T-14, PH-02..PH-05. Reasons in [README.md](README.md#decisions-and-packet-status).
