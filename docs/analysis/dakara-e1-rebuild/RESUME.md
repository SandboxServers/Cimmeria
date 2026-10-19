# Dakara_E1 rebuild: resume card

> Type: how-to. Updated 2026-10-19 (architect: owner **option C, 2026-10-19** for the SGC end; wave 6d written). Status source: [README § Packets](README.md#packets).

- **Owner rules (2026-10-18):** no agent lab use until the campaign ends; the owner runs every UAT. One read-only `packet-reviewer` per wave, its findings to a fresh `packet-coder`. Do not touch `.claude/state/active-campaign`.
- **Merged:** DK-00 to DK-05; waves 1 to 6b (see README); DK-99a #1914; wave 6c DK-40b #1929.
- **Wave 6c step 2, committed, review pending:** DK-14c `794010891` (`dk-14c`), DK-15b `42611f3b0` (`dk-15b`), DK-30b-r `28376d808` (`dk-30b`, on `9343b4f33`/`da57539d5`), DK-33-r `11bb681a9` (`dk-33`, on `7682e2023`/`ddd2bb14a`); all on `e7459ddcb`. DK-32c Ready (`dk-32c`).
- **Dormant by design (D-DK23):** 1648 to 1654 stay unreachable in play until 1647 (wave 7) ships.

## Option C (owner, 2026-10-19)

- Gate travel to the SGC is real dialling; this campaign builds the SGC end: [sgc-return-plan.md](sgc-return-plan.md), **wave 6d**.
- DK-SGC-01: gate 27 arrival pinned to P13 (167.00, -10.80, 18.00), yaw 0 (D-DK44, D-DK47).
- DK-SGC-02: template 37 "SGC Gate Control" gets `INT_DHD` (D-DK40, D-DK46); the way-back guard covers every address grant.
- D-DK2 reading accepted: start grant Omega only; gate 27 is a story grant.

## Next action, in order

1. Wave 6c step 2: one `packet-reviewer` for the four commits; fixes to a fresh `packet-coder`. Ship DK-14c and DK-15b; hold the other two. DK-32c (step 1) still to dispatch.
2. **Wave 6d now, in parallel:** `ops` makes worktree `dk-sgc` from `main` (branch `dakara/dk-sgc-return`). DK-SGC-01, its review loop to `ship`, then DK-SGC-02 in the same worktree; DK-SGC-02's review covers both commits (the wave review). One PR.
3. Through `ops`, post the [§ Notice](sgc-return-plan.md#notice-to-the-union-sgc-coordinator) to the Union SGC topic before the DK-SGC PR opens. It revises their D-US9 and asks them to fix `rig.rs` ARRIVAL.
4. Merge order: DK-SGC PR; then the DK-30b PR after US-30 (#1937) and DK-SGC; the DK-33 PR after US-31 (#1935) and DK-SGC.
5. **DK-99e** once DK-14c, DK-15b, DK-32c and DK-SGC merge (adds DK-U35 to DK-U37).
6. Wave 7 after the M2 playtest; then DK-99b to DK-99d (D-DK38).

## Owner checklist

- [x] **D-DK40**: the SGC way back: option C, built by DK-SGC-02 (owner, 2026-10-19).
- [x] **D-DK44**: gate 27 arrival pin, P13, built by DK-SGC-01 (owner, option C, 2026-10-19).
- [x] D-DK2 reading: Omega the only start grant, gate 27 by story (owner, option C, 2026-10-19).
- [ ] **D-DK48** (non-blocking; the SGC campaign's D-US6): should the SGC story departures become hand dials? Recommended no (keep Harriman).
- [x] D-DK21 to D-DK23, D-DK32 to D-DK39, D-DK41, D-DK42, D-DK45 accepted (owner, 2026-10-19).

## UAT owed (owner)

- M1 DK-U1 to DK-U10 and M2 DK-U11 to DK-U17: runnable now. M3 DK-U18 to DK-U25, DK-U32: 1647 GM-completed; Livewire after DK-14c/DK-15b. M4/M5 DK-U26 to DK-U31, DK-U33: after the held PRs merge; DK-U34 after DK-32c; DK-U35 to DK-U37 after the DK-SGC PR. Placement DK-P1 to DK-P9.
