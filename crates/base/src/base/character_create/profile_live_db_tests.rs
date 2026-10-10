//! Live-DB guards for the start profiles (Class Start v6, CS-02): every
//! char_def is created through the real handler with exactly its matrix row,
//! and a profile whose world the cell never announced refuses and leaves
//! nothing behind (lock L3).

use std::collections::BTreeSet;
use std::net::SocketAddr;
use std::sync::Arc;

use super::live_db_tests::{
    build_create_character_payload, cleanup, insert_account, make_connected,
};
use super::seed_parity_live_db_tests::{char_defs, create, default_choices};
use super::*;
use crate::test_support::{require_db_or_skip, TestTransport};

/// Sentinels after `seed_parity_live_db_tests`' `0x7000_1E01` and
/// `kit_rollback`'s `0x7000_1E02`.
const MATRIX_ACCOUNT: i32 = 0x7000_1E03;
const UNKNOWN_WORLD_ACCOUNT: i32 = 0x7000_1E04;
/// After `fail_code_tests`' `0x7000_1E05`.
const FREE_JAFFA_ACCOUNT: i32 = 0x7000_1E06;

/// Standard Chestplate, the Free Jaffa's starting armour.
const STANDARD_CHESTPLATE: i32 = 4342;
const INV_MAIN: i32 = 1;
const INV_CHEST: i32 = 7;

const INV_BANDOLIER: i32 = 3;

