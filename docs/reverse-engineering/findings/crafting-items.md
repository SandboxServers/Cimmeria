# Crafting Items — Blueprint Scrolls, Racial Paradigm Guides, Field Crafting Tools

> **Date**: 2026-09-26
> **Packet**: CR-E2 (Crafting & Applied Science campaign)
> **Confidence**: mixed — see per-question sections; every claim below is backed by a
> cooked-data byte offset or a Rust source citation, never inferred from the seed alone.
> **Sources**: `data/cache/CookedDataItems.pak` (category 4, 6,059 `COOKED_ITEM` entries),
> `data/cache/CookedBlueprints.pak` (category 15, 498 `COOKED_BLUPRINT` entries),
> `db/resources/Items/Seed/items.sql`, `db/resources/Entities/Seed/blueprints.sql`,
> `db/resources/Texts/Seed/texts.sql`,
> `crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs`,
> `docs/gameplay/inventory-system.md`, `docs/engine/cooked-data-pak-format.md`,
> `docs/reverse-engineering/findings/crafting-restoration.md`

## Summary

The client's cooked crafting data (`CookedDataItems.pak` / `CookedBlueprints.pak`) has **no
field anywhere that links a "Blueprint: …" item to the blueprint it teaches**, no field that
names which paradigm a "Racial Paradigm Guide" item raises, and no field that names which
applied science a "Field Crafting Tool" belongs to. All three families are stitched together
in the client (and now in this mapping) purely by **item name** — the cooked schema treats
every one of them as an ordinary elementary-component item (`IsElementaryComponent="true"`)
that fires a generic "use" event with no ability attached (`AbilityID="0" EventID="5"` for
scrolls/guides; Field Crafting Tools have no `<ItemEventSet>` at all). None of the 289
Blueprint-named items are ever referenced as a component inside any of the 498 blueprints'
component lists — they exist solely as name-carrying "teach" items, never as crafting
ingredients, which is consistent with (does not contradict) D-CR04's server-side design.

Racial Paradigm Guide items (Q2) **do already exist** in cooked data — 5 items, one per
paradigm, fully described (icon, description text matching D-CR03 verbatim, stackable to
100) — so no seed-only rendering workaround is needed. Field Crafting Tools (Q3) carry **no
applied-science or tool-type field**; the science is only readable from the name prefix
(BMAS/EAS/PSAS/MAS) or the free-text `Description` attribute, and their `AppliedScienceID`
attribute is uniformly `0` across every sampled tool. The `{17,15}` container restriction the
packet asked to confirm is exactly what the cooked data shows (Q3), but it is not
tool-specific — every item in this campaign's scope (blueprints, guides, tools) carries the
identical `ContainerSet` pair `{17, 15}`; container 17 has no entry in `bag_max_slots`
(`crates/wire/src/containers.rs`) and is not a real player-carryable bag, so bag 15
(`INV_Crafting`) is the only bag a player can actually hold these items in.

The item→blueprint mapping (Q1) resolves as follows out of 289 Blueprint items:

| Method | Count | Confidence | What it means |
|---|---:|---|---|
| `name-exact` | 36 | HIGH | The scroll's stripped name equals a blueprint's product name exactly, and only one blueprint has that product name. |
| `tc-suffix-match` | 154 | HIGH | The scroll's name is `Blueprint: <Archetype> Minigame Consumable TC<N>`; a cooked product `Minigame Consumable: <Archetype>` exists with `TechComp == N`, and exactly one blueprint produces it. |
| `name-exact-disambiguated` | 1 | HIGH | One of two same-named product candidates was picked using tier-progression evidence (see Q1.4 below). |
| `structural-tier-progression` | 1 | MEDIUM-HIGH | Resolved via cross-family structural analogy after finding the cooked data itself mislabels the target product (a genuine client-shipped naming bug, documented in Q1.4). |
| `name-exact-ambiguous` | 1 | MEDIUM (unresolved) | Two blueprints share the literal product name "Health Antidote"; nothing in cooked data disambiguates them. Reported as both candidates. |
| `subcombine-group-unresolved` | 45 | NONE (unresolved) | The scroll's name only narrows the answer to a group of 5–15 same-suffix candidate products; no field picks the specific one. |
| `tc-suffix-no-product` | 48 | NONE (dead end) | The named product was never shipped in cooked data — mostly the "Slappack Consumable" family. Not a mapping failure; the client never authored these products. |
| `UNMATCHED` | 3 | NONE (unresolved) | No cooked product exists under any name variant tried. |

