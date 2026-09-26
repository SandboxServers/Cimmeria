#!/usr/bin/env python3
"""Layering guard for the cimmeria-services crate split.

Builds the *production* module graph of ``crates/services/src``, maps every
module to the crate it is planned to move into (``crate-map.toml``), and
reports each module edge that the planned crate DAG does not allow:

* ``upward``  - the edge points from a lower crate to a higher one (the
  target crate depends, directly or not, on the source crate);
* ``no-path`` - neither crate depends on the other.

Every such edge must be listed in ``allowlist.txt``. The check fails when a
violation is missing from the allowlist, and also when an allowlisted edge
no longer exists, so the allowlist can only shrink. See README.md.

The graph is built from source text, not from rustc, so it is deliberately
careful about what counts as an edge (plan section 0):

* comments, doc comments and string/char literals are stripped first, so
  intra-doc links and log messages are never edges;
* every item carrying ``#[cfg(test)]`` (or ``#[cfg(all(test, ...))]``) is
  removed on its own, wherever it sits in the file - a test module in the
  middle of a file no longer hides the production code after it, and a
  ``#[cfg(test)] mod x;`` declaration keeps ``x`` out of the graph;
* ``pub(in crate::...)`` visibility is not an edge;
* ``use a::{b, c::{d, e}}`` groups are expanded, ``self``/``super``/child
  module paths are resolved relative to the module they appear in, and names
  are followed through ``use``/``pub use`` re-exports (including globs) to the
  module that defines them;
* importing a module is not an edge by itself; using an item through it is.

Usage:
    python tools/layering/check.py              # the CI check
    python tools/layering/check.py --list       # current violations, allowlist format
    python tools/layering/check.py --prune      # drop stale allowlist lines (never adds)
    python tools/layering/check.py --modules    # every production module, its crate and size
    python tools/layering/check.py --edges MOD  # outgoing and incoming edges of one module
"""

from __future__ import annotations

import argparse
import bisect
import re
import sys
from collections import defaultdict
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # pragma: no cover - Python < 3.11
    sys.exit("layering: Python 3.11+ is required (tomllib)")

HERE = Path(__file__).resolve().parent
REPO = HERE.parent.parent
SRC = REPO / "crates" / "services" / "src"
CRATE_MAP = HERE / "crate-map.toml"
ALLOWLIST = HERE / "allowlist.txt"

IDENT = r"[A-Za-z_][A-Za-z0-9_]*"

# --------------------------------------------------------------------------
# Lexing: blank out comments and literals, keeping offsets and newlines.
# --------------------------------------------------------------------------


def _blank(s: str) -> str:
    return "".join("\n" if ch == "\n" else " " for ch in s)


def _is_ident_char(ch: str) -> bool:
    return ch.isalnum() or ch == "_"


_RAW_STR = re.compile(r'(?:b|c)?r(#*)"')


def lex(src: str) -> tuple[str, dict[int, str]]:
    """Return ``(clean, strings)``.

    ``clean`` has the same length and line structure as ``src``; comments are
    spaces, string and char literals keep their quotes but their contents are
    spaces. ``strings`` maps the offset of each string's opening quote to its
    raw contents (for ``#[path = "..."]``).
    """
    out: list[str] = []
    strings: dict[int, str] = {}
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if c == "/" and nxt == "/":
            j = src.find("\n", i)
            j = n if j == -1 else j
            out.append(" " * (j - i))
            i = j
            continue
        if c == "/" and nxt == "*":
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            out.append(_blank(src[i:j]))
            i = j
            continue
        prev = src[i - 1] if i else ""
        if c in "bcr" and not _is_ident_char(prev):
            m = _RAW_STR.match(src, i)
            if m:
                close = '"' + m.group(1)
                end = src.find(close, m.end())
                end = n if end == -1 else end
                quote = m.end() - 1
                strings[quote] = src[m.end() : end]
                out.append(src[i : m.end()] + _blank(src[m.end() : end]) + close)
                i = end + len(close)
                continue
            if c in "bc" and nxt == '"':
                out.append(c)
                i += 1
                continue
        if c == '"':
            j = i + 1
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            strings[i] = src[i + 1 : j]
            out.append('"' + _blank(src[i + 1 : j]) + '"')
            i = j + 1
            continue
        if c == "'":
            # Char literal ('x', '\n', '\u{..}') or a lifetime / label ('a).
            if nxt == "\\":
                j = src.find("'", i + 3)
                j = n if j == -1 else j
                out.append("'" + _blank(src[i + 1 : j]) + "'")
                i = j + 1
                continue
            if i + 2 < n and src[i + 2] == "'":
                out.append("' '")
                i += 3
                continue
        out.append(c)
        i += 1
    return "".join(out), strings


