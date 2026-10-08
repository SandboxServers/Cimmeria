//! Dakara_E1 (world 61) arrival chains, Dakara rebuild DK-01: chain-replay
//! guards for chains 8001-8002 in
//! `db/resources/Content/Seed/dakara_e1_space_chains.sql`.
//!
//! * 8001: a Free Jaffa (archetype 7, `ARCHETYPE_Sholva`) who enters
//!   Dakara_E1 learns the Omega Site address (stargate 5), so the DHD on the
//!   gate plaza has somewhere to dial.
//! * 8002: the same arrival sends the one-time arrival notice, a
//!   `send_system_mail` behind a quiet cooldown of `i32::MAX` seconds.
//!
//! Every test loads the **whole** seeded engine (`build_engine`), not the
//! two chains alone, so "resolves exactly these actions" and "resolves
//! nothing" are statements about every chain in the seed: a later chain
//! that fires for a missionless Free Jaffa on arrival, or one that grants
//! an address to a visitor, fails here and has to be added on purpose.
//!
//! The executed tests go through the real dispatcher,
//! [`fire_player_loaded`], because the world and the archetype reach the
//! conditions through its context population, and through
//! `execute_actions`, because a resolve-only test cannot tell a wired
//! executor arm from the catch-all.
//!
//! Neither chain sets an interaction bit or binds a dialog set, so there is
//! no `player_loaded` restore chain to guard. The base half of the notice
//! (the claim, the mail row and the silent repeat) is guarded in
//! `cimmeria-base-methods` `mail/tests/content_live.rs`; the gate's arrival
//! pin and the way back in `cimmeria-cell-world` `gate_dakara_e1_tests.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::build_engine;
use super::super::fire_player_loaded;
use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::messages::{CellToBaseMsg, ContentMailCooldown, ContentSystemMail};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner::load_stargates;
use crate::test_support::require_db_or_skip;

const DAKARA: &str = "Dakara_E1";
const GRANT_CHAIN: i64 = 8001;
const NOTICE_CHAIN: i64 = 8002;
/// `resources.stargates.stargate_id` of Omega Site (OD-DK02's default).
const OMEGA_SITE_GATE: i32 = 5;
/// `ARCHETYPE_Sholva`: the SGU Jaffa of char_defs 8 and 18, the Free Jaffa.
const FREE_JAFFA: i32 = 7;
/// `ARCHETYPE_Jaffa`: the Praxis Jaffa, the archetype most easily confused
/// with it.
const PRAXIS_JAFFA: i32 = 8;

const PLAYER_EID: u32 = 8101;
const PLAYER_ID: i32 = 8102;
const ACCOUNT_ID: u32 = 8103;

/// The notice as chain 8002 seeds it (NEW CONTENT).
fn notice_action() -> Action {
    Action::SendSystemMail {
        sender_name: "Dakara Gate Watch".into(),
        subject: "Dakara: no orders yet".into(),
        body: "The Free Jaffa command on Dakara is not staffed yet, so there are no orders \
               for you here. The DHD in front of the Stargate dials Omega Site. The DHD at \
               Omega Site brings you back to Dakara."
            .into(),
        cash: 0,
        item: None,
        cooldown_secs: Some(i32::MAX as u32),
        quiet_cooldown: true,
    }
}

