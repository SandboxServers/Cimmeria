#!/usr/bin/env python3
"""Generate the workspace crate dependency graph (Mermaid) from `cargo metadata`.

The diagram in README.md and crates/README.md lives between these markers:

    <!-- crate-graph:begin -->
    ...generated...
    <!-- crate-graph:end -->

Usage:
    python tools/crate-graph/crate_graph.py            # rewrite both READMEs
    python tools/crate-graph/crate_graph.py --check    # exit 1 if either is stale (CI)
    python tools/crate-graph/crate_graph.py --print    # print the Mermaid block

Rules:
  * Nodes are workspace members; edges are normal and build dependencies on other
    workspace members. Dev-dependencies are left out (test-support is shown as a
    dev-only node instead).
  * Edges implied by a longer path are removed (transitive reduction), so the picture
    shows the layering instead of every direct `use`. The full edge set is available
    with --full.
  * Crates are grouped into layers from tools/crate-graph/groups.toml; crates not
    listed there fall back to name-prefix rules and then to "Other".
"""
from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    tomllib = None

ROOT = pathlib.Path(__file__).resolve().parents[2]
TARGETS = [ROOT / "README.md", ROOT / "crates" / "README.md"]
BEGIN = "<!-- crate-graph:begin -->"
END = "<!-- crate-graph:end -->"


def load_groups() -> tuple[list[tuple[str, str]], dict[str, str], list[tuple[str, str]]]:
    """Return (ordered groups [(id, title)], crate->group, prefix rules [(prefix, group)])."""
    path = ROOT / "tools" / "crate-graph" / "groups.toml"
    if tomllib is None:
        sys.exit("crate_graph.py needs Python 3.11+ (tomllib)")
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    groups = [(g["id"], g["title"]) for g in data["group"]]
    members: dict[str, str] = {}
    prefixes: list[tuple[str, str]] = []
    for g in data["group"]:
        for c in g.get("crates", []):
            members[c] = g["id"]
        for p in g.get("prefixes", []):
            prefixes.append((p, g["id"]))
    return groups, members, prefixes


def metadata() -> dict:
    out = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout
    return json.loads(out)


def short(name: str) -> str:
    return name[len("cimmeria-"):] if name.startswith("cimmeria-") else name


def node_id(name: str) -> str:
    parts = re.split(r"[^A-Za-z0-9]+", short(name))
    return parts[0] + "".join(p[:1].upper() + p[1:] for p in parts[1:])


def build_graph(meta: dict) -> tuple[list[str], dict[str, set[str]], set[str]]:
    members = {p["name"] for p in meta["packages"]}
    edges: dict[str, set[str]] = {m: set() for m in members}
    dev_only: set[str] = set()
    dev_edges: set[str] = set()
    for p in meta["packages"]:
        for d in p["dependencies"]:
            if d["name"] not in members or d["name"] == p["name"]:
                continue
            if d.get("kind") == "dev":
                dev_edges.add(d["name"])
                continue
            edges[p["name"]].add(d["name"])
    # Dev-only: used by other members only as a dev-dependency (e.g. test-support).
    normal_targets = {t for ts in edges.values() for t in ts}
    dev_only = {c for c in dev_edges if c not in normal_targets}
    return sorted(members), edges, dev_only


def transitive_reduction(edges: dict[str, set[str]]) -> dict[str, set[str]]:
    memo: dict[str, set[str]] = {}

    def reach(n: str, stack: frozenset = frozenset()) -> set[str]:
        if n in memo:
            return memo[n]
        if n in stack:  # a cycle would be a bug in the workspace; don't loop forever
            return set()
        r: set[str] = set()
        for m in edges.get(n, ()):
            r.add(m)
            r |= reach(m, stack | {n})
        memo[n] = r
        return r

    reduced: dict[str, set[str]] = {}
    for n, outs in edges.items():
        keep = set()
        for m in outs:
            implied = any(m in reach(o) for o in outs if o != m)
            if not implied:
                keep.add(m)
        reduced[n] = keep
    return reduced


def group_of(name: str, members: dict[str, str], prefixes: list[tuple[str, str]]) -> str:
    if name in members:
        return members[name]
    for p, g in prefixes:
        if name.startswith(p):
            return g
    return "other"


def render(full: bool = False) -> str:
    meta = metadata()
    names, edges, dev_only = build_graph(meta)
    shown = edges if full else transitive_reduction(edges)
    groups, members, prefixes = load_groups()
    by_group: dict[str, list[str]] = {}
    for n in names:
        by_group.setdefault(group_of(n, members, prefixes), []).append(n)
    lines = [
        "```mermaid",
        '%%{init: {"flowchart": {"htmlLabels": false}, "theme": "neutral"}}%%',
        "flowchart TD",
    ]
    order = [g for g, _ in groups] + (["other"] if "other" in by_group else [])
    titles = dict(groups)
    titles["other"] = "Other"
    for g in order:
        crates = by_group.get(g)
        if not crates:
            continue
        lines.append(f'    subgraph {g}["{titles[g]}"]')
        for c in crates:
            label = short(c) + (" (dev-only)" if c in dev_only else "")
            lines.append(f'        {node_id(c)}["{label}"]')
        lines.append("    end")
    for src in names:
        for dst in sorted(shown[src]):
            lines.append(f"    {node_id(src)} --> {node_id(dst)}")
    lines.append("```")
    count = len(names)
    edge_count = sum(len(v) for v in edges.values())
    shown_count = sum(len(v) for v in shown.values())
    caption = (
        f"*Generated by `tools/crate-graph/crate_graph.py` from `cargo metadata`: {count} workspace "
        f"crates, {edge_count} direct dependency edges"
        + ("" if full else f" ({shown_count} shown; an edge already implied by a longer path is omitted)")
        + ". Dev-dependencies are not drawn. Regenerate with `python tools/crate-graph/crate_graph.py`; "
        "CI fails when this block is stale.*"
    )
    return "\n".join(lines) + "\n\n" + caption


def splice(path: pathlib.Path, block: str) -> tuple[str, str]:
    raw = path.read_bytes().decode("utf-8")
    nl = "\r\n" if "\r\n" in raw else "\n"
    text = raw.replace("\r\n", "\n")
    if BEGIN not in text or END not in text:
        sys.exit(f"{path}: missing {BEGIN} / {END} markers")
    head, rest = text.split(BEGIN, 1)
    _, tail = rest.split(END, 1)
    new = head + BEGIN + "\n\n" + block + "\n\n" + END + tail
    return raw, new.replace("\n", nl)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="fail if a README is out of date")
    ap.add_argument("--print", action="store_true", help="print the Mermaid block only")
    ap.add_argument("--full", action="store_true", help="draw every edge (no transitive reduction)")
    args = ap.parse_args()
    block = render(full=args.full)
    if args.print:
        print(block)
        return 0
    stale = []
    for t in TARGETS:
        old, new = splice(t, block)
        if old != new:
            if args.check:
                stale.append(t.relative_to(ROOT).as_posix())
            else:
                t.write_bytes(new.encode("utf-8"))
                print(f"updated {t.relative_to(ROOT).as_posix()}")
    if stale:
        print("crate graph is stale in: " + ", ".join(stale))
        print("run: python tools/crate-graph/crate_graph.py")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
