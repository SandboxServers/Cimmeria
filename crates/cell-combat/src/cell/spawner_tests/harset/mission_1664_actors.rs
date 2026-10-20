//! Seed-pin guards for the two FZ-19 rows that mission 1664's chains click on:
//! template 260 (Sgt. Kaerlia, the friendly debrief NPC) and template 261 (the
//! Command Center door control), plus their spawns 360 and 380.
//!
//! Chains 6008-6016 fire on the tags `Harset_Kaerlia` and `Harset_CmdDoor`;
//! before FZ-19 nothing spawned either tag, so those clicks went nowhere.

use super::*;

/// Template 260: Sgt. Kaerlia. Named `DN_...` via moniker 27315.
const KAERLIA_TEMPLATE: i32 = 260;
/// Template 261: the Command Center door control.
const DOOR_CONTROL_TEMPLATE: i32 = 261;
/// Template 22: the wall-terminal mesh that Cellblock's panels use. The door
/// control borrows its mesh but must never borrow its cursor.
const CELLBLOCK_PANEL_TEMPLATE: i32 = 22;
/// Spawn 360: Kaerlia by the Stargate.
const KAERLIA_SPAWN: i32 = 360;
/// Spawn 380: the Command Center door control.
const DOOR_CONTROL_SPAWN: i32 = 380;

/// Template 260's columns as the friendly-NPC guard reads them: class,
/// faction, interaction_type, speaker_id, name_id, the moniker text, then
/// whether buy, sell, loot and trainer lists are NULL.
type KaerliaRow = (
    Option<String>,
    Option<i32>,
    Option<i64>,
    Option<i32>,
    Option<i32>,
    Option<String>,
    Option<bool>,
    Option<bool>,
    Option<bool>,
    Option<bool>,
);

/// **Friendly-NPC guard.** Template 260 is a `mob` on faction 1 with the
/// talk cursor, the Sgt. Kaerlia name and speaker, and no trade or loot lists.
///
/// Revert proof: reverting the seed row (or changing `interaction_type` away
/// from `134217728`, or `name_id` away from `27315`) fails the per-column
/// asserts below; dropping the row panics on the "must be seeded" check.
#[tokio::test]
async fn live_db_kaerlia_template_260_is_the_friendly_debrief_npc() {
    let pool = require_db_or_skip!();

    let row: Option<KaerliaRow> = sqlx::query_as(
        "SELECT t.class, t.faction, t.interaction_type, t.speaker_id, t.name_id, x.text, \
                t.buy_item_list IS NULL, t.sell_item_list IS NULL, \
                t.loot_table_id IS NULL, t.trainer_ability_list_id IS NULL \
         FROM resources.entity_templates t \
         LEFT JOIN resources.texts x ON x.moniker_id = t.name_id \
         WHERE t.template_id = $1",
    )
    .bind(KAERLIA_TEMPLATE)
    .fetch_optional(&pool)
    .await
    .expect("query must succeed");

    let (class, faction, interaction, speaker, name_id, name_text, buy, sell, loot, trainer) =
        row.unwrap_or_else(|| panic!("template {KAERLIA_TEMPLATE} must be seeded"));

    assert_eq!(class.as_deref(), Some("mob"), "Kaerlia must be a mob");
    assert_eq!(faction, Some(1), "Kaerlia must be on faction 1");
    assert_eq!(
        interaction,
        Some(134_217_728),
        "Kaerlia needs INT_NonAStoryMissionAvaliable (134217728) so the chain's cue has a talk cursor"
    );
    assert_eq!(speaker, Some(3219), "Kaerlia must speak as speaker 3219");
    assert_eq!(name_id, Some(27315), "Kaerlia must carry name 27315");
    assert_eq!(
        name_text.as_deref(),
        Some("Sgt. Kaerlia"),
        "moniker 27315 must resolve to 'Sgt. Kaerlia'"
    );
    assert_eq!(buy, Some(true), "Kaerlia must have no buy list");
    assert_eq!(sell, Some(true), "Kaerlia must have no sell list");
    assert_eq!(
        loot,
        Some(true),
        "Kaerlia must have no loot table (spec L-01)"
    );
    assert_eq!(
        trainer,
        Some(true),
        "Kaerlia must have no trainer ability list"
    );
}

