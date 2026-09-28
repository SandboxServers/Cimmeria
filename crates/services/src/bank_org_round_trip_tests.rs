//! The debug hub's Team and Command Bankers (bank-vault BV-10a, spawns 471
//! and 472), clicked end to end: the NPC spawned from its real seed row, the
//! right-click through the cell's wire dispatcher, the
//! `BankCellToBase::OrgVaultOpen` it asks through the base's real
//! `handle_cell_message`, the base's membership check against the live
//! database, and the base's `OrgVaultGranted` back into the cell's
//! `grant_org_vault`, which sends `onTeamVaultOpen` (107) or
//! `onCommandVaultOpen` (108).
//!
//! The cell half is in `cimmeria-cell-methods` and `cimmeria-cell-interactions`
//! and the base half in `cimmeria-base-world-entry`; neither track can depend
//! on the other, so the round trip lives in the facade, like the gate and
//! mission round trips.
//!
//! Sentinels (the BV-10a block, `0x7000_BA00`..`0x7000_BA3F`): account
//! `0x7000_BA00`, players `0x7000_BA01`..`0x7000_BA03`, entity
//! `0x7000_BA20`; organizations are named `Bv10a Team`, `Bv10a Command` and `Bv10a Solo`
//! and cleaned up by exact name key. Port 40910.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_base_session::base::organization::persistence::create_org;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::{NpcInteractionType, VaultScope};
use cimmeria_entity::organization::{org_text, OrgType, TextField};
use cimmeria_mercury::transport::Transport;

