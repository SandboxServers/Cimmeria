use super::*;
const A: &str = "launcher-20261002-aaaaaaa";
const B: &str = "launcher-20261002-bbbbbbb";
fn policy(known: Vec<KnownRelease>) -> CompatibilityPolicy {
    CompatibilityPolicy::new(
        Identity::from_parts("0.1.0", Some("source"), Some(A), Some("1000")),
        known,
    )
}
#[test]
fn minimum_dates_and_tags_are_independent_of_package_semver() {
    let p = policy(vec![]);
    for minimum in [
        None,
        Some(""),
        Some("  "),
        Some(A),
        Some("launcher-20261001-older"),
    ] {
        assert_eq!(p.check(minimum), MinimumStatus::Satisfied);
    }
    assert!(p.check(Some("launcher-20261003-newer")).blocks());
    let mut identity = p.identity().clone();
    identity.desktop_version = "999.0.0".into();
    assert!(CompatibilityPolicy::new(identity, vec![])
        .check(Some("launcher-20261003-newer"))
        .blocks());
}
#[test]
fn same_day_requires_the_minimums_publication_not_hash_order_or_other_release() {
    assert_eq!(
        policy(vec![]).check(Some(B)),
        MinimumStatus::UnknownSameDay { required: B.into() }
    );
    assert!(!policy(vec![KnownRelease {
        tag: "launcher-20261002-zzzzzzz".into(),
        published_at: 2000
    }])
    .check(Some(B))
    .blocks());
    for (published_at, blocked) in [(999, false), (1000, false), (1001, true)] {
        assert_eq!(
            policy(vec![KnownRelease {
                tag: B.into(),
                published_at
            }])
            .check(Some(B))
            .blocks(),
            blocked
        );
    }
    assert_eq!(
        policy(vec![KnownRelease {
            tag: A.into(),
            published_at: 2000
        }])
        .check(Some(A)),
        MinimumStatus::Satisfied
    );
}
#[test]
fn malformed_minimum_and_partial_build_stamps_preserve_legacy_exemptions() {
    for minimum in [
        "1.2.3",
        "launcher-2026102-a",
        "launcher-20261002-",
        "launcher-20261002-../x",
    ] {
        assert_eq!(
            policy(vec![]).check(Some(minimum)),
            MinimumStatus::Malformed {
                required: minimum.into()
            }
        );
    }
    for (tag, epoch) in [
        (None, None),
        (Some(A), None),
        (Some(A), Some("0")),
        (Some(A), Some("-1")),
        (Some("bad"), Some("1000")),
        (Some(A), Some("bad")),
    ] {
        let p = CompatibilityPolicy::new(Identity::from_parts("0.1.0", None, tag, epoch), vec![]);
        assert_eq!(p.check(Some(B)), MinimumStatus::DevelopmentExempt);
        assert_eq!(p.check(None), MinimumStatus::Satisfied);
    }
}
