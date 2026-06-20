---
name: pvp-duel-readiness
description: Where PvP/duels collide with the PvE-only combat pipeline — the gate, which death-tail stages misfire for a player victim, and the BSF_InCombat stuck-bit hazard
metadata:
  type: project
---

# PvP / duel readiness of the combat pipeline (as of 2026-06)

Combat is **hard PvE-only by an explicit gate**, not by accident. PvP doesn't leak through anywhere. Map of where a duel system collides:

**The gate (player→player blocked):** `cell/abilities/use_ability/handle.rs:223-224`
`if entity.is_player && (target.is_player || target.faction != combat::HOSTILE_FACTION) { return false; }`
- Flat `HOSTILE_FACTION` sentinel, not per-pair hostility. Author flagged this as THE PvP seam (handle.rs:215-222).
- Scoped to player *attackers* only — NPC→player already works end-to-end.
- Parallel gates for AoE (`abilities/dispatch.rs`) + cone (`abilities/cone_aoe.rs`) — MUST branch all three or AoE/cone leaks onto bystanders.

**Death tail for a player victim — what's ALREADY player-inert (no branch needed):**
- loot (`death.rs:213` gated `!target_is_player`), kill-credit (`kill_credit.rs:56` `!t.is_player`), XP (`damage_apply/mod.rs:369`), corpse threat fanout (`death.rs:127`).

**Death tail — what MISFIRES for a duel (the 5 "Yes" rows):**
- `BSF_DEAD`/`BSF_MOVEMENT_LOCK` set: `damage_apply/mod.rs:192-195`
- dead-state bit broadcast: `death.rs:230-237`
- Entity_Death anim (event 5001, event set 1025 for players too): `damage_apply/mod.rs:328-365`
- `onBeginAidWait` Defeat Window: `damage_apply/mod.rs:399-443`
- `handle_respawn` (teleport/heal/cooldown-wipe), triggered off Defeat Window: `respawn.rs:76,117`

Cleanest fix: **clamp HEALTH to 1 + trigger duel-end** before the `target_died` block (`damage_apply/mod.rs:177`) so none of the 5 fire — beats threading "is duel?" through all of them.

**BSF_InCombat (bit 3) for duelists:** driven EXCLUSIVELY by `threatened_mobs` set (`threat/player_combat.rs:46-48` set / `99-101` clear). `generate_threat` early-returns on player targets (`threat/aggro.rs:59-60`), so player→player hits set NO threat and NO BSF_InCombat. A duel that raw-`|=` the bit without a symmetric clear = stuck-bit (drawn weapon/combat cursor forever). Same failure class as the respawn `threatened_mobs.clear()` fix (`respawn.rs:170-190`). Use a parallel set/clear helper or route opponent through enter/exit_player_combat.

**Other traps:** 2x player-damage temp hack (`damage_apply/mod.rs:133-139`) doubles PvP damage — exclude duels. Don't remove the `generate_threat` player-target guard (keeps PvP out of NPC threat lists).

Intended SGW duel semantics (lethal? yield? arena-bounded?) = verify against `docs/reverse-engineering/findings/` + social-systems duel chapter; pcap > my clamp recommendation.
