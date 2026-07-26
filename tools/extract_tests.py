#!/usr/bin/env python3
"""Regenerate the test inventory under ``docs/testing/inventory/``.

This is the tracked replacement for the ``docs/testing/.scratch/`` scripts that
the inventory's maintenance doc used to describe. Those were gitignored, never
appeared in a diff, could never be reviewed, and are gone. Keeping the generator
in ``tools/`` is the point: the conventions it encodes (link shape, row order,
anchor derivation) become reviewable, and ``--check`` becomes CI-gateable.

What it does
------------
Walks the ``members`` of the root ``Cargo.toml`` (``exclude``d paths are never
visited), finds every ``#[test]`` / ``#[tokio::test]`` / ``#[tokio::test(...)]``
function, and records the crate, file, function name, **the line the ``fn`` is
actually on**, and whether the body contains ``require_db_or_skip!()`` (a
live-DB guard).

It then emits a machine-readable JSON and/or the per-crate markdown tables in
``docs/testing/inventory/``.

Curated columns survive regeneration
------------------------------------
``Kind`` / ``System / Feature`` / ``Added`` / ``What it tests`` / ``Notes`` are
human-curated. The generator keys existing rows by ``(file, test name)`` and
carries those cells forward verbatim; it only ever rewrites the link anchor and
the membership of the table. So a sweep fixes drifted ``#L`` anchors and adds or
drops rows without flattening the prose. New rows get a defaulted ``Kind``
(``live-DB`` when guarded, else ``unit``), a ``System / Feature`` derived from
the module path, and the test's first ``///`` doc line as ``What it tests`` --
per the conventions in ``docs/testing/inventory/maintenance.md``.

``Added`` is left blank for new rows. Deriving it needs ``git log -S`` per test,
which is minutes of subprocess churn for thousands of tests; it is supplied by
whoever runs the sweep, and preserved from then on.

Usage
-----
    python tools/extract_tests.py                     # summary only, writes nothing
    python tools/extract_tests.py --json out.json     # machine-readable dump
    python tools/extract_tests.py --write             # regenerate the markdown
    python tools/extract_tests.py --check             # exit 1 if --write would change anything
    python tools/extract_tests.py --verify-links      # anchor-only drift gate
    python tools/extract_tests.py --list-crates       # per-crate totals table

Only ``--write`` touches the repository. Every mode is safe to re-run: writing
twice in a row produces a byte-identical tree.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# ── repo layout ──────────────────────────────────────────────────────────────

REPO_ROOT = Path(__file__).resolve().parent.parent
INVENTORY_DIR = REPO_ROOT / "docs" / "testing" / "inventory"

#: Packages CI does not build or run (see the pre-PR checklist in CLAUDE.md).
CI_EXCLUDED_PACKAGES = frozenset(
    {
        "cimmeria-app",
        "cimmeria-content-editor",
        "cimmeria-scene-editor",
        "sgw-launcher",
        "cimmeria-client-telemetry",
    }
)

#: Directory names never descended into while walking a member.
SKIP_DIRS = frozenset({"target", "node_modules", ".git", "dist", "build"})

# ── source scanning ──────────────────────────────────────────────────────────

TEST_ATTR_RE = re.compile(r"^\s*#\[(?:tokio::)?test(?:\(.*\))?\]")
FN_RE = re.compile(
    r"^\s*(?:pub\s+(?:\([^)]*\)\s*)?)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?"
    r'(?:extern\s+"[^"]*"\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)'
)
ATTR_RE = re.compile(r"^\s*#\[")
DOC_RE = re.compile(r"^\s*///\s?(.*)$")
LIVE_DB_RE = re.compile(r"require_db_or_skip!")

#: How far past a test attribute to look for the `fn`. Generous enough for a
#: stack of `#[ignore]` / `#[should_panic]` / `#[cfg(...)]` attributes.
FN_LOOKAHEAD = 20


@dataclass
class TestFn:
    crate: str  # workspace member path, e.g. "crates/mercury"
    package: str  # Cargo package name, e.g. "cimmeria-mercury"
    file: str  # repo-relative, forward slashes
    name: str
    line: int  # 1-based line the `fn` is on
    attr_line: int  # 1-based line the test attribute is on
    attr: str  # the attribute text, e.g. "#[tokio::test(flavor = ...)]"
    live_db: bool
    doc: str = ""


@dataclass
class CrateInfo:
    member: str
    package: str
    inventory: str
    ci_gated: bool
    tests: list[TestFn] = field(default_factory=list)
    files_with_tests: set[str] = field(default_factory=set)
    #: Every ``require_db_or_skip!`` invocation, including those in shared
    #: helpers rather than test bodies.
    guard_sites: int = 0
    #: Guard invocations that sit outside any test body — a test calling one of
    #: these is live-DB *transitively*, which a per-body scan cannot see.
    orphan_guards: list[str] = field(default_factory=list)


def strip_noncode(line: str, in_block_comment: bool) -> tuple[str, bool]:
    """Blank out comments and string/char literal bodies so brace counting is safe.

    Returns the scrubbed line and the block-comment state to carry to the next
    line. Handles ``//``, ``/* */``, ``"..."`` with escapes, ``'x'``, and raw
    strings ``r"..."`` / ``r#"..."#``.
    """
    out: list[str] = []
    i = 0
    n = len(line)
    while i < n:
        ch = line[i]
        if in_block_comment:
            if line.startswith("*/", i):
                in_block_comment = False
                i += 2
            else:
                i += 1
            continue
        if line.startswith("//", i):
            break
        if line.startswith("/*", i):
            in_block_comment = True
            i += 2
            continue
        # Raw string: r"..." or r#"..."#, r##"..."##, ...
        if ch == "r" and i + 1 < n and line[i + 1] in '#"':
            j = i + 1
            hashes = 0
            while j < n and line[j] == "#":
                hashes += 1
                j += 1
            if j < n and line[j] == '"':
                terminator = '"' + "#" * hashes
                end = line.find(terminator, j + 1)
                if end == -1:
                    # Unterminated on this line; the remainder is literal body.
                    return "".join(out), in_block_comment
                i = end + len(terminator)
                continue
        if ch == '"':
            i += 1
            while i < n:
                if line[i] == "\\":
                    i += 2
                    continue
                if line[i] == '"':
                    i += 1
                    break
                i += 1
            continue
        if ch == "'":
            # Char literal or a lifetime. Lifetimes have no closing quote.
            if line.startswith("\\", i + 1):
                end = line.find("'", i + 2)
            elif i + 2 < n and line[i + 2] == "'":
                end = i + 2
            else:
                end = -1
            if end != -1:
                i = end + 1
                continue
        out.append(ch)
        i += 1
    return "".join(out), in_block_comment


def scrub_all(lines: list[str]) -> list[str]:
    """Scrub every line of a file once, carrying block-comment state across lines.

    Everything downstream (brace matching, guard detection) reads the scrubbed
    copy. That matters for guard detection specifically: three tests in
    ``cimmeria-services`` carry a comment reading "No ``require_db_or_skip!()`` —
    this branch deliberately …", and matching raw text would count them guarded.
    """
    out: list[str] = []
    in_block_comment = False
    for raw in lines:
        scrubbed, in_block_comment = strip_noncode(raw, in_block_comment)
        out.append(scrubbed)
    return out


def body_span(scrubbed: list[str], fn_index: int) -> tuple[int, int]:
    """Half-open ``[start, end)`` line index range of the fn body at ``fn_index``.

    Counts braces on scrubbed lines so a ``{`` inside a string literal cannot end
    the body early. Falls back to end-of-file on an unbalanced body.
    """
    depth = 0
    started = False
    for i in range(fn_index, len(scrubbed)):
        for ch in scrubbed[i]:
            if ch == "{":
                depth += 1
                started = True
            elif ch == "}":
                depth -= 1
        if started and depth <= 0:
            return fn_index, i + 1
    return fn_index, len(scrubbed)


def collect_doc(lines: list[str], attr_index: int) -> str:
    """First ``///`` line of the doc comment immediately above a test attribute."""
    docs: list[str] = []
    i = attr_index - 1
    while i >= 0:
        line = lines[i]
        m = DOC_RE.match(line)
        if m:
            docs.append(m.group(1).strip())
            i -= 1
            continue
        if ATTR_RE.match(line) or not line.strip():
            # Other attributes / blank lines can sit between doc and #[test];
            # a blank line ends the doc block though.
            if not line.strip():
                break
            i -= 1
            continue
        break
    docs.reverse()
    return first_sentence(" ".join(d for d in docs if d))


def first_sentence(text: str, limit: int = 160) -> str:
    """One short sentence, per the ``What it tests`` convention.

    Doc comments wrap across lines, so taking only the first physical line
    truncates mid-clause. Join the block, then cut at the first sentence end.
    """
    text = " ".join(text.split())
    if not text:
        return ""
    match = re.search(r"(?<![A-Z])\.(?:\s|$)", text)
    if match:
        text = text[: match.start() + 1]
    if len(text) > limit:
        text = text[: limit - 1].rstrip() + "…"
    return text


def scan_file(path: Path, crate: CrateInfo) -> list[TestFn]:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError as exc:  # pragma: no cover - unreadable file
        print(f"warning: cannot read {path}: {exc}", file=sys.stderr)
        return []
    lines = text.splitlines()
    scrubbed = scrub_all(lines)
    rel = path.relative_to(REPO_ROOT).as_posix()
    guard_sites = [i for i, s in enumerate(scrubbed) if LIVE_DB_RE.search(s)]
    claimed: set[int] = set()
    found: list[TestFn] = []
    for idx, line in enumerate(lines):
        if not TEST_ATTR_RE.match(line):
            continue
        attr = line.strip()
        fn_index = None
        for look in range(idx + 1, min(idx + 1 + FN_LOOKAHEAD, len(lines))):
            candidate = lines[look]
            m = FN_RE.match(candidate)
            if m:
                fn_index = look
                fn_name = m.group(1)
                break
            if ATTR_RE.match(candidate) or not candidate.strip():
                continue
            # Anything else between the attribute and the fn means this
            # attribute does not decorate a function (e.g. a macro body).
            break
        if fn_index is None:
            print(
                f"warning: {rel}:{idx + 1} test attribute with no following fn",
                file=sys.stderr,
            )
            continue
        start, end = body_span(scrubbed, fn_index)
        in_body = [g for g in guard_sites if start <= g < end]
        claimed.update(in_body)
        found.append(
            TestFn(
                crate=crate.member,
                package=crate.package,
                file=rel,
                name=fn_name,
                line=fn_index + 1,
                attr_line=idx + 1,
                attr=attr,
                live_db=bool(in_body),
                doc=collect_doc(lines, idx),
            )
        )
    crate.guard_sites += len(guard_sites)
    for g in sorted(set(guard_sites) - claimed):
        crate.orphan_guards.append(f"{rel}:{g + 1}")
    if found:
        crate.files_with_tests.add(rel)
    return found


# ── workspace discovery ──────────────────────────────────────────────────────


def parse_workspace_members(cargo_toml: Path) -> list[str]:
    """Read the ``[workspace] members`` list without a TOML dependency.

    Deliberately dependency-free: this script must run from a bare checkout with
    nothing but a stock Python 3.
    """
    text = cargo_toml.read_text(encoding="utf-8")
    match = re.search(r"^members\s*=\s*\[(.*?)^\]", text, re.S | re.M)
    if not match:
        raise SystemExit(f"could not find [workspace] members in {cargo_toml}")
    body = match.group(1)
    body = re.sub(r"#.*", "", body)
    return re.findall(r'"([^"]+)"', body)


def package_name(member_dir: Path) -> str:
    cargo = member_dir / "Cargo.toml"
    if not cargo.is_file():
        return member_dir.name
    text = cargo.read_text(encoding="utf-8")
    match = re.search(r"^\s*name\s*=\s*\"([^\"]+)\"", text, re.M)
    return match.group(1) if match else member_dir.name


def inventory_filename(member: str) -> str:
    """Map a workspace member path to its catalogue file under the inventory dir."""
    if member == "src-tauri":
        return "tauri-app.md"
    if member.startswith("tools/"):
        return "tools-" + member.split("/", 1)[1].lower() + ".md"
    return member.rsplit("/", 1)[-1] + ".md"


def crate_title(member: str) -> str:
    if member == "src-tauri":
        return "tauri-app"
    if member.startswith("tools/"):
        return member
    return member.rsplit("/", 1)[-1]


def iter_rust_files(root: Path):
    stack = [root]
    while stack:
        current = stack.pop()
        try:
            entries = sorted(current.iterdir())
        except OSError:  # pragma: no cover
            continue
        for entry in entries:
            if entry.is_dir():
                if entry.name in SKIP_DIRS:
                    continue
                stack.append(entry)
            elif entry.suffix == ".rs":
                yield entry


def collect() -> list[CrateInfo]:
    members = parse_workspace_members(REPO_ROOT / "Cargo.toml")
    crates: list[CrateInfo] = []
    for member in members:
        member_dir = REPO_ROOT / member
        if not member_dir.is_dir():
            print(f"warning: workspace member {member} not on disk", file=sys.stderr)
            continue
        pkg = package_name(member_dir)
        info = CrateInfo(
            member=member,
            package=pkg,
            inventory=inventory_filename(member),
            ci_gated=pkg not in CI_EXCLUDED_PACKAGES,
        )
        for rs in iter_rust_files(member_dir):
            info.tests.extend(scan_file(rs, info))
        info.tests.sort(key=lambda t: (t.file, t.line))
        crates.append(info)
    crates.sort(key=lambda c: (-len(c.tests), c.member))
    return crates


# ── markdown rendering ───────────────────────────────────────────────────────

TABLE_HEADER = (
    "| Test | Kind | System / Feature | Added | What it tests | Notes |\n"
    "|---|---|---|---|---|---|"
)
ROW_RE = re.compile(r"^\|\s*\[(?P<name>[^\]]+)\]\((?P<link>[^)]+)\)\s*\|")
ALL_TESTS_RE = re.compile(r"^##\s+All tests\b.*$", re.M)


def system_for(test: TestFn) -> str:
    """Derive the ``System / Feature`` cell from the module path.

    Matches the existing catalogue: ``crates/mercury/src/bundle.rs`` -> ``Bundle``,
    ``crates/mercury/src/channel/tests/channel_lifecycle.rs`` -> ``Channel``.
    """
    parts = test.file.split("/")
    if "src" in parts:
        tail = parts[parts.index("src") + 1 :]
    else:
        tail = parts[-1:]
    if not tail:
        return ""
    head = tail[0]
    if head.endswith(".rs"):
        head = head[:-3]
    if head in {"lib", "main", "mod"} and len(tail) > 1:
        head = tail[1][:-3] if tail[1].endswith(".rs") else tail[1]
    return " ".join(word.capitalize() for word in head.replace("-", "_").split("_"))


def normalize_name(text: str) -> str:
    """Catalogue link text for a test, reduced to the bare function name.

    Most rows use the plain name, but some crates wrap it in backticks. Both must
    key to the same test, or a sweep would drop the curated cells on those rows
    and the link checker would report them as dangling.
    """
    return text.strip().strip("`").strip()


def escape_cell(text: str) -> str:
    return text.replace("|", "\\|").strip()


def split_row(line: str) -> list[str]:
    """Split a markdown table row into its 6 columns.

    A stray ``|`` inside the free-text ``What it tests`` cell would over-split, so
    surplus fields are folded back into that column.
    """
    cols = line.split("|")[1:-1]
    if len(cols) > 6:
        merged = "|".join(cols[4 : len(cols) - 1])
        cols = cols[:4] + [merged, cols[-1]]
    while len(cols) < 6:
        cols.append("")
    return [c.strip() for c in cols]


def read_existing_rows(path: Path) -> dict[tuple[str, str], list[str]]:
    """Index a catalogue file's curated cells by ``(file path, test name)``."""
    if not path.is_file():
        return {}
    rows: dict[tuple[str, str], list[str]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        m = ROW_RE.match(line)
        if not m:
            continue
        link = m.group("link")
        rel = link.split("#", 1)[0]
        rel = re.sub(r"^(\.\./)+", "", rel)
        rows[(rel, normalize_name(m.group("name")))] = split_row(line)
    return rows


def default_header(member: str, total: int, ci_gated: bool) -> str:
    today = _dt.date.today().isoformat()
    return (
        f"# Tests — `{crate_title(member)}`\n"
        "\n"
        "> **Type**: reference  \n"
        "> **Audience**: engineers  \n"
        f"> **Last updated**: {today}  \n"
        f"> **Total tests**: {total:,}  \n"
        f"> **CI-gated**: {'yes' if ci_gated else 'no'}  \n"
        "> **Index**: [README](README.md) | **Playbook**: "
        "[TESTING.md](../../../TESTING.md)\n"
        "\n"
        "_Description pending — replace this line with one sentence on what the "
        "crate covers._\n"
    )


def rebuild_header(existing: str, total: int, ci_gated: bool) -> str:
    """Keep the curated header verbatim, refreshing only the generated figures.

    ``Last updated`` is deliberately *not* stamped: rewriting it on every run
    would make ``--check`` fail a day after any sweep for no real reason. Bump it
    by hand when you publish a sweep.
    """
    out = re.sub(
        r"(^>\s*\*\*Total tests\*\*:\s*)(.*?)(\s*)$",
        lambda m: f"{m.group(1)}{total:,}{m.group(3)}",
        existing,
        count=1,
        flags=re.M,
    )
    out = re.sub(
        r"(^>\s*\*\*CI-gated\*\*:\s*)(\S+)(.*)$",
        lambda m: f"{m.group(1)}{'yes' if ci_gated else 'no'}{m.group(3)}",
        out,
        count=1,
        flags=re.M,
    )
    return out


def render_crate(crate: CrateInfo, path: Path) -> str:
    curated = read_existing_rows(path)
    total = len(crate.tests)

    if path.is_file():
        text = path.read_text(encoding="utf-8")
        split = ALL_TESTS_RE.search(text)
        header = text[: split.start()] if split else text
        header = rebuild_header(header, total, crate.ci_gated)
    else:
        header = default_header(crate.member, total, crate.ci_gated)

    if not header.endswith("\n\n"):
        header = header.rstrip("\n") + "\n\n"

    lines = [header.rstrip("\n"), "", f"## All tests ({total:,})", "", TABLE_HEADER]
    for test in crate.tests:
        cells = curated.get((test.file, test.name))
        if cells:
            kind, system, added, what, notes = cells[1], cells[2], cells[3], cells[4], cells[5]
        else:
            kind = "live-DB" if test.live_db else "unit"
            system = system_for(test)
            added = ""
            what = escape_cell(test.doc)
            notes = ""
        link = f"../../../{test.file}#L{test.line}"
        lines.append(
            f"| [{test.name}]({link}) | {kind} | {system} | {added} | {what} | {notes} |"
        )
    return "\n".join(lines) + "\n"


# ── modes ────────────────────────────────────────────────────────────────────


def totals(crates: list[CrateInfo]) -> dict:
    all_tests = [t for c in crates for t in c.tests]
    gated = [t for c in crates if c.ci_gated for t in c.tests]
    files = {t.file for t in all_tests}
    orphans = [o for c in crates for o in c.orphan_guards]
    return {
        "tests": len(all_tests),
        "files": len(files),
        "ci_gated_tests": len(gated),
        "live_db_tests": sum(1 for t in all_tests if t.live_db),
        "live_db_call_sites": sum(c.guard_sites for c in crates),
        "live_db_call_sites_outside_tests": orphans,
        "crates": len(crates),
    }


def print_summary(crates: list[CrateInfo]) -> None:
    t = totals(crates)
    live_db_crates = sorted({c.package for c in crates if any(x.live_db for x in c.tests)})
    orphans = t["live_db_call_sites_outside_tests"]
    print(f"workspace members scanned : {t['crates']}")
    print(f"tests                     : {t['tests']:,}")
    print(f"files with tests          : {t['files']:,}")
    print(f"CI-gated tests            : {t['ci_gated_tests']:,}")
    print(f"live-DB tests             : {t['live_db_tests']:,}", end="")
    print(f"  ({', '.join(live_db_crates)})" if live_db_crates else "")
    print(f"live-DB call sites        : {t['live_db_call_sites']:,}")
    if orphans:
        print(
            f"  {len(orphans)} of those sit in shared helpers, not test bodies -\n"
            "  tests calling them are live-DB transitively and are NOT counted above:"
        )
        for site in orphans:
            print(f"    {site}")


def print_crate_table(crates: list[CrateInfo]) -> None:
    print(f"{'member':<32} {'package':<28} {'tests':>6} {'files':>6} {'live-DB':>8}  CI")
    for c in crates:
        live = sum(1 for t in c.tests if t.live_db)
        print(
            f"{c.member:<32} {c.package:<28} {len(c.tests):>6} "
            f"{len(c.files_with_tests):>6} {live:>8}  {'yes' if c.ci_gated else 'no'}"
        )


def write_json(crates: list[CrateInfo], out_path: Path) -> None:
    payload = {
        "generated_by": "tools/extract_tests.py",
        "schema_version": 1,
        "totals": totals(crates),
        "crates": [
            {
                "member": c.member,
                "package": c.package,
                "inventory": c.inventory,
                "ci_gated": c.ci_gated,
                "tests": len(c.tests),
                "files": len(c.files_with_tests),
                "live_db_tests": sum(1 for t in c.tests if t.live_db),
                "live_db_call_sites": c.guard_sites,
                "live_db_call_sites_outside_tests": c.orphan_guards,
            }
            for c in crates
        ],
        "tests": [
            {
                "crate": t.crate,
                "package": t.package,
                "file": t.file,
                "name": t.name,
                "line": t.line,
                "attr_line": t.attr_line,
                "attr": t.attr,
                "live_db": t.live_db,
                "doc": t.doc,
            }
            for c in crates
            for t in c.tests
        ],
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {out_path.relative_to(REPO_ROOT) if out_path.is_relative_to(REPO_ROOT) else out_path}")


def write_or_check(crates: list[CrateInfo], *, write: bool) -> int:
    changed: list[str] = []
    for crate in crates:
        path = INVENTORY_DIR / crate.inventory
        if not crate.tests and not path.is_file():
            continue
        rendered = render_crate(crate, path)
        current = path.read_text(encoding="utf-8") if path.is_file() else None
        if current == rendered:
            continue
        changed.append(crate.inventory)
        if write:
            path.write_text(rendered, encoding="utf-8", newline="\n")
    if write:
        for name in changed:
            print(f"updated docs/testing/inventory/{name}")
        if not changed:
            print("inventory already up to date")
        return 0
    if changed:
        print("test inventory is out of date; regenerate with:")
        print("    python tools/extract_tests.py --write")
        for name in changed:
            print(f"  - docs/testing/inventory/{name}")
        return 1
    print("test inventory is up to date")
    return 0


def verify_links(crates: list[CrateInfo]) -> int:
    """Re-derive every ``#L`` anchor in the catalogue and report mismatches.

    Cheaper and far more incrementally adoptable than ``--check``: it ignores
    catalogue membership entirely and only asks whether each existing link still
    points at its own test's ``fn`` line.
    """
    by_key: dict[tuple[str, str], TestFn] = {
        (t.file, t.name): t for c in crates for t in c.tests
    }
    broken = 0
    stale = 0
    checked = 0
    for path in sorted(INVENTORY_DIR.glob("*.md")):
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            m = ROW_RE.match(line)
            if not m:
                continue
            checked += 1
            link = m.group("link")
            name = normalize_name(m.group("name"))
            rel, _, anchor = link.partition("#")
            rel = re.sub(r"^(\.\./)+", "", rel)
            test = by_key.get((rel, name))
            if test is None:
                broken += 1
                print(f"{path.name}:{lineno}: no such test: {name} in {rel}")
                continue
            want = f"L{test.line}"
            if anchor != want:
                stale += 1
                print(f"{path.name}:{lineno}: {name} anchor {anchor or '(none)'} -> {want}")
    print(f"\nchecked {checked:,} links: {broken:,} dangling, {stale:,} stale anchors")
    return 1 if (broken or stale) else 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Extract the workspace test inventory.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Only --write modifies the repository.",
    )
    parser.add_argument("--json", metavar="PATH", help="write the machine-readable dump here")
    parser.add_argument(
        "--write", action="store_true", help="regenerate docs/testing/inventory/*.md"
    )
    parser.add_argument(
        "--check",
        "--dry-run",
        dest="check",
        action="store_true",
        help="exit non-zero if --write would change anything (writes nothing)",
    )
    parser.add_argument(
        "--verify-links",
        action="store_true",
        help="re-derive every #L anchor in the catalogue; exit non-zero on drift",
    )
    parser.add_argument(
        "--list-crates", action="store_true", help="print the per-crate totals table"
    )
    args = parser.parse_args(argv)

    if args.write and args.check:
        parser.error("--write and --check are mutually exclusive")

    crates = collect()

    if args.json:
        write_json(crates, Path(args.json))
    if args.list_crates:
        print_crate_table(crates)
        print()
    if args.verify_links:
        return verify_links(crates)
    if args.write or args.check:
        return write_or_check(crates, write=args.write)

    print_summary(crates)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
