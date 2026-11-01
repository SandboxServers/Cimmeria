---
name: project-enemy-combat-runtime-blockers
description: "Enemy-combat work (2026-09-26): an external 'live combat v3' handoff was reviewed, not imported; #819/#822 landed; two runtime blockers still open (MITIGATION 0/0, forced DT_PHYSICAL)"
metadata:
  type: project
---

On 2026-09-26 an external handoff (`CLAUDE_ENEMY_LIVE_COMBAT_HANDOFF_v3_ALL_WORLDS_AUDITED.md`) proposed a layer of NPC combat profiles: rank HP multipliers, armor profiles, presentation overrides, boss kits for Romney (169) and Muelbach (170), and per-world encounter recipes. It was reviewed against the repo and **not imported**. Most of its facts held; the Castle bosses exist only as level-1 guard clones with no ability set.

Landed the same day:

- **#819** removed the temporary 2x player damage multiplier (it was a testing aid).
- **#822** restored 220 `abilities.event_set_id` links. Finding: `docs/reverse-engineering/findings/ability-animation-links.md`. Still open there: abilities 1176, 1829 and 1640, and 1138 Cloak (belongs on effect 1290).

**Runtime blockers found in that review, still open at 2026-11-01:**

1. The `MITIGATION` stat is clamped to 0/0 (`s!(MITIGATION, 0, 0, 0)` in `crates/entity/src/stats/stat_list.rs`), so armor is inert.
2. Damage is `DT_PHYSICAL` unless an ammo shot supplies a type (`damage_apply/mod.rs` in `crates/cell-combat`), not the authored damage type.

Two more from that review are fixed: `EF_DONT_USE_QR` is now 16 (`crates/entity/src/abilities/defs.rs`), and a damage-over-time killing blow now goes through `dot_kill_credit` in `crates/cell-combat/src/cell/effects/pulsing/pulse.rs`.

There is also no rank/profile layer and no conditional boss selector.

**Owner decisions still needed:** rank HP multipliers versus the v1.2 handoff pack's "no guessed retail stat scaling" policy, and how armor profiles stack with cover (decision D-NA15a in `docs/analysis/npc-ai-restoration/`).

After the services split (#825), the fix sites are in `crates/cell-combat` (damage apply, abilities) and `crates/entity` (stat caps, EF constants). Label any reconstructed data as reconstruction: the seed is Project Giza's client-derived approximation, not 2009 data (`docs/agents/rules-and-gotchas.md`).

INVALIDATED WHEN: blocker 1 or 2 above is fixed or the NPC combat profile layer lands
checked: 2026-11-01
