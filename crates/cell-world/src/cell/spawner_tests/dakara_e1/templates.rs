//! DK-03: the Dakara_E1 cast and interactable prop templates (440-459).
//!
//! Every test reads the block through the production loader
//! ([`block_templates`]), which fails when a row is missing, so none of them
//! passes with the seed file removed.

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
/// named by the client's own display-name string for that Dakara_E1 actor.
/// Pinned by moniker name, not by text id, so the pairing reads as what it is.
const ROSTER: [(i32, &str, &str); 18] = [
    (440, "mob", "DN_npc_int_Lothta_DakaraE1"),
    (441, "mob", "DN_npc_int_Raknor_DakaraE1"),
    (442, "mob", "DN_npc_int_JaffaCaptain_DakaraE1"),
    (443, "mob", "DN_npc_int_Baal_DakaraE1_Hologram"),
    (444, "mob", "DN_npc_esc_Jaffa_DakaraE1_Escort"),
    (445, "being", "DN_ob_DakaraE1_sc_TentFlap_ToCommand"),
    (446, "being", "DN_ob_DakaraE1_sc_TentFlap_ToMohkatan"),
    (447, "being", "DN_ob_DakaraE1_sc_TentFlap_FromCommand"),
    (448, "being", "DN_ob_DakaraE1_sc_TentFlap_FromMohkatan"),
    (449, "being", "DN_ob_DakaraE1_int_DropLoc"),
    (450, "being", "DN_ob_DakaraE1_int_SG18Corpse01"),
    (451, "being", "DN_ob_DakaraE1_int_SG18Corpse02"),
    (452, "being", "DN_ob_DakaraE1_int_SG18Corpse03"),
    (453, "being", "DN_ob_DakaraE1_int_CommandTerminal"),
    (454, "being", "DN_ob_DakaraE1StoryRm_int_MohkatanTerminal"),
    (455, "being", "DN_ob_DakaraE1_int_RingControl"),
    (456, "being", "DN_ob_DakaraE1_int_TurretPowerSupply"),
    (457, "being", "DN_ob_DakaraE1_int_Vocuum"),
];

/// The tent flaps' monikers: the four props that are clickable from spawn.
const TENT_FLAP: &str = "_sc_TentFlap_";

