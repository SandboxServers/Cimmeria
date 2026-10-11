//! `lab_uat_run` argument and instance tests (moved out of `uat.rs`, which
//! sat at the 700-line cap, when lab-chaos CH-08 added `chaos`).

use super::*;

/// TG-PIPE-08: a run ends its session unless the caller says not to.
#[test]
fn uat_run_ends_the_session_by_default() {
    let args = |v| serde_json::from_value::<UatRunArgs>(v).unwrap().end_session;
    assert!(args(json!({})));
    assert!(!args(json!({ "end_session": false })));
}

/// lab-chaos CH-08: `chaos` reaches the run as typed, and a bad spec or a
/// `burst` (not wired yet) refuses the call, plan or not, before a lease or
/// a client is touched. Fails if the args drop the field or the check goes.
#[test]
fn uat_run_args_parse_chaos_and_refuse_a_bad_profile() {
    let args = |v| serde_json::from_value::<UatRunArgs>(v).unwrap();
    let ok = args(json!({ "chaos": ["cpu", "flush:pack"], "plan_only": true }));
    assert_eq!(
        run_chaos(&ok).unwrap(),
        vec!["cpu".to_string(), "flush:pack".to_string()]
    );
    assert!(run_chaos(&args(json!({}))).unwrap().is_empty());
    let refused = |spec: &str| {
        run_chaos(&args(json!({ "chaos": [spec] })))
            .unwrap_err()
            .message
            .to_string()
    };
    assert!(refused("disk").contains("unknown chaos profile"));
    assert!(refused("burst").contains("burst is not wired yet"));
    assert!(refused("net:loss=0.05").contains("one decimal"));
}

/// Regression guard: the chaos check runs before the lease is taken. With
/// the lab leased by someone else, a bad spec is refused for its chaos, not
/// for the lease; a good one then meets the lease. Fails if the check moves
/// after the lease acquisition.
#[tokio::test]
async fn a_bad_chaos_spec_is_refused_before_the_lease_is_taken() {
    let server = LabServer::new(supervisor(None));
    server
        .supervisor
        .leases()
        .acquire(crate::lease::AcquireRequest {
            owner: "other".into(),
            purpose: "busy".into(),
            ..Default::default()
        })
        .unwrap();
    let args = |v| serde_json::from_value::<UatRunArgs>(v).unwrap();
    let Err(bad) = server.run_start(&args(json!({ "chaos": ["disk"] }))) else {
        panic!("a bad spec is refused");
    };
    assert!(
        bad.message.contains("unknown chaos profile"),
        "{}",
        bad.message
    );
    let Err(good) = server.run_start(&args(json!({ "chaos": ["cpu"] }))) else {
        panic!("the lab is leased");
    };
    assert!(!good.message.contains("chaos"), "{}", good.message);
    // A plan takes no lease, so it passes with the lab leased.
    let plan = server.run_start(&args(json!({ "chaos": ["cpu"], "plan_only": true })));
    assert!(matches!(plan, Ok((_, None))));
}

#[test]
fn the_self_instance_check_ignores_case() {
    assert!(is_same_instance(Some("p2"), "p2"));
    assert!(is_same_instance(Some("P2"), "p2"));
    assert!(is_same_instance(Some("p2"), "P2"));
    assert!(!is_same_instance(None, "p2"));
    assert!(!is_same_instance(Some("p3"), "p2"));
}

/// Regression guard (LP-05b2): a takeover of p2 stops p2's actions even
/// inside p1's run scope, because each action checks p2's own lease.
#[tokio::test]
async fn p2_actions_check_p2s_own_lease() {
    use crate::lease::{permit, AcquireRequest, LeaseBook};
    let b1 = Arc::new(LeaseBook::default());
    let b2 = Arc::new(LeaseBook::default());
    let p1 = RunLease::acquire_own(b1.clone(), "p1".into()).unwrap();
    let p2 = RunLease::acquire_own(b2.clone(), "p2".into()).unwrap();
    permit::scope(
        p1.permit(),
        under_lease(Some(&p2), async {
            assert!(permit::ensure("bridge x").is_ok());
            b2.acquire(AcquireRequest {
                owner: "other".into(),
                purpose: "takeover".into(),
                force: true,
                reason: Some("test".into()),
                ..Default::default()
            })
            .unwrap();
            assert!(
                permit::ensure("bridge x").is_err(),
                "a takeover of p2 must stop p2's actions"
            );
        }),
    )
    .await;
}

fn supervisor(instance: Option<&str>) -> Arc<Supervisor> {
    let config = SupervisorConfig {
        install_dir: None,
        dll_path: None,
        patches_dll: None,
        helper_path: None,
        bind: "127.0.0.1".into(),
        port: 8770,
        instance: instance.map(String::from),
        telemetry: Default::default(),
    };
    let bridge = Arc::new(BridgeClient::new("127.0.0.1:1", ""));
    Arc::new(Supervisor::new(bridge, config))
}

/// A hosted p2 is the second player's own supervisor; a single-instance
/// daemon falls back to the in-process p2, never its own supervisor.
#[test]
fn p2_is_the_hosted_instance_or_the_in_process_fallback() {
    let s2 = supervisor(Some("p2"));
    let hosted = LabServer::new_multi(crate::server::Instances::new(vec![
        crate::server::Hosted {
            label: "default".into(),
            supervisor: supervisor(None),
        },
        crate::server::Hosted {
            label: "p2".into(),
            supervisor: s2.clone(),
        },
    ]));
    assert!(Arc::ptr_eq(&hosted.p2_lab("p2").supervisor, &s2));
    let own = supervisor(None);
    let single = LabServer::new(own.clone());
    assert!(!Arc::ptr_eq(&single.p2_lab("p2").supervisor, &own));
}
