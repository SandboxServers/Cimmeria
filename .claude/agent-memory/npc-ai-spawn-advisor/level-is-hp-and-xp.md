---
name: level-is-hp-and-xp
description: entity_templates.level only drives NPC max HP (200+50*level), kill XP (10*level) and the client onLevelUpdate; level 50 is the seed's "unknown level" sentinel, not a boss marker.
metadata:
  type: project
---

# What `entity_templates.level` actually does (measured 2026-09-17)

Three consumers, nothing else:

1. **Max HP** — `space_manager/spawn.rs:185`: `hp = 200 + level * 50`.
   NULL level → `unwrap_or(1)` → 250 HP. Level 50 → 2,700 HP.
2. **Kill XP** — `abilities/loot_drop.rs:118`: `kill_xp = 10 * level`.
   Flat, no level-difference scaling, no cap.
3. **Client `onLevelUpdate`** — `mercury/aoi/create.rs`, sent only when
   `class_id != 0x00`, i.e. for `class = 'being'` and `'mob'` but NOT
   `'spawnable'`.

Nothing in the AI tick, threat, aggro, leash, range or damage path reads
`level`. Level is **template-only** — `spawnlist` has no level column, so
it cannot be overridden per spawn.

## Level 50 is a sentinel, not a tier

Every named NPC the SGW importer couldn't level got 50: Ba'al 42, Anat 43,
Lethander 46, CaptCoppleman 48, Nerus 53, Moh'Katan 54, Ra 41, Sam Carter 33,
**and the ordinary Praxis Jaffa Guard 160 / Lieutenant 159**. Counter-examples
in the same seed: General Hammond 29 = level 1, Teal'c 30 = level 1.

So "level 50 = named story NPC" is a Harset-local convention, not a global
one. Cost of choosing it for a killable mob: 2,700 HP and 500 XP per kill.
For a talk-only NPC it is inert (never damaged, never killed).

Related: [[faction-10-gates-everything]]
