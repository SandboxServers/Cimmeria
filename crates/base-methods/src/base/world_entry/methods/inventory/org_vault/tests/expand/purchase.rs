//! A quote, a purchase, and a purchase sent twice.

use cimmeria_wire::cell::client_methods::organization::build_on_organization_cash_update;

use super::super::super::open::org_vault_bag_info;
use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Without a size the command only quotes: DEBUG `expand_quote` with the
/// size, the price and the treasury, a line naming the command that buys,
/// and nothing changed. With the size the leader buys one step: the vault
/// goes 40 to 50 and the treasury 250 to 150 in one row, one cash log row
/// (`vault_expansion`, no wallet), INFO `expand` and INFO
/// `org_cash_transfer` with the org fields, `onBagInfo` with the Team vault
/// at 50 to the buyer, and the new treasury to every online member. Fails
/// if the `UPDATE`, the log insert, the broadcast or the `onBagInfo` is
/// removed.
#[tokio::test]
async fn live_db_a_leader_quotes_then_buys_one_step_from_the_treasury() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 0, 2).await;
    let team = fx.org(OrgType::Team, 0, &[1], 250).await;
    fx.online(0);
    fx.online(1);
    let team_s = team.to_string();

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, None).await;
    assert_eq!(fx.state(team).await, (40, 250), "a quote changes nothing");
    one(
        &fx,
        &capture,
        "expand_quote",
        Level::DEBUG,
        0,
        &[
            ("org_id", &team_s),
            ("vault_slots", "40"),
            ("price", "100"),
            ("org_cash", "250"),
            ("scope", "team"),
        ],
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: the Team vault has 40 slots. The next +10 costs 100 from the \
         treasury, which holds 250. Type .orgvaultexpand 40 to buy it."
    ));

    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, Some(40)).await;
    assert_eq!(fx.state(team).await, (50, 150));
    one(
        &fx,
        &capture,
        "expand",
        Level::INFO,
        0,
        &[
            ("org_id", &team_s),
            ("org_type", "team"),
            ("rank", "8"),
            ("scope", "team"),
            ("vault_slots_before", "40"),
            ("vault_slots_after", "50"),
            ("price", "100"),
            ("org_cash_before", "250"),
            ("org_cash_after", "150"),
            ("trigger", "gm_console"),
        ],
    );
    one(
        &fx,
        &capture,
        "org_cash_transfer",
        Level::INFO,
        0,
        &[
            ("org_id", &team_s),
            ("direction", "vault_expansion"),
            ("amount", "100"),
            ("org_cash_before", "250"),
            ("org_cash_after", "150"),
            ("recipients", "2"),
        ],
    );
    assert_eq!(
        fx.cash_log(team).await,
        vec![(
            fx.player(0),
            "vault_expansion".into(),
            100,
            None,
            250,
            150,
            Some(40),
            Some(50)
        )]
    );
    assert!(
        fx.saw_bytes(0, &org_vault_bag_info(40, 50)),
        "the buyer's onBagInfo declares the Team vault at 50"
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: the Team vault now has 50 slots. The treasury paid 100 and holds 150."
    ));
    let cash = build_on_organization_cash_update(team, 150);
    assert!(fx.saw_bytes(0, &cash) && fx.saw_bytes(1, &cash));
    fx.teardown().await;
}

/// The same purchase sent twice at once (a double press, or a replayed
/// command): the organization lock serializes them and the second finds
/// the vault already at 50, so it is `replay` and charges nothing. One
/// step, one debit, one log row. Fails if the purchase is not keyed on the
/// size the GM named (both buy: 60 slots, 200 paid).
#[tokio::test]
async fn live_db_a_double_purchase_is_charged_once() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 1, 1).await;
    let team = fx.org(OrgType::Team, 0, &[], 1000).await;
    fx.online(0);

    let capture = LogCapture::install();
    tokio::join!(
        fx.expand(0, VaultScope::Team, Some(40)),
        fx.expand(0, VaultScope::Team, Some(40))
    );
    assert_eq!(fx.state(team).await, (50, 900));
    assert_eq!(bank_rows(&capture, "expand").len(), 1);
    let refused = bank_rows(&capture, "expand_rejected");
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(
        refused[0].has_field("reason", "replay")
            && refused[0].has_field("vault_slots", "50")
            && refused[0].has_field("offered_slots", "40"),
        "{:#?}",
        refused[0]
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: the Team vault has 50 slots, not 40. Nothing was charged."
    ));
    assert_eq!(fx.cash_log(team).await.len(), 1);
    fx.teardown().await;
}

/// 90 to 100 is the last step; after it the vault is at the ceiling and a
/// further purchase is `at_ceiling` with nothing charged. Fails if the
/// ceiling check is removed (the schema's CHECK then aborts the statement
/// and the refusal becomes `query_failed`).
#[tokio::test]
async fn live_db_the_ceiling_is_one_hundred_slots() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 2, 1).await;
    let team = fx.org(OrgType::Team, 0, &[], 1000).await;
    fx.set_slots(team, 90).await;
    fx.online(0);

    fx.expand(0, VaultScope::Team, Some(90)).await;
    assert_eq!(fx.state(team).await, (100, 900));

    let capture = LogCapture::install();
    fx.expand(0, VaultScope::Team, Some(100)).await;
    assert_eq!(fx.state(team).await, (100, 900));
    one(
        &fx,
        &capture,
        "expand_rejected",
        Level::WARN,
        0,
        &[("reason", "at_ceiling"), ("vault_slots", "100")],
    );
    assert!(fx.saw_text(
        0,
        "orgvaultexpand: the Team vault already has 100 slots. Nothing was charged."
    ));
    assert_eq!(fx.cash_log(team).await.len(), 1);
    fx.teardown().await;
}
