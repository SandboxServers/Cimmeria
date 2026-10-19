# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-18 (architect; D-DK1 decided as option F, D-DK17 and D-DK20 ruled) against `main` @ `d1bba0400`. Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no lab use by any agent until the campaign ends; the owner runs M1, M2 and every UAT himself. No packet-review-fixer: one read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder` (Haiku), the reviewer re-checks; at most 3 passes.
- **Wave 1 Done:** DK-06a (#1854), DK-07 (#1853), DK-08 (#1855), DK-12a (#1856). DK-08 review: `.claude/worktrees/dk-08/.claude/reports/dakara/DK-08-review.md`.
- **D-DK1 decided: option F (owner, 2026-10-18):** authored level-1 mission 63100 "Home to Dakara" via `MISSION_ADDITIONS` (catalog resync at character select, no patch) plus seed rows; Rak'nor's shipped 6110 completes it; 8040 then starts 1570 at 4906. Packets DK-10fs (wave 2) and DK-10fc (wave 3); DK-10a, DK-10b, DK-10e, DK-07 amended. Fallback B+E1 kept in the packet text.
- **D-DK17 decided:** proposed wording and Teal'c's letter as written (DK-10e, chain 8007).
- **D-DK20 decided:** accept being first user of the mission-catalog route (63100 is the first production `MISSION_ADDITIONS` entry).
- **Merged so far:** DK-00 to DK-05 (#1282 to #1288); DK-06a, DK-07, DK-08, DK-12a (#1853 to #1856).
- **Do not touch** `.claude/state/active-campaign`: it points at another campaign.

## Next action

1. Dispatch wave 2 now: DK-09b, DK-10a, DK-10e, DK-10fs (packet text in packets/wave-2.md and packets/wave-2-dk-10fs.md), and DK-09 (DK-08 and DK-12a are merged).
2. Architect: expand DK-06b from DK-06a's finding and rule D-DK10; DK-06b stays BlockedDecision until then.
3. After wave 2 is reviewed and merged, dispatch wave 3 per its dependencies; DK-10d Part A also waits on DK-06b.

## Waves

| Wave | Packets | Gate |
|---|---|---|
| 1 | DK-06a, DK-07, DK-08, DK-12a | Done (#1853 to #1856) |
| 2 | DK-06b, DK-09, DK-09b, DK-10a, DK-10e, DK-10fs | DK-06b: D-DK10; DK-09: DK-08 and DK-12a merged (met); the rest: Ready |
| 3 | DK-10b, DK-10c, DK-10d, DK-10fc, DK-11, DK-12b, DK-12c | DK-10a + DK-09 + DK-09b; DK-10d also DK-06b and Part A; DK-10fc also DK-10fs |
| 4-6 | stubs in work-packets.md | D-DK5 after the owner's M2 playtest |

## Open decisions

- **D-DK10** (architect, after DK-06a): the radio beat. Blocks DK-06b. **D-DK5** (owner, after M2): reconstructed fights. **D-DK2** (owner confirmation, non-blocking).

## UAT owed

- M1 (DK-U1 to DK-U10), M2 with the bridge checklist (DK-U11 to DK-U16): the owner runs them; no agent lab use.