Full row-by-row mapping: `docs/analysis/crafting/source/blueprint-items.csv`.

## Plain-language explanation

Imagine every "Blueprint: X" item in your inventory as a scroll with a title printed on it.
The client's data files never actually say *"this scroll teaches recipe #412"* — there is no
recipe-number field on the scroll at all. The only way to guess which recipe a scroll teaches
is to read its title and go looking for a recipe whose finished product has the same name.
For about two-thirds of the scrolls (the "Minigame Consumable" boost items and the plain
potion/stimpack scrolls) that trick works perfectly, because the title and the product name
really do match once you strip the word "Blueprint:". For the rest — mostly a family of
generic "Tier 1 <Science> Subcombine Aa/Ab/Ac…" scrolls — the title only narrows things down
to a small family of similarly-named ingredient items (there are 5–15 candidates sharing the
same title fragment), and nothing in the data says which exact one is meant. And for a chunk
of "Slappack" boost scrolls, the "finished product" they're supposed to teach was **never
actually built** by the original developers — the scroll exists, but the reward it promises
does not. Racial Paradigm Guides, by contrast, are simple and complete: five items, one per
in-game "race," fully described and ready to use exactly as the campaign designed them.

## Q1 — Item → blueprint link (evidence)

### Q1.1 No structural link exists

`CookedBlueprints.pak` entry `_412` (Titanium Plating, the campaign's UAT recipe):

```xml
<COOKED_BLUPRINT ...><ID>412</ID><ProductID>5401</ProductID><Quantity>1</Quantity>
<DisciplineID>21</DisciplineID><IsAlloy>false</IsAlloy>
<RequiresElementaryComponents>false</RequiresElementaryComponents>
<BlueprintComponentSets>...</BlueprintComponentSets></COOKED_BLUPRINT>
```

There is no `ItemID`, `TeachItemID`, or similar field naming a scroll. `CookedDataItems.pak`
entry `_5395` ("Blueprint: BioMedical SubComponent A"):

```xml
<COOKED_ITEM ... IsElementaryComponent="true" ... AppliedScienceID="0" ...
Name="Blueprint: BioMedical SubComponent A" ID="5395">
<InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="1"></InventorySet>
<ItemEventSet AbilityID="0" EventID="5"></ItemEventSet>
<ContainerSet>17</ContainerSet><ContainerSet>15</ContainerSet>
<Moniker MonikerID="552498541"></Moniker></COOKED_ITEM>
```

`IsElementaryComponent="true"` on every one of the 289 items — cooked data classifies these
as raw crafting materials, not as "knowledge" items. Cross-checked: **zero** of the 289
Blueprint item ids appear as an `ItemID` inside any of the 498 blueprints'
`BlueprintComponentList` entries, so they are never actually consumed as an ingredient either
— they are pure name-carrying, mechanically-inert items in the client's own data. D-CR04's
choice to make *using* them teach a blueprint is a Cimmeria design decision layered on top of
inert items, not something the client enforces or contradicts.

`MonikerID="552498541"` is identical across every sampled item in this campaign (blueprints,
guides, tools) — it does not encode anything per-item; the display string comes straight from
the `Name` attribute, not a localization lookup. (`texts.sql`'s `DN_It_Cft_Blueprint_*` rows
mostly just restate the same name or are empty — not a useful join key either.)

### Q1.2 Where plain-name matching works (36 rows, HIGH)

Where the client actually gave a scroll its product's exact flavor name (e.g., `8870
"Blueprint: Omni Antidote"` → blueprint `377`, product `6664 "Omni Antidote"`), stripping
`"Blueprint: "` and normalizing case/punctuation resolves the row unambiguously. This
includes all of the Antidote and Mark III/V/VII/X Stimpack scrolls except the two
disambiguated below.

### Q1.3 Minigame Consumable family (154 rows, HIGH)

`Blueprint: <Archetype> Minigame Consumable TC<N>` items (7 archetypes × 22 `TC` values =
154) match a cooked product `Minigame Consumable: <Archetype>` whose own `TechComp` attribute
equals `N` exactly — verified for all 7 archetypes (Asgard, Archaeologist, Commando, Goa'uld,
Jaffa, Soldier, Scientist), each with 22 `TechComp`-distinct product instances (`6798`/`7810`–
`7829` for Asgard, etc.). Each resolved product has exactly one producing blueprint. The
seed's own name string for the Goa'uld variant is SQL-escaped (`Goa''uld`); the cooked pak's
`Name` attribute is the clean `Goa'uld` and was used as the matching source of truth (see
Q1.6 seed-vs-cooked note).

