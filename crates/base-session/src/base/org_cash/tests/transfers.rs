//! Committed transfers: both balances, the log row, the fan-out and the
//! actor's sends; and the bits that gate each direction.

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture};

/// The leader deposits 300 and withdraws 120. Each moves exactly that much
/// between the wallet and the treasury (the sum never changes), writes one
/// log row with both balances before and after, logs INFO
/// `org_cash_transfer` with the same values, sends every online member the
/// new treasury (`onOrganizationCashUpdate`), and sends the actor the new
/// wallet (`onCashChanged`) and a line. Fails if either `UPDATE` or the log
/// insert is removed, or if the broadcast is dropped.
#[tokio::test]
async fn a_deposit_and_a_withdrawal_conserve_the_total_and_are_logged() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 0, 2, 1000).await;
    let team = fx.org(OrgType::Team, 0, &[1]).await;
    fx.online(0);
    fx.online(1);

    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Deposit(300)).await;
    assert_eq!((fx.wallet(0).await, fx.org_cash(team).await), (700, 300));
    let team_s = team.to_string();
    one(
        &fx,
        &capture,
        "org_cash_transfer",
        Level::INFO,
        0,
        &[
            ("org_id", &team_s),
            ("org_type", "team"),
            ("rank", "8"),
            ("direction", "deposit"),
            ("amount", "300"),
            ("player_cash_before", "1000"),
            ("player_cash_after", "700"),
            ("org_cash_before", "0"),
            ("org_cash_after", "300"),
            ("recipients", "2"),
        ],
    );
    let actor = fx.calls_to(0);
    assert_eq!(cash_updates(&actor), vec![700]);
    assert_eq!(org_cash_updates(&actor), vec![(team, 300)]);
    assert_eq!(
        lines(&actor),
        vec!["You deposited 300 naquadah into the team treasury. It now holds 300."]
    );
    let member = fx.calls_to(1);
    assert_eq!(org_cash_updates(&member), vec![(team, 300)]);
    assert!(cash_updates(&member).is_empty(), "only the actor's wallet");
    assert!(lines(&member).is_empty());

    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(0, team, CashDir::Withdraw(120)).await;
    assert_eq!((fx.wallet(0).await, fx.org_cash(team).await), (820, 180));
    one(
        &fx,
        &capture,
        "org_cash_transfer",
        Level::INFO,
        0,
        &[
            ("direction", "withdraw"),
            ("amount", "120"),
            ("player_cash_before", "700"),
            ("player_cash_after", "820"),
            ("org_cash_before", "300"),
            ("org_cash_after", "180"),
        ],
    );
    assert_eq!(cash_updates(&fx.calls_to(0)), vec![820]);
    assert_eq!(org_cash_updates(&fx.calls_to(1)), vec![(team, 180)]);

    let p = fx.player(0);
    let a = fx.account_id;
    assert_eq!(
        fx.cash_log(team).await,
        vec![
            (p, a, "deposit".into(), 300, Some(1000), Some(700), 0, 300),
            (p, a, "withdraw".into(), 120, Some(700), Some(820), 300, 180),
        ]
    );
    fx.teardown().await;
}

/// D-BV12: a Command's lowest rank may deposit but not withdraw. The
/// refused withdrawal changes nothing, writes no log row, and is WARN
/// `org_cash_rejected reason=no_permission perm=WithdrawCash` with a line;
/// once the rank holds `WithdrawCash` the same withdrawal goes through.
/// Without `DepositCash` a deposit is refused the same way. Fails if either
/// bit check is removed.
#[tokio::test]
async fn each_direction_needs_its_own_bit() {
    let pool = require_db_or_skip!();
    let fx = Fx::new(&pool, 1, 2, 500).await;
    let command = fx.org(OrgType::Command, 0, &[1]).await;
    fx.online(1);

    fx.transfer(1, command, CashDir::Deposit(200)).await;
    assert_eq!((fx.wallet(1).await, fx.org_cash(command).await), (300, 200));

    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(1, command, CashDir::Withdraw(50)).await;
    assert_eq!((fx.wallet(1).await, fx.org_cash(command).await), (300, 200));
    let command_s = command.to_string();
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        1,
        &[
            ("reason", "no_permission"),
            ("perm", "WithdrawCash"),
            ("org_id", &command_s),
            ("org_type", "command"),
            ("rank", "1"),
            ("direction", "withdraw"),
            ("amount", "50"),
            ("player_cash_before", "300"),
            ("player_cash_after", "300"),
            ("org_cash_before", "200"),
            ("org_cash_after", "200"),
        ],
    );
    assert_eq!(
        lines(&fx.calls_to(1)),
        vec!["Your rank may not withdraw naquadah."]
    );
    assert_eq!(fx.cash_log(command).await.len(), 1, "only the deposit");

    fx.set_member_bits(command, OrgPermission::WITHDRAW_CASH, true)
        .await;
    fx.transfer(1, command, CashDir::Withdraw(50)).await;
    assert_eq!((fx.wallet(1).await, fx.org_cash(command).await), (350, 150));

    fx.set_member_bits(command, OrgPermission::DEPOSIT_CASH, false)
        .await;
    drop(capture);
    fx.clear_sends();
    let capture = LogCapture::install();
    fx.transfer(1, command, CashDir::Deposit(10)).await;
    assert_eq!((fx.wallet(1).await, fx.org_cash(command).await), (350, 150));
    one(
        &fx,
        &capture,
        "org_cash_rejected",
        Level::WARN,
        1,
        &[("reason", "no_permission"), ("perm", "DepositCash")],
    );
    assert_eq!(
        lines(&fx.calls_to(1)),
        vec!["Your rank may not deposit naquadah."]
    );
    fx.teardown().await;
}
