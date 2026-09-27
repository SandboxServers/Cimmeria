#!/usr/bin/env python3
"""Regenerate every generated number and block in the repo's Markdown.

A PR that bumps a hand-maintained count in a shared doc conflicts with every other
open PR that bumps the same count. So those numbers are generated: this script
owns them, a workflow reruns it on `main` after every merge, and PRs leave them
alone.

It only rewrites text between explicit markers:

    <!-- gen:NAME -->...<!-- /gen:NAME -->          inline, e.g. a number in a sentence
    <!-- gen:NAME ARG ARG -->...<!-- /gen:NAME -->  generators that take arguments

A generator whose output spans several lines (a table) is written as a block: the
markers sit on their own lines with a blank line on each side of the content.
Everything outside the markers is left byte for byte, including CRLF line endings.
The crate graph keeps its own `<!-- crate-graph:begin/end -->` markers and is
rendered by tools/crate-graph/crate_graph.py.

Generators (NAME [ARGS]):
    tests-total, tests-files, tests-ci-gated, tests-live-db
                        workspace test counts, from tools/extract_tests.py
    tests-threshold     5% of tests-total: the "a PR that adds or removes this many
                        tests updates the inventory" threshold
    tests-totals        the canonical totals table (docs/testing/inventory/README.md)
    tests-by-crate      the per-crate table (same file)
    re-findings-count   *.md in docs/reverse-engineering/findings/, minus README.md
    docs-md-count       *.md anywhere under docs/
    section-table-rows  data rows in the tables between the marker and the next heading
    gap-count STATUSES  sum of the Summary Completion Matrix columns in
                        docs/gap-analysis.md; STATUSES is `total` or e.g. `CW+NT+IM`
    gap-pct STATUSES [DECIMALS]
                        that sum as a percentage of the total (default 1 decimal)
    gap-systems         number of system rows in the matrix

Usage:
    python tools/docs-gen/regen.py                # rewrite stale blocks (same as --write)
    python tools/docs-gen/regen.py --check        # exit 1 and list stale blocks
    python tools/docs-gen/regen.py --skip-crate-graph   # no cargo needed
"""
from __future__ import annotations

import argparse
import importlib.util
import os
import pathlib
import re
import sys
from dataclasses import dataclass, field
from typing import Callable

ROOT = pathlib.Path(__file__).resolve().parents[2]

MARKER_RE = re.compile(
    r"<!-- gen:(?P<name>[a-z0-9][a-z0-9-]*)(?P<args>(?: [^>]*?)?) -->"
    r"(?P<body>.*?)"
    r"<!-- /gen:(?P=name) -->",
    re.S,
)
#: Directories never searched for Markdown. `.claude` holds other worktrees.
SKIP_DIRS = frozenset({".git", ".claude", "target", "node_modules", "external", "game"})
#: Share of the workspace test count at which a PR must update the inventory.
INVENTORY_THRESHOLD = 0.05
GAP_STATUSES = ("CW", "NT", "IM", "KM", "NU")


