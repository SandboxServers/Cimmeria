---
name: reference-bm-system-seller
description: Black Market system seller sentinel — reserved account/player ids, sequence bounds, settlement sink, and idempotent ensure pattern
metadata:
  type: reference
---

## Reserved system seller identifiers

`SYSTEM_ACCOUNT_ID = 1` and `SYSTEM_SELLER_ID = 1` (both `i32`) are defined as `pub const` in `crates/services/src/base/black_market/seed.rs`.

**Why 1 is safe:**
- `accounts_account_id_seq` starts at 2 — value 1 is permanently unreachable.
- `sgw_characters_character_id_seq` starts at 61 — value 1 is permanently unreachable.
- Seed data: account_ids 2–9, player_ids 62–70 (highest seeded `setval` is 70).

## DB rows

`ensure_system_seller(pool)` inserts via `ON CONFLICT DO NOTHING`:
- `account (account_id=1, account_name='Black Market', password='')`
- `sgw_player (account_id=1, player_id=1, level=1, alignment=0, archetype=1, gender=1, player_name='Black Market', extra_name='', world_location='CombatSim', bodyset='BS_HumanMale.BS_HumanMale', pos_x/y/z=0.0, skin_color_id=0, bandolier_slot=0)`

Called inside `seed_active_auctions` before listing INSERTs. Safe on every boot.

## FK constraints

`sgw_auction.seller_id → sgw_player(player_id) ON DELETE RESTRICT` — system seller is never deleted, so no auction becomes orphaned through cascade.

## Settlement path (sold seeded listing)

Sweep calls `send_mail_to_player(pool, SYSTEM_SELLER_ID, cash, ...)` → inserts `sgw_gate_mail.character_id = 1`. FK satisfied. Cash is a sink. Item re-materialises for the buyer. No special-casing needed.

## Column-set reference for minimal player inserts

When inserting a bare `sgw_player` row (tests or ensure_system_seller), the required NOT NULL columns are:
`account_id, player_id, level, alignment, archetype, gender, player_name, extra_name, world_location, bodyset, pos_x, pos_y, pos_z, skin_color_id, bandolier_slot`

`bandolier_slot` has no DEFAULT in the schema — must be supplied explicitly. All other optional columns can be omitted.

See `db/sgw/Players/Tables/sgw_player.sql` for the full column list.

## Live-DB test location

`crates/services/src/base/black_market/seed.rs` `#[cfg(test)] mod tests`:
- `ensure_system_seller_is_idempotent_and_satisfies_fk` — idempotency + FK guard
- `seed_auctions_use_system_seller` — regression guard (fails if real-player lookup restored)
