//! The fan-out of a vault move to the organization's other online members
//! (bank-vault BV-07b, through ORG-07's `broadcast_to_org`): a deposit sends
//! them the new vault row, a withdrawal the row's removal, and the mover
//! never gets a removal of the item it just took.

use cimmeria_entity::cell_entity::VaultScope;

use super::moves::{at_banker, inv_item, mv};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Two members online on one transport: `who` is the mover, `other` sees
/// the fan-out. Fails if the move path stops broadcasting, or broadcasts
/// the removal to the mover too.
#[tokio::test]
async fn other_members_see_deposits_and_withdrawals() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 16, 2).await;
    let team = fx.org(0, 0, &[1]).await;
    let item = fx.item(0);
    fx.carry(0, item, 1, 0, BANKABLE, 5, false).await;

    // One session map holding both members, listed online.
    let mover = Client::in_world(fx.entity(0), 40887);
    let other_addr: SocketAddr = "127.0.0.1:40888".parse().unwrap();
    {
        let mut conn = mover.conn.lock().unwrap();
        for (addr, who) in [(mover.addr, 0), (other_addr, 1)] {
            let mut state = conn
                .remove(&addr)
                .unwrap_or_else(test_default_connected_client_state);
            state.player_entity_id = Some(fx.entity(who));
            state.active_player_id = Some(fx.player(who));
            state.account_id = fx.account_id as u32;
            state.listed_online = true;
            conn.insert(addr, state);
        }
        mover.e2a.lock().unwrap().insert(fx.entity(1), other_addr);
    }
    let enc = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0u8; 32]);
    let seen = |addr: SocketAddr, bytes: &[u8]| {
        mover
            .transport
            .filter_to(addr)
            .iter()
            .filter_map(|p| enc.decrypt(p).ok())
            .any(|p| p.windows(bytes.len()).any(|w| w == bytes))
    };
    let vault = at_banker(VaultScope::Team, team);

    let capture = LogCapture::install();
    mv(&fx, &mover, 0, item, 19, 3, -1, vault).await;
    assert!(
        seen(other_addr, &inv_item(item, BANKABLE, 5, 3, 19)),
        "the other member sees the deposit"
    );
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "bank" && c.has_field("event", "org_vault_fanout"))
        .expect("org_vault_fanout");
    assert!(row.has_field("updated_recipients", "1"), "{row:#?}");
    assert!(row.has_field("org_id", &team.to_string()), "{row:#?}");

    // A second carried row, so the mover's bag resync is an array of two
    // and cannot be mistaken for `onRemoveItem([item])`.
    fx.carry(0, fx.item(1), 1, 5, BANKABLE, 1, false).await;
    mover.transport.clear();
    mv(&fx, &mover, 0, item, 1, 0, -1, vault).await;
    let mut remove = 1u32.to_le_bytes().to_vec();
    remove.extend_from_slice(&item.to_le_bytes());
    assert!(
        seen(other_addr, &remove),
        "the other member drops the withdrawn row"
    );
    assert!(
        !seen(mover.addr, &remove),
        "the mover keeps the item it took"
    );
    assert!(
        seen(mover.addr, &inv_item(item, BANKABLE, 5, 0, 1)),
        "in the mover's bag"
    );
    fx.teardown().await;
}
