# G-SYM evidence: mission 1521 "Symbiote"

> **Type:** Reference. **Audience:** Castle restoration authors and reviewers.
> **Last updated:** 2026-10-17 (CS-00Rb). **Base:** `c481008904ba39c80168424f67d96b9b271b9ac9`.
> **Companions:** [reconciliation](reconciliation.md), [cooked interaction maps](cooked-interaction-maps.md),
> [dialog excerpts](dialog-excerpts.md), [seed excerpts](seed-excerpts.md), [worknote](../worknotes/CS-00Rb.md).

This seals what the shipped data, the client and the recovered scripts prove
about the 1521 Symbiote Tank, its minigame, the symbiote implant and the
Moh'katan report. It does not decide anything. Each row gives the value, its
provenance label, the proof and a confidence. Where the evidence ends, the row
says `UNRESOLVED (<gate>)` and names the missing evidence.

## Correction to the reconciliation ledger

`reconciliation.md` names items 2718/4652 as the symbiote candidates. **They are
not the Castle item.** Both are the generic "Goa'uld Symbiote" ("A wiggly
Goa'uld symbiote without a host", flags 35840; `db/resources/Items/Seed/items.sql:461`,
`:4609`). The Castle item is **5169 "Immature Goa'uld Symbiote"**
(`items.sql:12172`). Its name and description are monikers 8901
`DN_MsIt_Castle_Fn_Symbiote` and 8902 `DS_MsIt_Castle_Fn_Symbiote`
(`db/resources/Texts/Seed/texts.sql:57121`, `:57123`). The `MsIt_Castle` prefix
is the mission-item naming that 8897 (`texts.sql:57107`–`57109`) and the radio
monikers 8899/8900 share.

## Q1: the Symbiote Tank actor and client targeting

| # | Claim | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|---|
| 1.1 | Tank name moniker | 8897 "Symbiote Tank" | SHIPPED_DATA | `texts.sql:57107`–`57109`; contract `work-packets.md` §131-157 | High |
| 1.2 | Tank mesh exists in Castle | StaticMesh `CA-Props.CA-SymbioteChamber00` (import 298) plus textures `CA-SymbioteChamber00_D/E/N/S`, `01_D/N/S` | CLIENT_MAP | `Castle-00090003.umap` name/import tables read with `tools/upk_parser.py` `PackageReader.parse()` | High |
| 1.3 | Chamber prefab carries no interaction | four `CA-SymbioteChamber00_Pf0_Seq` exports (1026–1029), class Engine `Sequence`, outer `Prefabs`, serial sizes 285/287/287/287 bytes, with no interaction or mission Kismet; ambient only: SeqEvent_LevelLoaded x3, SeqAct_Delay x3, SeqAct_PlaySound x3, SeqVar_Object x3, SeqVar_RandomFloat x3 under CA-SymbioteChamber00_Pf0_Seq in Castle-00090003.umap | CLIENT_MAP | same parse; sizes too small for any op chain | High |
| 1.4 | No map actor binds 8897 | no tank, dispenser, minigame or interact name in any of 146 Castle packages (145 `.umap` + `Castle_MapData.upk`, all parse, 0 errors) | CLIENT_MAP | full scan, below "Negative checks" | High (for the names the parser exposes) |
| 1.5 | No cooked interaction map binds 8897 | 0 of 4,664 map records name 8896/8897 | SHIPPED_DATA | `cooked-interaction-maps.md:25`–`30` | High |
| 1.6 | How the client targets the tank | a server-spawned `being` with `name_id` 8897 and an interaction type set by tag, as Harset template 245 "Anat's Symbiote Tank" does (`Ga-Props.GA-PuzzleStation00`, `GLB_Components.WorldObject_Small`, flags 4, name_id 7597) | RECONSTRUCTION (pattern) | `db/resources/Entities/Seed/entity_templates.sql:953`–`955`; Dakara copy `entity_templates_dakara_e1.sql:188` | Medium |
| 1.7 | Castle tank template ID and spawn point | none | UNRESOLVED (G-ID, G-ROUTE) | no Castle template names 8897 (`git grep -n -w 8897` hits only `texts.sql`); chamber cluster `(398.83, 55.04, 929–941)` is unsealed (`work-packets.md` §340-356) | — |

