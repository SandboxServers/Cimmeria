---
name: omega-vendors
description: UO-06 Omega vendor proposal facts - guard only binds sell list 2 designs 21/55/3437, no price column, no sibling-zone vendor lists, vendor spans carry no outcome field
metadata:
  type: project
---

UO-06 (2026-10-18) wrote `docs/analysis/union-zone-planning/omega-packets/worknotes/UO-06-services.md`: eight Omega vendor templates 63812-63819, lists 63802-63809, owner questions D-UO8 and D-UO16.

- The arbitrage guard (`live_db_vendor_arbitrage.rs`) only constrains buy rows for designs 21 (1000), 55 (300) and 3437 (100), the whole of sell list 2. Any other design has no sell-back price, so the guard is vacuous for it.
- `resources.items` has no price, level or faction column. Prices are formulas, never evidence. `tech_comp` as a level gate is unconfirmed in docs.
- No vendor list for any other zone exists in the tree; "mirror a comparable zone" has no basis.
- Vendor spans (`vendor.open_store`, `vendor.purchase`, `vendor.sell`) record identity fields only. Success is a `target: "vendor"` INFO event from `vendor/telemetry.rs`: `store_opened` (action open), `transaction` (action buy / sell).
- Template 1302's `interaction_type` 49152 is two bits; Omega needs one. Its components include a weapon.
- UO-01 puts `svc.consumables` and `svc.common_crafting` in the SGC wing, against D-UO16 option A's Common Area wording.