/// What one char_def must start as (the ledger's Profiles, Abilities and
/// Gear matrices).
struct Want {
    world: &'static str,
    abilities: Vec<i32>,
    /// `(ability_id, source_kind)` provenance rows.
    grants: Vec<(i32, &'static str)>,
    /// Kit items (besides the visual-choice clothes).
    items: Vec<i32>,
}

fn want(char_def_id: i32) -> Want {
    let legacy = vec![592, 594, 597, 1218, 1646];
    match char_def_id {
        // SGU_FREE_JAFFA
        8 | 18 => Want {
            world: "Dakara_E1",
            abilities: vec![597, 1218, 1984],
            grants: vec![
                (597, "racial_core"),
                (1218, "racial_core"),
                (1984, "signature"),
            ],
            // 4342 comes from the forced Torso choice, not the profile;
            // `free_jaffa_wears_exactly_one_chestplate_live_db` pins it.
            items: vec![2797],
        },
        // SGU_ASGARD holding state (OD-CS09)
        9 => Want {
            world: "SGC_W1",
            abilities: legacy,
            grants: vec![],
            items: vec![55],
        },
        // PRA_GOAULD holding state (OD-CS08)
        10 | 19 => Want {
            world: "Castle_CellBlock",
            abilities: legacy,
            grants: vec![],
            items: vec![55],
        },
        // SGU humans
        2 | 12 | 4 | 14 | 21 | 23 | 6 | 16 => Want {
            world: "SGC_W1",
            abilities: vec![],
            grants: vec![],
            items: vec![],
        },
        // Praxis humans and Loyalist Jaffa
        _ => Want {
            world: "Castle_CellBlock",
            abilities: vec![],
            grants: vec![],
            items: vec![],
        },
    }
}

/// **Regression guard (CS-02 matrix).** Every char_def, created through the
/// real handler, starts in its profile's world at its point, at level 1 with
/// one training and one Applied Science point, knows exactly its matrix
/// abilities (no 592 for a normal human or Loyalist Jaffa), carries exactly
/// its provenance rows, holds its kit items with every gun at 0 rounds, and
/// is not a debug-kit character.
///
/// Reverting the seed's universal kit fails `abilities`; reverting the Free
/// Jaffa row to SGC_W1 fails `world`; dropping `record_profile_grants` fails
/// `grants`; reverting the creation ammo to `clip_size` fails `ammo`.
#[tokio::test]
async fn every_char_def_starts_with_its_matrix_row_live_db() {
    let pool = require_db_or_skip!();
    cleanup(&pool, MATRIX_ACCOUNT).await;
    insert_account(&pool, MATRIX_ACCOUNT, 0).await;

    let defs = char_defs(&pool).await;
    assert_eq!(defs.len(), 23);
    for char_def_id in defs {
        let w = want(char_def_id);
        let name = format!("Profile Row {char_def_id:02}");
        let player_id = create(&pool, MATRIX_ACCOUNT, 0, char_def_id, &name, false).await;

        let (world, x, y, z, level, tp, asp, abilities, debug_kit): (
            String,
            f32,
            f32,
            f32,
            i32,
            i32,
            i32,
            Vec<i32>,
            bool,
        ) = sqlx::query_as(
            "SELECT world_location::text, pos_x, pos_y, pos_z, level, training_points, \
                    applied_science_points, abilities, debug_kit \
               FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .expect("read created player");
        let (pw, px, py, pz): (String, f32, f32, f32) = sqlx::query_as(
            "SELECT starting_world::text, starting_x, starting_y, starting_z \
               FROM resources.char_creation WHERE char_def_id = $1",
        )
        .bind(char_def_id)
        .fetch_one(&pool)
        .await
        .expect("read profile");
        assert_eq!(world, w.world, "char_def {char_def_id}: world");
        assert_eq!((world.as_str(), [x, y, z]), (pw.as_str(), [px, py, pz]));
        assert_eq!(
            (level, tp, asp),
            (1, 1, 1),
            "char_def {char_def_id}: level 1"
        );
        assert!(
            !debug_kit,
            "char_def {char_def_id}: no profile sets the debug kit"
        );
        assert_eq!(abilities, w.abilities, "char_def {char_def_id}: abilities");

        let grants: Vec<(i32, String)> = sqlx::query_as(
            "SELECT ability_id, source_kind::text FROM sgw_player_ability_grants \
              WHERE player_id = $1 ORDER BY ability_id",
        )
        .bind(player_id)
        .fetch_all(&pool)
        .await
        .expect("read grants");
        let grants: Vec<(i32, &str)> = grants.iter().map(|(a, k)| (*a, k.as_str())).collect();
        assert_eq!(grants, w.grants, "char_def {char_def_id}: grants");

        let kit: Vec<(i32, i32, i32)> = sqlx::query_as(
            "SELECT i.type_id, i.container_id, i.ammo FROM sgw_inventory i \
              WHERE i.character_id = $1 AND i.type_id = ANY($2) ORDER BY i.type_id",
        )
        .bind(player_id)
        .bind(&w.items)
        .fetch_all(&pool)
        .await
        .expect("read kit items");
        let types: BTreeSet<i32> = kit.iter().map(|k| k.0).collect();
        assert_eq!(
            types,
            w.items.iter().copied().collect::<BTreeSet<_>>(),
            "char_def {char_def_id}: kit items"
        );
        let all_ammo: Vec<i32> = sqlx::query_scalar(
            "SELECT ammo FROM sgw_inventory WHERE character_id = $1 AND ammo <> 0",
        )
        .bind(player_id)
        .fetch_all(&pool)
        .await
        .expect("read ammo");
        assert!(
            all_ammo.is_empty(),
            "char_def {char_def_id}: every gun is created empty (OD-CS13), got {all_ammo:?}"
        );
        if w.items.contains(&55) || w.items.contains(&2797) {
            assert!(
                kit.iter().any(|k| k.1 == INV_BANDOLIER),
                "char_def {char_def_id}: the weapon lands in the bandolier"
            );
        }
    }
    cleanup(&pool, MATRIX_ACCOUNT).await;
}

/// **Regression guard (L3).** A profile whose world exists in
/// `resources.worlds` but has no cell space the cell announced is refused
/// with the profile-unusable code and leaves no character row. Reverting
/// the `is_world_enterable` check creates the character in a world nobody
/// can load.
#[tokio::test]
async fn a_start_world_with_no_cell_space_refuses_and_rolls_back_live_db() {
    let pool = require_db_or_skip!();
    cleanup(&pool, UNKNOWN_WORLD_ACCOUNT).await;
    insert_account(&pool, UNKNOWN_WORLD_ACCOUNT, 0).await;
    super::live_db_tests::register_start_worlds(&pool).await;

    // Egypt (world 66) is a resources.worlds row with no space this
    // server loads. Point the Praxis Commando profile at it for the run.
    let (old_world,): (String,) = sqlx::query_as(
        "SELECT starting_world::text FROM resources.char_creation WHERE char_def_id = 3",
    )
    .fetch_one(&pool)
    .await
    .expect("read profile");
    let unloaded: String =
        sqlx::query_scalar("SELECT world::text FROM resources.worlds WHERE world_id = 66")
            .fetch_one(&pool)
            .await
            .expect("world 66 is seeded");
    sqlx::query("UPDATE resources.char_creation SET starting_world = $1 WHERE char_def_id = 3")
        .bind(&unloaded)
        .execute(&pool)
        .await
        .expect("point the profile at an unloaded world");

    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55813".parse().unwrap();
    let connected = make_connected(addr, UNKNOWN_WORLD_ACCOUNT as u32, 0);
    let db_pool = Some(Arc::new(pool.clone()));
    let choices = default_choices(&pool, 3).await;
    let payload = build_create_character_payload("Nowhere Start", "", 3, &choices, 0);
    let result = handle_create_character(
        &dyn_transport,
        addr,
        [0u8; 32],
        UNKNOWN_WORLD_ACCOUNT as u32,
        &payload,
        &connected,
        &db_pool,
    )
    .await;

    // Undo before asserting, so a failure doesn't leak the change.
    sqlx::query("UPDATE resources.char_creation SET starting_world = $1 WHERE char_def_id = 3")
        .bind(&old_world)
        .execute(&pool)
        .await
        .expect("restore the profile");
    assert!(result.is_ok(), "the handler answers the client: {result:?}");
    assert_eq!(
        super::fail_code_tests::sent_message(&transport),
        super::fail_code_tests::expected(10001),
        "an unusable profile is ERROR_CharacterCreationInvalidCharacterType"
    );
    let players: i64 = sqlx::query_scalar("SELECT count(*) FROM sgw_player WHERE account_id = $1")
        .bind(UNKNOWN_WORLD_ACCOUNT)
        .fetch_one(&pool)
        .await
        .expect("count players");
    cleanup(&pool, UNKNOWN_WORLD_ACCOUNT).await;
    assert_eq!(players, 0, "no character in a world the cell cannot load");
}

/// **Regression guard (CS-08 F4).** A new Free Jaffa (char_defs 8 and 18)
/// owns exactly one Standard Chestplate, and wears it. The forced Torso
/// choice places 4342 in the Chest slot; the profile used to grant it again,
/// so a second one landed in the backpack. Re-adding the
/// `char_creation_items` row puts a `(4342, 1)` row next to `(4342, 7)` and
/// fails this.
#[tokio::test]
async fn free_jaffa_wears_exactly_one_chestplate_live_db() {
    let pool = require_db_or_skip!();
    cleanup(&pool, FREE_JAFFA_ACCOUNT).await;
    insert_account(&pool, FREE_JAFFA_ACCOUNT, 0).await;

    let mut found = Vec::new();
    for char_def_id in [8, 18] {
        let name = format!("Free Jaffa {char_def_id:02}");
        let player_id = create(&pool, FREE_JAFFA_ACCOUNT, 0, char_def_id, &name, false).await;
        let containers: Vec<i32> = sqlx::query_scalar(
            "SELECT container_id FROM sgw_inventory \
              WHERE character_id = $1 AND type_id = $2 ORDER BY container_id",
        )
        .bind(player_id)
        .bind(STANDARD_CHESTPLATE)
        .fetch_all(&pool)
        .await
        .expect("read chestplates");
        found.push((char_def_id, containers));
    }
    cleanup(&pool, FREE_JAFFA_ACCOUNT).await;

    assert_eq!(
        found,
        vec![(8, vec![INV_CHEST]), (18, vec![INV_CHEST])],
        "one Standard Chestplate, in the Chest slot (container {INV_CHEST}); \
         a second one in the backpack (container {INV_MAIN}) is the F4 duplicate"
    );
}