/// **Shared-cursor guard.** Template 261 carries no interaction flags of its
/// own: the door cue comes only from the per-player dsm 120301 bind. Template
/// 22 (Cellblock's panels) also keeps `interaction_type` 0, so this guard is
/// not the only thing holding Cellblock's panels clean.
///
/// Revert proof: setting template 261's `interaction_type` to `1073741824`
/// fails the `interaction_type` assert; changing 261's mesh or class fails the
/// others.
#[tokio::test]
async fn live_db_door_control_template_261_has_no_shared_cursor() {
    let pool = require_db_or_skip!();

    let row: Option<(Option<String>, Option<i64>, Option<String>, Option<i32>)> = sqlx::query_as(
        "SELECT class, interaction_type, static_mesh, respawn_secs \
         FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(DOOR_CONTROL_TEMPLATE)
    .fetch_optional(&pool)
    .await
    .expect("query must succeed");

    let (class, interaction, mesh, respawn) =
        row.unwrap_or_else(|| panic!("template {DOOR_CONTROL_TEMPLATE} must be seeded"));

    assert_eq!(
        class.as_deref(),
        Some("being"),
        "the door control must be a being"
    );
    assert_eq!(
        interaction,
        Some(0),
        "the door control must carry no shared interaction flags; the cursor comes from dsm 120301"
    );
    assert_eq!(
        mesh.as_deref(),
        Some("GLB-Global.GLB-RingTransporterSwitch_00"),
        "the door control must use the ring-transporter switch mesh"
    );
    assert_eq!(respawn, None, "the door control must not respawn");

    let cellblock_interaction: Option<i64> = sqlx::query_scalar(
        "SELECT interaction_type FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(CELLBLOCK_PANEL_TEMPLATE)
    .fetch_one(&pool)
    .await
    .expect("query must succeed");

    assert_eq!(
        cellblock_interaction,
        Some(0),
        "template 22 (Cellblock's panels) must keep interaction_type 0"
    );
}

/// **Spawn-tag guard.** Spawns 360 and 380 stand in world 57 with their
/// chain tags, the right template, and `is_stationary = true`. Each tag must
/// sit on exactly one spawn row in the whole table, byte-exact, because
/// `fire_interact_tag` matches byte-exactly.
///
/// Revert proof: renaming spawn 360's tag to `Harset_Kaerlia2` fails the tag
/// assert and leaves `Harset_Kaerlia` on zero rows; duplicating the tag on
/// another spawn fails the count assert.
#[tokio::test]
async fn live_db_mission_1664_spawns_stand_in_harset_with_their_tags() {
    let pool = require_db_or_skip!();

    let expectations: [(i32, &str, i32); 2] = [
        (KAERLIA_SPAWN, "Harset_Kaerlia", KAERLIA_TEMPLATE),
        (DOOR_CONTROL_SPAWN, "Harset_CmdDoor", DOOR_CONTROL_TEMPLATE),
    ];

    for (spawn_id, tag, template_id) in expectations {
        let row: Option<(Option<i32>, Option<String>, Option<i32>, Option<bool>)> = sqlx::query_as(
            "SELECT world_id, tag, template_id, is_stationary \
             FROM resources.spawnlist WHERE spawn_id = $1",
        )
        .bind(spawn_id)
        .fetch_optional(&pool)
        .await
        .expect("query must succeed");

        let (world, actual_tag, actual_template, stationary) =
            row.unwrap_or_else(|| panic!("spawn {spawn_id} must be seeded"));

        assert_eq!(
            world,
            Some(HARSET_WORLD_ID),
            "spawn {spawn_id} must be in world 57"
        );
        assert_eq!(
            actual_tag.as_deref(),
            Some(tag),
            "spawn {spawn_id} must carry the tag '{tag}'"
        );
        assert_eq!(
            actual_template,
            Some(template_id),
            "spawn {spawn_id} must use template {template_id}"
        );
        assert_eq!(
            stationary,
            Some(true),
            "spawn {spawn_id} must be stationary"
        );

        let holders: i64 =
            sqlx::query_scalar("SELECT count(*) FROM resources.spawnlist WHERE tag = $1")
                .bind(tag)
                .fetch_one(&pool)
                .await
                .expect("query must succeed");

        assert_eq!(
            holders, 1,
            "tag '{tag}' must sit on exactly one spawnlist row; fire_interact_tag matches byte-exactly"
        );
    }
}
