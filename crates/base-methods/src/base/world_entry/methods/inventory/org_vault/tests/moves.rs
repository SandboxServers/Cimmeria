//! Deposits into and withdrawals out of the Team and Command vaults
//! (bank-vault BV-07b): the round trip, the log row, and every
//! `org_move_rejected` reason, each with its line and snap-back. Live-DB
//! (TESTING.md type 3) and `LogCapture` (type 12).

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_wire::cell::vault::VaultAccess;
use tracing::Level;

use super::*;
use crate::base::world_entry::methods::inventory::move_::handle_move_inventory_item_with_vault;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};

const BANKER: u32 = 0x7000_B9D9;

/// An open, in-range session of `scope` for `org_id`.
pub(super) fn at_banker(scope: VaultScope, org_id: i32) -> VaultAccess {
    VaultAccess::Open {
        scope,
        org_id: Some(org_id),
        banker_id: Some(BANKER),
        distance: Some(2.0),
    }
}

/// Character `who` moves `item` to `(container, slot)`.
pub(super) async fn mv(
    fx: &Fx,
    client: &Client,
    who: usize,
    item: i32,
    container: i32,
    slot: i32,
    quantity: i32,
    vault: VaultAccess,
) {
    handle_move_inventory_item_with_vault(
        fx.entity(who),
        fx.player(who),
        item,
        container,
        slot,
        quantity,
        vault,
        &Some(Arc::new(fx.pool.clone())),
        &None,
        &client.dyn_transport,
        &client.conn,
        &client.e2a,
    )
    .await;
}

pub(super) fn bank(capture: &LogCaptureGuard, event: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", event))
        .collect()
}

/// Exactly one `org_move_rejected` with `reason`, carrying every `want`.
pub(super) fn refused(capture: &LogCaptureGuard, reason: &str, want: &[(&str, &str)]) -> Captured {
    let rows = bank(capture, "org_move_rejected");
    assert_eq!(rows.len(), 1, "{reason}: {:#?}", capture.all());
    let row = rows.into_iter().next().unwrap();
    assert_eq!(row.level, Level::WARN);
    assert!(row.has_field("reason", reason), "{row:#?}");
    for (k, v) in want {
        assert!(row.has_field(k, v), "{reason}: {k}={v}: {row:#?}");
    }
    row
}

/// The `InvItem` prefix of `item` as the client would see it: id, type,
/// stack, 1-indexed slot, container.
pub(super) fn inv_item(item: i32, type_id: i32, stack: i32, slot: i32, container: i32) -> Vec<u8> {
    [item, type_id, stack, slot + 1, container]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

/// D-BV15 and the telemetry rule: a deposit then a withdrawal by the
/// leader round-trips the same `item_id` through the Team vault, leaves it
/// in exactly one table at each step, writes one log row per move with the
/// actor's ids, and logs `org_move_accepted` with the org fields. Fails if
/// a move copies without deleting, renumbers the item, or skips the log.
#[tokio::test]
async fn deposit_then_withdraw_round_trips_the_same_item_id() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 3, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let item = fx.item(0);
    fx.carry(0, item, 1, 0, BANKABLE, 5, false).await;
    let client = Client::in_world(fx.entity(0), 40873);
    let vault = at_banker(VaultScope::Team, team);

    let capture = LogCapture::install();
    mv(&fx, &client, 0, item, 19, 7, -1, vault).await;
    assert_eq!(fx.vault(team).await, vec![(item, 7, 5)], "deposited whole");
    assert!(fx.bag(0).await.is_empty());
    assert!(
        client.saw_bytes(&inv_item(item, BANKABLE, 5, 7, 19)),
        "the client sees it in 19"
    );
    let row = &bank(&capture, "org_move_accepted")[0];
    for (k, v) in [
        ("account_id", &fx.account_id.to_string()[..]),
        ("player_id", &fx.player(0).to_string()[..]),
        ("entity_id", &fx.entity(0).to_string()[..]),
        ("org_id", &team.to_string()[..]),
        ("org_type", "team"),
        ("rank", "8"),
        ("perm", "DepositBank"),
        ("kind", "deposit"),
        ("direction", "deposit"),
        ("item_id", &item.to_string()[..]),
        ("type_id", &BANKABLE.to_string()[..]),
        ("quantity", "5"),
        ("source_container_id", "1"),
        ("target_container_id", "19"),
        ("target_slot_id", "7"),
        ("source_stack_before", "5"),
        ("source_stack_after", "0"),
        ("target_stack_after", "5"),
        ("vault_slots", "40"),
        ("banker_id", &BANKER.to_string()[..]),
    ] {
        assert!(row.has_field(k, v), "{k}={v}: {row:#?}");
    }
    assert_eq!(row.level, Level::DEBUG);

    mv(&fx, &client, 0, item, 1, 3, -1, vault).await;
    assert!(fx.vault(team).await.is_empty(), "withdrawn");
    assert_eq!(
        fx.bag(0).await,
        vec![(item, 1, 3, 5)],
        "the same item_id back"
    );
    assert!(fx.duplicated_ids().await.is_empty());
    let log = fx.log(team).await;
    let actor = (fx.account_id, fx.player(0));
    assert_eq!(
        log.iter()
            .map(|(d, k, i, q, a, p)| (d.as_str(), k.as_str(), *i, *q, (*a, *p)))
            .collect::<Vec<_>>(),
        vec![
            ("deposit", "deposit", item, 5, actor),
            ("withdraw", "withdraw", item, 5, actor)
        ]
    );
    fx.teardown().await;
}