The Harset template is precedent only. It is not a Castle binding
(`reconciliation.md`, Harset tank row). A Castle copy needs an ID from the
templates owner (G-ID) and a placement inside the chamber cluster (G-ROUTE).

## Q2: minigame type and parameters

| # | Claim | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|---|
| 2.1 | Shipped data names a game type | **none**: cooked `_1521` has no game, item, actor or dialog reference | SHIPPED_DATA (negative) | `CookedDataMissions.pak/_1521` (2,474 bytes) | High |
| 2.2 | Success text implies a security bypass | dialog 4990, screen 96946: "You quickly bypass the security protections on the Symbiote Tank. You reach inside and pull out a symbiote from the tank." | SHIPPED_DATA | `dialog-excerpts.md` (4990); cooked map `_5831` → 4990 | High (text); Low (as a game choice) |
| 2.3 | Set 1572 has an un-dialoged map | `_5829` → dialog 0, flags 1073741824 | SHIPPED_DATA | `cooked-interaction-maps.md:20` | High (value); meaning UNRESOLVED |
| 2.4 | Game type and parameters | **UNRESOLVED (G-SYM-GAME)** | — | missing: a Kismet or server reference to a game ID, or an aligned live interaction trace | — |
| 2.5 | Bounded game choice already sealed | Livewire with `INT_MinigameLivewire` cursor 256; Bypass and Hack stay PlaceholderGame auto-win | RECONSTRUCTION (existing seal) | `docs/gameplay/minigame-system.md:73`–`75`; `work-packets.md` §81-356 | Medium |

The word "bypass" in 4990 is narrative text. It does not prove the Bypass
minigame, because the shipped text never names a game and the server's Bypass
is a placeholder. Row 2.5 stays a bounded reconstruction behind the CS-02b0/b1/b2
receipt and identity gates.

## Q3: symbiote implant behavior

| # | Claim | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|---|
| 3.1 | The implant is an item | 5169 "Immature Goa'uld Symbiote", flags 36864, `container_sets {2}`, stack 1 | SHIPPED_DATA | `items.sql:12172` | High |
| 3.2 | Client cooked item agrees | `_5169`: `IsDeletable="false" IsSellable="false" MaxStackSize="1"`, `RequirementsSet IsUnique="true"`, `ItemEventSet AbilityID="1927" EventID="5"`, `ContainerSet 2` | SHIPPED_DATA (client) | `CookedDataItems.pak/_5169` | High |
| 3.3 | Use fires ability 1927 | event set `(1987, 5169, 1927, 5)`; EventID 5 is "Use" | SHIPPED_DATA | `db/resources/Items/Seed/items_event_sets.sql:3979` | High |
| 3.4 | Ability 1927 cures symbiote loss | ability 1927 → effect `{2481}` "Cure Symbiote Loss", flags 17 (its description is copied text: "Removes stasis sickness from your target.") | SHIPPED_DATA | `db/resources/Abilities/Seed/abilities.sql:2538`; `db/resources/Effects/Seed/effects.sql:3617` | High |
| 3.5 | Matching debuff exists | ability 1926 → effect `{2480}` "Symbiote Loss", flags 524288, icon `DD_Disruption_Shot` | SHIPPED_DATA | `abilities.sql:2536`; `db/resources/Effects/Seed/effects.sql:3615` | High |
| 3.6 | Same shape as Cellblock's Ambernol | item 19 → ability 1374 → effect 1636 (flags 17) cures; ability 1372 → effect 1634 (flags 524288) debuffs; chain 1034 runs `launch_ability 1374` on item use, chain 1112 runs `launch_ability 1372` on zone load | CIMMERIA_AUTHORING (precedent) | `db/resources/Content/Seed/castle_cellblock_chains.sql:996`–`1027`, `:1082`, `:1091` | High (structure) |
| 3.7 | Player is meant to carry Symbiote Loss | dialog 4989: the NID removed the warriors' symbiotes and "All are left weakened"; player: "Indeed. I suffer as well." | SHIPPED_DATA (text) → RECONSTRUCTION | `dialog-excerpts.md` (4989, screens 96950–96955) | Medium |
| 3.8 | Grant point | at tank victory (4990 says the player pulls the symbiote out) | UNRESOLVED (G-SYM-GAME) | row 2.2 (text only; the victory trigger is unproven) | Medium |
| 3.9 | Consume point | on Use (EventID 5 → 1927); whether the item is spent is not in the data | UNRESOLVED (G-SYM-CONSUME) | no consume flag recovered; missing: a use trace or a server script | — |
| 3.10 | Who applies Symbiote Loss (1926), and when | no chain or script applies it | UNRESOLVED (G-SYM-DEBUFF) | negative check below; missing: a recovered trigger | — |
| 3.11 | Pouch or slot | `container_sets {2}` / `ContainerSet 2`, the same container 5168 is granted into (`{"container": 2}`) | SHIPPED_DATA | rows 3.1, 3.2; `sgc_w1_chains.sql:377` | High |