/// Resolve `player_loaded` on `world` for a character with no mission, the
/// way `fire_player_loaded` populates it. `archetype` `None` is an entity
/// whose archetype was never stamped.
fn arrive(engine: &ChainEngine, world: &str, archetype: Option<i32>) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("world_name".to_string(), serde_json::json!(world));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    let event = TriggerEvent {
        trigger_type: TriggerType::PlayerLoaded,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// A resident Dakara_E1 space with the real stargate cache, and one player
/// entity of `archetype` standing on the gate plaza.
async fn staged_space(pool: &PgPool, entity_id: u32, archetype: Option<i32>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Dakara_E1" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Dakara_E1" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    // The executor checks the granted id against this cache, so it is the
    // seeded table, not a fixture row.
    mgr.stargates = load_stargates(pool).await.expect("load_stargates");
    stage_player(&mut mgr, entity_id, archetype);
    mgr
}

fn stage_player(mgr: &mut SpaceManager, entity_id: u32, archetype: Option<i32>) {
    mgr.create_entity(entity_id, DAKARA, [96.87, -16.75, 243.23], [0.0; 3])
        .expect("the player must stage");
    let p = mgr.get_entity_mut(entity_id).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.account_id = Some(ACCOUNT_ID);
    p.archetype_id = archetype;
    mgr.connect_entity(entity_id);
}

/// Fire the real `player_loaded` dispatcher and return what it sent.
async fn enter_world(
    engine: &ChainEngine,
    mgr: &mut SpaceManager,
    entity_id: u32,
    world: &str,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    fire_player_loaded(entity_id, PLAYER_ID, world, engine, &tx, mgr).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// The one message chain 8002 sends the base for `entity_id`.
fn notice_mail(entity_id: u32) -> ContentSystemMail {
    let Action::SendSystemMail {
        sender_name,
        subject,
        body,
        ..
    } = notice_action()
    else {
        unreachable!()
    };
    ContentSystemMail {
        entity_id,
        player_id: PLAYER_ID,
        account_id: Some(ACCOUNT_ID),
        chain_id: NOTICE_CHAIN,
        sender_name,
        subject,
        body,
        cash: 0,
        item: None,
        cooldown: Some(ContentMailCooldown {
            key: "send_system_mail/8002".into(),
            secs: i32::MAX as u32,
            quiet: true,
        }),
    }
}

/// `sent` is exactly one notice request for `entity_id`.
fn assert_only_the_notice(sent: &[CellToBaseMsg], entity_id: u32) {
    let [CellToBaseMsg::ContentSystemMail(mail)] = sent else {
        panic!("expected one ContentSystemMail, got {sent:?}");
    };
    assert_eq!(mail, &notice_mail(entity_id));
}

/// **Guard: a Free Jaffa arriving on Dakara_E1 resolves exactly the grant
/// and the notice, from the whole seed.** Delete either chain, change the
/// granted gate, drop the quiet cooldown or reword the notice and the list
/// changes. Both actions are immediate.
#[tokio::test]
async fn live_db_free_jaffa_arriving_on_dakara_resolves_exactly_the_grant_and_the_notice() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = arrive(&engine, DAKARA, Some(FREE_JAFFA));
    assert_eq!(
        resolved.actions,
        vec![
            (
                GRANT_CHAIN,
                Action::GrantStargateAddress {
                    stargate_id: OMEGA_SITE_GATE
                }
            ),
            (NOTICE_CHAIN, notice_action()),
        ],
    );
    super::assert_no_deferred_actions(&resolved, GRANT_CHAIN);
    super::assert_no_deferred_actions(&resolved, NOTICE_CHAIN);
}

/// **Guard: nobody else gets them, and nowhere else.** Every other
/// archetype (and an entity with none) resolves nothing at all on
/// Dakara_E1, from any chain in the seed; a Free Jaffa entering any other
/// world resolves neither chain. Drop the `archetype eq 7` condition, flip
/// it to `neq`, or clear the trigger's world key and this fails.
#[tokio::test]
async fn live_db_dakara_arrival_chains_fire_for_no_other_archetype_and_on_no_other_world() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Control: the positive case resolves, so the negatives are not vacuous.
    assert_eq!(arrive(&engine, DAKARA, Some(FREE_JAFFA)).actions.len(), 2);

    for archetype in [1, 2, 3, 4, 5, 6, PRAXIS_JAFFA] {
        let resolved = arrive(&engine, DAKARA, Some(archetype));
        assert!(
            resolved.actions.is_empty(),
            "archetype {archetype} arriving on Dakara_E1 must resolve nothing, got {:?}",
            resolved.actions
        );
    }
    let unstamped = arrive(&engine, DAKARA, None);
    assert!(
        unstamped.actions.is_empty(),
        "an entity with no archetype must resolve nothing, got {:?}",
        unstamped.actions
    );

    // Every world the seed knows, so a sibling name (`Dakara_E1_StoryRm`,
    // `Dakara_E2`) is covered without naming it.
    let worlds: Vec<String> = sqlx::query_scalar("SELECT world::text FROM resources.worlds")
        .fetch_all(&pool)
        .await
        .expect("worlds");
    assert!(worlds.iter().any(|w| w == DAKARA));
    assert!(worlds.iter().any(|w| w == "Dakara_E1_StoryRm"));
    for world in worlds.iter().filter(|w| *w != DAKARA) {
        let resolved = arrive(&engine, world, Some(FREE_JAFFA));
        assert!(
            !resolved
                .actions
                .iter()
                .any(|(id, _)| [GRANT_CHAIN, NOTICE_CHAIN].contains(id)),
            "a Free Jaffa entering {world} must not fire the Dakara arrival chains"
        );
    }
}

