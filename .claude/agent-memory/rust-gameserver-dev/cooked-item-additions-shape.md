---
name: cooked-item-additions-shape
description: Shipped COOKED_ITEM XML shape (not alphabetical, one \n, SOAP namespace prologue), new item ids go in item_overrides::ITEM_ADDITIONS, and AmmoType_Icons image names equal the EAmmoType labels
metadata:
  type: reference
---

Learned 2026-09-28 while doing ammo AM-07 (whole-entry item additions for ids 9000-9014).

- **Shipped item XML is not alphabetical.** The `item_overrides` doc comment and its `SAMPLE_2893` fixture claim alphabetized attributes and self-closed `<InventorySet .../>`. The committed `data/cache/CookedDataItems.pak` really has: the XML declaration, one `\n`, then `<COOKED_ITEM xmlns:SOAP-ENV=... xmlns:CookedData1="SGW" IsReverseEngineerable IsResearchable IsElementaryComponent IsKicker TechComp IconLocation Tier AppliedScienceID QualityID Description Name ID>`, then explicit close tags (`<InventorySet ...></InventorySet>`). There is no `ItemFlags` attribute. `generate_item_xml` reproduces `_10` and `_1086` byte for byte.
- **PAK facts.** 6059 items, ids 10-8951, and every item has `IsElementaryComponent="true"`. Container 15 only ever appears as `17, 15`.
- **New item ids** go in `crates/resources/src/base/item_overrides/` `ITEM_ADDITIONS`. `apply_item_overrides` inserts them, skips any id the PAK ships, and folds them into the items bump. Since #840, the bump alone makes the client resync the category, additions included.
- **Ammo icons.** The client's `AmmoType_Icons` imageset, declared in `TaharezLook.scheme`, names its images exactly after the `EAmmoType` labels (`Bullet_Hollow_Point`, `Dart_Stim`, the six `Dagger_*`). Use `set:AmmoType_Icons image:<label>`.
- **Granting any item for a UAT:** `/gmgiveitem <id> <qty>` (qty 1-1000).

**How to apply:** before authoring cooked XML for any category, diff the generator against a real shipped entry from `data/cache/`, not against a hand-written fixture. The fixtures drifted from the real shape.
