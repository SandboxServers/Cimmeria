---
name: project_class_start_v5_handoff
description: 2026-10-05 "SGW FINAL v1 class start/gear/abilities" handoff v5 reviewed vs origin/main b28813cc; not imported; reverses D-SA1 and the M687 two-way split; mission names wrong
metadata:
  type: project
---

External handoff `SGW_FINAL_V1_CLASS_START_GEAR_ABILITIES_IMPLEMENTATION_HANDOFF_v5.zip` (12 files, 3 CSVs; baseline b28813cc) was reviewed on 2026-10-05 and **not imported**. Nothing in it is retail evidence: every row is labelled `FINAL_V1_S2C` / PROJECT_FINAL. No earlier v2/v3/v4 character-start handoff exists in the repo.

How it compares with origin/main:
- **Matches the repo:** the universal kit really is 592/594/597/1218/1646 plus pistol 55 for all 23 char_defs (`char_creation_abilities.sql`, `starter_kit.rs`). Respec already removes only `trained_abilities` (D-AT03). Free-granted nodes already satisfy prerequisites without adding spend (`gates/spend.rs`). Trainer greys known nodes. Weapon AAs are transient (`swap_weapon_granted_abilities`). Item ids and AA pairs all match the item seed. Note (2026-10-05 v6 preflight): the Rust server never reads `abilities.item_monikers`, so weapon requirements are unenforced; 598 needs ITEM_Automatic_Weapon, which SGHC 6 (21) has and SK37 LMG (3260) lacks.
- **Reverses owner decisions:** "no spawn pistol" reverses D-SA1 (2026-10-04, `docs/analysis/debug-area/README.md`). The M687 five-way class split replaces the owner's two-way loot-window decision of 2026-09-28. The handoff's stealth set 3345.. is the invisible Aramid tier, not the shipped Covert set 3347...
- **Wrong about the repo:** M641 is "Preparation" and already grants 21. M622 already grants 55. The SGC pistol is M1559, not M1571. M1617 is "Orientation" and M1621 is "Heimdall"; neither has chains. The "two spare mags" rule conflicts with D-AM02, which makes default reloads free.
- **Needs infrastructure first:** a non-GM `GrantAbility` content action (only `GrantItem`/GM paths exist). A start-profile table (the insert hard-codes level 1; the v5/v6 "Free Jaffa L3" was corrected to level 1 by lomiada on 2026-10-05, and the L3 came from the Dakara missions' seeded levels, not a start rule). An Asgard starter-starship world (none exists). Dakara/Pertho start registry and spawns (phase 0 BLOCKED). The GM "Ability reset" (`gm_ability_bulk.rs`) restores `char_creation_abilities` and would wipe signature grants.
- **Fixed by the handoff:** dropping 1646 from the universal kit resolves the D-AT09 collision. 1646, 1647 and 1572 are not tree roots, so granting them free skips their prerequisites.

**How to apply:** if the owner adopts parts of it, the conflict-free packet is: GrantAbility action, an archetype-specific starter table, and 592 tutorial-grant plumbing. Starts, rewards and D-SA1 reversal need owner sign-off. Related: [[project_pass21_implementation_package]].