# --------------------------------------------------------------------------
# Removing #[cfg(test)] items.
# --------------------------------------------------------------------------

_CFG_TEST = re.compile(r"#\s*\[\s*cfg\s*\(\s*(?:test\s*|all\s*\(\s*test\b[^\]]*)\)\s*\]")
_OPEN = "([{"
_CLOSE = ")]}"
_SEMI_ITEMS = re.compile(
    r"(?:pub\s*(?:\([^)]*\))?\s*)?(?:use|const|static|type|let|extern\s+crate)\b"
)


def _skip_ws(text: str, i: int) -> int:
    while i < len(text) and text[i].isspace():
        i += 1
    return i


def _match_bracket(text: str, i: int) -> int:
    """Index just past the bracket group opening at ``text[i]``."""
    depth = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if ch in _OPEN:
            depth += 1
        elif ch in _CLOSE:
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return n


def _item_end(text: str, i: int) -> int:
    """End of the item (or statement, field, arm) starting at ``i``."""
    i = _skip_ws(text, i)
    while text.startswith("#", i):  # further attributes
        j = _skip_ws(text, i + 1)
        if j < len(text) and text[j] == "[":
            i = _skip_ws(text, _match_bracket(text, j))
        else:
            break
    semi_only = bool(_SEMI_ITEMS.match(text, i))
    depth = 0
    n = len(text)
    j = i
    while j < n:
        ch = text[j]
        if ch in _OPEN:
            depth += 1
        elif ch in _CLOSE:
            if depth == 0:
                return j  # end of the enclosing block
            depth -= 1
            if depth == 0 and ch == "}" and not semi_only:
                return j + 1
        elif depth == 0 and ch in ";,":
            return j + 1
        j += 1
    return n


def strip_cfg_test(text: str) -> str:
    """Blank every ``#[cfg(test)]`` item, wherever it sits."""
    while True:
        m = _CFG_TEST.search(text)
        if not m:
            return text
        end = _item_end(text, m.end())
        text = text[: m.start()] + _blank(text[m.start() : end]) + text[end:]


# --------------------------------------------------------------------------
# Module tree.
# --------------------------------------------------------------------------


class Module:
    def __init__(self, path: tuple[str, ...], file: Path, text: str, base: int, line_starts):
        self.path = path
        self.file = file
        self.text = text  # this module's own code, inline children blanked
        self.base = base  # offset of `text` inside the file
        self.line_starts = line_starts
        self.children: set[str] = set()
        self.defs: set[str] = set()
        self.imports: dict[str, list[str]] = {}
        self.globs: list[list[str]] = []
        self.uses: list[tuple[int, list[str], bool, bool]] = []  # (offset, segs, glob, pub)
        self.macro_exports: set[str] = set()
        self.code = ""  # `text` with `use` statements blanked, for the path scan

    @property
    def name(self) -> str:
        return "::".join(self.path) or "<root>"

    def line_of(self, offset: int) -> int:
        return bisect.bisect_right(self.line_starts, self.base + offset)

    def lines(self) -> int:
        return sum(1 for ln in self.text.split("\n") if ln.strip())


_MOD_DECL = re.compile(rf"\bmod\s+({IDENT})\s*([;{{])")
_PATH_ATTR = re.compile(r'#\s*\[\s*path\s*=\s*(")')


