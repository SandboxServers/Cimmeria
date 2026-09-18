---
name: converse-minigame-evidence
description: What the abilities seed reveals about the Converse social minigame's real rules (Trump mechanic, engagement state) and why it is unimplementable without SWF RE
metadata:
  type: project
---

# Converse minigame — evidence from the abilities seed

Server side Converse is a **placeholder auto-win** (see
[[game-implementation-status]]). But the abilities seed preserves real design
data about what the game actually was. Do NOT invent rules from this — it is a
lead for SWF disassembly, not a spec.

## The "Trump" mechanic

`db/resources/Abilities/Seed/abilities.sql` — four passive abilities whose
entire description is a Converse modifier:

| id | name | description |
|----|------|-------------|
| 778 | Conversation: Mediator | `+5% for Trump in Converse` (line 1782) |
| 779 | Conversation: Negotiator | `+10% for Trump in Converse` (line 4831) |
| 792 | Conversation: Diplomat | `+15% for Trump in Converse` (line 4847) |
| 793 | Conversation: Ambassador | `+25% for Trump in Converse` (line 328) |

"Trump" is a card/trick term — strong inference that Converse was a card-play
or suit-matching game where a passive raised your trump chance. These four are
`passive_yn = true`, `ABILITY_TYPE_Undefined`, `training_cost = 1`, so they were
trainer-purchasable ranks of one social-skill line.

The `abilities_mask` / `abilityBitfield` session field
(`minigame/session.rs:22`, emitted at `games/livewire/setup.rs:387`) is the
transport for exactly this class of modifier — the SWF reads it and applies the
bonus client-side. It is currently hardcoded to `0` at
`base/world_entry/cell_dispatch/minigame.rs:47`.

## Engagement state

- Ability **1327 "Converse Minigame"** (line 1994): *"This ability puts a
  humanoid into the engagement state."* `min_range 100, max_range 2000`,
  `effect_ids {1643,1642,1641,1640}`.
- Ability **2089 "Conversation: Humanoid: Basic"** (line 2740):
  `ABILITY_TYPE_Debuff`, `max_range 800`, `warmup 0.5`, description
  *"Ranged Single Target Conversation: Humanoid / Engagement State: 20 seconds"*,
  `effect_ids {2813,2812,2811}`.

So the flow was: cast a Conversation ability at an NPC → NPC enters a 20 s
"engagement state" debuff → Converse minigame window opens. That is an
ability-initiated minigame, **not** an `interact_tag` one — a different launch
path from every Livewire precedent in the repo.

There are also 2082/2083/2087/2088/2658 duplicates of
"Conversation: Humanoid: Basic" and a `MS020_080905_ConversationGame:*` family
(2394-2398) — design iteration leftovers.

## Content evidence it was used

`db/resources/Missions/Seed/mission_steps.sql:5327` — mission 582 step 2307,
`'Conversation Minigame With Marsh'`. Proof the Converse game reached content
authoring, not just design docs.

## Minigame-adjacent utility abilities

- 1661 / 2837 **"Minigame Win Container"** — *"Does 100% damage to container"*.
  This is how "search the containers" steps resolved: winning the minigame
  destroyed the container entity.
- 585 **"Minigame Despawn Mob"**, 586 **"Minigame Consume Item"** — the
  cleanup hooks fired from a minigame result.
- 1594 / 1753 **"* - Minigame Success Dialog"** — post-win VO.

## Verdict

Real Converse rules are **blocked** on Flash SWF disassembly
(`Flash/Converse.upk`, `Flash/ConverseBasicHumanoid.upk`). Section 1 of any
bible chapter here is "N/A — logic lives in the client SWF, not SGW.exe".
