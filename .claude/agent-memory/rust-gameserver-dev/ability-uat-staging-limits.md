---
name: ability-uat-staging-limits
description: What the ability UAT spec (abilities.toml, AB-R0) cannot stage from the seed, which plan ids were effect ids, and why multi-press rows are lettered
metadata:
  type: project
---

Facts found writing `docs/guides/uat-specs/abilities.toml` (AB-R0, 2026-10-04).

- **No seeded NPC casts with a warmup.** NPC ability sets 1-5 and 350-352 hold
  no ability with `warmup > 0`; 353 (Lo'taur pet) has only 2 s heals. The lab
  dummy (`.dummy`) gets no AI turn at all. So "interrupt a mob's warmup" rows
  (AB-U18's warmup half, AB-U20) need `.dummy caster <abilityId>` (#1188),
  which casts through the real launch at its owner; AB-U20 uses 1354
  Disabling Shot (4 s warmup). `/gmsetmobabilityset` only takes seeded sets.
- **No seeded Mental effect has a held mechanic**, so Clear: Mind (2099)
  removes 0. AB-U22 stages Absolution (2865, effect 4169 `Health:2`) against
  the two Health debuffs (4335, 4333) a `.dummy caster 1354` hit leaves.
- **Plan ids that were effect ids:** 1462 (Snare Shot is ability 717), 4306
  (Personal Shield is 1013), 2827 (Clear: Mind is 2099). Check `abilities.sql`
  `effect_ids` before trusting a number in a plan table.
- **One graded press per row.** A 30 s cooldown needs `@cooldowns_reset` (G)
  between presses, and any G step costs the row its N1 grade, so multi-target
  rows are lettered (AB-U1a-d). `@use_ability { place = true }` also presses,
  so setup places, then resets the cooldown and clears effects; client_event
  clauses use `since = "press"`.
- A live hostile for rows: `.spawn 24` (NID Guard, faction 10, set 3) replies
  `spawned npc <id> (template 24)`; remove it with `/gmdespawn <id>`.

**Why:** these cost a research round each and the plan table had them wrong.
**How to apply:** before adding or unblocking an ability UAT row, check the
seed for a staging path first; see [[lab-uat-runner-in-process-tool-calls]].
