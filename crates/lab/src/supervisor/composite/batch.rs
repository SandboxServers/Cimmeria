//! `client_batch`: an ordered list of read-only or probe steps run in one
//! tool call, so a probe that used to take one agent turn per read takes
//! one turn in all.
//!
//! Steps (`op`): `lua` (`chunk`), `mem_read` (`addr`, `len`, `as` =
//! `hex|u8|u16|u32|i32|f32|f64`; default `u32` for a 4-byte read, else `hex`), `call_native` (`addr`, `conv`, `args`,
//! `ret`), `wait` (`frames` or `ms`), `player_state` (`fields`) and
//! `window_text` (`window`, `children`). Each step may carry an `id`; a step
//! without one is named by its 1-based position.
//!
//! **References.** A string argument that is exactly `$id` (or `$id.key`,
//! or `$id+0x270` / `$id-4`) is replaced by that earlier step's value, with
//! the offset added when there is one: read a pointer, then read
//! `$ptr+0x270`. `${id}` inside a longer string (a Lua chunk) is replaced by
//! the value's text.
//!
//! **Floats.** A `call_native` argument written as a JSON float (`1.5`,
//! `2.0`) is passed as its IEEE-754 single-precision bits; `{"f32": x}` says
//! so explicitly, and `{"f64": x}` passes a double as two words (low, high).
//!
//! `call_native` goes through [`Supervisor::bridge_call`] like
//! `client_call_native`, so it is journaled and runs under the bridge's
//! main-thread exception guard.

use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::supervisor::flows::widgets::lua_quote;
use crate::supervisor::Supervisor;

/// Most steps one batch may hold.
pub const MAX_STEPS: usize = 64;
/// Longest single `wait`.
pub const MAX_WAIT: Duration = Duration::from_secs(30);

/// How a `mem_read` decodes its bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadAs {
    Hex,
    U8,
    U16,
    U32,
    I32,
    F32,
    F64,
}

impl ReadAs {
    fn parse(s: &str) -> Result<Self, String> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "hex" => Self::Hex,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" | "ptr" => Self::U32,
            "i32" => Self::I32,
            "f32" => Self::F32,
            "f64" => Self::F64,
            other => {
                return Err(format!(
                    "`as` must be hex, u8, u16, u32, i32, f32 or f64, not {other:?}"
                ))
            }
        })
    }

    /// Bytes per value (hex reads default to 4).
    fn width(self) -> u32 {
        match self {
            Self::U8 => 1,
            Self::U16 => 2,
            Self::Hex | Self::U32 | Self::I32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

/// One parsed step.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Lua {
        chunk: String,
    },
    MemRead {
        addr: Value,
        len: Option<u32>,
        read_as: ReadAs,
    },
    CallNative {
        addr: Value,
        conv: Option<String>,
        args: Vec<Value>,
        ret: Option<String>,
    },
    Wait {
        frames: Option<u64>,
        ms: Option<u64>,
    },
    PlayerState {
        fields: Vec<String>,
    },
    WindowText {
        window: String,
        children: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub id: String,
    pub op: Op,
}

fn str_field(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_string)
}

fn need_str(v: &Value, k: &str, op: &str) -> Result<String, String> {
    str_field(v, k).ok_or_else(|| format!("{op} needs `{k}`"))
}

/// Parse the raw `steps` array.
pub fn parse_steps(raw: &[Value]) -> Result<Vec<Step>, String> {
    if raw.is_empty() {
        return Err("no steps".into());
    }
    if raw.len() > MAX_STEPS {
        return Err(format!("{} steps; at most {MAX_STEPS}", raw.len()));
    }
    let mut out = Vec::with_capacity(raw.len());
    for (i, s) in raw.iter().enumerate() {
        let id = str_field(s, "id").unwrap_or_else(|| (i + 1).to_string());
        let op = parse_op(s).map_err(|e| format!("step {id}: {e}"))?;
        if out.iter().any(|s: &Step| s.id == id) {
            return Err(format!("two steps are named {id:?}"));
        }
        out.push(Step { id, op });
    }
    Ok(out)
}