def _load(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module  # dataclasses resolve annotations through sys.modules
    spec.loader.exec_module(module)
    return module


# ── sources ──────────────────────────────────────────────────────────────────


@dataclass
class TestStats:
    total: int
    files: int
    ci_gated: int
    live_db: int
    #: (member, package, tests, files, live-DB, CI-gated, catalogue file or "")
    crates: list[tuple[str, str, int, int, int, bool, str]] = field(default_factory=list)


@dataclass
class GapMatrix:
    #: one dict per system row: {"System": str, "Total": int, "CW": int, ...}
    rows: list[dict]

    def count(self, statuses: str) -> int:
        if statuses == "total":
            return sum(r["Total"] for r in self.rows)
        return sum(r[s] for r in self.rows for s in split_statuses(statuses))


def split_statuses(spec: str) -> list[str]:
    parts = spec.split("+")
    for p in parts:
        if p not in GAP_STATUSES:
            raise ValueError(f"unknown gap status {p!r} (expected total or {'+'.join(GAP_STATUSES)})")
    return parts


def parse_gap_matrix(text: str) -> GapMatrix:
    """Read the system rows of the Summary Completion Matrix, skipping the TOTALS row."""
    lines = text.replace("\r\n", "\n").split("\n")
    try:
        start = next(i for i, l in enumerate(lines) if l.startswith("## Summary Completion Matrix"))
    except StopIteration:
        raise ValueError("docs/gap-analysis.md: no '## Summary Completion Matrix' heading") from None
    header = None
    rows: list[dict] = []
    for line in lines[start + 1 :]:
        if line.startswith("#"):
            break
        if not line.startswith("|"):
            if header is not None and rows:
                break
            continue
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if header is None:
            header = cells
            continue
        if set("".join(cells)) <= set("-: "):
            continue  # separator row
        row = dict(zip(header, cells))
        if "TOTALS" in row.get("System", ""):
            continue
        parsed = {"System": row["System"]}
        for col in ("Total",) + GAP_STATUSES:
            parsed[col] = int(re.sub(r"[^0-9]", "", row[col]))
        rows.append(parsed)
    if not rows:
        raise ValueError("docs/gap-analysis.md: Summary Completion Matrix has no rows")
    return GapMatrix(rows)


def iter_markdown(root: pathlib.Path):
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = sorted(d for d in dirnames if d not in SKIP_DIRS)
        for f in sorted(filenames):
            if f.endswith(".md"):
                yield pathlib.Path(dirpath) / f


class Context:
    """Lazily computed inputs; each source is read at most once per run."""

    def __init__(self, root: pathlib.Path):
        self.root = root
        self._tests: TestStats | None = None
        self._gap: GapMatrix | None = None

    @property
    def tests(self) -> TestStats:
        if self._tests is None:
            et = _load("extract_tests", self.root / "tools" / "extract_tests.py")
            crates = et.collect()
            t = et.totals(crates)
            inventory = self.root / "docs" / "testing" / "inventory"
            self._tests = TestStats(
                total=t["tests"],
                files=t["files"],
                ci_gated=t["ci_gated_tests"],
                live_db=t["live_db_tests"],
                crates=[
                    (
                        c.member,
                        c.package,
                        len(c.tests),
                        len(c.files_with_tests),
                        sum(1 for x in c.tests if x.live_db),
                        c.ci_gated,
                        c.inventory if (inventory / c.inventory).is_file() else "",
                    )
                    for c in crates
                    if c.tests
                ],
            )
        return self._tests

    @tests.setter
    def tests(self, value: TestStats) -> None:
        self._tests = value

    @property
    def gap(self) -> GapMatrix:
        if self._gap is None:
            path = self.root / "docs" / "gap-analysis.md"
            self._gap = parse_gap_matrix(path.read_bytes().decode("utf-8"))
        return self._gap

    @gap.setter
    def gap(self, value: GapMatrix) -> None:
        self._gap = value

    def count_md(self, rel: str, exclude: tuple[str, ...] = ()) -> int:
        base = self.root / rel
        return sum(1 for p in iter_markdown(base) if p.name not in exclude) if base.is_dir() else 0


# ── generators ───────────────────────────────────────────────────────────────


def n(value: int) -> str:
    return f"{value:,}"


def threshold(total: int) -> int:
    return round(total * INVENTORY_THRESHOLD)


def gen_tests_totals(ctx: Context, args: list[str], after: str) -> str:
    t = ctx.tests
    return "\n".join(
        [
            "| Metric | Count |",
            "|---|---:|",
            f"| Tests (`#[test]` / `#[tokio::test]`) | {n(t.total)} |",
            f"| Files with tests | {n(t.files)} |",
            f"| Gated in CI (every crate but CI's exclude list) | {n(t.ci_gated)} |",
            f"| Live-DB tests (`require_db_or_skip!` in the body) | {n(t.live_db)} |",
            f"| Inventory threshold (5% of the tests) | {n(threshold(t.total))} |",
        ]
    )


def gen_tests_by_crate(ctx: Context, args: list[str], after: str) -> str:
    rows = [
        "| Crate | Package | Tests | Files | Live-DB | In CI | Catalogue |",
        "|---|---|---:|---:|---:|---|---|",
    ]
    for member, package, tests, files, live, gated, inv in ctx.tests.crates:
        cat = f"[{inv}]({inv})" if inv else "none"
        rows.append(
            f"| `{member}` | `{package}` | {n(tests)} | {n(files)} | {n(live)} | "
            f"{'yes' if gated else 'no'} | {cat} |"
        )
    return "\n".join(rows)


SEPARATOR_RE = re.compile(r"^\|[\s:|-]+\|?\s*$")


def gen_section_table_rows(ctx: Context, args: list[str], after: str) -> str:
    """Count table data rows from the marker to the next heading."""
    pipe = sep = 0
    for line in after.replace("\r\n", "\n").split("\n"):
        if line.startswith("#"):
            break
        if line.startswith("|"):
            pipe += 1
            if SEPARATOR_RE.match(line):
                sep += 1
    return n(pipe - 2 * sep)  # each table has one header and one separator row


def gen_gap_count(ctx: Context, args: list[str], after: str) -> str:
    return n(ctx.gap.count(args[0] if args else "total"))


def gen_gap_pct(ctx: Context, args: list[str], after: str) -> str:
    if not args:
        raise ValueError("gap-pct needs a STATUSES argument")
    decimals = int(args[1]) if len(args) > 1 else 1
    total = ctx.gap.count("total")
    return f"{100 * ctx.gap.count(args[0]) / total:.{decimals}f}%"


Generator = Callable[[Context, list, str], str]

GENERATORS: dict[str, Generator] = {
    "tests-total": lambda ctx, a, s: n(ctx.tests.total),
    "tests-files": lambda ctx, a, s: n(ctx.tests.files),
    "tests-ci-gated": lambda ctx, a, s: n(ctx.tests.ci_gated),
    "tests-live-db": lambda ctx, a, s: n(ctx.tests.live_db),
    "tests-threshold": lambda ctx, a, s: n(threshold(ctx.tests.total)),
    "tests-totals": gen_tests_totals,
    "tests-by-crate": gen_tests_by_crate,
    "re-findings-count": lambda ctx, a, s: n(
        ctx.count_md("docs/reverse-engineering/findings", exclude=("README.md",))
    ),
    "docs-md-count": lambda ctx, a, s: n(ctx.count_md("docs")),
    "section-table-rows": gen_section_table_rows,
    "gap-count": gen_gap_count,
    "gap-pct": gen_gap_pct,
    "gap-systems": lambda ctx, a, s: n(len(ctx.gap.rows)),
}


# ── splicing ─────────────────────────────────────────────────────────────────


@dataclass
class Change:
    name: str
    old: str
    new: str


def regen_text(text: str, ctx: Context, generators: dict[str, Generator] = GENERATORS) -> tuple[str, list[Change]]:
    """Return `text` with every marker body regenerated, and the bodies that changed.

    Only marker bodies change. A multi-line value is written as a block using the
    file's own newline, so a CRLF file stays CRLF.
    """
    nl = "\r\n" if "\r\n" in text else "\n"
    changes: list[Change] = []

    def replace(m: re.Match) -> str:
        name = m.group("name")
        if name not in generators:
            raise ValueError(f"unknown generator gen:{name}")
        args = m.group("args").split()
        value = generators[name](ctx, args, text[m.end() :])
        if "\n" in value:
            body = nl + nl + value.replace("\n", nl) + nl + nl
        else:
            body = value
        if body != m.group("body"):
            changes.append(Change(name, m.group("body"), body))
        open_tag = text[m.start() : m.start("body")]
        return open_tag + body + f"<!-- /gen:{name} -->"

    return MARKER_RE.sub(replace, text), changes


def regen_files(root: pathlib.Path, ctx: Context, *, write: bool) -> list[tuple[str, Change]]:
    stale: list[tuple[str, Change]] = []
    for path in iter_markdown(root):
        raw = path.read_bytes()
        if b"<!-- gen:" not in raw:
            continue
        text = raw.decode("utf-8")
        rel = path.relative_to(root).as_posix()
        try:
            new, changes = regen_text(text, ctx)
        except ValueError as exc:
            raise SystemExit(f"{rel}: {exc}") from None
        if not changes:
            continue
        stale.extend((rel, c) for c in changes)
        if write:
            path.write_bytes(new.encode("utf-8"))
    return stale


def crate_graph(write: bool) -> list[str]:
    """Regenerate the crate graph; return the stale files."""
    cg = _load("crate_graph", ROOT / "tools" / "crate-graph" / "crate_graph.py")
    block = cg.render()
    stale = []
    for target in cg.TARGETS:
        old, new = cg.splice(target, block)
        if old != new:
            stale.append(target.relative_to(ROOT).as_posix())
            if write:
                target.write_bytes(new.encode("utf-8"))
    return stale


def short(s: str, limit: int = 60) -> str:
    s = s.replace("\r", "").replace("\n", " ").strip()
    return s if len(s) <= limit else s[: limit - 3] + "..."


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    mode = ap.add_mutually_exclusive_group()
    mode.add_argument("--write", action="store_true", help="rewrite stale blocks (the default)")
    mode.add_argument("--check", action="store_true", help="exit 1 if any block is stale; write nothing")
    ap.add_argument("--skip-crate-graph", action="store_true", help="leave the crate graph alone (no cargo)")
    args = ap.parse_args(argv)
    write = not args.check

    stale = regen_files(ROOT, Context(ROOT), write=write)
    graph_stale = [] if args.skip_crate_graph else crate_graph(write)

    verb = "stale" if args.check else "updated"
    for rel, c in stale:
        print(f"{verb}: {rel}  gen:{c.name}  {short(c.old)!r} -> {short(c.new)!r}")
    for rel in graph_stale:
        print(f"{verb}: {rel}  crate-graph")
    if args.check and (stale or graph_stale):
        print("run: python tools/docs-gen/regen.py")
        return 1
    if not stale and not graph_stale:
        print("generated doc blocks are current")
    return 0


if __name__ == "__main__":
    sys.exit(main())
