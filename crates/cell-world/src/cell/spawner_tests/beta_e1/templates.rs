//! UB-02a: the Beta E1 cast and object templates (2300-2335, the reserved ids
//! 2303, 2327 and 2336 left empty). Each is named by the client's own
//! string for that actor or prop, and the objects are the mission world
//! objects the chains bind per player.

use std::collections::BTreeMap;

use cimmeria_entity::cell_entity::MobAggression;
use cimmeria_entity::interaction_flags::INT_MISSION_WORLD_OBJECT;

use super::*;
use crate::cell::combat::faction_reaction::{reaction, FACTION_COUNT};
use crate::cell::combat::{
    aggression_toward_players, is_hostile_to_players, player_may_attack_pve, seeks_npc_targets,
    HOSTILE_FACTION,
};
use crate::cell::spawner::class_id_for_class;

/// `(template id, class, moniker name of its name_id)`: each template is
/// named by the client's own display-name string. Pinned by moniker name, not
/// by text id. 2335 has no client moniker: its `name_id` is NULL.
const ROSTER: [(i32, &str, Option<&str>); 22] = [
    (2300, "mob", Some("DN_npc_MG_BetaE1_Admin_Carter")),
    (2301, "mob", Some("DN_npc_MG_BetaE1_Admin_Felger")),
    (2302, "mob", Some("DN_npc_MG_BetaE1_Admin_Satterfield")),
    (2304, "mob", Some("DN_NPC_Int_BetaE1_Admin_Debeaux")),
    (2305, "mob", Some("DN_npc_MG_BetaE1_AsgardBase_Mimir")),
    (2306, "mob", Some("DN_npc_MG_BetaE1_RasnorsCamp_Rasnor")),
    (2307, "mob", Some("DN_npc_Int_BetaE1_Various_SickJaffa")),
    (2320, "being", Some("DN_Ob_It_Human_BetaE1_XenobiologyCage")),
    (2321, "being", Some("DN_Ob_It_Human_BetaE1_FourPiecesCase")),
    (2322, "being", Some("DN_Ob_D_Human_BetaE1_HubComputer")),
    (
        2323,
        "being",
        Some("DN_Ob_It_Human_BetaE1_AdminWorkstation"),
    ),
    (
        2324,
        "being",
        Some("DN_Ob_D_Human_BetaE1_ScienceCenterComputer_02SGUDatabase"),
    ),
    (2325, "being", Some("DN_Ob_It_Human_BetaE1_CrateWreckage")),
    (2326, "being", Some("DN_Ob_D_Jaffa_BetaE1_BoxOfJaffaTools")),
    (
        2328,
        "being",
        Some("DN_Ob_it_Asgard_BetaE1_FireSuppressantManager"),
    ),
    (
        2329,
        "being",
        Some("DN_Ob_G_Human_BetaE1_SprinklerSystemManularControl"),
    ),
    (2330, "being", Some("DN_Ob_It_Asgard_BetaE1_AsgardScanner")),
    (
        2331,
        "being",
        Some("DN_Ob_G_ASG_BetaE1_AnalyticalAssistanceMachine"),
    ),
    (
        2332,
        "being",
        Some("DN_Ob_It_Asgard_BetaE1_MissingAsgardHologramEmitter"),
    ),
    (
        2333,
        "being",
        Some("DN_Ob_G_Human_BetaE1_DisabledMalfunctioningDrone"),
    ),
    (
        2334,
        "being",
        Some("DN_Ob_It_Asgard_BetaE1_ConsciousnessSimulator"),
    ),
    (2335, "being", None),
];

/// The template that draws a body although it is an object: the malfunctioning
/// drone (2333) takes template 69's body set, so it is pinned by id.
const DRONE_BODY: i32 = 2333;

/// The reserved ids: 2303 (Dr. Pierce), 2327 (Accelerant) and 2336 are UB-52's
/// twins 383 and 384 (D-UB7). No row may hold them until UB-52 ships.
const RESERVED: [i32; 3] = [2303, 2327, 2336];

