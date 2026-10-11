//! CH-05: the chaos tools are listed only in a `lab-chaos` build, refuse
//! while the registry is not installed, and pass every bound to the
//! registry unclamped. Status names entity ids, never an address.

use std::sync::Arc;

use cimmeria_admin_api::ws::broadcast_layer::LogBuffer;
use cimmeria_services::orchestrator::Orchestrator;

use super::LabTools;
use crate::state::LabState;

const CHAOS_TOOLS: [&str; 3] = [
    "server_chaos_set",
    "server_chaos_clear",
    "server_chaos_status",
];

fn tools() -> LabTools {
    let orchestrator = Arc::new(Orchestrator::new(Default::default()));
    LabTools::new(LabState::new(orchestrator, LogBuffer::new()))
}

#[tokio::test]
async fn chaos_tools_are_listed_only_in_a_lab_chaos_build() {
    let tools = tools();
    assert!(tools.tool_router.has_route("server_sessions"));
    for name in CHAOS_TOOLS {
        assert_eq!(
            tools.tool_router.has_route(name),
            cfg!(feature = "lab-chaos"),
            "{name} must be listed exactly when the feature is on"
        );
    }
}

#[cfg(feature = "lab-chaos")]
mod with_feature {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    use rmcp::handler::server::wrapper::Parameters;
    use rmcp::model::CallToolResult;
    use serde_json::{json, Value};

    use cimmeria_services::base::lab_chaos::FlushChaos;
    use cimmeria_services::base::lab_chaos::LabChaos;
    use cimmeria_services::base::OnlinePlayer;

    use super::super::chaos::{
        built_label, clear_on, flush_from, net_from, set_on, status_on, ClearArgs, FlushArgs,
        NetArgs, SetArgs, ShiftArg, OFF,
    };
    use super::tools;

    const ADDR_A: &str = "127.0.0.1:50000";
    const ADDR_B: &str = "127.0.0.1:50001";

    fn player(id: u32, session: &str) -> OnlinePlayer {
        OnlinePlayer {
            id,
            name: format!("Lab{id}"),
            player_id: Some(i32::try_from(id).expect("test ids fit i32")),
            account_id: 7,
            account_name: None,
            archetype: "Soldier",
            level: 5,
            zone: "Castle_CellBlock".to_string(),
            ping: None,
            status: "in_world",
            session: session.to_string(),
        }
    }

    fn pack_args(entity_id: u32) -> SetArgs {
        SetArgs {
            entity_id,
            net: None,
            flush: Some(FlushArgs {
                mode: "pack".to_string(),
                delay_ms: None,
                records: Some(24),
                shift: None,
            }),
            seed: Some(7),
            ttl_s: None,
            label: None,
        }
    }

    /// Sets with the same roster on both reads.
    async fn set_fixed(
        registry: &LabChaos,
        args: SetArgs,
        roster: &[OnlinePlayer],
    ) -> Result<Value, String> {
        set_on(registry, args, || {
            let r = roster.to_vec();
            async move { r }
        })
        .await
    }

    fn reply(result: CallToolResult) -> Value {
        assert_ne!(result.is_error, Some(true), "a refusal is not a tool error");
        let text = &result.content[0]
            .as_text()
            .expect("the reply is JSON text")
            .text;
        serde_json::from_str(text).expect("the reply parses")
    }

    #[tokio::test]
    async fn every_tool_refuses_when_the_registry_is_not_installed() {
        // Nothing in this test binary installs the registry (only the
        // server's main may), so `current()` is None here.
        let tools = tools();
        let set = tools.server_chaos_set(Parameters(pack_args(100))).await;
        let clear = tools
            .server_chaos_clear(Parameters(ClearArgs { entity_id: None }))
            .await;
        let status = tools.server_chaos_status().await;
        for result in [set, clear, status] {
            let v = reply(result.expect("a refusal is a successful call"));
            assert_eq!(v, json!({ "refused": OFF }));
        }
    }

