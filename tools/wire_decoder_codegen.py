#!/usr/bin/env python3
"""Generate per-method Rust decoders for the SGWPlayer wire surface.

Reads schemas from `docs/protocol/client-method-dispatch-table.md` and
emits a Rust file the wire_log module compiles in. The generated file
covers every method whose schema parses cleanly; methods using
composite types not covered here (StatUpdateList, ClientEffectResultList,
MAILBOX, NameValuePair, FIXED_DICTs) fall back to the hand-written
priority decoders in `outbound.rs`.

Run from the repo root:
    python tools/wire_decoder_codegen.py \
        > crates/services/src/wire_log/decoders/generated.rs
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


PRIMITIVES: dict[str, str] = {
    "INT8": "c.i8()?",
    "INT16": "c.i16_le()?",
    "INT32": "c.i32_le()?",
    "INT64": "c.i64_le()?",
    "UINT8": "c.u8()?",
    "UINT16": "c.u16_le()?",
    "UINT32": "c.u32_le()?",
    "UINT64": "c.u64_le()?",
    "FLOAT": "c.f32_le()?",
    "WSTRING": "c.wstring()?",
    "BOOL": "(c.u8()? != 0)",
}

# Helper functions emitted at the top of the file for ARRAY decoding.
ARRAY_HELPERS = """
/// Read an ARRAY<primitive> by reading a u32 count then `min(count,
/// 32)` repeats. Truncates at 32 to bound per-event size; `count` and
/// `truncated` fields preserve the original size for SigNoz queries.
macro_rules! read_array {
    ($cursor:expr, $read:expr) => {{
        let count = $cursor.u32_le()?;
        let cap = (count as usize).min(32);
        let mut items: Vec<Value> = Vec::with_capacity(cap);
        for _ in 0..cap {
            items.push(json!($read));
        }
        json!({
            "count": count,
            "items": items,
            "truncated": count > 32,
        })
    }};
}
"""


# Composite types we recognize but can't decode cleanly without
# per-type schemas. These methods skip codegen entirely and fall
# through to the hand-written priority decoders (or to the
# `args_hex` fallback) so the generated file stays correct-by-default.
UNDECODABLE = {
    "StatUpdateList",
    "ClientEffectResultList",
    "MAILBOX",
    "NameValuePair",
    "RosterInfo",
    "BMAuction",
    "MissionRewardsList",
    "MissionOffer",
    "AbilityTreeInfo",
    "StoreItem",
    "TradeProposalItem",
    "TradeResults",
    "ContactListEntry",
    "MinigameContact",
    "MailHeader",
    "MailAttachment",
}


@dataclass
class Arg:
    """One arg in a method schema. `kind` is the parsed shape."""
    name: str
    rust_decoder: str  # the Rust expr that reads + returns a serde_json::Value
    # If this arg is variable-length (WSTRING, ARRAY), the decoder
    # needs to do bounds checks at runtime; we just emit `?` chains.


@dataclass
class Method:
    index: int
    name: str
    args: list[Arg]


def parse_schema(schema: str) -> list[Arg] | None:
    """Tokenize a schema string into args. Return None if unsupported."""
    schema = schema.strip()
    if schema == "*(none)*" or schema == "—":
        return []

    # Split by comma — but ARRAY<T> may contain a nested type with
    # commas? In practice the doc never uses commas inside ARRAY<>;
    # the type T is always a single token. Verified by hand-scan.
    parts = [p.strip() for p in schema.split(",") if p.strip()]
    out: list[Arg] = []

    for part in parts:
        # Special syntactic sugar in the docs: `FLOAT locationX/Y/Z`
        # means three sequential floats with the X/Y/Z suffixes. Expand
        # before the normal parse.
        slash = re.match(r"^(\w+)\s+(\w+)([XYZ])(?:/[XYZ])+$", part)
        if slash:
            type_tok = slash.group(1)
            base_name = slash.group(2)
            if type_tok not in PRIMITIVES:
                return None
            for suffix in "XYZ":
                out.append(make_primitive_arg(type_tok, f"{base_name}{suffix}"))
            continue

        # Special: `INT8x3 dir` → three i8's
        repeat = re.match(r"^(\w+)x(\d+)\s+(\w+)$", part)
        if repeat:
            type_tok = repeat.group(1)
            count = int(repeat.group(2))
            base_name = repeat.group(3)
            if type_tok not in PRIMITIVES:
                return None
            for i in range(count):
                out.append(make_primitive_arg(type_tok, f"{base_name}_{i}"))
            continue

        # Generic: `<TYPE> <NAME>` or `ARRAY<T> <NAME>`
        m = re.match(r"^(\S+)\s+(\w+)$", part)
        if not m:
            # Possibly a bare composite type with no field name (e.g.
            # `ClientEffectResultList`) — skip the whole method.
            if part in UNDECODABLE:
                return None
            return None
        type_tok, name = m.group(1), m.group(2)

        if type_tok in PRIMITIVES:
            out.append(make_primitive_arg(type_tok, name))
            continue

        # VECTOR3 = three floats.
        if type_tok == "VECTOR3":
            for suffix in "xyz":
                out.append(make_primitive_arg("FLOAT", f"{name}_{suffix}"))
            continue

        # ARRAY<T> — read count, then T repeats. Only supported when
        # T is a primitive; nested composite arrays need hand-decode.
        # Emits `read_array!(c, <inner_expr>)` which calls the macro
        # defined at the top of the generated file.
        arr = re.match(r"^ARRAY<(\w+)>$", type_tok)
        if arr:
            inner = arr.group(1)
            if inner in UNDECODABLE:
                return None
            if inner not in PRIMITIVES:
                return None
            inner_expr = PRIMITIVES[inner]
            out.append(Arg(name=name, rust_decoder=f"read_array!(c, {inner_expr})"))
            continue

        # Unrecognized composite — bail on the whole method, let the
        # hand-written decoder or args_hex cover it.
        return None

    return out


def make_primitive_arg(type_tok: str, name: str) -> Arg:
    """Build an Arg for a primitive type token. The decoder expr is
    suitable for use as both a let-binding RHS and a json! value."""
    return Arg(name=name, rust_decoder=PRIMITIVES[type_tok])


def parse_dispatch_table(path: Path) -> list[Method]:
    """Extract all `| idx | name | args |` rows from the dispatch table."""
    pattern = re.compile(r"^\|\s*(\d+)\s*\|\s*`([^`]+)`\s*\|\s*`([^`]*)`\s*\|?")
    methods: list[Method] = []
    for line in path.read_text(encoding="utf-8").splitlines():
        m = pattern.match(line)
        if not m:
            continue
        idx = int(m.group(1))
        name = m.group(2)
        schema = m.group(3)
        args = parse_schema(schema)
        if args is None:
            # Unsupported schema — skip; hand-decoder or fallback covers it.
            continue
        methods.append(Method(index=idx, name=name, args=args))
    return methods


def emit_decoder(method: Method) -> str:
    """Return Rust source for the per-method decoder fn.

    Emits a let-binding per arg, then a json! that references each
    binding. This avoids putting block expressions inside the json!
    macro (which json! doesn't accept) while still letting each let
    use `?` for the bail-out path.
    """
    lines = [
        f"/// Method {method.index}: `{method.name}`. Auto-generated.",
        f"pub(super) fn decode_{method.index}(args: &[u8]) -> Option<Value> {{",
        "    let mut c = Cursor::new(args);",
    ]
    if not method.args:
        lines.append("    let _ = c;")
        lines.append("    Some(json!({}))")
        lines.append("}")
        return "\n".join(lines)

    # Compute each arg into a local. Use field-name as the var name,
    # lowercased for Rust convention; if it collides with a Rust
    # keyword the var gets `_arg` suffix.
    KEYWORDS = {"type", "ref", "fn", "let", "mut", "move", "match", "self"}
    var_names: list[str] = []
    for arg in method.args:
        var = arg.name
        # Lowercase first letter for Rust local-var convention
        var = var[0].lower() + var[1:] if var else var
        # If still ambiguous (e.g., starts with digit after lower), prefix
        if var and not (var[0].isalpha() or var[0] == "_"):
            var = "_" + var
        if var in KEYWORDS:
            var = var + "_"
        var_names.append(var)
        lines.append(f"    let {var} = {arg.rust_decoder};")

    lines.append("    Some(json!({")
    for arg, var in zip(method.args, var_names):
        lines.append(f"        \"{arg.name}\": {var},")
    lines.append("    }))")
    lines.append("}")
    return "\n".join(lines)


def emit_dispatch(methods: list[Method]) -> str:
    """Return the dispatch fn that routes by method_index."""
    arms = "\n".join(
        f"        {m.index} => decode_{m.index}(args),"
        for m in methods
    )
    return f"""
/// Dispatch — returns `Some` when the codegen has a decoder for this
/// method index. Methods missing here fall through to the hand-
/// written decoders in [`crate::wire_log::decoders::outbound`], then
/// to the args_hex fallback.
pub(super) fn decode_generated(method_index: u16, args: &[u8]) -> Option<Value> {{
    match method_index {{
{arms}
        _ => None,
    }}
}}
"""


HEADER = """//! Auto-generated wire-format decoders for SGWPlayer client methods.
//!
//! **DO NOT EDIT BY HAND** — regenerate via:
//!
//! ```sh
//! python tools/wire_decoder_codegen.py > \\
//!     crates/services/src/wire_log/decoders/generated.rs
//! cargo fmt -p cimmeria-services
//! ```
//!
//! Source schemas: `docs/protocol/client-method-dispatch-table.md`.
//! Schemas using composite types (StatUpdateList, ClientEffectResultList,
//! NameValuePair, FIXED_DICTs, MAILBOX, etc.) are skipped here — they
//! fall through to the hand-written decoders in
//! [`crate::wire_log::decoders::outbound`], which override the
//! generated table when both define the same method.

// Local variable names mirror the protocol's PascalCase / camelCase
// field names so the codegen stays simple and the generated source
// reads alongside the dispatch table. Clippy / rustc would otherwise
// flag every binding.
#![allow(non_snake_case, clippy::needless_question_mark)]

use serde_json::{json, Value};

use super::primitives::Cursor;
""" + ARRAY_HELPERS


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    dispatch_table = repo_root / "docs" / "protocol" / "client-method-dispatch-table.md"
    if not dispatch_table.exists():
        sys.stderr.write(f"missing: {dispatch_table}\n")
        return 1

    methods = parse_dispatch_table(dispatch_table)
    sys.stderr.write(
        f"parsed {len(methods)} decodable methods out of the full set\n"
    )

    parts = [HEADER, ""]
    for m in methods:
        parts.append(emit_decoder(m))
        parts.append("")
    parts.append(emit_dispatch(methods))

    sys.stdout.write("\n".join(parts))
    return 0


if __name__ == "__main__":
    sys.exit(main())
