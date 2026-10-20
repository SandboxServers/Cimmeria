# UO-28: ruling on the 63812-63815 § Gaps blocker

> **Type:** Reference (ruling note) · **Packet:** [UO-28](../UO-28.md) · **Worknote ruled on:** [UO-22-jaffa-stock.md](UO-22-jaffa-stock.md) § Gaps, lines 263-266, at base `437c45eec`
> **Trigger:** UO-28 coder returned `blocked` under rule 12 · **Ruling:** re-spec rule 12, copy lists 63812-63815 as sealed (62 rows), fill nothing · **Date:** 2026-10-19
> **Consistent with:** the UO-12 ruling on 63808 (`uo-12` worktree, `worknotes/UO-12-ruling.md`): a § Gaps line that restates a sealed rule's outcome is expected, not a hole to fill.

The owner asked for a decision, not a block. This ruling is an architect default the owner may overrule; the cost is given below.

## The blocker

UO-28 rule 12 said: stop if UO-22's § Gaps names one of lists 63812-63815. § Gaps names all four: 63812 has 17 rows, not 20; 63813-63815 have 16, 14 and 15 rows, and "the architect should decide whether to fill them from a family outside the rule". The coder stopped with no edits and no commit. It found nothing else wrong: UO-18 spawns 63859-63862 and UO-19 spawn 63863 match the packet.

## R1. Short armor lists are expected, not gaps (decision)

**Ruling: copy lists 63812-63815 verbatim, 17 + 16 + 14 + 15 = 62 rows. Add no row. Keep templates 63835-63838 and spawns 63859-63863.**

- **The contract is 1 to 20 rows, not 20.** UO-22 rule 1, the UO-28 contract ("at most 20 rows") and the UO-25 helper's `assert_list` (`(1..=20).contains(&rows.len())`) all accept 14-17 rows. Every list passed UO-22 § Checks.
- **The counts are what UO-22 rule 10 requires.** Rule 10 takes whole families, one piece per slot, until the next family would pass 20. At tier 1 the next family, `Standard Defensive` (5 pieces), would make 22. At tiers 2-4 every eligible family is already in (4, 3 and 3 families). A short list is the rule's outcome, as the 63808 heavy line was D-UO29's outcome.
- **There is no family outside the rule to fill from.** At tiers 2-4 every eligible Jaffa family is already stocked. Any other row breaks the contract: an off-tier piece fails rule 3 and the `assert_list` tier check; a Human piece (`AR_H_`) fails rule 6; an unsellable piece fails rule 2. The only legal filler is a second piece of a stocked family in a slot it already fills, at a higher `tech_comp`. The vendor would then show two Boots of the same family at two prices, and that is a new selection rule nobody sealed. A partial `Standard Defensive` set at tier 1 breaks rule 10's whole-family rule.
- **The Human mirror has the same shape.** UO-21's armor lists 63804-63807 have 18 rows each by the same whole-family rule, and UO-21 sealed its § Gaps as "None". Jaffa armor at 14-17 rows is the same policy.
- **D-UO27 leaves room.** Each list owns 50 ids and uses at most 20, so a later fill needs no renumbering.

Cost if overruled (the owner wants 20 rows per armor list): add a fill rule to UO-22 (for example the next-lowest `tech_comp` piece per slot within stocked families), re-run its survey script, reseal the worknote, and re-copy the four lists in one seed packet. Free ids: 638617-638619, 638666-638669, 638714-638719 and 638765-638769.

## R2. Rule 12 re-specified

UO-28 rule 12 now reads: stop on a worknote row that disagrees with this packet's ids, templates or tags. UO-22's § Gaps lines on 63812 (17 rows) and 63813-63815 (16, 14 and 15 rows) are expected and are **not** a stop. Any other § Gaps line that names one of lists 63812-63815 is still a stop, and so is a `### For UO-28` block whose row count differs from those. The acceptance now names the row counts. The revert proof is unchanged: list 63813's first row is 638650 (design 6382), so the `design_id` 21 mutation still applies.

## R3. The other UO-22 § Gaps lines

- **No tier 5 list.** This is D-UO28 working as decided. It is not a stop for UO-12 or UO-28.
- **Discipline split (rule 8).** Confirmed: classification by name and `AR_J_` visual or icon is the intended rule. UO-22 rule 7 specifies exactly that. Rule 8 only asked whether `discipline_ids` agree with it, and they do not, so they are not used. No list changes.
- **Sparse families, accepted as sealed.** `Standard` gives 2 pieces at tier 1 (Boots, Leggings), `Veiled Stealth` gives 1 at tier 2 (Boots) and `Dragon Ranged` gives 4 at tier 3 (no Leggings). These are whole families as the seed holds them. UO-U1 shows how they look on the vendor screen; nothing changes now.

## R4. UO-27, UO-29 and UO-30

The same rule-12 trigger was checked against each packet at base `437c45eec`. None fires, so no packet edit is needed. All three coders were still running at ruling time, with uncommitted edits in their worktrees, so nothing was written there.

| Packet | Rule 12 trigger | § Gaps at base | Ruling |
|---|---|---|---|
| UO-27 (Human Armor, 63804-63807) | UO-21's § Gaps names one of 63804-63807 | "None. Every list has 1-20 rows. Tier 1 has no heavy row by rule 4, ..." No list id. | **Not affected.** Lists are 18 rows each (UO-21 § Checks). If the coder still returns `blocked` on the "Tier 1 has no heavy row" sentence or on the 18-row counts, R1 applies unchanged: copy 63804-63807 verbatim, add no row, re-dispatch. |
| UO-29 (crafting, 63817-63820) | UO-23's § Gaps names one of 63817-63820 | Four lines: no price column, resource and kit name spelling, consumable families beyond the three, the arbitrage-check reading. None names a list id. | **Not affected.** If the coder still stops on one of these lines: copy verbatim; the kit name is `Beta Biomedical Research Kit` as the seed spells it. |
| UO-30 (consumables, 63816) | UO-23's § Gaps names 63816 | Same four lines; none names 63816. "Antidote Darts not stocked" is UO-23 rule 3 working as written. | **Not affected.** If the coder still stops on it, the same ruling applies: copy 63816 verbatim, add no row. |

## Follow-up (not part of UO-28; for the wave W8 review)

None new. The `HEAVY_WEAPONS` follow-up from the UO-12 ruling stands. It does not touch armor lists.

## Re-dispatch

Re-dispatch UO-28 to a fresh packet-coder with the amended [UO-28.md](../UO-28.md) in this worktree. Nothing else changes. No owner call is open from this ruling; the owner may overrule R1 at the cost above.
