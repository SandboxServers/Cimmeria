# v1.2 Additional Combat Formula Tests

- Full five-piece matching 30% armor set resolves to 30% mitigation.
- Torso-only 30% piece contributes 10.5% mitigation.
- +100 penetration removes 10 percentage points of matching armor.
- +15% explicit resistance multiplies post-armor damage by 0.85.
- Focus 100/50/0% produces Accuracy penalty 0/-100/-200 and H-component chance 10/50/90%.
- Status resist +100/+250 changes 85% base application to 75%/60%.
- AoE exposure None/Low/Medium/High/Occluded = 100/85/70/50/0%.
- TC1/25/55 Normal multiplier = 1.000/1.300/1.675.
- TC55 Fantastic = 1.8425.
- AP vs 30% armor = 0.765 direct multiplier; Hollow Point vs 0 armor = 1.15.

# QA Tests

## Trainer / progression
1. A new character sees exactly three branches for its archetype.
2. A node above character level is unavailable.
3. A node without its prerequisite is unavailable.
4. A node without enough branch points is unavailable.
5. Buying a node deducts exactly one project-v1 skill point.
6. Buying the same node twice is rejected server-side.
7. Learned abilities persist through logout/login.
8. Capstone requires Level 50 and the configured branch-point gate.
9. Raw `Training Cost=0` nodes still show a server log warning/metadata flag when purchased under project-v1 rules.
10. A character cannot buy nodes belonging to another archetype.

## Weapon / ability
1. Ability requiring a weapon family fails when the wrong weapon is active.
2. Correct active weapon enables the ability.
3. Auto-attack starts on right-click and stops on invalid target/death/range conditions.
4. Ammo is consumed from each ability according to configured cost.
5. Reload restores ammo according to the weapon/ammo model.
6. Ammo-mode toggles change the configured damage/ammo mode without creating duplicate permanent states.

## Cover
1. None/Low/Medium/High cover can be identified directionally.
2. Cover only protects from the appropriate attack direction.
3. Crouch state is independent of cover.
4. Cover Defense and Cover Penetration modifiers are logged separately.
5. High/Medium/Low UI state can be mapped to green/yellow/orange as expected by the client.
6. Unknown numeric formula values come from config, not hard-coded constants.

## Focus / Health
1. Focus and Health are distinct pools.
2. Effect payloads can damage Focus and Health independently.
3. Focus depletion triggers the configured low-Focus behavior.
4. Tooltip↔effect conflicts are visible in diagnostic logs.

## Scientist Robotics
1. Player can summon allowed turret type.
2. Pet/internal turret attacks are not purchasable player skills.
3. Repair/upgrade commands target a valid owned turret.
4. Dual-turret capstone obeys its configured pet-count rule.

## Goa'uld Servant Lord
1. Summon abilities create only permitted minion templates.
2. Pet-internal abilities are not player hotbar purchases unless explicitly marked.
3. Summoned entities are cleaned up on logout/world transfer as configured.

## World starts
1. SGU Human starts in Earth SGC project target.
2. Free Jaffa starts on Dakara project target.
3. Asgard starts on Pertho project target.
4. SGC_W1 placeholder starts are not reintroduced for Jaffa/Asgard.

## Castle CellBlock regression
1. Player progression remains inside CellBlock through Mission 688.
2. Drones → Ambernol → Ring Transport minigame sequence is preserved.
3. Armory is the CellBlock end.
4. Transition after Armory goes to Castle main.
5. Mission 701+ content is not spawned/started inside CellBlock.
