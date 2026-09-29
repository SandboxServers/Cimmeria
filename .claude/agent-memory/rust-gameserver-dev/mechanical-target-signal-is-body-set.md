---
name: mechanical-target-signal-is-body-set
description: The server has no mechanical/robot flag on entities, templates or factions; the only signal is CellEntity::body_set (entity_templates.body_set). EMP split evidence lives in ability 2864's effects.
metadata:
  type: project
---

No "mechanical" flag exists anywhere the server loads (checked 2026-09-28, AM-09): `EEntityFlags`
(ref_MOB_FLAGS), `entity_templates` columns and `EFaction` have none, and the cooked
"Mechanical Type Check" effects (4203/4204/4217) carry no NVPs or script. The usable signal is
`CellEntity::body_set`, set from `entity_templates.body_set` at spawn (`space_manager/spawn.rs`).
AM-09 keeps the list in `cell-world/src/cell/effects/ammo_emp.rs::MECHANICAL_BODY_SETS`
(PRU `MOB_CA_DroneTank.`, `MOB_Goauld_Drone.`, `MOB_AncientDrone.`, `MOB_BattleWalker.`,
`WP-Human.BS_Deployable`). Straegis deliberately excluded (lore: "lack biology" and "signs of machine").

**Why:** anything "vs mechanical" (EMP darts AM-11b, Reprogram/Reduce Processes abilities) needs the
same rule; reuse `ammo_emp::is_mechanical` rather than inventing a second list.

**How to apply:** EMP semantics per the EMP Grenade (2864): living target loses Focus only
(effect 4202 -100F/-0H), machine loses Health only (4200 -0F/-225H) plus disorient (4201).
The real client rule is still unfound; if someone finds it in the binary, replace the list.
Related: [[ability-event-sets-are-server-only]].
