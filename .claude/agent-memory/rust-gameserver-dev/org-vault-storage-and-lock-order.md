---
name: org-vault-storage-and-lock-order
description: Team/Command vault (BV-07) - items live in sgw_organization_vault_items not sgw_inventory; vault moves take advisory locks, then KEY SHARE on the player, then lock_org; refusals must never resend the routed org
metadata:
  type: project
---

BV-07 (2026-09-27) put org vault items in a standalone `sgw_organization_vault_items` (mail-escrow precedent), not a nullable `sgw_inventory.character_id`: some `sgw_inventory` writers filter by `item_id` alone (`inventory/ammo.rs`), so org rows there would be reachable by them. Items cross tables as `INSERT ... SELECT` + `DELETE`; item_id uniqueness across tables is sequence discipline only, asserted by `Fx::duplicated_ids` in the tests.

Lock order for any org-scoped item/cash write (two deadlocks found, one by `database-persistence`, one by `server-authority-enforcer`):
1. per-player advisory locks `(player, 0)` and the carried container's (vendor and trade take these before `sgw_player FOR UPDATE`);
2. `FOR KEY SHARE` on the actor's `sgw_player` (a withdraw's `sgw_inventory` insert takes KEY SHARE via its FK; a character delete holds that row and waits on its orgs);
3. `lock_org` / `member_access_locked`;
4. item rows (vault rows only after the org lock).

**Why:** both orders were real ABBA cycles; Postgres aborts one side after ~1 s.

**How to apply:** BV-08 cash and BV-09 expansion follow the same order. A refusal's snap-back must resend only rows of an org the session names and the player is a member of: `route`'s unlocked read of a client `item_id` is attacker-chosen (review B1). Also: `0x7000_B7xx` is BV-03's sentinel range; BV-07 uses `B8xx`/`B9xx` - grep a prefix before claiming it. See [[vacuous-guard-and-sentinel-collision-review]], [[forced-db-race-share-lock]].