class Crate:
    def __init__(self, root: Path):
        self.root = root
        self.modules: dict[tuple[str, ...], Module] = {}
        self.macro_home: dict[str, tuple[str, ...]] = {}
        self._load_file(root / "lib.rs", (), is_mod_rs=True)
        for m in self.modules.values():
            _scan_names(m)
            for name in m.macro_exports:
                self.macro_home[name] = m.path

    def _load_file(self, file: Path, path: tuple[str, ...], is_mod_rs: bool) -> None:
        raw = file.read_text(encoding="utf-8")
        clean, strings = lex(raw)
        clean = strip_cfg_test(clean)
        line_starts = [0] + [i + 1 for i, ch in enumerate(raw) if ch == "\n"]
        child_dir = file.parent if is_mod_rs else file.parent / file.stem
        self._load_text(file, path, clean, 0, line_starts, child_dir, strings)

    def _load_text(self, file, path, text, base, line_starts, child_dir, strings) -> None:
        mod = Module(path, file, text, base, line_starts)
        self.modules[path] = mod
        body = list(text)
        path_attrs = {}
        for m in _PATH_ATTR.finditer(text):
            end = _match_bracket(text, text.rfind("[", 0, m.start(1)))
            nxt = _MOD_DECL.search(text, end)
            if nxt:
                path_attrs[nxt.start()] = strings.get(base + m.start(1), "")
        pos = 0
        while True:
            m = _MOD_DECL.search(text, pos)
            if not m:
                break
            name = m.group(1)
            mod.children.add(name)
            if m.group(2) == ";":
                explicit = path_attrs.get(m.start())
                if explicit:
                    target = file.parent / explicit
                    self._load_file(target, path + (name,), target.name == "mod.rs")
                else:
                    flat = child_dir / f"{name}.rs"
                    nested = child_dir / name / "mod.rs"
                    if flat.exists():
                        self._load_file(flat, path + (name,), False)
                    elif nested.exists():
                        self._load_file(nested, path + (name,), True)
                    else:
                        sys.exit(f"layering: {file}: `mod {name};` has no file")
                pos = m.end()
            else:
                close = _match_bracket(text, m.end() - 1)
                inner = text[m.end() : close - 1]
                for k in range(m.end(), close - 1):
                    if body[k] != "\n":
                        body[k] = " "
                self._load_text(
                    file, path + (name,), inner, base + m.end(), line_starts,
                    child_dir / name, strings,
                )
                pos = close
        mod.text = "".join(body)


# --------------------------------------------------------------------------
# Names and `use` trees.
# --------------------------------------------------------------------------

_DEF = re.compile(
    rf"\b(?:fn|struct|enum|union|trait|type|static|mod)\s+({IDENT})"
    rf"|\bconst\s+(?!fn\b)({IDENT})|\bmacro_rules!\s*({IDENT})"
)
_USE = re.compile(r"(?<![A-Za-z0-9_$])use\s+(?=[A-Za-z_$:{])")
_VIS_IN = re.compile(r"\bpub\s*\(\s*in\b[^)]*\)")
_PUB_BEFORE = re.compile(r"pub\s*(?:\([^)]*\))?\s*$")
_TOKEN = re.compile(rf"::|\$crate|{IDENT}|[{{}},*;]")


def _depth_map(text: str) -> list[int]:
    depth, out = 0, []
    for ch in text:
        if ch == "}":
            depth -= 1
        out.append(depth)
        if ch == "{":
            depth += 1
    return out


def _parse_use_tree(tokens: list[str], i: int, prefix: list[str], out: list) -> int:
    """Parse one use tree at ``tokens[i]``; append ``(segs, alias, glob)``."""
    segs = list(prefix)
    if i < len(tokens) and tokens[i] == "::":
        if not segs:
            segs.append("::")  # `use ::std::...`: an extern path
        i += 1
    while i < len(tokens):
        t = tokens[i]
        if t == "{":
            i += 1
            while i < len(tokens) and tokens[i] != "}":
                i = _parse_use_tree(tokens, i, segs, out)
                if i < len(tokens) and tokens[i] == ",":
                    i += 1
            return i + 1
        if t == "*":
            out.append((segs, None, True))
            return i + 1
        if t in (",", "}", ";"):
            return i
        if t == "::":
            i += 1
            continue
        # an identifier
        nxt = tokens[i + 1] if i + 1 < len(tokens) else ""
        if nxt == "::":
            segs = segs + [t]
            i += 2
            continue
        alias = t
        leaf = segs + [t]
        if t == "self":
            leaf = segs
            alias = segs[-1] if segs else "self"
        i += 1
        if i < len(tokens) and tokens[i] == "as":
            alias = tokens[i + 1]
            i += 2
        out.append((leaf, alias, False))
        return i
    return i


