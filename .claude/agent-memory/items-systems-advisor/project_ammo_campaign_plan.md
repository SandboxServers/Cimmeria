---
name: project-ammo-campaign-plan
description: Ammo campaign (#1026) plan-only PR — wide-fan-out packet ledger, item-id/family choices, and the AM-01 RE findings that reshaped it
metadata:
  type: project
---

Ammo campaign (issue #1026) plan PR (AM-00) opened 2026-09-28, ledger at `docs/analysis/ammo/`. Restores special ammo (Hollow Point, Armor Piercing, Incendiary, EMP, Explosive, darts) as a finite, lootable bag item, while `Bullet_Default`/`Dart_Default` stay free-reload. Restructured mid-plan from the issue's serial suggestion into a wide fan-out: one small serial gate (AM-F: reserved item ids 9000-9014, `ammo_item_types` mapping table, `AmmoReserve`/`AmmoModifier` Rust contracts, `ammo.finite_special` flag), then 6 parallel Wave-1 packets (AM-02..AM-07), then 6 parallel Wave-2 family packets (AM-08..AM-11c), then AM-12 close-out.

**Why:** the owner explicitly asked for wide parallel fan-out with bottlenecks cleared up front, after the issue's own suggested order would have left most agents idle serially.

**How to apply:** any future ammo-campaign session should read `docs/analysis/ammo/work-packets.md`'s file-ownership matrix before touching `registry.rs` (cell-effects dispatch, one match arm per family, contended) or `db/database.sql` (every new seed file needs a `\ir` line, contended). Every packet resolves ammo items by `EAmmoType` through `ammo_item_types`, never a hardcoded item id — this is what let Wave 1 proceed without waiting on AM-01's RE findings.

**AM-01 (RE, PR #1040, `docs/reverse-engineering/findings/ammo-system.md`) landed while this plan was being written and reshaped it:**

- No reserve model of any kind exists in the 2009 client schema (HIGH confidence absence) — D-AM01 (items in bags) is a restoration design choice, not a recovery. `knownAmmoTypes` is a discovery/unlock flag array, not a count.
- Widening a weapon's `ammo_types` needs **no client push** — `getAmmoTypes`/`getCurrentAmmoType`/`requestAmmoChange` all resolve through a live, server-populated container cache (`SGWPlayer+0x8c → +0x24`), not `CookedDataItems.pak`. This is a DB-only change; narrows the client-push packet's scope to only the new ammo item *definitions*.
- Toggle abilities 715 (Hollow Point)/719 (Armor Piercing, effect 747) are architecturally **independent** of `requestAmmoChange` in the client — no auto-engage link found. This is a real open design question (recommended: apply the damage modifier directly from `cur_ammo_type`, never cast the toggle ability — see README open question 1).
- New-item-id cooked-data injection is architecturally sound (no static id-range check in the element-fetch path) but **only proven in production for existing ids** (#405/Slappack). Plan now runs a spike (push one new item, UAT live) before batch-authoring the rest; if it fails, only two seed files need repointing (contained by the `ammo_item_types` indirection).
- `/gmgiveammo`'s `Event_NetOut_GiveAmmo` is a real, registered, client-emittable event, but **no `.def` entry exists anywhere for a `GiveAmmo` cell/base method** — no recovered server receiver, byte layout unknown. GM tooling packet builds `.gmgiveammo`/`.gmsetinfiniteammo` as `.`-console commands instead of native-opcode handlers.

See also [[project_pr520_bandolier_ammo_fix]] for the earlier bandolier TOCTOU precedent this campaign's `AmmoReserve::draw` design follows (stack decrement under `FOR UPDATE`, keyed correctly).
