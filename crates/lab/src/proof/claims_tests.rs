//! Tests for the claim list: every name is routed and classified once.

use std::collections::HashSet;

use super::{claim, Claim, JUDGED, NO_CLAIM};
use crate::daemon::http_tests::test_server;
use crate::lease::policy;

#[test]
fn claim_lists_name_routed_tools() {
    let server = test_server();
    let routed: HashSet<String> = server
        .tool_router
        .list_all()
        .into_iter()
        .map(|t| t.name.to_string())
        .collect();
    for name in JUDGED.iter().chain(NO_CLAIM) {
        assert!(routed.contains(*name), "{name} is not a routed tool");
    }
}

#[test]
fn no_name_is_in_two_claim_lists() {
    for name in JUDGED {
        assert!(!NO_CLAIM.contains(name), "{name} is in both lists");
    }
}

#[test]
fn no_claim_tool_is_not_leased_open() {
    for name in NO_CLAIM {
        assert!(!policy::OPEN.contains(name), "{name} is also in OPEN");
    }
}

#[test]
fn unlisted_tools_are_unjudged() {
    assert_eq!(claim("client_some_future_tool"), Claim::Unjudged);
    assert_eq!(claim("lab_client_status"), Claim::NoClaim);
    assert_eq!(claim("client_lua_eval"), Claim::NoClaim);
}