    #[tokio::test]
    async fn set_without_ttl_applies_the_default_and_status_shows_the_entity() {
        let registry = LabChaos::new();
        let roster = [player(100, ADDR_A)];
        let v = set_fixed(&registry, pack_args(100), &roster)
            .await
            .expect("set succeeds");
        assert_eq!(v, json!({ "set": true, "expires_in_s": 300 }));

        let status = status_on(&registry, Instant::now());
        let row = &status["sessions"][0];
        assert_eq!(row["entity_id"], 100);
        assert!(row["expires_in_s"].as_u64().is_some_and(|s| s <= 300));
        assert_eq!(row["flush"], json!({ "mode": "pack", "records": 24 }));
        assert!(row.get("net").is_none());
        assert!(
            !status.to_string().contains("127.0.0.1"),
            "status names entity ids, never an address: {status}"
        );
    }

    #[tokio::test]
    async fn a_named_ttl_reaches_the_registry_unchanged() {
        let registry = LabChaos::new();
        let roster = [player(100, ADDR_A)];
        let set = |ttl_s| SetArgs {
            ttl_s: Some(ttl_s),
            ..pack_args(100)
        };
        assert_eq!(
            set_fixed(&registry, set(1801), &roster).await,
            Err("ttl 1801s is over the 1800 s cap".to_string()),
            "over the cap is refused, not clamped"
        );
        assert_eq!(
            set_fixed(&registry, set(0), &roster).await,
            Err("ttl 0s is not a usable expiry".to_string()),
            "a named 0 is not replaced by the default"
        );
        assert_eq!(
            status_on(&registry, Instant::now()),
            json!({ "sessions": [] })
        );
        assert_eq!(
            set_fixed(&registry, set(60), &roster).await,
            Ok(json!({ "set": true, "expires_in_s": 60 }))
        );
    }

    #[tokio::test]
    async fn a_knob_out_of_bounds_gets_the_registry_refusal() {
        let registry = LabChaos::new();
        let args = SetArgs {
            net: Some(NetArgs {
                latency: Some(6000),
                ..NetArgs::default()
            }),
            flush: None,
            ..pack_args(100)
        };
        assert_eq!(
            set_fixed(&registry, args, &[player(100, ADDR_A)]).await,
            Err("bad chaos knob: net latency is over 5000 ms".to_string())
        );
    }

    #[tokio::test]
    async fn set_refuses_an_entity_with_no_session() {
        let registry = LabChaos::new();
        assert_eq!(
            set_fixed(&registry, pack_args(100), &[player(200, ADDR_A)]).await,
            Err("no session for entity 100".to_string())
        );
        assert_eq!(
            status_on(&registry, Instant::now()),
            json!({ "sessions": [] })
        );
    }

    /// The CH-04 review's race: the entity's session ends between resolving
    /// its address and `set`, and the address goes to someone else or no one.
    #[tokio::test]
    async fn set_drops_its_entry_when_the_address_changed_hands_meanwhile() {
        for after in [
            vec![player(100, ADDR_B), player(200, ADDR_A)],
            vec![player(200, ADDR_A)],
            vec![],
        ] {
            let registry = LabChaos::new();
            let reads = AtomicUsize::new(0);
            let result = set_on(&registry, pack_args(100), || {
                let r = if reads.fetch_add(1, Ordering::SeqCst) == 0 {
                    vec![player(100, ADDR_A)]
                } else {
                    after.clone()
                };
                async move { r }
            })
            .await;
            assert_eq!(result, Err("no session for entity 100".to_string()));
            assert_eq!(reads.load(Ordering::SeqCst), 2, "the roster is read again");
            assert_eq!(
                status_on(&registry, Instant::now()),
                json!({ "sessions": [] }),
                "no chaos is left on an address the entity lost"
            );
        }
    }

    #[tokio::test]
    async fn clear_returns_how_many_it_removed() {
        let registry = LabChaos::new();
        let roster = [player(100, ADDR_A), player(200, ADDR_B)];
        for id in [100, 200] {
            set_fixed(&registry, pack_args(id), &roster)
                .await
                .expect("set succeeds");
        }
        assert_eq!(
            clear_on(&registry, &roster, Some(100)),
            json!({ "cleared": 1 })
        );
        assert_eq!(
            clear_on(&registry, &roster, Some(100)),
            json!({ "cleared": 0 })
        );
        assert_eq!(
            clear_on(&registry, &roster, Some(999)),
            json!({ "cleared": 0 }),
            "an entity with no session clears nothing"
        );
        assert_eq!(clear_on(&registry, &roster, None), json!({ "cleared": 1 }));
    }

