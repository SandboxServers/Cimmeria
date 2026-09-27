//! Live-DB guards for the rest of the Goa'uld Servant Lord pet roster (pets
//! campaign PT-11, `docs/analysis/pets/`): Summon Jaffa (1643) → template
//! 351, Summon Prime (1645) → 352 and Summon Lo'taur (1644) → 353.
//!
//! Each guard is about seed content that loads without error and is still
//! wrong:
//!
//! * a summon that names no template, the wrong one, or one that is not a
//!   pet (class, flag, loot);
//! * a roster pet placed in `spawnlist` (templates 350-359 are summoned
//!   creatures only, D-PT16);
//! * a roster pet that keeps level 1 at summon (`NoPetLeveling`), or that
//!   loses a stance through a `No*` flag no data asks for;
//! * a kit ability that does not exist, or whose effect does not load;
//! * a look that drifts from the template it copies, or a nameplate with
//!   no text;
//! * a summon with no cast effect.
//!
//! Each was proven to fail with the PT-11 seed rows removed (worknote
//! `docs/analysis/pets/worknotes/pt-11.md`).
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// `ENTITYFLAG_Pet` (`entities/defs/enumerations.xml`). The roster
    /// templates carry it and nothing else: no `NoPetLeveling` (8), so the
    /// summon gives the pet its owner's level (D-PT02), and no `NoPassive` /
    /// `NoDefensive` / `NoAggressive` (64/128/256), so every stance is
    /// offered.
    const ENTITYFLAG_PET: i64 = 1024;
    /// The faction every hostile NPC carries.
    const HOSTILE_FACTION: i32 = 10;
    /// Event set 1121 "Goauld summon source", as on 2826.
    const GOAULD_SUMMON_SOURCE: i32 = 1121;

    /// One roster pet: its summon, template, the template whose look it
    /// copies (if any), name moniker and its text, and its kit.
    struct RosterPet {
        summon: i32,
        template: i32,
        look_of: Option<i32>,
        name_id: i32,
        name_text: &'static str,
        kit: &'static [i32],
    }

    const ROSTER: [RosterPet; 3] = [
        // 160 Praxis Jaffa Guard; set 4 (584, 710) + 1652 Double Blast.
        RosterPet {
            summon: 1643,
            template: 351,
            look_of: Some(160),
            name_id: 8087,
            name_text: "Jaffa Soldier",
            kit: &[584, 710, 1652],
        },
        // 159 Praxis Jaffa Lieutenant; set 4 + 1654 Focus Degeneration.
        RosterPet {
            summon: 1645,
            template: 352,
            look_of: Some(159),
            name_id: 28892,
            name_text: "Jaffa Prime",
            kit: &[584, 710, 1654],
        },
        // Composed: Goa'uld male body in the AR_G_Underlings servant dress.
        RosterPet {
            summon: 1644,
            template: 353,
            look_of: None,
            name_id: 28891,
            name_text: "Lo'Taur Servant",
            kit: &[1653, 3326, 3327, 3328, 3329],
        },
    ];

    /// The Lo'taur's servant dress: every piece of it, all from
    /// `AR_G_Underlings`.
    const LOTAUR_DRESS: [&str; 5] = [
        "AR_G_Underlings.AR_GM_UT1_UT100",
        "AR_G_Underlings.AR_GM_UL1_US100",
        "AR_G_Underlings.AR_GM_UB1_UH100",
        "AR_G_Underlings.AR_GM_UG1_UB100",
        "AR_G_Underlings.AR_GM_SH1_SH100",
    ];
    const GOAULD_MALE: &str = "BS_GoauldMale.BS_GoauldMale";

    /// Each roster summon resolves, through the startup loader, to its own
    /// pet template with one active pet, and carries the Goa'uld summon
    /// cast effect.
    #[tokio::test]
    async fn roster_summons_resolve_to_their_pet_templates() {
        let pool = require_db_or_skip!();
        let summons = load_pet_summons(&pool).await.expect("pet summons load");
        let defs = load_ability_defs(&pool).await.expect("ability defs load");

        for pet in &ROSTER {
            assert_eq!(
                summons.pet_summon_for(pet.summon),
                Some(PetSummon {
                    ability_id: pet.summon,
                    template_id: pet.template,
                    max_active: 1,
                }),
                "{} must summon template {} with one active pet (D-PT04)",
                pet.summon,
                pet.template
            );
            let def = defs
                .get(&pet.summon)
                .unwrap_or_else(|| panic!("summon {} is seeded", pet.summon));
            assert_eq!(
                def.event_set_id,
                Some(GOAULD_SUMMON_SOURCE),
                "{} must carry event set 1121, the Goa'uld summon cast effect",
                pet.summon
            );
        }
    }

    /// Each roster template is a pet: class `pet`, `ENTITYFLAG_Pet` alone,
    /// friendly, no loot, no respawn, no interaction, never placed in
    /// `spawnlist`.
    #[tokio::test]
    async fn roster_templates_are_unplaced_pets() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");

        for pet in &ROSTER {
            let id = pet.template;
            let t = templates
                .get(&id)
                .unwrap_or_else(|| panic!("template {id} is seeded"));
            assert_eq!(t.class, "pet", "template {id} class");
            assert_eq!(
                t.flags, ENTITYFLAG_PET,
                "template {id}: ENTITYFLAG_Pet only (owner's level, every stance)"
            );
            assert!(t.faction.is_some(), "template {id} needs a faction");
            assert_ne!(t.faction, Some(HOSTILE_FACTION), "template {id} hostile");
            assert_eq!(t.loot_table_id, None, "template {id} drops no loot");
            assert_eq!(t.respawn_secs, None, "template {id} never respawns");
            assert_eq!(t.interaction_type, 0, "template {id} offers no interaction");
        }

        let ids: Vec<i32> = ROSTER.iter().map(|p| p.template).collect();
        let placed: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT spawn_id, template_id FROM resources.spawnlist \
             WHERE template_id = ANY($1) ORDER BY spawn_id",
        )
        .bind(&ids)
        .fetch_all(&pool)
        .await
        .expect("spawnlist query");
        assert!(
            placed.is_empty(),
            "roster pets placed in spawnlist (spawn_id, template_id): {placed:?}"
        );
    }

    /// Each roster template carries its kit, and every kit ability and
    /// every effect of it loads, so the pet bar and the AI never name an
    /// ability the server cannot resolve.
    #[tokio::test]
    async fn roster_kits_exist() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        let defs = load_ability_defs(&pool).await.expect("ability defs load");
        let effects = load_effect_defs(&pool).await.expect("effect defs load");

        let mut bad = Vec::new();
        for pet in &ROSTER {
            let t = templates
                .get(&pet.template)
                .unwrap_or_else(|| panic!("template {} is seeded", pet.template));
            assert_eq!(
                t.ability_ids, pet.kit,
                "template {} kit (ascending: the lowest id is the primary)",
                pet.template
            );
            for ability_id in pet.kit {
                let Some(def) = defs.get(ability_id) else {
                    bad.push(format!("{}: ability {ability_id} missing", pet.template));
                    continue;
                };
                for effect_id in &def.effect_ids {
                    if !effects.contains_key(effect_id) {
                        bad.push(format!(
                            "{}: ability {ability_id} effect {effect_id} missing",
                            pet.template
                        ));
                    }
                }
            }
        }
        assert!(bad.is_empty(), "roster kit gaps:\n{}", bad.join("\n"));
    }

    /// The Jaffa and the Prime wear exactly the look they copy, the
    /// Lo'taur wears the Goa'uld male body in the full servant dress, and
    /// every nameplate has text.
    #[tokio::test]
    async fn roster_looks_and_names() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");

        for pet in &ROSTER {
            let t = &templates[&pet.template];
            if let Some(source) = pet.look_of {
                let s = templates
                    .get(&source)
                    .unwrap_or_else(|| panic!("template {source} is seeded"));
                assert_eq!(t.body_set, s.body_set, "{} body set", pet.template);
                assert_eq!(t.components, s.components, "{} components", pet.template);
                assert_eq!(t.event_set_id, s.event_set_id, "{} anim set", pet.template);
            }

            assert_eq!(t.name_id, Some(pet.name_id), "{} name_id", pet.template);
            let text: Option<(String,)> =
                sqlx::query_as("SELECT text FROM resources.texts WHERE moniker_id = $1")
                    .bind(pet.name_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("texts query");
            assert_eq!(
                text.map(|(t,)| t).as_deref(),
                Some(pet.name_text),
                "{} nameplate text",
                pet.template
            );
        }

        let lotaur = &templates[&353];
        assert_eq!(lotaur.body_set, GOAULD_MALE);
        let components = lotaur.components.clone().unwrap_or_default();
        for piece in LOTAUR_DRESS {
            assert!(
                components.iter().any(|c| c == piece),
                "353 must wear {piece}, has {components:?}"
            );
            // The piece must be declared for the Goa'uld male body, or the
            // client has no mesh to hang on it.
            let fits: Option<(bool,)> = sqlx::query_as(
                "SELECT $2 = ANY(body_sets) FROM resources.body_components \
                 WHERE component_name = $1",
            )
            .bind(piece)
            .bind(GOAULD_MALE)
            .fetch_optional(&pool)
            .await
            .expect("body_components query");
            assert_eq!(fits, Some((true,)), "{piece} must fit {GOAULD_MALE}");
        }
    }
}
