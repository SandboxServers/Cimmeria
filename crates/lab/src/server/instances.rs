//! The lab instances one `cimmeria-lab` process hosts, and which one a
//! tool call is for (#1312, LP-05a). Each instance is one lab account
//! and one `SGW.exe` with its own supervisor, watchdog and lease book.

use std::sync::Arc;

use rmcp::{
    model::{CallToolRequestParams, Tool},
    ErrorData as McpError,
};
use serde_json::{json, Value};

use super::lease::LEASE_ARG;
use super::LabServer;
use crate::supervisor::{instance, Supervisor};

pub use crate::supervisor::instance::INSTANCES_ENV;

/// The optional argument every tool takes when more than one instance
/// is hosted.
pub const INSTANCE_ARG: &str = "instance";

/// One hosted instance.
#[derive(Clone)]
pub struct Hosted {
    /// `default` or the instance name (`p2`).
    pub label: String,
    /// The instance's own supervisor: client, watchdog and lease book.
    pub supervisor: Arc<Supervisor>,
}

/// Every hosted instance; the first is the target when a call names none.
#[derive(Clone)]
pub struct Instances {
    list: Vec<Hosted>,
}

/// Parse [`INSTANCES_ENV`]: `None` entries are the default instance.
/// Refuses duplicates and bad names (`instance::validate_name`).
pub fn parse_list(raw: &str) -> Result<Vec<Option<String>>, String> {
    let mut labels: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for item in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let entry = if item.eq_ignore_ascii_case("default") {
            None
        } else {
            Some(instance::validate_name(item)?)
        };
        let label = entry.clone().unwrap_or_else(|| "default".to_string());
        if labels.iter().any(|l| l.eq_ignore_ascii_case(&label)) {
            return Err(format!("{INSTANCES_ENV} lists {label:?} more than once"));
        }
        labels.push(label);
        out.push(entry);
    }
    if out.is_empty() {
        return Err(format!("{INSTANCES_ENV} names no instance"));
    }
    if out.len() > instance::CEILING_MAX_CLIENTS {
        return Err(format!(
            "{INSTANCES_ENV} lists {} instances; at most {} lab clients can run",
            out.len(),
            instance::CEILING_MAX_CLIENTS
        ));
    }
    Ok(out)
}

impl Instances {
    /// The hosted instances, in order. Panics on an empty list: a
    /// programming error, `parse_list` refuses one.
    pub fn new(list: Vec<Hosted>) -> Self {
        assert!(!list.is_empty(), "an Instances needs at least one instance");
        Self { list }
    }

    /// The target when a call names no instance.
    pub fn first(&self) -> &Hosted {
        &self.list[0]
    }

    /// How many instances are hosted (at least one).
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// The instances in order.
    pub fn iter(&self) -> impl Iterator<Item = &Hosted> {
        self.list.iter()
    }

    /// The instance called `name` (case-insensitive).
    pub fn by_label(&self, name: &str) -> Option<&Hosted> {
        self.iter().find(|h| h.label.eq_ignore_ascii_case(name))
    }

    /// The instance called `name`: by label, else by its lab account name
    /// (both case-insensitive). Lets a caller say `lab2` for the `p2` client.
    pub fn by_name(&self, name: &str) -> Option<&Hosted> {
        self.by_label(name).or_else(|| {
            self.iter().find(|h| {
                h.supervisor
                    .account_name()
                    .is_some_and(|account| account.eq_ignore_ascii_case(name))
            })
        })
    }

    /// The instance whose lease book holds `lease_id` as its current lease.
    pub fn by_lease(&self, lease_id: &str) -> Option<&Hosted> {
        self.iter().find(|h| h.supervisor.leases().holds(lease_id))
    }

    /// Every instance's label, in order.
    pub fn labels(&self) -> Vec<String> {
        self.iter().map(|h| h.label.clone()).collect()
    }
}

impl LabServer {
    /// A server hosting every instance in `instances`. A call runs on the
    /// instance it names, else the one holding its lease, else the first.
    pub fn new_multi(instances: Instances) -> Self {
        let mut server = Self::new(instances.first().supervisor.clone());
        server.instances = Arc::new(instances);
        server
    }

