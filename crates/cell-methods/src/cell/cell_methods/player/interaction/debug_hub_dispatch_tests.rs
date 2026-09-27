//! Live-DB end-to-end guards for the Castle_CellBlock stasis-room debug hub:
//! each hub NPC, built from its real seed row by the startup spawn path,
//! answers a right-click (`interact`, driven through the player dispatcher
//! from the wire's method index) with the interaction it exists to test.
//!
//! | NPC       | template | expected answer                                |
//! |-----------|----------|------------------------------------------------|
//! | vendor    | 300      | `NpcInteractionType::Vendor` → `OpenVendorStore` |
//! | trainer   | 301      | `onTrainerOpen`, and respec then passes the pin |
//! | dialog    | 302      | chain 7001 → `onDialogDisplay` 100100            |
//! | Livewire  | 303      | chain 7004 → `StartMinigame(Livewire)`           |
//! | loot crate| 304      | alive: combat reroute; dead: `onLootDisplay` with table 3 |
//! | pet trainer | 360    | `onTrainerOpen` with list 350 (pets campaign PT-07) |
//! | Banker    | 370      | `onVaultOpen` and a personal vault session (bank-vault BV-04) |
//!
//! The vendor row is the one that regressed silently: nothing ever set
//! `NpcInteractionType::Vendor`, so a vendor-only template reached the `None`
//! arm of `handle_interact` and the click did nothing. Every template here
//! goes through `spawn_npc_from_record`, so the derivation is exercised
//! exactly as a running cell does it.

use std::collections::HashSet;

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;

use crate::cell::cell_methods::player::{dispatch, INTERACT, RESET_MY_ABILITIES};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER: u32 = 1;
const PLAYER_ID: i32 = 4242;

const ON_TARGET_UPDATE: u16 = 16;
const ON_DIALOG_DISPLAY: u16 = 105;
const ON_TRAINER_OPEN: u16 = 113;
const ON_LOOT_DISPLAY: u16 = 114;
const ON_VAULT_OPEN: u16 = cimmeria_wire::cell::client_methods::player::ON_VAULT_OPEN;

/// The startup caches the hub needs, read with the cell's own loaders. A
/// macro rather than a function because this crate has no `sqlx`
/// dependency to name the pool type with.
macro_rules! load_hub {
    ($pool:expr) => {
        HubSeed {
            template_trainer_lists: spawner::load_template_trainer_lists(&$pool)
                .await
                .expect("template trainer lists must load"),
            trainer_abilities: spawner::load_trainer_abilities(&$pool)
                .await
                .expect("trainer abilities must load"),
            loot_tables: spawner::load_loot_tables(&$pool)
                .await
                .expect("loot tables must load"),
            records: spawner::load_spawns_from_db(&$pool)
                .await
                .expect("spawns must load"),
        }
    };
}

struct HubSeed {
    template_trainer_lists: std::collections::HashMap<i32, i32>,
    trainer_abilities: std::collections::HashMap<(i32, i32), Vec<i32>>,
    loot_tables: std::collections::HashMap<i32, Vec<spawner::LootTableEntry>>,
    records: Vec<spawner::SpawnRecord>,
}

/// The hub, spawned exactly as the cell spawns it at startup, plus the
/// player. Returns the manager and each hub NPC's entity id by tag.
fn staged_hub(seed: HubSeed) -> (SpaceManager, Vec<(String, u32)>) {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    mgr.template_trainer_lists = seed.template_trainer_lists;
    mgr.trainer_abilities = seed.trainer_abilities;
    mgr.loot_tables = seed.loot_tables;
    let records = seed.records;
    let mut hub = Vec::new();
    for record in records
        .iter()
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with("DebugHub_")))
    {
        let eid = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(eid, record)
            .expect("hub NPC must spawn from its record");
        hub.push((record.tag.clone().unwrap(), eid));
    }
    assert_eq!(
        hub.len(),
        7,
        "the hub seeds seven NPCs (#846's five, the PT-07 pet trainer and the BV-04 Banker): {hub:?}"
    );

    // Any archetype list 1 offers, so the trainer has something to show.
    let archetype = mgr
        .trainer_abilities
        .keys()
        .find(|(list, _)| *list == 1)
        .map(|(_, arch)| *arch)
        .expect("trainer list 1 must offer abilities");
    mgr.create_entity(
        PLAYER,
        "Castle_CellBlock",
        [-334.231, 73.472, -228.026],
        [0.0; 3],
    )
    .expect("the player must stage at the stasis respawner");
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.archetype_id = Some(archetype);
    mgr.connect_entity(PLAYER);
    (mgr, hub)
}

