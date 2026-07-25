#!/usr/bin/env python3
"""
re_parity.py — LLM-free structural parity check for reverse-engineered functions.

Ported from the deterministic verification layer of Dryxio/auto-re-agent
(the parity engine + objective verifier), with the LLM "reverser/checker"
brains stripped out. This script uses **no network and no LLM** — every
signal is a purely structural heuristic over three text inputs:

  * --decompile  Ghidra decompiled C pseudocode for the target function
  * --source     your reconstructed source / notes (C, Rust, or pseudocode)
  * --asm        (optional) the disassembly listing for the target

It answers one question cheaply and offline: *does the reconstruction look
structurally consistent with the binary, or did we hallucinate a simpler
function than the bytes actually contain?* It is the gate that catches an
LLM (or a human) confidently writing a 6-line stub for an 80-instruction
function.

It reports two things:

  1. Parity signals — 11 heuristics classified RED (blocking), YELLOW
     (inspect), or INFO. Ported 1:1 from auto-re-agent/parity/signals.py.
  2. Objective verdict — a conservative structural PASS/FAIL/UNKNOWN over
     call-count and control-flow gaps. Ported from
     auto-re-agent/verification/objective.py.

Exit code is 1 when the result is blocking (any RED signal or an objective
FAIL), else 0 — so it drops straight into a reverser/checker loop as a gate.

The "plugin call" concept from the original (GTA plugin-SDK specific) is
generalised here to a configurable `--wrapper-prefix` list; with none set
(the default) the plugin-specific signals stay inert rather than misfiring.

Usage:
    python tools/re_parity.py --decompile dc.c --source recon.rs [--asm fn.asm]
    python tools/re_parity.py --decompile dc.c --source recon.rs --callee-count 9 --json
    python tools/re_parity.py --selftest

--callee-count overrides the decompile-derived callee count with an
authoritative value (e.g. from a Ghidra call-graph / xref query via MCP),
which sharpens the low-call-count signal and the objective call check.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# --- severities ------------------------------------------------------------

RED = "RED"      # blocking — reconstruction is almost certainly wrong/incomplete
YELLOW = "YELLOW"  # warning — worth a manual look
INFO = "INFO"    # informational — no action implied

# --- tokens ----------------------------------------------------------------

# Keywords that look like `name(` but are not function calls.
_KEYWORDS = {
    "if", "for", "while", "switch", "return", "sizeof", "do", "else", "case",
    "catch", "defined", "alignof", "decltype", "typeid", "typeof",
    "static_cast", "reinterpret_cast", "const_cast", "dynamic_cast",
    # Rust
    "match", "loop", "fn", "let", "mut", "unsafe", "impl", "struct", "enum",
    "union", "trait", "where", "move", "ref", "as", "dyn",
}

# Ghidra decompiler intrinsics that print like calls but are pseudo-ops.
_GHIDRA_INTRINSIC = re.compile(
    r"^(CONCAT|SUB|ZEXT|SEXT|CARRY|SCARRY|SBORROW|NAN|ABS|POPCOUNT|SQRT|"
    r"ROUND|TRUNC|CEIL|FLOOR|INT|FLOAT)\d*$"
)

# Macro names that are stubs, not real calls, for call-counting purposes.
_STUB_MACROS = {"todo", "unimplemented", "unreachable"}

# Default stub markers (RED) — extend with --stub-marker.
_DEFAULT_STUB_MARKERS = [
    "NOTSA_UNREACHABLE",
    "todo!",
    "unimplemented!",
    "unreachable!",
    "STUB",
    "__stub",
    "/* stub */",
    "// stub",
    "not implemented",
    "NotImplemented",
    "FIXME: stub",
]

# x86 floating-point / SSE mnemonics used to detect FP work in disassembly.
_FP_MNEMONICS = {
    "movss", "movsd", "movaps", "movups", "addss", "addsd", "subss", "subsd",
    "mulss", "mulsd", "divss", "divsd", "minss", "minsd", "maxss", "maxsd",
    "sqrtss", "sqrtsd", "rcpss", "rsqrtss", "cvtsi2ss", "cvtsi2sd",
    "cvtss2sd", "cvtsd2ss", "cvttss2si", "cvttsd2si", "cvtss2si", "cvtsd2si",
    "comiss", "comisd", "ucomiss", "ucomisd", "xorps", "andps", "orps",
    "unpcklps", "shufps", "pshufd",
    # x87
    "fld", "fst", "fstp", "fadd", "faddp", "fsub", "fsubp", "fmul", "fmulp",
    "fdiv", "fdivp", "fcom", "fcomp", "fcomi", "fild", "fist", "fistp",
    "fsqrt", "fabs", "fchs", "fxch", "fucom", "fucomi", "fldz", "fld1",
}

_MATH_TOKEN_RE = re.compile(
    r"[*/%]"                                  # multiplicative operators
    r"|\bf32\b|\bf64\b|\bfloat\b|\bdouble\b"  # float types
    r"|\b(?:sqrt|sqrtf|sin|cos|tan|atan2?|fabs|fabsf|abs|pow|powf|powi|"
    r"floor|ceil|round|exp|log|hypot|lerp|clamp|min|max)\b"
    r"|\b\d+\.\d+\b"                          # float literals
    r"|\b0x[0-9a-fA-F]+p[+-]?\d+\b",          # hex float literals
)

_NAN_RE = re.compile(r"\bNAN\s*\(|\bisnan\b|\bis_nan\b|\bf32::NAN\b|\bf64::NAN\b")

_CALL_RE = re.compile(r"(?<![\w.])([A-Za-z_]\w*)\s*(!?)\s*\(")
# method / free-fn calls including `.foo(`, but not `.0(` tuple indexing etc.
_METHOD_CALL_RE = re.compile(r"\.\s*([A-Za-z_]\w*)\s*(!?)\s*\(")

_CF_KEYWORDS = ("if", "for", "while", "case", "goto", "switch", "match", "loop")
_CF_KEYWORD_RE = re.compile(r"\b(?:" + "|".join(_CF_KEYWORDS) + r")\b")


# --- metrics ---------------------------------------------------------------


@dataclass
class Metrics:
    """Structural metrics extracted from the three inputs."""

    source_lines: int = 0
    source_calls: int = 0
    source_wrapper_calls: int = 0
    source_cf: int = 0
    source_has_math: bool = False
    source_has_isnan: bool = False

    decompile_calls: int = 0
    decompile_cf: int = 0
    decompile_has_nan: bool = False
    decompile_has_fp: bool = False

    asm_present: bool = False
    asm_instr: int = 0
    asm_calls: int = 0
    asm_has_fp: bool = False

    # Authoritative callee count (from Ghidra call graph) if provided, else
    # falls back to decompile_calls.
    callee_count: int = 0

    @property
    def source_nonwrapper_calls(self) -> int:
        return max(0, self.source_calls - self.source_wrapper_calls)

    @property
    def has_fp(self) -> bool:
        return self.asm_has_fp or self.decompile_has_fp

    @property
    def reference_calls(self) -> int:
        """Calls to compare source against: asm if present, else decompile."""
        return self.asm_calls if self.asm_present else self.decompile_calls


def _iter_call_names(text: str):
    for m in _CALL_RE.finditer(text):
        yield m.group(1), (m.group(2) == "!")
    for m in _METHOD_CALL_RE.finditer(text):
        yield m.group(1), (m.group(2) == "!")


def count_calls(text: str, wrapper_prefixes: tuple[str, ...]) -> tuple[int, int]:
    """Return (total_calls, wrapper_calls) over C/Rust-ish text.

    Excludes control-flow keywords, Ghidra intrinsics, and stub macros.
    """
    total = 0
    wrapper = 0
    for name, is_macro in _iter_call_names(text):
        if name in _KEYWORDS:
            continue
        if _GHIDRA_INTRINSIC.match(name):
            continue
        if is_macro and name in _STUB_MACROS:
            continue
        total += 1
        if wrapper_prefixes and name.startswith(wrapper_prefixes):
            wrapper += 1
    return total, wrapper


def count_control_flow(text: str) -> int:
    """Approximate CFG edge count: branch/loop keywords + short-circuits.

    Applied identically to decompile and source so the comparison is fair
    even if absolute counts are rough.
    """
    cf = len(_CF_KEYWORD_RE.findall(text))
    cf += text.count("&&") + text.count("||")
    return cf


_SRC_SKIP_LINES = {"{", "}", "};", ")", ");", "(", "],", "],", "]", "[", "});"}


def count_source_lines(text: str) -> int:
    """Count meaningful source lines — skip blanks, bare braces, comments."""
    n = 0
    for raw in text.splitlines():
        s = raw.strip()
        if not s or s in _SRC_SKIP_LINES:
            continue
        if s.startswith(("//", "/*", "*", "*/", "#")):
            continue
        n += 1
    return n


def count_asm(text: str) -> tuple[int, int, bool]:
    """Return (instruction_count, call_count, has_fp) over a disasm listing."""
    instr = 0
    calls = 0
    has_fp = False
    for raw in text.splitlines():
        s = raw.strip()
        if not s or s.startswith((";", "#", "//")):
            continue
        # Tokenise; the mnemonic is the first alphabetic token on the line,
        # skipping a leading address/byte column.
        tokens = re.findall(r"[A-Za-z][A-Za-z0-9]*", s)
        if not tokens:
            continue
        mnem = None
        for tok in tokens:
            low = tok.lower()
            # skip pure hex address/byte columns
            if re.fullmatch(r"[0-9a-fA-F]{2,}", tok) and low not in _FP_MNEMONICS:
                continue
            mnem = low
            break
        if mnem is None:
            continue
        instr += 1
        if mnem in ("call", "callf"):
            calls += 1
        if mnem in _FP_MNEMONICS:
            has_fp = True
    return instr, calls, has_fp


def extract_metrics(
    decompile: str,
    source: str,
    asm: str | None,
    wrapper_prefixes: tuple[str, ...],
    callee_count: int | None,
) -> Metrics:
    m = Metrics()

    m.source_lines = count_source_lines(source)
    m.source_calls, m.source_wrapper_calls = count_calls(source, wrapper_prefixes)
    m.source_cf = count_control_flow(source)
    m.source_has_math = bool(_MATH_TOKEN_RE.search(source))
    m.source_has_isnan = bool(_NAN_RE.search(source))

    m.decompile_calls, _ = count_calls(decompile, wrapper_prefixes)
    m.decompile_cf = count_control_flow(decompile)
    m.decompile_has_nan = bool(_NAN_RE.search(decompile))
    m.decompile_has_fp = bool(re.search(r"\bfloat\b|\bdouble\b|\b__m128\b", decompile))

    if asm is not None:
        m.asm_present = True
        m.asm_instr, m.asm_calls, m.asm_has_fp = count_asm(asm)

    m.callee_count = callee_count if callee_count is not None else m.decompile_calls
    return m


# --- signals (parity engine) ----------------------------------------------


@dataclass
class Signal:
    code: str
    severity: str
    message: str

    def as_dict(self) -> dict:
        return {"code": self.code, "severity": self.severity, "message": self.message}


def run_signals(m: Metrics, source: str) -> list[Signal]:
    """Ported 1:1 from auto-re-agent/parity/signals.py (11 heuristics)."""
    out: list[Signal] = []

    # 1 — missing source (RED)
    if not source.strip():
        out.append(Signal("missing_source", RED, "reconstructed source is empty"))
        return out  # nothing else is meaningful

    # 3 — trivial stub (RED): <=14 lines, <=1 non-wrapper calls, zero CF
    if m.source_lines <= 14 and m.source_nonwrapper_calls <= 1 and m.source_cf == 0:
        out.append(Signal(
            "trivial_stub", RED,
            f"looks like a trivial stub: {m.source_lines} lines, "
            f"{m.source_nonwrapper_calls} non-wrapper call(s), no control flow",
        ))

    # 4 — large asm, tiny source (RED): asm >=80 instr but source <=12 lines
    if m.asm_present and m.asm_instr >= 80 and m.source_lines <= 12:
        out.append(Signal(
            "large_asm_tiny_source", RED,
            f"{m.asm_instr} asm instructions but only {m.source_lines} source "
            f"lines - reconstruction is almost certainly incomplete",
        ))

    # 5 — wrapper-call heavy (YELLOW): wrappers dominate, body not trivial
    if (m.source_wrapper_calls >= 2 * max(1, m.source_nonwrapper_calls)
            and m.source_lines > 14):
        out.append(Signal(
            "wrapper_call_heavy", YELLOW,
            f"wrapper calls ({m.source_wrapper_calls}) dominate real calls "
            f"({m.source_nonwrapper_calls}) - confirm the body is real",
        ))

    # 6 — short body (YELLOW): <6 lines
    if m.source_lines < 6:
        out.append(Signal(
            "short_body", YELLOW,
            f"very short body ({m.source_lines} lines) - inspect manually",
        ))

    # 7 — low call count vs decompile (YELLOW)
    if m.source_calls <= 1 and m.callee_count >= 6:
        out.append(Signal(
            "low_call_count", YELLOW,
            f"source has {m.source_calls} call(s) but the binary shows "
            f"{m.callee_count} callees",
        ))

    # 8 — FP sensitivity (YELLOW): FP in binary, no math tokens in source
    if m.has_fp and not m.source_has_math:
        out.append(Signal(
            "fp_sensitivity", YELLOW,
            "binary contains floating-point ops but source has no math tokens",
        ))

    # 9 — call-count mismatch (YELLOW): |ref - source| > 3
    if abs(m.reference_calls - m.source_calls) > 3:
        ref_from = "asm" if m.asm_present else "decompile"
        out.append(Signal(
            "call_count_mismatch", YELLOW,
            f"call count diverges by {abs(m.reference_calls - m.source_calls)} "
            f"({ref_from}={m.reference_calls}, source={m.source_calls})",
        ))

    # 10 — NaN logic (YELLOW): decompile NaN-sensitive, source isn't
    if m.decompile_has_nan and not m.source_has_isnan:
        out.append(Signal(
            "nan_logic", YELLOW,
            "decompile has NaN-sensitive logic but source lacks isnan/NAN handling",
        ))

    # 2 — stub markers (RED) — checked against the raw source text
    markers = [mk for mk in _stub_markers_active if mk.lower() in source.lower()]
    if markers:
        out.append(Signal(
            "stub_markers", RED,
            f"source contains stub marker(s): {', '.join(sorted(set(markers)))}",
        ))

    # 11 — inline wrapper (INFO): single forwarding call, no control flow
    if m.source_cf == 0 and m.source_calls == 1 and m.source_lines <= 4:
        out.append(Signal(
            "inline_wrapper", INFO,
            "looks like an inline forwarding wrapper to an internal impl",
        ))

    return out


# Populated from CLI before run_signals is called.
_stub_markers_active: list[str] = list(_DEFAULT_STUB_MARKERS)


# --- objective verifier ----------------------------------------------------

PASS = "PASS"
FAIL = "FAIL"
UNKNOWN = "UNKNOWN"


@dataclass
class ObjectiveVerdict:
    verdict: str
    checks_run: int
    reasons: list[str] = field(default_factory=list)

    def as_dict(self) -> dict:
        return {
            "verdict": self.verdict,
            "checks_run": self.checks_run,
            "reasons": self.reasons,
        }


def objective_verify(
    m: Metrics, source: str, call_tol: int = 3, cf_tol: int = 2
) -> ObjectiveVerdict:
    """Conservative, LLM-free structural verifier.

    Ported from auto-re-agent/verification/objective.py. Fails only on
    *strong* structural mismatches where the source has materially fewer
    calls or branches than the binary reference. Deliberately tolerant.
    """
    if not source.strip():
        return ObjectiveVerdict(FAIL, 0, ["source is empty"])

    reasons: list[str] = []
    checks_run = 0
    failed = False

    # call-count check vs decompile/callee count
    ref_calls = m.callee_count
    if ref_calls > 0 or m.decompile_calls > 0:
        checks_run += 1
        gap = ref_calls - m.source_calls
        if gap >= call_tol:
            failed = True
            reasons.append(
                f"call gap {gap} >= {call_tol} (binary={ref_calls}, "
                f"source={m.source_calls})"
            )

    # control-flow check vs decompile
    if m.decompile_cf > 0:
        checks_run += 1
        gap = m.decompile_cf - m.source_cf
        if gap >= cf_tol:
            failed = True
            reasons.append(
                f"control-flow gap {gap} >= {cf_tol} (decompile={m.decompile_cf}, "
                f"source={m.source_cf})"
            )

    # asm-call check (only when a disassembly listing is available)
    if m.asm_present and m.asm_calls > 0:
        checks_run += 1
        gap = m.asm_calls - m.source_calls
        if gap >= call_tol:
            failed = True
            reasons.append(
                f"asm call gap {gap} >= {call_tol} (asm={m.asm_calls}, "
                f"source={m.source_calls})"
            )

    if failed:
        return ObjectiveVerdict(FAIL, checks_run, reasons)
    if checks_run == 0:
        return ObjectiveVerdict(UNKNOWN, 0, ["insufficient structural data"])
    return ObjectiveVerdict(PASS, checks_run, reasons)


# --- top-level result ------------------------------------------------------


@dataclass
class ParityResult:
    verdict: str
    blocking: bool
    signals: list[Signal]
    objective: ObjectiveVerdict
    metrics: Metrics

    def as_dict(self) -> dict:
        return {
            "verdict": self.verdict,
            "blocking": self.blocking,
            "signals": [s.as_dict() for s in self.signals],
            "objective": self.objective.as_dict(),
            "metrics": {
                "source_lines": self.metrics.source_lines,
                "source_calls": self.metrics.source_calls,
                "source_cf": self.metrics.source_cf,
                "decompile_calls": self.metrics.decompile_calls,
                "decompile_cf": self.metrics.decompile_cf,
                "callee_count": self.metrics.callee_count,
                "asm_present": self.metrics.asm_present,
                "asm_instr": self.metrics.asm_instr,
                "asm_calls": self.metrics.asm_calls,
            },
        }


def evaluate(
    decompile: str,
    source: str,
    asm: str | None,
    wrapper_prefixes: tuple[str, ...] = (),
    callee_count: int | None = None,
    call_tol: int = 3,
    cf_tol: int = 2,
) -> ParityResult:
    m = extract_metrics(decompile, source, asm, wrapper_prefixes, callee_count)
    signals = run_signals(m, source)
    objective = objective_verify(m, source, call_tol, cf_tol)

    has_red = any(s.severity == RED for s in signals)
    blocking = has_red or objective.verdict == FAIL
    if blocking:
        verdict = FAIL
    elif objective.verdict == PASS:
        verdict = PASS
    else:
        verdict = UNKNOWN
    return ParityResult(verdict, blocking, signals, objective, m)


# --- rendering -------------------------------------------------------------

_SEV_ORDER = {RED: 0, YELLOW: 1, INFO: 2}
_SEV_GLYPH = {RED: "[RED ]", YELLOW: "[YEL ]", INFO: "[INFO]"}


def render_human(res: ParityResult) -> str:
    lines: list[str] = []
    lines.append(f"parity verdict: {res.verdict}"
                 + ("  (BLOCKING)" if res.blocking else ""))
    lines.append("")
    if res.signals:
        lines.append("signals:")
        for s in sorted(res.signals, key=lambda x: _SEV_ORDER[x.severity]):
            lines.append(f"  {_SEV_GLYPH[s.severity]} {s.code}: {s.message}")
    else:
        lines.append("signals: none")
    lines.append("")
    ov = res.objective
    lines.append(f"objective: {ov.verdict} ({ov.checks_run} check(s) run)")
    for r in ov.reasons:
        lines.append(f"  - {r}")
    lines.append("")
    mt = res.metrics
    lines.append(
        "metrics: "
        f"source[{mt.source_lines} lines, {mt.source_calls} calls, {mt.source_cf} cf] "
        f"binary[{mt.callee_count} callees, {mt.decompile_cf} cf"
        + (f", {mt.asm_instr} asm instr, {mt.asm_calls} asm calls" if mt.asm_present else "")
        + "]"
    )
    return "\n".join(lines)


# --- selftest --------------------------------------------------------------


def _selftest() -> int:
    failures = 0

    def check(name, cond):
        nonlocal failures
        status = "ok" if cond else "FAIL"
        if not cond:
            failures += 1
        print(f"  [{status}] {name}")

    # Case 1: an 80+ instruction function reduced to a 3-line stub → RED/FAIL.
    asm_big = "\n".join(f"00401{ i:03x}  8b 45 fc  mov eax, dword ptr [ebp-4]"
                        for i in range(90))
    dc = """
    int process(int *a, int n) {
      int i; int sum = 0;
      for (i = 0; i < n; i = i + 1) {
        if (a[i] > 0) { sum = sum + foo(a[i]); }
        else { sum = sum + bar(a[i]); }
      }
      return baz(sum);
    }
    """
    stub = "fn process(a: &[i32]) -> i32 { 0 }"
    r = evaluate(dc, stub, asm_big, callee_count=3)
    check("large-asm-tiny-source is blocking", r.verdict == FAIL and r.blocking)
    check("large_asm_tiny_source signal present",
          any(s.code == "large_asm_tiny_source" for s in r.signals))
    check("objective fails on cf/call gap", r.objective.verdict == FAIL)

    # Case 2: a faithful reconstruction → PASS, no RED.
    good = """
    fn process(a: &[i32], n: usize) -> i32 {
        let mut sum = 0;
        for i in 0..n {
            if a[i] > 0 { sum += foo(a[i]); }
            else { sum += bar(a[i]); }
        }
        baz(sum)
    }
    """
    r = evaluate(dc, good, None, callee_count=3)
    check("faithful recon passes", r.verdict == PASS)
    check("faithful recon has no RED",
          not any(s.severity == RED for s in r.signals))

    # Case 3: explicit stub marker → RED.
    marked = """
    fn process(a: &[i32], n: usize) -> i32 {
        // real body pending
        todo!("reverse this properly")
    }
    """
    r = evaluate(dc, marked, None, callee_count=3)
    check("stub marker is blocking", r.blocking)
    check("stub_markers signal present",
          any(s.code == "stub_markers" for s in r.signals))

    # Case 4: empty inputs → objective UNKNOWN, not a false PASS.
    r = evaluate("", "fn f() { g(); h(); }", None)
    check("no reference data yields UNKNOWN, not PASS", r.verdict != PASS)

    # Case 5: FP in binary but not in source → YELLOW fp_sensitivity.
    dc_fp = "float dist(float *v) { return v[0] * v[0] + v[1] * v[1]; }"
    asm_fp = "00401000 f3 0f10 45 f8  movss xmm0, dword ptr [ebp-8]\n" \
             "00401008 f3 0f59 c0     mulss xmm0, xmm0"
    src_nofp = "fn dist(v: &Vec3) -> i32 { compute(v) }"
    r = evaluate(dc_fp, src_nofp, asm_fp, callee_count=1)
    check("fp_sensitivity fires",
          any(s.code == "fp_sensitivity" for s in r.signals))

    print()
    if failures:
        print(f"selftest: {failures} FAILURE(S)")
        return 1
    print("selftest: all passed")
    return 0


# --- cli -------------------------------------------------------------------


def _read(path: str | None) -> str | None:
    if path is None:
        return None
    return Path(path).read_text(encoding="utf-8", errors="replace")


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(
        description="LLM-free structural parity check for RE'd functions.",
    )
    p.add_argument("--decompile", help="Ghidra decompiled C pseudocode file")
    p.add_argument("--source", help="reconstructed source / notes file")
    p.add_argument("--asm", help="optional disassembly listing file")
    p.add_argument("--callee-count", type=int, default=None,
                   help="authoritative callee count (e.g. from Ghidra call graph)")
    p.add_argument("--wrapper-prefix", action="append", default=[],
                   help="identifier prefix treated as a framework/wrapper call "
                        "(repeatable); default none")
    p.add_argument("--stub-marker", action="append", default=[],
                   help="extra stub marker string to flag RED (repeatable)")
    p.add_argument("--call-tol", type=int, default=3,
                   help="objective call-gap tolerance (default 3)")
    p.add_argument("--cf-tol", type=int, default=2,
                   help="objective control-flow-gap tolerance (default 2)")
    p.add_argument("--json", action="store_true", help="emit machine-readable JSON")
    p.add_argument("--selftest", action="store_true",
                   help="run embedded fixtures and exit")
    args = p.parse_args(argv)

    if args.selftest:
        return _selftest()

    if not args.decompile or not args.source:
        p.error("--decompile and --source are required (or use --selftest)")

    global _stub_markers_active
    _stub_markers_active = list(_DEFAULT_STUB_MARKERS) + list(args.stub_marker)

    decompile = _read(args.decompile) or ""
    source = _read(args.source) or ""
    asm = _read(args.asm)

    res = evaluate(
        decompile=decompile,
        source=source,
        asm=asm,
        wrapper_prefixes=tuple(args.wrapper_prefix),
        callee_count=args.callee_count,
        call_tol=args.call_tol,
        cf_tol=args.cf_tol,
    )

    if args.json:
        print(json.dumps(res.as_dict(), indent=2))
    else:
        print(render_human(res))

    return 1 if res.blocking else 0


if __name__ == "__main__":
    sys.exit(main())