def _scan_names(mod: Module) -> None:
    text = _VIS_IN.sub(lambda m: _blank(m.group(0)), mod.text)
    depth = _depth_map(text)
    for m in _DEF.finditer(text):
        if depth[m.start()] == 0:
            mod.defs.add(m.group(1) or m.group(2) or m.group(3))
    for m in re.finditer(rf"#\s*\[\s*macro_export\s*\]\s*macro_rules!\s*({IDENT})", text):
        mod.macro_exports.add(m.group(1))
    spans = []
    for m in _USE.finditer(text):
        end = text.find(";", m.end())
        if end == -1:
            continue
        is_pub = bool(_PUB_BEFORE.search(text[max(0, m.start() - 40) : m.start()]))
        tokens = _TOKEN.findall(text[m.end() : end])
        leaves: list = []
        i = 0
        while i < len(tokens):
            i = _parse_use_tree(tokens, i, [], leaves)
            if i < len(tokens) and tokens[i] == ",":
                i += 1
            elif i < len(tokens):
                i += 1
        for segs, alias, glob in leaves:
            if not segs or segs[0] == "::":
                continue
            if glob:
                mod.globs.append(segs)
            elif alias and alias != "_":
                mod.imports.setdefault(alias, segs)
            mod.uses.append((m.start(), segs, glob, is_pub))
        spans.append((m.start(), end + 1))
    # Code with `use` statements and restricted visibility blanked, for the
    # expression-path scan.
    chars = list(text)
    for a, b in spans:
        for k in range(a, b):
            if chars[k] != "\n":
                chars[k] = " "
    mod.code = "".join(chars)


# --------------------------------------------------------------------------
# Resolution.
# --------------------------------------------------------------------------


class Resolver:
    def __init__(self, crate: Crate):
        self.c = crate

    def to_abs(self, mod: Module, segs: list[str], depth: int = 0):
        """Absolute segments for a path written in ``mod``, or None if external."""
        if not segs or depth > 20:
            return None
        head = segs[0]
        if head in ("crate", "$crate"):
            return list(segs[1:])
        if head == "self":
            return list(mod.path) + list(segs[1:])
        if head == "super":
            base = list(mod.path)
            i = 0
            while i < len(segs) and segs[i] == "super":
                if not base:
                    return None
                base.pop()
                i += 1
            return base + list(segs[i:])
        if head in mod.children or head in mod.defs:
            return list(mod.path) + list(segs)
        if head in mod.imports:
            target = self.to_abs(mod, mod.imports[head], depth + 1)
            return None if target is None else target + list(segs[1:])
        for g in mod.globs:
            gabs = self.to_abs(mod, g, depth + 1)
            if gabs is None:
                continue
            gmod = self.c.modules.get(tuple(gabs))
            if gmod and self._has_name(gmod, head, set()):
                return list(gmod.path) + list(segs)
        return None

    def _has_name(self, mod: Module, name: str, seen: set) -> bool:
        if mod.path in seen:
            return False
        seen.add(mod.path)
        if name in mod.children or name in mod.defs or name in mod.imports:
            return True
        for g in mod.globs:
            gabs = self.to_abs(mod, g)
            gmod = self.c.modules.get(tuple(gabs)) if gabs is not None else None
            if gmod and self._has_name(gmod, name, seen):
                return True
        return False

    def target(self, abs_segs, seen=None):
        """``(module_path, is_module)`` that defines what ``abs_segs`` names."""
        seen = seen if seen is not None else set()
        if len(abs_segs) == 1 and abs_segs[0] in self.c.macro_home:
            return self.c.macro_home[abs_segs[0]], False
        cur: tuple[str, ...] = ()
        for i, s in enumerate(abs_segs):
            nxt = cur + (s,)
            if nxt in self.c.modules:
                cur = nxt
                continue
            return self._item(self.c.modules[cur], s, abs_segs[i + 1 :], seen), False
        return cur, True

    def _item(self, mod: Module, name: str, rest, seen):
        key = (mod.path, name)
        if key in seen:
            return mod.path
        seen.add(key)
        if name in mod.defs:
            return mod.path
        if name in mod.imports:
            abs_segs = self.to_abs(mod, mod.imports[name])
            if abs_segs is None:
                return None  # re-exported from another crate
            return self.target(abs_segs + list(rest), seen)[0]
        for g in mod.globs:
            gabs = self.to_abs(mod, g)
            gmod = self.c.modules.get(tuple(gabs)) if gabs is not None else None
            if gmod and self._has_name(gmod, name, set()):
                return self._item(gmod, name, rest, seen)
        # The name is not defined here or behind a glob of this crate, but the
        # module glob-imports another crate (`pub use constants::*` where
        # `constants` now lives in cimmeria-wire): the name comes from there.
        if any(self.to_abs(mod, g) is None for g in mod.globs):
            return None
        return mod.path


