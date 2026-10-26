//! Unit tests of the access-token fences (contract C16.3).

use std::time::Duration;

use super::*;

const ACCOUNT: u32 = 7;
const OTHER: u32 = 8;

/// Whether a mint for `family` of `ACCOUNT`, fenced at `fence`, is refused.
fn refused(access: &AccessTokens, family: &str, fence: u64, now: Instant) -> bool {
    access
        .mint_fenced(ACCOUNT, family, true, now, fence)
        .is_none()
}

#[test]
fn a_mint_fenced_before_a_family_revoke_is_refused() {
    let access = AccessTokens::default();
    let now = Instant::now();
    let fence = access.fence();
    access.revoke_family("fam-a");
    assert!(
        refused(&access, "fam-a", fence, now),
        "a mint fenced before its family's revoke was allowed"
    );
    assert!(
        access
            .mint_fenced(ACCOUNT, "fam-b", true, now, fence)
            .is_some(),
        "a revoke of one family refused another family"
    );
}

#[test]
fn a_mint_fenced_after_the_revoke_succeeds() {
    let access = AccessTokens::default();
    let now = Instant::now();
    access.revoke_family("fam-a");
    let fence = access.fence();
    assert!(
        access
            .mint_fenced(ACCOUNT, "fam-a", true, now, fence)
            .is_some(),
        "a mint fenced after the revoke was refused"
    );
}

#[test]
fn account_revoke_refuses_every_family_and_except_keeps_one() {
    let access = AccessTokens::default();
    let now = Instant::now();
    let fence = access.fence();
    access.revoke_account(ACCOUNT);
    assert!(refused(&access, "fam-a", fence, now));
    assert!(refused(&access, "fam-b", fence, now));
    assert!(
        access
            .mint_fenced(OTHER, "fam-c", true, now, fence)
            .is_some(),
        "a revoke of one account refused another account"
    );

    let fence = access.fence();
    access.revoke_account_except(ACCOUNT, "fam-a");
    assert!(
        !refused(&access, "fam-a", fence, now),
        "the kept family was refused by its account's except"
    );
    assert!(
        refused(&access, "fam-b", fence, now),
        "a family other than the kept one was not refused"
    );
}

#[test]
fn a_second_except_with_another_keep_revokes_the_first_keep() {
    let access = AccessTokens::default();
    let now = Instant::now();
    let before = access.fence();
    access.revoke_account_except(ACCOUNT, "fam-a");
    let between = access.fence();
    access.revoke_account_except(ACCOUNT, "fam-b");
    assert!(
        refused(&access, "fam-b", before, now),
        "the first keep kept its exemption after a second except with another keep"
    );
    assert!(
        !refused(&access, "fam-b", between, now),
        "the second keep was refused by its own except"
    );
    assert!(
        refused(&access, "fam-a", between, now),
        "the first keep was not refused after the second except"
    );
}

#[test]
fn grant_live_is_false_for_a_grant_minted_before_its_family_revoke() {
    let access = AccessTokens::default();
    let now = Instant::now();
    let token = access.mint(ACCOUNT, "fam-a", true, now);
    let grant = access.check(&token, now).expect("the grant is held");
    assert!(access.grant_live(&grant), "a fresh grant is not live");

    access.revoke_family("fam-a");
    assert!(!access.grant_live(&grant), "a revoked grant is still live");

    // A grant that is held again after its family's revoke is still refused:
    // the tombstone decides, not the map.
    locked(&access.inner)
        .by_hash
        .insert(hash_of(&token), grant.clone());
    assert!(
        !access.grant_live(&grant),
        "a grant minted before its family's revoke was live"
    );
}

#[test]
fn tombstones_expire_after_access_life_and_count_against_the_cap() {
    let access = AccessTokens::default();
    let base = Instant::now();
    let fence = access.fence();
    access.revoke_family("fam-a");
    access.reap(base + Duration::from_secs(1));
    assert!(
        refused(&access, "fam-a", fence, base),
        "a tombstone was reaped before its life ended"
    );
    access.reap(base + ACCESS_LIFE + Duration::from_secs(1));
    assert!(
        !refused(&access, "fam-a", fence, base),
        "an expired tombstone still refused a mint"
    );

    let mut inner = Inner::default();
    for i in 0..STORE_CAP as u64 {
        inner
            .family_revoked
            .insert(format!("fam-{i}"), (i + 1, base + Duration::from_millis(i)));
    }
    make_room(&mut inner);
    assert_eq!(inner.family_revoked.len(), STORE_CAP - 1);
    assert!(
        !inner.family_revoked.contains_key("fam-0"),
        "the oldest tombstone survived a full store"
    );
    assert!(inner.family_revoked.contains_key("fam-1"));
}
