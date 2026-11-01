//! Server corroboration (D-AX11): asks the server whether the client's
//! claim holds. An absent or failed server never denies: it is
//! `server_unreachable` (`ok` absent), not `false`. Only a server that
//! answers can say `server_disagrees`.

use std::sync::{Arc, OnceLock};

use serde_json::Value;

#[cfg(test)]
use serde_json::json;

#[cfg(test)]
use super::{Check, Source, Why};
use crate::uat::invoke::ServerTools;
#[cfg(test)]
use crate::uat::invoke::{ServerInvoker, ToolOutcome};

/// What the server should confirm about the world.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub enum Expect {
    /// The entity's position is within `within_m` metres of `at` (server metres).
    EntityPosNear {
        entity_id: u32,
        at: [f64; 3],
        within_m: f64,
    },
    /// The JSON pointer, read from the `server_entity_get` reply, equals `equals`.
    EntityField {
        entity_id: u32,
        pointer: String,
        equals: Value,
    },
    /// A server log line containing `contains`, logged after `since_ms`.
    LogLine { contains: String, since_ms: i64 },
}

/// The in-server lab endpoint from the environment, built once per process.
#[allow(dead_code)] // Callers land in AX-32.
pub fn default_server() -> Option<Arc<ServerTools>> {
    static SERVER: OnceLock<Option<Arc<ServerTools>>> = OnceLock::new();
    SERVER
        .get_or_init(|| ServerTools::from_env().map(Arc::new))
        .clone()
}

/// The position in a `server_entity_get` reply (`/entity/position`).
pub fn entity_position(reply: &Value) -> Option<[f64; 3]> {
    match reply.pointer("/entity/position")?.as_array()?.as_slice() {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

/// Asks the server about `e` and returns one check named `name`. `server`
/// `None`, a failed call or a malformed reply leave `ok` absent.
#[cfg(test)]
pub async fn check(
    server: Option<&dyn ServerInvoker>,
    name: &str,
    e: &Expect,
    required: bool,
) -> Check {
    let Some(server) = server else {
        return unreachable(name, required);
    };
    match e {
        Expect::EntityPosNear {
            entity_id,
            at,
            within_m,
        } => {
            let Some(reply) = entity_reply(server, *entity_id).await else {
                return unreachable(name, required);
            };
            if gone(&reply) {
                return gone_check(name, required).with_before(json!(at));
            }
            let Some(pos) = entity_position(&reply) else {
                return unreachable(name, required);
            };
            let ok = distance(pos, *at) <= *within_m;
            let c = Check::new(name, Source::Server, Some(ok), required)
                .with_before(json!(at))
                .with_after(json!(pos));
            if ok {
                c
            } else {
                c.with_why(Why::ServerDisagrees)
            }
        }
        Expect::EntityField {
            entity_id,
            pointer,
            equals,
        } => {
            let Some(reply) = entity_reply(server, *entity_id).await else {
                return unreachable(name, required);
            };
            if gone(&reply) {
                return gone_check(name, required);
            }
            let got = reply.pointer(pointer);
            let ok = got == Some(equals);
            let c = Check::new(name, Source::Server, Some(ok), required)
                .with_after(got.cloned().unwrap_or(Value::Null));
            if ok {
                c
            } else {
                c.with_why(Why::ServerDisagrees)
            }
        }
        Expect::LogLine { contains, since_ms } => {
            let out = server
                .call(
                    "server_log_tail",
                    json!({
                        "contains": contains,
                        "since_ms": since_ms,
                        "limit": 50,
                        "include_client": false,
                    }),
                )
                .await;
            if !out.ok {
                return unreachable(name, required);
            }
            let Some(entries) = out.json.get("entries").and_then(Value::as_array) else {
                return unreachable(name, required);
            };
            let found = !entries.is_empty();
            let c = Check::new(name, Source::Server, Some(found), required)
                .with_after(json!(entries.len()));
            if found {
                c
            } else {
                c.with_why(Why::ServerDisagrees)
            }
        }
    }
}

/// The `server_entity_get` reply for `entity_id`, or `None` when the call failed.
#[cfg(test)]
async fn entity_reply(server: &dyn ServerInvoker, entity_id: u32) -> Option<Value> {
    let out: ToolOutcome = server
        .call("server_entity_get", json!({ "entity_id": entity_id }))
        .await;
    out.ok.then_some(out.json)
}

/// The server answers that the entity no longer exists (`entity: null`).
#[cfg(test)]
fn gone(reply: &Value) -> bool {
    reply.get("entity") == Some(&Value::Null)
}

#[cfg(test)]
fn gone_check(name: &str, required: bool) -> Check {
    Check::new(name, Source::Server, Some(false), required).with_why(Why::ServerDisagrees)
}

#[cfg(test)]
fn unreachable(name: &str, required: bool) -> Check {
    Check::new(name, Source::Server, None, required).with_why(Why::ServerUnreachable)
}

#[cfg(test)]
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
#[path = "corroborate_tests.rs"]
mod tests;
