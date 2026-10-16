//! Offline checks for the Lua wrapper, limits and saved definitions.

use serde_json::{json, Map, Value};

use super::probe_catalog::{test_catalog, ProbeDefinition, ProbeMode};
use super::probes::{decode_bridge, execute, globals_chunk, run_chunk};
use std::sync::{Arc, Barrier};

#[test]
fn mutation_execution_refuses_missing_or_foreign_lease() {
    let server = crate::daemon::http_tests::test_server();
    let instances = server.instances();
    let leased = crate::daemon::workbench::api::leased;
    assert!(leased(instances, "default", "", "client_lua_eval").is_err());
    assert!(leased(instances, "default", "made-up", "client_lua_eval").is_err());
    assert!(leased(instances, "missing", "made-up", "client_lua_eval").is_err());
}

fn read(name: &str, chunk: &str) -> ProbeDefinition {
    ProbeDefinition::new(name.into(), ProbeMode::Read, chunk.into()).unwrap()
}

#[test]
fn catalog_reload_is_sorted_and_hash_checked() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = test_catalog(temp.path());
    let mut beta = read("beta", "return 2");
    beta.favorite = true;
    catalog.save("default", beta.clone()).unwrap();
    catalog.save("default", read("alpha", "return 1")).unwrap();
    catalog.record("default", "alpha", "ok").unwrap();
    let reopened = test_catalog(temp.path());
    let list = reopened.list("default").unwrap();
    assert_eq!(
        list.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(),
        ["alpha", "beta"]
    );
    assert_eq!(list[1], beta);
    assert_eq!(reopened.history("default").unwrap().len(), 1);
    let path = temp.path().join("workbench/probes/default/beta.json");
    let mut tampered: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    tampered["chunk"] = json!("return 99");
    std::fs::write(path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(reopened
        .list("default")
        .unwrap_err()
        .contains("hash mismatch"));
    assert!(reopened.get("../escape", "beta").is_err());
}

#[test]
fn parameter_data_cannot_escape_the_lua_literal() {
    let def = read("parameter", "local p = ...; return p.text");
    let mut params = Map::new();
    params.insert("text".into(), json!("\"); error('escaped'); --"));
    let chunk = run_chunk(&def, &params).unwrap();
    // This fixture enables the required hook; live client availability is unverified.
    let lua = unsafe { mlua::Lua::unsafe_new() };
    let text: String = lua.load(&chunk).eval().unwrap();
    let result: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(result["status"], "ok");
    assert_eq!(result["value"]["value"], params["text"]);
}

#[test]
fn cyclic_and_huge_lua_returns_are_partial_without_userdata_tostring() {
    // This fixture enables the required hook; live client availability is unverified.
    let lua = unsafe { mlua::Lua::unsafe_new() };
    let cyclic = read("cycle", "local t={}; t.self=t; return t");
    let response: String = lua
        .load(run_chunk(&cyclic, &Map::new()).unwrap())
        .eval()
        .unwrap();
    let outcome = decode_bridge(json!({"ok":true,"results":[response]}));
    assert_eq!(outcome.status, "partial");
    assert_eq!(outcome.value.unwrap()["items"][0]["value"]["kind"], "cycle");

    let huge = read("huge", "return string.rep('x', 100000)");
    let response: String = lua
        .load(run_chunk(&huge, &Map::new()).unwrap())
        .eval()
        .unwrap();
    let outcome = decode_bridge(json!({"ok":true,"results":[response]}));
    assert_eq!(outcome.status, "partial");
    assert!(outcome.value.unwrap()["value"].as_str().unwrap().len() <= 1024);
    let over = decode_bridge(json!({"ok":true,"results":["x".repeat(100000)]}));
    assert_eq!(over.status, "truncated");
}