fn eid_of(hub: &[(String, u32)], tag: &str) -> u32 {
    hub.iter()
        .find(|(t, _)| t == tag)
        .unwrap_or_else(|| panic!("hub NPC {tag} must be staged"))
        .1
}

/// Step the player next to `npc` (inside `MAX_INTERACT_DISTANCE`) and
/// right-click it through the wire dispatcher.
async fn click(mgr: &mut SpaceManager, engine: &ChainEngine, npc: u32) -> Vec<CellToBaseMsg> {
    let at = mgr.get_entity(npc).unwrap().position;
    mgr.update_entity_position(PLAYER, [at.x + 1.0, at.y, at.z], [0; 3], [0.0; 3]);
    let (tx, mut rx) = mpsc::channel(64);
    assert!(
        dispatch(
            PLAYER,
            INTERACT,
            &(npc as i32).to_le_bytes(),
            &tx,
            mgr,
            engine
        )
        .await
    );
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

fn methods(msgs: &[CellToBaseMsg]) -> Vec<u16> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } if *entity_id == PLAYER => Some(*method_index),
            _ => None,
        })
        .collect()
}

fn opened_store(msgs: &[CellToBaseMsg]) -> Vec<Option<i32>> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::OpenVendorStore {
                vendor_template_id, ..
            } => Some(*vendor_template_id),
            _ => None,
        })
        .collect()
}

/// The vendor, trainer, dialog NPC and Livewire terminal each answer a
/// click with their own interaction and nobody else's.
#[tokio::test]
async fn debug_hub_npcs_answer_a_click_with_their_own_interaction() {
    let pool = require_db_or_skip!();
    let (mut mgr, hub) = staged_hub(load_hub!(pool));
    let engine = crate::cell::content::build_engine(Some(&pool)).await;

    // Vendor: the spawn path derived Vendor from INT_VendorGeneral, and the
    // click opens template 300's store.
    let vendor = eid_of(&hub, "DebugHub_Vendor");
    assert_eq!(
        mgr.get_entity(vendor).unwrap().interaction_type,
        Some(NpcInteractionType::Vendor),
        "template 300's vendor bit must make it a Vendor at spawn"
    );
    let msgs = click(&mut mgr, &engine, vendor).await;
    assert_eq!(
        opened_store(&msgs),
        vec![Some(300)],
        "the vendor must open exactly one store, its own; got {msgs:?}"
    );
    assert_eq!(mgr.get_entity(PLAYER).unwrap().vendor_entity, Some(vendor));

    // Trainer: onTrainerOpen, no store.
    let trainer = eid_of(&hub, "DebugHub_Trainer");
    let msgs = click(&mut mgr, &engine, trainer).await;
    assert_eq!(methods(&msgs), vec![ON_TRAINER_OPEN], "trainer: {msgs:?}");
    assert!(opened_store(&msgs).is_empty());

    // Dialog NPC: chain 7001 shows dialog 100100, spoken by that NPC.
    let dialog_npc = eid_of(&hub, "DebugHub_DialogNpc");
    let msgs = click(&mut mgr, &engine, dialog_npc).await;
    let displays: Vec<(i32, i32)> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index: ON_DIALOG_DISPLAY,
                args,
                ..
            } => Some((
                i32::from_le_bytes(args[0..4].try_into().unwrap()),
                i32::from_le_bytes(args[4..8].try_into().unwrap()),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        displays,
        vec![(dialog_npc as i32, 100100)],
        "dialog NPC: {msgs:?}"
    );

    // Livewire terminal: chain 7004 starts one Livewire session.
    let terminal = eid_of(&hub, "DebugHub_LivewireTerminal");
    let msgs = click(&mut mgr, &engine, terminal).await;
    let starts: Vec<(&str, &[i64])> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::StartMinigame {
                game_name,
                on_victory_chains,
                ..
            } => Some((game_name.as_str(), on_victory_chains.as_slice())),
            _ => None,
        })
        .collect();
    assert_eq!(
        starts,
        vec![("Livewire", &[7005i64][..])],
        "terminal: {msgs:?}"
    );

    // None of the four is a vendor except the vendor.
    for npc in [trainer, dialog_npc, terminal] {
        assert_eq!(mgr.get_entity(npc).unwrap().interaction_type, None);
    }
}

