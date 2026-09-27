//! Live-DB guards for the pet seed (pets campaign PT-S,
//! `docs/analysis/pets/`): `resources.pet_summons`, the pet templates
//! 350-359, the Straegis pet's ability set and the summon VFX on ability
//! 2826.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a summon row naming a template that is not a pet (a mob with a loot
//!   table, or a template some other campaign owns);
//! * a pet template placed in `spawnlist`, which would stand an ownerless
//!   pet in the world;
//! * Summon Straegis losing its event set, which leaves the 6 s summon with
//!   no cast effect;
//! * the Straegis pet drifting from the Straegis Fighter body (78), losing
//!   its readable name, or picking up the death-burst Explode as an attack.
//!
//! Each was proven to fail with the PT-S seed rows removed (worknote
//! `docs/analysis/pets/worknotes/pt-s.md`).
mod live_db {
    use std::collections::BTreeSet;

    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `ENTITYFLAG_Pet` (`entities/defs/enumerations.xml`).
    const ENTITYFLAG_PET: i64 = 1024;
    /// `ENTITYFLAG_NoPetLeveling`.
    const ENTITYFLAG_NO_PET_LEVELING: i64 = 8;
    /// The pet templates. The campaign owns 350-369; PT-07 split off
    /// 360-369 for its placed NPCs (the debug-hub pet trainer, 360), which
    /// are ordinary class-'mob' spawns, so the pet guards cover 350-359.
    const PET_TEMPLATES: std::ops::RangeInclusive<i32> = 350..=359;
    /// The faction every hostile NPC carries; anything else is friendly to
    /// players (D-PT06).
    const HOSTILE_FACTION: i32 = 10;

    const SUMMON_STRAEGIS: i32 = 2826;
    const STRAEGIS_PET: i32 = 350;
    const STRAEGIS_FIGHTER: i32 = 78;
    /// `DN_Mb_Ms_Agnos_Summoned_Straegis_Fighter_Force_43`, "Summoned
    /// Straegis Fighter". The pet moniker 28894 has empty text.
    const STRAEGIS_PET_NAME: i32 = 27377;
    /// Ability set 350: 221 Energy Shock (primary) + 1156 Disengage.
    const STRAEGIS_PET_ABILITIES: [i32; 2] = [221, 1156];
    /// 1240 Straegis Explode: cooldown 0, death-burst event set 1507.
    const STRAEGIS_EXPLODE: i32 = 1240;

    /// Event set 1121 "Goauld summon source" and its sequences.
    const GOAULD_SUMMON_SOURCE: i32 = 1121;
    const SUMMON_CAST_PFX: i32 = 2292;
    const SUMMON_INTERRUPT_SFX: i32 = 2904;
    /// Event set 1122 "Goauld summon target", Effect_Init (2000) → 2293.
    const GOAULD_SUMMON_TARGET: i32 = 1122;
    const EFFECT_INIT: i32 = 2000;
    const SUMMON_TARGET_PFX: i32 = 2293;

    /// Every `pet_summons` row names a pet template: in 350-359, class
    /// `pet`, `ENTITYFLAG_Pet` set, no loot table. Read through the startup
    /// loaders, so a broken loader query fails here too.
    #[tokio::test]
    async fn every_pet_summon_points_at_a_pet_template() {
        let pool = require_db_or_skip!();
        let summons = load_pet_summons(&pool).await.expect("pet summons load");
        let templates = load_spawn_templates(&pool).await.expect("templates load");

        assert_eq!(
            summons.pet_summon_for(SUMMON_STRAEGIS),
            Some(PetSummon {
                ability_id: SUMMON_STRAEGIS,
                template_id: STRAEGIS_PET,
                max_active: 1,
            }),
            "2826 Summon Straegis must summon template 350 with one active pet"
        );

        let rows: Vec<(i32, i32, i32)> =
            sqlx::query_as("SELECT ability_id, template_id, max_active FROM resources.pet_summons")
                .fetch_all(&pool)
                .await
                .expect("pet_summons query");
        assert_eq!(rows.len(), summons.len(), "the loader keeps every row");

        let mut bad = Vec::new();
        for (ability_id, template_id, max_active) in rows {
            if !PET_TEMPLATES.contains(&template_id) {
                bad.push(format!(
                    "{ability_id}: template {template_id} is outside 350-359"
                ));
            }
            if max_active < 1 {
                bad.push(format!("{ability_id}: max_active {max_active}"));
            }
            let Some(t) = templates.get(&template_id) else {
                bad.push(format!("{ability_id}: template {template_id} did not load"));
                continue;
            };
            if t.class != "pet" {
                bad.push(format!(
                    "{ability_id}: template {template_id} class {:?}",
                    t.class
                ));
            }
            if t.flags & ENTITYFLAG_PET == 0 {
                bad.push(format!(
                    "{ability_id}: template {template_id} lacks ENTITYFLAG_Pet"
                ));
            }
            if t.loot_table_id.is_some() {
                bad.push(format!(
                    "{ability_id}: template {template_id} has a loot table"
                ));
            }
        }
        assert!(
            bad.is_empty(),
            "pet_summons rows that name a non-pet template:\n{}",
            bad.join("\n")
        );
    }

