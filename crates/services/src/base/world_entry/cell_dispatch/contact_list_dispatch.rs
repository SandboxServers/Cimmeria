//! Contact-list dispatch arms for `CellToBaseMsg`.
//!
//! Routes the six `ContactList*` variants to the base-side handlers in
//! `crate::base::contact_list::handlers`. Mirrors the shape of
//! `progression_dispatch` — thin arms that extract variant fields and call
//! the appropriate handler.

use crate::base::contact_list::handlers::{
    fanout_contact_event, handle_add_members, handle_create, handle_delete, handle_flags_update,
    handle_remove_members, handle_rename,
};
use crate::cell::messages::CellToBaseMsg;

use super::DispatchCtx;

/// Route a `ContactList*` `CellToBaseMsg` to its handler.
///
/// Called by [`super::handle_cell_message`] after the outer match narrows
/// `msg` to the contact-list family.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::ContactListCreate {
            entity_id,
            player_id,
            name,
            flags,
        } => {
            handle_create(
                entity_id,
                player_id,
                name,
                flags,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListDelete {
            entity_id,
            player_id,
            list_id,
        } => {
            handle_delete(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListRename {
            entity_id,
            player_id,
            list_id,
            name,
        } => {
            handle_rename(
                entity_id,
                player_id,
                list_id,
                name,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListFlagsUpdate {
            entity_id,
            player_id,
            list_id,
            flags,
        } => {
            handle_flags_update(
                entity_id,
                player_id,
                list_id,
                flags,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }

        CellToBaseMsg::ContactListAddMembers {
            entity_id,
            player_id,
            list_id,
            names,
        } => {
            handle_add_members(
                entity_id,
                player_id,
                list_id,
                names,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
            // If this targeted the Ignore list, re-sync the ignore cache + cell
            // AoI filter (no-op for Friends/custom lists).
            crate::base::dispatch::ignore::resync_ignore_after_member_change(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
            )
            .await;
        }

        CellToBaseMsg::ContactListRemoveMembers {
            entity_id,
            player_id,
            list_id,
            names,
        } => {
            handle_remove_members(
                entity_id,
                player_id,
                list_id,
                names,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await;
            crate::base::dispatch::ignore::resync_ignore_after_member_change(
                entity_id,
                player_id,
                list_id,
                ctx.db_pool,
                ctx.connected,
                ctx.entity_to_addr,
                ctx.cell_tx,
            )
            .await;
        }

        CellToBaseMsg::ContactListPresenceEvent {
            player_name,
            event_id,
            data_value,
        } => {
            // Fire-and-forget: spawn so the cell channel drains immediately
            // rather than blocking on the DB lookup + network fan-out.
            let db_pool = ctx.db_pool.clone();
            let transport = std::sync::Arc::clone(ctx.transport);
            let connected = std::sync::Arc::clone(ctx.connected);
            let entity_to_addr = std::sync::Arc::clone(ctx.entity_to_addr);
            tokio::spawn(async move {
                fanout_contact_event(
                    &player_name,
                    event_id,
                    data_value,
                    &db_pool,
                    &transport,
                    &connected,
                    &entity_to_addr,
                )
                .await;
            });
        }

        // Unreachable by construction — the outer match only routes
        // ContactList* variants to this function.
        _ => unreachable!("contact_list_dispatch::route received non-ContactList variant"),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    use cimmeria_mercury::transport::Transport;
    use tokio::sync::mpsc;

    use super::{route, DispatchCtx};
    use crate::base::contact_list::persistence::{
        ensure_system_lists, load_contact_lists, IGNORE_LIST_FLAGS,
    };
    use crate::base::ConnectedClientState;
    use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
    use crate::test_support::{
        require_db_or_skip, test_default_connected_client_state, TestTransport,
    };

    fn session(name: &str, entity_id: u32) -> ConnectedClientState {
        let mut s = test_default_connected_client_state();
        s.player_name = Some(name.to_string());
        s.player_entity_id = Some(entity_id);
        s
    }

    /// Live-DB: `route(ContactListAddMembers)` on the Ignore list runs the full
    /// chain — `handle_add_members` (DB insert + CM 87 echo) then `resync`
    /// (cache + cell `UpdateIgnoreList`) — and `route(ContactListRemoveMembers)`
    /// reverses it. Exercises the dispatch arms end-to-end through a real
    /// `DispatchCtx`. Self-skips without `DATABASE_URL`.
    #[tokio::test]
    async fn route_add_remove_ignore_members_end_to_end() {
        let pool = require_db_or_skip!();
        // Sentinel base distinct from persistence (4000), crafting (2000/3000),
        // and the resync test (5000).
        let account_id: i32 = 0x7000_6000;
        let player_id: i32 = 0x7000_6001;
        let entity_id: u32 = 760_001;
        let addr: SocketAddr = "127.0.0.1:7600".parse().unwrap();

        // FK-safe seed: account + sgw_player (contact lists cascade on delete).
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("cld-test-{account_id}"))
            .execute(&pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("cld-{player_id}"))
        .execute(&pool)
        .await
        .expect("insert player");
        let (_friends, ignore_id) = ensure_system_lists(&pool, player_id)
            .await
            .expect("ensure lists");

        // Wire a DispatchCtx around in-memory transport/maps + a real pool.
        let tt = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = tt.clone();
        let connected = Arc::new(Mutex::new(HashMap::from([(
            addr,
            session("Me", entity_id),
        )])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
        let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(8);
        let cell_tx = Some(tx);
        let db_pool = Some(Arc::new(pool.clone()));
        let registry = None;
        let ctx = DispatchCtx {
            transport: &transport,
            connected: &connected,
            entity_to_addr: &entity_to_addr,
            cell_tx: &cell_tx,
            db_pool: &db_pool,
            minigame_registry: &registry,
            minigame_external_host: "",
            minigame_external_port: 0,
        };

        // --- Add "Griefer" to the Ignore list ---
        route(
            CellToBaseMsg::ContactListAddMembers {
                entity_id,
                player_id,
                list_id: ignore_id,
                names: vec!["Griefer".to_string()],
            },
            &ctx,
        )
        .await;

        let lists = load_contact_lists(&pool, player_id).await.unwrap();
        assert!(
            lists
                .iter()
                .any(|l| l.flags == IGNORE_LIST_FLAGS && l.members.iter().any(|m| m == "Griefer")),
            "member must be persisted to the Ignore list"
        );
        assert!(
            !tt.filter_to(addr).is_empty(),
            "add must echo CM 87 to the player"
        );
        match rx.try_recv() {
            Ok(BaseToCellMsg::UpdateIgnoreList {
                entity_id: eid,
                ignore_names,
            }) => {
                assert_eq!(eid, entity_id);
                assert!(ignore_names.contains("Griefer"));
            }
            _ => panic!("add to the Ignore list must push UpdateIgnoreList to the cell"),
        }
        assert!(connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .ignore_set
            .contains("Griefer"));

        tt.clear();

        // --- Remove "Griefer" again ---
        route(
            CellToBaseMsg::ContactListRemoveMembers {
                entity_id,
                player_id,
                list_id: ignore_id,
                names: vec!["Griefer".to_string()],
            },
            &ctx,
        )
        .await;

        let lists = load_contact_lists(&pool, player_id).await.unwrap();
        assert!(
            !lists
                .iter()
                .any(|l| l.members.iter().any(|m| m == "Griefer")),
            "member must be deleted"
        );
        assert!(
            !tt.filter_to(addr).is_empty(),
            "remove must echo CM 88 to the player"
        );
        match rx.try_recv() {
            Ok(BaseToCellMsg::UpdateIgnoreList { ignore_names, .. }) => {
                assert!(
                    !ignore_names.contains("Griefer"),
                    "resync after remove must drop the name"
                );
            }
            _ => panic!("remove from the Ignore list must push UpdateIgnoreList to the cell"),
        }
        assert!(!connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .ignore_set
            .contains("Griefer"));

        // Cleanup.
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(&pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(&pool)
            .await;
    }
}
