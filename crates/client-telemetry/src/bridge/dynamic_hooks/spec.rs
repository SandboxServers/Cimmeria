//! Capture-spec parsing for dynamic logging hooks (issue #686 scope 4).
//!
//! A capture spec says *what a hook records when it fires*: which stack
//! arguments, which registers, which typed pointer chases, how many hits
//! to keep (`hit_limit`), and how heavily to sample (`sample_rate`). It is
//! parsed and validated up front so a malformed spec is a clean
//! `INVALID_PARAMS` before any code is patched into the client. Parsing is
//! pure and unit-tested; the native detour ([`super::native`]) reads a
//! validated spec.
//!
//! # What the native path actually captures (phase 3)
//!
//! The native entry-hook detour is **cdecl, function-entry only, no asm**
//! (see [`super::native`]). It captures the requested **stack args** and
//! **typed dereferences**. General-purpose *register* capture at entry
//! needs a naked asm stub, which the #686 spike deliberately avoided, so
//! `registers` is parsed and reported for forward-compatibility but is a
//! documented no-op on the cdecl path — a follow-up adds an asm-stub
//! capture path if a probe needs live register values. Nothing silently
//! lies: `hook_list` echoes the spec as parsed.

use serde_json::Value;

/// Registers we accept in a capture spec. Kept as a validation allowlist
/// even though the cdecl entry-detour can't snapshot them without asm
/// (see the module docs).
pub const KNOWN_REGISTERS: [&str; 8] = ["eax", "ebx", "ecx", "edx", "esi", "edi", "ebp", "esp"];

/// Typed widths a dereference can read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerefType {
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    U64,
    I64,
    F32,
    F64,
    /// Read the pointer value itself (a u32 on i686).
    Ptr,
    /// NUL-terminated narrow (ASCII/UTF-8) string.
    CStr,
    /// NUL-terminated wide (UTF-16LE) string.
    WStr,
}

impl DerefType {
    /// Parse a type name. Total — returns `None` for an unknown width so
    /// the caller can raise a specific error.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "u8" => Self::U8,
            "i8" => Self::I8,
            "u16" => Self::U16,
            "i16" => Self::I16,
            "u32" => Self::U32,
            "i32" => Self::I32,
            "u64" => Self::U64,
            "i64" => Self::I64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "ptr" => Self::Ptr,
            "cstr" => Self::CStr,
            "wstr" => Self::WStr,
            _ => return None,
        })
    }

    /// Bytes read for a fixed-width type; `None` for the variable-length
    /// string types.
    pub fn fixed_len(self) -> Option<usize> {
        Some(match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 2,
            Self::U32 | Self::I32 | Self::F32 | Self::Ptr => 4,
            Self::U64 | Self::I64 | Self::F64 => 8,
            Self::CStr | Self::WStr => return None,
        })
    }
}

/// Where a dereference chain starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerefSource {
    /// Nth captured stack argument (0-based).
    StackArg(u8),
    /// A register (only meaningful once asm register capture lands).
    Register(&'static str),
}

/// One typed pointer chase: start at `source`, add each offset chasing a
/// pointer at every step but the last, then read `ty` at the final
/// address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerefSpec {
    pub source: DerefSource,
    pub offsets: Vec<i32>,
    pub ty: DerefType,
    /// Human label emitted in the event field map; defaults to a
    /// generated name when absent.
    pub label: Option<String>,
}

/// Max stack dwords a cdecl entry-detour can read. Bounds the detour's
/// declared arity (see [`super::native`]).
pub const MAX_STACK_ARGS: u8 = 8;

/// A validated capture spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSpec {
    /// Registers to record (forward-compat; no-op on the cdecl path).
    pub registers: Vec<&'static str>,
    /// How many raw stack dwords to capture at entry (0..=[`MAX_STACK_ARGS`]).
    pub stack_args: u8,
    /// Typed pointer chases.
    pub derefs: Vec<DerefSpec>,
    /// Stop capturing (and auto-disable the hook) after this many hits.
    pub hit_limit: Option<u64>,
    /// Capture one hit in every `sample_rate` (>= 1; 1 = every hit).
    pub sample_rate: u32,
}

impl Default for CaptureSpec {
    fn default() -> Self {
        Self {
            registers: Vec::new(),
            stack_args: 0,
            derefs: Vec::new(),
            hit_limit: None,
            sample_rate: 1,
        }
    }
}