/// One step's op and its arguments.
fn parse_op(s: &Value) -> Result<Op, String> {
    let op_name = str_field(s, "op").ok_or("no `op`")?;
    Ok(match op_name.as_str() {
        "lua" => Op::Lua {
            chunk: need_str(s, "chunk", "lua")?,
        },
        "mem_read" => {
            let len = s.get("len").and_then(Value::as_u64).map(|n| n as u32);
            // A 4-byte read (the default) is a u32 unless `as` says
            // otherwise, so `$ptr+0x270` after it adds to a number, not
            // to a byte-order hex dump (review of #1309).
            let default_as = if len.is_none_or(|n| n == 4) {
                "u32"
            } else {
                "hex"
            };
            Op::MemRead {
                addr: s.get("addr").cloned().ok_or("mem_read needs `addr`")?,
                len,
                read_as: ReadAs::parse(&str_field(s, "as").unwrap_or_else(|| default_as.into()))?,
            }
        }
        "call_native" => Op::CallNative {
            addr: s.get("addr").cloned().ok_or("call_native needs `addr`")?,
            conv: str_field(s, "conv"),
            args: s
                .get("args")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default(),
            ret: str_field(s, "ret"),
        },
        "wait" => {
            let frames = s.get("frames").and_then(Value::as_u64);
            let ms = s.get("ms").and_then(Value::as_u64);
            if frames.is_none() && ms.is_none() {
                return Err("wait needs `frames` or `ms`".into());
            }
            Op::Wait { frames, ms }
        }
        "player_state" => Op::PlayerState {
            fields: s
                .get("fields")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|f| f.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        },
        "window_text" => Op::WindowText {
            window: need_str(s, "window", "window_text")?,
            children: s.get("children").and_then(Value::as_bool).unwrap_or(false),
        },
        other => {
            return Err(format!(
                "unknown op {other:?} (lua, mem_read, call_native, wait, player_state, \
                 window_text)"
            ))
        }
    })
}

/// A number from a JSON number or a decimal / `0x` hex string.
pub fn as_number(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => {
            let s = s.trim();
            match s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
                Some(h) => i64::from_str_radix(h, 16).ok(),
                None => s.parse().ok(),
            }
        }
        _ => None,
    }
}

/// Follow `id.key.key` into the values seen so far.
fn lookup(path: &str, values: &Map<String, Value>) -> Result<Value, String> {
    let mut parts = path.split('.');
    let id = parts.next().unwrap_or_default();
    let mut v = values
        .get(id)
        .ok_or_else(|| format!("${id} names no earlier step"))?;
    for p in parts {
        v = match v {
            Value::Array(a) => p.parse::<usize>().ok().and_then(|i| a.get(i)),
            other => other.get(p),
        }
        .ok_or_else(|| format!("${path}: no {p:?} in {v}"))?;
    }
    Ok(v.clone())
}

/// Resolve one `$...` reference (the whole string).
fn resolve_ref(s: &str, values: &Map<String, Value>) -> Result<Value, String> {
    let body = &s[1..];
    // Ids and paths are word characters and dots; an offset starts at the
    // first `+` or `-`.
    let split = body.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'));
    if let Some(i) = split {
        if !body[i..].starts_with(['+', '-']) {
            return Err(format!("{s}: step ids are letters, digits and _"));
        }
    }
    let (path, offset) = match split {
        Some(i) => (&body[..i], Some(&body[i..])),
        None => (body, None),
    };
    let v = lookup(path, values)?;
    let Some(off) = offset else { return Ok(v) };
    let (sign, digits) = off.split_at(1);
    let delta = as_number(&Value::String(digits.to_string()))
        .ok_or_else(|| format!("{s}: bad offset {digits:?}"))?;
    let base = as_number(&v).ok_or_else(|| format!("{s}: ${path} is not a number ({v})"))?;
    Ok(json!(if sign == "+" {
        base + delta
    } else {
        base - delta
    }))
}

/// The text a value interpolates as.
fn text_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Replace references in `v` (recursively through arrays and objects).
pub fn resolve(v: &Value, values: &Map<String, Value>) -> Result<Value, String> {
    match v {
        Value::String(s) if s.starts_with('$') && !s.starts_with("${") => resolve_ref(s, values),
        Value::String(s) if s.contains("${") => {
            let mut out = String::with_capacity(s.len());
            let mut rest = s.as_str();
            while let Some(i) = rest.find("${") {
                out.push_str(&rest[..i]);
                let tail = &rest[i + 2..];
                let end = tail
                    .find('}')
                    .ok_or_else(|| format!("unclosed ${{ in {s:?}"))?;
                out.push_str(&text_of(&lookup(&tail[..end], values)?));
                rest = &tail[end + 1..];
            }
            out.push_str(rest);
            Ok(Value::String(out))
        }
        Value::Array(a) => a
            .iter()
            .map(|x| resolve(x, values))
            .collect::<Result<_, _>>()
            .map(Value::Array),
        Value::Object(o) => o
            .iter()
            .map(|(k, x)| resolve(x, values).map(|r| (k.clone(), r)))
            .collect::<Result<Map<_, _>, _>>()
            .map(Value::Object),
        other => Ok(other.clone()),
    }
}