    /// The server a call runs on: `None` for this one, else a clone whose
    /// supervisor is the target instance's. Takes the `instance` argument
    /// out of the call. Order: the `instance` argument; else the instance
    /// holding the call's `lease_id` (left in the arguments for the lease
    /// gate); else the first instance. `None` only when the call falls
    /// through to this server's own instance.
    pub(super) fn route(
        &self,
        request: &mut CallToolRequestParams,
    ) -> Result<Option<LabServer>, McpError> {
        let (hosted, explicit) = match take_instance_arg(request)? {
            Some(name) => {
                let hosted = self.instances.by_name(&name).ok_or_else(|| {
                    McpError::invalid_params(
                        format!(
                            "no lab instance or account {name:?}: this daemon hosts {} (or an account such as lab, lab2, ...)",
                            self.instances.labels().join(", ")
                        ),
                        None,
                    )
                })?;
                (hosted, true)
            }
            None => match given_lease(request).and_then(|id| self.instances.by_lease(&id)) {
                Some(hosted) => (hosted, true),
                None => (self.instances.first(), false),
            },
        };
        if !explicit && Arc::ptr_eq(&hosted.supervisor, &self.supervisor) {
            return Ok(None);
        }
        Ok(Some(LabServer {
            supervisor: hosted.supervisor.clone(),
            routed_explicitly: explicit,
            ..self.clone()
        }))
    }

    /// Whether this server's call chose its instance (see [`LabServer::route`]).
    pub(super) fn routed_explicitly(&self) -> bool {
        self.routed_explicitly
    }

    /// Advertise the `instance` argument on every tool, when more than one
    /// instance is hosted (see [`instance_arg_wanted`]).
    pub(super) fn advertise_hosted(&self, tools: Vec<Tool>) -> Vec<Tool> {
        if instance_arg_wanted(self.instances.len()) {
            advertise_instance(tools, &self.instances.labels())
        } else {
            tools
        }
    }
}

/// Remove the `instance` argument from a call and return its name.
fn take_instance_arg(request: &mut CallToolRequestParams) -> Result<Option<String>, McpError> {
    let Some(value) = request
        .arguments
        .as_mut()
        .and_then(|args| args.remove(INSTANCE_ARG))
    else {
        return Ok(None);
    };
    // A client that fills optional properties with null means "none".
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(|s| Some(s.to_string()))
        .ok_or_else(|| McpError::invalid_params(format!("{INSTANCE_ARG} must be a string"), None))
}

/// The string `lease_id` argument of a call, left in place.
fn given_lease(request: &CallToolRequestParams) -> Option<String> {
    request
        .arguments
        .as_ref()
        .and_then(|args| args.get(LEASE_ARG))
        .and_then(Value::as_str)
        .map(String::from)
}

/// Whether the `instance` argument is advertised: only when several
/// instances are hosted, so a single-instance daemon lists its tools as before.
pub(super) fn instance_arg_wanted(hosted: usize) -> bool {
    hosted > 1
}