use crate::base::world_entry::handle_cell_message;
use crate::cell::cell_methods::player::{dispatch, INTERACT};
use crate::cell::client_methods::player::{ON_COMMAND_VAULT_OPEN, ON_TEAM_VAULT_OPEN};
use crate::cell::interactions::grant_org_vault;
use crate::cell::messages::{BankBaseToCell, BankCellToBase, BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use crate::cell::spawner;
use crate::test_support::{
    require_db_or_skip, test_default_connected_client_state, LogCapture, TestTransport,
};

const ACCOUNT: i32 = 0x7000_BA00;
/// Leads a Team and a Command.
const IN_BOTH: i32 = 0x7000_BA01;
/// Leads a Team only.
const TEAM_ONLY: i32 = 0x7000_BA02;
/// In no organization.
const IN_NONE: i32 = 0x7000_BA03;
const PLAYER: u32 = 0x7000_BA20;
const PORT: u16 = 40910;
const TEAM_NAME: &str = "Bv10a Team";
const COMMAND_NAME: &str = "Bv10a Command";
const SOLO_NAME: &str = "Bv10a Solo";

/// The hub's Bankers by tag: `(tag, scope, client method a member gets)`.
const ORG_BANKERS: [(&str, VaultScope, u16); 2] = [
    ("DebugHub_TeamBanker", VaultScope::Team, ON_TEAM_VAULT_OPEN),
    (
        "DebugHub_CommandBanker",
        VaultScope::Command,
        ON_COMMAND_VAULT_OPEN,
    ),
];

fn name_keys() -> Vec<String> {
    [TEAM_NAME, COMMAND_NAME, SOLO_NAME]
        .iter()
        .map(|n| org_text::name_key(&org_text::validate(TextField::Name, n).unwrap()))
        .collect()
}

async fn teardown(pool: &PgPool) {
    let orgs: Vec<i32> =
        sqlx::query_scalar("SELECT org_id FROM sgw_organizations WHERE name_key = ANY($1)")
            .bind(name_keys())
            .fetch_all(pool)
            .await
            .unwrap();
    for sql in [
        "DELETE FROM sgw_organization_vault_items WHERE org_id = ANY($1)",
        "DELETE FROM sgw_organization_vault_log WHERE org_id = ANY($1)",
        "DELETE FROM sgw_organization_events WHERE org_id = ANY($1)",
        "DELETE FROM sgw_organizations WHERE org_id = ANY($1)",
    ] {
        sqlx::query(sql).bind(&orgs).execute(pool).await.unwrap();
    }
    sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(ACCOUNT)
        .execute(pool)
        .await
        .unwrap();
}

/// The account, three characters and three organizations: `IN_BOTH` leads
/// a Team and a Command, `TEAM_ONLY` leads a second Team, `IN_NONE` is in
/// none. Whatever a crashed run left is removed first.
async fn seed(pool: &PgPool) {
    teardown(pool).await;
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(ACCOUNT)
        .bind(format!("bv10a-test-{ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");
    for player_id in [IN_BOTH, TEAM_ONLY, IN_NONE] {
        sqlx::query(
            "INSERT INTO sgw_player (                account_id, player_id, level, alignment, archetype, gender,                 player_name, extra_name, world_location, bodyset,                 pos_x, pos_y, pos_z, skin_color_id, naquadah             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'Castle_CellBlock',                        'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0)",
        )
        .bind(ACCOUNT)
        .bind(player_id)
        .bind(format!("bv10a-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }
    let mut tx = pool.begin().await.unwrap();
    for (org_type, name, leader) in [
        (OrgType::Team, TEAM_NAME, IN_BOTH),
        (OrgType::Command, COMMAND_NAME, IN_BOTH),
        (OrgType::Team, SOLO_NAME, TEAM_ONLY),
    ] {
        create_org(&mut tx, org_type, name, leader)
            .await
            .unwrap_or_else(|e| panic!("create {name}: {e:?}"));
    }
    tx.commit().await.unwrap();
}

/// The cell with every `DebugHub_*` NPC spawned from its real row, and the
/// player at the stasis respawner as `player_id`. Returns the Bankers'
/// entity ids by tag.
async fn stage(pool: &PgPool) -> (SpaceManager, HashMap<String, u32>) {
    let records = spawner::load_spawns_from_db(pool)
        .await
        .expect("spawns must load");
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="false" MinX="-1000" MaxX="1000" MinY="-1000" MaxY="1000" /></Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();
    mgr.template_trainer_lists = spawner::load_template_trainer_lists(pool)
        .await
        .expect("trainer lists must load");
    let mut hub = HashMap::new();
    for record in records
        .iter()
        .filter(|r| r.tag.as_deref().is_some_and(|t| t.starts_with("DebugHub_")))
    {
        let eid = mgr.allocate_npc_id();
        mgr.spawn_npc_from_record(eid, record)
            .expect("hub NPC must spawn from its record");
        hub.insert(record.tag.clone().unwrap(), eid);
    }
    mgr.create_entity(
        PLAYER,
        "Castle_CellBlock",
        [-334.231, 73.472, -228.026],
        [0.0; 3],
    )
    .expect("the player must stage at the stasis respawner");
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.account_id = Some(ACCOUNT as u32);
    mgr.connect_entity(PLAYER);
    (mgr, hub)
}

/// The base's side: one in-world client for `PLAYER`.
struct Base {
    transport: Arc<TestTransport>,
    dyn_transport: Arc<dyn Transport>,
    addr: SocketAddr,
    e2a: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    conn: Arc<Mutex<HashMap<SocketAddr, crate::base::ConnectedClientState>>>,
    pool: Option<Arc<PgPool>>,
}

impl Base {
    fn new(pool: &PgPool) -> Base {
        let transport = Arc::new(TestTransport::new());
        let dyn_transport: Arc<dyn Transport> = transport.clone();
        let addr: SocketAddr = format!("127.0.0.1:{PORT}").parse().unwrap();
        let mut state = test_default_connected_client_state();
        state.player_entity_id = Some(PLAYER);
        Base {
            transport,
            dyn_transport,
            addr,
            e2a: Arc::new(Mutex::new(HashMap::from([(PLAYER, addr)]))),
            conn: Arc::new(Mutex::new(HashMap::from([(addr, state)]))),
            pool: Some(Arc::new(pool.clone())),
        }
    }

    /// Whether any packet to the client carries `needle` as UTF-16LE.
    fn saw_text(&self, needle: &str) -> bool {
        let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
        let want: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
        self.transport
            .filter_to(self.addr)
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .any(|p| p.windows(want.len()).any(|w| w == want))
    }
}

/// What one click produced end to end.
#[derive(Debug)]
struct Outcome {
    /// Every `OrgVaultOpen` scope the cell asked the base for.
    asked: Vec<VaultScope>,
    /// Every `OrgVaultGranted` scope the base answered with.
    granted: Vec<VaultScope>,
    /// Every client method the cell sent the player, click and grant.
    methods: Vec<u16>,
}

/// Right-click `npc` as `player_id`, route the cell's bank requests through
/// the base's real dispatcher, and feed the base's grants back to the cell
/// the way `CellService`'s `BaseToCellMsg::Bank` arm does.
async fn click(
    mgr: &mut SpaceManager,
    engine: &ChainEngine,
    base: &Base,
    npc: u32,
    player_id: i32,
) -> Outcome {
    mgr.get_entity_mut(PLAYER).unwrap().player_id = Some(player_id);
    mgr.get_entity_mut(PLAYER).unwrap().vault_session = None;
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
    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    let cell_tx = Some(cell_tx);
    let mut out = Outcome {
        asked: Vec::new(),
        granted: Vec::new(),
        methods: Vec::new(),
    };
    let mut to_base = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        to_base.push(msg);
    }
    for msg in to_base {
        match msg {
            CellToBaseMsg::EntityMethodCall { method_index, .. } => out.methods.push(method_index),
            CellToBaseMsg::Bank(BankCellToBase::OrgVaultOpen { scope, .. }) => {
                out.asked.push(scope);
                handle_cell_message(
                    msg,
                    &base.dyn_transport,
                    &base.conn,
                    &base.e2a,
                    &cell_tx,
                    &base.pool,
                    &None,
                    "",
                    0,
                )
                .await;
            }
            other => panic!("the Banker click sent the base {other:?}"),
        }
    }
    while let Ok(msg) = cell_rx.try_recv() {
        let BaseToCellMsg::Bank(BankBaseToCell::OrgVaultGranted {
            entity_id,
            player_id,
            scope,
            org_id,
            banker_id,
        }) = msg
        else {
            panic!("the base answered the open with a non-grant message");
        };
        out.granted.push(scope);
        grant_org_vault(entity_id, player_id, scope, org_id, banker_id, &tx, mgr).await;
    }
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            out.methods.push(method_index);
        }
    }
    out
}

/// Each org Banker, from its real seed row: a member of that organization
/// type gets exactly its window (107 for the Team Banker, 108 for the
/// Command Banker) and a session naming the organization; a character in no
/// organization is refused `not_in_org` and told so; a Team-only character
/// is refused by the Command Banker. Fails if 471 or 472 lose their rows or
/// `INT_Banker`, if a seed names the wrong `vault_scope`, if anything claims
/// the click first (the real content engine is loaded), or if either half of
/// the round trip drops the request.
#[tokio::test]
async fn live_db_debug_hub_org_bankers_open_the_members_vault_and_refuse_others() {
    let pool = require_db_or_skip!();
    seed(&pool).await;
    let (mut mgr, hub) = stage(&pool).await;
    let engine = crate::cell::content::build_engine(Some(&pool)).await;
    let base = Base::new(&pool);

    for (tag, scope, method) in ORG_BANKERS {
        let banker = *hub
            .get(tag)
            .unwrap_or_else(|| panic!("hub NPC {tag} must be staged: {hub:?}"));
        assert_eq!(
            mgr.get_entity(banker).unwrap().interaction_type,
            Some(NpcInteractionType::Banker { scope }),
            "{tag} must spawn as a {scope:?} Banker"
        );

        // A member: the Team or Command it leads opens.
        let member = click(&mut mgr, &engine, &base, banker, IN_BOTH).await;
        assert_eq!(member.asked, vec![scope], "{tag}: {member:?}");
        assert_eq!(member.granted, vec![scope], "{tag}: {member:?}");
        assert_eq!(member.methods, vec![method], "{tag}: {member:?}");
        let session = mgr
            .get_entity(PLAYER)
            .unwrap()
            .vault_session
            .clone()
            .expect("a granted open records a session");
        assert_eq!(session.banker_id, Some(banker), "{tag}");
        assert_eq!(session.scope, scope, "{tag}");
        assert!(session.org_id.is_some(), "{tag}: the session names the org");

        // In no organization: refused on the base, told, no window.
        let capture = LogCapture::install();
        let outsider = click(&mut mgr, &engine, &base, banker, IN_NONE).await;
        assert_eq!(outsider.asked, vec![scope], "{tag}: {outsider:?}");
        assert!(outsider.granted.is_empty(), "{tag}: {outsider:?}");
        assert!(outsider.methods.is_empty(), "{tag}: {outsider:?}");
        assert!(mgr.get_entity(PLAYER).unwrap().vault_session.is_none());
        let refusals: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.target == "bank" && c.has_field("event", "org_vault_open_rejected"))
            .collect();
        assert_eq!(refusals.len(), 1, "{tag}: {:#?}", capture.all());
        assert_eq!(refusals[0].level, Level::WARN);
        assert!(refusals[0].has_field("reason", "not_in_org"), "{tag}");
        assert!(
            refusals[0].has_field("banker_id", &banker.to_string()),
            "{tag}: {:#?}",
            refusals[0]
        );
        drop(capture);
        let label = if scope == VaultScope::Team {
            "Team"
        } else {
            "Command"
        };
        assert!(
            base.saw_text(&format!(
                "You are not in a {label}, so there is no {label} vault to open."
            )),
            "{tag}: the outsider is told"
        );
    }

    // A Team-only character: the Team Banker opens its Team, the Command
    // Banker refuses it. A Command Banker seeded with `vault_scope = 'team'`
    // would open the Team vault here instead.
    let team_banker = hub["DebugHub_TeamBanker"];
    let command_banker = hub["DebugHub_CommandBanker"];
    let opened = click(&mut mgr, &engine, &base, team_banker, TEAM_ONLY).await;
    assert_eq!(opened.methods, vec![ON_TEAM_VAULT_OPEN], "{opened:?}");
    let refused = click(&mut mgr, &engine, &base, command_banker, TEAM_ONLY).await;
    assert_eq!(refused.asked, vec![VaultScope::Command], "{refused:?}");
    assert!(refused.granted.is_empty(), "{refused:?}");
    assert!(refused.methods.is_empty(), "{refused:?}");

    teardown(&pool).await;
}
