//! The MCP handler for [`LabServer`]: each `tools/call` is routed to its
//! lab instance ([`super::instances`]), passes the lease gate and has its
//! result compacted; `tools/list` advertises the lease and instance
//! arguments.

use std::time::Instant;

use rmcp::{model::*, tool_handler, ErrorData as McpError, ServerHandler};
use serde_json::Value;
use tracing::Instrument;

use super::{compact, lease, LabServer};

/// The longest `error` text a `lab.call` row carries.
const ERROR_CAP: usize = 200;

/// The owner of the lease held right now, if one is held.
fn holder_owner(lease_status: &Value) -> Option<String> {
    lease_status
        .pointer("/lease/owner")
        .and_then(Value::as_str)
        .map(String::from)
}

/// Write the one INFO `lab.call` row for a finished `tools/call`. Outcome is
/// `refused` when the lease gate turned the call away, `error` for a failed
/// tool, else `ok`. The arguments are never logged: they can carry Lua or
/// chat text.
fn log_call(
    instance: &str,
    tool: &str,
    lease_owner: Option<&str>,
    started: Instant,
    refused: bool,
    out: &Result<CallToolResponse, McpError>,
) {
    let failed = match out {
        Ok(CallToolResponse::Complete(r)) => r.is_error == Some(true),
        Ok(_) => false,
        Err(_) => true,
    };
    let outcome = match (refused, failed) {
        (true, _) => "refused",
        (false, true) => "error",
        (false, false) => "ok",
    };
    let error: Option<String> = match out {
        Err(e) => Some(e.message.chars().take(ERROR_CAP).collect()),
        Ok(_) => None,
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    tracing::info!(
        target: "lab.call",
        tool,
        instance,
        lease_owner,
        outcome,
        duration_ms,
        error = error.as_deref(),
        "lab tool call"
    );
}

/// Compact a tool's text results and its error data for the MCP client
/// ([`compact`]). Images and non-JSON text pass through.
fn shape_response(
    shape: &compact::Shape,
    out: Result<CallToolResponse, McpError>,
) -> Result<CallToolResponse, McpError> {
    match out {
        Ok(CallToolResponse::Complete(mut r)) => {
            for block in &mut r.content {
                match block {
                    ContentBlock::Text(t) => {
                        if let Some(s) = shape.apply_text(&t.text) {
                            t.text = s;
                        }
                    }
                    ContentBlock::Image(img) if !shape.image && !shape.verbose => {
                        let note = match compact::save_image_in(
                            &compact::image_dir(),
                            &shape.tool,
                            &img.data,
                        ) {
                            Ok(p) => format!(
                                "image saved: {} (image: true returns it inline)",
                                p.display()
                            ),
                            Err(e) => format!("image not saved ({e}); image: true returns it"),
                        };
                        *block = ContentBlock::text(note);
                    }
                    _ => {}
                }
            }
            if let Some(sc) = r.structured_content.take() {
                r.structured_content = Some(shape.apply(sc));
            }
            Ok(CallToolResponse::Complete(r))
        }
        Ok(other) => Ok(other),
        Err(mut e) => {
            e.data = e.data.map(|d| shape.apply(d));
            Err(e)
        }
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for LabServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::from_build_env())
            .with_instructions(
                "Live Research Lab supervisor. Probe tools (client_lua_eval, \
                 client_module_info, client_mem_read) forward to the injected \
                 cimmeria-client-telemetry DLL over a token-gated loopback TCP \
                 channel. Supervisor tools (lab_client_start/stop/restart/status, \
                 lab_screenshot, lab_crash_report, and the lab_* flows) own the \
                 SGW.exe process lifecycle and crash recovery. lab_timeline merges \
                 local client events with server packet-tap rows (from \
                 cimmeria-lab-mcp over HTTP) into one clock-aligned window. \
                 One agent drives at a time: call lab_lease_acquire {owner, \
                 purpose} first and pass its lease_id to every tool that \
                 drives the client (they list it as a required argument); \
                 read-only tools need none. lab_lease_release when done. \
                 Fewest calls: lab_ensure_in_world gets in the world as a \
                 character from any state; client_batch runs many reads and \
                 probes in one call; client_ui_sequence runs one scripted UI \
                 step. Results are compact (no nulls or empties, floats to 2 \
                 decimals, lists capped at 50, no per-step native trail); any \
                 tool takes verbose: true for the full result and fields: [..] \
                 to keep only some top-level keys. Images (lab_screenshot) come back as a saved file path; image: true returns them inline."
                    .to_string(),
            )
    }

    // Written out instead of generated by `tool_handler` so every call
    // passes the lease gate and every guarded tool advertises `lease_id`.
    // The UAT runner dispatches through `tool_router` directly (in-process),
    // under the run's own lease.
    async fn call_tool(
        &self,
        mut request: CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let started = Instant::now();
        let tool = request.name.to_string();
        let routed = match self.route(&mut request) {
            Ok(routed) => routed,
            Err(e) => {
                let out = Err(e);
                log_call(self.supervisor.label(), &tool, None, started, false, &out);
                return out;
            }
        };
        let target = routed.as_ref().unwrap_or(self);
        // Read before the gate: a call that is turned away still shows who holds the lease.
        let holder = holder_owner(&target.supervisor.leases().status());
        let instance = target.supervisor.label();
        let span = tracing::info_span!("lab_call", instance);
        // Shaping arguments the tool declares itself stay with the tool.
        let own: Vec<String> = target
            .tool_router
            .get(request.name.as_ref())
            .and_then(|t| {
                t.input_schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .map(|p| {
                        compact::SHAPE_ARGS
                            .iter()
                            .filter(|a| p.contains_key(**a))
                            .map(|a| a.to_string())
                            .collect()
                    })
            })
            .unwrap_or_default();
        let shape = compact::Shape::take(request.name.as_ref(), request.arguments.as_mut(), &own);
        let permit = match target.gate_call(&mut request) {
            Ok(permit) => permit,
            Err(e) => {
                let out = Err(e);
                log_call(instance, &tool, holder.as_deref(), started, true, &out);
                return shape_response(&shape, out);
            }
        };
        let tcc = rmcp::handler::server::tool::ToolCallContext::new(target, request, context);
        let out = match permit {
            // The gate admits once; the permit makes every action of the
            // tool re-check the lease, so a takeover stops it mid-flow.
            Some(p) => {
                crate::lease::permit::scope(p, target.tool_router.call(tcc))
                    .instrument(span)
                    .await
            }
            None => target.tool_router.call(tcc).instrument(span).await,
        };
        log_call(instance, &tool, holder.as_deref(), started, false, &out);
        shape_response(&shape, out)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let cache_hints = context
            .protocol_version()
            .is_some_and(|v| v >= ProtocolVersion::V_2026_07_28);
        Ok(ListToolsResult {
            result_type: Some(ResultType::COMPLETE),
            tools: compact::slim(compact::advertise(
                self.advertise_hosted(lease::advertise_lease(self.tool_router.list_all())),
            )),
            meta: None,
            next_cursor: None,
            ttl_ms: cache_hints.then_some(0),
            cache_scope: cache_hints.then_some(CacheScope::Public),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use serde_json::json;
    use tracing::field::{Field, Visit};
    use tracing::Event;
    use tracing_subscriber::layer::{Context, SubscriberExt};
    use tracing_subscriber::Layer;

    use crate::daemon::http_tests::{spawn_daemon, test_server, McpSession};

    // A local capture rather than `cimmeria_test_support::LogCapture`: a
    // test-support dev-dependency puts a crate on the live-DB wrapper's list
    // (`live_db_wrapper_tests`), and the lab has no live-DB tests and is not
    // built by CI's workspace jobs.

    /// One captured event: its target and every field as text.
    #[derive(Clone, Debug)]
    struct Row {
        target: String,
        fields: Vec<(String, String)>,
    }

    impl Row {
        fn field(&self, name: &str) -> Option<&str> {
            self.fields
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.as_str())
        }
    }

    #[derive(Clone, Default)]
    struct Capture(Arc<Mutex<Vec<Row>>>);

    #[derive(Default)]
    struct Fields(Vec<(String, String)>);

    impl Visit for Fields {
        fn record_str(&mut self, field: &Field, value: &str) {
            self.0.push((field.name().to_string(), value.to_string()));
        }

        fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
            self.0
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    impl<S: tracing::Subscriber> Layer<S> for Capture {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            let mut fields = Fields::default();
            event.record(&mut fields);
            self.0.lock().unwrap().push(Row {
                target: event.metadata().target().to_string(),
                fields: fields.0,
            });
        }
    }

    /// A lease-gated tool called with no lease is refused, and its one
    /// `lab.call` row says so, names the tool and carries no arguments.
    #[tokio::test]
    async fn unleased_tool_call_logs_refused() {
        // An always-interested global dispatcher, so no callsite is cached
        // as `never` before the scoped capture below exists (#891).
        let _ = tracing::subscriber::set_global_default(tracing_subscriber::registry());
        let cap = Capture::default();
        // Thread-local; the current-thread test runtime runs the daemon's
        // spawned tasks on this thread, so their events land here too.
        let _guard =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(cap.clone()));
        let url = spawn_daemon(test_server()).await;
        let mut s = McpSession::open(&url).await;

        let r = s
            .call("client_lua_eval", json!({ "chunk": "return 1" }))
            .await;
        assert!(r["error"].is_object(), "{r}");

        let rows: Vec<Row> = cap
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.target == "lab.call")
            .cloned()
            .collect();
        assert_eq!(rows.len(), 1, "one row per call: {rows:?}");
        let row = &rows[0];
        assert_eq!(row.field("outcome"), Some("refused"), "{row:?}");
        assert_eq!(row.field("tool"), Some("client_lua_eval"), "{row:?}");
        assert!(row.field("duration_ms").is_some(), "{row:?}");
        assert!(row.field("lease_owner").is_none(), "{row:?}");
        assert!(row.field("chunk").is_none(), "{row:?}");
        assert!(
            row.fields.iter().all(|(_, v)| !v.contains("return 1")),
            "{row:?}"
        );
    }
}
