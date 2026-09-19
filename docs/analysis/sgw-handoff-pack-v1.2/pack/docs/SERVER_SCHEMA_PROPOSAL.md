# Server Data Model Proposal

This is a mapping target, not a mandatory schema. Prefer adapting the existing server model.

## Character progression

### character_skill_state
- character_id
- archetype
- available_skill_points
- spent_skill_points
- version

### character_learned_ability
- character_id
- ability_id
- learned_at_level
- source (`starter`, `trainer`, `mission`, `system`)
- learned_at

### skill_tree_node
- archetype
- branch
- node_order
- ability_id
- unlock_level
- required_branch_points
- skill_point_cost
- primary_prereq_ability_id
- is_branch_root
- is_capstone
- evidence_status
- project_version

### skill_tree_node_prereq
- archetype
- branch
- ability_id
- prereq_ability_id

## Runtime ability definition

Keep original Ability IDs as identities. Runtime data should reference:
- cooldown/warmup
- range/targeting
- weapon family requirement
- ammo/resource cost
- linked effect IDs
- status mechanics
- cover interaction

Do not duplicate effect payloads into the trainer tree unless the current server architecture requires denormalization.

## Purchase validation

A trainer purchase should validate, server-side:

1. Character archetype matches the tree.
2. Node exists and is enabled.
3. Character does not already know the ability.
4. Character level >= unlock level.
5. Required branch points satisfied.
6. Primary and additional prerequisite abilities learned.
7. Skill points >= cost.
8. Any faction/trainer access condition passes.
9. Transaction deducts points and persists learned ability atomically.

The client UI being gray/active is not security. The server must enforce the same gates.
