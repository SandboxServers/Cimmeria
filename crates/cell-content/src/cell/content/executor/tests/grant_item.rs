//! `Action::GrantItem` for a gun: the executor only asks the base, and never
//! writes the cell's bandolier itself (CS-01b review finding 1).

use super::*;
use cimmeria_entity::cell_entity::BandolierItem;

/// A gun reward granted while the active bandolier slot already holds a
/// weapon must leave that slot alone.
///
/// Bug shape: the executor used to insert the granted gun into the
/// *active* slot as a guess, while the base stores it in the first free
/// slot. With the pistol in slot 0 and an SMG reward, the cell believed
/// slot 0 was an SMG while the client showed the pistol, and the pistol's
/// rounds were never saved again. The base's `UpdateBandolierItem` names
/// the real slot; nothing else may.
#[tokio::test]
async fn gun_grant_leaves_the_occupied_active_slot_alone() {
    const PISTOL: i32 = 55;
    const SMG: i32 = 21;

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    mgr.item_defs.insert(
        SMG,
        crate::cell::spawner::WeaponDef {
            clip_size: 30,
            default_ammo_type: 1,
            allowed_ammo_types: Vec::new(),
            holster_animation_duration: std::time::Duration::from_millis(1000),
        },
    );
    mgr.item_containers.insert(SMG, 3);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(42);
        e.active_bandolier_slot = 0;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                instance_id: 7001,
                item_id: PISTOL,
                clip_size: 15,
                default_ammo_type: 1,
                current_ammo: 7,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_ammo_dirty.clear();
    }

    let (tx, mut rx) = mpsc::channel(8);
    let resolved = ResolvedActions {
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
        actions: vec![(
            3008,
            Action::GrantItem {
                item_id: SMG,
                count: 1,
                container_id: Some(3),
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &ChainEngine::new()).await;

    let e = mgr.get_entity(1).unwrap();
    let slot0 = &e.bandolier_items[&0];
    assert_eq!(
        (slot0.instance_id, slot0.item_id, slot0.current_ammo),
        (7001, PISTOL, 7),
        "the active slot still holds the pistol with its 7 rounds",
    );
    assert_eq!(
        e.bandolier_items.len(),
        1,
        "the grant adds no cell slot itself"
    );
    assert!(
        e.bandolier_ammo_dirty.is_empty(),
        "no slot is marked for an ammo persist",
    );
    assert!(
        matches!(
            rx.try_recv(),
            Ok(CellToBaseMsg::GrantItem {
                item_id: SMG,
                container_id: 3,
                ..
            })
        ),
        "the grant goes to the base, which picks the slot",
    );
}
