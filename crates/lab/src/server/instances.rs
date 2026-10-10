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
    Ok(out)
}

impl Instances {
    /// Panics on an empty list: a programming error, `parse_list` refuses one.
    pub fn new(list: Vec<Hosted>) -> Self {
        assert!(!list.is_empty(), "an Instances needs at least one instance");
        Self { list }
    }

    /// The target when a call names no instance.
    pub fn first(&self) -> &Hosted {
        &self.list[0]
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Hosted> {
        self.list.iter()
    }

    /// The instance called `name` (case-insensitive).
    pub fn by_label(&self, name: &str) -> Option<&Hosted> {
        self.iter().find(|h| h.label.eq_ignore_ascii_case(name))
    }

    /// The instance whose lease book holds `lease_id` as its current lease.
    pub fn by_lease(&self, lease_id: &str) -> Option<&Hosted> {
        self.iter().find(|h| h.supervisor.leases().holds(lease_id))
    }

    pub fn labels(&self) -> Vec<String> {
        self.iter().map(|h| h.label.clone()).collect()
    }
}

impl LabServer {
    /// The server a call runs on: a clone whose supervisor is the target
    /// instance's. Takes the `instance` argument out of the call. Order: the
    /// `instance` argument; else the instance holding the call's `lease_id`
    /// (left in the arguments for the lease gate); else the first instance.
    pub(super) fn route(&self, request: &mut CallToolRequestParams) -> Result<LabServer, McpError> {
        let hosted = match take_instance_arg(request)? {
            Some(name) => self.instances.by_label(&name).ok_or_else(|| {
                McpError::invalid_params(
                    format!(
                        "no lab instance {name:?}: this daemon hosts {}",
                        self.instances.labels().join(", ")
                    ),
                    None,
                )
            })?,
            None => given_lease(request)
                .and_then(|id| self.instances.by_lease(&id))
                .unwrap_or_else(|| self.instances.first()),
        };
        Ok(LabServer {
            supervisor: hosted.supervisor.clone(),
            ..self.clone()
        })
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
        let config = SupervisorConfig {
            install_dir: None,
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
        Hosted {
            label: label.into(),
            supervisor: supervisor(instance),
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

    #[test]
    fn instance_arg_is_advertised_only_with_several_instances() {
        assert!(!instance_arg_wanted(1));
        assert!(instance_arg_wanted(2));
        assert!(instance_arg_wanted(5));
    }
}