impl CaptureSpec {
    /// Parse + validate a capture spec from the `capture` object of a
    /// `hook_install` request. A missing `capture` yields the default
    /// (record nothing but the hit itself). Every failure is a specific
    /// message for `INVALID_PARAMS`.
    pub fn parse(v: &Value) -> Result<Self, String> {
        if v.is_null() {
            return Ok(Self::default());
        }
        let obj = v
            .as_object()
            .ok_or_else(|| "capture must be an object".to_string())?;

        let registers = match obj.get("registers") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(arr)) => arr
                .iter()
                .map(parse_register)
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("capture.registers must be an array".to_string()),
        };

        let stack_args = match obj.get("stack_args") {
            None | Some(Value::Null) => 0,
            Some(v) => {
                let n = v
                    .as_u64()
                    .ok_or_else(|| "capture.stack_args must be an integer".to_string())?;
                if n > MAX_STACK_ARGS as u64 {
                    return Err(format!("capture.stack_args must be <= {MAX_STACK_ARGS}"));
                }
                n as u8
            }
        };

        let derefs = match obj.get("derefs") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(arr)) => arr
                .iter()
                .map(|d| parse_deref(d, stack_args))
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err("capture.derefs must be an array".to_string()),
        };

        let hit_limit = match obj.get("hit_limit") {
            None | Some(Value::Null) => None,
            Some(v) => {
                let n = v
                    .as_u64()
                    .ok_or_else(|| "capture.hit_limit must be an integer".to_string())?;
                if n == 0 {
                    return Err("capture.hit_limit must be >= 1".to_string());
                }
                Some(n)
            }
        };

        let sample_rate = match obj.get("sample_rate") {
            None | Some(Value::Null) => 1,
            Some(v) => {
                let n = v
                    .as_u64()
                    .ok_or_else(|| "capture.sample_rate must be an integer".to_string())?;
                if n == 0 {
                    return Err("capture.sample_rate must be >= 1".to_string());
                }
                u32::try_from(n).map_err(|_| "capture.sample_rate too large".to_string())?
            }
        };

        Ok(Self {
            registers,
            stack_args,
            derefs,
            hit_limit,
            sample_rate,
        })
    }

    /// Serialize back to a JSON object for `hook_list` (echoes the spec as
    /// parsed, so a probe author sees exactly what will be captured).
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "registers": self.registers,
            "stack_args": self.stack_args,
            "derefs": self.derefs.iter().map(deref_to_json).collect::<Vec<_>>(),
            "hit_limit": self.hit_limit,
            "sample_rate": self.sample_rate,
        })
    }
}

fn parse_register(v: &Value) -> Result<&'static str, String> {
    let s = v
        .as_str()
        .ok_or_else(|| "capture.registers entries must be strings".to_string())?
        .to_ascii_lowercase();
    KNOWN_REGISTERS
        .iter()
        .copied()
        .find(|&r| r == s)
        .ok_or_else(|| format!("unknown register '{s}'"))
}

fn parse_deref(v: &Value, stack_args: u8) -> Result<DerefSpec, String> {
    let obj = v
        .as_object()
        .ok_or_else(|| "each deref must be an object".to_string())?;

    let source_s = obj
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| "deref.source is required (e.g. \"stack0\" or \"ecx\")".to_string())?;
    let source = parse_deref_source(source_s, stack_args)?;

    let offsets = match obj.get("offsets") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(arr)) => arr
            .iter()
            .map(|o| {
                o.as_i64()
                    .and_then(|n| i32::try_from(n).ok())
                    .ok_or_else(|| "deref.offsets entries must be i32".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?,
        Some(_) => return Err("deref.offsets must be an array".to_string()),
    };

    let ty_s = obj
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "deref.type is required".to_string())?;
    let ty = DerefType::parse(ty_s).ok_or_else(|| format!("unknown deref type '{ty_s}'"))?;

    let label = obj.get("label").and_then(Value::as_str).map(str::to_string);

    Ok(DerefSpec {
        source,
        offsets,
        ty,
        label,
    })
}

fn parse_deref_source(s: &str, stack_args: u8) -> Result<DerefSource, String> {
    let s = s.to_ascii_lowercase();
    if let Some(idx) = s.strip_prefix("stack") {
        let n: u8 = idx.parse().map_err(|_| format!("bad stack source '{s}'"))?;
        if n >= stack_args {
            return Err(format!(
                "deref source stack{n} needs capture.stack_args > {n}"
            ));
        }
        return Ok(DerefSource::StackArg(n));
    }
    KNOWN_REGISTERS
        .iter()
        .copied()
        .find(|&r| r == s)
        .map(DerefSource::Register)
        .ok_or_else(|| format!("bad deref source '{s}' (want stackN or a register)"))
}

