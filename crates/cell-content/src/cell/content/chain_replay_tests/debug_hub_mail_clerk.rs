//! Castle_CellBlock stasis-room debug hub, the Gate Mail Clerk
//! (social-systems SS-U3): chain-replay guards for chains 7010-7011 in
//! `db/resources/Content/Seed/debug_hub_chains.sql`.
//!
//! * 7010: clicking the clerk opens dialog 60104, spoken by the clerk.
//! * 7011: 60104's button runs `send_system_mail`, and the executor sends
//!   the base exactly one `ContentSystemMail`: to the clicking player, from
//!   "Gate Mail Clerk", 50 naquadah and 5 Health Slappack TC1, behind a
//!   600 s cooldown keyed on the chain.
//!
//! Both chains are loaded from the seeded database, resolved, and pushed
//! through [`execute_actions`], so the seed row, the loader arm and the
//! executor arm are all on the path. The base half (the cooldown claim and
//! the mail row) is guarded in `cimmeria-base-methods`
//! `mail/tests/content_live.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::engine_loader::load_single_chain_for_test;
use super::super::executor::execute_actions;
use crate::cell::messages::{CellToBaseMsg, ContentMailCooldown, ContentSystemMail};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::require_db_or_skip;

const PLAYER_EID: u32 = 7402;
const PLAYER_ID: i32 = 43;
const ACCOUNT_ID: u32 = 9043;
/// The clerk. Fixed so the dialog frame can name it.
const CLERK_EID: u32 = 100_701;
const CLERK_TAG: &str = "DebugHub_MailClerk";
const CLERK_DIALOG: i32 = 60_104;

/// Wire index of `onDialogDisplay`, spelled out.
const ON_DIALOG_DISPLAY: u16 = 105;

/// A Castle_CellBlock space with the player and the clerk staged at their
/// seeded spots, and the dialog caches the executor reads.
async fn staged_space(pool: &PgPool) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.create_entity(
        PLAYER_EID,
        "Castle_CellBlock",
        [-325.0, 73.472, -228.5],
        [0.0; 3],
    )
    .expect("the player must stage");
    let p = mgr.get_entity_mut(PLAYER_EID).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    p.account_id = Some(ACCOUNT_ID);
    mgr.connect_entity(PLAYER_EID);
    mgr.spawn_npc(
        CLERK_EID,
        "Castle_CellBlock",
        [-324.11, 73.472, -227.84],
        [0.0; 3],
    )
    .expect("the clerk must stage");
    mgr.get_entity_mut(CLERK_EID).unwrap().tag = Some(CLERK_TAG.into());

    mgr.monologue_dialog_ids = spawner::load_monologue_dialog_ids(pool)
        .await
        .expect("monologue dialog ids must load");
    mgr.dialog_screen_text = spawner::load_dialog_screen_text(pool)
        .await
        .expect("dialog screen text must load");
    mgr
}

async fn engine_with(pool: &PgPool, chain_id: i32) -> ChainEngine {
    let chain = load_single_chain_for_test(pool, chain_id)
        .await
        .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
        .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
    let mut engine = ChainEngine::new();
    engine.register_chain(chain);
    engine
}

fn fire(
    engine: &ChainEngine,
    trigger_type: TriggerType,
    params: &[(&str, serde_json::Value)],
) -> ResolvedActions {
    let mut ctx = ExecutionContext::new();
    for (k, v) in params {
        ctx.set_param(k.to_string(), v.clone());
    }
    let event = TriggerEvent {
        trigger_type,
        source_entity: None,
        target_entity: None,
        params: ctx.params.clone(),
    };
    engine.resolve_event(&event, &ctx)
}

/// Run `resolved` for the staged player and return everything it sent.
async fn execute(resolved: ResolvedActions, mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    let exec_engine = ChainEngine::new();
    execute_actions(resolved, PLAYER_EID, PLAYER_ID, &tx, mgr, &exec_engine).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// Chain 7010: clicking the clerk opens 60104 with the clerk as speaker;
/// another hub NPC's tag fires nothing.
#[tokio::test]
async fn mail_clerk_click_opens_dialog_60104_as_the_clerk() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 7010).await;

    let resolved = fire(
        &engine,
        TriggerType::InteractTag,
        &[
            ("entity_tag", serde_json::json!(CLERK_TAG)),
            ("target_entity_id", serde_json::json!(CLERK_EID)),
        ],
    );
    assert_eq!(
        resolved.actions,
        vec![(
            7010,
            Action::DisplayDialog {
                dialog_id: CLERK_DIALOG
            }
        )],
    );

    let mut mgr = staged_space(&pool).await;
    let sent = execute(resolved, &mut mgr).await;
    assert_eq!(sent.len(), 1, "one onDialogDisplay; got {sent:?}");
    let CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index,
        args,
    } = &sent[0]
    else {
        panic!("expected onDialogDisplay, got {:?}", sent[0]);
    };
    assert_eq!((*entity_id, *method_index), (PLAYER_EID, ON_DIALOG_DISPLAY));
    assert_eq!(
        (
            i32::from_le_bytes(args[0..4].try_into().unwrap()),
            i32::from_le_bytes(args[4..8].try_into().unwrap()),
        ),
        (CLERK_EID as i32, CLERK_DIALOG),
        "the frame names the clerk as speaker and dialog 60104"
    );

    let wrong = fire(
        &engine,
        TriggerType::InteractTag,
        &[("entity_tag", serde_json::json!("DebugHub_DialogNpc"))],
    );
    assert!(wrong.actions.is_empty());
}

/// Chain 7011: the button sends exactly one mail request, to the clicking
/// player, with the seeded contents and a cooldown keyed on the chain; and
/// the chain keys on dialog 60104 only.
#[tokio::test]
async fn mail_clerk_button_sends_exactly_one_mail() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, 7011).await;
    let choose = |dialog_id: i32| {
        fire(
            &engine,
            TriggerType::DialogChoice,
            &[
                ("dialog_id", serde_json::json!(dialog_id)),
                ("button_id", serde_json::json!(8)),
            ],
        )
    };
    // The trigger matches the satisfying context, so the negative below is
    // not vacuous.
    assert!(!choose(CLERK_DIALOG).actions.is_empty());
    assert!(
        choose(60_100).actions.is_empty(),
        "chain 7011 keys on dialog 60104 only"
    );

    let resolved = choose(CLERK_DIALOG);
    assert_eq!(resolved.actions.len(), 1, "{:?}", resolved.actions);
    let mut mgr = staged_space(&pool).await;
    let sent = execute(resolved, &mut mgr).await;
    assert_eq!(sent.len(), 1, "exactly one message; got {sent:?}");
    let CellToBaseMsg::ContentSystemMail(mail) = &sent[0] else {
        panic!("expected ContentSystemMail, got {:?}", sent[0]);
    };
    assert_eq!(
        mail,
        &ContentSystemMail {
            entity_id: PLAYER_EID,
            player_id: PLAYER_ID,
            account_id: Some(ACCOUNT_ID),
            chain_id: 7011,
            sender_name: "Gate Mail Clerk".into(),
            subject: "Gate Mail test delivery".into(),
            body: "A test mail from the stasis-room debug hub. Take the naquadah and the \
                   Health Slappacks from this mail. The Gate Mail Clerk can send you another \
                   one in 10 minutes."
                .into(),
            cash: 50,
            item: Some((2893, 5)),
            cooldown: Some(ContentMailCooldown {
                key: "send_system_mail/7011".into(),
                secs: 600,
            }),
        }
    );
}
