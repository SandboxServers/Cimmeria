# CS-00Rb worknote: seal G-SYM

> **Packet:** CS-00Rb. **Agent:** game-archaeology-specialist. **Date:** 2026-10-17.
> **Base:** `c481008904ba39c80168424f67d96b9b271b9ac9`. **Output:** [g-sym.md](../evidence/g-sym.md).

## Sources read

- `docs/analysis/castle-continuation/work-packets.md` §§81-356 (gate contract, 1521 graph, moniker 8897, chamber cluster).
- `docs/analysis/castle-continuation/evidence/`: `reconciliation.md`, `cooked-interaction-maps.md`, `dialog-excerpts.md`, `seed-excerpts.md`, `id-census.csv`.
- `docs/analysis/four-zone-planning/id-allocation.md` (chain blocks, 1366–1370 claims) and `story-research.md` (oral history).
- `docs/gameplay/minigame-system.md:73`–`75`; `docs/content/consumable-via-onitemuse-pattern.md:134`–`152`; `docs/analysis/issue-triage-2026-09-25/findings/content.md:49`–`85` (issue #334, still OPEN).
- Seeds: `Items/Seed/items.sql` (461, 4609, 12170, 12172), `items_event_sets.sql:3979`, `Abilities/Seed/abilities.sql:2536`, `:2538`, `Effects/Seed/effects.sql:3615`, `:3617`, `Texts/Seed/texts.sql:57107`–`57123`, `Entities/Seed/entity_templates.sql:953`–`955`, `entity_templates_dakara_e1.sql:188`, `Dialogs/Seed/dialog_sets.sql:769`.
- Content: `castle_cellblock_chains.sql:996`–`1027`, `:1082`, `:1091`; `castle_701_chains.sql:62`–`77`, `:260`–`278`; `castle_706_708_chains.sql:654`; `sgc_w1_chains.sql:377`, `:384`, `:437`; `harset_opcore_chains.sql:20`, `:453`–`496`.
- Client: `CookedDataItems.pak` (`_5168`, `_5169`), `CookedDataMissions.pak` (`_1521`), `CookedInteractionSet.pak` (set 1572); all 146 Castle packages (145 `.umap` + `Castle_MapData.upk`, all parse, 0 errors) via `tools/upk_parser.py` `PackageReader.parse()`.
- `deprecated/python` (git grep, no hits).

## Findings that change the ledger

- The Castle symbiote is item **5169**, not 2718/4652. `reconciliation.md` needs that correction, but it is outside this packet's allowlist.
- 5169's Use event fires ability 1927, "Cure Symbiote Loss" (effect 2481). The debuff is ability 1926 / effect 2480. This is the same shape as Cellblock's Ambernol (1374/1636 and 1372/1634).
- The radio is item 5168. Its client Use is inert (`AbilityID="0"`, #334).
- Chains 1369/1370 are contract only. No 1521 content is seeded at base.

## Open questions

1. G-SYM-GAME: what game, if any, did the tank launch? Flag 1073741824 on map `_5829` is unexplained.
2. G-SYM-CONSUME: is 5169 spent on Use?
3. G-SYM-DEBUFF: what applied Symbiote Loss (1926) to the player?
4. G-SYM-RADIO: how did a Castle player get 5168, and how was the Use affordance shown?
5. D-CA-SYM: grant and consume 5169 (Option A) or keep zero item mutation (Option B)? This is the owner's call; the items advisor reviews.
6. G-ID / G-ROUTE: tank template ID and placement.

## Follow-ups outside the allowlist

- Correct the 2718/4652 row in `reconciliation.md` (G-SYM line 97).
- Claim chains 1374–1377 in `id-allocation.md` if the reconstruction is adopted. The earlier 1371–1374 draft collided with nothing on base, but CS-00Ra keeps 1371–1373 (commit `822bddde6` on `castle/cs-00ra-g-med`), so the block moves to 1374–1377. The 1374–1377 check found zero chain rows on base and the Cellblock, Harset and Tollana branches; the word hits are ability IDs and Harset mission keys.
