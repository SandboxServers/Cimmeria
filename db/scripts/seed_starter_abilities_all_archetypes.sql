-- =============================================================================
-- Seed starter abilities for the 14 char_def_ids that were shipping with EMPTY
-- ability lists. Before this migration only char_def_ids 1-4 (Soldier M/F,
-- Commando M/F) had starter abilities; everyone else (Scientist, Archeologist,
-- Asgard, Goa'uld, Sholva, Jaffa - both alignments, both genders) spawned with
-- `sgw_player.abilities = '{}'` and could do nothing combat-relevant.
--
-- Baseline loadout matches Soldier/Commando: Pistol Shot (592), Strike (594),
-- Heal Focus (597). This is a pragmatic minimum so every archetype is playable
-- end-to-end. Per-class flavorful loadouts (e.g., Goa'uld hand-device, Jaffa
-- staff weapon) are tracked in #419 Phase 7 (content authoring).
--
-- Idempotent: `ON CONFLICT DO NOTHING` so re-running the migration is safe
-- even after a content pass adds more abilities to these char_defs.
--
-- Reference: issue #419, docs/protocol/item-sequence-lookup.md
-- =============================================================================

INSERT INTO resources.char_creation_abilities (char_def_id, ability_id) VALUES
    -- char_def 5  ARCHETYPE_Archeologist  ALIGNMENT_Praxis  Male
    (5, 592), (5, 594), (5, 597),
    -- char_def 6  ARCHETYPE_Archeologist  ALIGNMENT_SGU     Male
    (6, 592), (6, 594), (6, 597),
    -- char_def 7  ARCHETYPE_Jaffa         ALIGNMENT_Praxis  Male
    (7, 592), (7, 594), (7, 597),
    -- char_def 8  ARCHETYPE_Sholva        ALIGNMENT_SGU     Male
    (8, 592), (8, 594), (8, 597),
    -- char_def 9  ARCHETYPE_Asgard        ALIGNMENT_SGU     Male
    (9, 592), (9, 594), (9, 597),
    -- char_def 10 ARCHETYPE_Goauld        ALIGNMENT_Praxis  Male
    (10, 592), (10, 594), (10, 597),
    -- char_def 15 ARCHETYPE_Archeologist  ALIGNMENT_Praxis  Female
    (15, 592), (15, 594), (15, 597),
    -- char_def 16 ARCHETYPE_Archeologist  ALIGNMENT_SGU     Female
    (16, 592), (16, 594), (16, 597),
    -- char_def 17 ARCHETYPE_Jaffa         ALIGNMENT_Praxis  Female
    (17, 592), (17, 594), (17, 597),
    -- char_def 18 ARCHETYPE_Sholva        ALIGNMENT_SGU     Female
    (18, 592), (18, 594), (18, 597),
    -- char_def 19 ARCHETYPE_Goauld        ALIGNMENT_Praxis  Female
    (19, 592), (19, 594), (19, 597),
    -- char_def 20 ARCHETYPE_Scientist     ALIGNMENT_Praxis  Male
    (20, 592), (20, 594), (20, 597),
    -- char_def 21 ARCHETYPE_Scientist     ALIGNMENT_SGU     Male
    (21, 592), (21, 594), (21, 597),
    -- char_def 22 ARCHETYPE_Scientist     ALIGNMENT_Praxis  Female
    (22, 592), (22, 594), (22, 597),
    -- char_def 23 ARCHETYPE_Scientist     ALIGNMENT_SGU     Female
    (23, 592), (23, 594), (23, 597)
ON CONFLICT DO NOTHING;