_EXPR_PATH = re.compile(
    rf"(?<![A-Za-z0-9_:$])(\$?{IDENT})((?:\s*::\s*{IDENT})+)"
)


def build_edges(crate: Crate):
    """``{(src, dst): (file, line)}`` for every cross-module production edge."""
    r = Resolver(crate)
    edges: dict[tuple, tuple] = {}

    def add(mod: Module, dst, offset: int) -> None:
        if dst is None or dst == mod.path:
            return
        key = (mod.path, dst)
        if key not in edges:
            rel = mod.file.relative_to(REPO).as_posix()
            edges[key] = (rel, mod.line_of(offset))

    for mod in crate.modules.values():
        for offset, segs, glob, is_pub in mod.uses:
            abs_segs = r.to_abs(mod, segs)
            if abs_segs is None:
                continue
            dst, is_module = r.target(abs_segs)
            if glob or (is_module and is_pub):
                add(mod, dst, offset)
            elif not is_module:
                add(mod, dst, offset)
            # A private import of a module is not an edge: the items used
            # through it are, and the expression scan below finds them.
        for m in _EXPR_PATH.finditer(mod.code):
            segs = [m.group(1)] + [s.strip() for s in m.group(2).split("::")[1:]]
            abs_segs = r.to_abs(mod, segs)
            if abs_segs is None:
                continue
            dst, _ = r.target(abs_segs)
            add(mod, dst, m.start())
    return edges


# --------------------------------------------------------------------------
# Crate map and the DAG.
# --------------------------------------------------------------------------


class CrateMap:
    def __init__(self, path: Path):
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        self.deps: dict[str, list[str]] = data["crates"]
        self.prefix: dict[str, str] = data.get("prefix", {})
        self.exact: dict[str, str] = data.get("exact", {})
        errors = []
        for table in (self.prefix, self.exact):
            for mod, crate in table.items():
                if crate not in self.deps:
                    errors.append(f"module `{mod}` maps to unknown crate `{crate}`")
        for crate, deps in self.deps.items():
            for d in deps:
                if d not in self.deps:
                    errors.append(f"crate `{crate}` depends on unknown crate `{d}`")
        if errors:
            sys.exit("layering: crate-map.toml:\n  " + "\n  ".join(errors))
        self.reach = {c: self._closure(c, []) for c in self.deps}

    def _closure(self, crate: str, stack: list[str]) -> set[str]:
        if crate in stack:
            sys.exit(f"layering: crate-map.toml: dependency cycle {' -> '.join(stack + [crate])}")
        out: set[str] = set()
        for d in self.deps[crate]:
            out.add(d)
            out |= self._closure(d, stack + [crate])
        return out

    def crate_of(self, path: tuple[str, ...]):
        name = "::".join(path)
        if name in self.exact:
            return self.exact[name]
        for i in range(len(path), 0, -1):
            key = "::".join(path[:i])
            if key in self.prefix:
                return self.prefix[key]
        return None

    def kind(self, src: str, dst: str):
        """None if the edge is allowed, else ``upward`` or ``no-path``."""
        if src == dst or dst in self.reach[src]:
            return None
        return "upward" if src in self.reach[dst] else "no-path"


def fmt_mod(path) -> str:
    return "::".join(path) or "<root>"


