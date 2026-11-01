//! `ui_clickables` against a fake bridge that answers the walk with a canned
//! page (the Lua return shape, as the client sends it).

use std::sync::Arc;

use serde_json::{json, Value};

use super::*;
use crate::lease::policy::OPEN;
use crate::proof::claims::{claim, Claim, NO_CLAIM};
use crate::supervisor::events::fake_bridge::{self, lua_ok, Responder};

/// A supervisor whose every `lua_eval` answers with `page`.
async fn answering(page: Value) -> Supervisor {
    let text = page.to_string();
    let responder: Responder = Arc::new(move |method, _params| {
        if method != "lua_eval" {
            return Err(format!("unexpected call: {method}"));
        }
        Ok(lua_ok(std::slice::from_ref(&text)))
    });
    fake_bridge::supervisor(responder).await
}

#[tokio::test]
async fn auto_flag_and_empty_fields() {
    let sup = answering(json!({
        "found": true, "truncated": false, "visited": 2,
        "windows": [
            { "name": "VendorWin__auto_closebutton__", "text": "",
              "visible": true, "enabled": true, "parent": "VendorWin" },
            { "name": "BuyButton", "type": "TaharezLook/Button",
              "text": "Buy", "rect": [10, 20, 80, 24],
              "visible": true, "enabled": true, "parent": "VendorWin" }
        ]
    }))
    .await;
    let out = sup
        .ui_clickables(Some("VendorWin"), false, None, None)
        .await
        .unwrap();
    let w = out["windows"].as_array().unwrap();
    assert_eq!(w[0]["auto"], true);
    assert_eq!(w[1]["auto"], false);
    assert!(w[0].get("type").is_none(), "no type key");
    assert!(w[0].get("rect").is_none(), "no rect key");
    assert_eq!(w[1]["type"], "TaharezLook/Button");
    assert_eq!(w[1]["rect"], json!([10, 20, 80, 24]));
}

#[tokio::test]
async fn truncated_is_reported() {
    let page = |next: Option<u32>| {
        let mut p = json!({ "found": true, "windows": {}, "truncated": true, "visited": 2000 });
        if let Some(n) = next {
            p["next"] = json!(n);
        }
        p
    };
    let sup = answering(page(Some(2000))).await;
    let out = sup.ui_clickables(None, false, None, None).await.unwrap();
    assert_eq!(out["truncated"], true);
    assert_eq!(out["visited"], 2000);
    assert_eq!(out["next"], 2000);
    assert_eq!(out["windows"], json!([]));

    let sup = answering(page(None)).await;
    let out = sup.ui_clickables(None, false, None, None).await.unwrap();
    assert!(out["next"].is_null(), "no next in the walk gives null");
}

#[tokio::test]
async fn missing_root_is_an_error() {
    let sup = answering(json!({ "found": false })).await;
    let e = sup
        .ui_clickables(Some("NoSuchWin"), false, None, None)
        .await
        .unwrap_err();
    assert!(e.contains("NoSuchWin"), "{e}");
}

#[test]
fn clickables_is_open_and_no_claim() {
    assert!(OPEN.contains(&"client_ui_clickables"));
    assert!(!NO_CLAIM.contains(&"client_ui_clickables"));
    assert_eq!(claim("client_ui_clickables"), Claim::NoClaim);
}