fn deref_to_json(d: &DerefSpec) -> Value {
    let source = match d.source {
        DerefSource::StackArg(n) => format!("stack{n}"),
        DerefSource::Register(r) => r.to_string(),
    };
    let ty = match d.ty {
        DerefType::U8 => "u8",
        DerefType::I8 => "i8",
        DerefType::U16 => "u16",
        DerefType::I16 => "i16",
        DerefType::U32 => "u32",
        DerefType::I32 => "i32",
        DerefType::U64 => "u64",
        DerefType::I64 => "i64",
        DerefType::F32 => "f32",
        DerefType::F64 => "f64",
        DerefType::Ptr => "ptr",
        DerefType::CStr => "cstr",
        DerefType::WStr => "wstr",
    };
    serde_json::json!({
        "source": source,
        "offsets": d.offsets,
        "type": ty,
        "label": d.label,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_capture_is_default() {
        let s = CaptureSpec::parse(&Value::Null).unwrap();
        assert_eq!(s, CaptureSpec::default());
        assert_eq!(s.sample_rate, 1);
        assert_eq!(s.stack_args, 0);
    }

    #[test]
    fn full_spec_parses() {
        let s = CaptureSpec::parse(&json!({
            "registers": ["ECX", "edx"],
            "stack_args": 3,
            "derefs": [
                { "source": "stack0", "offsets": [0, 16], "type": "u32", "label": "hp" },
                { "source": "ecx", "offsets": [], "type": "wstr" }
            ],
            "hit_limit": 100,
            "sample_rate": 10
        }))
        .unwrap();
        assert_eq!(s.registers, vec!["ecx", "edx"]);
        assert_eq!(s.stack_args, 3);
        assert_eq!(s.derefs.len(), 2);
        assert_eq!(s.derefs[0].source, DerefSource::StackArg(0));
        assert_eq!(s.derefs[0].offsets, vec![0, 16]);
        assert_eq!(s.derefs[0].ty, DerefType::U32);
        assert_eq!(s.derefs[0].label.as_deref(), Some("hp"));
        assert_eq!(s.derefs[1].source, DerefSource::Register("ecx"));
        assert_eq!(s.derefs[1].ty, DerefType::WStr);
        assert_eq!(s.hit_limit, Some(100));
        assert_eq!(s.sample_rate, 10);
    }

    #[test]
    fn zero_sample_rate_and_hit_limit_rejected() {
        assert!(CaptureSpec::parse(&json!({ "sample_rate": 0 })).is_err());
        assert!(CaptureSpec::parse(&json!({ "hit_limit": 0 })).is_err());
    }

    #[test]
    fn stack_args_over_cap_rejected() {
        assert!(CaptureSpec::parse(&json!({ "stack_args": 9 })).is_err());
        assert!(CaptureSpec::parse(&json!({ "stack_args": 8 })).is_ok());
    }

    #[test]
    fn unknown_register_and_type_rejected() {
        assert!(CaptureSpec::parse(&json!({ "registers": ["r15"] })).is_err());
        assert!(CaptureSpec::parse(&json!({
            "stack_args": 1,
            "derefs": [{ "source": "stack0", "type": "u128" }]
        }))
        .is_err());
    }

    #[test]
    fn deref_source_out_of_range_rejected() {
        // stack1 needs stack_args > 1.
        let e = CaptureSpec::parse(&json!({
            "stack_args": 1,
            "derefs": [{ "source": "stack1", "type": "u32" }]
        }))
        .unwrap_err();
        assert!(e.contains("stack_args"), "{e}");
    }

    #[test]
    fn roundtrips_through_json() {
        let src = json!({
            "registers": ["ecx"],
            "stack_args": 2,
            "derefs": [{ "source": "stack1", "offsets": [4], "type": "i32", "label": "dmg" }],
            "hit_limit": 5,
            "sample_rate": 2
        });
        let s = CaptureSpec::parse(&src).unwrap();
        let back = CaptureSpec::parse(&s.to_json()).unwrap();
        assert_eq!(s, back);
    }
}
