# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-18 (architect, after the DK-08 review) against `main` @ `d1bba0400`. Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no lab use by any agent until the campaign ends; the owner runs M1, M2 and every UAT himself. No packet-review-fixer: one read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder` (Haiku), the reviewer re-checks; at most 3 passes.
- **Wave 1 coded, not shipped.** Worktrees dk-06a (379b8af80), dk-07 (5b35e77f6), dk-08 (591fa37d1), dk-12a (400f4ed47). DK-08 review: `.claude/worktrees/dk-08/.claude/reports/dakara/DK-08-review.md`.
- **Decided now:** D-DK18 (4902 objectives are three client-hinted regions on the remains, point sets 2128-2130, seeded by DK-09) and D-DK19 (DK-10c 8060-8074, DK-10d moved to 8160-8179, later missions 8180-8499). DK-08's open questions 1 and 2 are superseded by them.
- **New packet DK-09b** (wave 2, Ready, no gate): the region-key lint reads every `point_sets*.sql` file and moves to its own file (F20). Can run now.
- **Merged so far:** DK-00 to DK-05 (#1282 to #1288).
- **Do not touch** `.claude/state/active-campaign`: it points at another campaign.

## Next action

1. packet-coder on dk-07: rewrite its `docs/analysis/four-zone-planning/id-allocation.md` row to the wave-1.md DK-07 text (mission 1570 8040-8074 and 8160-8179, later missions 8180-8499, 8075-8079 unassigned). Optional on dk-08: a one-line note atop the worknote's open questions 1-2 pointing at D-DK18/D-DK19.
2. packet-reviewer re-checks dk-07, then ops ships wave 1 as one PR (`ship pr -C <wt>`), merge on green CI, retire the worktrees the day it merges.
3. Dispatch DK-09b now (ops `mk-worktree dk-09b`, branch in its table in packets/wave-2.md).
4. Architect, before the rest of wave 2: expand DK-06b from DK-06a's finding and rule D-DK10.
5. Architect, before DK-10d: fill its Part A (8160-8163) after DK-06b. DK-10c and DK-12c rows are filled.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| 1 | DK-06a, DK-07, DK-08, DK-12a | none (coded) |
| 2 | DK-06b, DK-09, DK-09b, DK-10a, DK-10e | DK-06a + D-DK10; DK-08 + DK-12a; none; D-DK1; D-DK17 |
| 3 | DK-10b, DK-10c, DK-10d, DK-11, DK-12b, DK-12c | DK-10a + DK-09 + DK-09b; DK-10d also DK-06b and Part A |
| 4-6 | stubs in work-packets.md | D-DK5 after the owner's M2 playtest |

## Open decisions

- **D-DK1** (owner, BlockedDecision): who gives the Free Jaffa their first mission. Option E (2026-10-18) adds three bridges to B (README § D-DK1 option E); B with E2 recommended (a Teal'c letter plus Rak'nor's hail; adds DK-10f). Blocks DK-10a to DK-10e and DK-11.
- **D-DK17**: the owner chose the proposed wording (2026-10-18); DK-10e still waits on D-DK1.
- **D-DK5** (owner, BlockedDecision, after M2): reconstructed fights. Blocks wave 4 onward.
- **D-DK10** (architect, after DK-06a): the radio beat. An owner call only if a client patch is the only way.
- **D-DK2** (owner confirmation, non-blocking): Omega Site only at the start.

## UAT owed

- M1 (DK-U1 to DK-U10) and M2 (after wave 3): the owner runs them; no agent lab use.