    #[tokio::test]
    async fn status_lists_at_most_ten_sessions() {
        let registry = LabChaos::new();
        let roster: Vec<OnlinePlayer> = (0..12)
            .map(|i| player(100 + i, &format!("127.0.0.1:{}", 50000 + i)))
            .collect();
        for p in &roster {
            set_fixed(&registry, pack_args(p.id), &roster)
                .await
                .expect("set succeeds");
        }
        let status = status_on(&registry, Instant::now());
        let rows = status["sessions"].as_array().expect("a list");
        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0]["entity_id"], 100, "sorted by entity id");
    }

    #[tokio::test]
    async fn the_label_is_the_given_text_or_built_from_the_profiles() {
        let registry = LabChaos::new();
        let roster = [player(100, ADDR_A), player(200, ADDR_B)];
        let given = SetArgs {
            label: Some("flush:pack:records=24 seed=7".to_string()),
            ..pack_args(100)
        };
        set_fixed(&registry, given, &roster).await.expect("set");
        set_fixed(&registry, pack_args(200), &roster)
            .await
            .expect("set");
        let labels: Vec<String> = registry
            .status(Instant::now())
            .into_iter()
            .map(|s| s.label)
            .collect();
        assert_eq!(labels, ["flush:pack:records=24 seed=7", "flush:pack"]);

        let long = SetArgs {
            label: Some("x".repeat(121)),
            ..pack_args(100)
        };
        assert!(set_fixed(&registry, long, &roster)
            .await
            .is_err_and(|e| e.starts_with("bad chaos knob: label")));
    }

    #[test]
    fn net_percentages_become_per_thousand_and_negatives_are_refused() {
        let net = net_from(
            &NetArgs {
                latency: Some(120),
                loss: Some(1.2),
                dup: Some(0.05),
                ..NetArgs::default()
            },
            Some(7),
        )
        .expect("valid knobs");
        assert_eq!(net.latency_ms, 120);
        assert_eq!(net.loss_per_thousand, 12);
        assert_eq!(net.dup_per_thousand, 1, "0.05 percent rounds to 1");
        assert_eq!(net.seed, 7);
        assert_eq!(built_label(Some(&net), None), "net seed=7");

        let negative = NetArgs {
            loss: Some(-1.0),
            ..NetArgs::default()
        };
        assert_eq!(
            net_from(&negative, None),
            Err("bad chaos knob: net loss is negative".to_string())
        );
    }

    #[test]
    fn flush_modes_resolve_and_a_missing_knob_is_refused() {
        let flush = |mode: &str, shift: Option<ShiftArg>| FlushArgs {
            mode: mode.to_string(),
            delay_ms: None,
            records: None,
            shift,
        };
        assert_eq!(
            flush_from(
                &flush("shift", Some(ShiftArg::Word("sweep".into()))),
                Some(9)
            ),
            Ok(FlushChaos::ShiftSweep { seed: 9 })
        );
        assert_eq!(
            flush_from(&flush("shift", Some(ShiftArg::Bytes(4))), None),
            Ok(FlushChaos::Shift { len: 4 })
        );
        assert_eq!(
            flush_from(&flush("reorder", None), None),
            Ok(FlushChaos::Reorder { seed: 0 })
        );
        for (mode, shift) in [
            ("shift", Some(ShiftArg::Word("wide".into()))),
            ("shift", None),
            ("delay", None),
            ("pack", None),
            ("burst", None),
        ] {
            assert!(
                flush_from(&flush(mode, shift), None)
                    .is_err_and(|e| e.starts_with("bad chaos knob: ")),
                "{mode} must be refused"
            );
        }
        let reorder = FlushChaos::Reorder { seed: 3 };
        assert_eq!(built_label(None, Some(&reorder)), "flush:reorder seed=3");
    }
}
