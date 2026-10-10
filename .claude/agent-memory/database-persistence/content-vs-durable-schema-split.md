---
name: content-vs-durable-schema-split
description: resources schema = reseedable content (86 tables), public schema = durable player state (25 tables); the cross-schema FKs and the resources."EAmmoType" column type that make a content reseed destroy data
metadata:
  type: project
---

Verified 2026-10-10 at `87912c644` while drafting the #1290 ADR (`docs/architecture/persistent-database-profile.md`, Proposed).

- `db/database.sql` loads `resources` (from `db/resources/`, 86 tables, 131 enum types + 1 composite) then `public` (from `db/sgw/`, 25 tables). No runtime code creates or migrates schema; no sqlx migrations exist.
- Cross-schema hard deps from durable into content: FKs `sgw_inventory.type_id`, `sgw_gate_mail_item.type_id`, `sgw_organization_vault_items.type_id` -> `resources.items`; `sgw_player.world_id`/`world_location` -> `resources.worlds`. And the column TYPE `resources."EAmmoType"` on `sgw_inventory_base` (inherited by `sgw_inventory`), `sgw_gate_mail_item`, `sgw_organization_vault_items`. `DROP SCHEMA resources CASCADE` would drop those durable columns.
- Many soft refs (int arrays, no FK) from `sgw_player`/`sgw_mission`/grants/tutorials into abilities, stargates, respawners, missions, dialogs, disciplines, blueprints. Full list in the ADR §3.3.
- Runtime writers into `resources`: admin-api content editor (`content_*`), `resource_update_trigger` -> `resource_versions`, nothing else outside `#[cfg(test)]` (the GM `.`-console only formats seed SQL). Everything else in `resources` is read-only at runtime.
- The `public` seed is not empty: 14 GM dev/lab accounts (password `test`), 9 playtest characters with starting kit, contact lists, 1 shard.

**Why:** any persistent-DB or reseed design has to handle these before touching `resources`.
**How to apply:** before proposing a content reload against a live DB, check the ADR's decisions (D-5, D-6) and whether `EAmmoType` has moved out of `resources`. Related: [[sgw-mission-objective-arrays]].
