# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-19 (architect, autonomous: waves 4 to 7 written). Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no agent lab use until the campaign ends; the owner runs every UAT. One read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder`. Do not touch `.claude/state/active-campaign`.
- **Done:** DK-00 to DK-05 (#1282 to #1288); wave 1 (DK-06a #1854, DK-07 #1853, DK-08 #1855, DK-12a #1856; DK-08 review: `.claude/worktrees/dk-08/.claude/reports/dakara/DK-08-review.md`); waves 2 and 3 (wave 3: #1881, #1883, #1885, #1887, #1889, #1892).
- **Shipping:** wave 3b, DK-06b PR #1894 and DK-10d-B PR #1896 (reviewed).
- **Dormant by design (D-DK23):** waves 5 and 6 build now; in play nothing past 1646 is reachable until 1647 (wave 7) ships.

## Next action, in order

1. Now: dispatch **DK-41** ([wave 4](packets/wave-4.md)) and **DK-31a** ([wave 6](packets/wave-6.md), desk note), each in its own worktree.
2. When PR #1894 and PR #1896 have both merged: **DK-10d-A** (wave 3c, findings folded in, [wave 3](packets/wave-3.md)) and **DK-40** (wave 4), worktrees from that `main`.
3. When DK-40 and DK-41 have merged: wave 5 (**DK-13, DK-14a, DK-14b, DK-15**) and wave 6 (**DK-30a, DK-30b, DK-31b, DK-32a, DK-33**) together, one worktree each; one reviewer per wave.
4. DK-30b and DK-33: open the PRs, **hold the merge** until the Union SGC campaign's US-30 and US-31 merge (F26).
5. After DK-31a and DK-31b: the architect expands **DK-31c** (wave 6b).
6. Wave 7 (GDK1, DK-20, DK-21, DK-32b) waits for D-DK5, then the M2 playtest.

## Morning owner checklist

- [ ] **D-DK5** (blocks wave 7): A Castle-style fights (recommended), B peaceful placeholders, C stop at 1646, D skip the fights. [Options](packets/wave-7-blocked.md#d-dk5-options-owner).
- [ ] Review the autonomous decisions **D-DK21 to D-DK31** and the D-DK2 amendment (README § Decisions); each can be reversed before its wave merges.
- [ ] D-DK25: closing the 1649 offer accepts it, whatever button is pressed. OK?
- [ ] D-DK28: 1650 step 4927 moves on entering Moh'katan's tent area, not on a click. OK?
- [ ] D-DK26, D-DK27: 1651 and 1653 auto-accept (no blurbs exist). OK?
- [ ] D-DK27: 1653 step 4937 reuses radio dialog 5849 (check in DK-U29).
- [ ] D-DK29: west/east naming of the ring controls and supplies is a map-axis guess (DK-U27).
- [ ] Vocuum (1653 O5643, optional): pick a prop and a place, or leave it out.
- [ ] Merge holds: DK-30b and DK-33 wait on the Union SGC campaign's US-30 and US-31.
- [ ] D-DK2: confirm Omega Site as the only granted address (non-blocking).

## UAT owed (owner)

- M1 DK-U1 to DK-U10; M2 DK-U11 to DK-U17 (after wave 3c); M3 DK-U18 to DK-U25 (after wave 5; GM-complete 1647 first); M4/M5 DK-U26 to DK-U31 (after wave 6 and US-30/US-31). Includes carrier markers (DK-U25), the Command Terminal reach (DK-U22) and the DHD crossings (DK-U26, DK-U30).