/// The templates whose `display_name` is the nameplate (their client string
/// is empty or absent).
const LITERAL_NAMES: [i32; 2] = [2301, 2335];

/// The block's templates as the cell loads them, keyed by template id.
/// Panics unless the block holds exactly the roster's ids.
async fn block_templates(pool: &sqlx::PgPool) -> BTreeMap<i32, SpawnRecord> {
    let block: BTreeMap<i32, SpawnRecord> = load_spawn_templates(pool)
        .await
        .expect("load_spawn_templates must succeed")
        .into_iter()
        .filter(|(id, _)| (BLOCK.0..=BLOCK.1).contains(id))
        .collect();
    assert_eq!(
        block.keys().copied().collect::<Vec<_>>(),
        ROSTER.iter().map(|(id, ..)| *id).collect::<Vec<_>>(),
        "templates {}-{} loaded from entity_templates_beta_e1.sql",
        BLOCK.0,
        BLOCK.1
    );
    block
}

/// Spawn one template into a fixture space beside a player, as a spawn row
/// or a GM `.spawn` would, and return the manager and the NPC's entity id.
fn spawned(
    record: &SpawnRecord,
) -> (
    crate::cell::space_manager::SpaceManager,
    /* player */ u32,
    /* npc */ u32,
) {
    const PLAYER: u32 = 1;
    let mut mgr = crate::test_support::make_space_manager_with_player(PLAYER);
    let mut record = record.clone();
    record.world_name = "Agnos".to_string();
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc, &record)
        .unwrap_or_else(|e| panic!("template {} must spawn: {e}", record.template_id));
    (mgr, PLAYER, npc)
}

/// The block holds the twenty-two templates, each of its class and each named
/// by the client's own string for that actor or prop.
///
/// Revert proof: remove the seed file's `\ir` line from `database.sql`, or
/// point a `name_id` at another moniker, and this fails.
#[tokio::test]
async fn live_db_beta_e1_cast_and_objects_match_the_roster() {
    let pool = require_db_or_skip!();
    block_templates(&pool).await;
    let rows: Vec<(i32, String, Option<String>)> = sqlx::query_as(
        "SELECT t.template_id, t.class::text, x.moniker_name::text \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id BETWEEN $1 AND $2 \
         ORDER BY t.template_id",
    )
    .bind(BLOCK.0)
    .bind(BLOCK.1)
    .fetch_all(&pool)
    .await
    .expect("roster query must succeed");
    let expected: Vec<(i32, String, Option<String>)> = ROSTER
        .iter()
        .map(|(id, class, moniker)| (*id, class.to_string(), moniker.map(str::to_string)))
        .collect();
    assert_eq!(rows, expected, "(template id, class, name_id moniker)");
}

/// Every template shows a name. Its `name_id` text is not empty, or it has a
/// `display_name`; never both and never neither. Exactly the two literals
/// (2301 Dr. Felger, whose string is empty, and 2335, which has no moniker)
/// take a `display_name`. The literal reaches the spawned entity, and the
/// class is one the client binds the name methods on.
///
/// Revert proof: clear 2301's `display_name`, and this fails naming 2301.
#[tokio::test]
async fn live_db_every_beta_e1_template_shows_a_name() {
    let pool = require_db_or_skip!();
    let block = block_templates(&pool).await;
    let rows: Vec<(i32, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT t.template_id, t.template_name::text, x.text::text, t.display_name::text \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id BETWEEN $1 AND $2 \
         ORDER BY t.template_id",
    )
    .bind(BLOCK.0)
    .bind(BLOCK.1)
    .fetch_all(&pool)
    .await
    .expect("name query must succeed");
    assert_eq!(rows.len(), block.len());

    let mut literals = Vec::new();
    for (id, template_name, text, display_name) in &rows {
        let has_text = text.as_deref().is_some_and(|t| !t.trim().is_empty());
        let has_literal = display_name
            .as_deref()
            .is_some_and(|n| !n.trim().is_empty());
        assert!(
            has_text != has_literal,
            "template {id} ({template_name}): name_id text {text:?}, display_name \
             {display_name:?}; exactly one must name it (the client's text where it has \
             one, a display_name where that text is empty)"
        );
        if has_literal {
            literals.push(*id);
        }

        let record = &block[id];
        assert_eq!(
            &record.display_name, display_name,
            "template {id}: the loader carries display_name"
        );
        assert!(
            (0x01..=0x05).contains(&class_id_for_class(&record.class)),
            "template {id} ({template_name}) is class {:?}: the client binds the name \
             methods on SGWBeing classes only, so it would show no name",
            record.class
        );
        let (mgr, _, npc) = spawned(record);
        let e = mgr.get_entity(npc).expect("spawned NPC");
        assert_eq!(&e.display_name, display_name, "template {id}: spawned");
        assert_eq!(e.name_id, record.name_id, "template {id}: spawned name_id");
    }
    assert_eq!(
        literals,
        LITERAL_NAMES.to_vec(),
        "the templates whose nameplate is a display_name literal"
    );
}

