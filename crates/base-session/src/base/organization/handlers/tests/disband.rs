//! `.org_disband`: the GM gate, the cascade and fanout, and the vault
//! refusal.

use cimmeria_entity::organization::OrgLeaveReason;
use cimmeria_wire::cell::client_methods::organization::build_on_organization_left;
use tracing::Level;

use super::*;
use crate::base::organization::api::VAULT_EMPTY_OVERRIDE;
use crate::base::organization::handlers::{gm_disband, GmCaller, OrgReject, GM_ACCESS_LEVEL};
use crate::test_support::{require_db_or_skip, LogCapture};

/// Character `i` as a GM (or not).
fn gm(fx: &Fixture, i: usize, access_level: u32) -> GmCaller {
    fx.online(i);
    fx.connected
        .lock()
        .unwrap()
        .get_mut(&fx.addr(i))
        .unwrap()
        .access_level = access_level;
    GmCaller {
        entity_id: fx.entity(i),
        player_id: fx.player_id(i),
    }
}

fn disband_row(capture: &crate::test_support::LogCaptureGuard) -> crate::test_support::Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "org" && c.has_field("event", "org.disband"))
        .collect();
    assert_eq!(rows.len(), 1, "exactly one outcome row: {rows:#?}");
    rows.into_iter().next().unwrap()
}

/// A GM disbands a Command of three (two online): the organization, its
/// ranks and its members are gone, each online member gets
/// `onOrganizationLeft(Disbanded)`, and the GM a confirmation line.
#[tokio::test]
async fn gm_disband_cascades_and_tells_online_members() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 9, 4, &["Org06 Disband"]).await;
    let cmd = fx.org(OrgType::Command, "Org06 Disband", 0, &[1, 2]).await;
    fx.online(0);
    fx.online(1);
    let caller = gm(&fx, 3, GM_ACCESS_LEVEL);
    let capture = LogCapture::install();
    assert_eq!(gm_disband(&fx.ctx(), caller, cmd).await, Ok(3));
    let row = disband_row(&capture);
    assert!(row.has_field("outcome", "ok"), "{row:?}");
    assert!(!fx.org_exists(cmd).await);
    assert!(fx.member_ids(cmd).await.is_empty());
    let ranks: i64 =
        sqlx::query_scalar("SELECT count(*) FROM sgw_organization_ranks WHERE org_id = $1")
            .bind(cmd)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ranks, 0);
    let left = (
        36,
        build_on_organization_left(OrgLeaveReason::Disbanded, cmd),
    );
    assert_eq!(fx.calls_to(0), vec![left.clone()]);
    assert_eq!(fx.calls_to(1), vec![left]);
    let lines = feedback_lines(&fx.calls_to(3));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("3 member(s), 2 online told"), "{lines:?}");
    assert!(capture
        .all()
        .iter()
        .any(|c| c.level == Level::DEBUG && c.has_field("event", "disbanded")));
    fx.teardown().await;
}

/// D-ORG20 holds for GMs too: a non-empty vault refuses the disband and
/// nothing is deleted or sent to the members.
#[tokio::test]
async fn gm_disband_refused_while_vault_not_empty() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 10, 3, &["Org06 Disband Vault"]).await;
    let team = fx.org(OrgType::Team, "Org06 Disband Vault", 0, &[1]).await;
    fx.online(0);
    let caller = gm(&fx, 2, GM_ACCESS_LEVEL);
    let capture = LogCapture::install();
    let r = VAULT_EMPTY_OVERRIDE
        .scope(false, gm_disband(&fx.ctx(), caller, team))
        .await;
    assert_eq!(r, Err(OrgReject::VaultNotEmpty));
    let row = disband_row(&capture);
    assert!(row.has_field("reason", "vault_not_empty"), "{row:?}");
    assert!(fx.org_exists(team).await);
    assert_eq!(fx.member_ids(team).await.len(), 2);
    assert!(fx.calls_to(0).is_empty());
    assert_eq!(feedback_lines(&fx.calls_to(2)).len(), 1);
    fx.teardown().await;
}

/// The base re-reads the access level: a forwarded disband from a session
/// below GameMaster is refused and deletes nothing.
#[tokio::test]
async fn gm_disband_rejects_non_gm() {
    let pool = require_db_or_skip!();
    let fx = Fixture::new(&pool, 11, 2, &["Org06 Not Gm"]).await;
    let team = fx.org(OrgType::Team, "Org06 Not Gm", 0, &[]).await;
    let caller = gm(&fx, 1, GM_ACCESS_LEVEL - 1);
    let capture = LogCapture::install();
    assert_eq!(
        gm_disband(&fx.ctx(), caller, team).await,
        Err(OrgReject::NotGm)
    );
    let row = disband_row(&capture);
    assert!(row.has_field("reason", "not_gm"), "{row:?}");
    assert!(fx.org_exists(team).await);
    fx.teardown().await;
}