/// An address argument as the bridge takes it (a hex string).
pub fn addr_text(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        other => as_number(other)
            .map(|n| format!("{:#x}", n as u32))
            .ok_or_else(|| format!("address {other} is not a number")),
    }
}

/// `call_native` arguments as bridge words: floats become their f32 bits,
/// `{"f64": x}` becomes two words (low, high), negatives wrap to u32.
pub fn encode_args(args: &[Value]) -> Result<Vec<Value>, String> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        match a {
            Value::Number(n) if n.is_f64() => {
                let f = n.as_f64().unwrap_or_default() as f32;
                out.push(json!(format!("{:#010x}", f.to_bits())));
            }
            Value::Number(n) => {
                let i = n
                    .as_i64()
                    .ok_or_else(|| format!("argument {n} does not fit 32 bits"))?;
                out.push(json!(i as u32));
            }
            Value::Object(o) if o.contains_key("f32") => {
                let f = o["f32"].as_f64().ok_or("{\"f32\": x} needs a number")? as f32;
                out.push(json!(format!("{:#010x}", f.to_bits())));
            }
            Value::Object(o) if o.contains_key("f64") => {
                let bits = o["f64"]
                    .as_f64()
                    .ok_or("{\"f64\": x} needs a number")?
                    .to_bits();
                out.push(json!(format!("{:#010x}", bits as u32)));
                out.push(json!(format!("{:#010x}", (bits >> 32) as u32)));
            }
            Value::String(_) => out.push(a.clone()),
            other => {
                return Err(format!(
                    "argument {other} is not a number, hex string or {{f32|f64}}"
                ))
            }
        }
    }
    Ok(out)
}

/// Decode a `mem_read` hex string.
pub fn decode_read(hex: &str, read_as: ReadAs) -> Result<Value, String> {
    if read_as == ReadAs::Hex {
        return Ok(json!(hex));
    }
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16))
        .collect::<Result<_, _>>()
        .map_err(|e| format!("bad hex from mem_read: {e}"))?;
    let w = read_as.width() as usize;
    let vals: Vec<Value> = bytes
        .chunks_exact(w)
        .map(|c| match read_as {
            ReadAs::U8 => json!(c[0]),
            ReadAs::U16 => json!(u16::from_le_bytes([c[0], c[1]])),
            ReadAs::U32 => json!(u32::from_le_bytes([c[0], c[1], c[2], c[3]])),
            ReadAs::I32 => json!(i32::from_le_bytes([c[0], c[1], c[2], c[3]])),
            ReadAs::F32 => json!(f32::from_le_bytes([c[0], c[1], c[2], c[3]]) as f64),
            ReadAs::F64 => json!(f64::from_le_bytes(c.try_into().unwrap_or([0; 8]))),
            ReadAs::Hex => unreachable!(),
        })
        .collect();
    match vals.len() {
        0 => Err(format!(
            "read {} bytes, fewer than one {w}-byte value",
            bytes.len()
        )),
        1 => Ok(vals.into_iter().next().unwrap_or(Value::Null)),
        _ => Ok(Value::Array(vals)),
    }
}

/// The value a `call_native` step reports, by `ret`.
pub fn native_value(result: &Value, ret: Option<&str>) -> Value {
    match ret.unwrap_or("u32") {
        "void" => json!(true),
        "i32" => result["ret_i32"].clone(),
        "f32" => result["ret_f32"].clone(),
        "f64" => result["ret_f64"].clone(),
        "hex" => result["ret_hex"].clone(),
        _ => result["ret_u32"].clone(),
    }
}