/// **Guard: the arrival, executed.** Through `fire_player_loaded` and the
/// executor, a Free Jaffa's first entry sends the client
/// `updateStargateAddress(5, 1, 0)`, sends the base
/// `GrantStargateAddress` for gate 5 and one `ContentSystemMail`, and puts
/// the address in the cell's own book. A second entry by a character who
/// holds the address sends the notice request alone (the base then refuses
/// it quietly): no second grant, no second client method.
#[tokio::test]
async fn live_db_dakara_arrival_grants_omega_site_once_and_requests_the_notice() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let mut mgr = staged_space(&pool, PLAYER_EID, Some(FREE_JAFFA)).await;
    assert!(
        mgr.get_entity(PLAYER_EID)
            .unwrap()
            .known_stargates
            .is_empty(),
        "a new character's address book is empty"
    );

    let sent = enter_world(&engine, &mut mgr, PLAYER_EID, DAKARA).await;

    let [CellToBaseMsg::EntityMethodCall {
        entity_id: update_entity,
        method_index,
        args,
    }, CellToBaseMsg::GrantStargateAddress {
        entity_id: grant_entity,
        player_id,
        stargate_id,
    }, notice] = sent.as_slice()
    else {
        panic!("expected updateStargateAddress, GrantStargateAddress and the notice, got {sent:?}");
    };
    // updateStargateAddress(INT32 addressId, UINT8 hasAddress, UINT8 hidden)
    let mut update_args = OMEGA_SITE_GATE.to_le_bytes().to_vec();
    update_args.extend_from_slice(&[1, 0]);
    assert_eq!(
        (*update_entity, *method_index, args),
        (PLAYER_EID, UPDATE_STARGATE_ADDRESS, &update_args)
    );
    assert_eq!(
        (*grant_entity, *player_id, *stargate_id),
        (PLAYER_EID, PLAYER_ID, OMEGA_SITE_GATE)
    );
    assert_only_the_notice(std::slice::from_ref(notice), PLAYER_EID);
    assert_eq!(
        mgr.get_entity(PLAYER_EID).unwrap().known_stargates,
        vec![OMEGA_SITE_GATE],
        "the cell's book is what the dial gate enforces"
    );

    // The same entity again, at once: the grant is a no-op, and the mail is
    // inside the executor's one-second debounce or goes out once more.
    let again = enter_world(&engine, &mut mgr, PLAYER_EID, DAKARA).await;
    assert!(
        again
            .iter()
            .all(|m| matches!(m, CellToBaseMsg::ContentSystemMail(_))),
        "a second run must not grant again, got {again:?}"
    );

    // A relog: a new cell entity whose book the base stamped from the
    // database. Only the notice request goes out, for the base to refuse.
    let relogged = PLAYER_EID + 1;
    stage_player(&mut mgr, relogged, Some(FREE_JAFFA));
    mgr.get_entity_mut(relogged).unwrap().known_stargates = vec![OMEGA_SITE_GATE];
    let relog = enter_world(&engine, &mut mgr, relogged, DAKARA).await;
    assert_only_the_notice(&relog, relogged);
    assert_eq!(
        mgr.get_entity(relogged).unwrap().known_stargates,
        vec![OMEGA_SITE_GATE],
        "no duplicate in the book"
    );
}

/// **Guard: a visitor gets nothing, executed.** A Praxis Jaffa and a Human
/// entering Dakara_E1, and a Free Jaffa entering another world, send
/// nothing and learn nothing.
#[tokio::test]
async fn live_db_dakara_arrival_sends_a_visitor_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    for archetype in [Some(PRAXIS_JAFFA), Some(1), None] {
        let mut mgr = staged_space(&pool, PLAYER_EID, archetype).await;
        let sent = enter_world(&engine, &mut mgr, PLAYER_EID, DAKARA).await;
        assert!(
            sent.is_empty(),
            "archetype {archetype:?} on Dakara_E1 must be sent nothing, got {sent:?}"
        );
        assert!(mgr
            .get_entity(PLAYER_EID)
            .unwrap()
            .known_stargates
            .is_empty());
    }

    // The world comes from the dispatcher's caller, never from where the
    // entity happens to stand: a Free Jaffa whose world entry names another
    // world gets neither action.
    let mut mgr = staged_space(&pool, PLAYER_EID, Some(FREE_JAFFA)).await;
    let sent = enter_world(&engine, &mut mgr, PLAYER_EID, "Omega_Site").await;
    assert!(
        !sent.iter().any(|m| matches!(
            m,
            CellToBaseMsg::GrantStargateAddress { .. } | CellToBaseMsg::ContentSystemMail(_)
        )),
        "a Free Jaffa entering Omega_Site must get no grant and no notice, got {sent:?}"
    );
    assert!(mgr
        .get_entity(PLAYER_EID)
        .unwrap()
        .known_stargates
        .is_empty());
}