/// The crate is a combat target while alive (the click becomes an attack),
/// and after it dies its corpse carries every loot table 3 row, which a
/// click displays. Dying goes through the same resolver a GM `.kill` and a
/// killing shot use, so the loot comes from the real roll.
#[tokio::test]
async fn debug_hub_crate_is_shot_then_looted_from_table_3() {
    let pool = require_db_or_skip!();
    let (mut mgr, hub) = staged_hub(load_hub!(pool));
    let engine = ChainEngine::new();
    let crate_eid = eid_of(&hub, "DebugHub_LootCrate");

    let msgs = click(&mut mgr, &engine, crate_eid).await;
    assert!(
        methods(&msgs).contains(&ON_TARGET_UPDATE),
        "a live crate is a combat target: the click must reroute to an attack; got {msgs:?}"
    );
    assert!(!methods(&msgs).contains(&ON_LOOT_DISPLAY));

    let (tx, _rx) = mpsc::channel(256);
    assert!(
        crate::cell::abilities::kill_npc_out_of_band(crate_eid, PLAYER, true, false, &tx, &mut mgr)
            .await,
        "the crate must die through the shared death resolver"
    );
    let dropped: HashSet<Option<i32>> = mgr
        .get_entity(crate_eid)
        .unwrap()
        .loot
        .iter()
        .map(|l| l.design_id)
        .collect();
    let table: HashSet<Option<i32>> = mgr.loot_tables[&3].iter().map(|e| e.design_id).collect();
    assert_eq!(
        dropped, table,
        "every table-3 row is probability 1, so the corpse carries all of them"
    );

    let msgs = click(&mut mgr, &engine, crate_eid).await;
    assert_eq!(
        methods(&msgs),
        vec![ON_LOOT_DISPLAY],
        "a dead crate with loot opens the loot window; got {msgs:?}"
    );
    assert_eq!(
        mgr.get_entity(PLAYER).unwrap().looting_entity,
        Some(crate_eid)
    );
}

/// AT-08's respec gate accepts the hub trainer as the pinned trainer: after
/// a click on template 301 pins it, `resetMyAbilities` (method 72) reaches
/// the base. Pinned to the vendor instead, the same call is refused, so the
/// pass is the trainer's doing and not a gate that lets everything through.
#[tokio::test]
async fn debug_hub_trainer_pin_passes_the_respec_gate() {
    let pool = require_db_or_skip!();
    let (mut mgr, hub) = staged_hub(load_hub!(pool));
    let engine = ChainEngine::new();
    let trained = {
        let p = mgr.get_entity_mut(PLAYER).unwrap();
        p.tree_progress.trained_abilities = vec![598];
        p.tree_progress.tree_points_spent = 1;
        p.archetype_id
    };
    assert!(trained.is_some());

    async fn respec(mgr: &mut SpaceManager, engine: &ChainEngine) -> Vec<CellToBaseMsg> {
        let (tx, mut rx) = mpsc::channel(16);
        assert!(dispatch(PLAYER, RESET_MY_ABILITIES, &[], &tx, mgr, engine).await);
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            out.push(msg);
        }
        out
    }
    let resets = |msgs: &[CellToBaseMsg]| {
        msgs.iter()
            .filter(|m| {
                matches!(
                    m,
                    CellToBaseMsg::ResetAbilities {
                        player_id: PLAYER_ID,
                        ..
                    }
                )
            })
            .count()
    };

    click(&mut mgr, &engine, eid_of(&hub, "DebugHub_Vendor")).await;
    let refused = respec(&mut mgr, &engine).await;
    assert_eq!(
        resets(&refused),
        0,
        "pinned to the vendor, respec must be refused"
    );

    click(&mut mgr, &engine, eid_of(&hub, "DebugHub_Trainer")).await;
    let accepted = respec(&mut mgr, &engine).await;
    assert_eq!(
        resets(&accepted),
        1,
        "pinned to the hub trainer, respec must reach the base; got {accepted:?}"
    );
}