/// Lua (after the JSON prelude) reading a window's text, and its visible
/// children's names and texts.
pub fn window_text_chunk(window: &str, children: bool) -> String {
    format!(
        r#"local w = _G[{w}]
if w == nil then return __jenc({{ missing = true }}) end
local out = {{ text = __jcall(function() return w:getText() end),
  visible = __jcall(function() return w:isVisible() end) }}
local r = __jcall(function() return w:getUnclippedPixelRect() end)
if r then out.rect = {{ r.left, r.top, r.right, r.bottom }} end
if {children} then
  local c = {{}}
  local n = __jcall(function() return w:getChildCount() end) or 0
  for i = 0, n - 1 do
    local x = __jcall(function() return w:getChildAtIdx(i) end)
    if x and __jcall(function() return x:isVisible() end) then
      c[#c + 1] = {{ name = __jcall(function() return x:getName() end),
        text = __jcall(function() return x:getText() end) }}
    end
  end
  out.children = c
end
return __jenc(out)"#,
        w = lua_quote(window)
    )
}

/// One batch's outcome.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchOutcome {
    /// Step id → value, or `{"error": ...}`.
    pub values: Map<String, Value>,
    /// The step the batch stopped at (stop_on_error).
    pub stopped_at: Option<String>,
    pub ms: u64,
}

impl BatchOutcome {
    pub fn to_json(&self) -> Value {
        let mut out = json!({ "steps": self.values, "ms": self.ms });
        if let Some(s) = &self.stopped_at {
            out["stopped_at"] = json!(s);
        }
        out
    }
}

impl Supervisor {
    /// Run one step with its references resolved.
    async fn batch_step(&self, op: &Op, values: &Map<String, Value>) -> Result<Value, String> {
        match op {
            Op::Lua { chunk } => {
                let chunk = text_of(&resolve(&json!(chunk), values)?);
                let r = self.lua_results(&chunk).await?;
                Ok(match r.len() {
                    0 => json!(true),
                    1 => json!(r[0]),
                    _ => json!(r),
                })
            }
            Op::MemRead { addr, len, read_as } => {
                let addr = addr_text(&resolve(addr, values)?)?;
                let len = len.unwrap_or(read_as.width()).clamp(1, 65536);
                let r = self
                    .bridge_call("mem_read", json!({ "addr": addr, "len": len }))
                    .await?;
                decode_read(r["hex"].as_str().unwrap_or_default(), *read_as)
            }
            Op::CallNative {
                addr,
                conv,
                args,
                ret,
            } => {
                let addr = addr_text(&resolve(addr, values)?)?;
                let args = encode_args(
                    &resolve(&Value::Array(args.clone()), values)?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                )?;
                let mut params = json!({ "addr": addr, "args": args });
                if let Some(c) = conv {
                    params["conv"] = json!(c);
                }
                if let Some(r) = ret.as_deref().filter(|r| *r != "hex") {
                    params["ret"] = json!(r);
                }
                let r = self.bridge_call("call_native", params).await?;
                Ok(native_value(&r, ret.as_deref()))
            }
            Op::Wait { frames, ms } => {
                if let Some(ms) = ms {
                    self.idle(Duration::from_millis(*ms).min(MAX_WAIT)).await?;
                }
                if let Some(n) = frames {
                    self.wait_frames(*n).await?;
                }
                Ok(Value::Null)
            }
            Op::PlayerState { fields } => {
                let v = self.ui_player_state(false, false).await?;
                Ok(if fields.is_empty() {
                    v
                } else {
                    crate::server::compact::project(&v, fields)
                })
            }
            Op::WindowText { window, children } => {
                self.lua_json(&window_text_chunk(window, *children)).await
            }
        }
    }

    /// `client_batch`: run `steps` in order. A failed step records
    /// `{"error": ...}`; with `stop_on_error` the batch ends there.
    pub async fn batch(&self, steps: &[Step], stop_on_error: bool) -> BatchOutcome {
        let t0 = Instant::now();
        let mut values = Map::new();
        let mut stopped_at = None;
        for s in steps {
            match self.batch_step(&s.op, &values).await {
                Ok(Value::Null) => {}
                Ok(v) => {
                    values.insert(s.id.clone(), v);
                }
                Err(e) => {
                    values.insert(s.id.clone(), json!({ "error": e }));
                    if stop_on_error {
                        stopped_at = Some(s.id.clone());
                        break;
                    }
                }
            }
        }
        BatchOutcome {
            values,
            stopped_at,
            ms: t0.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(test)]
#[path = "batch_tests.rs"]
mod tests;