/// Nothing in the block is hostile to a Union player, and nothing in it can be
/// attacked. The server keeps no reaction per archetype, so the check is on
/// the spawned NPC itself: each is friendly to players, outside the hostile
/// faction a player's attack needs, a target for no faction's NPCs, and looks
/// for no NPC target.
///
/// Revert proof: set `faction = 10` (or any faction the table makes hostile)
/// on a row and this names the template.
#[tokio::test]
async fn live_db_no_beta_e1_template_is_hostile_or_attackable() {
    let pool = require_db_or_skip!();
    for (id, record) in &block_templates(&pool).await {
        let (mgr, player, npc) = spawned(record);
        let e = mgr.get_entity(npc).expect("spawned NPC");
        let name = &record.template_name;
        assert!(
            !is_hostile_to_players(e),
            "template {id} ({name}) aggroes players on sight (faction {})",
            e.faction
        );
        assert_eq!(
            aggression_toward_players(e),
            MobAggression::Friendly,
            "template {id} ({name}), faction {}",
            e.faction
        );
        assert_ne!(e.faction, HOSTILE_FACTION, "template {id} ({name})");
        assert!(
            !player_may_attack_pve(mgr.get_entity(player).expect("player"), e),
            "template {id} ({name}) can be attacked by a player"
        );
        let hunters: Vec<u8> = (0..FACTION_COUNT as u8)
            .filter(|viewer| reaction(*viewer, e.faction).is_hostile())
            .collect();
        assert!(
            hunters.is_empty(),
            "template {id} ({name}), faction {}: NPCs of factions {hunters:?} would attack it",
            e.faction
        );
        assert!(
            !seeks_npc_targets(e),
            "template {id} ({name}) looks for NPC targets"
        );
    }
}

/// Every template names a body or a mesh, has stationary spawn settings and
/// carries no loot, vendor or trainer data (the campaign's seed rules). Mobs
/// draw a body and objects a mesh, except the drone (2333), which draws a
/// body and is pinned by id: its body set is template 69's.
///
/// Revert proof: null a prop's `static_mesh`, or `respawn_secs`, or give a
/// row a `wander_radius`, and this names the template.
#[tokio::test]
async fn live_db_every_beta_e1_template_has_visual_and_spawn_settings() {
    let pool = require_db_or_skip!();
    for (id, r) in &block_templates(&pool).await {
        let name = &r.template_name;
        let has_body =
            !r.body_set.is_empty() && r.components.as_ref().is_some_and(|c| !c.is_empty());
        let has_mesh = r.static_mesh.as_ref().is_some_and(|m| !m.is_empty());
        if r.class == "mob" || *id == DRONE_BODY {
            assert!(has_body && !has_mesh, "{id} ({name}) draws a body");
        } else {
            assert!(has_mesh && !has_body, "prop {id} ({name}) draws a mesh");
        }
        assert!(
            r.respawn_secs.is_some(),
            "template {id} ({name}) sets respawn_secs"
        );
        assert_eq!(r.loot_table_id, None, "template {id} ({name}): no loot");
        assert!(
            r.patrol_path.is_empty() && r.wander_radius == 0.0,
            "template {id} ({name}) neither patrols nor wanders"
        );
    }
    let (services,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.entity_templates \
         WHERE template_id BETWEEN $1 AND $2 \
           AND (buy_item_list IS NOT NULL OR sell_item_list IS NOT NULL \
                OR repair_item_list IS NOT NULL OR recharge_item_list IS NOT NULL \
                OR trainer_ability_list_id IS NOT NULL OR patrol_path_id IS NOT NULL)",
    )
    .bind(BLOCK.0)
    .bind(BLOCK.1)
    .fetch_one(&pool)
    .await
    .expect("service column query must succeed");
    assert_eq!(services, 0, "no vendor list, trainer list or patrol path");
}

