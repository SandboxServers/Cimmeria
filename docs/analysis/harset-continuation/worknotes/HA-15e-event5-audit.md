# HA-15e: event-5 item function audit

> Type: research worknote (docs-only), items-systems advisor, 2026-10-18, worktree tip `29f9d3fdb`. Decision: O18-07 in the [O18 register](owner-decisions-2026-10-18.md#o18-07--event-5-items). Brief: [work-packets, HA-15e](../work-packets.md#ha-15e--event-5-item-function-audit). Parent census: [HA-15 audit](HA-15-audit.md). Nothing here edits a seed, allocates an ID or runs a live system; every row below was read from `db/resources/` and `crates/` in this worktree. Labels follow rule R4: FACT (with file and row), INFERENCE (with confidence), DECISION, PROVISIONAL.

## 1. Answer in one screen

- **No event-5 binding on a Harset item consumes anything today.** FACT: `classify` in `crates/cell-content/src/cell/content/consumable_use.rs` returns `NotNative` (all four real abilities) or `Placeholder` (all 597 rows), and `try_native_use` returns `false` for both, so the use falls through to chains. The base also never consumes (`use_instance.rs`: "firing ItemUsed (no consumption — chain decides)").
- **The seed supports the intent of four abilities, not their mechanics.** Each real ability (2068, 2092, 2409, 2410) has one effect whose `effect_desc` and `name` describe the quest function, but every one of those effects has `script_name NULL` and no `effect_nvps` rows. Launching them applies nothing. A real function therefore has to be a content chain, and none of the four fits an `item_use` chain alone for the targeted uses (section 4).
- **Verdicts:** 4396 scanner PROVISIONAL, 4690 tag gun PROVISIONAL, 2743 Tollan tech PROVISIONAL, 2818 symbiote PROVISIONAL for mission 741 and PLACEHOLDER (no ability support) for any 1353 use, 2819 REAL (chain 6103 already live), 2823 and 2864 PLACEHOLDER. Table in [section 7](#7-verdicts).
- **The "event 5 plus `remove_item` consumes twice" trap does not fire for these seven items as seeded.** It needs a native-eligible ability (section 5). Two other double-consume shapes remain real. Guards in section 5.

## 2. The four real bindings

All four ability rows are in `db/resources/Abilities/Seed/abilities.sql` (rows 2068, 2092, 2409, 2410); effects in `db/resources/Effects/Seed/effects.sql`; binding rows in `db/resources/Items/Seed/items_event_sets.sql` (item_event_id 95, 119, 1144, 1476).

| Item | Binding | Ability name, type, range, target type | Effect (id, name, `effect_desc`) | `script_name` |
|---|---|---|---|---|
| 2743 Tollan Control Technology | `(2410, 5)` | `NO ABILITY DISPLAY NAME!`, `ABILITY_TYPE_Undefined`, max range 500, `target_type_id` 2, `TCM_Single`, cooldown 1 | 3472 "Use on Dawson": "Detects Goauld in Dawson, prompts hostility switch" | NULL |
| 2818 Goa'uld Symbiote | `(2068, 5)` | same placeholder name, `ABILITY_TYPE_Buff`, range 0, `is_ranged` true, `target_type_id` 2, cooldown 30 | 2745 "Infect Dawson": "Turns Dawson into a Goauld" | NULL |
| 4396 Straegis Scanner | `(2092, 5)` | same placeholder name, `ABILITY_TYPE_Undefined`, max range 500, `target_type_id` 1, cooldown 1 | 2816 "Use Straegis Scanner": "Search for Straegis using the Scanner." | NULL |
| 4690 Nanite Tracking Gun | `(2409, 5)` | same placeholder name, `ABILITY_TYPE_Buff`, range 0, `target_type_id` 2, icon `Buff_Aim`, cooldown 30 | 3471 "Use Tagging Gun": "Places a micro tracker on target" | NULL |

FACT for every cell above. What the columns do not say, and what stays INFERENCE:

- **The ability display name and description are the client placeholders** ("NO ABILITY DISPLAY NAME!"). They carry no information, so the effect `name` and `effect_desc` are the only intent evidence. This is the same legacy-extract shape the [rebuild audit](../../harset-rebuild/audit.md) cites for 2743 and 4396 (lines 171-175 there). INFERENCE, high: these four rows are legacy-authored quest abilities whose effect scripts were never extracted, not generic filler. Reasoning: each effect name is item-specific, none of them is shared with 597, and the other bindings in the same duplicate group (2716, 2828, 4651, 4709) fall back to 597.
- **`target_type_id` 1 against 2.** On the one row whose behaviour is known (597 "Heal Focus", `target_type_id` 1, heals the user) 1 reads as "self". 2068, 2409 and 2410 use 2, 2092 uses 1. INFERENCE, low: 2 means "another entity", 1 means "self". I did not find the enum in `docs/protocol/` and did not look in Ghidra (not allowed). This matches the intent text for 2410, 2068 and 2409 (act on Dawson or an NPC) and for 2092 (a self-centred scan pulse), which is why the pairing is worth keeping, but it is unproven.
- **Range 500 on 2092 and 2410, 0 on 2068 and 2409.** FACT only. Do not read a gameplay radius into it (INFERENCE, low).
- **Item flags differ.** 2743 and 4690 carry flags 32768, 4396 carries 35840, 2818 carries 36864 (`items.sql`). I did not decode the bits; the claim table does not depend on them.

## 3. The ability-597 placeholders

FACT: 597 is `Heal Focus` ("Heals 35% of the player's Focus pool", `ABILITY_TYPE_Heal`, cooldown 30, effect 659 `Focus Heal`, `script_name 'HealFocus'`, `HealPercentage 35`). `items_event_sets.sql` binds it under event 5 to 158 items. `consumable_use.rs` names it `PLACEHOLDER_ITEM_USE_ABILITY` and states the reason: firing it from a quest prop would hand out free focus heals. The gate is load-bearing (see the [native-consumable memory note](../../../../.claude/agent-memory/items-systems-advisor/native_consumables_and_stat_buffs.md)).

Per item:

| Item | Binding row | Chain on `item_use` | What happens on click today | Better-supported ability? |
|---|---|---|---|---|
| 2819 Jaffa Disguise | `(597, 5)`, id 120 | Chain 6103, trigger `item_use` key `2819`, in `harset_goauld_chains.sql` | Chain advances step 2504, shows blurb 2637, lights three baskets; nothing consumed. Reusable by design. | Not needed. FACT: the chain is the function. |
| 2823 Petbe's Robes | `(597, 5)`, id 122 | none | Placeholder, silent (debug log `consumable_skipped`, then no chain matches). Mission 1245 role unrecorded. | None found (below). |
| 2864 Scarab Listening Device Map | `(597, 5)`, id 129 | none | Same silent no-op. | Not needed: it is a turn-in token. |

The "better ability" search (INFERENCE, medium). I listed abilities and effects whose names mention Disguise, Robe, Scarab, Listening, Symbiote, Tagging, Scanner, Tollan or Dawson. The only wearable-disguise matches are the stat abilities `Disguise: Humanoid/Alien: Mark I-III` (1081-1083, 1185-1188), `MS020_Disguise:*` (2009-2014) and the `Disguise Boost: Consumable` family (2258, 2796-2799). All of them adjust a Disguise rating stat (effects 1229-1354, 3187). The campaign does not have a rating reader for them (the memory note records `DISGUISE_RATING` as unread by the server). They are not a quest costume. So no better-supported ability exists for 2823, and none is needed for 2819 or 2864, whose functions come from chains 6103, 6110 and 6111.

2864 detail (FACT): chain 6110 grants it (`add_item`), chain 6111 consumes it with one `remove_item` on the `dialog_choice` trigger 2640. Event 5 is not involved, and the 597 gate keeps a click on the map from healing.

## 4. Whether the source supports a real function

The test applied: the item name, the effect text, the legacy-extract lines in the rebuild audit and the current server primitives must agree. Server primitive facts used:

- FACT: `fire_item_use` builds its `TriggerEvent` with `target_entity: None` (`crates/cell-content/src/cell/content/event_dispatch/inventory.rs`), and the 2819 chain comment says the same ("`item_use` does not stamp `target_entity_id`"). An `item_use` chain therefore knows the user, the stack and the mission state, but not what the player had selected.
- FACT: the chain engine has `launch_ability` (`crates/content-engine/src/loader/action.rs`, executor in `cell-content/.../executor/dispatch.rs`) and an `interact_tag` trigger (`loader/trigger.rs`). Chain 1034 (the Ambernol vial) already pairs `launch_ability` with `remove_item`.
- INFERENCE, medium: "use this item on that NPC" is expressed today as an `interact_tag` chain on the NPC gated on the mission step, as the rebuild plan proposes for 1363 and 1365. Whether the real client sends a use-on-target request for these mission-bag items at all (so the player can click the item and then the NPC) is unverified. That needs the lab; this packet may not use it.

Per item:

- **4396 Straegis Scanner (mission 1374).** Source: item text "A scanner developed by Nerus to find Straegis", effect "Search for Straegis using the Scanner". The rebuild audit lists four scan anchors (Shield Controls, Bank, Market, Storage: steps 4087-4090) and says the scanner is the scan pulse (CLAIM from a draft, not client evidence). Function supported: a self-centred scan whose result depends on where the player stands. Needs: four region anchors (HA-01 placements, still BlockedEvidence) and an `item_use` chain gated on step plus region. INFERENCE, medium: REAL intent, mechanics PROVISIONAL. It must be reusable (four scans), so it belongs in `KNOWN_REUSABLES`, never consumed by the use.
- **4690 Nanite Tracking Gun (mission 1363).** Source: item text "A dart gun that fires tiny nanite trackers", effect "Places a micro tracker on target", five tag targets (rebuild audit line 110). Function supported: a targeted tag. A pure `item_use` chain cannot know the target (section 4 first bullet), so this is `interact_tag` on each of the five NPCs, gated on step 4047 and on the gun being held (the rebuild notes there is no `HasItem` condition; gate on the step). INFERENCE, medium. Must be reusable (five uses), must not aggro (spec M-10 per the rebuild audit), and the visible first-press feedback is the chain's job because the effect itself does nothing visible. PROVISIONAL. Not bandolier-eligible: `container_sets` is `{2}`, so `use_instance.rs` does not reroute the click to the equip path even though the icon is a pistol (FACT).
- **2743 Tollan Control Technology (mission 1365).** Source: item text "A Tollan tool that can seperate a Goa'uld from its host", effect "Use on Dawson: Detects Goauld in Dawson, prompts hostility switch". The effect text names Dawson only. The rebuild plan also has two optional trooper tests with the same item (steps 4673, 4672), which the effect text does not cover (INFERENCE, medium: the troopers are a rebuild addition, not source). Function supported: a detect that flips Dawson's per-instance clone hostile. Needs the Storage instance and `set_aggression` on the instance clone only (shared-world actors are never mutated per player, rule R6). PROVISIONAL.
- **2818 Goa'uld Symbiote (missions 741 and 1353).** Source for 741: effect "Infect Dawson" ("Turns Dawson into a Goauld") and the rebuild audit line 129 ("put Dawson at ease 2493 (+use symbiote 2902)"). Dawson only. For 1353 the rebuild plan has the tank grant 2818 and a separate volunteer implant effect; nothing in ability 2068 supports a use on the player. PROVISIONAL for 741, PLACEHOLDER for 1353 (ability 2068 supports no use there): if 1353 consumes 2818 it must do so at the hand-over (Nerus, step 4012) with an explicit chain, not through this ability. Open question for the owner: is the 741 symbiote consumed by the Dawson use or kept? The source does not say. The item is shared by two missions, so each mission's consume point must be separate and tested for "consumed once".

## 5. The consumption trace and the exact double-consume case

Trace, FACT unless marked:

1. Client `UseInventoryItem` reaches the cell, which sends `CellToBaseMsg::UseInventoryItem` (`cell-methods/.../inventory/item_ops.rs:242`).
2. Base `handle_use_inventory_item` (`base-methods/.../inventory/core/use_instance.rs`) checks ownership and container access, reroutes bandolier-eligible items (`container_sets` contains the bandolier container) to the move handler, hands crafting items to the plugin, and otherwise enqueues `CellOutboxPayload::ItemUsed` with the clicked `instance_id`. It logs "firing ItemUsed (no consumption — chain decides)". It consumes nothing.
3. The cell runs `fire_item_use`, which calls `try_native_use` first.
4. `try_native_use` owns the use only for a **Native** plan: an event-5 ability that is not 597, whose every effect has `script_name` in `HealHealth`, `HealFocus` or `StatBuff`, and for which `ChainEngine::has_item_use_chain(item)` is false. It then sends `ConsumeItemForUse`; the base takes one unit under a row lock and only on commit answers `ItemUseConsumed`, and the cell applies the ability. That is the only place event 5 consumes. A second click or an outbox redelivery finds no unit and applies nothing.
5. Every other case returns `false` and the cell resolves chains on `item_use::<item>`; consumption is whatever the matched chains' `remove_item` rows do (each uses the clicked `instance_id`).

Result for the seven items: 2743, 2818, 4396 and 4690 are `NotNative` (`effect_not_native`, because `script_name` is NULL) and 2819, 2823, 2864 are `Placeholder`. Their container preference is `{2}` (mission bag), so the "bag consumable with no effect yet" refusal does not apply either and the click is silent unless a chain matches. So event 5 contributes zero consumption for all seven, and a chain `remove_item` on `item_use` cannot double-consume against it. The audit and R6 wording "event 5 plus `remove_item` consumes twice" is a hazard class for native consumables, not a live state for these rows.

**The exact case where it would double-consume.** An item whose event-5 ability is Native (non-597, effects scripted `HealHealth`/`HealFocus`/`StatBuff`) AND which gets consumed again by a `remove_item` that does not ride an `item_use` chain for that same item, for example an `interact_tag` or `dialog_choice` chain that removes it. The click consumes one unit natively (no chain stands aside because none triggers on `item_use`), and the later chain removes a second. Seeding a script on 2092, 2409, 2410 or 2068 (or a copy of 597's `HealFocus` script) would create exactly this. It also arises if an `item_use` chain is disabled (`has_item_use_chain` checks `enabled`, not conditions): the native path takes the item and the disabled chain's `remove_item` no longer runs, so the stack is consumed once but the chain's mission effects are lost. The inverse, a chain enabled but gated off, still owns the item and does nothing, as authored.

**The guard that prevents it (FACT where it exists):**

- `has_item_use_chain` makes any enabled `item_use` chain own its item outright; the native path stands aside (`chain/mod.rs:235`, tests in `chain/tests.rs:557`).
- Native consumption is consume-first under a row lock (`ConsumeItemForUse`), so the native path cannot double-apply.
- The lint `crates/content-engine/tests/it/onitemuse_remove_item_pairing.rs` walks every `item_use` chain; `KNOWN_CONSUMABLES = [19, 2893]`, `KNOWN_REUSABLES = [3438, 5168, 2819]`. A new `item_use` chain on any other item id fails CI until it is classified. Items 4396, 4690 and 2743 will need adding to `KNOWN_REUSABLES` when their chains land (INFERENCE, high: all three are used several times).

**Two further double-consume shapes that are real** (carried from the earlier memory note, FACT of engine behaviour: every matching chain fires): (a) one chain with two `remove_item` rows for the same item; (b) two chains on `item_use::<same item>` with `once = false`, each removing a unit. Check both in HA-07, HA-08a, HA-09 and HA-11 reviews.

**Recommendation (INFERENCE, high):** leave `script_name` NULL on effects 2745, 2816, 3471 and 3472 and implement the functions as chains. The event-5 rows can stay in the seed as provenance, because the native path ignores them. Do not "implement" them by adding `HealHealth`/`StatBuff` scripts.

## 6. Provenance and what this does not prove

- Item-to-ability bindings are FACT from `items_event_sets.sql`. The effect text is the only intent evidence and comes from the legacy extract (the data is Project Giza's reconstruction, not the original database). The rebuild audit is a draft: a claim, not client evidence.
- Not verified: the client's request for a use-on-target of a mission item, the client rendering of any of these abilities (there are no display names), and the unknown enum `target_type_id`. All three need the lab or Ghidra; both are out of scope here.
- Not read: `docs/protocol/client-method-dispatch-table.md`; no wire propId is touched by this note.

## 7. Verdicts

REAL means the source and the live code already agree and the function ships. PROVISIONAL means the intent is supported by source but the mechanics need a chain plus unproven evidence. PLACEHOLDER means the event-5 row is inert filler and no function is claimed.

| Item | Verdict | Consumed on use? | Chain shape | Packet |
|---|---|---|---|---|
| 4396 Straegis Scanner | PROVISIONAL | No (reusable) | `item_use` + step + region gate, four anchors | HA-09 |
| 4690 Nanite Tracking Gun | PROVISIONAL | No (reusable, five tags) | `interact_tag` on five NPCs, step gate, no aggro | HA-07 |
| 2743 Tollan Control Technology | PROVISIONAL | No (reusable) | `interact_tag` on Dawson instance clone; troopers are rebuild additions | HA-08a |
| 2818 Goa'uld Symbiote | PROVISIONAL (741), PLACEHOLDER (1353 use) | Owner decision | 741: `interact_tag` on Dawson; 1353: hand-over consume only | HA-11 (741); mission 1353 packet (HA-04 to HA-14 range, owner assigns) |
| 2819 Jaffa Disguise | REAL | No (reusable) | Chain 6103, live | HA-11 (already seeded in the 742 group) |
| 2823 Petbe's Robes | PLACEHOLDER | Unknown | none; mission 1245 role unrecorded | mission 1245 packet |
| 2864 Scarab Listening Device Map | PLACEHOLDER (turn-in token) | By chain 6111 only | chains 6110 and 6111, live | HA-11 |

Unresolved decisions for the owner: (1) whether the 741 symbiote is consumed on the Dawson use; (2) whether the Tollan tool is also used on the two optional troopers; (3) that the four scanner anchors and the Storage instance exist (HA-01 evidence).

## Tests the implementation owes

| Test | Bug shape it guards | Revert that must fail it |
|---|---|---|
| Live-DB: every seeded binding for 2743, 2818, 4396 and 4690 classifies `NotNative` and 2819, 2823, 2864 classify `Placeholder` | A script added to 2745, 2816, 3471 or 3472 silently makes the item native and consumes on click | Set `script_name` to `HealFocus` on effect 2816: the test fails |
| Chain replay: click 4396, 4690 and 2743 repeatedly, assert stack count unchanged each time | A reusable tool consumed by a stray `remove_item` or a native consume | Add `remove_item` to the chain: fails |
| Chain replay: mission 741/1353 consumes 2818 exactly once per mission, with a relog between clicks | Double consume across the two missions that share the item | Add a second `remove_item`, or enable a duplicate chain with `once = false`: fails |
| Lint update: add 4396, 4690 and 2743 to `KNOWN_REUSABLES` in `onitemuse_remove_item_pairing.rs` | An unclassified `item_use` chain passes CI | Delete the list entry: the lint fails |
| Replay: `interact_tag` on a wrong-step player, wrong archetype, or without the item gives visible refusal text, no state change | Silent no-op and out-of-order advance | Remove the step gate: fails |

## Telemetry the implementation owes

- A `consumable_skipped` debug event already exists for placeholders and non-native abilities; keep it. Add an info-level chain-start event for each of the four function chains carrying `item_type_id`, `item_name`, `instance_id`, mission and step, and the decision outcome (applied, refused, duplicate).
- Refusals (wrong step, missing item, wrong target) must log a WARN with the reason, the player and the NPC tag, with names alongside ids, so a silent click is debuggable from the log alone.
- Consumption telemetry for 2818 must record which mission and step consumed it, so a double consume shows as two events on one instance id.