/// Add the optional `instance` argument to every tool's schema. It is not
/// required: an omitted one routes by `lease_id`, else to the first label.
pub(super) fn advertise_instance(tools: Vec<Tool>, labels: &[String]) -> Vec<Tool> {
    let first = labels.first().map_or("default", String::as_str);
    let description = format!(
        "Which lab client: {} or its account (lab, lab2, ...). Default: the instance your lease_id belongs to, else {first}.",
        labels.join(", ")
    );
    tools
        .into_iter()
        .map(|mut t| {
            let mut schema = (*t.input_schema).clone();
            schema
                .entry("type")
                .or_insert_with(|| Value::String("object".into()));
            let props = schema.entry("properties").or_insert_with(|| json!({}));
            if let Some(p) = props.as_object_mut() {
                p.insert(
                    INSTANCE_ARG.into(),
                    json!({ "type": "string", "description": description }),
                );
            }
            t.input_schema = Arc::new(schema);
            t
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::BridgeClient;
    use crate::supervisor::SupervisorConfig;

    fn supervisor(instance: Option<&str>) -> Arc<Supervisor> {
        supervisor_in(instance, None)
    }

    fn supervisor_in(
        instance: Option<&str>,
        install_dir: Option<std::path::PathBuf>,
    ) -> Arc<Supervisor> {
        let config = SupervisorConfig {
            install_dir,
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

    fn hosted(label: &str, instance: Option<&str>) -> Hosted {
        hosted_in(label, instance, None)
    }

    fn hosted_in(
        label: &str,
        instance: Option<&str>,
        install_dir: Option<std::path::PathBuf>,
    ) -> Hosted {
        Hosted {
            label: label.into(),
            supervisor: supervisor_in(instance, install_dir),
        }
    }

    #[test]
    fn parse_list_reads_default_and_names() {
        assert_eq!(
            parse_list("default,p2,P3").unwrap(),
            vec![None, Some("p2".into()), Some("P3".into())]
        );
        assert_eq!(
            parse_list(" DEFAULT , ,p2 ").unwrap(),
            vec![None, Some("p2".into())]
        );
    }

    #[test]
    fn parse_list_refuses_duplicates_bad_names_and_empty() {
        assert!(parse_list("p2,P2").is_err());
        assert!(parse_list("default,Default").is_err());
        assert!(parse_list("p2,bad name").is_err());
        assert!(parse_list("p2,../x").is_err());
        assert!(parse_list("").is_err());
        assert!(parse_list(" , ").is_err());
        assert!(parse_list("default,p2,p3,p4,p5").is_ok());
        assert!(
            parse_list("default,p2,p3,p4,p5,p6").is_err(),
            "the cap is five"
        );
    }

    #[test]
    fn by_label_ignores_case() {
        let list = Instances::new(vec![hosted("default", None), hosted("p2", Some("p2"))]);
        assert_eq!(list.by_label("P2").map(|h| h.label.as_str()), Some("p2"));
        assert_eq!(
            list.by_label("DEFAULT").map(|h| h.label.as_str()),
            Some("default")
        );
        assert!(list.by_label("p9").is_none());
        assert_eq!(list.labels(), vec!["default", "p2"]);
        assert_eq!(list.first().label, "default");
    }

    #[test]
    fn advertise_instance_adds_the_optional_property() {
        let s = LabServer::new(supervisor(None));
        let before = s.tool_router.list_all();
        let labels = vec!["default".to_string(), "p2".to_string()];
        let after = advertise_instance(s.tool_router.list_all(), &labels);
        assert_eq!(before.len(), after.len());
        for (b, a) in before.iter().zip(&after) {
            let prop = &a.input_schema["properties"][INSTANCE_ARG];
            assert_eq!(prop["type"], "string");
            assert!(prop["description"]
                .as_str()
                .unwrap()
                .contains("default, p2"));
            assert_eq!(
                b.input_schema.get("required"),
                a.input_schema.get("required")
            );
        }
    }

    fn two() -> LabServer {
        LabServer::new_multi(Instances::new(vec![
            hosted("default", None),
            hosted("p2", Some("p2")),
        ]))
    }

    fn call(name: &str, args: Value) -> CallToolRequestParams {
        CallToolRequestParams::new(name.to_string())
            .with_arguments(args.as_object().unwrap().clone())
    }

    fn lease_on(s: &LabServer, label: &str) -> String {
        let req = crate::lease::AcquireRequest {
            owner: format!("agent-{label}"),
            purpose: "routing test".into(),
            ..Default::default()
        };
        s.instances
            .by_label(label)
            .unwrap()
            .supervisor
            .leases()
            .acquire(req)
            .unwrap()
            .lease_id
    }

    fn routed_label(s: &LabServer, routed: &Option<LabServer>) -> String {
        let sup = &routed.as_ref().unwrap_or(s).supervisor;
        s.instances
            .iter()
            .find(|h| Arc::ptr_eq(&h.supervisor, sup))
            .unwrap()
            .label
            .clone()
    }

    /// Regression guard (LP-05a): the `instance` argument picks the
    /// instance and never reaches the tool's own arguments.
    #[test]
    fn route_by_instance_strips_the_argument() {
        let s = two();
        for name in ["p2", "P2"] {
            let mut req = call("client_ui_state", json!({ "instance": name, "x": 1 }));
            let routed = s.route(&mut req).unwrap();
            assert_eq!(routed_label(&s, &routed), "p2");
            let args = req.arguments.unwrap();
            assert!(!args.contains_key(INSTANCE_ARG));
            assert_eq!(args["x"], 1);
        }
        let mut req = call("client_ui_state", json!({ "instance": null }));
        assert_eq!(routed_label(&s, &s.route(&mut req).unwrap()), "default");
    }

    #[test]
    fn route_refuses_an_unknown_or_non_string_instance() {
        let s = two();
        let Err(err) = s.route(&mut call("client_ui_state", json!({ "instance": "p9" }))) else {
            panic!("an unknown instance must be refused");
        };
        assert!(err.message.contains("default, p2"), "{}", err.message);
        assert!(s
            .route(&mut call("client_ui_state", json!({ "instance": 2 })))
            .is_err());
    }

    /// Regression guard (LP-05a): with no `instance`, a lease id routes to
    /// the instance that issued it, else the first instance.
    #[test]
    fn route_by_lease_lands_on_its_own_instance() {
        let s = two();
        let id = lease_on(&s, "p2");
        let mut req = call("client_lua_eval", json!({ "lease_id": id }));
        let routed = s.route(&mut req).unwrap();
        assert_eq!(routed_label(&s, &routed), "p2");
        assert!(routed.as_ref().unwrap_or(&s).gate_call(&mut req).is_ok());
        let mut req = call("client_lua_eval", json!({ "lease_id": "unknown" }));
        assert_eq!(routed_label(&s, &s.route(&mut req).unwrap()), "default");
    }

    /// Regression guard (LP-05a): a lease from one instance does not
    /// admit a call that names another.
    #[test]
    fn a_lease_from_one_instance_is_refused_on_another() {
        let s = two();
        let id = lease_on(&s, "default");
        let mut req = call(
            "client_lua_eval",
            json!({ "instance": "p2", "lease_id": id }),
        );
        let routed = s.route(&mut req).unwrap();
        assert_eq!(routed_label(&s, &routed), "p2");
        assert!(routed.as_ref().unwrap_or(&s).gate_call(&mut req).is_err());
    }

    #[test]
    fn a_single_instance_server_lists_no_instance_argument() {
        let s = LabServer::new(supervisor(None));
        for t in s.advertise_hosted(s.tool_router.list_all()) {
            assert!(
                t.input_schema
                    .get("properties")
                    .and_then(|p| p.get(INSTANCE_ARG))
                    .is_none(),
                "{}",
                t.name
            );
        }
        for t in two().advertise_hosted(s.tool_router.list_all()) {
            assert!(
                t.input_schema
                    .get("properties")
                    .and_then(|p| p.get(INSTANCE_ARG))
                    .is_some(),
                "{}",
                t.name
            );
        }
    }

    #[test]
    fn instance_arg_is_advertised_only_with_several_instances() {
        assert!(!instance_arg_wanted(1));
        assert!(instance_arg_wanted(2));
        assert!(instance_arg_wanted(5));
    }

    /// The `instance` argument also takes a lab account name (LP-05b).
    #[test]
    fn route_by_account_name() {
        let dir = tempfile::tempdir().unwrap();
        let default_account = instance::account_path(dir.path(), None);
        let p2_account = instance::account_path(dir.path(), Some("p2"));
        for path in [&default_account, &p2_account] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        }
        std::fs::write(&default_account, r#"{"username":"lab","password":"x"}"#).unwrap();
        std::fs::write(&p2_account, r#"{"username":"lab2","password":"x"}"#).unwrap();
        let s = LabServer::new_multi(Instances::new(vec![
            hosted_in("default", None, Some(dir.path().into())),
            hosted_in("p2", Some("p2"), Some(dir.path().into())),
        ]));
        for name in ["LAB2", "lab2"] {
            let mut req = call("client_ui_state", json!({ "instance": name }));
            assert_eq!(routed_label(&s, &s.route(&mut req).unwrap()), "p2");
        }
        let mut req = call("client_ui_state", json!({ "instance": "lab" }));
        assert_eq!(routed_label(&s, &s.route(&mut req).unwrap()), "default");
    }

    #[test]
    fn explicit_routing_is_recorded() {
        let s = two();
        let mut req = call("client_ui_state", json!({ "instance": "default" }));
        let routed = s.route(&mut req).unwrap();
        assert!(routed.as_ref().is_some_and(|r| r.routed_explicitly()));
        assert!(!s.routed_explicitly());
        let mut req = call("client_ui_state", json!({}));
        assert!(s.route(&mut req).unwrap().is_none());
    }

    #[test]
    fn instances_status_lists_every_instance_without_lease_ids() {
        let s = two();
        let id = lease_on(&s, "default");
        let rows = vec![
            (
                "default".to_string(),
                Some("lab".to_string()),
                s.instances.first().supervisor.leases().status(),
                json!(100),
            ),
            ("p2".to_string(), None, json!({}), Value::Null),
        ];
        let out = crate::server::lease::instances_status(&rows);
        let list = out["instances"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0]["instance"], "default");
        assert_eq!(list[0]["account"], "lab");
        assert_eq!(list[0]["client_pid"], 100);
        assert!(list[1]["account"].is_null());
        assert!(list[1]["client_pid"].is_null());
        let text = out.to_string();
        assert!(!text.contains("lease_id"), "{text}");
        assert!(!text.contains(&id), "{text}");
    }

    /// A refusal on a routed call names the instance that refused it.
    #[test]
    fn a_refusal_names_its_instance() {
        let s = two();
        let id = lease_on(&s, "default");
        let mut req = call(
            "client_lua_eval",
            json!({ "instance": "p2", "lease_id": id }),
        );
        let routed = s.route(&mut req).unwrap();
        let Err(err) = routed.as_ref().unwrap_or(&s).gate_call(&mut req) else {
            panic!("a lease from another instance must be refused");
        };
        assert!(err.message.contains("(instance p2)"), "{}", err.message);
    }
}
