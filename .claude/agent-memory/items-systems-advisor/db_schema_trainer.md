---
name: db-schema-trainer
description: DB tables for trainer feature — trainer_abilities, trainer_ability_lists, archetype_ability_tree — with key join pattern
metadata:
  type: project
---

All in schema `resources`.

## trainer_ability_lists
```sql
list_id  integer PK (serial)
description  varchar(200)
```

## trainer_abilities
```sql
list_id    integer    -- FK → trainer_ability_lists.list_id
archetype  EArchetype -- enum
ability_id integer
```

Key: `(list_id, archetype)` → `Vec<ability_id>`. Loaded into `SpaceManager.trainer_abilities: HashMap<(i32,i32), Vec<i32>>`.

## archetype_ability_tree
```sql
archetype              EArchetype
ability_index          integer
ability_id             integer
tree_index             integer  -- 0/1/2 (three trees per archetype)
level                  integer  -- player level required
prerequisite_abilities integer[]  -- must all be in player's known set
```

`level` here is the **gate for training**, not a column on `abilities`. The `abilities.training_cost` column is NOT the level gate.

## entity_templates (relevant column)
```sql
trainer_ability_list_id  integer  -- NULL for non-trainer NPCs
```

Loaded into `SpaceManager.template_trainer_lists: HashMap<template_id, list_id>`.

INVALIDATED WHEN: the trainer tables in db/resources/Abilities or archetype_ability_tree gain or lose columns (it has since gained branch columns not listed here)
checked: 2026-11-01
