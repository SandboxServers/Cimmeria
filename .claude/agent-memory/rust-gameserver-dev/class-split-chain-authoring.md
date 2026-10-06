---
name: class-split-chain-authoring
description: How to split one content trigger by class/archetype (Class Start v6 CS-04): partition rules, grant-row gating, the cooked per-class Cellblock crate dialogs, and the CRLF chain seed
metadata:
  type: project
---

Learned on Class Start v6 CS-04 (2026-10-05), splitting the Cellblock Guard
search (chains 1005/1010) and the mission 687 crate (chains 1098, 1099,
1192-1195) by archetype.

- **A chain's conditions only AND.** A set such as "Humans and Loyalist
  Jaffa" (1-4, 8) cannot be one chain's positive gate. Write it as the
  complement of a contiguous range (`neq 5`, `neq 6`, `neq 7`) with the
  sibling chain on the range (`gt 4`, `lt 8`), or one `eq` chain per class
  plus a fallback chain carrying every `neq`.
- **Always author the fallback.** A missing archetype reads -1. If no chain
  of a mission-critical split matches, the press is silent and the mission
  dead-ends (chain 1191 only answers presses outside step 2354). The guard
  is a test that walks archetypes -1..10 and `None` and asserts exactly one
  grant or one window.
- **Compare the whole resolved action list against `build_engine`**, not one
  chain loaded alone: an overlapping gate (two windows, two pistols) and a
  chain the loader refused (unknown ability or tutorial id) both show up
  only there.
- **`grant_ability` rows carry their own `archetypes`** even when the chain
  is gated, and `tutorial` grants may carry one too. The cell and the base
  check it against the real archetype, so a trigger that lost its param
  cannot teach the wrong class.
- **Sort order:** append new actions past the highest existing `sort_order`
  (review rule). On 1005 that puts the grant and the tutorial after
  `advance_step`; only pistol → abilities → tutorial order matters.
- **Dialogs 2517 / 4408 / 4409** are cooked, button-less crate dialogs
  (Soldier "heavy weapon and body armor", Scientist "deployment belt",
  Archaeologist "Asgard hologram emitter") that no recovered script wired.
  The v6 item sets match their text. GC2 in castle-cellblock-rebuild had
  closed them as unused; CS-04 wires them (our authoring, not retail).
- **1984 Staff Swing** is in the Shol'va tree only; for a Loyalist Jaffa
  (8) the signature is a plain grant with no tree node.
- **`castle_cellblock_chains.sql` is CRLF in the index.** Edit it in binary
  mode and write CRLF back, or the diff becomes the whole file. Same for
  `docs/content/mission-chains.md`, `association-map.md`,
  `archetype-content-map.md` and `docs/gameplay/loot-system.md`.
- **New `loot.sql` rows shift the ammo rows' ids.** `ammo_loot.sql` takes
  its `loot_id`s from the sequence after `loot.sql`'s `setval`, so bump the
  `setval` with the new rows; nothing may name an ammo loot row by id.

**Why:** each of these cost a look-up or would have shipped a dead crate.
**How to apply:** when a packet splits an existing chain by class or adds
class loot tables (CS-05's M1569 is the same shape).