### Proposed BlockedDecision: D-CA-SYM (not decided)

| Field | Value |
|---|---|
| Decision | Does 1521 grant and consume item 5169? |
| Option A (data-faithful) | Tank victory runs `add_item 5169 {"container": 2}`; the player uses 5169, which fires `launch_ability 1927` via event set 1987 and clears effect 2480. Optionally apply 1926 on 1521 accept to make the cure visible. |
| Option B (current contract) | Zero item grant or removal, as the gate contract states (`work-packets.md` §81-356); the items advisor boundary stands and 5169 stays unreferenced. |
| Evidence for A | rows 3.1–3.7 |
| Evidence against A | no recovered chain or script grants 5169 or applies 1926 (rows 3.9, 3.10); the cooked mission names no item |
| Owner | maintainer; the items-systems-advisor must review any grant |
| Status | **BlockedDecision, proposed**. No packet may assume either option. |

## Q4: Moh'katan's report

| # | Claim | Value | Provenance | Proof | Confidence |
|---|---|---|---|---|---|
| 4.1 | Report is step 4458 / objective 5178 | 4458 "Use the Tau'ri radio to report to Moh'katan."; objective 5178, task 6381 | SHIPPED_DATA | `seed-excerpts.md:46`–`64`, `:142`–`160`, `:244`–`268`; cooked `_1521` | High |
| 4.2 | Report dialog | 4991, screens 96956–96958 ("Have you found symbiotes?"), speaker 2499 Moh'katan, no button rows | SHIPPED_DATA | `dialog-excerpts.md` (4991); cooked map `_5832` → 4991 | High |
| 4.3 | The radio is item 5168 | "Radio", "A small hand-held radio."; monikers 8899 `DN_MsIt_CastleSGC_Fn_Various_Radio`, 8900 | SHIPPED_DATA | `items.sql:12170`; `texts.sql:57113`, `:57119` | High |
| 4.4 | Radio Use is client-inert | cooked `_5168`: `ItemEventSet AbilityID="0" EventID="5"`; no seed event set; issue #334 OPEN | SHIPPED_DATA (client) | `CookedDataItems.pak/_5168`; `docs/analysis/issue-triage-2026-09-25/findings/content.md:49`–`85` | High |
| 4.5 | Radio-use precedent | SGC 1561: chain 3020 `add_item 5168 {"container": 2}`, chains 3021/3025 trigger `item_use '5168'` | CIMMERIA_AUTHORING | `db/resources/Content/Seed/sgc_w1_chains.sql:377`, `:384`, `:437`; `docs/content/consumable-via-onitemuse-pattern.md:134`–`152` | High (precedent) |
| 4.6 | Moh'katan-radio dialog precedent | chain 1205 `display_dialog 5862` after a dialog choice, NEW CONTENT under D-CA13, relying on the `last_interaction_target` pin | CIMMERIA_AUTHORING | `castle_701_chains.sql:260`–`278`, `:62`–`77` | High (precedent) |
| 4.7 | What fires the report | radio `item_use '5168'` while 4458 is active, then `display_dialog 4991` | UNRESOLVED (G-SYM-RADIO) | rows 4.1, 4.3, 4.5 (precedent only) | Low–Medium |
| 4.8 | Player holds a radio in Castle | no Castle chain grants 5168 | UNRESOLVED (G-SYM-RADIO) | `git grep -n -w 5168 -- db/resources/Content` hits only `sgc_w1_chains.sql`; missing: a grant point, plus the #334 Use-affordance fix | — |