/// A player who is not in the organization is refused under the lock
/// (`not_a_member`) even with a session naming it, and told; the item
/// stays in their bag and is resent. Fails if the move trusts the cell's
/// session instead of re-reading membership.
#[tokio::test]
async fn a_non_member_is_refused_not_a_member() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 4, 2).await;
    let team = fx.org(0, 1, &[]).await;
    let item = fx.item(0);
    fx.carry(0, item, 1, 0, BANKABLE, 5, false).await;
    let client = Client::in_world(fx.entity(0), 40874);
    let capture = LogCapture::install();

    mv(
        &fx,
        &client,
        0,
        item,
        19,
        0,
        -1,
        at_banker(VaultScope::Team, team),
    )
    .await;

    refused(&capture, "not_a_member", &[("org_id", &team.to_string())]);
    assert_eq!(fx.bag(0).await, vec![(item, 1, 0, 5)]);
    assert!(fx.vault(team).await.is_empty());
    assert!(client.saw_text("You are no longer in this Team, so you cannot use its vault."));
    assert!(
        client.saw_bytes(&inv_item(item, BANKABLE, 5, 0, 1)),
        "snap-back"
    );
    fx.teardown().await;
}

/// D-BV12: a default-rank member may deposit but not withdraw. The
/// withdrawal is refused `missing_permission perm=WithdrawBank`, the item
/// stays in the vault and is resent from there. Fails if the move skips
/// the WithdrawBank bit.
#[tokio::test]
async fn a_member_without_withdraw_bank_is_refused() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 5, 2).await;
    let cmd = fx.org(1, 1, &[0]).await;
    let item = fx.item(0);
    fx.put(cmd, item, 4, BANKABLE, 3).await;
    let client = Client::in_world(fx.entity(0), 40875);
    let capture = LogCapture::install();

    mv(
        &fx,
        &client,
        0,
        item,
        1,
        0,
        -1,
        at_banker(VaultScope::Command, cmd),
    )
    .await;

    refused(
        &capture,
        "missing_permission",
        &[
            ("perm", "WithdrawBank"),
            ("org_id", &cmd.to_string()),
            ("org_type", "command"),
            ("rank", "2"),
            ("account_id", &fx.account_id.to_string()),
        ],
    );
    assert_eq!(fx.vault(cmd).await, vec![(item, 4, 3)]);
    assert!(fx.bag(0).await.is_empty());
    assert!(client.saw_text("Your Command rank cannot withdraw items from the Command vault."));
    assert!(
        client.saw_bytes(&inv_item(item, BANKABLE, 3, 4, 20)),
        "snap-back from 20"
    );

    // The same member may deposit.
    let mine = fx.item(1);
    fx.carry(0, mine, 1, 0, BANKABLE, 2, false).await;
    mv(
        &fx,
        &client,
        0,
        mine,
        20,
        5,
        -1,
        at_banker(VaultScope::Command, cmd),
    )
    .await;
    assert_eq!(fx.vault(cmd).await, vec![(item, 4, 3), (mine, 5, 2)]);
    fx.teardown().await;
}

