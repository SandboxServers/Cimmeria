//! A tiny JSON encoder that runs inside the client's Lua VM, so a reader
//! can return nested data (containers of slots, a window subtree, chat
//! lines) as one string that `serde_json` parses, instead of the tagged
//! TSV lines the flow reads use.
//!
//! The client's Lua is 5.1 with wide strings; the encoder only uses
//! `string.gsub`, `string.format`, `string.byte`, `tostring`, `pairs` and
//! `table.concat`, which the stock UI code uses too. Userdata (CEGUI
//! objects, vectors) is encoded as its `tostring`; readers that need a
//! vector's fields call `__jvec` explicitly. NaN and infinities become
//! `null` (JSON has no spelling for them).

use serde_json::Value;

/// The encoder, prepended to every reader chunk. Defines the locals
/// `__jenc(value)` (JSON text), `__jvec(v)` (`{x, y, z}` of a vector
/// userdata, or nil) and `__jcall(f, ...)` (a pcall that returns the
/// value or nil, so one missing binding never sinks a whole read).
pub const PRELUDE: &str = r#"local __jmap = { ['"'] = '\\"', ['\\'] = '\\\\', ['\n'] = '\\n', ['\r'] = '\\r', ['\t'] = '\\t' }
local function __jesc(s)
  return (string.gsub(tostring(s), '[%c"\\]', function(c) return __jmap[c] or string.format('\\u%04x', string.byte(c)) end))
end
local __jenc
__jenc = function(v, d)
  local t = type(v)
  if t == 'string' then return '"' .. __jesc(v) .. '"' end
  if t == 'number' then
    if v ~= v or v == math.huge or v == -math.huge then return 'null' end
    return tostring(v)
  end
  if t == 'boolean' then return v and 'true' or 'false' end
  if t == 'nil' then return 'null' end
  if t == 'table' then
    d = (d or 0) + 1
    if d > 8 then return '"<deep>"' end
    local parts = {}
    local n = #v
    if n > 0 then
      for i = 1, n do parts[i] = __jenc(v[i], d) end
      return '[' .. table.concat(parts, ',') .. ']'
    end
    for k, x in pairs(v) do
      local tk = type(k)
      if tk == 'string' or tk == 'number' then
        parts[#parts + 1] = '"' .. __jesc(k) .. '":' .. __jenc(x, d)
      end
    end
    return '{' .. table.concat(parts, ',') .. '}'
  end
  return '"' .. __jesc(tostring(v)) .. '"'
end
local function __jcall(f, ...)
  if type(f) ~= 'function' then return nil end
  local r = { pcall(f, ...) }
  if r[1] then return r[2] end
  return nil
end
local function __jvec(v)
  if v == nil then return nil end
  local ok, r = pcall(function() return { x = v.x, y = v.y, z = v.z } end)
  if ok then return r end
  return nil
end
"#;

/// Prefix `body` (which must end in `return __jenc(...)`) with the encoder.
pub fn chunk(body: &str) -> String {
    format!("{PRELUDE}{body}")
}

/// Parse the one JSON string a reader chunk returns.
pub fn decode(results: &[String]) -> Result<Value, String> {
    let text = results
        .first()
        .ok_or_else(|| "the reader returned nothing".to_string())?;
    serde_json::from_str(text).map_err(|e| {
        let head: String = text.chars().take(160).collect();
        format!("the reader returned invalid JSON ({e}): {head}")
    })
}

/// The encoder writes an empty Lua table as `{}`; callers that expect a
/// list read it through this, which treats `{}` (and null) as empty.
pub fn list(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_reads_the_first_result() {
        let v = decode(&[r#"{"a":[1,2],"b":"x"}"#.to_string()]).unwrap();
        assert_eq!(v["a"][1], 2);
        assert_eq!(v["b"], "x");
    }

    #[test]
    fn decode_names_invalid_json_and_an_empty_result() {
        assert!(decode(&[]).unwrap_err().contains("nothing"));
        let e = decode(&["{oops".to_string()]).unwrap_err();
        assert!(e.contains("invalid JSON") && e.contains("{oops"));
    }

    #[test]
    fn empty_tables_read_as_empty_lists() {
        assert!(list(&serde_json::json!({})).is_empty());
        assert!(list(&Value::Null).is_empty());
        assert_eq!(list(&serde_json::json!([1])).len(), 1);
    }

    /// The prelude is spliced into the bridge's capture wrapper as plain
    /// source: it must define the three helpers and keep the escapes that
    /// make control characters and quotes valid JSON.
    #[test]
    fn prelude_defines_the_helpers_and_escapes() {
        for name in [
            "local __jenc",
            "local function __jcall",
            "local function __jvec",
        ] {
            assert!(PRELUDE.contains(name), "{name}");
        }
        assert!(PRELUDE.contains(r#"'[%c"\\]'"#));
        assert!(PRELUDE.contains(r#"'\\u%04x'"#));
        assert!(chunk("return __jenc(1)").ends_with("return __jenc(1)"));
    }
}