/// The block's templates as the cell loads them, keyed by template id.
/// Panics unless the block holds exactly the roster's ids.
async fn block_templates(pool: &sqlx::PgPool) -> BTreeMap<i32, SpawnRecord> {
    let block: BTreeMap<i32, SpawnRecord> = load_spawn_templates(pool)
        .await
        .expect("load_spawn_templates must succeed")
        .into_iter()
        .filter(|(id, _)| (CAST_AND_PROPS.0..=CAST_AND_PROPS.1).contains(id))
        .collect();
    assert_eq!(
        block.keys().copied().collect::<Vec<_>>(),
        ROSTER.iter().map(|(id, ..)| *id).collect::<Vec<_>>(),
        "templates {}-{} loaded from entity_templates_dakara_e1.sql",
        CAST_AND_PROPS.0,
        CAST_AND_PROPS.1
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

/// The block holds the eighteen DK-03 templates, each of its class and each
/// named by the client's own string for that actor. Revert proof: remove the
/// seed file's `\ir` line, or point a row at another moniker, and this fails.
#[tokio::test]
async fn live_db_dakara_e1_cast_and_props_match_the_roster() {
    let pool = require_db_or_skip!();
    block_templates(&pool).await;
    let rows: Vec<(i32, String, Option<String>)> = sqlx::query_as(
        "SELECT t.template_id, t.class::text, x.moniker_name::text \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id BETWEEN $1 AND $2 \
         ORDER BY t.template_id",
    )
    .bind(CAST_AND_PROPS.0)
    .bind(CAST_AND_PROPS.1)
    .fetch_all(&pool)
    .await
    .expect("roster query must succeed");
    let expected: Vec<(i32, String, Option<String>)> = ROSTER
        .iter()
        .map(|(id, class, moniker)| (*id, class.to_string(), Some(moniker.to_string())))
        .collect();
    assert_eq!(rows, expected, "(template id, class, name_id moniker)");
}

/// DK-03 acceptance: every template shows a name. Its `name_id` text is not
/// empty, or it has a `display_name`; never both (a literal would hide the
/// client's own text) and never neither (a blank nameplate). The literal
/// reaches the spawned entity, and the class is one the client binds the name
/// methods on (`mercury::aoi::create`: a class-0 spawnable is sent no name).
///
/// Revert proof: clear a `display_name` whose string is empty, or set
/// `class = 'spawnable'` on a row, and this names the template.
#[tokio::test]
async fn live_db_every_dakara_e1_cast_and_prop_shows_a_name() {
    let pool = require_db_or_skip!();
    let block = block_templates(&pool).await;
    let rows: Vec<(i32, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT t.template_id, t.template_name::text, x.text::text, t.display_name::text \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id BETWEEN $1 AND $2 \
         ORDER BY t.template_id",
    )
    .bind(CAST_AND_PROPS.0)
    .bind(CAST_AND_PROPS.1)
    .fetch_all(&pool)
    .await
    .expect("name query must succeed");
    assert_eq!(rows.len(), block.len());

    let (mut client_text, mut literal) = (0, 0);
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
        client_text += usize::from(has_text);
        literal += usize::from(has_literal);

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
    assert!(
        client_text > 0 && literal > 0,
        "the block has both kinds: {client_text} client texts, {literal} literals"
    );
}

/// DK-03 acceptance: nothing in the block is hostile to a Free Jaffa
/// (archetype 7), and nothing in it can be attacked. The server keeps no
/// reaction per archetype: every player, whatever the archetype, reacts as
/// `PLAYER_REACTION_FACTION`, so the check is on the spawned NPC itself. Each
/// one is friendly to players, outside the hostile faction a player's attack
/// needs, a target for no faction's NPCs, and looks for no NPC target.
///
/// Revert proof: set `faction = 10` (or any faction the table makes hostile)
/// on a row and this names the template.
#[tokio::test]
async fn live_db_no_dakara_e1_cast_or_prop_is_hostile_or_attackable() {
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

/// Every template names a body or mesh, has stationary spawn settings and
/// carries no loot, vendor or trainer data (the campaign's seed rules).
/// The server only applies `respawn_secs` to mobs; this test checks the seed
/// column, not a death/respawn cycle or client rendering.
///
/// Revert proof: null a prop's `static_mesh`, or `respawn_secs`, or give a
/// row a `wander_radius`, and this names the template.
#[tokio::test]
async fn live_db_every_dakara_e1_cast_and_prop_has_visual_and_spawn_settings() {
    let pool = require_db_or_skip!();
    for (id, r) in &block_templates(&pool).await {
        let name = &r.template_name;
        let has_body =
            !r.body_set.is_empty() && r.components.as_ref().is_some_and(|c| !c.is_empty());
        let has_mesh = r.static_mesh.as_ref().is_some_and(|m| !m.is_empty());
        match r.class.as_str() {
            "mob" => assert!(has_body && !has_mesh, "cast {id} ({name}) draws a body"),
            _ => assert!(has_mesh && !has_body, "prop {id} ({name}) draws a mesh"),
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
    .bind(CAST_AND_PROPS.0)
    .bind(CAST_AND_PROPS.1)
    .fetch_one(&pool)
    .await
    .expect("service column query must succeed");
    assert_eq!(services, 0, "no vendor list, trainer list or patrol path");
}

/// No look in the block is new: each is worn, whole, by a template outside
/// the campaign's ids (440-479). The client ships no costume or mesh for
/// these actors, so each look was copied from one the repo already renders,
/// and a prop has no other guard (the Debug Area lineup covers characters
/// only). A look is the body set, the components in any order, the two
/// colours, the skin tint and the static mesh, with NULL and '' the same.
///
/// Revert proof: change a row's `static_mesh` or a component to a value no
/// other template uses and this names it. To give a row a look of its own,
/// prove it in a client first, then exempt the row here by id.
#[tokio::test]
async fn live_db_every_dakara_e1_look_is_one_another_template_wears() {
    let pool = require_db_or_skip!();
    block_templates(&pool).await;
    let unproven: Vec<(i32, String)> = sqlx::query_as(
        "WITH look AS ( \
           SELECT template_id, template_name::text AS template_name, body_set, \
                  (SELECT array_agg(c ORDER BY c) FROM unnest(components) c) AS comps, \
                  primary_color_id, secondary_color_id, skin_tint, \
                  coalesce(static_mesh, '') AS mesh \
           FROM resources.entity_templates) \
         SELECT s.template_id, s.template_name FROM look s \
         WHERE s.template_id BETWEEN $1 AND $2 \
           AND NOT EXISTS ( \
             SELECT 1 FROM look o WHERE o.template_id NOT BETWEEN $1 AND $3 \
               AND (o.body_set, o.comps, o.primary_color_id, o.secondary_color_id, \
                    o.skin_tint, o.mesh) IS NOT DISTINCT FROM \
                   (s.body_set, s.comps, s.primary_color_id, s.secondary_color_id, \
                    s.skin_tint, s.mesh)) \
         ORDER BY s.template_id",
    )
    .bind(CAST_AND_PROPS.0)
    .bind(CAST_AND_PROPS.1)
    .bind(479_i32)
    .fetch_all(&pool)
    .await
    .expect("look query must succeed");
    assert!(
        unproven.is_empty(),
        "templates whose look no template outside 440-479 wears: {unproven:?}"
    );
}

/// The four tent flaps carry `INT_MissionWorldObject` on the template, so
/// they are clickable for everyone from spawn; every other row carries no
/// interaction bit, because a mission packet raises it per player. The bit
/// is only a cursor: no flap becomes a vendor, a banker or a DHD.
///
/// Revert proof: zero a flap's `interaction_type`, or put a bit on a mission
/// prop, and this names the template.
#[tokio::test]
async fn live_db_dakara_e1_tent_flaps_are_clickable_from_spawn_and_nothing_else_is() {
    let pool = require_db_or_skip!();
    let block = block_templates(&pool).await;
    let mut flaps = 0;
    for (id, _, moniker) in ROSTER {
        let record = &block[&id];
        let want = if moniker.contains(TENT_FLAP) {
            flaps += 1;
            INT_MISSION_WORLD_OBJECT
        } else {
            0
        };
        assert_eq!(
            record.interaction_type, want,
            "template {id} ({moniker}): interaction_type"
        );
        let (mgr, _, npc) = spawned(record);
        let e = mgr.get_entity(npc).expect("spawned NPC");
        assert_eq!(e.interaction_type_flags, want, "template {id}: spawned");
        assert_eq!(
            e.interaction_type, None,
            "template {id} ({moniker}) opens no static interaction"
        );
    }
    assert_eq!(
        flaps, 4,
        "to and from the command tent and Moh'katan's tent"
    );
}
