//! Mission / progression / grant / console-authoring / spawn / mail / minigame
//! dispatch arms.
//!
//! Each function is the body of one `CellToBaseMsg` arm carved out of the
//! [`super::handle_cell_message`] match. These route player-progression and
//! GM-grant traffic into the existing [`super::super::methods`] handlers and
//! the crafting / console-authoring / gm-spawn / minigame modules.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg, MailOp};

use super::super::super::ConnectedClientState;
use super::super::methods::{
    handle_grant_cash, handle_grant_item, handle_grant_xp, handle_mail_request,
    handle_mission_update,
};
use super::{minigame, DispatchCtx};

/// Route the mission / grant / console / spawn / mail / minigame family of
/// `CellToBaseMsg`.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any other variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op,
        } => {
            mail_request(
                entity_id,
                player_id,
                op,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.db_pool,
            )
            .await
        }
        CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id,
            status,
            current_step_id,
            completed_step_ids,
            completed_objective_ids,
            active_objective_ids,
            failed_objective_ids,
            repeats,
        } => {
            mission_update(
                player_id,
                mission_id,
                status,
                current_step_id,
                completed_step_ids,
                completed_objective_ids,
                active_objective_ids,
                failed_objective_ids,
                repeats,
                ctx.db_pool,
            )
            .await
        }
        CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
            notify_gm,
        } => {
            grant_xp(
                entity_id,
                xp_amount,
                notify_gm,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::TrainAbility {
            entity_id,
            player_id,
            ability_id,
        } => {
            train_ability(
                entity_id,
                player_id,
                ability_id,
                ctx.db_pool,
                ctx.connected,
                ctx.cell_tx,
                ctx.transport,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            container_id,
            count,
            notify_gm,
        } => {
            grant_item(
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
                notify_gm,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
            notify_gm,
        } => {
            grant_cash(
                entity_id,
                player_id,
                amount,
                notify_gm,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GrantExpertise {
            entity_id,
            player_id,
            discipline_id,
            amount,
        } => {
            grant_expertise(
                entity_id,
                player_id,
                discipline_id,
                amount,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GrantAppliedSciencePoints {
            entity_id,
            player_id,
            amount,
        } => {
            grant_applied_science_points(
                entity_id,
                player_id,
                amount,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::ExecuteAuthoringSql {
            entity_id,
            label,
            sql,
        } => {
            execute_authoring_sql(
                entity_id,
                label,
                sql,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::ConsoleSearch {
            entity_id,
            kind,
            query,
        } => {
            console_search(
                entity_id,
                kind,
                query,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::GmSpawnNpc {
            entity_id,
            template_id,
            space_id,
            world_name,
            position,
        } => {
            gm_spawn_npc(
                entity_id,
                template_id,
                space_id,
                world_name,
                position,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::StartMinigame {
            entity_id,
            player_id,
            game_name,
            difficulty,
            on_victory_chains,
        } => {
            start_minigame(
                entity_id,
                player_id,
                game_name,
                difficulty,
                on_victory_chains,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.minigame_registry,
                ctx.minigame_external_host,
                ctx.minigame_external_port,
            )
            .await
        }
        CellToBaseMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            minigame_result(
                entity_id,
                result_code,
                on_victory_chains,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
            )
            .await
        }
        other => unreachable!("progression_dispatch::route got unexpected variant: {other:?}"),
    }
}

/// `CellToBaseMsg::MailRequest`.
pub(super) async fn mail_request(
    entity_id: u32,
    player_id: i32,
    op: MailOp,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    handle_mail_request(
        entity_id,
        player_id,
        op,
        transport,
        connected,
        entity_to_addr,
        db_pool,
    )
    .await;
}

/// `CellToBaseMsg::MissionUpdate`.
pub(super) async fn mission_update(
    player_id: i32,
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: Vec<i32>,
    completed_objective_ids: Vec<i32>,
    active_objective_ids: Vec<i32>,
    failed_objective_ids: Vec<i32>,
    repeats: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    handle_mission_update(
        player_id,
        mission_id,
        status,
        current_step_id,
        &completed_step_ids,
        &completed_objective_ids,
        &active_objective_ids,
        &failed_objective_ids,
        repeats,
        db_pool,
    )
    .await;
}

/// `CellToBaseMsg::GrantXP`.
pub(super) async fn grant_xp(
    entity_id: u32,
    xp_amount: u64,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_grant_xp(
        entity_id,
        xp_amount,
        notify_gm,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::TrainAbility`.
pub(super) async fn train_ability(
    entity_id: u32,
    player_id: i32,
    ability_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    super::super::methods::progression::handle_train_ability(
        entity_id,
        player_id,
        ability_id,
        db_pool,
        connected,
        cell_tx,
        transport,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::GrantItem`.
pub(super) async fn grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_grant_item(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        notify_gm,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::GrantCash`.
pub(super) async fn grant_cash(
    entity_id: u32,
    player_id: i32,
    amount: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_grant_cash(
        entity_id,
        player_id,
        amount,
        notify_gm,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::GrantExpertise`.
pub(super) async fn grant_expertise(
    entity_id: u32,
    player_id: i32,
    discipline_id: i32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    crate::base::crafting::handlers::handle_grant_expertise(
        entity_id,
        player_id,
        discipline_id,
        amount,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::GrantAppliedSciencePoints`.
pub(super) async fn grant_applied_science_points(
    entity_id: u32,
    player_id: i32,
    amount: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    crate::base::crafting::handlers::handle_grant_applied_science(
        entity_id,
        player_id,
        amount,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::ExecuteAuthoringSql`.
pub(super) async fn execute_authoring_sql(
    entity_id: u32,
    label: String,
    sql: String,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    crate::base::console_authoring::handle_execute_authoring_sql(
        entity_id,
        &label,
        &sql,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::ConsoleSearch`.
pub(super) async fn console_search(
    entity_id: u32,
    kind: u8,
    query: String,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    crate::base::console_authoring::handle_console_search(
        entity_id,
        kind,
        &query,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::GmSpawnNpc`.
pub(super) async fn gm_spawn_npc(
    entity_id: u32,
    template_id: i32,
    space_id: u32,
    world_name: String,
    position: [f32; 3],
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    crate::base::gm_spawn::handle_gm_spawn_npc(
        entity_id,
        template_id,
        space_id,
        world_name,
        position,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::StartMinigame`.
pub(super) async fn start_minigame(
    entity_id: u32,
    player_id: i32,
    game_name: String,
    difficulty: u32,
    on_victory_chains: Vec<i64>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    minigame::start_minigame(
        entity_id,
        player_id,
        game_name,
        difficulty,
        on_victory_chains,
        transport,
        connected,
        entity_to_addr,
        minigame_registry,
        minigame_external_host,
        minigame_external_port,
    )
    .await;
}

/// `CellToBaseMsg::MinigameResult`.
pub(super) async fn minigame_result(
    entity_id: u32,
    result_code: u8,
    on_victory_chains: Vec<i64>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    minigame::minigame_result(
        entity_id,
        result_code,
        on_victory_chains,
        transport,
        connected,
        entity_to_addr,
        cell_tx,
    )
    .await;
}
