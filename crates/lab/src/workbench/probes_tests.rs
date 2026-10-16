//! Offline checks for the Lua wrapper, limits and saved definitions.

use serde_json::{json, Map, Value};

use super::probe_catalog::{test_catalog, ProbeDefinition, ProbeMode};
use super::probes::{decode_bridge, execute, globals_chunk, run_chunk};

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

#[tokio::test]
async fn fake_executor_surfaces_lua_error_and_timeout() {
    let def = read("error", "error('bad')");
    let result = execute(&def, &Map::new(), |_chunk| async {
        Ok(json!({"ok":true,"results":["{\"status\":\"lua_error\",\"detail\":\"bad\"}"]}))
    })
    .await
    .unwrap();
    assert_eq!(result.status, "lua_error");
    assert_eq!(result.detail.as_deref(), Some("bad"));
}
