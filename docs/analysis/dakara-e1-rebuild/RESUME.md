# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-19 (architect: owner rulings on D-DK24 and D-DK31, wave 6c written; owner answers recorded; D-DK45 decided, DK-32c added). Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no agent lab use until the campaign ends; the owner runs every UAT. One read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder`. Do not touch `.claude/state/active-campaign`.
- **Merged:** DK-00 to DK-05; waves 1 to 3; 3b (#1894, #1896); 3c #1897; 4 #1898, #1899; 5 #1903 to #1906; 6 DK-31a #1900, DK-30a #1907, DK-31b #1909, DK-32a #1910; 6b DK-31c #1913; DK-99a #1914; docs #1901.
- **Held (no PR):** `dk-30b` (DK-30b `34abe129b`, DK-30b-n `8b1ac4673`) and `dk-33` (DK-33 `41686b50c`, DK-33-n `ffec26d33`). Both get their wave-6c `-r` commit and ship as one PR each. **Dormant by design (D-DK23):** 1648 to 1654 stay unreachable in play until 1647 (wave 7) ships.

## Owner answers (2026-10-19)

- D-DK5 = **A** (Castle-style fights): wave 7 waits on the owner's M2 playtest, then a re-slice.
- D-DK2 = Omega Site only as the **start** grant; gate 27 is now a story grant (D-DK24). Flagged as a conflict; reading recorded in D-DK2. **Still awaits the owner's answer** (D-DK2 against D-DK24).
- D-DK24: **real gate travel** for 1651 and 1654, SGC address granted (D-DK43): wave 6c DK-30b-r, DK-33-r.
- D-DK31: **Livewire required** on the terminal hacks (D-DK42): wave 6c DK-14c, DK-15b. **Owner (2026-10-19): Livewire on both terminals, difficulty 1; D-DK42 accepted.**
- D-DK25 to D-DK30 accepted (D-DK29 still checked in DK-U27). **D-DK45 decided (owner delegated, 2026-10-19):** the Vocuum (1653 O5643) is a hologram pad (template 457) at the courtyard centre, spawn 8098; place LOW-MEDIUM, DK-U34 corrects it. Built by **DK-32c**.
- **Accepted (owner, 2026-10-19):** all other autonomous decisions, D-DK21 to D-DK23 and D-DK32 to D-DK45, except those marked BlockedDecision on owner input.
- **Still awaiting the owner:** D-DK2 against D-DK24, **D-DK40** (SGC dial-in with no way back) and **D-DK44** (gate 27 arrival pin).

## Next action, in order

1. Wave 6c step 1: **DK-40b** and **DK-32c**, new worktrees `dk-40b` and `dk-32c` from `main` ([packets](packets/wave-6c.md)); one reviewer for the two; PR and merge each. Step 2 waits only for DK-40b.
2. `ops` rebases `dakara/dk-30b` and `dakara/dk-33` onto that `main` and records the SHA.
3. Step 2: **DK-14c**, **DK-15b** (new worktrees), **DK-30b-r** (`dk-30b`), **DK-33-r** (`dk-33`); one reviewer for the four. Ship DK-14c and DK-15b; hold the other two.
4. **DK-99e** once DK-14c, DK-15b and DK-32c merge ([wave 99](packets/wave-99.md#dk-99e-uat-guide-refresh-for-wave-6c)).
5. Tell the Union SGC campaign: C2/C3 now arrive by gate 27 (D-DK44 arrival pin), and ask if this answers its D-US4/D-US6 return trip.
6. Open the held PRs when US-30/US-31, D-DK40 and D-DK44 clear. Wave 7 after the M2 playtest. Then DK-99b to DK-99d (D-DK38).

## Owner checklist

- [ ] **D-DK40**: a player can dial the SGC off-story and be stuck there (no DHD). Accept, or wait for the SGC way back? Gates the DK-30b-r merge.
- [x] **D-DK42**: Livewire on both terminals at difficulty 1, decode stays a click. Accepted (owner, 2026-10-19).
- [ ] **D-DK44**: gate 27 arrival pin on Dakara for C2/C3. Accept, or choose another arrival? Gates the held PRs.
- [ ] D-DK2 reading: Omega stays the only start grant; gate 27 by story only. OK?
- [x] D-DK45: Vocuum prop and place delegated to the architect (owner, 2026-10-19): hologram pad at A-14 (DK-32c). D-DK41 (no hack cursor on the terminals, DK-U25) is accepted.
- [x] D-DK21 to D-DK23 and D-DK32 to D-DK39 accepted (owner, 2026-10-19); each can still be reversed before its packet merges.

## UAT owed (owner)

- M1 DK-U1 to DK-U10 and M2 DK-U11 to DK-U17: **runnable now**. M3 DK-U18 to DK-U25 and DK-U32: GM-complete 1647; Livewire after DK-14c/DK-15b. M4/M5 DK-U26 to DK-U31, DK-U33: the SGC legs after the held PRs merge; DK-U34 (Vocuum) after DK-32c. Placement DK-P1 to DK-P9.
