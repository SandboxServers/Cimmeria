//! Pets campaign PT-07: the GM `.pet` console (`console/pet.rs`).
//!
//! Filter prefix: `pt07_`.
//!
//! Bug shapes: a summon that spawns the wrong template or no owner; a second
//! summon that leaves two pets; a refused summon that already dismissed the
//! old pet; a stance outside `EPetStance` coerced instead of refused; a
//! stance update fanned to someone other than the owner; and a GM verb that
//! reaches another player's pet.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::PetStance;
use tokio::sync::mpsc;

use super::decode_feedback;
use crate::cell::client_methods::pet::ON_PET_STANCE_UPDATE;
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::{PetSummon, PetSummonCatalog, SpawnRecord};
use crate::mercury::SGWPET_CLASS_ID;

const GM: u32 = 1;
const OTHER: u32 = 2;
const FAR: u32 = 3;
const PET_TEMPLATE: i32 = 350;
const SUMMON_STRAEGIS: i32 = 2826;
const PET_ABILITIES: [i32; 2] = [221, 1156];

fn pet_record(template_id: i32) -> SpawnRecord {
    SpawnRecord {
        spawn_id: -1,
        world_name: String::new(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        heading: 0.0,
        tag: None,
        template_id,
        template_name: "Summoned Straegis Fighter".to_string(),
        class: "pet".to_string(),
        static_mesh: None,
        body_set: "MOB_StraegisFighter.BS_MOB_StraegisFighter".to_string(),
        components: None,
        flags: 1032,
        interaction_type: 0,
        event_set_id: Some(570),
        level: Some(1),
        alignment: Some(0),
        faction: Some(1),
        name_id: Some(27377),
        speaker_id: None,
        static_interaction_sets: vec![],
        has_dynamic_properties: true,
        loot_table_id: None,
        is_stationary: false,
        ability_ids: PET_ABILITIES.to_vec(),
        respawn_secs: None,
        patrol_path: vec![],
        patrol_point_delay_secs: 2.0,
        wander_radius: 0.0,
        wander_min_dwell_secs: 3.0,
        wander_max_dwell_secs: 8.0,
        follow_min_distance: 2.0,
        follow_max_distance: 5.0,
        move_speed: 0.9,
        leash_distance: None,
        aggro_radius: None,
        assist_radius: None,
        aggression_override: None,
        use_cover: Some(false),
    }
}

fn add_player(mgr: &mut SpaceManager, id: u32, world: &str, pos: [f32; 3]) {
    mgr.create_entity(id, world, pos, [0.0; 3]).unwrap();
    mgr.connect_entity(id);
    let e = mgr.get_entity_mut(id).unwrap();
    e.is_player = true;
    e.player_id = Some(70 + id as i32);
    e.character_name = Some(format!("Tester{id}"));
}

/// GM (1) and another player (2) in Agnos, a third player (3) in Harset;
/// template 350 cached and 2826 mapped to it.
fn pet_world() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /><Space WorldName="Harset" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /><Space WorldName="Harset" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    add_player(&mut mgr, GM, "Agnos", [10.0, 0.0, 10.0]);
    mgr.get_entity_mut(GM).unwrap().access_level = 2;
    add_player(&mut mgr, OTHER, "Agnos", [20.0, 0.0, 20.0]);
    add_player(&mut mgr, FAR, "Harset", [10.0, 0.0, 10.0]);
    mgr.spawn_templates
        .insert(PET_TEMPLATE, pet_record(PET_TEMPLATE));
    mgr.pet_summons = PetSummonCatalog::from_rows([PetSummon {
        ability_id: SUMMON_STRAEGIS,
        template_id: PET_TEMPLATE,
        max_active: 1,
    }]);
    mgr
}

/// Run `.pet <args>` as `caller` with `target` selected; return every
/// message sent.
async fn pet(
    mgr: &mut SpaceManager,
    caller: u32,
    target: Option<u32>,
    args: &[&str],
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    exec("pet", caller, args, target, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn feedback(msgs: &[CellToBaseMsg]) -> Vec<String> {
    msgs.iter().filter_map(decode_feedback).collect()
}

fn stance_updates(msgs: &[CellToBaseMsg]) -> Vec<(u32, u32, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                ..
            } if *method_index == ON_PET_STANCE_UPDATE => {
                Some((*witness_id, *entity_id, args.clone()))
            }
            _ => None,
        })
        .collect()
}

