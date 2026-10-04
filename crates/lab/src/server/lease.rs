//! The lease tools (`lab_lease_*`) and the gate every other tool call goes
//! through ([`LabServer::gate_call`], called from `ServerHandler::call_tool`).
//! The rules are in [`crate::lease`]; which tools are guarded in
//! [`crate::lease::policy`].

use std::sync::Arc;

use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_router,
    ErrorData as McpError,
};
use serde_json::{json, Value};

use super::LabServer;
use crate::lease::permit::Permit;
use crate::lease::policy::{self, Gate};
use crate::lease::{grant_json, AcquireRequest, DEFAULT_TTL_S, MAX_TTL_S};

/// The argument every guarded tool takes.
pub const LEASE_ARG: &str = "lease_id";

/// Args for `lab_lease_acquire`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LeaseAcquireArgs {
    /// Who is driving: your session or agent name, as other sessions will
    /// see it in a refusal.
    pub owner: String,
    /// What for, in a few words (`ability UAT row AB-3`).
    pub purpose: String,
    /// Lease length in seconds (default 600, 30..=3600). Every guarded tool
    /// call renews it.
    #[serde(default)]
    pub ttl_s: Option<u64>,
    /// Take the lease over from its holder (logged at WARN; needs `reason`).
    #[serde(default)]
    pub force: bool,
    /// Why you are taking it over (required with `force`).
    #[serde(default)]
    pub reason: Option<String>,
}

/// Args for `lab_lease_renew`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LeaseRenewArgs {
    pub lease_id: String,
    /// New length in seconds from now (default: the lease's own).
    #[serde(default)]
    pub ttl_s: Option<u64>,
}

/// Args for `lab_lease_release`.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct LeaseReleaseArgs {
    pub lease_id: String,
}

fn text(v: &Value) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(
        serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()),
    )])
}

#[tool_router(router = lease_router, vis = "pub(super)")]
impl LabServer {
    #[tool(
        description = "Take the lab lease before driving the client: every tool that changes the client, drives its input or UI, runs caller-chosen Lua or native code, or reads a shared event cursor needs the returned lease_id, and each such call renews the lease. One holder at a time across every session; a refusal names the holder, their purpose and since when. force: true with a reason takes it over (logged). Default ttl 600 s, max 3600. While nobody holds a lease the watchdog does not relaunch a dead client."
    )]
    async fn lab_lease_acquire(
        &self,
        Parameters(a): Parameters<LeaseAcquireArgs>,
    ) -> Result<CallToolResult, McpError> {
        let req = AcquireRequest {
            owner: a.owner,
            purpose: a.purpose,
            ttl_s: a.ttl_s,
            force: a.force,
            reason: a.reason,
        };
        self.supervisor
            .leases()
            .acquire(req)
            .map(|l| text(&grant_json(&l)))
            .map_err(|e| McpError::invalid_request(e, Some(self.supervisor.leases().status())))
    }

    #[tool(
        description = "Extend your lab lease by its ttl (or ttl_s) from now. Any guarded tool call renews it too, so this is for long pauses between calls."
    )]
    async fn lab_lease_renew(
        &self,
        Parameters(a): Parameters<LeaseRenewArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.supervisor
            .leases()
            .renew(&a.lease_id, a.ttl_s)
            .map(|l| text(&grant_json(&l)))
            .map_err(|e| McpError::invalid_request(e, None))
    }

    #[tool(description = "Release your lab lease when you are done driving the client.")]
    async fn lab_lease_release(
        &self,
        Parameters(a): Parameters<LeaseReleaseArgs>,
    ) -> Result<CallToolResult, McpError> {
        self.supervisor
            .leases()
            .release(&a.lease_id)
            .map(|e| text(&json!({ "released": true, "owner": e.owner, "purpose": e.purpose })))
            .map_err(|e| McpError::invalid_request(e, None))
    }

    #[tool(
        description = "Who holds the lab lease (owner, purpose, since, expires_at), the last few leases and how they ended (released, expired, taken over). Never shows a lease id."
    )]
    async fn lab_lease_status(&self) -> Result<CallToolResult, McpError> {
        Ok(text(&self.supervisor.leases().status()))
    }
}

impl LabServer {
    /// The lease gate for one `tools/call`. Open tools pass untouched. A
    /// guarded tool must carry the current `lease_id`, which is then taken
    /// out of the arguments (the tools' own argument types do not know it)
    /// and the lease renewed. Returns the [`Permit`] the tool then runs
    /// under, so every action it takes re-checks that lease.
    pub(super) fn gate_call(
        &self,
        request: &mut CallToolRequestParams,
    ) -> Result<Option<Permit>, McpError> {
        let tool = request.name.as_ref();
        if policy::gate(tool) == Gate::Open {
            return Ok(None);
        }
        let id = request
            .arguments
            .as_mut()
            .and_then(|args| args.remove(LEASE_ARG))
            .and_then(|v| v.as_str().map(String::from));
        let book = self.supervisor.leases();
        book.check(id.as_deref(), tool)
            .map_err(|e| McpError::invalid_params(e, Some(book.status())))?;
        Ok(id.map(|id| Permit::Lease {
            book: book.clone(),
            id,
        }))
    }
}

/// Add the required `lease_id` argument to every guarded tool's schema, so
/// a client sees it in `tools/list` like any other argument.
pub(super) fn advertise_lease(tools: Vec<Tool>) -> Vec<Tool> {
    tools
        .into_iter()
        .map(|mut t| {
            if policy::gate(t.name.as_ref()) == Gate::Open {
                return t;
            }
            let mut schema = (*t.input_schema).clone();
            schema
                .entry("type")
                .or_insert_with(|| Value::String("object".into()));
            let props = schema.entry("properties").or_insert_with(|| json!({}));
            if let Some(p) = props.as_object_mut() {
                p.insert(
                    LEASE_ARG.into(),
                    json!({
                        "type": "string",
                        "description": format!(
                            "Your lab lease (lab_lease_acquire). This call renews it \
                             (default ttl {DEFAULT_TTL_S} s, max {MAX_TTL_S})."
                        ),
                    }),
                );
            }
            let required = schema.entry("required").or_insert_with(|| json!([]));
            if let Some(r) = required.as_array_mut() {
                if !r.iter().any(|v| v == LEASE_ARG) {
                    r.push(Value::String(LEASE_ARG.into()));
                }
            }
            t.input_schema = Arc::new(schema);
            t
        })
        .collect()
}

#[cfg(test)]
#[path = "lease_tests.rs"]
mod tests;