#[test]
fn global_browser_uses_raw_access_and_marks_opaque_values() {
    let lua = mlua::Lua::new();
    let source = globals_chunk("TestGlobal").unwrap();
    let setup =
        "TestGlobal = setmetatable({x=3}, {__index=function() error('metamethod called') end})";
    lua.load(setup).exec().unwrap();
    let text: String = lua.load(&source).eval().unwrap();
    let outcome = decode_bridge(json!({"ok":true,"results":[text]}));
    assert_eq!(outcome.status, "ok");
    assert_eq!(outcome.value.unwrap()["kind"], "table");
}

#[tokio::test(start_paused = true)]
async fn fake_executor_surfaces_lua_error_and_timeout() {
    let def = read("error", "error('bad')");
    let result = execute(&def, &Map::new(), |_chunk| async {
        Ok(json!({"ok":true,"results":["{\"status\":\"lua_error\",\"detail\":\"bad\"}"]}))
    })
    .await
    .unwrap();
    assert_eq!(result.status, "lua_error");
    assert_eq!(result.detail.as_deref(), Some("bad"));
    let timed_out = execute(&def, &Map::new(), |_chunk| async {
        std::future::pending::<Result<Value, String>>().await
    })
    .await
    .unwrap();
    assert_eq!(timed_out.status, "timeout");
    assert_eq!(timed_out.gaps, ["execution_unknown"]);
}

#[test]
fn valid_escaped_chunk_survives_catalog_reload() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = test_catalog(temp.path());
    let def = read("escaped", &"\u{0001}".repeat(8_192));
    catalog.save("default", def.clone()).unwrap();
    assert_eq!(catalog.get("default", "escaped").unwrap(), Some(def));
    assert!(ProbeDefinition::new("history".into(), ProbeMode::Read, "return 1".into()).is_err());
}

#[test]
fn failed_replace_preserves_existing_probe_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("probe.json");
    std::fs::write(&target, b"old bytes").unwrap();
    assert!(super::probe_catalog::replace_file(&temp.path().join("missing.tmp"), &target).is_err());
    assert_eq!(std::fs::read(target).unwrap(), b"old bytes");
}

#[test]
fn concurrent_versioned_saves_and_history_keep_every_accepted_update() {
    let temp = tempfile::tempdir().unwrap();
    let catalog = test_catalog(temp.path());
    let original = read("race", "return 1");
    catalog.save("default", original.clone()).unwrap();
    let gate = Arc::new(Barrier::new(3));
    let mut threads = Vec::new();
    for chunk in ["return 2", "return 3"] {
        let catalog = catalog.clone();
        let gate = gate.clone();
        let hash = original.sha256.clone();
        threads.push(std::thread::spawn(move || {
            gate.wait();
            catalog.save_checked("default", read("race", chunk), Some(&hash), true)
        }));
    }
    gate.wait();
    let results: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| result.as_ref().err().is_some_and(|e| e == "probe_stale"))
            .count(),
        1
    );
    assert_eq!(
        catalog.get("default", "race").unwrap().unwrap().sha256,
        results.into_iter().find_map(Result::ok).unwrap().sha256
    );

    let gate = Arc::new(Barrier::new(9));
    let mut threads = Vec::new();
    for _ in 0..8 {
        let catalog = catalog.clone();
        let gate = gate.clone();
        threads.push(std::thread::spawn(move || {
            gate.wait();
            catalog.record("default", "race", "ok")
        }));
    }
    gate.wait();
    for thread in threads {
        thread.join().unwrap().unwrap();
    }
    assert_eq!(catalog.history("default").unwrap().len(), 8);
}

#[test]
fn unsupported_lua_keys_mark_tree_partial() {
    let lua = unsafe { mlua::Lua::unsafe_new() };
    let def = read(
        "keys",
        "local t = {}; t[function() end] = 1; t[1.5] = 2; t.good = 3; return t",
    );
    let response: String = lua
        .load(run_chunk(&def, &Map::new()).unwrap())
        .eval()
        .unwrap();
    let outcome = decode_bridge(json!({"ok":true,"results":[response]}));
    assert_eq!(outcome.status, "partial");
    assert_eq!(outcome.gaps, ["tree_truncated"]);
}