/// The entry rules for a shared vault, one refusal each, with the item
/// kept in the bag: a bound item (`bound_item_not_org_storable`), a mission
/// item (`mission_item_not_bankable`), an item the personal vault would not
/// take (`item_not_allowed_in_container`), a slot past the Team's size
/// (`target_slot_beyond_vault_slots`), and a quantity above the stack.
#[tokio::test]
async fn deposits_obey_the_vault_entry_rules() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 6, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40876);
    let cases: [(i32, bool, i32, i32, &str, &str); 5] = [
        (
            BANKABLE,
            true,
            0,
            -1,
            "bound_item_not_org_storable",
            "Bound items cannot be stored in the Team vault.",
        ),
        (
            MISSION,
            false,
            0,
            -1,
            "mission_item_not_bankable",
            "Mission items cannot be stored in the Team vault.",
        ),
        (
            CARRIED_ONLY,
            false,
            0,
            -1,
            "item_not_allowed_in_container",
            "That item cannot be placed there.",
        ),
        (
            BANKABLE,
            false,
            40,
            -1,
            "target_slot_beyond_vault_slots",
            "That vault slot is locked. The Team vault has 40 slots.",
        ),
        (
            BANKABLE,
            false,
            0,
            9,
            "quantity_exceeds_stack",
            "That stack is not that large.",
        ),
    ];
    for (type_id, bound, slot, quantity, reason, line) in cases {
        let item = fx.item(0);
        fx.carry(0, item, 1, 0, type_id, 5, bound).await;
        let capture = LogCapture::install();
        mv(&fx, &client, 0, item, 19, slot, quantity, vault).await;
        refused(&capture, reason, &[("org_id", &team.to_string())]);
        assert!(client.saw_text(line), "{reason}: {line}");
        assert_eq!(fx.bag(0).await, vec![(item, 1, 0, 5)], "{reason}: kept");
        assert!(fx.vault(team).await.is_empty(), "{reason}");
        sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(item)
            .execute(&pool)
            .await
            .unwrap();
        client.transport.clear();
    }
    fx.teardown().await;
}

/// The cell's verdict: no session (`no_vault_session`), a personal session
/// (`vault_scope_mismatch`), a Team session for the Command vault (the
/// same), and an item in another org's vault (`item_not_in_vault`). Fails
/// if the org path does not check the verdict's scope and org.
#[tokio::test]
async fn the_verdict_must_open_this_orgs_vault() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 7, 2).await;
    let team = fx.org(0, 0, &[]).await;
    let other = fx.org(2, 1, &[]).await;
    let client = Client::in_world(fx.entity(0), 40877);
    let item = fx.item(0);
    fx.carry(0, item, 1, 0, BANKABLE, 5, false).await;
    let personal = VaultAccess::Open {
        scope: VaultScope::Personal,
        org_id: None,
        banker_id: Some(BANKER),
        distance: Some(1.0),
    };
    for (vault, container, reason) in [
        (VaultAccess::NO_SESSION, 19, "no_vault_session"),
        (personal, 19, "vault_scope_mismatch"),
        (
            at_banker(VaultScope::Team, team),
            20,
            "vault_scope_mismatch",
        ),
    ] {
        let capture = LogCapture::install();
        mv(&fx, &client, 0, item, container, 0, -1, vault).await;
        refused(&capture, reason, &[("vault_end", "target")]);
        assert_eq!(fx.bag(0).await, vec![(item, 1, 0, 5)], "{reason}");
    }

    let theirs = fx.item(1);
    fx.put(other, theirs, 0, BANKABLE, 2).await;
    let capture = LogCapture::install();
    mv(
        &fx,
        &client,
        0,
        theirs,
        1,
        5,
        -1,
        at_banker(VaultScope::Team, team),
    )
    .await;
    refused(
        &capture,
        "item_not_in_vault",
        &[("org_id", &team.to_string())],
    );
    assert_eq!(fx.vault(other).await, vec![(theirs, 0, 2)], "untouched");

    // Server-authority review B1: with no session, the refusal must not
    // resend (or lock) the organization the forged id points at. Fails if
    // the verdict refusal uses the routing read's organization.
    for vault in [
        VaultAccess::NO_SESSION,
        at_banker(VaultScope::Command, team),
    ] {
        client.transport.clear();
        let capture = LogCapture::install();
        mv(&fx, &client, 0, theirs, 1, 5, -1, vault).await;
        let row = &bank(&capture, "org_move_rejected")[0];
        assert!(
            !row.has_field("org_id", &other.to_string()),
            "the refusal names the victim org: {row:#?}"
        );
        assert!(
            !client.saw_bytes(&inv_item(theirs, BANKABLE, 2, 0, 19)),
            "another org's vault row reached this client"
        );
    }
    fx.teardown().await;
}

