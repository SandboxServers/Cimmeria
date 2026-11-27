# Wave 0 ruling: D1, the class of an ability-to-event-set link

Source: `.claude/reports/npc-combat-reference/wave0-review-1.md` (nc-01-resolver
worktree), § Not fixed D1. Architect ruling, 2026-11-27.

## Ruling

A seed value is classed by its evidence, not by who wrote it. The rule is now
in README § Evidence classifications (main checkout), and the RECONSTRUCTED
row there now reads "Behaviour or data Cimmeria chose by judgement, in this
campaign or an earlier PR". In short:

| Provenance of the link | Class |
|---|---|
| Giza seed already held it (non-NULL before PR #822, `f153138b5`) | EARLIEST LEGACY DATA |
| PR #822 Tier 1 (family rule) or Tier 2 (item-fired) | STRONGLY INFERRED |
| PR #822 Tier 3 (judgement) | RECONSTRUCTED (cite "PR #822 Tier 3") |
| A cooked client record names the ability | ORIGINAL CLIENT VERIFIED |

CURRENT CIMMERIA IMPLEMENTATION is never used for a seed value.

Provenance checked in git: `git show f153138b5 -- db/resources/Abilities/Seed/abilities.sql`
changes 1433 and 983 (NULL to 300) and none of 559, 584, 710 or 221. No later
commit on that file touches any of the six. So 559→15, 584→3, 710→300 and
221→802 are Giza links (EARLIEST LEGACY DATA), and 1433→300 and 983→300 are
PR #822 Tier 3 (RECONSTRUCTED). NC-04's "Tier 1 for 584 and 710, Tier 2 for
559" is wrong: those tiers covered only rows that were NULL.

## Fix notes

Rules for every note: edit only the named file in its own worktree under
`C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\npc-combat-reference\`;
the files are CRLF and must stay CRLF; locate lines by the quoted text (line
numbers are as of the reviewed heads and move after N1-N3); apply together with
the pass-1 notes N1-N3 in one commit per packet.

### NC-02, `docs/analysis/npc-combat-reference/evidence-registry.md` (nc-02-registry)

1. Line 22, Classes table: replace the RECONSTRUCTED meaning with
   `Behaviour or data Cimmeria chose by judgement, in this campaign or an earlier PR, such as a damage payload taken from the ability's own description`.
2. Line 83 (`Unassigned ability 1433`): in the Value cell replace `event set 300`
   with `event set 300 (added by PR #822; see the link row)`. Class stays
   EARLIEST LEGACY DATA (the rest of the row is Giza data).
3. Line 85 (`Unassigned ability 983`): the same replacement as note 2.
4. Insert after line 86 (`Effect 1092`) this row, verbatim:

   ```text
   | Link 1433 and 983 → event set 300 | Both NULL in the Giza seed; PR #822 set 300 as Tier-3 judgement calls | RECONSTRUCTED | [ability-animation-links.md](../../reverse-engineering/findings/ability-animation-links.md) § Tier 3; commit `f153138b5` | NC-05 |
   ```

5. Line 88 (ScavDog `Event sets`): in the Value cell, change the words
   "ability event set 300" to "event set 300", and append to that cell
   " (its link from 1433 and 983 is the RECONSTRUCTED row above)". Class stays
   EARLIEST LEGACY DATA.
6. No change to the 559, 584, 710 and 221 rows or the NID, Jaffa and PRU
   `Event sets` rows: they are Giza links and already EARLIEST LEGACY DATA.

Verify: `Select-String 'RECONSTRUCTED' evidence-registry.md` shows the new link
row; the file is CRLF.

### NC-04, `docs/analysis/npc-combat-reference/presentation-chain.md` (nc-04-seed-trace)

7. Line 41: replace the whole row with, verbatim:

   ```text
   | Ability to set, Giza seed (584→3, 559→15, 710→300, 221→802) | EARLIEST LEGACY DATA (present before PR #822, which did not touch these rows; 584, 559 and 710 match the family rule, animation-links.md:35-37) | `abilities.sql`; commit `f153138b5` changes none of them |
   ```

8. Line 42: change the class cell to
   `RECONSTRUCTED (PR #822 Tier 3; both NULL before)`. Leave the source cell.
9. Line 43 (`Ability to set, seed only (221→802)`): delete the row; note 7 now
   covers 221.
10. Line 233: replace `the 1433 link (RECONSTRUCTED until then).` with
    `the 1433 and 983 links (RECONSTRUCTED, PR #822 Tier 3, unless a client record names the ability).`

Verify: `Select-String 'Tier 1 for 584' presentation-chain.md` returns nothing.

### NC-05, `docs/reverse-engineering/findings/scavdog-combat-assets.md` (nc-05-scav-assets)

11. Line 13: replace `as a Tier-3 RECONSTRUCTION, not legacy data` with
    `as a Tier-3 judgement call, so RECONSTRUCTED, not legacy data`.
12. Line 137: replace `**CURRENT CIMMERIA IMPLEMENTATION**` with
    `**RECONSTRUCTED**`. Keep the parenthetical.
13. Line 141: replace `(CURRENT CIMMERIA IMPLEMENTATION, a RECONSTRUCTION since PR #822)` with
    `(RECONSTRUCTED, PR #822 Tier 3)`.
14. Line 142 (`RECONSTRUCTED at best`): no change.

Verify: `Select-String 'CURRENT CIMMERIA IMPLEMENTATION' scavdog-combat-assets.md`
matches only line 17 (the class list).

### NC-06, `docs/reverse-engineering/findings/reference-npc-presentation.md` (nc-06-presentation-assets)

15. No change for D1. Its "Seed chain" tables class the event sets' sequence
    rows, not any ability's link, and those are Giza data (EARLIEST LEGACY
    DATA). The doc names none of 559, 584, 710, 221, 1433 or 983. Apply N3 only.

### NC-31 (carried, not a wave-0 fix)

16. When NC-31 rewrites README F6 (O1), change its class cell to
    `EARLIEST LEGACY DATA / RECONSTRUCTED / UNRESOLVED` and its sentence to
    "The 1433 and 983 → event set 300 links are PR #822 Tier-3 judgement calls
    (RECONSTRUCTED)". F3 stays CURRENT CIMMERIA IMPLEMENTATION: it records that
    the gap was closed, not the class of any one link. NC-31 must not
    reclassify registry rows against this rule.

## Owner decision

None needed. The class rule is campaign vocabulary and changes no data or
behaviour. Whether the ScavDog gets 1433 is still D-NC5, unchanged.
