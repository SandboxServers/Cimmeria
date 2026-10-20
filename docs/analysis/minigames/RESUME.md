# Minigames resume card

> Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md) and [packets/](packets/). Updated 2026-10-19 at gate G1.

## State

- Waves 0A, 0B and 1 are **integrated** into `minigames/integration` (worktree `mg-integration`). Merge shas: R1 `e476cd854`, R4 `026527b36`, R3 `3dffcbab1`, R2 `b9e45cfc2`, R5 `0fe7355ac`, MG-06 `4647624c2`, R7 `e9644d779`, R6 `95756b98b`, MG-02 `8b15eeb0a`. Their packet worktrees are retired.
- **G1 done.** Decisions D-MG17-D-MG22 in the README; D-MG3 = A, D-MG6 = A, D-MG11 default off. No owner decision blocks a packet.
- Not applicable: MG-23, MG-25, MG-26, MG-65. MG-63 is replaced by MG-63a and MG-63b (wave 2).

## Owner directives

- Autonomous to release-ready. No push or PR until the campaign is done, then one PR from `minigames/integration`.
- Every packet branches from `minigames/integration`; ops merges it back with `merge-noff.ps1` after its review says `ship`, then retires the packet worktree and its target dir.
- Lab use is ask-first (D-MG16). Status docs change only in MG-99.

## Next action

Dispatch in one message (each in its own worktree off `minigames/integration`):

- wave 2: **MG-22**, **MG-63a** ([packets/wave2-launch.md](packets/wave2-launch.md));
- wave 3: **MG-AL1**, **MG-CG1**, **MG-GC1a**, **MG-CH1**, **MG-LW1** ([packets/wave3-5-ports.md](packets/wave3-5-ports.md));
- wave 6: **MG-64** ([packets/wave6-7-content-close.md](packets/wave6-7-content-close.md)).

Then, as each predecessor merges: MG-21 after MG-22, MG-24 after MG-21, MG-63b after MG-63a and MG-24; GC1b after GC1a; each game's next slice after its last.

## In flight (updated mid-wave 2-5)

Merged since G1: MG-GC1a, MG-CH1, MG-CG1, MG-64, MG-22 (GC1b ship-verdict, merge pending via ops).
- MG-63a: fix 0fa97dbce (hold-time anchor); pass 2 review + authority pass 2 running.
- MG-21 coder; MG-CH2 coder; MG-CG2 coder (CH2/CG2 need authority review too).
- MG-LW1 and MG-AL1: review-1 fixes being applied; pass 2 next.
- Next: GC2 coder after GC1b merge; AL2a after AL1; CG3 after CG2; CH3 after CH2; MG-24 after MG-21; MG-63b after 63a+24; then waves 6 (MG-62, MG-61) and 7.
- Carry to campaign review: unported games run PlaceholderGame with the 60 s countdown (ends in defeat); listener_tests.rs at 692 lines (split when next touched); gen-block regen in MG-99.

## Open owner decisions

None blocking. Follow-ups for the owner at close-out: D-MG11 default on after lab UAT; an offer path for mission 582 (M10).