/// Each object carries `INT_MissionWorldObject` on its template, so it is
/// clickable from spawn, and each NPC carries no interaction bit: chains reach
/// the NPCs through per-player binds only (D-UB10). The bit is only a cursor;
/// no object becomes a vendor or a terminal of another kind.
///
/// Revert proof: set 2300's `interaction_type` to 1073741824, and this names
/// the NPC.
#[tokio::test]
async fn live_db_beta_e1_objects_are_mission_world_objects_and_npcs_are_not() {
    let pool = require_db_or_skip!();
    let block = block_templates(&pool).await;
    let mut objects = 0;
    for (id, class, _) in ROSTER {
        let record = &block[&id];
        let want = if class == "being" {
            objects += 1;
            INT_MISSION_WORLD_OBJECT
        } else {
            0
        };
        assert_eq!(
            record.interaction_type, want,
            "template {id} ({class}): interaction_type"
        );
        let (mgr, _, npc) = spawned(record);
        let e = mgr.get_entity(npc).expect("spawned NPC");
        assert_eq!(e.interaction_type_flags, want, "template {id}: spawned");
        assert_eq!(
            e.interaction_type, None,
            "template {id} opens no static interaction"
        );
    }
    assert_eq!(objects, 15, "the fifteen object templates");
}

/// The reserved ids 2303, 2327 and 2336 have no row, so the block holds only
/// the roster. D-UB7 ruled UB-52's twins out of the first ship, so these ids
/// stay empty.
///
/// Revert proof: insert a row at 2327, and this names the id.
#[tokio::test]
async fn live_db_beta_e1_reserved_ids_are_absent() {
    let pool = require_db_or_skip!();
    block_templates(&pool).await;
    let (present,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM resources.entity_templates WHERE template_id = ANY($1)",
    )
    .bind(RESERVED.to_vec())
    .fetch_one(&pool)
    .await
    .expect("reserved-id query must succeed");
    assert_eq!(present, 0, "reserved ids {RESERVED:?} must have no row");
}

/// The `entity_templates` id sequence hands out ids past the whole Beta block
/// (2349), not just past the last row (2335), so a default-id insert or GM
/// `.spawn`-created template cannot land in the reserved 2336-2349 tail.
/// `seeded_sequences_allocate_past_every_seeded_row` cannot see this: the
/// Debug Area footers loaded after Beta's lift the sequence to the table's
/// MAX, which hides a missing Beta footer.
///
/// Revert proof: delete the footer line of `entity_templates_beta_e1.sql`, and
/// the next id is 2336 and this fails.
#[tokio::test]
async fn live_db_beta_e1_template_sequence_clears_the_whole_block() {
    let pool = require_db_or_skip!();
    let (next,): (i64,) = sqlx::query_as(
        "SELECT CASE WHEN is_called THEN last_value + 1 ELSE last_value END \
         FROM resources.entity_templates_template_id_seq",
    )
    .fetch_one(&pool)
    .await
    .expect("sequence query must succeed");
    assert!(
        next > i64::from(BLOCK.1),
        "entity_templates_template_id_seq would hand out {next}, inside the Beta block \
         {}-{}",
        BLOCK.0,
        BLOCK.1
    );
}
