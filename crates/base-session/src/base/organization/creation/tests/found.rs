//! `found_organization` against the real schema (TESTING.md type 3): the
//! CAT-M-03 creation guards and the D-ORG15 debit.

use cimmeria_entity::organization::{OrgRank, OrgType};

use super::super::super::persistence::{load_memberships, OrgStoreError};
use super::super::{found_organization, found_organization_at_cost, is_eligible, FoundReject};
use super::{memberships, naquadah, org_id_sequence, setup, teardown};
use crate::test_support::require_db_or_skip;

/// CAT-M-03: a name that breaks D-ORG10 founds nothing: empty, 61 UTF-16
/// units, a bidi control, a character outside the name set. None of them
/// reaches the database, so none draws an organization id.
#[tokio::test]
async fn live_db_create_rejects_invalid_names() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 0, &[0], &[]).await;
    let leader = fx.player(0);
    let before = org_id_sequence(&pool).await;

    let too_long = "A".repeat(61);
    for name in [
        "",
        "   ",
        too_long.as_str(),
        "Evil\u{202E}Twin",
        "Org<script>",
    ] {
        let got = found_organization(&pool, OrgType::Team, name, leader).await;
        assert!(
            matches!(got, Err(FoundReject::Store(OrgStoreError::InvalidText(_)))),
            "{name:?} must be refused as invalid text, got {got:?}"
        );
    }
    let at_cap = "B".repeat(60);
    let after = org_id_sequence(&pool).await;
    let memberships_after = memberships(&pool, leader).await;
    let sixty = found_organization(&pool, OrgType::Team, &at_cap, leader).await;
    teardown(&pool, &fx).await;

    assert_eq!(
        after, before,
        "an invalid name must not draw an organization id"
    );
    assert_eq!(memberships_after, 0);
    assert!(sixty.is_ok(), "60 units is the cap, not over it: {sixty:?}");
}

/// CAT-M-03 and the coordinator's rule: a taken name (compared on the
/// case-folded key) costs nothing and burns no organization id, even when
/// creation has a price. The first founder pays; the second keeps every
/// coin and joins nothing.
#[tokio::test]
async fn live_db_create_duplicate_name_costs_nothing() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 1, &[500, 500], &["Org05 Dup"]).await;
    let (first, second) = (fx.player(0), fx.player(1));

    let founded = found_organization_at_cost(&pool, OrgType::Command, "Org05 Dup", first, 100)
        .await
        .expect("the first founder gets the name");
    let before = org_id_sequence(&pool).await;
    let refused =
        found_organization_at_cost(&pool, OrgType::Command, "  org05   DUP ", second, 100).await;
    let after = org_id_sequence(&pool).await;
    let cash = (naquadah(&pool, first).await, naquadah(&pool, second).await);
    let second_joined = memberships(&pool, second).await;
    // The same name is free for the other type (D-ORG10: unique per type).
    let team = found_organization(&pool, OrgType::Team, "Org05 Dup", second).await;
    teardown(&pool, &fx).await;

    assert_eq!(founded.cost, 100);
    assert_eq!(founded.cash, Some((500, 400)));
    assert!(
        matches!(refused, Err(FoundReject::Store(OrgStoreError::NameTaken))),
        "{refused:?}"
    );
    assert_eq!(
        after, before,
        "a taken name must not draw an organization id"
    );
    assert_eq!(cash, (400, 500), "only the founder paid");
    assert_eq!(second_joined, 0);
    assert!(team.is_ok(), "{team:?}");
}

/// D-ORG18: one Team and one Command per player. A second Team is refused
/// before an id is drawn; a Command beside the Team is allowed.
#[tokio::test]
async fn live_db_create_rejects_a_second_organization_of_the_type() {
    let pool = require_db_or_skip!();
    let fx = setup(
        &pool,
        2,
        &[0],
        &["Org05 First", "Org05 Second", "Org05 Cmd"],
    )
    .await;
    let leader = fx.player(0);

    found_organization(&pool, OrgType::Team, "Org05 First", leader)
        .await
        .expect("first Team");
    let eligible_team = is_eligible(&pool, leader, OrgType::Team).await.unwrap();
    let eligible_command = is_eligible(&pool, leader, OrgType::Command).await.unwrap();
    let before = org_id_sequence(&pool).await;
    let second = found_organization(&pool, OrgType::Team, "Org05 Second", leader).await;
    let after = org_id_sequence(&pool).await;
    let command = found_organization(&pool, OrgType::Command, "Org05 Cmd", leader).await;
    let held = load_memberships(&pool, leader).await.unwrap();
    teardown(&pool, &fx).await;

    assert!(!eligible_team && eligible_command);
    assert!(
        matches!(
            second,
            Err(FoundReject::Store(OrgStoreError::AlreadyInType))
        ),
        "{second:?}"
    );
    assert_eq!(
        after, before,
        "a second Team must not draw an organization id"
    );
    assert!(command.is_ok(), "{command:?}");
    let got: Vec<(OrgType, OrgRank)> = held.iter().map(|m| (m.header.org_type, m.rank)).collect();
    assert_eq!(
        got,
        vec![
            (OrgType::Team, OrgRank::LEADER),
            (OrgType::Command, OrgRank::LEADER)
        ]
    );
}

/// D-ORG15: a founder who cannot pay founds nothing, keeps their naquadah,
/// and draws no organization id.
#[tokio::test]
async fn live_db_create_without_the_cost_founds_nothing() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 3, &[50], &["Org05 Poor"]).await;
    let leader = fx.player(0);
    let before = org_id_sequence(&pool).await;
    let got = found_organization_at_cost(&pool, OrgType::Team, "Org05 Poor", leader, 100).await;
    let after = org_id_sequence(&pool).await;
    let cash = naquadah(&pool, leader).await;
    let joined = memberships(&pool, leader).await;
    teardown(&pool, &fx).await;

    assert!(
        matches!(
            got,
            Err(FoundReject::InsufficientFunds {
                cost: 100,
                have: Some(50)
            })
        ),
        "{got:?}"
    );
    assert_eq!(
        after, before,
        "a short purse must not draw an organization id"
    );
    assert_eq!((cash, joined), (50, 0));
}

/// The debit is in the creation's transaction: when the organization
/// commits, the cost is gone, and nothing else moves. At D-ORG15's price
/// (0) the character's naquadah is not touched at all.
#[tokio::test]
async fn live_db_create_debits_in_the_same_transaction_and_free_touches_nothing() {
    let pool = require_db_or_skip!();
    let fx = setup(&pool, 4, &[250, 250], &["Org05 Paid", "Org05 Free"]).await;
    let (payer, free) = (fx.player(0), fx.player(1));

    let paid = found_organization_at_cost(&pool, OrgType::Team, "Org05 Paid", payer, 250)
        .await
        .expect("exactly enough");
    let gratis = found_organization(&pool, OrgType::Team, "Org05 Free", free)
        .await
        .expect("free creation");
    let cash = (naquadah(&pool, payer).await, naquadah(&pool, free).await);
    teardown(&pool, &fx).await;

    assert_eq!(paid.cash, Some((250, 0)));
    assert_eq!((gratis.cost, gratis.cash), (0, None));
    assert_eq!(cash, (0, 250));
    assert_eq!(super::super::creation_cost(OrgType::Team), 0, "D-ORG15");
    assert_eq!(super::super::creation_cost(OrgType::Command), 0, "D-ORG15");
}
