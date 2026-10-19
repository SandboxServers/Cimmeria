# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-19 late (architect, autonomous: wave 6b and the close-out written). Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no agent lab use until the campaign ends; the owner runs every UAT. One read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder`. Do not touch `.claude/state/active-campaign`.
- **Merged:** DK-00 to DK-05; waves 1 to 3; 3b (#1894, #1896); 3c DK-10d-A #1897; 4 DK-40 #1898, DK-41 #1899; 5 DK-13 #1903, DK-14a #1904, DK-14b #1905, DK-15 #1906; 6 DK-31a #1900, DK-30a #1907, DK-31b #1909, DK-32a #1910; docs #1901.
- **Held (reviewed, PR not opened):** DK-30b `34abe129b` in `dk-30b` until SGC US-30; DK-33 `41686b50c` in `dk-33` until SGC US-31 (F26).
- **Dormant by design (D-DK23):** 1648 to 1654 are merged but unreachable in play until 1647 (wave 7) ships.

## Next action, in order

1. Now, wave 6b ([packets](packets/wave-6b.md)): **DK-31c** in a new worktree `dk-31c` (branch `dakara/dk-31c-hatak-sequences`) from `main`; **DK-30b-n** in `dk-30b` and **DK-33-n** in `dk-33`. One `packet-reviewer` for the three; ship DK-31c when clean.
2. After DK-31c merges: **DK-99a** (unified UAT guide, worktree `dk-99a`, [wave 99](packets/wave-99.md#dk-99a-the-unified-uat-guide)).
3. When US-30 / US-31 merge: open and merge the DK-30b and DK-33 PRs (with their nit commits).
4. When the owner answers D-DK5: the architect re-slices wave 7 ([wave 7](packets/wave-7-blocked.md)); under C, wave 7 drops.
5. DK-99b and DK-99c (one PR, worktree `dk-99`) when every shipping code packet has merged (D-DK38); then DK-99d through `ops`.

## Morning owner checklist

- [ ] **D-DK5** (blocks wave 7 and sets the close-out date, D-DK38): A Castle-style fights (recommended), B peaceful placeholders, C stop at 1646, D skip the fights. [Options](packets/wave-7-blocked.md#d-dk5-options-owner).
- [ ] Review the autonomous decisions **D-DK21 to D-DK39** and the D-DK2 amendment (README § Decisions); each can be reversed before its packet merges.
- [ ] D-DK25: closing the 1649 offer accepts it, whatever button is pressed. OK?
- [ ] D-DK28: 1650 step 4927 moves on entering Moh'katan's tent area, not on a click. OK?
- [ ] D-DK26, D-DK27: 1651 and 1653 auto-accept (no blurbs exist). D-DK27 reuses radio 5849 at step 4937 (DK-U29).
- [ ] D-DK29: west/east naming of the ring controls and supplies is a map-axis guess (DK-U27).
- [ ] D-DK33 to D-DK36: Ha'taks show on entering Dakara during 1652 (not on accept), explode 3 s after each click, stay gone after a relog. OK?
- [ ] D-DK37: the UAT guide's placement rows become DK-P1 to DK-P9. D-DK39: delete all 22 merged `dakara/*` remote branches at close-out.
- [ ] Vocuum (1653 O5643, optional): pick a prop and a place, or leave it out.
- [ ] Merge holds: DK-30b and DK-33 wait on the Union SGC campaign's US-30 and US-31. If those never land, close out without them (D-DK38)?
- [ ] D-DK2: confirm Omega Site as the only granted address (non-blocking).

## UAT owed (owner)

- M1 DK-U1 to DK-U10 and M2 DK-U11 to DK-U17: **runnable now**. M3 DK-U18 to DK-U25: GM-complete 1647 first. M4/M5 DK-U26 to DK-U31: after wave 6b; the SGC legs (DK-U26, DK-U30) after US-30/US-31. Placement checks DK-P1 to DK-P9 once DK-99a lands. DK-U27 also asks whether the Ha'tak fire keeps burning.