/// The PT-07 pet trainer answers a click with `onTrainerOpen` listing
/// list 350: the pet nodes for a Goa'uld, nothing for any other archetype
/// (the list is keyed to the Goa'uld only, the one tree that holds them).
#[tokio::test]
async fn debug_hub_pet_trainer_opens_list_350() {
    let pool = require_db_or_skip!();
    let (mut mgr, hub) = staged_hub(load_hub!(pool));
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let pet_trainer = eid_of(&hub, "DebugHub_PetTrainer");
    let goauld = mgr
        .trainer_abilities
        .keys()
        .find(|(list, _)| *list == 350)
        .map(|(_, arch)| *arch)
        .expect("trainer list 350 must be seeded");

    // `onTrainerOpen`: INT32 trainer id, UINT32 count, count x (INT32 id,
    // UINT8 trainable), INT32 respec cost.
    let offered = |msgs: &[CellToBaseMsg]| -> Vec<i32> {
        let args = msgs
            .iter()
            .find_map(|m| match m {
                CellToBaseMsg::EntityMethodCall {
                    method_index: ON_TRAINER_OPEN,
                    args,
                    ..
                } => Some(args.clone()),
                _ => None,
            })
            .expect("onTrainerOpen");
        let count = u32::from_le_bytes(args[4..8].try_into().unwrap()) as usize;
        let mut ids: Vec<i32> = (0..count)
            .map(|i| i32::from_le_bytes(args[8 + i * 5..12 + i * 5].try_into().unwrap()))
            .collect();
        ids.sort_unstable();
        ids
    };

    mgr.get_entity_mut(PLAYER).unwrap().archetype_id = Some(goauld);
    let msgs = click(&mut mgr, &engine, pet_trainer).await;
    assert_eq!(
        methods(&msgs),
        vec![ON_TRAINER_OPEN],
        "pet trainer: {msgs:?}"
    );
    assert_eq!(offered(&msgs), vec![1643, 1644, 1645, 1652, 1654, 2826]);

    mgr.get_entity_mut(PLAYER).unwrap().archetype_id = Some(goauld + 1);
    let msgs = click(&mut mgr, &engine, pet_trainer).await;
    assert!(offered(&msgs).is_empty(), "no pet nodes for a non-Goa'uld");
}

/// The BV-04 Banker, spawned from its real row, answers a click with one
/// `onVaultOpen` naming itself and pins a personal vault session on it.
/// Nothing that runs before the static dispatch (a trainer list, a chain on
/// its tag or template, a dialog bind) may claim the click: the real content
/// engine is loaded, so a chain added on `DebugHub_Banker` fails this.
#[tokio::test]
async fn debug_hub_banker_opens_the_personal_vault() {
    use cimmeria_entity::cell_entity::VaultScope;

    let pool = require_db_or_skip!();
    let (mut mgr, hub) = staged_hub(load_hub!(pool));
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let banker = eid_of(&hub, "DebugHub_Banker");
    assert_eq!(
        mgr.get_entity(banker).unwrap().interaction_type,
        Some(NpcInteractionType::Banker {
            scope: VaultScope::Personal
        }),
        "template 370's INT_Banker bit must make it a personal Banker at spawn"
    );

    let msgs = click(&mut mgr, &engine, banker).await;
    let opens: Vec<i32> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: PLAYER,
                method_index: ON_VAULT_OPEN,
                args,
            } => Some(i32::from_le_bytes(args[0..4].try_into().unwrap())),
            _ => None,
        })
        .collect();
    assert_eq!(
        opens,
        vec![banker as i32],
        "one onVaultOpen naming the Banker; got {msgs:?}"
    );
    assert!(opened_store(&msgs).is_empty());
    let session = mgr
        .get_entity(PLAYER)
        .unwrap()
        .vault_session
        .clone()
        .expect("the click opens a vault session");
    assert_eq!(session.banker_id, Some(banker));
    assert_eq!(session.scope, VaultScope::Personal);
}
