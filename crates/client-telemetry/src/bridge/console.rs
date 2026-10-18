//! Fail-closed legacy `client_console` endpoint.
//!
//! The known `execConsoleCommand` entry takes an `FFrame`, and no
//! string-taking address or `FString` ABI has been proven for the installed
//! client. A caller-supplied address cannot establish that proof. Stock chat
//! submission is tracked separately by the lab's slash transport candidate.

use serde::Deserialize;
use serde_json::Value;

use super::dispatch::{RpcResponse, INVALID_PARAMS};

/// No proven built-in string-console entry exists.
pub const CONSOLE_EXEC_ADDR: usize = 0;

#[derive(Deserialize)]
struct ConsoleParams {
    line: String,
    #[serde(default)]
    addr: Option<Value>,
}

/// Encode a prospective raw string argument for offline ABI research only.
/// This buffer is never passed to a client function by this endpoint.
pub fn encode_console_string(line: &str, wide: bool) -> Vec<u8> {
    if wide {
        let mut out = Vec::with_capacity((line.len() + 1) * 2);
        for unit in line.encode_utf16().chain(core::iter::once(0)) {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out
    } else {
        let mut out = line.as_bytes().to_vec();
        out.push(0);
        out
    }
}

/// Reject native console calls until a fingerprinted address and ABI exist.
pub fn dispatch(id: Value, params: &Value) -> RpcResponse {
    let p: ConsoleParams = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => return RpcResponse::error(id, INVALID_PARAMS, format!("console params: {e}")),
    };
    if p.line.is_empty() || p.line.len() > 256 || p.line.chars().any(char::is_control) {
        return RpcResponse::error(id, INVALID_PARAMS, "console: invalid command line");
    }
    let detail = if p.addr.is_some() {
        "caller-supplied address is unproved"
    } else {
        "no console-exec address is proved"
    };
    RpcResponse::error(
        id,
        INVALID_PARAMS,
        format!(
            "console: {detail}; execConsoleCommand @0x539850 takes FFrame, not a string; \
             string-taking address, ABI, and client fingerprint must be verified first"
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn offline_encoding_is_nul_terminated() {
        assert_eq!(encode_console_string("/x", true), vec![47, 0, 120, 0, 0, 0]);
        assert_eq!(encode_console_string("/x", false), vec![47, 120, 0]);
    }

    #[test]
    fn unproved_address_and_abi_are_refused() {
        for params in [
            json!({ "line": "/who" }),
            json!({ "line": "/who", "addr": "0x539850", "conv": "thiscall" }),
            json!({ "line": "/who", "addr": "0x600000", "this": "0x1" }),
        ] {
            let e = dispatch(json!(1), &params).error.expect("rejected");
            assert_eq!(e.code, INVALID_PARAMS);
            assert!(e.message.contains("unproved") || e.message.contains("no console-exec"));
        }
    }

    #[test]
    fn invalid_lines_are_refused_before_address_lookup() {
        for line in ["", "/who\nnext", &"/".repeat(257)] {
            let e = dispatch(json!(2), &json!({ "line": line }))
                .error
                .expect("rejected");
            assert!(e.message.contains("invalid command line"));
        }
    }
}
