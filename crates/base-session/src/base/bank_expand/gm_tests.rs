//! GM `.bankexpand` on the base (BV-05, owner decision while the Expand
//! dialog is quarantined): the same purchase handler with
//! `trigger=gm_console` and no offer, so the base quotes the current step
//! and buys it with the same keyed statement, checks and telemetry.
//!
//! Sentinels: `tests::caller(0x200..=0x206)` (accounts and players
//! `0x7000_BD00..=0x7000_BD07`), entities `0x7000_BC00..=0x7000_BC03`. Skip when `DATABASE_URL` is unset.

use std::sync::Arc;

use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_wire::cell::messages::ExpandTrigger;
use tracing::Level;

use super::tests::{caller, cleanup, in_world, one, row, setup, TestClient};
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A GM `.bank` session: open, no Banker.
const GM_SESSION: VaultAccess = VaultAccess::Open {
    scope: VaultScope::Personal,
    banker_id: None,
    distance: None,
};

async fn gm_expand(pool: &PgPool, client: &TestClient, c: ExpandCaller, vault: VaultAccess) {
    handle_expand(
        c,
        None,
        vault,
        ExpandTrigger::GmConsole,
        &Some(Arc::new(pool.clone())),
        &client.dyn_transport,
        &client.conn,
    )
    .await;
}

/// `.bankexpand` in a GM session buys the current step at the seeded price
/// with no offer: 60 to 70, 100 paid, INFO `expand trigger=gm_console
/// gm_override=true`, and the same three sends. Fails if the console
/// trigger is refused as `no_offer` like a dialog answer without one.
#[tokio::test]
async fn gm_bankexpand_buys_the_current_step() {
    let pool = require_db_or_skip!();
    let c = caller(0x200, 0x7000_BC00);
    setup(&pool, c, 60, 150).await;
    let client = in_world(c, 40930);
    let capture = LogCapture::install();

    gm_expand(&pool, &client, c, GM_SESSION).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (70, 50));
    let e = one(
        &capture,
        "expand",
        Level::INFO,
        c,
        &[
            ("trigger", "gm_console"),
            ("gm_override", "true"),
            ("bank_slots_before", "60"),
            ("bank_slots_after", "70"),
            ("cash_before", "150"),
            ("cash_after", "50"),
        ],
    );
    assert!(!e.fields.contains_key("banker_id"), "{e:#?}");
    assert!(client.saw_bytes(&vault_resize_bag_info_args(70)));
    assert!(client.saw_text("Your vault now has 70 slots. You paid 100 naquadah."));
}

/// The console gets the dialog's checks: at 100 it is `at_ceiling`, short
/// of cash it is `insufficient_cash`, and nothing changes either way.
#[tokio::test]
async fn gm_bankexpand_keeps_the_ceiling_and_the_cash_check() {
    let pool = require_db_or_skip!();
    for (n, entity_id, slots, cash, reason) in [
        (0x202, 0x7000_BC01, 100i16, 1000, "at_ceiling"),
        (0x204, 0x7000_BC02, 40, 99, "insufficient_cash"),
    ] {
        let c = caller(n, entity_id);
        setup(&pool, c, slots, cash).await;
        let client = in_world(c, 40931);
        let capture = LogCapture::install();

        gm_expand(&pool, &client, c, GM_SESSION).await;
        let after = row(&pool, c.player_id).await;
        cleanup(&pool, c).await;

        assert_eq!(after, (slots, cash), "{reason}");
        one(
            &capture,
            "expand_rejected",
            Level::WARN,
            c,
            &[("reason", reason), ("trigger", "gm_console")],
        );
    }
}

/// `.bankexpand` with no vault session is refused with the verdict's
/// label and a line telling the GM to open the vault first.
#[tokio::test]
async fn gm_bankexpand_needs_an_open_vault() {
    let pool = require_db_or_skip!();
    let c = caller(0x206, 0x7000_BC03);
    setup(&pool, c, 40, 500).await;
    let client = in_world(c, 40932);
    let capture = LogCapture::install();

    gm_expand(&pool, &client, c, VaultAccess::NO_SESSION).await;
    let after = row(&pool, c.player_id).await;
    cleanup(&pool, c).await;

    assert_eq!(after, (40, 500));
    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        c,
        &[("reason", "no_vault_session"), ("trigger", "gm_console")],
    );
    assert!(client.saw_text(
        "bankexpand: open your vault first (.bank, or a Banker in range). Nothing was charged."
    ));
}