The contract's alternative, "closing radio dialog 4991 completes 5178 and 1521",
needs a way to open 4991. Set 1572 has no template or content binding
(`dialog_sets.sql:769` is the only reference), so the opener is G-SYM-RADIO.

## Q5: step and objective transitions

All four steps and objectives are disabled; tasks 6377–6381 are enabled,
TaskType 1 (cooked `_1521`; `seed-excerpts.md`). The mission is Level 3,
`IsAStory` true, `CanAbandon` false.

| Transition | Credit | Status | Provenance | Proof |
|---|---|---|---|---|
| accept → 4455 | 5175 (tasks 6377, 6378) | Contract reconstruction (chains 1369/1370, archetype 8, `Castle.Infirmary`, world 8, 704 completed). Not seeded at base: `git grep -w 1521 -- db/resources/Content` returns nothing | RECONSTRUCTION | `work-packets.md` §81-356 |
| 4455 → 4456 | 5175 credited by `advance_step 1521 4456`, never `complete_objective` | Contract reconstruction | RECONSTRUCTION | same |
| 4456 → 4457 | 5176 (task 6379) via tank interaction → game → victory, dialog 4990 | Step order and the 4990 text are SHIPPED_DATA; the trigger is not proven | UNRESOLVED (G-SYM-GAME, G-ID) | rows 2.2–2.4 |
| 4457 → 4458 | 5177 (task 6380) via a separate tank interaction | Step text is SHIPPED_DATA; trigger not proven | UNRESOLVED (G-ID) | `seed-excerpts.md:46`–`64` |
| 4458 → complete | 5178 (task 6381) via report dialog 4991 | Dialog 4991 is SHIPPED_DATA; opener not proven | UNRESOLVED (G-SYM-RADIO) | rows 4.1–4.8 |

**Proven:** the step order 4455→4458, the objective and task membership, the
disabled/enabled flags and the dialogs in set 1572 (all SHIPPED_DATA). **Not
proven:** every trigger. No client, map or script source names one.

## Q6: bounded RECONSTRUCTION and collision audit

Where the evidence ends, this is the smallest authored graph that matches the
shipped steps, text and the gate contract. It is a proposal (O14-03). It
activates nothing until CS-02b0/b1/b2, G-ID, G-ROUTE and D-CA-SYM close.

| Chain | Trigger | Condition | Actions |
|---|---|---|---|
| 1374 | interaction with the Castle tank (tag, template per G-ID) | `step_status 1521 4456 active` | start the sealed Livewire game (row 2.5) |
| 1375 | game victory callback | `step_status 1521 4456 active` | `advance_step 1521 4457` (credits 5176); `display_dialog 4990`; item grant only if D-CA-SYM picks Option A |
| 1376 | interaction with the tank | `step_status 1521 4457 active` | `advance_step 1521 4458` (credits 5177) |
| 1377 | report opener per G-SYM-RADIO, then dialog 4991 closed | `step_status 1521 4458 active` | complete 5178 and 1521 |

Chains are 1374–1377 (renumbered from the earlier 1371–1374 draft). Chains
1371–1373 stay with CS-00Ra (commit `822bddde6` on `castle/cs-00ra-g-med`), so
this proposal does not use them.

**Collision audit.** The castle restoration block is 1366–1400. 1366–1370 are
claimed (`docs/analysis/four-zone-planning/id-allocation.md:35`–`46`), 1371–1373
are CS-00Ra's (above), and 1374–1400 are free on base. Command, run on
2026-10-17 from the worktree:

```powershell
foreach($b in 'c481008904ba39c80168424f67d96b9b271b9ac9','content/cellblock-campaign',
              'content/harset-campaign','content/tollana-campaign'){
  foreach($id in 1374,1375,1376,1377){
    git grep -n -E "\(\s*$id\s*," $b -- db/resources/Content     # chain-row form
    git grep -n -w $id $b -- db/resources/Content                # any word
  } }
```

Output: **0 chain-row hits** for all 16 branch/ID pairs. Word hits are
identical on each branch and none is a chain claim:

