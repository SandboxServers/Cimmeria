# UO-12: ruling on the 63808 § Gaps blocker

> **Type:** Reference (ruling note) · **Packet:** [UO-12](../UO-12.md) · **Worknote ruled on:** [UO-22-jaffa-stock.md](UO-22-jaffa-stock.md) § Gaps, line 262, at base `437c45eec`
> **Trigger:** UO-12 coder returned `blocked` under rule 12 · **Ruling:** re-spec rule 12, copy list 63808 as sealed, nothing dropped · **Date:** 2026-10-19

The owner asked for a decision, not a block. This ruling is an architect default the owner may overrule; the cost is given below.

## The blocker

UO-12 rule 12 said: stop if UO-22's § Gaps names one of lists 63808-63811. § Gaps line 262 reads "**63808 (tier 1 heavy):** no heavy weapon is in this list. Tier 1 has no eligible heavy weapon under rule 4." The coder stopped, made no edits and no commit, and found nothing else wrong: the UO-22 lists, the UO-18 spawns 63853-63856 and the UO-19 spawn 63857 all match the packet.

## R1. The 63808 line is expected, not a gap (decision)

**Ruling: copy list 63808 verbatim, all 20 rows. Keep template 63831 and spawns 63853 and 63857. No fallback item.**

- **63808 is not a heavy-weapons list.** Template 63831 is moniker 27224 `DN_npc_ven_OmegaSite_JaffaWeaponsTier01`, a general tier-1 Jaffa weapons vendor. UO-22 fills it with 20 rows by its round-robin (staff, zat'ni'katel, ribbon device, heavy): 7 staffs, 7 zat'ni'katels and 6 ribbon devices, tiers and prices checked (§ Checks: all pass). The vendor is full; only the heavy class is empty.
- **The empty heavy class is what D-UO29 requires.** D-UO29: heavy weapons are stocked "only at their own tier, and only at tier 2 or higher". UO-22 rule 4 applied it and excluded 16 tier-1 heavy candidates. The line in § Gaps restates that rule; it is not a hole to fill.
- **A fallback heavy item would break the contract.** A tier-1 heavy breaks D-UO29 and trips the UO-25 helper's `HEAVY_WEAPONS` guard. A tier-2 heavy in a tier-1 list trips `assert_list`'s check that the item tier equals the list tier. Either way `omega_jaffa_weapons_lists_follow_d_uo29` fails.
- **Dropping the vendor would break three decisions.** D-UO8 B: one template per tier, T1-T4. D-UO28: T1 stands at the plain Weapon spot, which is spawn 63857 in the Common Area (UO-19) as well as 63853 in the Free Jaffa wing (UO-18). D-UO27: 63808 keeps its item ids 638400-638419. Dropping 63831, 63808, 63853 or 63857 would leave the T1 moniker unserved and the Common Area Weapon spot empty, for no gain.

Cost if overruled (the owner wants a tier-1 heavy): amend D-UO29 to allow tier-1 heavies, add `HEAVY_WEAPONS` handling in the UO-25 helper, and re-run UO-22's build for 63808 in one seed packet.

## R2. Rule 12 re-specified

UO-12 rule 12 now reads: stop on a worknote row that disagrees with this packet's ids, templates or tags. UO-22's § Gaps line on 63808 (no heavy weapon) is expected and is **not** a stop. Any other § Gaps line that names one of lists 63808-63811 still stops. The other UO-22 § Gaps lines name only armor lists 63812-63815 (UO-28's), tier 5 and the discipline split, so none of them stops UO-12.

## R3. Revert proof fixed

The old second mutation said "move heavy weapon 5094's row (if stocked) into list 63808". Designs 5094, 6617 and 4565 are in no UO-21, UO-22 or UO-23 list (checked by grep, 2026-10-19), so that branch never applies. The row now names its mutation outright: add 1 to the `naquadah` of row 638450 (Venom Staff, list 63809, 4000 becomes 4001). It fails the D-UO29 price check in `omega_jaffa_weapons_lists_follow_d_uo29`.

## R4. UO-11 (Human mirror)

**Not affected. No UO-11 row and no UO-11.md edit.** UO-21's § Gaps reads "None. Every list has 1-20 rows. Tier 1 has no heavy row by rule 4, ...". It names no list id, so UO-11 rule 12 (stop if UO-21's § Gaps names one of 63800-63803) does not fire. List 63800 has 20 rows across 8 classes and no heavy row, by the same D-UO29 bar (designs 4990, 6491, 6480 excluded at tier 1). If the UO-11 coder still returns `blocked` on that sentence, this ruling applies unchanged: copy list 63800 verbatim, keep template 63812 and its spawns 63841 and 63845, and re-dispatch. UO-11's revert proof already names a fixed mutation (row 1 of 63801), so R3 does not apply to it.

## Follow-up (not part of UO-12; for the wave W8 review)

The UO-25 helper's `HEAVY_WEAPONS` lists only the six designs from UO-06 Q4 (4990, 6491, 6480, 5094, 6617, 4565). UO-22 also counts any `Rak'tel` or `Jaffa Cannon` name as heavy, and so does UO-21 for grenade launchers. The seeds already obey the wider class, because no tier-1 row in 63800 or 63808 is one. But the test would not catch a tier-1 `Rak'tel` row. Widening the guard means editing `live_db_union_omega_vendor_common.rs`, which is UO-25's file. The W8 wave reviewer should decide whether to raise that as a fix note or a follow-up packet. It does not block UO-11 or UO-12.

## Re-dispatch

Re-dispatch UO-12 to a fresh packet-coder with the amended [UO-12.md](../UO-12.md) in this worktree. Nothing else changes. No owner call is open from this ruling.