    /// No pet template is ever placed in `spawnlist`, and every template
    /// that looks like a pet (class `pet` or `ENTITYFLAG_Pet`) is one of
    /// ours. A placed pet would stand in the world with no owner.
    #[tokio::test]
    async fn pet_templates_are_never_placed_in_spawnlist() {
        let pool = require_db_or_skip!();

        let pets: Vec<(i32, String, i64)> = sqlx::query_as(
            "SELECT template_id, class, flags FROM resources.entity_templates \
             WHERE template_id BETWEEN 350 AND 359 OR class = 'pet' OR flags & 1024 <> 0 \
             ORDER BY template_id",
        )
        .fetch_all(&pool)
        .await
        .expect("pet template query");
        let ids: BTreeSet<i32> = pets.iter().map(|(id, _, _)| *id).collect();
        assert!(
            ids.contains(&STRAEGIS_PET),
            "control: the pet template query must see template 350, got {ids:?}"
        );
        for (id, class, flags) in &pets {
            assert!(
                PET_TEMPLATES.contains(id),
                "pet-like template {id} is outside 350-359"
            );
            assert_eq!(
                class, "pet",
                "template {id} in the pet range must have class 'pet'"
            );
            assert_ne!(
                flags & ENTITYFLAG_PET,
                0,
                "template {id} must carry ENTITYFLAG_Pet"
            );
        }

        let placed: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT s.spawn_id, s.template_id FROM resources.spawnlist s \
             JOIN resources.entity_templates t ON t.template_id = s.template_id \
             WHERE t.template_id BETWEEN 350 AND 359 OR t.class = 'pet' OR t.flags & 1024 <> 0",
        )
        .fetch_all(&pool)
        .await
        .expect("spawnlist query");
        assert!(
            placed.is_empty(),
            "pet templates placed in spawnlist (spawn_id, template_id): {placed:?}"
        );
    }

    /// Summon Straegis carries the Goa'uld summon source event set, and the
    /// sequences the cast plays resolve through the startup sequence map.
    /// The target set 1122 is not an ability-level set; the summon code
    /// plays its Effect_Init sequence at the new pet.
    #[tokio::test]
    async fn summon_straegis_carries_the_goauld_summon_event_set() {
        let pool = require_db_or_skip!();
        let defs = load_ability_defs(&pool).await.expect("ability defs load");
        let sequences = load_event_set_sequences(&pool)
            .await
            .expect("sequence map loads");

        let def = defs.get(&SUMMON_STRAEGIS).expect("ability 2826 is seeded");
        assert_eq!(
            def.event_set_id,
            Some(GOAULD_SUMMON_SOURCE),
            "2826 Summon Straegis must carry event set 1121 (Goauld summon source)"
        );
        assert_eq!(
            sequences.get(&(GOAULD_SUMMON_SOURCE, EVENT_ABILITY_END)),
            Some(&SUMMON_CAST_PFX),
            "Ability_End of 1121 is the PFX-GoauldSummon cast effect"
        );
        assert_eq!(
            sequences.get(&(GOAULD_SUMMON_SOURCE, EVENT_ABILITY_INTERRUPT)),
            Some(&SUMMON_INTERRUPT_SFX),
            "Ability_Interrupt of 1121 is the cast-interrupt sound"
        );
        assert_eq!(
            sequences.get(&(GOAULD_SUMMON_TARGET, EFFECT_INIT)),
            Some(&SUMMON_TARGET_PFX),
            "1122 must keep its Effect_Init PFX-GoauldSummonTarget sequence for the summon code"
        );
    }

    /// Template 350 is the Straegis Fighter (78) body, named "Summoned
    /// Straegis Fighter", armed with set 350, never hostile, never looted,
    /// never respawned.
    #[tokio::test]
    async fn straegis_pet_template_is_the_straegis_fighter_body() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        let pet = templates
            .get(&STRAEGIS_PET)
            .expect("template 350 is seeded");
        let fighter = templates
            .get(&STRAEGIS_FIGHTER)
            .expect("template 78 is seeded");

        assert_eq!(pet.template_name, "Summoned Straegis Fighter");
        assert_eq!(pet.class, "pet");
        assert_eq!(
            pet.flags,
            ENTITYFLAG_PET | ENTITYFLAG_NO_PET_LEVELING,
            "ENTITYFLAG_Pet | NoPetLeveling and nothing else"
        );
        assert_eq!(
            pet.body_set, fighter.body_set,
            "body set must match the Straegis Fighter"
        );
        assert_eq!(
            pet.components, fighter.components,
            "components must match the Straegis Fighter"
        );
        assert_eq!(
            pet.event_set_id, fighter.event_set_id,
            "animation event set must match"
        );
        assert_ne!(
            pet.faction,
            Some(HOSTILE_FACTION),
            "a pet must not be hostile to players"
        );
        assert!(pet.faction.is_some(), "a pet needs a faction");
        assert_eq!(pet.loot_table_id, None, "a pet drops no loot");
        assert_eq!(pet.respawn_secs, None, "a pet never respawns on its own");
        assert_eq!(pet.interaction_type, 0, "a pet offers no interaction");

        assert_eq!(
            pet.ability_ids, STRAEGIS_PET_ABILITIES,
            "set 350: 221 Energy Shock as the primary, 1156 Disengage as the fallback"
        );
        assert!(
            !pet.ability_ids.contains(&STRAEGIS_EXPLODE),
            "1240 Explode must never be a pet attack (cooldown 0, death-burst VFX)"
        );

        // The nameplate text must not be empty: 28894 DN_Pet_Straegis_Tier_1 is.
        assert_eq!(pet.name_id, Some(STRAEGIS_PET_NAME));
        let text: Option<(String,)> =
            sqlx::query_as("SELECT text FROM resources.texts WHERE moniker_id = $1")
                .bind(STRAEGIS_PET_NAME)
                .fetch_optional(&pool)
                .await
                .expect("texts query");
        assert_eq!(
            text.map(|(t,)| t).as_deref(),
            Some("Summoned Straegis Fighter"),
            "the pet's name moniker must carry its display text"
        );
    }
}