- 1374 → 6 hits. Five are ability 1374 (`launch_ability 1374` at
  `castle_cellblock_chains.sql:1027` and the C03 comments at `:996`–`1009`).
  One is the Harset **mission** key 1374 in the comment at
  `harset_opcore_chains.sql:20` (`1374: 6621-6645`).
- 1375 → 1 hit, `harset_opcore_chains.sql:21`, a Harset **mission** key
  (`1375: 6646-6665`).
- 1376 → 0 hits.
- 1377 → 1 hit, `harset_opcore_chains.sql:21`, a Harset **mission** key
  (`1377: 6666-6680`).

Chains 1374–1377 are free on all four branches. **Claim them** in
`id-allocation.md` before seeding. No template ID is proposed (G-ID).

## Negative checks (one per provenance class)

| Class | Negative claim | Command / method | Result |
|---|---|---|---|
| SHIPPED_DATA | No content chain references abilities 1926/1927 or effects 2480/2481 | `git grep -n -w -E '1926\|1927\|2480\|2481' <base> -- db/resources/Content` | 0 hits |
| RECOVERED_SCRIPT | No recovered legacy script handles 1521 | `git grep` of `deprecated/python` for `1521`, `symbiote`, `5169`, `8897`, `Moh` | 0 hits |
| CLIENT_MAP | No Castle map names a tank, dispenser, minigame or interact actor | `PackageReader.parse()` over all 146 Castle packages (145 `.umap` + `Castle_MapData.upk`), name/import/export tables | only `CA-SymbioteChamber00*` in `Castle-00090003.umap` (rows 1.2–1.3) |
| RECONSTRUCTION | Proposed chains 1374–1377 collide with nothing | Q6 audit | 0 chain rows on 4 branches |
| ORAL_HISTORY | Oral history gives no 1521 mechanics | read `docs/analysis/four-zone-planning/story-research.md` | line 23 names 1521 only as a recovered boundary; tone guidance only (`reconciliation.md:16`) |
| CIMMERIA_AUTHORING | No authored 1521 content exists at base | `git grep -n -w -E '1521\|4455\|4456\|4457\|4458\|5169' <base> -- db/resources/Content` | hits only Harset **dialog** IDs 4456–4458 in `harset_opcore_chains.sql:453`–`496`; no 1521 step or 5169 row |

## Gates left open

| Gate | Missing evidence |
|---|---|
| G-SYM-GAME | a client or server reference to the tank's game ID, or an aligned live interaction trace |
| G-SYM-CONSUME | whether Use spends 5169 (a use trace or a server script) |
| G-SYM-DEBUFF | the trigger that applies ability 1926 / effect 2480 |
| G-SYM-RADIO | a Castle grant of 5168 and the #334 Use-affordance fix, or another opener for 4991 |
| G-ID | a Castle tank template ID from the templates owner |
| G-ROUTE | tank placement inside the `(398.83, 55.04, 929–941)` chamber cluster |
| D-CA-SYM | owner choice, table above |

## R2 seal

CS-03S (2026-10-17, base `da2b82c70`) sealed the Option B skeleton for mission 1521 in [worknotes/CS-03S.md](../worknotes/CS-03S.md).

- G-ID: template 413 and spawn 13802 are free on base and the three campaign branches (CIMMERIA_AUTHORING); 412 / 13801 belong to CS-02S. Closed for Option B.
- G-ROUTE: closed by placing the tank in the Infirmary at (356.0, 70.18, 896.0), navmesh component 250 (nav_inspect, `--v-tol 0.05`, dy 0), the same component as Ogilvie. The chamber cluster is component 468 and unreachable from the Infirmary (nav_inspect exit 3), so the chamber placement is not playable. The `find_path` replay is CS-03c2.
- G-SYM-GAME, G-SYM-CONSUME, G-SYM-DEBUFF, G-SYM-RADIO: not closed, not needed. Option B grants no item, applies no debuff and consumes nothing; they gate only the Option A deltas listed in the worknote.
- Still UATPending: client highlight on the tank, Livewire UI, the speaker label on dialog 4991 (not a monologue, so it takes the tank as speaker), journal text.
- Dialog 4989 stays unreferenced by the 1521 chains.