def compute(crate: Crate, cmap: CrateMap):
    unmapped = sorted(fmt_mod(p) for p in crate.modules if cmap.crate_of(p) is None)
    if unmapped:
        sys.exit(
            "layering: production modules with no crate in crate-map.toml "
            "(add a [prefix] or [exact] row):\n  " + "\n  ".join(unmapped)
        )
    edges = build_edges(crate)
    violations = {}
    for (src, dst), where in edges.items():
        a, b = cmap.crate_of(src), cmap.crate_of(dst)
        kind = cmap.kind(a, b)
        if kind:
            violations[(fmt_mod(src), fmt_mod(dst))] = (a, b, kind, where)
    return edges, violations


def read_allowlist(path: Path) -> set[tuple[str, str]]:
    out = set()
    if not path.exists():
        return out
    for n, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if " -> " not in line:
            sys.exit(f"layering: {path.name}:{n}: expected `<module> -> <module>`")
        a, b = (s.strip() for s in line.split(" -> ", 1))
        out.add((a, b))
    return out


def format_violation(key, info) -> str:
    a, b, kind, (file, line) = info
    return f"{key[0]} -> {key[1]}  # {kind}: {a} -> {b} ({file}:{line})"


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--list", action="store_true", help="print current violations in allowlist format")
    ap.add_argument("--prune", action="store_true", help="remove allowlist lines whose edge is gone")
    ap.add_argument("--modules", action="store_true", help="print every production module with its crate")
    ap.add_argument("--edges", metavar="MODULE", help="print the edges of one module")
    args = ap.parse_args()

    crate = Crate(SRC)
    cmap = CrateMap(CRATE_MAP)

    if args.modules:
        total = 0
        for p in sorted(crate.modules):
            m = crate.modules[p]
            total += m.lines()
            print(f"{m.lines():6}  {cmap.crate_of(p) or '??':28} {fmt_mod(p)}")
        print(f"{total:6}  total non-blank production lines, {len(crate.modules)} modules")
        return 0

    edges, violations = compute(crate, cmap)

    if args.edges:
        want = tuple(args.edges.split("::")) if args.edges != "<root>" else ()
        for (src, dst), (file, line) in sorted(edges.items()):
            if src == want:
                print(f"-> {fmt_mod(dst):60} {cmap.crate_of(dst):28} {file}:{line}")
        for (src, dst), (file, line) in sorted(edges.items()):
            if dst == want:
                print(f"<- {fmt_mod(src):60} {cmap.crate_of(src):28} {file}:{line}")
        return 0

    if args.list:
        for key in sorted(violations):
            print(format_violation(key, violations[key]))
        return 0

    allowed = read_allowlist(ALLOWLIST)
    current = set(violations)
    new = sorted(current - allowed)
    stale = sorted(allowed - current)

    if args.prune:
        if not stale:
            print("layering: nothing to prune")
            return 0
        drop = set(stale)
        kept = []
        for raw in ALLOWLIST.read_text(encoding="utf-8").splitlines():
            line = raw.split("#", 1)[0].strip()
            if line and tuple(s.strip() for s in line.split(" -> ", 1)) in drop:
                continue
            kept.append(raw)
        ALLOWLIST.write_text("\n".join(kept) + "\n", encoding="utf-8", newline="\n")
        print(f"layering: pruned {len(stale)} stale allowlist line(s)")
        return 0

    ok = True
    if new:
        ok = False
        print(f"layering: {len(new)} module edge(s) break the planned crate DAG "
              "(docs/architecture/services-crate-split.md):")
        for key in new:
            print("  " + format_violation(key, violations[key]))
        print("Move the item down, invert the call, or map the module to another crate "
              "in tools/layering/crate-map.toml. The allowlist only shrinks.")
    if stale:
        ok = False
        print(f"layering: {len(stale)} allowlisted edge(s) no longer exist; remove them "
              "(`python tools/layering/check.py --prune`):")
        for a, b in stale:
            print(f"  {a} -> {b}")
    if ok:
        print(f"layering: OK - {len(crate.modules)} modules, {len(edges)} edges, "
              f"{len(current)} allowlisted violation(s) remaining")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