fn only_pet_of(mgr: &SpaceManager, owner: u32) -> u32 {
    let pets = mgr.pets.pets_of(owner);
    assert_eq!(pets.len(), 1, "owner {owner} must have exactly one pet");
    pets[0]
}

#[tokio::test]
async fn pt07_pet_summon_by_ability_spawns_its_template_owned_by_the_gm() {
    let mut mgr = pet_world();
    let fb = feedback(&pet(&mut mgr, GM, None, &["summon", "2826"]).await);

    let pet_id = only_pet_of(&mgr, GM);
    let e = mgr.get_entity(pet_id).unwrap();
    assert_eq!(e.template_id, Some(PET_TEMPLATE));
    assert_eq!(e.class_id, SGWPET_CLASS_ID);
    let state = e.pet.as_deref().expect("pet state");
    assert_eq!(
        (state.owner_id, state.summon_ability_id),
        (GM, SUMMON_STRAEGIS)
    );
    assert_eq!(state.ability_list, PET_ABILITIES.to_vec());
    assert!(
        fb.iter()
            .any(|l| l.contains(&format!("pet {pet_id} from template 350 via ability 2826"))),
        "{fb:?}"
    );
}

#[tokio::test]
async fn pt07_pet_summon_by_template_id_records_no_summon_ability() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "350"]).await;
    let pet_id = only_pet_of(&mgr, GM);
    let state = mgr.get_entity(pet_id).unwrap().pet.as_deref().unwrap();
    assert_eq!(state.summon_ability_id, 0);
}

/// D-PT04: one pet per owner. The second summon despawns the first.
#[tokio::test]
async fn pt07_pet_summon_again_replaces_the_pet() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let first = only_pet_of(&mgr, GM);
    let fb = feedback(&pet(&mut mgr, GM, None, &["summon", "350"]).await);

    let second = only_pet_of(&mgr, GM);
    assert_ne!(first, second);
    assert!(
        mgr.get_entity(first).is_none(),
        "the first pet is despawned"
    );
    assert!(
        fb.iter()
            .any(|l| l.contains(&format!("replacing [{first}]"))),
        "{fb:?}"
    );
}

/// A summon that cannot succeed must be refused before the old pet goes.
#[tokio::test]
async fn pt07_pet_summon_of_an_unknown_id_keeps_the_old_pet() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let first = only_pet_of(&mgr, GM);

    for bad in [&["summon", "99999"][..], &["summon", "x"], &["summon"]] {
        let fb = feedback(&pet(&mut mgr, GM, None, bad).await);
        assert_eq!(fb.len(), 1, "{bad:?} answers once: {fb:?}");
    }
    assert_eq!(only_pet_of(&mgr, GM), first, "the old pet survives");
    assert!(mgr.get_entity(first).is_some());
}

#[tokio::test]
async fn pt07_pet_stance_sets_it_and_tells_only_the_owner() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let pet_id = only_pet_of(&mgr, GM);

    let msgs = pet(&mut mgr, GM, None, &["stance", "2"]).await;

    assert_eq!(
        mgr.get_entity(pet_id)
            .unwrap()
            .pet
            .as_deref()
            .unwrap()
            .stance,
        PetStance::Aggressive
    );
    assert_eq!(
        stance_updates(&msgs),
        vec![(GM, pet_id, vec![2u8])],
        "exactly one onPetStanceUpdate(INT8 2), owner only"
    );
}

/// `EPetStance` is 0..=2. Nothing else may be truncated or coerced.
#[tokio::test]
async fn pt07_pet_stance_refuses_values_outside_epetstance() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let pet_id = only_pet_of(&mgr, GM);

    for bad in ["3", "-1", "300", "258", "x"] {
        let msgs = pet(&mut mgr, GM, None, &["stance", bad]).await;
        assert!(stance_updates(&msgs).is_empty(), "{bad}: nothing sent");
        assert_eq!(feedback(&msgs).len(), 1, "{bad}: refused with feedback");
    }
    assert_eq!(
        mgr.get_entity(pet_id)
            .unwrap()
            .pet
            .as_deref()
            .unwrap()
            .stance,
        PetStance::Defensive
    );
}

