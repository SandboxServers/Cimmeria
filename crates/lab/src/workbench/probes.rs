//! Bounded Lua probe execution and safe global-object browsing.

use std::future::Future;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Map, Value};

use super::probe_catalog::{ProbeDefinition, ProbeMode};

/// Maximum bytes accepted from the bridge for one probe result.
pub const OUTPUT_CAP: usize = 32 * 1024;
const MAX_VALUE_DEPTH: usize = 5;
const MAX_VALUE_NODES: usize = 128;
const RUN_TIMEOUT: Duration = Duration::from_secs(3);

/// A probe result can be a Lua error, transport timeout, truncated tree or value.
#[derive(Debug, Serialize)]
pub struct ProbeOutcome {
    pub status: &'static str,
    pub value: Option<Value>,
    pub detail: Option<String>,
    pub gaps: Vec<&'static str>,
}

fn lua_string(value: &str) -> String {
    let mut out = String::from("\"");
    for byte in value.bytes() {
        out.push_str(&format!("\\{byte:03}"));
    }
    out.push('"');
    out
}

fn lua_value(value: &Value, depth: usize) -> Result<String, String> {
    if depth > 4 {
        return Err("parameter nesting exceeds 4".into());
    }
    match value {
        Value::Null => Ok("nil".into()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) if n.as_f64().is_some_and(f64::is_finite) => Ok(n.to_string()),
        Value::String(s) if s.len() <= 1_024 => Ok(lua_string(s)),
        Value::Array(values) if values.len() <= 32 => Ok(format!(
            "{{{}}}",
            values
                .iter()
                .map(|v| lua_value(v, depth + 1))
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        )),
        Value::Object(values) if values.len() <= 32 => Ok(format!(
            "{{{}}}",
            values
                .iter()
                .map(|(k, v)| {
                    if k.len() > 64 {
                        return Err("parameter key exceeds 64 bytes".into());
                    }
                    Ok(format!("[{}]={}", lua_string(k), lua_value(v, depth + 1)?))
                })
                .collect::<Result<Vec<_>, String>>()?
                .join(",")
        )),
        _ => Err("parameter value exceeds bounds".into()),
    }
}

fn params_literal(params: &Map<String, Value>) -> Result<String, String> {
    if serde_json::to_vec(params).map_err(|e| e.to_string())?.len() > 4_096 {
        return Err("parameters exceed 4096 bytes".into());
    }
    lua_value(&Value::Object(params.clone()), 0)
}

