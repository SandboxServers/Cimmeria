---
name: project-enemy-combat-runtime-blockers
description: "Enemy-combat work (2026-09-26): an external 'live combat v3' handoff was reviewed, not imported; #819/#822 landed; four runtime blockers remain (MITIGATION 0/0, forced DT_PHYSICAL, EF_DONT_USE_QR value, DoT killing blow without entity_death)"
metadata:
  type: project
---

On 2026-09-26 an external handoff (`CLAUDE_ENEMY_LIVE_COMBAT_HANDOFF_v3_ALL_WORLDS_AUDITED.md`) proposed a layer of NPC combat profiles: rank HP multipliers, armor profiles, presentation overrides, boss kits for Romney (169) and Muelbach (170), and per-world encounter recipes. It was reviewed against the repo and **not imported**. Most of its facts held; the Castle bosses exist only as level-1 guard clones with no ability set.

Landed the same day:

- **#819** removed the temporary 2x player damage multiplier (it was a testing aid).
- **#822** restored 220 `abilities.event_set_id` links. Finding: `docs/reverse-engineering/findings/ability-animation-links.md`. Still open there: abilities 1176, 1829 and 1640, and 1138 Cloak (belongs on effect 1290).

**Runtime blockers found in that review (as of 2026-09-26; re-verify before acting):**

1. The `MITIGATION` stat is clamped to 0/0, so armor is inert.
2. Damage is forced to `DT_PHYSICAL` instead of the authored damage type.
3. `EF_DONT_USE_QR` is defined as 32 but should be 16, and nothing reads it.
4. A damage-over-time killing blow fires no `entity_death` (also recorded in `docs/analysis/castle-rebuild/worknotes/m702-704.md`).

There is also no rank/profile layer and no conditional boss selector.

**Owner decisions still needed:** rank HP multipliers versus the v1.2 handoff pack's "no guessed retail stat scaling" policy, and how armor profiles stack with cover (decision D-NA15a in `docs/analysis/npc-ai-restoration/`).

After the services split (#825), the fix sites are in `crates/cell-combat` (damage apply, abilities) and `crates/entity` (stat caps, EF constants). Label any reconstructed data as reconstruction: the seed is Project Giza's client-derived approximation, not 2009 data (`docs/agents/rules-and-gotchas.md`).