/// The mutating verbs act on the caller's own pet only. Selecting another
/// player's pet does not make it the GM's.
#[tokio::test]
async fn pt07_pet_dismiss_and_stance_never_touch_another_owners_pet() {
    let mut mgr = pet_world();
    let theirs = mgr
        .spawn_pet_from_template(OTHER, PET_TEMPLATE, SUMMON_STRAEGIS)
        .unwrap();

    let stance = pet(&mut mgr, GM, Some(theirs), &["stance", "0"]).await;
    let dismiss = pet(&mut mgr, GM, Some(theirs), &["dismiss"]).await;

    assert!(stance_updates(&stance).is_empty());
    assert_eq!(feedback(&stance), vec![".pet stance: you have no pet out"]);
    assert_eq!(
        feedback(&dismiss),
        vec![".pet dismiss: you have no pet out"]
    );
    assert_eq!(only_pet_of(&mgr, OTHER), theirs);
    assert_eq!(
        mgr.get_entity(theirs)
            .unwrap()
            .pet
            .as_deref()
            .unwrap()
            .stance,
        PetStance::Defensive
    );

    // With a pet of its own, the GM's dismiss removes only that one.
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let mine = only_pet_of(&mgr, GM);
    pet(&mut mgr, GM, Some(theirs), &["dismiss"]).await;
    assert!(mgr.pets.pets_of(GM).is_empty());
    assert!(mgr.get_entity(mine).is_none());
    assert_eq!(only_pet_of(&mgr, OTHER), theirs);
}

#[tokio::test]
async fn pt07_pet_info_reports_owner_stance_lists_ai_and_distance() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let pet_id = only_pet_of(&mgr, GM);

    let fb = feedback(&pet(&mut mgr, GM, None, &["info"]).await).join("\n");

    for want in [
        format!("pet {pet_id}, template 350, owner Tester1 (1), summoned by ability 2826"),
        "stance defensive (allowed: passive, defensive, aggressive)".to_string(),
        "AI state".to_string(),
        "abilities [221, 1156], toggled off []".to_string(),
        "distance to owner 2.0 u".to_string(),
        "last teleport never".to_string(),
    ] {
        assert!(fb.contains(&want), "missing {want:?} in:\n{fb}");
    }
}

/// A selected pet is inspected whoever owns it (read only).
#[tokio::test]
async fn pt07_pet_info_reads_a_selected_pet_of_another_owner() {
    let mut mgr = pet_world();
    let theirs = mgr.spawn_pet_from_template(OTHER, PET_TEMPLATE, 0).unwrap();
    let fb = feedback(&pet(&mut mgr, GM, Some(theirs), &["info"]).await);
    assert!(
        fb[0].contains(&format!("pet {theirs}")) && fb[0].contains("owner Tester2 (2)"),
        "{fb:?}"
    );
}

#[tokio::test]
async fn pt07_pet_list_shows_only_pets_in_the_callers_space() {
    let mut mgr = pet_world();
    pet(&mut mgr, GM, None, &["summon", "2826"]).await;
    let theirs = mgr.spawn_pet_from_template(OTHER, PET_TEMPLATE, 0).unwrap();
    let far = mgr.spawn_pet_from_template(FAR, PET_TEMPLATE, 0).unwrap();

    let fb = feedback(&pet(&mut mgr, GM, None, &["list"]).await);

    assert_eq!(fb[0], ".pet list: 2 pet(s) in this space");
    assert_eq!(fb.len(), 3, "{fb:?}");
    assert!(fb.iter().any(|l| l.contains(&format!("pet {theirs} "))));
    assert!(!fb.iter().any(|l| l.contains(&format!("pet {far} "))));
}

#[tokio::test]
async fn pt07_pet_without_a_known_verb_prints_usage() {
    let mut mgr = pet_world();
    for args in [&["frobnicate"][..], &["10"]] {
        let fb = feedback(&pet(&mut mgr, GM, None, args).await);
        assert_eq!(fb.len(), 1);
        assert!(fb[0].starts_with("Usage: .pet summon"), "{fb:?}");
    }
}