// The result is built only from primitives and raw table access. In particular,
// it never invokes tostring on a userdata or indexes it via __index.
const SAFE_TREE: &str = r#"
local __seen, __nodes, __partial = {}, 0, false
local function __tree(v, depth)
  local kind = type(v)
  __nodes = __nodes + 1
  if __nodes > 128 then __partial = true; return {kind='truncated', reason='node_limit'} end
  if kind == 'nil' then return {kind='nil'} end
  if kind == 'boolean' or kind == 'number' then return {kind=kind, value=v} end
  if kind == 'string' then
    if string.len(v) > 1024 then __partial = true; v=string.sub(v,1,1024) end
    return {kind='string', value=v}
  end
  if kind ~= 'table' then return {kind=kind, value='<opaque>'} end
  if __seen[v] then __partial = true; return {kind='cycle'} end
  if depth >= 4 then __partial = true; return {kind='truncated', reason='depth_limit'} end
  __seen[v] = true
  local items, key, scanned = {}, nil, 0
  while #items < 64 and __nodes < 128 and scanned < 256 do
    key = next(v, key)
    if key == nil then break end
    scanned = scanned + 1
    local kt = type(key)
  if kt == 'string' or (kt == 'number' and key % 1 == 0) then
      items[#items+1] = {key=__tree(key, depth+1), value=__tree(rawget(v,key), depth+1)}
  else
    __partial = true
    end
  end
  if key ~= nil then __partial = true end
  return {kind='table', items=items}
end
"#;

/// Construct an isolated, instruction-budgeted chunk. A C binding that blocks
/// inside a call cannot be interrupted by Lua's debug hook.
pub fn run_chunk(def: &ProbeDefinition, params: &Map<String, Value>) -> Result<String, String> {
    let args = params_literal(params)?;
    let source = lua_string(&def.chunk);
    Ok(format!("{}\n{}\nlocal __params = {}\nlocal __fn, __syntax = loadstring({})\nif not __fn then return __jenc({{status='lua_error', detail=__syntax}}) end\nif not debug or not debug.sethook or not debug.gethook then return __jenc({{status='hook_unavailable'}}) end\nlocal __old, __mask, __count = debug.gethook()\nlocal __ticks = 0\nlocal function __budget() __ticks=__ticks+1; if __ticks > 100 then debug.sethook(); error('probe_instruction_budget') end end\ndebug.sethook(__budget, '', 1000)\nlocal __ok, __value = pcall(__fn, __params)\ndebug.sethook(__old, __mask, __count)\nif not __ok then return __jenc({{status='lua_error', detail=string.sub(tostring(__value),1,1024)}}) end\nlocal __result = __tree(__value, 0)\nreturn __jenc({{status='ok', value=__result, partial=__partial}})\n", crate::supervisor::ui::lua_json::PRELUDE, SAFE_TREE, args, source))
}

/// Enumerate a named global by raw table access. The selected global's name
/// is data and is never interpolated as Lua source.
pub fn globals_chunk(name: &str) -> Result<String, String> {
    if name.len() > 128 || name.as_bytes().contains(&0) {
        return Err("global name invalid".into());
    }
    Ok(format!("{}\n{}\nlocal __value = rawget(_G, {})\nlocal __result = __tree(__value, 0)\nreturn __jenc({{status='ok', value=__result, partial=__partial}})\n", crate::supervisor::ui::lua_json::PRELUDE, SAFE_TREE, lua_string(name)))
}

fn bounded(value: &Value, depth: usize, nodes: &mut usize, partial: &mut bool) -> Value {
    *nodes += 1;
    if depth > MAX_VALUE_DEPTH || *nodes > MAX_VALUE_NODES {
        *partial = true;
        return json!({"kind":"truncated"});
    }
    match value {
        Value::String(s) if s.len() > 1_024 => {
            *partial = true;
            Value::String(s.chars().take(1_024).collect())
        }
        Value::Array(items) => {
            if items.len() > 64 {
                *partial = true;
            }
            Value::Array(
                items
                    .iter()
                    .take(64)
                    .map(|v| bounded(v, depth + 1, nodes, partial))
                    .collect(),
            )
        }
        Value::Object(items) => {
            if items.len() > 64 {
                *partial = true;
            }
            Value::Object(
                items
                    .iter()
                    .take(64)
                    .map(|(k, v)| (k.clone(), bounded(v, depth + 1, nodes, partial)))
                    .collect(),
            )
        }
        _ => value.clone(),
    }
}

/// Interpret the bridge's pcall result without assuming return capture exists.
pub fn decode_bridge(response: Value) -> ProbeOutcome {
    let bytes = serde_json::to_vec(&response).unwrap_or_default();
    if bytes.len() > OUTPUT_CAP {
        return ProbeOutcome {
            status: "truncated",
            value: None,
            detail: Some("bridge output exceeded 32768 bytes".into()),
            gaps: vec!["output_limit"],
        };
    }
    if response["ok"] != true {
        return ProbeOutcome {
            status: "lua_error",
            value: None,
            detail: response["error"]
                .as_str()
                .map(|s| s.chars().take(1024).collect()),
            gaps: vec![],
        };
    }
    let Some(text) = response["results"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_str)
    else {
        return ProbeOutcome {
            status: "unavailable",
            value: None,
            detail: Some("bridge did not capture a Lua return value".into()),
            gaps: vec!["return_capture_unavailable"],
        };
    };
    let Ok(decoded) = serde_json::from_str::<Value>(text) else {
        return ProbeOutcome {
            status: "lua_error",
            value: None,
            detail: Some("probe returned invalid JSON".into()),
            gaps: vec!["invalid_return"],
        };
    };
    let status = decoded["status"].as_str().unwrap_or("lua_error");
    if status != "ok" {
        return ProbeOutcome {
            status: "lua_error",
            value: None,
            detail: decoded["detail"]
                .as_str()
                .map(|s| s.chars().take(1024).collect())
                .or_else(|| Some(status.into())),
            gaps: vec![],
        };
    }
    let mut partial = decoded["partial"] == true;
    let mut nodes = 0;
    let value = bounded(&decoded["value"], 0, &mut nodes, &mut partial);
    ProbeOutcome {
        status: if partial { "partial" } else { "ok" },
        value: Some(value),
        detail: None,
        gaps: if partial {
            vec!["tree_truncated"]
        } else {
            vec![]
        },
    }
}

/// Execute under an already checked lease; the timeout is a host wait cap.
/// A blocked native C call may remain in flight after this returns.
pub async fn execute<F, Fut>(
    def: &ProbeDefinition,
    params: &Map<String, Value>,
    eval: F,
) -> Result<ProbeOutcome, String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<Value, String>>,
{
    let chunk = run_chunk(def, params)?;
    execute_raw(chunk, eval).await
}

/// Execute a generated, bounded reader chunk through a fake or real bridge.
pub async fn execute_raw<F, Fut>(chunk: String, eval: F) -> Result<ProbeOutcome, String>
where
    F: FnOnce(String) -> Fut,
    Fut: Future<Output = Result<Value, String>>,
{
    match tokio::time::timeout(RUN_TIMEOUT, eval(chunk)).await {
        Ok(Ok(response)) => Ok(decode_bridge(response)),
        Ok(Err(e)) => Ok(ProbeOutcome {
            status: "transport_error",
            value: None,
            detail: Some(e.chars().take(1024).collect()),
            gaps: vec!["bridge_error"],
        }),
        Err(_) => Ok(ProbeOutcome {
            status: "timeout",
            value: None,
            detail: Some(
                "host wait exceeded 3 seconds; client execution may still be in flight".into(),
            ),
            gaps: vec!["execution_unknown"],
        }),
    }
}

/// Label a saved snippet for UI display and mutation confirmation.
pub fn is_mutation(def: &ProbeDefinition) -> bool {
    def.mode == ProbeMode::Mutate
}
