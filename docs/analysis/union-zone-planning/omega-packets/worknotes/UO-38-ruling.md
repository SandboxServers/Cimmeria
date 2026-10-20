# UO-38: ruling on the wave 10a review

> **Type:** Reference (ruling note) · **Packet:** [UO-38](../UO-38.md) · **Worknote ruled on:** [UO-38-asgard-stock.md](UO-38-asgard-stock.md) at 0ad70e865
> **Review:** wave 10a review, § UO-38 (verdict `decision`; N4 and the "Not fixed" bullets on the armor gap, the program rule and the Council cast) · **Ruling:** re-spec the worknote; accept with follow-up edits · **Date:** 2026-10-19

The owner asked for decisions, not blocks. F42 means no Asgard character can be created today, so everything below ships as seed rows and live-DB tests that no player sees yet. Every call is an architect default the owner may overrule; the cost is in each item. Nothing here needs an owner answer before UO-39 or UO-40 runs.

## Applying this ruling

A fresh `packet-coder` edits **only** `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-38-asgard-stock.md` (and, optionally, one line in `.claude/agent-memory/items-systems-advisor/` recording R1's armor rule). Rules:

1. PowerShell only, absolute paths, no bash. Start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-38;`.
2. No cargo and no build. Re-run the selection with the survey scripts already in `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-38\.claude\reports\union-omega\stock\` (extend them for R1 and R3), run as `python -I <script>`. The scripts are not committed.
3. `docs/**/*.md` is CRLF. After editing, convert with `[IO.File]::WriteAllText(p, ([IO.File]::ReadAllText(p)).Replace("`r`n","`n").Replace("`n","`r`n"))` on the absolute path.
4. Files: soft cap 500 lines, hard cap 700 (the worknote ends near 260 lines).
5. H-LBL: provenance words only in SQL comments.
6. No seed, code, ledger or other packet edits. No lab, no `sgw` database, no push, no `git stash`, no `git worktree prune`.
7. One local commit with the attribution lines from your brief. Report to `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\uo-38\.claude\reports\union-omega\UO-38-ruling-1.md`.
8. Your script's output must equal the R1 and R3 tables below, row for row. If it differs, stop and return `blocked` with the first differing row.

## R1. Asgard armor exists: stock it from the shipped designs (decision)

**Ruling: map existing designs.** The premise "no Asgard armor design exists" is wrong. The survey looked for the word `Asgard` and for an `AR_A` visual; the armor rows carry neither. They are found by the client's own names.

Evidence (re-run 2026-10-19 on this branch):

- `db/resources/Texts/Seed/texts.sql` 28243-28442 ships 195 `DN_Armor_Asgard_<Family>_<Slot>_<tier><letter>_<Prefix>` monikers (RESTORED_SOURCE): families Absorption, Phasic, Regeneration; slots Chest, Feet, Hands, Head, Legs; tiers 1-5 with one Norse prefix per tier.
- `db/resources/Items/Seed/items.sql` holds **369 designs** named `<Prefix> <Family> <Slot word>`, and every one of the 369 names equals a `DN_Armor_Asgard_*` moniker text. They all have flags 36224 (CAN_BE_SOLD set), a NULL `visual_component` (as the programs do), and description `NO ITEM DESCRIPTION`. No other design uses these names.
- The prefix is the tier, in every row: Fyrstyr 1, Annarr 2, Pridi 3, Fjordi 4, Fimm 5.
- The container is the slot, as for Human armor: `{1,4,17}` Helmet, `{1,7,17}` Vest, `{1,8,17}` Gloves, `{1,11,17}` Pants, `{1,12,17}` Boots.
- `db/resources/Visuals/Seed/body_components.sql` ships 90 `AR_A_*` components for `BS_Asgard.BS_Asgard` only (`AR_A_Absorbtion`, `AR_A_Avoidance`, `AR_A_Regeneration`). The client has Asgard armor art; it is not wired to these designs. Nothing here changes that.

Why this option:

- **Least invented content.** The designs and their names are already in the seed and the client. Nothing is authored but the list rows, as in every other stock seal.
- **No client patch.** The store shows existing designs through the existing armor bit.
- **The others are worse.** Empty stores fail `assert_list` (1-20 rows), and the `buy_item_list` foreign key still needs an `item_lists` row: a vendor whose window opens empty looks broken. Dropping the four vendors throws away four shipped monikers (27256-27259) when their stock exists.

**Classification rule** (the worknote's § Method states it so a reviewer can re-run it):

```sql
WHERE name ~ '^(Fyrstyr|Annarr|Pridi|Fjordi|Fimm) (Absorption|Phasic|Regeneration) (Helmet|Vest|Gloves|Pants|Boots)$'
```

Gates: rules 2 and 5 of the packet, as for programs, plus one data-quality gate:

- **Slot-word gate.** Exclude a design whose slot word disagrees with its container slot (map above). 24 rows fail it: 23 Absorption "Boots" in the hands container `{1,8,17}` (tiers 1-5; the client's `DN_Armor_Asgard_Absorption_Hands_*` texts are empty except `Pridi Absorption Gloves`, so the seed's "Boots" name is a reconstruction slip), and 1 Fimm Regeneration "Vest" in `{1,12,17}` (tier 5, out of scope anyway). Without this gate a tier list shows two rows called "... Absorption Boots".

Eligible after the gates: tier 1 **98**, tier 2 **84**, tier 3 **80**, tier 4 **42**; tier 5 has 45 rows and no list (D-UO28).

**Selection** (the packet's armor rule, unchanged): per tier, take whole families at the family's lowest `tech_comp` in that tier, in the order Absorption, Phasic, Regeneration, until the next family would pass 20 rows. All three fit in every tier. Order the rows by `tech_comp`, then `item_id`. Price `naquadah = 100 × tech_comp` (D-UO29, D-UO38). `item_id` = 638000 + (list − 63800) × 50 + k.

| List | Tier, tc, price | Rows | Design ids in row order (k = 0..n−1) |
|---|---|---|---|
| 63825 | 1, tc 5, 500 | 14 | 7958, 7983, 8008, 8058, 8083, 8108, 8133, 8158, 8183, 8208, 8233, 8258, 8283, 8308 |
| 63826 | 2, tc 20, 2000 | 14 | 7965, 7990, 8015, 8065, 8090, 8115, 8140, 8165, 8190, 8215, 8240, 8265, 8290, 8315 |
| 63827 | 3, tc 33, 3300 | 14 | 7971, 7996, 8021, 8046, 8071, 8096, 8121, 8146, 8171, 8196, 8221, 8246, 8296, 8321 |
| 63828 | 4, tc 45, 4500 | 14 | 7977, 8002, 8027, 8077, 8102, 8127, 8152, 8177, 8202, 8227, 8252, 8277, 8302, 8327 |

Tiers 1, 2 and 4 have no Absorption gloves (the slot-word gate). Tier 3 has no Regeneration pants: the seed has none in tier 3.

**Worknote edits:**

- § Method: replace the "Asgard armor (rule 1)" bullet with the rule, the evidence bullets above (counts 195, 369, 90), the slot-word gate and its 24 rows, and the eligible counts. Replace the "Selection (rule 6, armor)" bullet with the selection above. In the "Price" bullet, drop "(none stocked)". Fill the table rows 63825-63828 (Eligible 98/84/80/42; by family; Picked 14; Mix "Absorption 4, Phasic 5, Regeneration 5" for tiers 1, 2 and 4, "Absorption 5, Phasic 5, Regeneration 4" for tier 3).
- § Gaps: delete the Asgard armor bullet; write "Armor: none (UO-38 ruling R1)."
- § SQL `### For UO-40`: four fenced `sql` blocks in the program blocks' form. Header comment `-- CIMMERIA_AUTHORING (D-UO8 B, D-UO29, D-UO38, UO-38 ruling R1): Asgard Armor tier <n>, 100 x tech_comp`; `INSERT INTO item_lists (item_list_id, name) VALUES (6382<5..8>, 'Omega Asgard Armor Tier <n>');`; each row's trailing comment `-- <name>, <family>, tc <n>`.
- § Checks: rows 63825-63828 become 14 rows, "pass" in every column; the "Program gates" column reads "n/a (armor)" for them.
- § Excluded: add the slot-word gate's 24 rows (ids grouped by tier) and the 45 tier-5 armor rows (no list).
- § Store-open proof: 63825-63828 each `buy_count` 14.

**Consequences.**

- **UO-40: no packet change.** Templates 63847-63850 keep `INT_VENDOR_ARMOR` (8192), moniker 27256-27259 and lists 63825-63828; spawns 63828-63831 come from UO-37 unchanged; all five tests and the revert proofs stand. Its rule 10 stop ("UO-38's § Gaps names one of lists 63825-63828") no longer fires once this worknote is fixed. The UO-U1 `buy_count` for each armor vendor is 14.
- **UO-39: no change from R1.** R3 changes only list 63821's rows, which UO-39 pastes verbatim.
- Known and accepted: the vendors serve any player who clicks (as the Programs vendors and the class trainers do, UO-39 § Why). Whether a non-Asgard can equip these items is the existing item rules' business; the sell list 2 buys back only 21, 55 and 3437, so there is no arbitrage.

Cost if overruled: drop the four armor vendors: remove four templates, four spawns, four lists and two of UO-40's five tests.

## R2. The program rule (confirmed, with one correction)

**Ruling: confirmed.** The 205 designs whose `name` and `description` end in " Program" are the Asgard programs. No owner confirmation is needed; it is an architect default like D-UO38.

- All 205 names equal a client `DN_Weap_Program_<Kind>_<tier><letter>_<Name>` moniker text (`texts.sql`, e.g. 10025 `Workhorse General Purpose Program`, 10026 `Warrior Attack Program`). The client files programs under weapons, which is D-UO38's reason for the weapons bit.
- `docs/analysis/ability-trees/worknotes/at05b.md:45`: `ARCHETYPE_Asgard` trains Attack, Defensive and Scientific Programs. No other archetype does.
- **Correction:** `container_sets '{3,1,17}'` is the weapon container that 1469 designs share. It confirms that a program sits in the weapon slot; it does not mark a design as Asgard. Keep it in the query as a filter.

**Worknote edits:** in § Method's "Asgard programs" bullet, add the moniker evidence and the correction above. In § Gaps, replace the "Program rule, for the architect to confirm" bullet with "Program rule: confirmed (UO-38 ruling R2)."

Cost if overruled: re-pick lists 63821-63824.

## R3. N4: drop design 8536 (confirmed as written)

**Ruling: apply N4 exactly.** 8536 has flags 35840 (no other program does), empty `discipline_ids`, and the same name, tier, `tech_comp` 1 and price 200 as 3676, so the store would show two identical rows. I re-ran the round-robin with 8536 excluded; it reproduces the reviewer's result:

| item_id | Design | Kind | tc | naquadah |
|---|---|---|---|---|
| 639050-639066 | unchanged: 3677, 5698, 5726, 3678, 5782, 5825, 3679, 5898, 3676, 5673, 5699, 5727, 5752, 5783, 5828, 5856, 5899 | | | |
| 639067 | 5674 Warrior Attack Program | Weapon | 8 | 1600 |
| 639068 | 5700 Hound Attack Program | Reflex | 10 | 2000 |
| 639069 | 5728 Eagle Attack Program | Targeting | 10 | 2000 |

**Worknote edits:** list 63821's block takes these three rows at 639067-639069. The § Method table's tier-1 Mix becomes "Weapon 3, Reflex 3, Targeting 3, Protection 2, Hazard 2, Ward 2, Resilience 2, Diffusion 2, General Purpose 1"; its Eligible stays 61 and "Eligible by kind" keeps "General 2". § Excluded adds "8536 Workhorse General Purpose Program: flags/disciplines outlier duplicate of 3676." Lists 63822-63824 do not change.

## R4. Council cast spawns 63838 and 63839: keep them reserved and unused

**Ruling: keep them reserved, unsealed and unseeded.** UO-16 § Hearing names two speakers, Thor (63836) and Otin (63837); UO-16 ruling R1 keeps 63838/63839 as `council.extra.*` for a speaker the owner may add under D-UO34. D-UO34 stays BlockedDecision and mission 1522 and Otin stay deferred. Sealing a point for an unnamed speaker would be invented content, and an unused id costs nothing.

- UO-37's worknote is right as it stands ("Not used"). No UO-37 fix.
- If D-UO34 adds a speaker, the full UO-42 packet seals that point on the world-80 floor UO-37 already probed.
- The only edit is the seal's text (follow-up F6 below).

## R5. Follow-up edits owed

Docs-only; the coordinator makes them, no packet. **F3 and F4 must land before UO-39 and UO-40 are dispatched**; the rest belong to the W10 close-out ledger pass.

| # | File | Exact edit |
|---|---|---|
| F1 | `omega-work-packets.md` § ID map, `entity_templates` | Replace the row `\| \| 63862-63899 \| spare \|` with two rows: `\| \| 63862 \| Thor (Hologram) 28128, hub 481 offer (UO-42, UO-16 ruling R1) \|` and `\| \| 63863-63899 \| spare \|`. |
| F2 | `omega-work-packets.md` § ID map, `spawnlist` | Replace `\| \| 63873-63879 \| spare \|` with `\| \| 63873 \| Asgard hub Thor, world 18, tag `UnionOmega_AsgardThor` (UO-42, UO-16 ruling R1) \|` and `\| \| 63874-63879 \| spare \|`. Replace the cast row's text `Council cast (UO-42)` with `Council cast (UO-42): 63836 Thor, 63837 Otin (D-UO34); 63838-63839 reserved, unsealed (UO-38 ruling R4)`. Add the missions block from UO-16 ruling R2 (`missions` 63800-63805, `mission_steps` 63810-63829, `mission_objectives` 63830-63849, `mission_tasks` 63850-63869, all UO-48), if it is still missing. |
| F3 | `UO-39.md` and `UO-40.md` § Rules | Append one rule: "Spawn 63873 and template 63862 belong to UO-42 (the hub Thor); never take them. UO-37 sealed your points' spacing against it." |
| F4 | `UO-40.md` § Acceptance | Append: "Each armor list holds 14 rows (UO-38 ruling R1); UO-U1's `buy_count` for 63847-63850 is 14." |
| F5 | `omega-work-packets.md` § Decisions, D-UO38 row | Append to the Decision cell: "Asgard armor is the 369 designs named `<Fyrstyr\|Annarr\|Pridi\|Fjordi\|Fimm> <Absorption\|Phasic\|Regeneration> <slot>` (the client's `DN_Armor_Asgard_*` names); Asgard programs are the 205 `DN_Weap_Program_*` designs (UO-38 ruling R1, R2)." In § Packets, the UO-37 row's scope becomes "trainer, 12 vendors, hub Thor, Council cast". |
| F6 | `worknotes/UO-16-asgard-seal.md` | Line 44: replace "the Council cast is four spawns (spawnlist 63836-63839, UO-37 seals their points)" with "the Council cast is two named speakers, Thor (63836) and Otin (63837, only if D-UO34 says he speaks); UO-37 sealed both points. 63838-63839 stay reserved for a speaker the owner may add under D-UO34 (UO-38 ruling R4)". Line 94: "Council cast (63836-63839)" becomes "Council cast (63836, 63837; 63838-63839 reserved)". |
| F7 | `UO-37.md` | Title: "(13 points plus the cast)" becomes "(14 points plus the cast)". Contract table: add the row `asg.thor` \| 63873 \| 63862 \| `UnionOmega_AsgardThor` \| UO-42. § Areas: "The trainer and the twelve vendors" becomes "The trainer, the twelve vendors and the hub Thor". Step 5's `### For UO-42` "(63836-63839, or fewer)" becomes "(63836, 63837 and the hub Thor 63873)". Acceptance: "13 service rows plus 2-4 cast rows" becomes "14 service rows (13 plus the hub Thor 63873) and 2 cast rows; 63838/63839 not used". |
| F8 | `UO-42.md` stub | Templates: add "63862 hub Thor on 28128, the same row and body as 63858 (UO-16 ruling R1)". Spawns: "63836-63839, the Council cast" becomes "63836, 63837 (only if D-UO34 says Otin speaks) and the hub Thor 63873, copied from UO-37 § SQL `### For UO-42`; 63838-63839 stay reserved (UO-38 ruling R4)". The `[UO-42]` marker text becomes "63836-63839, 63873". Add: "The 481 offer binds to template 63862 only." |
| F9 | `UO-41.md` stub | Add: "The hearing never offers 481 (UO-16 ruling R1)." |

## Owner calls left

None from this ruling. D-UO34 stays BlockedDecision, as before. R1-R4 are architect defaults the owner may overrule at the costs stated.