### Q1.4 The Mark VII / Mark X Stimpack naming bug (2 rows, HIGH / MEDIUM-HIGH)

Item `8938` ("Blueprint: Mark VII Stimpack: Intellect/Morale") and `8939` ("Blueprint: Mark X
Stimpack: Intellect/Morale") both name-match a product called "Mark VII Stimpack:
Intellect/Morale" — there are **two** such products, `6726` (`TechComp=35`) and `6727`
(`TechComp=50`). Comparing against the parallel Coordination/Engagement pair (products
`6697`/`6717`/`6718`, correctly labeled Mark V/VII/X at `TechComp` 25/35/50, blueprints
`386`/`392`/`398`) shows the expected tier ladder is 25/35/50 → Mark V/VII/X. Product `6726`
sits at the correct "Mark VII" position (`TechComp=35`); product `6727` sits at the "Mark X"
position (`TechComp=50`) but its cooked `Name` attribute was never updated — **a client-
shipped copy/paste naming bug**, not a seed error. Blueprint `402` is the sole blueprint
producing `6727`. Resolution: `8938` → blueprint `396` (product `6726`, correctly labeled);
`8939` → blueprint `402` (product `6727`, mislabeled in cooked data). This is exactly the
class of shipped-but-wrong client naming this campaign's evidence trail should flag rather
than silently "fix" — CR-15 should treat product `6727`'s in-game tooltip name as wrong and
either accept the client showing "Mark VII" for the Mark X item, or correct it via a cooked-
data item override (see `crates/resources/src/base/resources/apply_overrides.rs` pattern) if
UAT finds the wrong label confusing.

### Q1.5 Genuinely unresolved rows

- **`8882` "Blueprint: Health Antidote"** — two blueprints, `367` (product `6657`, discipline
  23 "Breakthrough Biochemistry") and `369` (product `6659`, discipline 61 "Naquadah Energy
  Systems"), both literally named "Health Antidote" at identical `TechComp=10`. Nothing in
  cooked data disambiguates them (the item itself carries `discipline_ids: '{}'` in the seed).
  Reported as both candidates; CR-15 needs an owner call (e.g., pick by lower blueprint id, or
  teach both).
- **`5395`/`5439` "Blueprint: BioMedical SubComponent A/B"** — no cooked product is named
  "BioMedical SubComponent A" or "…B" anywhere (searched the full 6,059-item pak); these two
  low-id scrolls (part of the oldest, pre-"Subcombine" naming generation alongside `6483`)
  have no surviving target at all. Confirmed dead ends.
- **`8885` "Blueprint: Mental Antidote"** — no cooked product is named exactly "Mental
  Antidote"; only compound names exist (`"Kinetic/Mental Antidote"`, `"Health/Mental
  Antidote"`, etc.). Confirmed dead end.
- **45 "Tier 1 `<Science>` Subcombine `<Group><Variant>`" scrolls** — see Q1.6.
- **48 "`<Flavor>` Slappack Consumable TC`<N>`" scrolls** — see Q1.6.

### Q1.6 Two structurally different "unresolvable" buckets

**Subcombine scrolls (45 rows, `subcombine-group-unresolved`).** These items (ids in the
8400s–8600s range, e.g. `8418` "Blueprint: Tier 1 Power Systems Subcombine Aa") are a
*second, later* naming generation layered on top of an older one: the oldest generation (ids
in the 5300s–6483 range, e.g. `6483` "Blueprint: Steel Plating (Materials Subcombine A)")
gives each scroll a real flavor name that matches its product 1:1 (resolved under Q1.2/Q1.3).
The newer generation was apparently never given real names before shipping — it only encodes
"Tier `<T>` `<Science>` Subcombine `<Group><Variant>`" where `<Group>` is a single letter
(A–D) and `<Variant>` is a second lowercase letter (a/b/c). Each `(Science, Group)` pair
matches **5–15** distinct cooked products sharing the display-name suffix
`"(<Science> Subcombine <Group>)"` — e.g. `("Power Systems", "A")` has 15 candidates (`5385,
5386, 5387, 5402, 5484, 5485, 5548, 5549, 5550, 5603, 5604, 5605, 5656, 5657, 5658`). The
block spacing (four numeric ranges roughly 5335xx/5480xx/5540xx/5600xx per science) strongly
suggests these are **quality-tier duplicates** (Normal/Good/Great/Fantastic, per audit C-38's
quality scale) of the same 3–4 named components, but nothing in either pak — no quality field
on the blueprint, no per-tier product reference on the scroll — confirms that hypothesis or
gives the `<Variant>` letter an ordinal meaning. The CSV lists every same-group candidate
blueprint/product id for these 45 rows; **do not treat the listed ids as authoritative** —
they are the full candidate set, not a resolved answer. Resolving this further needs either a
client-side UI trace (does the crafting page show a specific quality when using one of these
scrolls?) or an owner decision to drop this generation of scrolls from D-CR22's seed table
entirely, since — per Q1.1 — none of them function as recipe ingredients either.

**Slappack scrolls (48 rows, `tc-suffix-no-product`).** `Blueprint: <Flavor> Slappack
Consumable TC<N>` items exist for Disguise (8), Energy (10), Focus (10), Health (10), and
Stealth (9) at various `TC` values 5–50. The **only** non-blueprint items with "Slappack" in
their cooked name are two copies of `"Health Slappack TC1"` (ids `2893` `TechComp=1` and
`4735` `TechComp=18`) — a display-name bug in its own right (two different tech-comp items
sharing one hard-coded "TC1" label) — and neither matches any of the 48 scroll `TC` suffixes.
No "Energy/Focus/Stealth/Disguise Slappack" product exists in cooked data at all. This is not
a matching failure: **the reward these 48 scrolls promise was never built.** CR-15 should
either exclude this family from the seed table or treat it as a documented gap.

## Q2 — Racial Paradigm Guide items (HIGH confidence, resolved)

All 5 guides already exist in `CookedDataItems.pak`, ids `7805`–`7809`, one per paradigm
(Human, Common, Asgard, Goa'uld, Ancient — the same 5 paradigms as `racial_paradigm.sql`):

```xml
<COOKED_ITEM ... IsElementaryComponent="true" TechComp="33" IconLocation="set:ItemIcon003
image:Earth_PDA_MC" Tier="3" AppliedScienceID="0" QualityID="2000" Description="Permanently
increases player's Racial Paradigm score by one, to a maximum of 10." Name="Racial Paradigm
Guide: Human" ID="7805">
<InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="100"></InventorySet>
<ItemEventSet AbilityID="0" EventID="5"></ItemEventSet>
<ContainerSet>17</ContainerSet><ContainerSet>15</ContainerSet>
<Moniker MonikerID="552498541"></Moniker></COOKED_ITEM>
```

The `Description` text matches D-CR03's cited client text verbatim. All 5 already exist as
rows in `db/resources/Items/Seed/items.sql` with matching descriptions. There is no numeric
"paradigm id" attribute — same pattern as blueprints, the paradigm is only readable from the
`Name` suffix, matched against `racial_paradigm.sql`'s `name` column (`Human`, `Common`,
`Asgard`, `Goa'uld`, `Ancient`). Because the client already ships and can fully render these
items (icon, description, stacking to 100), **D-CR22's fallback question ("if not, what does
a seed-only item need to render") does not apply** — no cooked-data workaround is needed.

## Q3 — Field Crafting Tools (HIGH confidence, negative finding)

48 items, ids `5369` and `8402`–`8466` (contiguous with the tool family described in
D-CR21). Sampled across all four sciences and several `TC` grades (5, 10, 15, 35):

```xml
<COOKED_ITEM ... AppliedScienceID="0" ... Description="Bio-Medical Engineering Field
Crafting Tool" Name="BMAS-5 Field Crafting Tool" ID="5369">
<InventorySet IsDeletable="true" IsSellable="true" MaxStackSize="1"></InventorySet>
<ContainerSet>17</ContainerSet><ContainerSet>15</ContainerSet>
<Moniker MonikerID="552498541"></Moniker></COOKED_ITEM>
```

- **`AppliedScienceID="0"` on every sampled tool** — the numeric applied-science field is
  never populated for tools (same as for every other item family in this doc). The science
  is only readable from (a) the name prefix (`BMAS`/`EAS`/`PSAS`/`MAS`) or (b) the free-text
  `Description` attribute ("Bio-Medical Engineering Field Crafting Tool", "Power Systems
  Engineering Field Crafting Tool", "Materials Engineering Field Crafting Tool", "Electronics
  Engineering Field Crafting Tool"). There is no `ToolType`, `IsCraftingTool`, or similar
  boolean/enum field in the schema at all.
- **No `<ItemEventSet>` element** — unlike blueprints/guides (which carry `AbilityID="0"
  EventID="5"`), Field Crafting Tools have no use-event entry whatsoever. This is consistent
  with D-CR21's "Tools are not consumed" rule and (see Q4) means the client's generic "use"
  event category never applies to them.
- **Container restriction confirmed as `{17, 15}`, not tool-specific.** Every item sampled in
  this campaign — tools, blueprint scrolls, paradigm guides — carries the identical
  `ContainerSet` pair `17, 15`. Container `17` has no entry in `bag_max_slots`
  (`crates/wire/src/containers.rs:9-18`, falls through to `_ => 0`), so it is not a real
  player-carryable bag (most likely a vendor/sellable-stock pseudo-container, consistent with
  `IsSellable="true"` appearing on nearly every item sampled). Bag `15` (`INV_Crafting`) is
  therefore the only bag a player can actually hold these items in — D-CR21's restriction
  holds, just not because of anything unique to tools.

## Q4 — The item-use path (server: HIGH; client gating: MEDIUM/inferred)

**Wire path (confirmed in Rust, `docs/gameplay/inventory-system.md`).** The client sends one
generic cell method for every item type: `useItem(itemID, targetID)` — no separate "learn
blueprint" or "raise paradigm" verb exists on the wire. Server-side,
`handle_use_inventory_item` (`crates/base-methods/src/base/world_entry/methods/inventory/core/use_instance.rs:81`)
resolves the inventory instance, and — except for the bandolier auto-equip/unequip short
circuit — **always fires the `ItemUsed` content-engine event regardless of the item's cooked
`AbilityID`/`EventID`**; it does not check whether the item "has an ability" at all. The only
refusal path is "instance not found for this character" (not owned / already consumed). This
mirrors the legacy Python reference cited in the file's own doc comment
(`python/cell/Inventory.py:419-432`): `useItem` is a pure event-fire, and per-item behavior
(including whether anything is consumed) is entirely the chain/handler's decision. **This
means the current Rust implementation already accepts a `useItem` call for a Blueprint,
Guide, or Field-Crafting-Tool item with no special-casing — CR-15 only needs to add a
type-id-keyed handler that reacts to the fired event** (learn blueprint / raise paradigm),
matching the `OnItemUse`-content-chain pattern documented in
`docs/content/consumable-via-onitemuse-pattern.md`.

**Client-side gating (inferred, not decompiled this session).** Audit finding C-40 already
established that every client-side *crafting-page* check "logs a warning through
`Mercury__unknown_00ceae50` and sends the request anyway" — the client never blocks a
crafting verb locally. For the generic item right-click "Use" action specifically, this
session did not decompile the client's context-menu enable/disable logic; the only new
evidence is structural: Field Crafting Tools ship with **no** `<ItemEventSet>` element at all
(see Q3), while every Blueprint/Guide item ships `AbilityID="0" EventID="5"`. Given the
client's XSD schema names this element `CookedData:ItemEventSetType`
(`docs/engine/cooked-data-pipeline.md:303`) and every other "usable" item in the pak (potions,
kickers, Ambernol Vial `_19`) also carries an `ItemEventSet`, it is a reasonable but
**unconfirmed** hypothesis that the client's own item context menu keys its "Use" option off
`ItemEventSet` presence — meaning Field Crafting Tools likely show no "Use" option at all
(consistent with "tools are not consumed," they'd be equipped/toggled some other way, or
simply inert from the player's perspective outside the crafting UI). Raising this to HIGH
confidence needs a decompiled read of the item-context-menu build function (the counterpart
to `Crafting_isCraftTypeAllowed` at `0x00e465d0` documented in `crafting-restoration.md`) or a
live client trace of right-clicking a Field Crafting Tool — neither was done this session.

## 2009-vs-2026 notes

- The client's own crafting-item authoring is unfinished in at least two ways this campaign
  can now cite with evidence: the Slappack Consumable reward family (48 scrolls, Q1.6) was
  never given a producible product, and the second-generation Subcombine scroll naming (45
  scrolls, Q1.6) was never given per-item flavor names before shipping the older generation's
  1:1-named counterparts already existed right next to them in the id range. Neither gap is a
  Cimmeria regression — it is exactly the "2009 shipped it broken" case the archaeology
  methodology exists to distinguish from a Cimmeria bug.
- The `Mark VII`/`Mark X` Stimpack duplicate label (Q1.4) is a small, isolated instance of the
  same class of bug `annotation-script-shift-bugs.md` catalogs for annotation scripts — here
  it is in the client's own cooked *data*, not an annotation artifact, so it is a genuine
  shipped content bug rather than an RE-tooling error.
- D-CR04's "using a Blueprint item teaches its blueprint" design is a **Cimmeria-authored
  reinterpretation** of items the client itself never wires to any blueprint — it is not
  contradicted by anything in cooked data (the items are otherwise completely inert), but it
  is also not confirmed by anything in cooked data. Nothing here disputes the owner's
  approved decision; it just means CR-15's seed table is the **only** place this link will
  ever exist.

## Open questions

1. Does the client actually suppress the "Use" context-menu entry for items with no
   `ItemEventSet` (Field Crafting Tools)? Needs a decompiled read of the item right-click
   menu builder or a live client trace.
2. Are the 45 Subcombine-group candidate products (Q1.6) really quality-tier duplicates of
   3–4 named components, and if so, does the crafting page ever show which quality a given
   scroll produces? Needs either further Ghidra work on the crafting UI or an owner decision
   to drop this scroll generation from the CR-15 seed table.
3. Should CR-15 seed a mapping for `8882` "Blueprint: Health Antidote" to both blueprint `367`
   and `369`, or pick one? Owner/CR-15 call — no cooked-data evidence favors either.
4. Should product `6727`'s cooked-mislabeled tooltip ("Mark VII" for what is functionally the
   Mark X tier item, Q1.4) be corrected via a Cimmeria item-name override, matching the
   pattern in `apply_overrides.rs`? Cosmetic; does not block CR-15's blueprint linkage.

## Cross-reference targets

- `docs/analysis/crafting/README.md` — D-CR21 (tool science/tech_comp rule), D-CR22 (item-use
  mapping design) can now cite this doc's Q1–Q4 answers directly instead of "TBD".
- `docs/analysis/crafting/work-packets.md` — CR-E2's acceptance criteria are met; CR-15's
  scope should reference this doc's confidence table when writing the seed table and its
  guard.
- `docs/gameplay/inventory-system.md` — could gain a one-line pointer to this doc under "Item
  use" once CR-15 lands the type-id-keyed handler.
- `docs/engine/cooked-data-pak-format.md` — this doc's `ItemEventSet`/`AppliedScienceID`
  observations corroborate (and slightly extend) that document's existing schema notes; no
  correction needed there.