/// A vault item another member withdrew since this client last saw the
/// vault: dragging it is refused `item_not_in_vault` and the stale item is
/// removed from the window (`onRemoveItem`), with a line. Fails if the
/// personal path's silent "source item not found" swallows it.
#[tokio::test]
async fn a_stale_vault_item_is_refused_and_removed_from_the_window() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 8, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let gone = fx.item(0);
    let client = Client::in_world(fx.entity(0), 40878);
    let capture = LogCapture::install();

    mv(
        &fx,
        &client,
        0,
        gone,
        1,
        0,
        -1,
        at_banker(VaultScope::Team, team),
    )
    .await;

    refused(
        &capture,
        "item_not_in_vault",
        &[("org_id", &team.to_string())],
    );
    assert!(client.saw_text("That item is no longer in the Team vault."));
    let mut remove = 1u32.to_le_bytes().to_vec();
    remove.extend_from_slice(&gone.to_le_bytes());
    assert!(client.saw_bytes(&remove), "onRemoveItem([item])");
    fx.teardown().await;
}

/// Where a withdrawal may land, and the shared split rule, one refusal
/// each with the item kept in the vault: buyback (16) is not
/// player-movable, a slot past the bag's size is invalid, and a split onto
/// an occupied slot that cannot merge is refused.
#[tokio::test]
async fn withdrawals_land_only_in_carried_slots() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 12, 1).await;
    let team = fx.org(0, 0, &[]).await;
    let vault = at_banker(VaultScope::Team, team);
    let client = Client::in_world(fx.entity(0), 40883);
    let (item, other) = (fx.item(0), fx.item(1));
    fx.put(team, item, 0, BANKABLE, 5).await;
    fx.put(team, other, 1, MISSION, 1).await;
    for (container, slot, quantity, reason, line) in [
        (
            16,
            0,
            -1,
            "target_container_not_player_movable",
            "Items can only move between your bags and the Team vault.",
        ),
        (
            1,
            40,
            -1,
            "invalid_target_slot",
            "That item cannot be placed there.",
        ),
        (
            19,
            1,
            2,
            "split_onto_occupied_slot",
            "Split a stack onto an empty slot.",
        ),
    ] {
        let capture = LogCapture::install();
        mv(&fx, &client, 0, item, container, slot, quantity, vault).await;
        refused(&capture, reason, &[("org_id", &team.to_string())]);
        assert!(client.saw_text(line), "{reason}: {line}");
        assert_eq!(
            fx.vault(team).await,
            vec![(item, 0, 5), (other, 1, 1)],
            "{reason}"
        );
        assert!(fx.bag(0).await.is_empty(), "{reason}");
        client.transport.clear();
    }
    fx.teardown().await;
}
