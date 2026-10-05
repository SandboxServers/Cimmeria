---
name: reference-world-1300-not-gm-enforced
description: DebugArea world 1300 is "GM-only travel" by absence of a stargate row only; no login/respawn/space-entry GM check (verified 2026-10-04 on main)
metadata:
  type: reference
---

> **Update (PR #1230, 2026-10-04):** now enforced. `cimmeria_base_session::base::world_entry::gm_only_worlds` redirects a non-GM (access level below 2) to the faction start at login (`world_entry_db`) and on every cross-world transfer (`gate_travel`), with a WARN `gm_only_world_refused` and a chat line on arrival. Same-world moves need no check. Keep the advice below for rigs anyway: defence in depth.

World 1300 (`DebugArea`, on the Ihpet_Crater_Light map, `db/resources/Worlds/Seed/worlds.sql`) is described as "GM-only travel". That rests only on 1300 having no stargate row. As of 2026-10-04, `main` has no base or cell code that checks `access_level` when a player logs in, respawns into, or enters space 1300. A GM's `.summon` (which always brings the player to the caller) puts a non-GM there, and a relog keeps them there.

**How to apply:** when reviewing anything placed in world 1300 (DA-02..04 plaza, vendors, granters, dummies), treat non-GMs as able to reach it. A privileged NPC must check the GM bit itself, as the `gm_ability_bulk` action does through `CellEntity::access_level`. Economic rigs there must not create value. See [[exploit-vendor-buy-sell-arbitrage]].

Also from the PR #1230 review: `classify` in `use_ability/support_shot.rs` is the single ally/hostile authority shared by beneficial casts, support shots and `target_gate`. The `TrainingDummy` mark (template flag or `.dummy`) is the only non-player way to become `Ally`.
